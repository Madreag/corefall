//! M9 § Destructible terrain — 5-tier per-pixel integrity band machine.
//!
//! Per-pixel integrity is a `f32` in `0.0..=1.0` keyed by
//! `(ChunkCoord, local_pos)`. Each projectile hit decrements integrity by
//! `(impact_energy * (1 - normalized_hardness) / normalized_hardness)`, so
//! soft materials (sand 0.1) shed integrity ~16x faster than hard materials
//! (metal 0.9) under the same impact. Crossing a band threshold fires
//! `terrain.material_state_changed`; reaching 0 fires `terrain.pixel_removed`
//! and triggers the cascade decay rule on direct neighbors whose normalized
//! hardness is at or below `cascade_threshold` (default 0.6).
//!
//! The 5-tier band thresholds mirror `cf-ui::reactor_hp_bar::IntegrityBand`
//! so the player learns one damage grammar across terrain + reactor armor.
//!
//! Anti-scope (later milestones):
//! - Heat-coupled hardness shifts (ice melting on thermal input) — M16+
//! - Material-specific cascade probability (vs. flat threshold) — M14+
//! - Recursive cascade depth > 1 — M14+

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::chunked::{
    material_affordance, MaterialId, MATERIAL_AIR, MATERIAL_ANCHOR, MATERIAL_CONCRETE, MATERIAL_DIRT, MATERIAL_HAZARD,
    MATERIAL_LOOSE_FILL, MATERIAL_METAL_NOHOOK, MATERIAL_REPAIR_FILL, MATERIAL_SUPPORT_BEAM,
};

/// 5-tier integrity band. Mirrors `cf-ui::IntegrityBand`; kept in cf-terrain so
/// projectile-vs-terrain emission can label band crossings without depending
/// on the UI crate.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum IntegrityBand {
    Pristine,
    Scratched,
    Cracked,
    Critical,
    Destroyed,
}

impl IntegrityBand {
    /// Resolve a band from integrity in `0.0..=1.0`. Thresholds match the
    /// M9 spec § Destructible terrain table.
    #[must_use]
    pub fn from_integrity(integrity: f32) -> Self {
        if integrity <= 0.0 {
            IntegrityBand::Destroyed
        } else if integrity < 0.25 {
            IntegrityBand::Critical
        } else if integrity < 0.50 {
            IntegrityBand::Cracked
        } else if integrity < 0.75 {
            IntegrityBand::Scratched
        } else {
            IntegrityBand::Pristine
        }
    }

    /// Stable canonical name for replay / cfctl payload. Matches the
    /// `terrain.material_state_changed` schema enum.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            IntegrityBand::Pristine => "Pristine",
            IntegrityBand::Scratched => "Scratched",
            IntegrityBand::Cracked => "Cracked",
            IntegrityBand::Critical => "Critical",
            IntegrityBand::Destroyed => "Destroyed",
        }
    }
}

/// Kind of damage applied to a pixel. Drives the `cause` field on the
/// `terrain.material_state_changed` event. Stable string identifiers so
/// future damage sources extend the enum without breaking event schema.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum DamageKind {
    /// Projectile-vs-terrain hit (rifle, sniper, guard).
    ProjectileHit,
    /// Direct dig action (digger tool).
    Dig,
    /// Explosive blast.
    Blast,
    /// Cascade decay from a neighbor destruction.
    NeighborDestroyed,
}

impl DamageKind {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            DamageKind::ProjectileHit => "projectile_hit",
            DamageKind::Dig => "dig",
            DamageKind::Blast => "blast",
            DamageKind::NeighborDestroyed => "neighbor_destroyed",
        }
    }
}

/// Per-pixel integrity metadata. Lazily allocated — only damaged pixels
/// occupy entries in `ChunkedTerrain::pixel_meta_grid`. Air pixels never
/// have metadata. Pristine pixels (untouched) also have no entry; their
/// integrity is implicit 1.0.
///
/// Layout is `Serialize` so snapshot round-trips preserve damage state
/// for replay determinism — required by M4's `sim_state_v1` checksum
/// scope and M8A's per-tick determinism contract.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PixelMeta {
    /// Integrity in `0.0..=1.0`. Decays under damage; reaching 0 destroys
    /// the pixel.
    pub integrity: f32,
    /// Tick of the most-recent damage event. Used by future thermal coupling
    /// (M16+) for "what was just hit" decay accounting.
    pub last_damage_tick: u64,
    /// Most-recent damage kind. Surfaces on cfctl
    /// `observe.terrain.material_at` as the audit field.
    pub damage_kind: DamageKind,
    /// Optional record id of the originating event (e.g. projectile spawn
    /// event id). Used to walk the cause chain in M10 viewer.
    pub damage_source: Option<String>,
}

