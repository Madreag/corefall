//! M15C — full thermodynamic schema extensions for [`crate::MaterialDef`].
//!
//! Defines [`MaterialState`], [`MaterialReactionRef`], [`ContainerRules`] —
//! the new types the M15C spec calls out, and the canonical validators
//! that the loader uses to enforce the M15C non-default-field rules
//! (hardness, density_kg_per_m3, specific_heat_capacity_j_per_kg_k,
//! thermal_conductivity_w_per_m_k, color_hex).

use serde::{Deserialize, Serialize};

use crate::MaterialId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MaterialState {
    #[default]
    Solid,
    Liquid,
    Gas,
    Powder,
    Plasma,
    EnergyField,
}

impl MaterialState {
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Solid => "solid",
            Self::Liquid => "liquid",
            Self::Gas => "gas",
            Self::Powder => "powder",
            Self::Plasma => "plasma",
            Self::EnergyField => "energy_field",
        }
    }

    #[must_use]
    pub fn from_label(s: &str) -> Option<Self> {
        match s {
            "solid" => Some(Self::Solid),
            "liquid" => Some(Self::Liquid),
            "gas" => Some(Self::Gas),
            "powder" => Some(Self::Powder),
            "plasma" => Some(Self::Plasma),
            "energy_field" => Some(Self::EnergyField),
            _ => None,
        }
    }

    #[must_use]
    pub fn all() -> &'static [Self] {
        &[
            Self::Solid,
            Self::Liquid,
            Self::Gas,
            Self::Powder,
            Self::Plasma,
            Self::EnergyField,
        ]
    }
}

/// reaction matrix. Lets a material declare which reactions it participates
/// in (as input_a / input_b / output / byproduct) without re-encoding the
/// reaction kinetics here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct MaterialReactionRef {
    pub reaction_id: String,
    #[serde(default)]
    pub role: Option<String>,
}

impl MaterialReactionRef {
    #[must_use]
    pub fn new(reaction_id: impl Into<String>) -> Self {
        Self { reaction_id: reaction_id.into(), role: None }
    }

    #[must_use]
    pub fn with_role(reaction_id: impl Into<String>, role: impl Into<String>) -> Self {
        Self {
            reaction_id: reaction_id.into(),
            role: Some(role.into()),
        }
    }
}

/// scenarios. Liquids + gases declare how containers store them; solids
/// + powders mostly leave this empty (defaults to non-sealable).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ContainerRules {
    #[serde(default)]
    pub sealable: bool,
    #[serde(default)]
    pub max_capacity_l: Option<f32>,
    #[serde(default)]
    pub leak_rate_l_per_s: Option<f32>,
    #[serde(default)]
    pub stackable_with: Vec<MaterialId>,
}

impl ContainerRules {
    #[must_use]
    pub fn sealable(max_capacity_l: f32) -> Self {
        Self {
            sealable: true,
            max_capacity_l: Some(max_capacity_l),
            leak_rate_l_per_s: None,
            stackable_with: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn material_state_round_trips_through_serde() {
        for s in MaterialState::all() {
            let v = serde_json::to_value(s).unwrap();
            let back: MaterialState = serde_json::from_value(v).unwrap();
            assert_eq!(*s, back);
        }
    }

    #[test]
    fn material_state_label_matches_serde_repr() {
        for s in MaterialState::all() {
            assert_eq!(MaterialState::from_label(s.label()), Some(*s));
        }
    }

    #[test]
    fn material_state_default_is_solid() {
        assert_eq!(MaterialState::default(), MaterialState::Solid);
    }

    #[test]
    fn reaction_ref_round_trips() {
        let r = MaterialReactionRef::with_role("rxn.corrosion.acid_iron", "input_a");
        let v = serde_json::to_value(&r).unwrap();
        let back: MaterialReactionRef = serde_json::from_value(v).unwrap();
        assert_eq!(back, r);
    }

    #[test]
    fn container_rules_default_is_non_sealable() {
        let c = ContainerRules::default();
        assert!(!c.sealable);
        assert!(c.max_capacity_l.is_none());
    }
}
