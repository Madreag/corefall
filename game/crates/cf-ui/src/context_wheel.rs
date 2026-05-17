//! M7B: Q-hold context wheel verb enumerator.
//!
//! Spec § "Tab tactical overlay reads verb registry + formation registry;
//! Q-hold context wheel reads same." The Q-hold context wheel is the radial
//! action menu surfaced while the player holds Q. The wheel groups verbs by
//! family and renders the entries in the canonical registry order.

use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};

use cf_ai::{
    squad_command_grammar::{builtin_registry, verb_family_label, VerbFamily, VerbRegistry},
};

/// **M7B**: one slice in the context wheel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextWheelSlice {
    pub verb_id: String,
    pub display_name: String,
    pub family: String,
    pub valid_target: String,
}

/// **M7B**: a wheel section grouping one family's verbs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextWheelSection {
    pub family: String,
    pub family_label: String,
    pub slices: Vec<ContextWheelSlice>,
}

/// **M7B**: full Q-hold context wheel state. cf-ui resource the engine
/// populates from the cf-ai registry per session start.
#[derive(Resource, Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ContextWheelState {
    pub sections: Vec<ContextWheelSection>,
}

impl ContextWheelState {
    /// **M7B**: build the wheel from cf-ai's builtin registry.
    pub fn from_builtin() -> Self {
        Self::from_registry(&builtin_registry())
    }

    /// **M7B**: build the wheel from a caller-supplied registry.
    pub fn from_registry(registry: &VerbRegistry) -> Self {
        let families: [VerbFamily; 5] = [
            VerbFamily::Movement,
            VerbFamily::Engagement,
            VerbFamily::MovementToContact,
            VerbFamily::RoleSpecific,
            VerbFamily::Logistics,
        ];
        let sections: Vec<ContextWheelSection> = families
            .iter()
            .map(|family| {
                let slices: Vec<ContextWheelSlice> = registry
                    .by_family(*family)
                    .map(|def| ContextWheelSlice {
                        verb_id: def.verb_id.clone(),
                        display_name: def.display_name.clone(),
                        family: def.family.as_str().to_string(),
                        valid_target: def.valid_target.clone(),
                    })
                    .collect();
                ContextWheelSection {
                    family: family.as_str().to_string(),
                    family_label: verb_family_label(*family).to_string(),
                    slices,
                }
            })
            .collect();
        Self { sections }
    }

    /// Total slices across every section.
    pub fn slice_count(&self) -> usize {
        self.sections.iter().map(|s| s.slices.len()).sum()
    }

    pub fn section_count(&self) -> usize {
        self.sections.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wheel_has_five_sections() {
        let wheel = ContextWheelState::from_builtin();
        assert_eq!(wheel.section_count(), 5);
    }

    #[test]
    fn wheel_lists_at_least_50_slices() {
        let wheel = ContextWheelState::from_builtin();
        assert!(
            wheel.slice_count() >= 50,
            "context wheel has only {} slices",
            wheel.slice_count()
        );
    }

    #[test]
    fn movement_section_includes_move_to() {
        let wheel = ContextWheelState::from_builtin();
        let movement = wheel
            .sections
            .iter()
            .find(|s| s.family == "movement")
            .expect("movement section");
        assert!(movement.slices.iter().any(|s| s.verb_id == "move_to"));
    }

    #[test]
    fn engagement_section_includes_press_attack() {
        let wheel = ContextWheelState::from_builtin();
        let eng = wheel
            .sections
            .iter()
            .find(|s| s.family == "engagement")
            .expect("engagement section");
        assert!(eng.slices.iter().any(|s| s.verb_id == "press_attack"));
    }

    #[test]
    fn wheel_and_overlay_share_verb_set() {
        // The acceptance scenario "Verb + formation registries surface to
        // UI" requires the wheel + tactical overlay enumerate the SAME
        // registry. We verify the same set of verb_ids is produced.
        let wheel = ContextWheelState::from_builtin();
        let wheel_ids: std::collections::BTreeSet<&str> = wheel
            .sections
            .iter()
            .flat_map(|s| s.slices.iter().map(|sl| sl.verb_id.as_str()))
            .collect();
        let overlay = crate::tactical_overlay::TacticalOverlayVerbList::from_builtin();
        let overlay_ids: std::collections::BTreeSet<&str> =
            overlay.verbs.iter().map(|v| v.verb_id.as_str()).collect();
        assert_eq!(wheel_ids, overlay_ids);
    }
}