impl PixelMeta {
    /// Fresh pristine entry. Callers usually skip allocating until the first
    /// damage event lands — this is provided for snapshot restoration.
    #[must_use]
    pub fn pristine() -> Self {
        Self {
            integrity: 1.0,
            last_damage_tick: 0,
            damage_kind: DamageKind::ProjectileHit,
            damage_source: None,
        }
    }

    /// Current band derived from integrity.
    #[must_use]
    pub fn band(&self) -> IntegrityBand {
        IntegrityBand::from_integrity(self.integrity)
    }
}

/// Default cascade threshold per M9 spec § Cascade rule. Materials with
/// `normalized_hardness <= cascade_threshold` are subject to cascade decay
/// when a neighbor reaches Destroyed.
pub const DEFAULT_CASCADE_THRESHOLD: f32 = 0.6;

/// Default cascade decay percent per M9 spec § Cascade rule. When a pixel
/// is destroyed, neighbors at or below `cascade_threshold` lose this
/// fraction of their current integrity (clamped to [0, 1]). The Gherkin
/// example ("integrity=0.4 → 0.3 after cascade") is reproduced when this
/// constant is 0.1.
pub const DEFAULT_CASCADE_DECAY_PCT: f32 = 0.1;

/// Default cascade depth — `1` at M9 = direct 4-neighbors only. M14+ may
/// raise this for chained cascades through fragile material strata.
pub const DEFAULT_CASCADE_DEPTH: u32 = 1;

/// Normalized hardness in `0.0..=1.0` per material. Drives the per-pixel
/// integrity damage formula and the cascade-threshold gate. Distinct from
/// the raw `MaterialAffordance::hardness` (which is in CCCP-normalized
/// `0..=400` integrity-squared scale and feeds the binary
/// projectile-penetration check at `cf-physics::try_penetrate`). Per the
/// M9 spec table § Destructible terrain — 5-tier HP color states.
#[must_use]
pub fn normalized_hardness(id: MaterialId) -> f32 {
    match id {
        MATERIAL_AIR => 0.0,
        MATERIAL_DIRT => 0.2,
        MATERIAL_LOOSE_FILL => 0.1,
        MATERIAL_CONCRETE => 0.7,
        // threshold so neighbor cave-ins do not cascade through it.
        // Same numeric tier as `metal_nohook` (0.9) per the M14E table.
        MATERIAL_METAL_NOHOOK | MATERIAL_SUPPORT_BEAM => 0.9,
        MATERIAL_HAZARD => 0.5,
        MATERIAL_REPAIR_FILL => 0.3,
        MATERIAL_ANCHOR => 0.95,
        _ => material_affordance(id)
            .map(|a| (a.hardness / 100.0).clamp(0.0, 1.0))
            .unwrap_or(0.0),
    }
}

/// Apply the M9 per-pixel integrity damage formula:
/// `damage = impact_energy * (1 - hardness) / hardness`, clamped to
/// `[0, 1]`. Returns the new integrity after damage.
///
/// Reproduces the spec's Gherkin "Each material has distinct hardness
/// curve":
/// - sand (h=0.1) under impact_energy=0.5 → damage=4.5, clamped → integrity ~0.0
/// - metal (h=0.9) under impact_energy=0.5 → damage=0.0556 → integrity ~0.94
#[must_use]
pub fn apply_damage_formula(integrity_before: f32, impact_energy: f32, hardness: f32) -> f32 {
    if impact_energy <= 0.0 {
        return integrity_before.clamp(0.0, 1.0);
    }
    let safe_hardness = hardness.max(0.01);
    let softness = (1.0 - hardness).max(0.0);
    let damage = (impact_energy * softness / safe_hardness).clamp(0.0, 1.0);
    (integrity_before - damage).clamp(0.0, 1.0)
}

/// One band-crossing event observed during a damage application. Returned
/// from `ChunkedTerrain::try_penetrate_pixel` so the engine can emit the
/// `terrain.material_state_changed` record at the right tick with the
/// right `parent_event_id`.
#[derive(Debug, Clone, PartialEq)]
pub struct BandCrossing {
    /// Pixel-space position of the crossed pixel.
    pub pos: [i64; 2],
    /// Material at the pixel at the moment of crossing.
    pub material_id: MaterialId,
    /// Material name for the event payload (stable string).
    pub material_name: &'static str,
    /// Band before damage (e.g. Pristine).
    pub from_band: IntegrityBand,
    /// Band after damage (e.g. Scratched).
    pub to_band: IntegrityBand,
    /// Integrity value before damage.
    pub integrity_before: f32,
    /// Integrity value after damage.
    pub integrity_after: f32,
    /// Damage kind that produced the crossing.
    pub cause: DamageKind,
}

