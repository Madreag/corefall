//! Terrain crate. Owns:
//!
//! - **M1.5 soft-breach barrier proxy** (`BreachStrip`, `BreachWorld`, `try_dig`,
//!   `DigRequest`, `DigOutcome`, `BreachView`) — kept alive for the M1.5
//!   `micro_breach` scenario backward compat. Event names match M2's emit
//!   shape so replay tooling does not migrate.
//! - **M2 chunked pixel terrain** (`chunked` module): `ChunkedTerrain`,
//!   `MaterialId`, `MaterialRegistry`, `MaterialAffordance`, `try_carve`,
//!   `try_blast`, `fill_aabb`, `fill_circle`, `aabb_overlaps_solid`,
//!   `column_top_solid_y`, `ChunkedTerrainSnapshot`, `TerrainStamp`,
//!   plus the DR-007 launch material set (8 ids: air, dirt, concrete,
//!   metal_nohook, hazard, loose_fill, repair_fill, anchor).
//!
//! The two layers coexist for BP2: scenarios that opt into chunked terrain
//! (`scenario.terrain = Some(...)`) drive `act.player.dig` against
//! `ChunkedTerrain`; scenarios still using `breaches[]` (e.g. `micro_breach`)
//! drive `BreachWorld`. The engine prefers chunked terrain when both are
//! present.
//!
//! Anti-scope (lands at M5.6 Material Kernel): active CA / reaction table /
//! phase change / chemistry. Anti-scope (lands at M5.5): full collision
//! matrix. Anti-scope (lands at M5.10 / M7.5): atmospherics.

#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(
    clippy::module_name_repetitions,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::doc_markdown,
    clippy::missing_const_for_fn,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_lossless,
    clippy::float_cmp,
    clippy::redundant_closure,
    clippy::derivable_impls,
    clippy::wildcard_in_or_patterns,
    clippy::needless_pass_by_value,
    clippy::manual_is_multiple_of,
    clippy::trivially_copy_pass_by_ref,
    clippy::needless_range_loop,
    clippy::single_match_else,
    clippy::needless_continue,
    clippy::cast_precision_loss,
    clippy::option_if_let_else,
    clippy::too_many_lines,
    clippy::struct_excessive_bools,
    clippy::unused_self,
    clippy::map_unwrap_or,
    clippy::manual_let_else,
    clippy::similar_names
)]

// M2 / M3 spec "## Files" wiring: re-export the canonical types via thin
// modules so consumers that import per the spec paths
// (`cf_terrain::breach::*`, `cf_terrain::chunk::*`, `cf_terrain::dirty::*`,
// `cf_terrain::carve::*`, `cf_terrain::checksum::*`) compile cleanly.
pub mod active_region;
pub mod air;
pub mod breach;
pub mod ca;
pub mod carve;
pub mod cave_in;
pub mod checksum;
pub mod chunk;
pub mod chunked;
pub mod constants;
pub mod dirty;
pub mod heat;
pub mod integrity;
pub mod m14a_overlay;
pub mod parallel;
pub mod structural_integrity;
pub mod wall_collapse;
pub use air::{AirField, AIR_GRID_CELLS, AIR_GRID_SIZE, AMBIENT_PRESSURE_KPA};
pub use ca::{ca_movement_class, step_ca, step_ca_filtered, CaMovementClass, CaStepReport, CaStepperState};
pub use heat::{ambient_temperature_k, HeatField, HEAT_GRID_CELLS, HEAT_GRID_SIZE};
pub use m14a_overlay::{material_thermal_contact, material_walk_modulator, ThermalContact, WalkModulator};

