//! M7: Mission director v0.5 — multi-objective `DiGraph<ObjectiveNode>`.
//!
//! Spec § Mission director v0.5 — multi-objective DiGraph with sequential +
//! parallel + branching support. M7 ships a thin graph wrapper around the
//! existing `Objective` Vec; the engine consumes a graph view that exposes
//! which objectives are active given current state.
//!
//! Forward-compat: M25 narrative director will reuse this graph to author
//! storyteller-style scripted missions; the schema we publish here is the
//! M7 v0.5 form (additive at M25).

use serde::{Deserialize, Serialize};

/// baseline. The existing `ObjectiveKind` enum (in lib.rs) is the primary
/// type; this enum captures the additional kinds the director's graph
/// nodes can declare, so scenario manifests can serialize them without
/// disturbing M2 wire shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ExtendedObjectiveKind {
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
        target_zone: [f32; 4],
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

impl ExtendedObjectiveKind {
    pub fn category(&self) -> &'static str {
        match self {
            ExtendedObjectiveKind::KillN { .. } => "kill_n",
            ExtendedObjectiveKind::DefendActor { .. } => "defend_actor_v05",
            ExtendedObjectiveKind::RetrieveItem { .. } => "retrieve_item",
            ExtendedObjectiveKind::PlantItem { .. } => "plant_item",
            ExtendedObjectiveKind::DetectAlarm { .. } => "detect_alarm",
            ExtendedObjectiveKind::SneakStealth { .. } => "sneak_stealth",
            ExtendedObjectiveKind::RescueDowned { .. } => "rescue_downed",
            ExtendedObjectiveKind::BreachContainer { .. } => "breach_container",
            ExtendedObjectiveKind::Optional { .. } => "optional",
            ExtendedObjectiveKind::Branching { .. } => "branching",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObjectiveNode {
    pub id: String,
    pub kind: ExtendedObjectiveKind,
    /// Ids of objective nodes that must complete before this one becomes
    /// active.
    pub depends_on: Vec<String>,
    /// True if this node can be parallelised with its siblings.
    pub parallel: bool,
    /// True if completing this node is optional (mission may win
    /// regardless).
    pub optional: bool,
    /// Branching path label this node belongs to (or "" if not in a
    /// branch).
    pub branch_label: String,
    pub status: ObjectiveNodeStatus,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ObjectiveNodeStatus {
    #[default]
    Pending,
    Active,
    Completed,
    Failed,
    Skipped,
}

impl ObjectiveNodeStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            ObjectiveNodeStatus::Pending => "pending",
            ObjectiveNodeStatus::Active => "active",
            ObjectiveNodeStatus::Completed => "completed",
            ObjectiveNodeStatus::Failed => "failed",
            ObjectiveNodeStatus::Skipped => "skipped",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ObjectiveGraph {
    pub nodes: Vec<ObjectiveNode>,
    pub branches: Vec<BranchingPoint>,
}

impl ObjectiveGraph {
    pub fn push(&mut self, node: ObjectiveNode) {
        self.nodes.push(node);
    }

    pub fn iter(&self) -> impl Iterator<Item = &ObjectiveNode> + '_ {
        self.nodes.iter()
    }

    /// Return active nodes (status == Active OR depends-on satisfied and
    /// status == Pending).
    pub fn active_ids(&self) -> Vec<String> {
        let completed_ids: std::collections::BTreeSet<&str> = self
            .nodes
            .iter()
            .filter(|n| matches!(n.status, ObjectiveNodeStatus::Completed | ObjectiveNodeStatus::Skipped))
            .map(|n| n.id.as_str())
            .collect();
        self.nodes
            .iter()
            .filter(|n| {
                matches!(n.status, ObjectiveNodeStatus::Active)
                    || (matches!(n.status, ObjectiveNodeStatus::Pending)
                        && n.depends_on.iter().all(|d| completed_ids.contains(d.as_str())))
            })
            .map(|n| n.id.clone())
            .collect()
    }

    pub fn mark_completed(&mut self, id: &str) {
        if let Some(n) = self.nodes.iter_mut().find(|n| n.id == id) {
            n.status = ObjectiveNodeStatus::Completed;
        }
    }

    pub fn checksum_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.nodes.len() * 8 + 4);
        out.extend_from_slice(&(self.nodes.len() as u32).to_le_bytes());
        for n in &self.nodes {
            out.push(n.status as u8);
            out.extend_from_slice(&(n.id.len() as u16).to_le_bytes());
            out.extend_from_slice(n.id.as_bytes());
        }
        out
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BranchingPoint {
    pub id: String,
    pub branch_a_id: String,
    pub branch_b_id: String,
    /// `None` until the player picks a branch.
    pub chosen_branch: Option<String>,
    pub offered_tick: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObjectiveBranchedEvent {
    pub branching_point_id: String,
    pub chosen_branch: String,
    pub other_branch: String,
    pub tick: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OptionalOfferedEvent {
    pub objective_id: String,
    pub tick: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_node(id: &str, deps: Vec<&str>) -> ObjectiveNode {
        ObjectiveNode {
            id: id.to_string(),
            kind: ExtendedObjectiveKind::KillN {
                target_class: "rifleman".into(),
                count: 1,
            },
            depends_on: deps.iter().map(std::string::ToString::to_string).collect(),
            parallel: false,
            optional: false,
            branch_label: String::new(),
            status: ObjectiveNodeStatus::Pending,
        }
    }

    #[test]
    fn active_ids_walk_dependencies() {
        let mut g = ObjectiveGraph::default();
        g.push(mk_node("a", vec![]));
        g.push(mk_node("b", vec!["a"]));
        g.push(mk_node("c", vec!["b"]));
        let active = g.active_ids();
        assert_eq!(active, vec!["a".to_string()]);
        g.mark_completed("a");
        let active = g.active_ids();
        assert_eq!(active, vec!["b".to_string()]);
    }
}
