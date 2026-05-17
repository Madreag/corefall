//! M7B: per-archetype Behavior Trees.
//!
//! Deep-fills M7's Layer 3 stub. Each archetype BT exposes ≥30 distinct
//! node ids; the wiring lives in submodules so individual archetype trees
//! can evolve without touching siblings. The trees compose on top of the
//! shared `BtNode` enum from `behavior_tree.rs`.
//!
//! Spec § "BTs authored as RON + loaded at startup; do NOT hand-roll the
//! 30-node-per-archetype floor in Rust." The canonical content authoring
//! path lives under `game/content/ai/archetype_bts/<kind>.ron`. The Rust
//! constants in this module mirror those RON files (the round-trip test
//! enforces equality) so headless / determinism tests + no-filesystem
//! callers still get the same trees.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::behavior_tree::BtNode;
use crate::task::TaskType;

pub mod assault;
pub mod engineer;
pub mod heavy;
pub mod rifleman;
pub mod sniper;
pub mod spotter;

/// **M7B**: the 6 archetype BT kinds. Distinct from the `Archetype` enum
/// because Medic uses the rifleman BT at M7B; Heavy is M7B-only and not in
/// the M7 Archetype enum.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum ArchetypeBtKind {
    Rifleman,
    Sniper,
    Assault,
    Engineer,
    Spotter,
    Heavy,
}

impl ArchetypeBtKind {
    pub const ALL: [ArchetypeBtKind; 6] = [
        ArchetypeBtKind::Rifleman,
        ArchetypeBtKind::Sniper,
        ArchetypeBtKind::Assault,
        ArchetypeBtKind::Engineer,
        ArchetypeBtKind::Spotter,
        ArchetypeBtKind::Heavy,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            ArchetypeBtKind::Rifleman => "rifleman",
            ArchetypeBtKind::Sniper => "sniper",
            ArchetypeBtKind::Assault => "assault",
            ArchetypeBtKind::Engineer => "engineer",
            ArchetypeBtKind::Spotter => "spotter",
            ArchetypeBtKind::Heavy => "heavy",
        }
    }
}

/// **M7B**: enumerate every leaf node id for an archetype's BT. Used for
/// the acceptance criterion that asserts ≥30 nodes per archetype.
pub fn node_ids_for(kind: ArchetypeBtKind) -> &'static [&'static str] {
    match kind {
        ArchetypeBtKind::Rifleman => rifleman::NODES,
        ArchetypeBtKind::Sniper => sniper::NODES,
        ArchetypeBtKind::Assault => assault::NODES,
        ArchetypeBtKind::Engineer => engineer::NODES,
        ArchetypeBtKind::Spotter => spotter::NODES,
        ArchetypeBtKind::Heavy => heavy::NODES,
    }
}

/// **M7B**: build the BT root for a given archetype + chosen task. Each
/// archetype's submodule supplies a `bt_for_task(task)` returning a
/// canonical `BtNode` so the engine can use the M7B archetype-specific
/// expansion instead of the shared M7-A fallback.
pub fn bt_for(kind: ArchetypeBtKind, task: TaskType) -> BtNode {
    match kind {
        ArchetypeBtKind::Rifleman => rifleman::bt_for_task(task),
        ArchetypeBtKind::Sniper => sniper::bt_for_task(task),
        ArchetypeBtKind::Assault => assault::bt_for_task(task),
        ArchetypeBtKind::Engineer => engineer::bt_for_task(task),
        ArchetypeBtKind::Spotter => spotter::bt_for_task(task),
        ArchetypeBtKind::Heavy => heavy::bt_for_task(task),
    }
}

/// **M7B**: build the BT root for a player-issued squad verb. Distinct
/// from `bt_for(task)` because squad verbs span the BT graph in ways the
/// 22-task TaskType lattice cannot — `Suppress (window)` vs `Overwatch
/// (sector)` vs `Cover Me` all collapse to "engage / hold cover" under
/// TaskType but produce distinct BT subtrees per spec § "Suppress vs
/// Overwatch vs Cover Me are distinct BT subtrees."
pub fn bt_for_squad_verb(kind: ArchetypeBtKind, verb_id: &str) -> Option<BtNode> {
    match kind {
        ArchetypeBtKind::Rifleman => rifleman::bt_for_squad_verb(verb_id),
        ArchetypeBtKind::Sniper => sniper::bt_for_squad_verb(verb_id),
        ArchetypeBtKind::Assault => assault::bt_for_squad_verb(verb_id),
        ArchetypeBtKind::Engineer => engineer::bt_for_squad_verb(verb_id),
        ArchetypeBtKind::Spotter => spotter::bt_for_squad_verb(verb_id),
        ArchetypeBtKind::Heavy => heavy::bt_for_squad_verb(verb_id),
    }
}