/// One cascade-decay event observed when a destroyed pixel triggered
/// integrity loss in a neighbor. Returned alongside the primary outcome so
/// the engine emits `terrain.cascade_triggered` records with real
/// `from_pos` / `to_pos` / `affected_count` payload state.
#[derive(Debug, Clone, PartialEq)]
pub struct CascadeEvent {
    /// Pixel-space position of the destroyed source pixel.
    pub from_pos: [i64; 2],
    /// Pixel-space position of the affected neighbor.
    pub to_pos: [i64; 2],
    /// Material id at the affected pixel.
    pub material_id: MaterialId,
    /// Material name for the event payload.
    pub material_name: &'static str,
    /// Integrity at the neighbor before cascade decay.
    pub integrity_before: f32,
    /// Integrity at the neighbor after cascade decay.
    pub integrity_after: f32,
    /// Band at the neighbor before cascade decay.
    pub from_band: IntegrityBand,
    /// Band at the neighbor after cascade decay.
    pub to_band: IntegrityBand,
    /// True when the cascade decay destroyed the neighbor outright (rare
    /// at default decay_pct=0.1; more common at low pre-existing integrity).
    pub destroyed_neighbor: bool,
    /// Cascade depth — `1` at M9 (direct neighbors only).
    pub depth: u32,
    /// Cascade threshold that was honored for this decay.
    pub threshold: f32,
}

/// Outcome of one `ChunkedTerrain::try_penetrate_pixel` call.
#[derive(Debug, Clone, PartialEq)]
pub struct PenetrationOutcome {
    /// Pixel-space position the damage was applied to.
    pub pos: [i64; 2],
    /// Material id at the pixel before damage. `MATERIAL_AIR` indicates
    /// the engine called against an empty pixel — caller should ignore.
    pub material_id: MaterialId,
    /// Material name for the event payload.
    pub material_name: &'static str,
    /// Integrity value before damage.
    pub integrity_before: f32,
    /// Integrity value after damage.
    pub integrity_after: f32,
    /// Band before damage.
    pub band_before: IntegrityBand,
    /// Band after damage.
    pub band_after: IntegrityBand,
    /// True when a band threshold was crossed (drives
    /// `terrain.material_state_changed` emission).
    pub band_crossed: bool,
    /// True when the pixel reached `integrity == 0` and was removed from
    /// the world (set to air). Drives `terrain.pixel_removed` emission.
    pub destroyed: bool,
    /// Cascade decay events triggered by this damage. Empty when the
    /// pixel survived or when no neighbor met the cascade-threshold gate.
    pub cascades: Vec<CascadeEvent>,
}

impl PenetrationOutcome {
    /// Build the per-event band-crossing record for the engine to emit.
    /// `None` when no band was actually crossed (engine skips emission).
    #[must_use]
    pub fn as_band_crossing(&self, cause: DamageKind) -> Option<BandCrossing> {
        if !self.band_crossed {
            return None;
        }
        Some(BandCrossing {
            pos: self.pos,
            material_id: self.material_id,
            material_name: self.material_name,
            from_band: self.band_before,
            to_band: self.band_after,
            integrity_before: self.integrity_before,
            integrity_after: self.integrity_after,
            cause,
        })
    }
}

/// Sparse per-pixel integrity grid. Keyed by `(ChunkCoord, local_xy)` so
/// snapshot round-trips and determinism checksums can iterate in a stable
/// order. Air pixels never appear here; Pristine (1.0) pixels are
/// implicit and only get an entry once damage lands.
///
/// Layout-stable map type so M4 snapshot writers can hash the grid into
/// `sim_state_v1` without re-encoding when new fields are added.
pub type PixelMetaGrid = BTreeMap<PixelMetaKey, PixelMeta>;

/// Composite key into the per-pixel integrity grid. Stored as raw
/// `(chunk.cx, chunk.cy, local_x, local_y)` so the grid is stable across
/// runs (chunks iterate in `BTreeMap` order).
#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PixelMetaKey {
    pub cx: i32,
    pub cy: i32,
    pub lx: u32,
    pub ly: u32,
}