pub use cave_in::{
    cascade_neighbors_for_chunk, cave_in_chance_per_tick, cave_in_roll, falling_debris_count, CascadeNeighbor,
    CaveInOutcome, CaveInPayload, CAVE_IN_BASE_COEFFICIENT, FALLING_DEBRIS_CAP, UNSUPPORTED_SPAN_FLOOR_PX,
    VIBRATION_MODIFIER_BASELINE, VIBRATION_MODIFIER_PLASMA_CUTTER,
};
pub use wall_collapse::{
    lateral_cascade_neighbors_for_chunk, pressure_blowout_triggers, wall_bulging_chance_per_tick,
    wall_bulging_roll, wall_crack_advanced_chance_per_tick, wall_crack_advanced_roll, wall_rupture_chance_per_tick,
    wall_rupture_debris_count, wall_rupture_roll, CumulativeStress, WallCollapseOutcome, WallCollapsePayload,
    WallCollapseStage, WALL_COLLAPSE_BASE_COEFFICIENT, WALL_LATERAL_SPAN_FLOOR_PX, WALL_LATERAL_STABLE_SPAN_PX,
    WALL_RUPTURE_DEBRIS_CAP,
};
pub use chunked::{
    material_affordance, material_id_from_name, material_name_from_id, Chunk, ChunkCoord, ChunkedCarveNoOp,
    ChunkedCarveOutcome, ChunkedCarveRefusal, ChunkedCarveStats, ChunkedTerrain, ChunkedTerrainSnapshot,
    ChunkedTerrainSnapshotChunk, MaterialAffordance, MaterialId, MaterialRegistry, TerrainStamp, CHUNK_SIZE,
    MATERIAL_AIR, MATERIAL_ANCHOR, MATERIAL_CONCRETE, MATERIAL_DIRT, MATERIAL_HAZARD, MATERIAL_LOOSE_FILL,
    MATERIAL_METAL_NOHOOK, MATERIAL_REPAIR_FILL, MATERIAL_SCHEMA_VERSION, MATERIAL_SUPPORT_BEAM,
};
pub use structural_integrity::{
    compute_integrity_pass, compute_lateral_integrity_pass, lock_radius_to_beam, unlock_radius, IntegrityField,
    IntegrityPassOutcome, INTEGRITY_BEAM_LOCKED, INTEGRITY_CASCADE_THRESHOLD, INTEGRITY_FIELD_CELLS,
    INTEGRITY_FIELD_HEIGHT, INTEGRITY_FIELD_WIDTH, INTEGRITY_LOCKED, INTEGRITY_PASS_CADENCE_TICKS,
};
pub use integrity::{
    apply_damage_formula, normalized_hardness, BandCrossing, CascadeEvent, DamageKind, IntegrityBand,
    PenetrationOutcome, PixelMeta, PixelMetaGrid, PixelMetaKey, DEFAULT_CASCADE_DECAY_PCT, DEFAULT_CASCADE_DEPTH,
    DEFAULT_CASCADE_THRESHOLD,
};

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// One M1.5 breach strip. Position is encoded as a pair of `[x, y]` arrays so this
/// crate stays free of `cf-actor` / `cf-physics` dependencies (those crates own
/// their own `Vec2`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BreachStrip {
    pub id: String,
    pub bbox_min: [f32; 2],
    pub bbox_max: [f32; 2],
    pub material: String,
    /// Total HP of the strip. `0.0` means the strip is permanently broken.
    pub max_hp: f32,
    /// HP remaining. When this hits `0.0` the strip is broken; further digs are
    /// refused with `already_broken` so replay viewers can see the player still
    /// pressing dig past completion (instead of silent no-ops).
    pub hp: f32,
    /// Damage absorbed by one successful dig. Higher = breaks faster. M1.5 default
    /// is 20.0 against a 60.0 hp wall, so three carves break the wall.
    pub hardness: f32,
    /// Maximum world-space distance from the player's centre to the nearest point
    /// on the strip's AABB for the dig to be considered "in range".
    pub dig_range: f32,
    /// Persistent refusal reason for this material (e.g. `metal_nohook`). When set,
    /// every dig refuses with this label even if the player is in range. `None` =
    /// the strip is breakable.
    #[serde(default)]
    pub refusal_reason: Option<String>,
    /// True once HP reached zero and a final `terrain_carved { broken: true }`
    /// event was emitted. Locks future digs to refuse.
    #[serde(default)]
    pub broken: bool,
}

