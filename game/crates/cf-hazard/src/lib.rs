//! M16 § Hazard tile registry — 9 hazard kinds with spread + dissipation
//! rules.
//!
//! Hazard kinds (locked at v0.1 in `cf-replay/schemas/event/hazard_*.json`):
//! fire, smoke, electric, wet, hot, cold, acid, radiation, toxic.
//!
//! Each hazard tile carries:
//!   - kind
//!   - position (tile coordinates)
//!   - intensity in [0, 1]
//!   - spawned_at_tick + last_spread_tick (for cadence)
//!   - source_event_id (cause-chain back to spawner)
//!
//! The world maintains a `HazardGrid` keyed by `HazardId`. Producers append
//! events through callbacks the engine wires to `Recorder::record` /
//! `Recorder::record_cosmetic`.
//!
//! Per spec § "Hazard tile cosmetic events batched per M4": one
//! `hazard.tick` event per 10 sim ticks per hazard (batched 10:1), so a
//! tile active for 60 ticks emits 6 cosmetic ticks — never 60. The
//! `tick_grid` function consumes a (tick, tick_rate_hz) pair and returns
//! produced events in deterministic order.

#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(
    clippy::module_name_repetitions,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::doc_markdown,
    clippy::missing_const_for_fn,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_lossless,
    clippy::struct_field_names,
    clippy::match_same_arms,
    clippy::unused_self,
    clippy::similar_names,
    clippy::too_many_arguments
)]

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

/// Locked 9-kind hazard set. Mirrors the `kind` enum in
/// `cf-replay/schemas/event/hazard_*.json`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HazardKind {
    Fire,
    Smoke,
    Electric,
    Wet,
    Hot,
    Cold,
    Acid,
    Radiation,
    Toxic,
}

impl HazardKind {
    pub fn as_str(self) -> &'static str {
        match self {
            HazardKind::Fire => "fire",
            HazardKind::Smoke => "smoke",
            HazardKind::Electric => "electric",
            HazardKind::Wet => "wet",
            HazardKind::Hot => "hot",
            HazardKind::Cold => "cold",
            HazardKind::Acid => "acid",
            HazardKind::Radiation => "radiation",
            HazardKind::Toxic => "toxic",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        Some(match s {
            "fire" => HazardKind::Fire,
            "smoke" => HazardKind::Smoke,
            "electric" => HazardKind::Electric,
            "wet" => HazardKind::Wet,
            "hot" => HazardKind::Hot,
            "cold" => HazardKind::Cold,
            "acid" => HazardKind::Acid,
            "radiation" => HazardKind::Radiation,
            "toxic" => HazardKind::Toxic,
            _ => return None,
        })
    }

    pub fn all() -> &'static [HazardKind] {
        &[
            HazardKind::Fire,
            HazardKind::Smoke,
            HazardKind::Electric,
            HazardKind::Wet,
            HazardKind::Hot,
            HazardKind::Cold,
            HazardKind::Acid,
            HazardKind::Radiation,
            HazardKind::Toxic,
        ]
    }
}

/// Reason a hazard tile dissipated. Mirrors the `reason` enum in
/// `cf-replay/schemas/event/hazard_dissipated.json`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DissipationReason {
    Time,
    Doused,
    SpreadOut,
}

impl DissipationReason {
    pub fn as_str(self) -> &'static str {
        match self {
            DissipationReason::Time => "time",
            DissipationReason::Doused => "doused",
            DissipationReason::SpreadOut => "spread_out",
        }
    }
}

