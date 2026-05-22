//! RON scenario loader. The full schema lives in `spec/prototype-roadmap.md`
//! Scenario Manifest Schema; M0/M1/M1.5 implement a subset:
//!
//! - M0 ships engine bootstrap (no actors, empty regions).
//! - M1 adds typed `actors[]` entries (player + optional dummies) and a `floor_y`
//!   so `cf-physics` can resolve ground collisions.
//! - M1.5 adds `breaches[]`, `objectives[]`, and per-actor `enemy: ReactiveGuard`
//!   parameters so the micro breach scenario can run end-to-end.

use std::path::Path;

use serde::{Deserialize, Serialize};

use cf_actor::{ActorId, ActorState, Inventory, InventoryItem, ItemSlot, Vec2};
use cf_ai::ReactiveGuardParams;
use cf_equipment::{rifle_preset, RifleState};
use cf_mission::{
    BossState, BranchingPoint, ExtendedObjectiveKind, LossConditions, MissionPhase, Objective, ObjectiveGraph,
    ObjectiveKind, ObjectiveNode, ObjectiveNodeStatus, ObjectiveStatus, PhaseState, Reactor, ReinforcementWave,
};
use cf_terrain::{material_id_from_name, BreachStrip, ChunkedTerrain, MaterialId, TerrainStamp};

#[allow(unused_imports)]
use crate::scenario::*;


/// Mirrors [`cf_physics::ProjectileSnapshot`] but uses serde-friendly
/// types for RON. The engine converts these to runtime snapshots at
/// scenario load via `build_m14d_projectile_snapshot`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioM14dProjectile {
    /// Stable id (must be unique across the pool).
    pub id: u64,
    /// Kind discriminator (`kinetic_rifle` / `explosive_grenade` /
    /// `energy_beam` / `heat_round` / `apfsds_round` / `aps_laser`).
    pub kind: cf_physics::ProjectileKind,
    /// World position (px) at scenario init.
    pub position: (f32, f32),
    /// Velocity in world units per second.
    pub velocity: (f32, f32),
    /// Effective collision radius. Default 1.0.
    #[serde(default = "default_m14d_radius")]
    pub radius: f32,
    /// Scalar mass (kg). Default 0.01.
    #[serde(default = "default_m14d_mass_kg")]
    pub mass_kg: f32,
    /// Owner actor id (0 for base-mounted modules like C-RAM).
    #[serde(default)]
    pub owner_actor_id: u64,
}

/// Drives the per-tick collapse-check pass on a single chunk per
/// VAL-M14E-001..VAL-M14E-018.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioTunnelSpan {
    /// Stable id for diagnostics + replay matching.
    pub id: String,
    /// Chunk coordinate the tunnel ceiling occupies.
    pub chunk_id: (i32, i32),
    /// Pixel-space AABB of the unsupported ceiling region.
    pub bbox_min: (i64, i64),
    pub bbox_max: (i64, i64),
    /// Unsupported tunnel span (pixels). The cave-in roll consumes this
    /// alongside `vibration_modifier`.
    pub unsupported_span_px: u32,
    /// Ceiling thickness (pixels). Used in
    /// `falling_debris_count(span_px, ceiling_thickness)`.
    #[serde(default = "default_ceiling_thickness")]
    pub ceiling_thickness_px: u32,
    /// Vibration modifier (1.0 baseline; 2.0 plasma cutter).
    #[serde(default = "default_vibration_modifier")]
    pub vibration_modifier: f32,
    /// True when the tunnel has at least one anchored support beam
    /// covering the span. At init the integrity field locks the ±8 px
    /// around the beam to integrity 500.
    #[serde(default)]
    pub anchored: bool,
    /// Optional cascade-neighbor chunk ids that should re-run the
    /// integrity pass when this tunnel cave-in fires.
    #[serde(default)]
    pub cascade_neighbors: Vec<(i32, i32)>,
    /// True when a downstream actor should receive cave-in falling
    /// debris (drives the fall_impulse_chain → KnockedDown wiring per
    /// VAL-M14E-027).
    #[serde(default)]
    pub damage_actor_id: Option<u64>,
}

