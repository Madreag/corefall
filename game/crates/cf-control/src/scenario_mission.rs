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


#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScenarioMission {
    /// Time limit in ticks (`0` = no limit). At 60 Hz, 5400 = 90 seconds.
    #[serde(default)]
    pub time_limit_ticks: u64,
    #[serde(default = "default_true")]
    pub player_dead_loses: bool,
}

impl ScenarioMission {
    pub fn loss_conditions(&self) -> LossConditions {
        LossConditions {
            player_dead: self.player_dead_loses,
            time_limit_ticks: self.time_limit_ticks,
        }
    }
}

/// the 4-phase pacing parameters consumed by `M7AiWorld.phase`. All
/// three durations default to spec defaults (30s / 60s / 120s).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioPhaseState {
    #[serde(default = "default_setup_seconds")]
    pub setup_seconds: f32,
    #[serde(default = "default_buildup_seconds")]
    pub buildup_seconds: f32,
    #[serde(default = "default_climax_seconds")]
    pub climax_seconds: f32,
}

impl ScenarioPhaseState {
    pub fn build_phase_state(&self) -> PhaseState {
        let mut s = PhaseState::new(0);
        s.setup_seconds = self.setup_seconds.max(0.0);
        s.buildup_seconds = self.buildup_seconds.max(0.0);
        s.climax_seconds = self.climax_seconds.max(0.0);
        s
    }
}

/// one reinforcement wave. Waves trigger when the active phase + the
/// cumulative kill count both match the wave's spec.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioReinforcementWave {
    pub id: String,
    pub phase: ScenarioMissionPhase,
    pub trigger_kill_count: u32,
    pub dropship_zone: (f32, f32),
    #[serde(default = "default_spawn_count")]
    pub spawn_count: u32,
}

impl ScenarioReinforcementWave {
    pub fn build_wave(&self) -> ReinforcementWave {
        let mut w = ReinforcementWave::new(
            self.id.clone(),
            self.phase.into_phase(),
            self.trigger_kill_count,
            [self.dropship_zone.0, self.dropship_zone.1],
        );
        w.spawn_count = self.spawn_count.max(1);
        w
    }
}

/// so RON manifests can author phases without depending on cf-mission.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScenarioMissionPhase {
    Setup,
    Buildup,
    Climax,
    Debrief,
}

impl ScenarioMissionPhase {
    pub fn into_phase(self) -> MissionPhase {
        match self {
            ScenarioMissionPhase::Setup => MissionPhase::Setup,
            ScenarioMissionPhase::Buildup => MissionPhase::Buildup,
            ScenarioMissionPhase::Climax => MissionPhase::Climax,
            ScenarioMissionPhase::Debrief => MissionPhase::Debrief,
        }
    }
}

/// of the mini-boss state. The engine seeds `M7AiWorld.boss` from this
/// at scenario start and routes hits whose `target == actor_id` into
/// `apply_boss_damage`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioBossState {
    pub actor_id: u64,
    pub display_name: String,
    pub max_hp: f32,
    #[serde(default = "default_boss_phase_2_threshold")]
    pub phase_2_hp_threshold: f32,
    #[serde(default = "default_boss_phase_3_threshold")]
    pub phase_3_hp_threshold: f32,
}

impl ScenarioBossState {
    pub fn build_boss_state(&self) -> BossState {
        let mut b = BossState::new(self.actor_id, self.display_name.clone(), self.max_hp.max(0.001));
        b.phase_2_hp_threshold = self.phase_2_hp_threshold.clamp(0.0, 1.0);
        b.phase_3_hp_threshold = self.phase_3_hp_threshold.clamp(0.0, 1.0);
        b
    }
}