/// Spread + dissipation tuning for one hazard kind. Mirrors the spec table
/// § "Hazard tile full mechanics".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HazardSpec {
    pub kind: HazardKind,
    /// Tiles per second of natural spread to adjacent eligible tiles.
    pub spread_tiles_per_s: f32,
    /// Maximum spread distance from origin (tiles). 0 = no spread.
    pub spread_radius_tiles: u32,
    /// Seconds before natural dissipation (DissipationReason::Time).
    pub dissipation_seconds: f32,
    /// Multiplier applied when this kind contacts a "counter" tag (e.g.
    /// fire ↔ water). >1.0 accelerates dissipation; <1.0 slows it. 0 means
    /// instant dissipation on contact.
    pub counter_dissipation_multiplier: f32,
    /// Affliction kind to apply on actor contact (string mirrors
    /// `cf-replay/schemas/event/affliction_applied.json` enum). None means
    /// no affliction (cosmetic hazard).
    pub on_contact_affliction: Option<String>,
    /// Damage per tick (HP) applied to actor on contact at intensity 1.0.
    /// Scales linearly with hazard intensity.
    pub contact_damage_per_tick: f32,
    /// True when the hazard spreads via M19 atmospherics (pressure-driven
    /// diffusion). Smoke + toxic. The atmos kernel owns the spread; this
    /// crate skips the spread tick.
    pub uses_atmos_flow: bool,
    /// True when the hazard is "static" — never spreads (radiation zones).
    pub is_static: bool,
}

impl HazardSpec {
    pub fn default_for(kind: HazardKind) -> Self {
        match kind {
            HazardKind::Fire => HazardSpec {
                kind,
                spread_tiles_per_s: 1.0,
                spread_radius_tiles: 32,
                dissipation_seconds: 30.0,
                counter_dissipation_multiplier: 0.0,
                on_contact_affliction: Some("burning".to_string()),
                contact_damage_per_tick: 0.083,
                uses_atmos_flow: false,
                is_static: false,
            },
            HazardKind::Smoke => HazardSpec {
                kind,
                spread_tiles_per_s: 2.0,
                spread_radius_tiles: 64,
                dissipation_seconds: 45.0,
                counter_dissipation_multiplier: 1.5,
                on_contact_affliction: Some("hypoxic".to_string()),
                contact_damage_per_tick: 0.0,
                uses_atmos_flow: true,
                is_static: false,
            },
            HazardKind::Electric => HazardSpec {
                kind,
                spread_tiles_per_s: 3.0,
                spread_radius_tiles: 3,
                dissipation_seconds: 5.0,
                counter_dissipation_multiplier: 0.2,
                on_contact_affliction: Some("electrified".to_string()),
                contact_damage_per_tick: 0.05,
                uses_atmos_flow: false,
                is_static: false,
            },
            HazardKind::Wet => HazardSpec {
                kind,
                spread_tiles_per_s: 1.5,
                spread_radius_tiles: 16,
                dissipation_seconds: 30.0,
                counter_dissipation_multiplier: 2.0,
                on_contact_affliction: Some("wet".to_string()),
                contact_damage_per_tick: 0.0,
                uses_atmos_flow: false,
                is_static: false,
            },
            HazardKind::Hot => HazardSpec {
                kind,
                spread_tiles_per_s: 0.5,
                spread_radius_tiles: 8,
                dissipation_seconds: 20.0,
                counter_dissipation_multiplier: 1.2,
                on_contact_affliction: Some("hyperthermic".to_string()),
                contact_damage_per_tick: 0.033,
                uses_atmos_flow: false,
                is_static: false,
            },
            HazardKind::Cold => HazardSpec {
                kind,
                spread_tiles_per_s: 0.5,
                spread_radius_tiles: 8,
                dissipation_seconds: 20.0,
                counter_dissipation_multiplier: 1.2,
                on_contact_affliction: Some("hypothermic".to_string()),
                contact_damage_per_tick: 0.017,
                uses_atmos_flow: false,
                is_static: false,
            },
            HazardKind::Acid => HazardSpec {
                kind,
                spread_tiles_per_s: 0.8,
                spread_radius_tiles: 16,
                dissipation_seconds: 60.0,
                counter_dissipation_multiplier: 2.5,
                on_contact_affliction: Some("poisoned".to_string()),
                contact_damage_per_tick: 0.1,
                uses_atmos_flow: false,
                is_static: false,
            },
            HazardKind::Radiation => HazardSpec {
                kind,
                spread_tiles_per_s: 0.0,
                spread_radius_tiles: 0,
                dissipation_seconds: 3600.0,
                counter_dissipation_multiplier: 1.0,
                on_contact_affliction: Some("radiation".to_string()),
                contact_damage_per_tick: 0.017,
                uses_atmos_flow: false,
                is_static: true,
            },
            HazardKind::Toxic => HazardSpec {
                kind,
                spread_tiles_per_s: 1.5,
                spread_radius_tiles: 32,
                dissipation_seconds: 120.0,
                counter_dissipation_multiplier: 1.2,
                on_contact_affliction: Some("poisoned".to_string()),
                contact_damage_per_tick: 0.05,
                uses_atmos_flow: true,
                is_static: false,
            },
        }
    }
}