impl BreachStrip {
    /// True if the strip is fully carved through.
    pub fn is_broken(&self) -> bool {
        self.broken || self.hp <= 0.0
    }

    /// Centre of the strip's AABB.
    pub fn center(&self) -> [f32; 2] {
        [
            (self.bbox_min[0] + self.bbox_max[0]) * 0.5,
            (self.bbox_min[1] + self.bbox_max[1]) * 0.5,
        ]
    }

    /// Half-extents of the strip (positive). Convenient for renderers.
    pub fn half_extents(&self) -> [f32; 2] {
        [
            ((self.bbox_max[0] - self.bbox_min[0]) * 0.5).max(0.0),
            ((self.bbox_max[1] - self.bbox_min[1]) * 0.5).max(0.0),
        ]
    }

    /// Restore HP to `max_hp` and clear `broken`. Used by `scenario.reset`.
    pub fn reset(&mut self) {
        self.hp = self.max_hp;
        self.broken = false;
    }

    /// Distance from `(px, py)` to the nearest point on the strip's AABB, clamped
    /// to zero when the point is inside.
    pub fn distance_to(&self, px: f32, py: f32) -> f32 {
        let dx = (self.bbox_min[0] - px).max(0.0).max(px - self.bbox_max[0]);
        let dy = (self.bbox_min[1] - py).max(0.0).max(py - self.bbox_max[1]);
        ((dx * dx) + (dy * dy)).sqrt()
    }
}

/// Outcome of one [`try_dig`] call.
#[derive(Debug, Clone, PartialEq)]
pub enum DigOutcome {
    /// Dig landed and reduced the strip's HP.
    Carved {
        strip_id: String,
        material: String,
        bbox_min: [f32; 2],
        bbox_max: [f32; 2],
        damage_applied: f32,
        hp_remaining: f32,
        broken: bool,
    },
    /// Dig was refused with a structured reason. Reason names match what M2 will
    /// emit (`out_of_range`, `material_metal_nohook`, `already_broken`).
    Refused {
        reason: String,
        strip_id: Option<String>,
        material: Option<String>,
        bbox_min: Option<[f32; 2]>,
        bbox_max: Option<[f32; 2]>,
    },
}

impl DigOutcome {
    pub fn is_carved(&self) -> bool {
        matches!(self, DigOutcome::Carved { .. })
    }

    pub fn refused_reason(&self) -> Option<&str> {
        match self {
            DigOutcome::Refused { reason, .. } => Some(reason.as_str()),
            DigOutcome::Carved { .. } => None,
        }
    }
}

/// World-state container the engine clones per tick. Maps strip id → `BreachStrip`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BreachWorld {
    pub strips: BTreeMap<String, BreachStrip>,
}

impl BreachWorld {
    pub fn new(strips: Vec<BreachStrip>) -> Self {
        let mut map = BTreeMap::new();
        for s in strips {
            map.insert(s.id.clone(), s);
        }
        Self { strips: map }
    }

    pub fn get(&self, id: &str) -> Option<&BreachStrip> {
        self.strips.get(id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut BreachStrip> {
        self.strips.get_mut(id)
    }

    pub fn is_broken(&self, id: &str) -> bool {
        self.strips.get(id).is_some_and(BreachStrip::is_broken)
    }

    pub fn iter(&self) -> impl Iterator<Item = &BreachStrip> {
        self.strips.values()
    }

    /// Reset every strip to full HP. Used by `scenario.reset`.
    pub fn reset(&mut self) {
        for s in self.strips.values_mut() {
            s.reset();
        }
    }

    /// Determinism hash bytes. Layout-stable; M2 will append per-pixel chunk data
    /// without disturbing the M1.5 prefix.
    pub fn checksum_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.strips.len() * 32);
        out.extend_from_slice(&(self.strips.len() as u64).to_le_bytes());
        for (id, s) in &self.strips {
            out.extend_from_slice(&(id.len() as u32).to_le_bytes());
            out.extend_from_slice(id.as_bytes());
            out.extend_from_slice(&quantize(s.bbox_min[0]).to_le_bytes());
            out.extend_from_slice(&quantize(s.bbox_min[1]).to_le_bytes());
            out.extend_from_slice(&quantize(s.bbox_max[0]).to_le_bytes());
            out.extend_from_slice(&quantize(s.bbox_max[1]).to_le_bytes());
            out.extend_from_slice(&quantize(s.hp).to_le_bytes());
            out.push(u8::from(s.broken));
        }
        out
    }

