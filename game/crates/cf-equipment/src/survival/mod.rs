//! M6C: survival SKU registry (8 SKUs).
//!
//! Per M6C § "Survival (8 new)":
//! - water_bottle_1l, food_ration_mre, sleeping_bag, tent_2_person,
//!   cooking_pot, lighter_zippo, compass_magnetic, binoculars_8x.

use serde::{Deserialize, Serialize};

pub const WATER_BOTTLE_1L_ID: &str = "water_bottle_1l";
pub const FOOD_RATION_MRE_ID: &str = "food_ration_mre";
pub const SLEEPING_BAG_ID: &str = "sleeping_bag";
pub const TENT_2_PERSON_ID: &str = "tent_2_person";
pub const COOKING_POT_ID: &str = "cooking_pot";
pub const LIGHTER_ZIPPO_ID: &str = "lighter_zippo";
pub const COMPASS_MAGNETIC_ID: &str = "compass_magnetic";
pub const BINOCULARS_8X_ID: &str = "binoculars_8x";

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SurvivalEffectKind {
    Hydration = 0,
    Nutrition = 1,
    RestRecovery = 2,
    Shelter = 3,
    Cookware = 4,
    Ignition = 5,
    Navigation = 6,
    Optics = 7,
}

impl SurvivalEffectKind {
    pub fn as_str(self) -> &'static str {
        match self {
            SurvivalEffectKind::Hydration => "hydration",
            SurvivalEffectKind::Nutrition => "nutrition",
            SurvivalEffectKind::RestRecovery => "rest_recovery",
            SurvivalEffectKind::Shelter => "shelter",
            SurvivalEffectKind::Cookware => "cookware",
            SurvivalEffectKind::Ignition => "ignition",
            SurvivalEffectKind::Navigation => "navigation",
            SurvivalEffectKind::Optics => "optics",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SurvivalPreset {
    pub id: String,
    pub display_name: String,
    pub kind: SurvivalEffectKind,
    pub apply_seconds: f32,
    /// Per-use thirst / hunger / fatigue restoration (0.0 - 1.0).
    pub restore_fraction: f32,
    pub mass_kg: f32,
}

#[must_use]
pub fn m6c_survival_presets() -> Vec<SurvivalPreset> {
    vec![
        SurvivalPreset {
            id: WATER_BOTTLE_1L_ID.to_string(),
            display_name: "Water Bottle (1L)".to_string(),
            kind: SurvivalEffectKind::Hydration,
            apply_seconds: 2.0,
            restore_fraction: 0.50,
            mass_kg: 1.2,
        },
        SurvivalPreset {
            id: FOOD_RATION_MRE_ID.to_string(),
            display_name: "Food Ration (MRE)".to_string(),
            kind: SurvivalEffectKind::Nutrition,
            apply_seconds: 60.0,
            restore_fraction: 0.60,
            mass_kg: 0.6,
        },
        SurvivalPreset {
            id: SLEEPING_BAG_ID.to_string(),
            display_name: "Sleeping Bag".to_string(),
            kind: SurvivalEffectKind::RestRecovery,
            apply_seconds: 600.0,
            restore_fraction: 0.80,
            mass_kg: 1.8,
        },
        SurvivalPreset {
            id: TENT_2_PERSON_ID.to_string(),
            display_name: "Tent (2-person)".to_string(),
            kind: SurvivalEffectKind::Shelter,
            apply_seconds: 90.0,
            restore_fraction: 0.0,
            mass_kg: 3.5,
        },
        SurvivalPreset {
            id: COOKING_POT_ID.to_string(),
            display_name: "Cooking Pot".to_string(),
            kind: SurvivalEffectKind::Cookware,
            apply_seconds: 120.0,
            restore_fraction: 0.0,
            mass_kg: 0.9,
        },
        SurvivalPreset {
            id: LIGHTER_ZIPPO_ID.to_string(),
            display_name: "Lighter (Zippo)".to_string(),
            kind: SurvivalEffectKind::Ignition,
            apply_seconds: 1.0,
            restore_fraction: 0.0,
            mass_kg: 0.1,
        },
        SurvivalPreset {
            id: COMPASS_MAGNETIC_ID.to_string(),
            display_name: "Magnetic Compass".to_string(),
            kind: SurvivalEffectKind::Navigation,
            apply_seconds: 1.0,
            restore_fraction: 0.0,
            mass_kg: 0.1,
        },
        SurvivalPreset {
            id: BINOCULARS_8X_ID.to_string(),
            display_name: "Binoculars (8x)".to_string(),
            kind: SurvivalEffectKind::Optics,
            apply_seconds: 0.0,
            restore_fraction: 0.0,
            mass_kg: 0.7,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_eight_skus() {
        assert_eq!(m6c_survival_presets().len(), 8);
    }

    #[test]
    fn ids_unique() {
        let ids: std::collections::BTreeSet<String> =
            m6c_survival_presets().into_iter().map(|p| p.id).collect();
        assert_eq!(ids.len(), 8);
    }
}