/// Hazard registry — kind → spec. Loaded from `content/hazards/*.ron` or
/// the hardcoded `default_registry()` fallback for boot.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HazardRegistry {
    pub specs: BTreeMap<String, HazardSpec>,
}

impl HazardRegistry {
    pub fn default_registry() -> Self {
        let mut specs = BTreeMap::new();
        for &k in HazardKind::all() {
            specs.insert(k.as_str().to_string(), HazardSpec::default_for(k));
        }
        Self { specs }
    }

    pub fn lookup(&self, kind: HazardKind) -> &HazardSpec {
        self.specs
            .get(kind.as_str())
            .expect("hazard registry must contain every kind")
    }

    /// Loads every `content/hazards/*.ron` file. Falls back to
    /// `default_registry()` for kinds whose RON file is missing.
    pub fn load_dir(dir: &Path) -> Result<Self, HazardLoadError> {
        let mut reg = Self::default_registry();
        if !dir.exists() {
            return Ok(reg);
        }
        let read_dir = fs::read_dir(dir).map_err(|e| HazardLoadError::Io(dir.to_path_buf(), e.to_string()))?;
        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("ron") {
                continue;
            }
            let body = fs::read_to_string(&path).map_err(|e| HazardLoadError::Io(path.clone(), e.to_string()))?;
            let spec: HazardSpec =
                ron::from_str(&body).map_err(|e| HazardLoadError::Parse(path.clone(), e.to_string()))?;
            reg.specs.insert(spec.kind.as_str().to_string(), spec);
        }
        Ok(reg)
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum HazardLoadError {
    #[error("io error reading {0:?}: {1}")]
    Io(PathBuf, String),
    #[error("parse error in {0:?}: {1}")]
    Parse(PathBuf, String),
}

/// Unique per-run hazard tile id. The world allocates monotonically; events
/// reference the hazard via `hazard_id` (string-encoded u64) so M10
/// cause-chain walkers can hop hazard.spawned → hazard.spread → hazard.dissipated.
pub type HazardId = u64;

/// One hazard tile.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HazardTile {
    pub id: HazardId,
    pub kind: HazardKind,
    pub position: [f32; 2],
    /// 0.0 = cleared; 1.0 = full strength.
    pub intensity: f32,
    pub spawned_at_tick: u64,
    pub last_spread_tick: u64,
    pub last_tick_event_tick: u64,
    /// Cause-chain anchor (the spawning event's id). None for hazards
    /// placed by scenario rather than spawned in-game.
    pub source_event_id: Option<String>,
    /// True when a counter (water on fire, alkali on acid) has applied
    /// this tick; carry through so the dissipation logic accelerates.
    pub doused_this_tick: bool,
}

