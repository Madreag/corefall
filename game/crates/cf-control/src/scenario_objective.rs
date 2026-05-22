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


/// One M1.5 objective row. Discriminator strings match `cf-mission::ObjectiveKind`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioObjective {
    pub id: String,
    pub kind: ScenarioObjectiveKind,
    #[serde(default)]
    pub optional: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ScenarioObjectiveKind {
    BreachBarrier { target: String },
    NeutralizeActor { target: u64 },
    ReachZone { min: (f32, f32), max: (f32, f32) },
    DefendReactor { target: String },
}

impl ScenarioObjective {
    pub fn into_objective(self) -> Objective {
        let kind = match self.kind {
            ScenarioObjectiveKind::BreachBarrier { target } => ObjectiveKind::BreachBarrier { target },
            ScenarioObjectiveKind::NeutralizeActor { target } => ObjectiveKind::NeutralizeActor { target },
            ScenarioObjectiveKind::ReachZone { min, max } => ObjectiveKind::ReachZone {
                min: [min.0, min.1],
                max: [max.0, max.1],
            },
            ScenarioObjectiveKind::DefendReactor { target } => ObjectiveKind::DefendReactor { target },
        };
        Objective {
            id: self.id,
            kind,
            optional: self.optional,
            status: ObjectiveStatus::Pending,
            progress_milestone_index: 0,
            // M2 re-audit (2026-05-13): new continuous progress + fail_sensor
            // fields. Default progress=0.0 + None fail_sensor; the engine
            // populates progress as the objective advances.
            progress: 0.0,
            fail_sensor: None,
        }
    }
}

/// of the v0.5 objective DiGraph. Nodes carry their `kind` + dependency
/// list + optional/parallel/branch-label flags. Branching points are
/// listed separately so the engine knows which `(branch_a, branch_b)`
/// pairs are mutually exclusive.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScenarioObjectiveGraph {
    #[serde(default)]
    pub nodes: Vec<ScenarioObjectiveGraphNode>,
    #[serde(default)]
    pub branches: Vec<ScenarioObjectiveGraphBranch>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioObjectiveGraphNode {
    pub id: String,
    pub kind: ScenarioExtendedObjectiveKind,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub parallel: bool,
    #[serde(default)]
    pub optional: bool,
    #[serde(default)]
    pub branch_label: String,
}

impl ScenarioObjectiveGraphNode {
    pub fn build_node(&self) -> ObjectiveNode {
        ObjectiveNode {
            id: self.id.clone(),
            kind: self.kind.clone().build_kind(),
            depends_on: self.depends_on.clone(),
            parallel: self.parallel,
            optional: self.optional,
            branch_label: self.branch_label.clone(),
            status: ObjectiveNodeStatus::Pending,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ScenarioExtendedObjectiveKind {
    KillN {
        target_class: String,
        count: u32,
    },
    DefendActor {
        target: u64,
        survive_ticks: u64,
    },
    RetrieveItem {
        item_id: String,
    },
    PlantItem {
        item_id: String,
        target_zone: (f32, f32, f32, f32),
    },
    DetectAlarm {
        alarm_id: String,
    },
    SneakStealth {
        zone_id: String,
        no_alarm_within_ticks: u64,
    },
    RescueDowned {
        target: u64,
    },
    BreachContainer {
        container_id: String,
    },
    Optional {
        inner_id: String,
    },
    Branching {
        branch_a_id: String,
        branch_b_id: String,
    },
}

impl ScenarioExtendedObjectiveKind {
    pub fn build_kind(self) -> ExtendedObjectiveKind {
        match self {
            ScenarioExtendedObjectiveKind::KillN { target_class, count } => {
                ExtendedObjectiveKind::KillN { target_class, count }
            }
            ScenarioExtendedObjectiveKind::DefendActor { target, survive_ticks } => {
                ExtendedObjectiveKind::DefendActor { target, survive_ticks }
            }
            ScenarioExtendedObjectiveKind::RetrieveItem { item_id } => ExtendedObjectiveKind::RetrieveItem { item_id },
            ScenarioExtendedObjectiveKind::PlantItem { item_id, target_zone } => ExtendedObjectiveKind::PlantItem {
                item_id,
                target_zone: [target_zone.0, target_zone.1, target_zone.2, target_zone.3],
            },
            ScenarioExtendedObjectiveKind::DetectAlarm { alarm_id } => ExtendedObjectiveKind::DetectAlarm { alarm_id },
            ScenarioExtendedObjectiveKind::SneakStealth {
                zone_id,
                no_alarm_within_ticks,
            } => ExtendedObjectiveKind::SneakStealth {
                zone_id,
                no_alarm_within_ticks,
            },
            ScenarioExtendedObjectiveKind::RescueDowned { target } => ExtendedObjectiveKind::RescueDowned { target },
            ScenarioExtendedObjectiveKind::BreachContainer { container_id } => {
                ExtendedObjectiveKind::BreachContainer { container_id }
            }
            ScenarioExtendedObjectiveKind::Optional { inner_id } => ExtendedObjectiveKind::Optional { inner_id },
            ScenarioExtendedObjectiveKind::Branching {
                branch_a_id,
                branch_b_id,
            } => ExtendedObjectiveKind::Branching {
                branch_a_id,
                branch_b_id,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioObjectiveGraphBranch {
    pub id: String,
    pub branch_a_id: String,
    pub branch_b_id: String,
    #[serde(default)]
    pub chosen_branch: Option<String>,
    #[serde(default)]
    pub offered_tick: Option<u64>,
}

impl ScenarioObjectiveGraphBranch {
    pub fn build_branch(&self) -> BranchingPoint {
        BranchingPoint {
            id: self.id.clone(),
            branch_a_id: self.branch_a_id.clone(),
            branch_b_id: self.branch_b_id.clone(),
            chosen_branch: self.chosen_branch.clone(),
            offered_tick: self.offered_tick,
        }
    }
}

impl ScenarioObjectiveGraph {
    pub fn build_graph(&self) -> ObjectiveGraph {
        let mut g = ObjectiveGraph::default();
        for node in &self.nodes {
            g.push(node.build_node());
        }
        for branch in &self.branches {
            g.branches.push(branch.build_branch());
        }
        g
    }
}