    /// Map of `strip_id -> broken?`. Cheap to compute; cf-mission consumes this.
    pub fn broken_map(&self) -> BTreeMap<String, bool> {
        self.strips.iter().map(|(k, v)| (k.clone(), v.is_broken())).collect()
    }

    /// **M1.5**: map of `strip_id -> carve_progress` in `[0.0, 1.0]`. Used by
    /// `cf-mission` to emit `mission.objective_updated` at 25/50/75/100%
    /// progress milestones for `BreachBarrier` objectives. Strips with
    /// `max_hp == 0` report `1.0` (immediate-broken).
    pub fn progress_map(&self) -> BTreeMap<String, f32> {
        self.strips
            .iter()
            .map(|(k, v)| {
                let pct = if v.max_hp > 0.0 {
                    (1.0 - v.hp / v.max_hp).clamp(0.0, 1.0)
                } else {
                    1.0
                };
                (k.clone(), pct)
            })
            .collect()
    }
}

fn quantize(value: f32) -> i32 {
    if !value.is_finite() {
        return 0;
    }
    (value * 1024.0).round() as i32
}

/// Inputs for one dig attempt.
#[derive(Debug, Clone, PartialEq)]
pub struct DigRequest {
    /// Player centre in world space.
    pub origin: [f32; 2],
    /// Aim unit vector. Used to bias the picker so aiming through a barrier
    /// targets the strip in front of the player.
    pub aim: [f32; 2],
    /// Optional explicit target id. When `None` the helper picks the nearest
    /// in-range strip.
    pub explicit_target: Option<String>,
}

/// Try to land a dig action. Picks the nearest in-range strip when no explicit
/// target id is provided, applies `hardness` damage on success, and returns a
/// structured [`DigOutcome`] the engine turns into recorder events.
#[must_use]
pub fn try_dig(world: &mut BreachWorld, req: DigRequest) -> DigOutcome {
    let _ = req.aim; // Reserved; M2 will use aim to pick chunks along a swept ray.

    if let Some(id) = req.explicit_target.as_deref() {
        match world.strips.get_mut(id) {
            None => {
                return DigOutcome::Refused {
                    reason: "unknown_target".to_string(),
                    strip_id: Some(id.to_string()),
                    material: None,
                    bbox_min: None,
                    bbox_max: None,
                };
            }
            Some(strip) => return apply_dig_to(strip, req.origin),
        }
    }

    // No explicit target: pick the nearest in-range strip. We deliberately do NOT
    // skip refusal-only strips here so the player still gets a refusal event when
    // they swing at a metal-nohook anchor — that's the documented teaching path.
    let mut best: Option<(String, f32)> = None;
    for (id, strip) in &world.strips {
        let d = strip.distance_to(req.origin[0], req.origin[1]);
        if d > strip.dig_range {
            continue;
        }
        match &best {
            None => best = Some((id.clone(), d)),
            Some((_, current_d)) if d < *current_d => best = Some((id.clone(), d)),
            _ => {}
        }
    }
    match best {
        None => DigOutcome::Refused {
            reason: "out_of_range".to_string(),
            strip_id: None,
            material: None,
            bbox_min: None,
            bbox_max: None,
        },
        Some((id, _)) => {
            let strip = world.strips.get_mut(&id).expect("picked id exists");
            apply_dig_to(strip, req.origin)
        }
    }
}