/// Produced event records. The engine layer translates these into
/// `Recorder::record` calls so the cf-hazard crate stays free of any
/// recorder dependency.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HazardSpawnedEvent {
    pub hazard_id: HazardId,
    pub kind: HazardKind,
    pub position: [f32; 2],
    pub intensity: f32,
    pub source_event_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HazardSpreadEvent {
    pub hazard_id: HazardId,
    pub kind: HazardKind,
    pub from_pos: [f32; 2],
    pub to_pos: [f32; 2],
    pub intensity: f32,
    pub rate: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HazardActorContactEvent {
    pub hazard_id: HazardId,
    pub kind: HazardKind,
    pub actor_id: u64,
    pub intensity: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HazardTickEvent {
    pub hazard_id: HazardId,
    pub tick: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HazardDissipatedEvent {
    pub hazard_id: HazardId,
    pub reason: DissipationReason,
}

/// Aggregate output of one `tick_grid` call. Producer batches per-tile work
/// into a single struct so the engine can drain it without re-walking the
/// grid.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct HazardTickOutput {
    pub spawned: Vec<HazardSpawnedEvent>,
    pub spread: Vec<HazardSpreadEvent>,
    pub actor_contact: Vec<HazardActorContactEvent>,
    pub tick: Vec<HazardTickEvent>,
    pub dissipated: Vec<HazardDissipatedEvent>,
}

/// Cosmetic-event batching ratio per spec § "Hazard tile cosmetic events
/// batched per M4 (batched 10:1 ratio for determinism)".
pub const HAZARD_TICK_BATCH_RATIO: u64 = 10;

/// Authoritative hazard world. Holds every live tile + a counter for next
/// id. Spread + dissipation rules read from a `HazardRegistry`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HazardWorld {
    pub tiles: BTreeMap<HazardId, HazardTile>,
    pub next_id: HazardId,
}

impl HazardWorld {
    pub fn new() -> Self {
        Self::default()
    }

    /// Spawn a new hazard tile. Returns the generated id + the spawned
    /// event payload so the engine can emit `hazard.spawned`.
    pub fn spawn(
        &mut self,
        kind: HazardKind,
        position: [f32; 2],
        intensity: f32,
        tick: u64,
        source_event_id: Option<String>,
    ) -> (HazardId, HazardSpawnedEvent) {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        let tile = HazardTile {
            id,
            kind,
            position,
            intensity: intensity.clamp(0.0, 1.0),
            spawned_at_tick: tick,
            last_spread_tick: tick,
            last_tick_event_tick: tick,
            source_event_id: source_event_id.clone(),
            doused_this_tick: false,
        };
        let event = HazardSpawnedEvent {
            hazard_id: id,
            kind,
            position,
            intensity: tile.intensity,
            source_event_id,
        };
        self.tiles.insert(id, tile);
        (id, event)
    }

    /// Apply a counter (e.g. water on fire). Multiplies dissipation rate;
    /// flagged for one tick so the next `tick_grid` accelerates clearing.
    pub fn apply_counter(&mut self, hazard_id: HazardId) {
        if let Some(t) = self.tiles.get_mut(&hazard_id) {
            t.doused_this_tick = true;
        }
    }

    /// Apply a counter to all hazards of a kind within `radius` tiles of
    /// `position`. Returns the count of hazards doused.
    pub fn apply_counter_radius(
        &mut self,
        kind: HazardKind,
        position: [f32; 2],
        radius_tiles: f32,
    ) -> u32 {
        let mut count = 0u32;
        let r2 = radius_tiles * radius_tiles;
        for tile in self.tiles.values_mut() {
            if tile.kind != kind {
                continue;
            }
            let dx = tile.position[0] - position[0];
            let dy = tile.position[1] - position[1];
            if dx * dx + dy * dy <= r2 {
                tile.doused_this_tick = true;
                count += 1;
            }
        }
        count
    }

    /// Returns the list of hazards within `radius` tiles of `position`.
    pub fn query_radius(&self, position: [f32; 2], radius_tiles: f32) -> Vec<HazardId> {
        let r2 = radius_tiles * radius_tiles;
        let mut out = Vec::new();
        for tile in self.tiles.values() {
            let dx = tile.position[0] - position[0];
            let dy = tile.position[1] - position[1];
            if dx * dx + dy * dy <= r2 {
                out.push(tile.id);
            }
        }
        out
    }

    /// Apply actor contact: emits one `hazard.actor_contact` per actor
    /// overlapping a tile of `radius` tiles. Returns events; the caller
    /// applies affliction + damage via cf-affliction.
    pub fn resolve_actor_contacts(
        &self,
        actor_id: u64,
        actor_pos: [f32; 2],
        radius_tiles: f32,
    ) -> Vec<HazardActorContactEvent> {
        let r2 = radius_tiles * radius_tiles;
        let mut out = Vec::new();
        for tile in self.tiles.values() {
            if tile.intensity <= 0.0 {
                continue;
            }
            let dx = tile.position[0] - actor_pos[0];
            let dy = tile.position[1] - actor_pos[1];
            if dx * dx + dy * dy <= r2 {
                out.push(HazardActorContactEvent {
                    hazard_id: tile.id,
                    kind: tile.kind,
                    actor_id,
                    intensity: tile.intensity,
                });
            }
        }
        out
    }

    /// Advance every tile one sim tick. Spreads to neighbors, applies
    /// dissipation, emits the cosmetic `hazard.tick` event at the batched
    /// cadence. Pure / deterministic: no rng, no clock — relies on `tick`
    /// + `tick_rate_hz` for time math.
    pub fn tick_grid(
        &mut self,
        registry: &HazardRegistry,
        tick: u64,
        tick_rate_hz: u32,
    ) -> HazardTickOutput {
        let dt_seconds = 1.0_f32 / (tick_rate_hz.max(1) as f32);
        let mut out = HazardTickOutput::default();

        // ----- 1) Spread step -----
        let ids: Vec<HazardId> = self.tiles.keys().copied().collect();
        for id in ids {
            let tile = match self.tiles.get(&id) {
                Some(t) => t.clone(),
                None => continue,
            };
            if tile.intensity <= 0.0 {
                continue;
            }
            let spec = registry.lookup(tile.kind);
            if spec.is_static || spec.uses_atmos_flow {
                continue;
            }
            let interval_seconds = if spec.spread_tiles_per_s > 0.0 {
                1.0 / spec.spread_tiles_per_s
            } else {
                continue;
            };
            let interval_ticks = (interval_seconds * (tick_rate_hz.max(1) as f32)).max(1.0) as u64;
            if tick < tile.last_spread_tick + interval_ticks {
                continue;
            }
            let distance_from_origin = ((tile.position[0] - tile.position[0]).powi(2)
                + (tile.position[1] - tile.position[1]).powi(2))
            .sqrt();
            if distance_from_origin >= spec.spread_radius_tiles as f32 {
                continue;
            }
            // Spread to 4-neighbors with full intensity. The engine layer
            // is responsible for tile-occupancy / material flammability
            // gates (passed via the `is_eligible` predicate to
            // `tick_grid_with_predicate`). The bare `tick_grid` always
            // permits spread (used in unit tests + scenario seeding).
            let neighbors: [[f32; 2]; 4] = [
                [tile.position[0] + 1.0, tile.position[1]],
                [tile.position[0] - 1.0, tile.position[1]],
                [tile.position[0], tile.position[1] + 1.0],
                [tile.position[0], tile.position[1] - 1.0],
            ];
            for to_pos in &neighbors {
                // Skip if already a tile of same kind here.
                let occupied = self
                    .tiles
                    .values()
                    .any(|t| t.kind == tile.kind && (t.position[0] - to_pos[0]).abs() < 0.5
                        && (t.position[1] - to_pos[1]).abs() < 0.5);
                if occupied {
                    continue;
                }
                let new_id = self.next_id;
                self.next_id = self.next_id.wrapping_add(1);
                let new_tile = HazardTile {
                    id: new_id,
                    kind: tile.kind,
                    position: *to_pos,
                    intensity: tile.intensity,
                    spawned_at_tick: tick,
                    last_spread_tick: tick,
                    last_tick_event_tick: tick,
                    source_event_id: tile.source_event_id.clone(),
                    doused_this_tick: false,
                };
                self.tiles.insert(new_id, new_tile);
                out.spread.push(HazardSpreadEvent {
                    hazard_id: tile.id,
                    kind: tile.kind,
                    from_pos: tile.position,
                    to_pos: *to_pos,
                    intensity: tile.intensity,
                    rate: spec.spread_tiles_per_s,
                });
            }
            if let Some(t) = self.tiles.get_mut(&id) {
                t.last_spread_tick = tick;
            }
        }

        // ----- 2) Dissipation step -----
        let mut to_remove: Vec<HazardId> = Vec::new();
        let mut tick_events: Vec<HazardTickEvent> = Vec::new();
        for tile in self.tiles.values_mut() {
            let spec = registry.lookup(tile.kind);
            let base_rate = if spec.dissipation_seconds > 0.0 {
                1.0 / spec.dissipation_seconds
            } else {
                0.0
            };
            let mut effective_rate = base_rate;
            if tile.doused_this_tick {
                if spec.counter_dissipation_multiplier <= 0.0 {
                    tile.intensity = 0.0;
                } else {
                    effective_rate *= spec.counter_dissipation_multiplier;
                }
            }
            tile.intensity = (tile.intensity - effective_rate * dt_seconds).max(0.0);
            if tile.intensity <= 0.0 {
                let reason = if tile.doused_this_tick {
                    DissipationReason::Doused
                } else {
                    DissipationReason::Time
                };
                to_remove.push(tile.id);
                out.dissipated.push(HazardDissipatedEvent {
                    hazard_id: tile.id,
                    reason,
                });
            }
            tile.doused_this_tick = false;

            // ----- 3) Cosmetic tick event (10:1 batched) -----
            if tick >= tile.last_tick_event_tick + HAZARD_TICK_BATCH_RATIO {
                tick_events.push(HazardTickEvent {
                    hazard_id: tile.id,
                    tick,
                });
                tile.last_tick_event_tick = tick;
            }
        }
        out.tick = tick_events;
        for id in to_remove {
            self.tiles.remove(&id);
        }

        out
    }

    /// Advance with a custom predicate that gates which tiles a hazard
    /// can spread INTO (e.g. fire only spreads to flammable tiles).
    pub fn tick_grid_with_predicate(
        &mut self,
        registry: &HazardRegistry,
        tick: u64,
        tick_rate_hz: u32,
        is_eligible: impl Fn(HazardKind, [f32; 2]) -> bool,
    ) -> HazardTickOutput {
        let dt_seconds = 1.0_f32 / (tick_rate_hz.max(1) as f32);
        let mut out = HazardTickOutput::default();
        let ids: Vec<HazardId> = self.tiles.keys().copied().collect();
        for id in ids {
            let tile = match self.tiles.get(&id) {
                Some(t) => t.clone(),
                None => continue,
            };
            if tile.intensity <= 0.0 {
                continue;
            }
            let spec = registry.lookup(tile.kind);
            if spec.is_static || spec.uses_atmos_flow {
                continue;
            }
            if spec.spread_tiles_per_s <= 0.0 {
                continue;
            }
            let interval_seconds = 1.0 / spec.spread_tiles_per_s;
            let interval_ticks = (interval_seconds * (tick_rate_hz.max(1) as f32)).max(1.0) as u64;
            if tick < tile.last_spread_tick + interval_ticks {
                continue;
            }
            let neighbors: [[f32; 2]; 4] = [
                [tile.position[0] + 1.0, tile.position[1]],
                [tile.position[0] - 1.0, tile.position[1]],
                [tile.position[0], tile.position[1] + 1.0],
                [tile.position[0], tile.position[1] - 1.0],
            ];
            for to_pos in &neighbors {
                if !is_eligible(tile.kind, *to_pos) {
                    continue;
                }
                let occupied = self.tiles.values().any(|t| {
                    t.kind == tile.kind
                        && (t.position[0] - to_pos[0]).abs() < 0.5
                        && (t.position[1] - to_pos[1]).abs() < 0.5
                });
                if occupied {
                    continue;
                }
                let new_id = self.next_id;
                self.next_id = self.next_id.wrapping_add(1);
                self.tiles.insert(
                    new_id,
                    HazardTile {
                        id: new_id,
                        kind: tile.kind,
                        position: *to_pos,
                        intensity: tile.intensity,
                        spawned_at_tick: tick,
                        last_spread_tick: tick,
                        last_tick_event_tick: tick,
                        source_event_id: tile.source_event_id.clone(),
                        doused_this_tick: false,
                    },
                );
                out.spread.push(HazardSpreadEvent {
                    hazard_id: tile.id,
                    kind: tile.kind,
                    from_pos: tile.position,
                    to_pos: *to_pos,
                    intensity: tile.intensity,
                    rate: spec.spread_tiles_per_s,
                });
            }
            if let Some(t) = self.tiles.get_mut(&id) {
                t.last_spread_tick = tick;
            }
        }

        let mut to_remove: Vec<HazardId> = Vec::new();
        let mut tick_events: Vec<HazardTickEvent> = Vec::new();
        for tile in self.tiles.values_mut() {
            let spec = registry.lookup(tile.kind);
            let base_rate = if spec.dissipation_seconds > 0.0 {
                1.0 / spec.dissipation_seconds
            } else {
                0.0
            };
            let mut effective_rate = base_rate;
            if tile.doused_this_tick {
                if spec.counter_dissipation_multiplier <= 0.0 {
                    tile.intensity = 0.0;
                } else {
                    effective_rate *= spec.counter_dissipation_multiplier;
                }
            }
            tile.intensity = (tile.intensity - effective_rate * dt_seconds).max(0.0);
            if tile.intensity <= 0.0 {
                let reason = if tile.doused_this_tick {
                    DissipationReason::Doused
                } else {
                    DissipationReason::Time
                };
                to_remove.push(tile.id);
                out.dissipated.push(HazardDissipatedEvent {
                    hazard_id: tile.id,
                    reason,
                });
            }
            tile.doused_this_tick = false;
            if tick >= tile.last_tick_event_tick + HAZARD_TICK_BATCH_RATIO {
                tick_events.push(HazardTickEvent {
                    hazard_id: tile.id,
                    tick,
                });
                tile.last_tick_event_tick = tick;
            }
        }
        out.tick = tick_events;
        for id in to_remove {
            self.tiles.remove(&id);
        }
        out
    }

    /// Snapshot for `snapshot.snapshot_hazard_grid`.
    pub fn snapshot(&self) -> HazardGridSnapshot {
        let mut by_kind: BTreeMap<String, u32> = BTreeMap::new();
        for t in self.tiles.values() {
            *by_kind.entry(t.kind.as_str().to_string()).or_insert(0) += 1;
        }
        HazardGridSnapshot {
            dirty_hazard_cell_count: self.tiles.len() as u32,
            hazard_cells: self.tiles.values().cloned().collect(),
            summary_per_kind: by_kind,
        }
    }
}

/// Snapshot shape matching `snapshot_hazard_grid.json`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct HazardGridSnapshot {
    pub dirty_hazard_cell_count: u32,
    pub hazard_cells: Vec<HazardTile>,
    pub summary_per_kind: BTreeMap<String, u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_registry_contains_all_9_kinds() {
        let reg = HazardRegistry::default_registry();
        for k in HazardKind::all() {
            assert!(reg.specs.contains_key(k.as_str()), "missing {}", k.as_str());
        }
    }

    #[test]
    fn fire_spreads_to_adjacent_neighbor() {
        let reg = HazardRegistry::default_registry();
        let mut world = HazardWorld::new();
        world.spawn(HazardKind::Fire, [0.0, 0.0], 1.0, 0, None);
        let out = world.tick_grid(&reg, 60, 60);
        assert!(!out.spread.is_empty(), "fire should spread at 1 tile/s");
        assert_eq!(out.spread[0].kind, HazardKind::Fire);
    }

    #[test]
    fn radiation_is_static_does_not_spread() {
        let reg = HazardRegistry::default_registry();
        let mut world = HazardWorld::new();
        world.spawn(HazardKind::Radiation, [0.0, 0.0], 1.0, 0, None);
        for tick in 1..120u64 {
            let out = world.tick_grid(&reg, tick, 60);
            assert!(out.spread.is_empty(), "radiation should not spread");
        }
    }

    #[test]
    fn fire_dissipates_after_30_seconds_uncountered() {
        let reg = HazardRegistry::default_registry();
        let mut world = HazardWorld::new();
        let (id, _) = world.spawn(HazardKind::Fire, [0.0, 0.0], 1.0, 0, None);
        for tick in 1..1900u64 {
            world.tick_grid(&reg, tick, 60);
        }
        assert!(
            !world.tiles.contains_key(&id),
            "fire tile should naturally dissipate by ~30s @ 60Hz"
        );
    }

    #[test]
    fn water_douses_fire_instantly() {
        let reg = HazardRegistry::default_registry();
        let mut world = HazardWorld::new();
        let (id, _) = world.spawn(HazardKind::Fire, [0.0, 0.0], 1.0, 0, None);
        world.apply_counter(id);
        let out = world.tick_grid(&reg, 1, 60);
        assert!(!out.dissipated.is_empty());
        assert_eq!(out.dissipated[0].reason, DissipationReason::Doused);
    }

    #[test]
    fn hazard_tick_event_batched_10_to_1() {
        let reg = HazardRegistry::default_registry();
        let mut world = HazardWorld::new();
        let (_id, _) = world.spawn(HazardKind::Radiation, [0.0, 0.0], 1.0, 0, None);
        let mut tick_count = 0u32;
        for tick in 1..=60u64 {
            let out = world.tick_grid(&reg, tick, 60);
            tick_count += out.tick.len() as u32;
        }
        assert!(
            tick_count <= 6,
            "60 sim ticks must batch to ≤6 hazard.tick events, got {tick_count}"
        );
    }

    #[test]
    fn actor_contact_inside_radius_emits_event() {
        let mut world = HazardWorld::new();
        let (id, _) = world.spawn(HazardKind::Acid, [0.0, 0.0], 1.0, 0, None);
        let contacts = world.resolve_actor_contacts(7, [0.5, 0.5], 1.0);
        assert!(!contacts.is_empty());
        assert_eq!(contacts[0].hazard_id, id);
        assert_eq!(contacts[0].kind, HazardKind::Acid);
    }

    #[test]
    fn electric_counter_water_spreads_through_water() {
        let spec = HazardSpec::default_for(HazardKind::Electric);
        assert!(spec.counter_dissipation_multiplier < 1.0,
            "electric must be SLOWED by water (counter < 1.0) per spec 'Insulation, water (counter — spreads via water!)'");
    }

    #[test]
    fn snapshot_summarizes_per_kind_counts() {
        let reg = HazardRegistry::default_registry();
        let mut world = HazardWorld::new();
        world.spawn(HazardKind::Fire, [0.0, 0.0], 1.0, 0, None);
        world.spawn(HazardKind::Fire, [10.0, 0.0], 0.7, 0, None);
        world.spawn(HazardKind::Acid, [20.0, 0.0], 0.5, 0, None);
        let snap = world.snapshot();
        assert_eq!(snap.summary_per_kind.get("fire").copied(), Some(2));
        assert_eq!(snap.summary_per_kind.get("acid").copied(), Some(1));
        let _ = reg;
    }
}