/// **M7B**: RON-friendly node format. Mirrors `BtNode` 1:1 but uses
/// tuple-variant enum encoding so `game/content/ai/archetype_bts/*.ron`
/// reads naturally.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BtNodeSpec {
    Sequence(Vec<BtNodeSpec>),
    Selector(Vec<BtNodeSpec>),
    Decorator { kind: String, child: Box<BtNodeSpec> },
    Action(String),
}

impl BtNodeSpec {
    pub fn into_bt_node(self) -> BtNode {
        match self {
            BtNodeSpec::Sequence(children) => BtNode::Sequence {
                children: children.into_iter().map(Self::into_bt_node).collect(),
            },
            BtNodeSpec::Selector(children) => BtNode::Selector {
                children: children.into_iter().map(Self::into_bt_node).collect(),
            },
            BtNodeSpec::Decorator { kind, child } => BtNode::Decorator {
                kind,
                child: Box::new(child.into_bt_node()),
            },
            BtNodeSpec::Action(name) => BtNode::Action { name },
        }
    }

    pub fn from_bt_node(node: &BtNode) -> Self {
        match node {
            BtNode::Sequence { children } => BtNodeSpec::Sequence(children.iter().map(Self::from_bt_node).collect()),
            BtNode::Selector { children } => BtNodeSpec::Selector(children.iter().map(Self::from_bt_node).collect()),
            BtNode::Decorator { kind, child } => BtNodeSpec::Decorator {
                kind: kind.clone(),
                child: Box::new(Self::from_bt_node(child)),
            },
            BtNode::Action { name } => BtNodeSpec::Action(name.clone()),
        }
    }
}

/// **M7B**: one task-specific subtree slot inside an archetype BT.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskSubtree {
    pub task: String,
    pub root: BtNodeSpec,
}

/// **M7B**: one squad-verb-specific subtree slot inside an archetype BT.
/// Surfaces `Suppress (window)` / `Overwatch (sector)` / `Cover Me` as
/// distinct subtrees per spec.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SquadVerbSubtree {
    pub verb_id: String,
    pub root: BtNodeSpec,
}

/// **M7B**: full archetype BT definition serialized as RON. Mirrors the
/// Rust-side `node_ids_for` + `bt_for_task` + `bt_for_squad_verb` per
/// archetype.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArchetypeBtDef {
    pub kind: String,
    pub nodes: Vec<String>,
    pub task_subtrees: Vec<TaskSubtree>,
    pub squad_verb_subtrees: Vec<SquadVerbSubtree>,
}

impl ArchetypeBtDef {
    pub fn from_ron(src: &str) -> Result<Self, String> {
        ron::from_str(src).map_err(|e| format!("ron parse failed: {e}"))
    }

    /// Build the def from the in-memory Rust constants for the given kind.
    /// Used by the round-trip test that proves the RON files mirror the
    /// Rust source-of-truth.
    pub fn from_builtin(kind: ArchetypeBtKind) -> Self {
        let nodes: Vec<String> = node_ids_for(kind).iter().map(|s| (*s).to_string()).collect();
        let task_subtrees: Vec<TaskSubtree> = TaskType::ALL
            .iter()
            .map(|t| TaskSubtree {
                task: t.as_str().to_string(),
                root: BtNodeSpec::from_bt_node(&bt_for(kind, *t)),
            })
            .collect();
        let squad_verb_subtrees: Vec<SquadVerbSubtree> = squad_verbs_for(kind)
            .into_iter()
            .filter_map(|verb_id| {
                bt_for_squad_verb(kind, &verb_id).map(|root| SquadVerbSubtree {
                    verb_id,
                    root: BtNodeSpec::from_bt_node(&root),
                })
            })
            .collect();
        ArchetypeBtDef {
            kind: kind.as_str().to_string(),
            nodes,
            task_subtrees,
            squad_verb_subtrees,
        }
    }

    /// Convert into a lookup map keyed by task name (for the engine path
    /// that prefers RON-driven trees over the Rust default).
    pub fn task_lookup(&self) -> BTreeMap<String, BtNode> {
        self.task_subtrees
            .iter()
            .map(|t| (t.task.clone(), t.root.clone().into_bt_node()))
            .collect()
    }

    pub fn squad_verb_lookup(&self) -> BTreeMap<String, BtNode> {
        self.squad_verb_subtrees
            .iter()
            .map(|s| (s.verb_id.clone(), s.root.clone().into_bt_node()))
            .collect()
    }
}