/// authored by the scenario manifest. Drives the per-tick lateral
/// integrity pass + the bulging → crack_advanced → rupture cascade.
/// The chunk's integrity field is shared with the M14E ceiling pass
/// (per VAL-CROSS-005); this row only carries the lateral-axis
/// metadata (wall span, yield strength, topology tag).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LateralWallSpan {
    /// Stable id for diagnostics + replay matching.
    pub id: String,
    /// Chunk coordinate the sidewall occupies.
    pub chunk_id: (i32, i32),
    /// Pixel-space AABB of the lateral wall region.
    pub bbox_min: (i64, i64),
    pub bbox_max: (i64, i64),
    /// Unsupported lateral span (pixels) driving the bulging/rupture
    /// roll.
    pub unsupported_span_px: u32,
    /// Wall thickness (pixels). Drives the falling-debris cone size
    /// on rupture.
    #[serde(default = "default_wall_thickness")]
    pub wall_thickness_px: u32,
    /// Per-material lateral yield strength (concrete=50, brick=30,
    /// steel=200, wood=15, dirt=10). Drives the lateral-pass decay
    /// rate + the pressure-blowout threshold.
    #[serde(default = "default_lateral_yield_strength")]
    pub lateral_yield_strength: u16,
    /// Vibration modifier driving the bulging chance (1.0 baseline).
    #[serde(default = "default_vibration_modifier")]
    pub vibration_modifier: f32,
    /// Lateral cascade neighbors that re-run the integrity pass when
    /// this chunk's rupture fires (VAL-M14F-026).
    #[serde(default)]
    pub cascade_neighbors: Vec<(i32, i32)>,
    /// Optional downstream-actor id that registers submerged / damp
    /// after a dam rupture (VAL-M14F-009) or vacuum exposure after a
    /// sealed-room rupture (VAL-M14F-011).
    #[serde(default)]
    pub downstream_actor_id: Option<u64>,
    /// Optional topology tag — `"mineshaft"` (default integrity-decay
    /// cascade), `"dam"` (drives M15 fluid + sets the downstream
    /// actor's submerged flag), or `"sealed_room"` (drives M19
    /// pressure equalization + M19C vacuum exposure on the actor
    /// inside the sealed room).
    #[serde(default = "default_lateral_topology")]
    pub topology: String,
    /// Initial sealed-room pressure (kPa). Defaults to 101 (Earth
    /// ambient). Used by VAL-M14F-008 / VAL-M14F-011 to compute the
    /// pressure equalization curve through the breach.
    #[serde(default = "default_sealed_room_pressure")]
    pub sealed_room_pressure_kpa: f32,
    /// **VAL-CROSS-024**: opts the lateral wall into the composite-
    /// cascade topology. When `true`, a `terrain.wall_rupture` on this
    /// chunk also drives M14E cave-in emit on every `cascade_neighbors`
    /// chunk that has an `m14e_tunnel_spans` row. The default `false`
    /// keeps standalone mineshafts / dams / sealed-rooms isolated from
    /// the ceiling pass — only explicit dam-above-tunnel / lateral-
    /// adjacent-to-tunnel scenarios opt in. Also flips the per-chunk
    /// `m14f_owns_rupture_emit` flag on the M14E chunk state for this
    /// chunk (when false) → the lateral pass owns the rupture surface
    /// and the M14E cave-in roll is suppressed; setting this `true`
    /// keeps the M14E roll surface available so the composite scenario
    /// can express both ceilings AND walls on the same chunk_id.
    #[serde(default)]
    pub m14e_composite_cascade_allowed: bool,
}

/// Models an actor zone resting against a tile at a steady temperature
/// so the engine can fire the burn / frostbite escalation ladder
/// deterministically.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioThermalZone {
    /// Actor id receiving the thermal contact.
    pub actor_id: u64,
    /// Body zone tag (e.g. `"foot_right"`, `"hand_left"`).
    pub zone: String,
    /// Steady tile temperature in Kelvin. ≥ 320 K = hot ladder, ≤ 260 K
    /// = cold ladder, otherwise safe band (no emit).
    pub temperature_k: f32,
    /// Optional tick at which the dwell counter starts (the actor must
    /// actually be on the tile from `start_tick` onward). Default 0.
    #[serde(default)]
    pub start_tick: u64,
    /// Optional tick at which the contact ends (inclusive). `None`
    /// means the contact persists for the rest of the scenario run.
    #[serde(default)]
    pub end_tick: Option<u64>,
}

/// an actor zone touching a hazardous material (acid / refrigerant /
/// ammonia / chlorine) at constant intensity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioMaterialContact {
    /// Actor id receiving the material contact.
    pub actor_id: u64,
    /// Body zone tag.
    pub zone: String,
    /// Material name (canonical lowercase: `"acid"`, `"refrigerant"`,
    /// etc.).
    pub material: String,
    /// Contact intensity ∈ [0, 1]; passed to
    /// [`cf_material::classify_reaction`].
    #[serde(default = "default_material_intensity")]
    pub intensity: f32,
    /// Tick on which the wound is emitted. The engine fires the
    /// classify_reaction call once on this tick (mirrors a one-frame
    /// chemistry contact). Default 0 = first tick after init.
    #[serde(default)]
    pub fire_tick: u64,
}

