//! M7B: Tab tactical overlay verb + formation enumerator.
//!
//! Spec § "Tab tactical overlay reads verb registry + formation registry;
//! Q-hold context wheel reads same." This module wraps the cf-ai
//! `VerbRegistry` + formation catalog so the Tab overlay renders the canon
//! enumeration without a UI-side duplicate of the verb list. The engine
//! mirrors `TacticalOverlayVerbList::from_builtin` into a Bevy resource the
//! overlay reads per frame.

use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};

use cf_ai::{
    archetype_bt::ArchetypeBtKind,
    formation::FormationKind,
    squad_command_grammar::{builtin_registry, verb_family_label, VerbFamily, VerbRegistry},
    FormationDef,
};

/// **M7B**: one entry in the overlay's verb list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TacticalOverlayVerbEntry {
    pub verb_id: String,
    pub display_name: String,
    pub family: String,
    pub family_label: String,
    pub args_summary: String,
    pub valid_target: String,
}

/// **M7B**: one entry in the overlay's formation list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TacticalOverlayFormationEntry {
    pub kind: String,
    pub slot_count: usize,
}

/// **M7B**: one entry in the overlay's per-archetype BT row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TacticalOverlayArchetypeBtEntry {
    pub kind: String,
    pub node_count: usize,
}

/// **M7B**: Bevy resource the Tab overlay reads to populate its verb list,
/// formation list, and archetype BT panel. Populated from cf-ai's builtin
/// registry by default; the engine refreshes from RON when content reloads.
#[derive(Resource, Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TacticalOverlayVerbList {
    pub verbs: Vec<TacticalOverlayVerbEntry>,
    pub formations: Vec<TacticalOverlayFormationEntry>,
    pub archetype_bts: Vec<TacticalOverlayArchetypeBtEntry>,
}

impl TacticalOverlayVerbList {
    /// **M7B**: build from cf-ai's built-in registry.
    pub fn from_builtin() -> Self {
        Self::from_registry(&builtin_registry())
    }

    /// **M7B**: build from a caller-provided registry (used when RON
    /// reloads).
    pub fn from_registry(registry: &VerbRegistry) -> Self {
        let verbs: Vec<TacticalOverlayVerbEntry> = registry
            .iter()
            .map(|def| TacticalOverlayVerbEntry {
                verb_id: def.verb_id.clone(),
                display_name: def.display_name.clone(),
                family: def.family.as_str().to_string(),
                family_label: verb_family_label(def.family).to_string(),
                args_summary: summarize_args(def),
                valid_target: def.valid_target.clone(),
            })
            .collect();

        let formations: Vec<TacticalOverlayFormationEntry> = FormationKind::ALL
            .iter()
            .map(|kind| TacticalOverlayFormationEntry {
                kind: kind.as_str().to_string(),
                slot_count: FormationDef::builtin(*kind).slots.len(),
            })
            .collect();

        let archetype_bts: Vec<TacticalOverlayArchetypeBtEntry> = ArchetypeBtKind::ALL
            .iter()
            .map(|k| TacticalOverlayArchetypeBtEntry {
                kind: k.as_str().to_string(),
                node_count: cf_ai::archetype_bt::node_ids_for(*k).len(),
            })
            .collect();

        Self {
            verbs,
            formations,
            archetype_bts,
        }
    }

    pub fn verb_count(&self) -> usize {
        self.verbs.len()
    }

    pub fn formation_count(&self) -> usize {
        self.formations.len()
    }

    pub fn verbs_by_family<'a>(
        &'a self,
        family: VerbFamily,
    ) -> impl Iterator<Item = &'a TacticalOverlayVerbEntry> + 'a {
        let key = family.as_str();
        self.verbs.iter().filter(move |v| v.family == key)
    }
}

fn summarize_args(def: &cf_ai::squad_command_grammar::VerbDef) -> String {
    if def.args.is_empty() {
        return "()".to_string();
    }
    let parts: Vec<String> = def
        .args
        .iter()
        .map(|a| {
            let suffix = if a.required { "" } else { "?" };
            format!("{}:{}{}", a.name, a.kind.as_str(), suffix)
        })
        .collect();
    format!("({})", parts.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_enumerates_at_least_50_verbs() {
        let list = TacticalOverlayVerbList::from_builtin();
        assert!(
            list.verb_count() >= 50,
            "tactical overlay enumerates only {} verbs",
            list.verb_count()
        );
    }

    #[test]
    fn list_enumerates_all_nine_formations() {
        let list = TacticalOverlayVerbList::from_builtin();
        assert_eq!(list.formation_count(), 9);
    }

    #[test]
    fn list_lists_each_of_six_archetype_bts() {
        let list = TacticalOverlayVerbList::from_builtin();
        assert_eq!(list.archetype_bts.len(), 6);
        for entry in &list.archetype_bts {
            assert!(entry.node_count >= 30);
        }
    }

    #[test]
    fn breach_chain_verbs_listed() {
        let list = TacticalOverlayVerbList::from_builtin();
        for id in ["stack_door", "breach_door", "frag_out", "advance"] {
            assert!(
                list.verbs.iter().any(|v| v.verb_id == id),
                "tactical overlay missing breach-chain verb {id}"
            );
        }
    }

    #[test]
    fn family_filter_returns_subset() {
        let list = TacticalOverlayVerbList::from_builtin();
        let movement_count = list.verbs_by_family(VerbFamily::Movement).count();
        assert!(movement_count > 0);
        let logistics_count = list.verbs_by_family(VerbFamily::Logistics).count();
        assert!(logistics_count > 0);
        assert!(movement_count + logistics_count <= list.verb_count());
    }
}