impl PixelMetaKey {
    #[must_use]
    pub fn new(cx: i32, cy: i32, lx: u32, ly: u32) -> Self {
        Self { cx, cy, lx, ly }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn band_thresholds_match_spec() {
        assert_eq!(IntegrityBand::from_integrity(1.0), IntegrityBand::Pristine);
        assert_eq!(IntegrityBand::from_integrity(0.80), IntegrityBand::Pristine);
        assert_eq!(IntegrityBand::from_integrity(0.74), IntegrityBand::Scratched);
        assert_eq!(IntegrityBand::from_integrity(0.50), IntegrityBand::Scratched);
        assert_eq!(IntegrityBand::from_integrity(0.49), IntegrityBand::Cracked);
        assert_eq!(IntegrityBand::from_integrity(0.25), IntegrityBand::Cracked);
        assert_eq!(IntegrityBand::from_integrity(0.10), IntegrityBand::Critical);
        assert_eq!(IntegrityBand::from_integrity(0.0), IntegrityBand::Destroyed);
    }

    #[test]
    fn damage_formula_matches_spec_sand_vs_metal() {
        let sand_after = apply_damage_formula(1.0, 0.5, normalized_hardness(MATERIAL_LOOSE_FILL));
        let metal_after = apply_damage_formula(1.0, 0.5, normalized_hardness(MATERIAL_METAL_NOHOOK));
        assert!(
            sand_after < 0.05,
            "sand integrity should drop to ~0.0 under impact 0.5; got {sand_after}"
        );
        assert!(
            (metal_after - 0.94).abs() < 0.02,
            "metal integrity should drop to ~0.94 under impact 0.5; got {metal_after}"
        );
    }

    #[test]
    fn damage_formula_light_hit_dirt_lands_in_scratched_band() {
        let after = apply_damage_formula(1.0, 0.1, normalized_hardness(MATERIAL_DIRT));
        assert_eq!(IntegrityBand::from_integrity(after), IntegrityBand::Scratched);
    }

    #[test]
    fn damage_formula_clamps_no_negative() {
        let after = apply_damage_formula(0.1, 10.0, normalized_hardness(MATERIAL_LOOSE_FILL));
        assert_eq!(after, 0.0);
    }

    #[test]
    fn damage_formula_no_energy_no_change() {
        let after = apply_damage_formula(0.7, 0.0, normalized_hardness(MATERIAL_DIRT));
        assert!((after - 0.7).abs() < 1e-6);
    }

    #[test]
    fn normalized_hardness_covers_launch_set() {
        assert_eq!(normalized_hardness(MATERIAL_AIR), 0.0);
        assert!(normalized_hardness(MATERIAL_LOOSE_FILL) < normalized_hardness(MATERIAL_DIRT));
        assert!(normalized_hardness(MATERIAL_DIRT) < normalized_hardness(MATERIAL_CONCRETE));
        assert!(normalized_hardness(MATERIAL_CONCRETE) < normalized_hardness(MATERIAL_METAL_NOHOOK));
        assert!(normalized_hardness(MATERIAL_METAL_NOHOOK) < normalized_hardness(MATERIAL_ANCHOR));
    }

    #[test]
    fn cascade_threshold_gates_correct_materials() {
        assert!(normalized_hardness(MATERIAL_LOOSE_FILL) <= DEFAULT_CASCADE_THRESHOLD);
        assert!(normalized_hardness(MATERIAL_DIRT) <= DEFAULT_CASCADE_THRESHOLD);
        assert!(normalized_hardness(MATERIAL_REPAIR_FILL) <= DEFAULT_CASCADE_THRESHOLD);
        assert!(normalized_hardness(MATERIAL_HAZARD) <= DEFAULT_CASCADE_THRESHOLD);
        assert!(normalized_hardness(MATERIAL_CONCRETE) > DEFAULT_CASCADE_THRESHOLD);
        assert!(normalized_hardness(MATERIAL_METAL_NOHOOK) > DEFAULT_CASCADE_THRESHOLD);
        assert!(normalized_hardness(MATERIAL_ANCHOR) > DEFAULT_CASCADE_THRESHOLD);
    }

    #[test]
    fn pixel_meta_pristine_starts_at_full_integrity() {
        let pm = PixelMeta::pristine();
        assert_eq!(pm.integrity, 1.0);
        assert_eq!(pm.band(), IntegrityBand::Pristine);
    }

    #[test]
    fn pixel_meta_key_orders_deterministically() {
        let mut grid: PixelMetaGrid = BTreeMap::new();
        grid.insert(PixelMetaKey::new(0, 0, 1, 1), PixelMeta::pristine());
        grid.insert(PixelMetaKey::new(0, 0, 1, 0), PixelMeta::pristine());
        grid.insert(PixelMetaKey::new(0, 0, 0, 0), PixelMeta::pristine());
        let keys: Vec<_> = grid.keys().copied().collect();
        assert_eq!(keys[0], PixelMetaKey::new(0, 0, 0, 0));
        assert_eq!(keys[1], PixelMetaKey::new(0, 0, 1, 0));
        assert_eq!(keys[2], PixelMetaKey::new(0, 0, 1, 1));
    }
}
