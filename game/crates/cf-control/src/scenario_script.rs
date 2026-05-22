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


/// tick against `tick`; on the matching tick it patches `pending_intent`
/// on the player actor with the provided overrides (aim / fire /
/// ammo_kind) before the actor sim runs.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ScenarioScriptStep {
    /// Tick on which to inject the intent. The engine drives this
    /// exactly once; subsequent ticks see `clear_edges` clear the
    /// edge-triggered fields.
    pub tick: u64,
    /// Optional aim vector `(x, y)` to assign to `pending_intent.aim`
    /// before the fire/reload edges fire. The actor sim normalizes the
    /// vector so any non-zero direction is accepted.
    #[serde(default)]
    pub aim: Option<(f32, f32)>,
    /// True = set `pending_intent.fire = true` and `fire_held = true`.
    /// One-shot edge — cleared by `ControlIntent::clear_edges` at end-of-tick.
    #[serde(default)]
    pub fire: bool,
    /// Per-shot ammo kind override (mirrors
    /// `cfctl.act.player.fire { ammo_kind: ... }`). Accepted snake_case
    /// values: `regular` / `tracer` / `high_explosive` / `pellet` /
    /// `heat` / `apfsds`. Unknown values are dropped at scenario-load
    /// time so a typo doesn't silently fail at runtime.
    #[serde(default)]
    pub ammo_kind: Option<String>,
    /// Optional `pending_intent.reload = true` edge.
    #[serde(default)]
    pub reload: bool,
}

impl ScenarioScriptStep {
    /// Returns `None` for empty / unknown values so the engine falls back
    /// to the weapon's `RifleSpec::primary_round`.
    pub fn resolved_ammo_kind(&self) -> Option<cf_equipment::RoundKind> {
        self.ammo_kind
            .as_deref()
            .and_then(cf_equipment::RoundKind::from_str_snake)
    }
}