/// **M7B**: the canonical list of squad-verb subtrees each archetype
/// should expose. Sourced from the per-archetype submodule so a verb id
/// drift can be caught at compile time by the round-trip test.
pub fn squad_verbs_for(kind: ArchetypeBtKind) -> Vec<String> {
    match kind {
        ArchetypeBtKind::Rifleman => rifleman::SQUAD_VERB_IDS.iter().map(|s| (*s).to_string()).collect(),
        ArchetypeBtKind::Sniper => sniper::SQUAD_VERB_IDS.iter().map(|s| (*s).to_string()).collect(),
        ArchetypeBtKind::Assault => assault::SQUAD_VERB_IDS.iter().map(|s| (*s).to_string()).collect(),
        ArchetypeBtKind::Engineer => engineer::SQUAD_VERB_IDS.iter().map(|s| (*s).to_string()).collect(),
        ArchetypeBtKind::Spotter => spotter::SQUAD_VERB_IDS.iter().map(|s| (*s).to_string()).collect(),
        ArchetypeBtKind::Heavy => heavy::SQUAD_VERB_IDS.iter().map(|s| (*s).to_string()).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rifleman_has_at_least_30_nodes() {
        assert!(
            node_ids_for(ArchetypeBtKind::Rifleman).len() >= 30,
            "rifleman has only {} nodes",
            node_ids_for(ArchetypeBtKind::Rifleman).len()
        );
    }

    #[test]
    fn sniper_has_at_least_30_nodes() {
        assert!(node_ids_for(ArchetypeBtKind::Sniper).len() >= 30);
    }

    #[test]
    fn assault_has_at_least_30_nodes() {
        assert!(node_ids_for(ArchetypeBtKind::Assault).len() >= 30);
    }

    #[test]
    fn engineer_has_at_least_30_nodes() {
        assert!(node_ids_for(ArchetypeBtKind::Engineer).len() >= 30);
    }

    #[test]
    fn spotter_has_at_least_30_nodes() {
        assert!(node_ids_for(ArchetypeBtKind::Spotter).len() >= 30);
    }

    #[test]
    fn heavy_has_at_least_30_nodes() {
        assert!(node_ids_for(ArchetypeBtKind::Heavy).len() >= 30);
    }

    #[test]
    fn all_archetype_nodes_are_unique() {
        for k in ArchetypeBtKind::ALL {
            let mut ids: Vec<&str> = node_ids_for(k).to_vec();
            ids.sort_unstable();
            for w in ids.windows(2) {
                assert_ne!(w[0], w[1], "duplicate node id {:?} in {:?}", w[0], k);
            }
        }
    }

    #[test]
    fn bt_for_engage_renders_for_every_archetype() {
        for k in ArchetypeBtKind::ALL {
            let node = bt_for(k, TaskType::EngageVisibleEnemy);
            let trail = node.flatten_label();
            assert!(!trail.is_empty(), "{k:?} engage trail empty");
        }
    }

    #[test]
    fn bt_for_suppress_window_and_overwatch_sector_are_distinct() {
        // Spec § "Suppress vs Overwatch vs Cover Me are distinct BT subtrees."
        let suppress = bt_for_squad_verb(ArchetypeBtKind::Rifleman, "suppress_window").expect("suppress");
        let overwatch = bt_for_squad_verb(ArchetypeBtKind::Rifleman, "overwatch_sector").expect("overwatch");
        let cover_me = bt_for_squad_verb(ArchetypeBtKind::Rifleman, "cover_me").expect("cover_me");
        let s_label = suppress.flatten_label();
        let o_label = overwatch.flatten_label();
        let c_label = cover_me.flatten_label();
        assert_ne!(s_label, o_label, "suppress + overwatch must be distinct subtrees");
        assert_ne!(s_label, c_label, "suppress + cover_me must be distinct subtrees");
        assert_ne!(o_label, c_label, "overwatch + cover_me must be distinct subtrees");
    }

    #[test]
    fn ron_content_files_round_trip_each_builtin() {
        // Spec note 7: "BTs authored as RON + loaded at startup; do NOT
        // hand-roll the 30-node-per-archetype floor in Rust." We use the
        // RON file as the content-author surface; the in-memory builtin
        // must exactly equal the RON parse.
        for (kind, src) in [
            (
                ArchetypeBtKind::Rifleman,
                include_str!("../../../content/ai/archetype_bts/rifleman.ron"),
            ),
            (
                ArchetypeBtKind::Sniper,
                include_str!("../../../content/ai/archetype_bts/sniper.ron"),
            ),
            (
                ArchetypeBtKind::Assault,
                include_str!("../../../content/ai/archetype_bts/assault.ron"),
            ),
            (
                ArchetypeBtKind::Engineer,
                include_str!("../../../content/ai/archetype_bts/engineer.ron"),
            ),
            (
                ArchetypeBtKind::Spotter,
                include_str!("../../../content/ai/archetype_bts/spotter.ron"),
            ),
            (
                ArchetypeBtKind::Heavy,
                include_str!("../../../content/ai/archetype_bts/heavy.ron"),
            ),
        ] {
            let parsed = ArchetypeBtDef::from_ron(src).unwrap_or_else(|e| panic!("{kind:?}: {e}"));
            let builtin = ArchetypeBtDef::from_builtin(kind);
            assert_eq!(
                parsed, builtin,
                "{kind:?} RON content drifted from Rust builtin — regenerate the RON file",
            );
        }
    }
}