fn apply_dig_to(strip: &mut BreachStrip, origin: [f32; 2]) -> DigOutcome {
    let in_range = strip.distance_to(origin[0], origin[1]) <= strip.dig_range;
    if !in_range {
        return DigOutcome::Refused {
            reason: "out_of_range".to_string(),
            strip_id: Some(strip.id.clone()),
            material: Some(strip.material.clone()),
            bbox_min: Some(strip.bbox_min),
            bbox_max: Some(strip.bbox_max),
        };
    }
    if strip.is_broken() {
        return DigOutcome::Refused {
            reason: "already_broken".to_string(),
            strip_id: Some(strip.id.clone()),
            material: Some(strip.material.clone()),
            bbox_min: Some(strip.bbox_min),
            bbox_max: Some(strip.bbox_max),
        };
    }
    if let Some(_reason) = &strip.refusal_reason {
        // M2/M3 audit pass 5 (2026-05-13): all non-diggable strip refusals
        // use the stable `material_not_diggable` reason vocabulary per spec.
        // The specific material name remains on the `material` field for
        // structured consumers.
        return DigOutcome::Refused {
            reason: "material_not_diggable".to_string(),
            strip_id: Some(strip.id.clone()),
            material: Some(strip.material.clone()),
            bbox_min: Some(strip.bbox_min),
            bbox_max: Some(strip.bbox_max),
        };
    }
    let damage = strip.hardness.max(1.0);
    let damage_applied = damage.min(strip.hp);
    strip.hp = (strip.hp - damage).max(0.0);
    let broken_now = strip.hp <= 0.0;
    if broken_now {
        strip.broken = true;
    }
    DigOutcome::Carved {
        strip_id: strip.id.clone(),
        material: strip.material.clone(),
        bbox_min: strip.bbox_min,
        bbox_max: strip.bbox_max,
        damage_applied,
        hp_remaining: strip.hp,
        broken: broken_now,
    }
}

/// View projection of a strip for `observe.frame`. M4 will style the HUD; M1.5 only
/// needs raw values so the run-bundle viewer can render a progress bar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BreachView {
    pub id: String,
    pub material: String,
    pub bbox_min: [f32; 2],
    pub bbox_max: [f32; 2],
    pub hp: f32,
    pub max_hp: f32,
    pub broken: bool,
    pub refusal_reason: Option<String>,
    pub dig_range: f32,
}

impl From<&BreachStrip> for BreachView {
    fn from(s: &BreachStrip) -> Self {
        Self {
            id: s.id.clone(),
            material: s.material.clone(),
            bbox_min: s.bbox_min,
            bbox_max: s.bbox_max,
            hp: s.hp,
            max_hp: s.max_hp,
            broken: s.broken,
            refusal_reason: s.refusal_reason.clone(),
            dig_range: s.dig_range,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world() -> BreachWorld {
        BreachWorld::new(vec![
            BreachStrip {
                id: "outer_wall".to_string(),
                bbox_min: [600.0, 16.0],
                bbox_max: [664.0, 96.0],
                material: "concrete_soft".to_string(),
                max_hp: 60.0,
                hp: 60.0,
                hardness: 20.0,
                dig_range: 48.0,
                refusal_reason: None,
                broken: false,
            },
            BreachStrip {
                id: "anchor".to_string(),
                bbox_min: [800.0, 16.0],
                bbox_max: [832.0, 96.0],
                material: "metal_nohook".to_string(),
                max_hp: 999.0,
                hp: 999.0,
                hardness: 1.0,
                dig_range: 48.0,
                refusal_reason: Some("metal_nohook".to_string()),
                broken: false,
            },
        ])
    }

    #[test]
    fn out_of_range_refuses() {
        let mut w = world();
        let outcome = try_dig(
            &mut w,
            DigRequest {
                origin: [10.0, 32.0],
                aim: [1.0, 0.0],
                explicit_target: None,
            },
        );
        assert_eq!(outcome.refused_reason(), Some("out_of_range"));
        assert_eq!(w.get("outer_wall").unwrap().hp, 60.0);
    }

    #[test]
    fn metal_nohook_refuses_with_material_label() {
        let mut w = world();
        let outcome = try_dig(
            &mut w,
            DigRequest {
                origin: [820.0, 32.0],
                aim: [1.0, 0.0],
                explicit_target: None,
            },
        );
        assert_eq!(outcome.refused_reason(), Some("material_not_diggable"));
        assert_eq!(w.get("anchor").unwrap().hp, 999.0);
    }

    #[test]
    fn carving_takes_three_attempts_for_default_breach() {
        let mut w = world();
        for i in 0..3 {
            let outcome = try_dig(
                &mut w,
                DigRequest {
                    origin: [620.0, 32.0],
                    aim: [1.0, 0.0],
                    explicit_target: None,
                },
            );
            assert!(outcome.is_carved(), "attempt {i} expected carve");
        }
        assert!(w.is_broken("outer_wall"));
        let final_attempt = try_dig(
            &mut w,
            DigRequest {
                origin: [620.0, 32.0],
                aim: [1.0, 0.0],
                explicit_target: None,
            },
        );
        assert_eq!(final_attempt.refused_reason(), Some("already_broken"));
    }

    #[test]
    fn dig_picks_nearest_in_range_strip() {
        let mut w = world();
        let outcome = try_dig(
            &mut w,
            DigRequest {
                origin: [820.0, 32.0],
                aim: [1.0, 0.0],
                explicit_target: None,
            },
        );
        // The anchor strip is the only one in range at x=820, so the picker returns it.
        match outcome {
            DigOutcome::Refused { reason, strip_id, .. } => {
                assert_eq!(reason, "material_not_diggable");
                assert_eq!(strip_id.as_deref(), Some("anchor"));
            }
            DigOutcome::Carved { .. } => panic!("expected anchor refusal, got carved"),
        }
    }

    #[test]
    fn explicit_target_routes_correctly() {
        let mut w = world();
        let outcome = try_dig(
            &mut w,
            DigRequest {
                origin: [620.0, 32.0],
                aim: [1.0, 0.0],
                explicit_target: Some("anchor".to_string()),
            },
        );
        assert_eq!(outcome.refused_reason(), Some("out_of_range"));
    }

    #[test]
    fn unknown_target_refuses() {
        let mut w = world();
        let outcome = try_dig(
            &mut w,
            DigRequest {
                origin: [620.0, 32.0],
                aim: [1.0, 0.0],
                explicit_target: Some("does_not_exist".to_string()),
            },
        );
        assert_eq!(outcome.refused_reason(), Some("unknown_target"));
    }

    #[test]
    fn reset_restores_hp_and_clears_broken() {
        let mut w = world();
        for _ in 0..3 {
            let _ = try_dig(
                &mut w,
                DigRequest {
                    origin: [620.0, 32.0],
                    aim: [1.0, 0.0],
                    explicit_target: None,
                },
            );
        }
        assert!(w.is_broken("outer_wall"));
        w.reset();
        let s = w.get("outer_wall").unwrap();
        assert_eq!(s.hp, s.max_hp);
        assert!(!s.broken);
    }

    #[test]
    fn broken_map_is_consistent() {
        let mut w = world();
        let m = w.broken_map();
        assert_eq!(m.get("outer_wall"), Some(&false));
        assert_eq!(m.get("anchor"), Some(&false));
        for _ in 0..3 {
            let _ = try_dig(
                &mut w,
                DigRequest {
                    origin: [620.0, 32.0],
                    aim: [1.0, 0.0],
                    explicit_target: None,
                },
            );
        }
        let m = w.broken_map();
        assert_eq!(m.get("outer_wall"), Some(&true));
    }

    #[test]
    fn checksum_changes_when_strip_is_carved() {
        let mut w = world();
        let before = w.checksum_bytes();
        let _ = try_dig(
            &mut w,
            DigRequest {
                origin: [620.0, 32.0],
                aim: [1.0, 0.0],
                explicit_target: None,
            },
        );
        let after = w.checksum_bytes();
        assert_ne!(before, after);
    }
}
