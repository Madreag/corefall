//! M6C: personal protective equipment (PPE) SKU registry (18 SKUs).
//!
//! Per M6C § "PPE": helmets, body armor tiers, gloves, boots, pads,
//! hardsuits, EVA suit, radiation/hazmat/insulated suits, and the
//! modular plate carrier.

pub mod armor_calc;
pub mod eva;

use serde::{Deserialize, Serialize};

// Helmets ---------------------------------------------------------------------
pub const HELMET_LIGHT_KEVLAR_ID: &str = "helmet_light_kevlar";
pub const HELMET_MEDIUM_STEEL_ID: &str = "helmet_medium_steel";
pub const HELMET_HEAVY_TITANIUM_ID: &str = "helmet_heavy_titanium";

// Body armor ------------------------------------------------------------------
pub const ARMOR_KEVLAR_LIGHT_ID: &str = "armor_kevlar_light";
pub const ARMOR_CERAMIC_MEDIUM_ID: &str = "armor_ceramic_medium";
pub const ARMOR_STEEL_HEAVY_ID: &str = "armor_steel_heavy";
pub const ARMOR_HEAVY_PLATE_ID: &str = "armor_heavy_plate";
pub const ARMOR_MODULAR_PLATE_CARRIER_ID: &str = "armor_modular_plate_carrier";

// Gloves / boots / pads -------------------------------------------------------
pub const COMBAT_GLOVES_LIGHT_ID: &str = "combat_gloves_light";
pub const COMBAT_GLOVES_HEAVY_ID: &str = "combat_gloves_heavy";
pub const TACTICAL_BOOTS_ID: &str = "tactical_boots";
pub const KNEE_PADS_ID: &str = "knee_pads";
pub const ELBOW_PADS_ID: &str = "elbow_pads";

// Sealed suits ----------------------------------------------------------------
pub const HARDSUIT_FULL_ID: &str = "hardsuit_full";
pub const EVA_SUIT_ID: &str = "eva_suit";
pub const RADIATION_SUIT_ID: &str = "radiation_suit";
pub const HAZMAT_SUIT_ID: &str = "hazmat_suit";
pub const INSULATED_SUIT_ID: &str = "insulated_suit";

/// PPE category. Drives which slot the item occupies in the
/// per-actor [`cf_actor::body_armor_slot::BodyArmorSlot`].
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PpeKind {
    Helmet = 0,
    BodyArmor = 1,
    Gloves = 2,
    Boots = 3,
    KneePads = 4,
    ElbowPads = 5,
    Hardsuit = 6,
    EvaSuit = 7,
    RadiationSuit = 8,
    HazmatSuit = 9,
    InsulatedSuit = 10,
    ModularPlateCarrier = 11,
}

impl PpeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            PpeKind::Helmet => "helmet",
            PpeKind::BodyArmor => "body_armor",
            PpeKind::Gloves => "gloves",
            PpeKind::Boots => "boots",
            PpeKind::KneePads => "knee_pads",
            PpeKind::ElbowPads => "elbow_pads",
            PpeKind::Hardsuit => "hardsuit",
            PpeKind::EvaSuit => "eva_suit",
            PpeKind::RadiationSuit => "radiation_suit",
            PpeKind::HazmatSuit => "hazmat_suit",
            PpeKind::InsulatedSuit => "insulated_suit",
            PpeKind::ModularPlateCarrier => "modular_plate_carrier",
        }
    }

    /// True when this PPE is a body-armor-replacement plate.
    pub fn is_body_armor(self) -> bool {
        matches!(
            self,
            PpeKind::BodyArmor
                | PpeKind::Hardsuit
                | PpeKind::EvaSuit
                | PpeKind::RadiationSuit
                | PpeKind::HazmatSuit
                | PpeKind::InsulatedSuit
                | PpeKind::ModularPlateCarrier
        )
    }

    /// True when this PPE provides hermetic sealing (M6C-6 EVA + vacuum).
    pub fn is_sealed(self) -> bool {
        matches!(
            self,
            PpeKind::Hardsuit | PpeKind::EvaSuit | PpeKind::RadiationSuit | PpeKind::HazmatSuit
        )
    }
}

/// Full PPE descriptor consumed by the cf-actor body_armor_slot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PpePreset {
    pub id: String,
    pub display_name: String,
    pub kind: PpeKind,
    /// Mass in kg (drives M14A mass aggregation).
    pub mass_kg: f32,
    /// Kinetic damage reduction (0..1; 0.20 = -20%).
    pub kinetic_damage_reduction: f32,
    /// Thermal damage reduction (0..1).
    pub thermal_damage_reduction: f32,
    /// Cold-affliction tick reduction (0..1).
    pub cold_affliction_reduction: f32,
    /// Heat-affliction tick reduction (0..1).
    pub heat_affliction_reduction: f32,
    /// Radiation tick reduction (0..1).
    pub radiation_tick_reduction: f32,
    /// Chemical contact reduction (0..1).
    pub chemical_contact_reduction: f32,
    /// Walk-speed modifier (positive = bonus; negative = penalty).
    pub mobility_modifier: f32,
    /// Grip modifier (0..1 positive only; 0.05 = +5%).
    pub grip_modifier: f32,
    /// Prone-scrape damage reduction (knee pads).
    pub prone_scrape_reduction: f32,
    /// Lean-scrape damage reduction (elbow pads).
    pub lean_scrape_reduction: f32,
    /// Initial durability (HP-equivalent for the armor layer).
    pub durability_hp: f32,
    /// True when the suit is hermetically sealed (M6C-6 vacuum compatible).
    pub sealed: bool,
    /// Number of modular plate inserts the item accepts (plate carriers).
    pub plate_insert_count: u8,
}

impl PpePreset {
    pub fn new_basic(id: &str, display: &str, kind: PpeKind, mass_kg: f32, durability_hp: f32) -> Self {
        Self {
            id: id.to_string(),
            display_name: display.to_string(),
            kind,
            mass_kg,
            kinetic_damage_reduction: 0.0,
            thermal_damage_reduction: 0.0,
            cold_affliction_reduction: 0.0,
            heat_affliction_reduction: 0.0,
            radiation_tick_reduction: 0.0,
            chemical_contact_reduction: 0.0,
            mobility_modifier: 0.0,
            grip_modifier: 0.0,
            prone_scrape_reduction: 0.0,
            lean_scrape_reduction: 0.0,
            durability_hp,
            sealed: kind.is_sealed(),
            plate_insert_count: 0,
        }
    }
}

#[must_use]
pub fn m6c_ppe_presets() -> Vec<PpePreset> {
    vec![
        // Helmets ---------------------------------------------------------
        PpePreset {
            kinetic_damage_reduction: 0.10,
            ..PpePreset::new_basic(HELMET_LIGHT_KEVLAR_ID, "Light Kevlar Helmet", PpeKind::Helmet, 1.2, 200.0)
        },
        PpePreset {
            kinetic_damage_reduction: 0.20,
            ..PpePreset::new_basic(HELMET_MEDIUM_STEEL_ID, "Medium Steel Helmet", PpeKind::Helmet, 2.5, 400.0)
        },
        PpePreset {
            kinetic_damage_reduction: 0.35,
            // M6C § "EVA suit + helmet seal vacuum": the heavy titanium
            // helmet is pressure-vessel grade, so it satisfies the
            // sealed-helmet half of the M6C-6 vacuum-survivability test.
            sealed: true,
            ..PpePreset::new_basic(HELMET_HEAVY_TITANIUM_ID, "Heavy Titanium Helmet", PpeKind::Helmet, 3.8, 700.0)
        },
        // Body armor ------------------------------------------------------
        PpePreset {
            kinetic_damage_reduction: 0.20,
            ..PpePreset::new_basic(ARMOR_KEVLAR_LIGHT_ID, "Light Kevlar Armor", PpeKind::BodyArmor, 4.0, 400.0)
        },
        PpePreset {
            kinetic_damage_reduction: 0.40,
            thermal_damage_reduction: 0.20,
            ..PpePreset::new_basic(ARMOR_CERAMIC_MEDIUM_ID, "Medium Ceramic Armor", PpeKind::BodyArmor, 7.0, 700.0)
        },
        PpePreset {
            kinetic_damage_reduction: 0.60,
            mobility_modifier: -0.10,
            ..PpePreset::new_basic(ARMOR_STEEL_HEAVY_ID, "Heavy Steel Armor", PpeKind::BodyArmor, 12.0, 1200.0)
        },
        PpePreset {
            kinetic_damage_reduction: 0.80,
            mobility_modifier: -0.30,
            ..PpePreset::new_basic(ARMOR_HEAVY_PLATE_ID, "Heavy Plate Armor", PpeKind::BodyArmor, 18.0, 1800.0)
        },
        PpePreset {
            kinetic_damage_reduction: 0.20,
            plate_insert_count: 4,
            ..PpePreset::new_basic(ARMOR_MODULAR_PLATE_CARRIER_ID, "Modular Plate Carrier", PpeKind::ModularPlateCarrier, 5.0, 800.0)
        },
        // Gloves / boots / pads --------------------------------------------
        PpePreset {
            grip_modifier: 0.05,
            ..PpePreset::new_basic(COMBAT_GLOVES_LIGHT_ID, "Light Combat Gloves", PpeKind::Gloves, 0.2, 80.0)
        },
        PpePreset {
            cold_affliction_reduction: 0.10,
            ..PpePreset::new_basic(COMBAT_GLOVES_HEAVY_ID, "Heavy Combat Gloves", PpeKind::Gloves, 0.4, 120.0)
        },
        PpePreset {
            mobility_modifier: 0.05,
            ..PpePreset::new_basic(TACTICAL_BOOTS_ID, "Tactical Boots", PpeKind::Boots, 1.2, 300.0)
        },
        PpePreset {
            prone_scrape_reduction: 0.20,
            ..PpePreset::new_basic(KNEE_PADS_ID, "Knee Pads", PpeKind::KneePads, 0.4, 150.0)
        },
        PpePreset {
            lean_scrape_reduction: 0.20,
            ..PpePreset::new_basic(ELBOW_PADS_ID, "Elbow Pads", PpeKind::ElbowPads, 0.3, 150.0)
        },
        // Sealed suits -----------------------------------------------------
        PpePreset {
            kinetic_damage_reduction: 0.40,
            thermal_damage_reduction: 0.20,
            radiation_tick_reduction: 0.50,
            chemical_contact_reduction: 0.50,
            mobility_modifier: -0.30,
            ..PpePreset::new_basic(HARDSUIT_FULL_ID, "Full Hardsuit", PpeKind::Hardsuit, 18.0, 1500.0)
        },
        PpePreset {
            kinetic_damage_reduction: 0.10,
            thermal_damage_reduction: 0.10,
            radiation_tick_reduction: 0.40,
            chemical_contact_reduction: 0.30,
            mobility_modifier: -0.20,
            ..PpePreset::new_basic(EVA_SUIT_ID, "EVA Suit", PpeKind::EvaSuit, 14.0, 1000.0)
        },
        PpePreset {
            radiation_tick_reduction: 0.90,
            chemical_contact_reduction: 0.30,
            ..PpePreset::new_basic(RADIATION_SUIT_ID, "Radiation Suit", PpeKind::RadiationSuit, 8.0, 600.0)
        },
        PpePreset {
            chemical_contact_reduction: 0.90,
            radiation_tick_reduction: 0.30,
            ..PpePreset::new_basic(HAZMAT_SUIT_ID, "Hazmat Suit", PpeKind::HazmatSuit, 6.0, 400.0)
        },
        PpePreset {
            cold_affliction_reduction: 0.40,
            heat_affliction_reduction: 0.20,
            ..PpePreset::new_basic(INSULATED_SUIT_ID, "Insulated Suit", PpeKind::InsulatedSuit, 5.0, 500.0)
        },
    ]
}

/// Look up a PPE preset by id.
#[must_use]
pub fn ppe_preset(id: &str) -> Option<PpePreset> {
    m6c_ppe_presets().into_iter().find(|p| p.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_eighteen_skus() {
        assert_eq!(m6c_ppe_presets().len(), 18);
    }

    #[test]
    fn light_kevlar_armor_matches_spec_table() {
        // Spec: armor_kevlar_light (-20% kinetic / 4 kg)
        let a = ppe_preset(ARMOR_KEVLAR_LIGHT_ID).unwrap();
        assert!((a.kinetic_damage_reduction - 0.20).abs() < 1e-3);
        assert!((a.mass_kg - 4.0).abs() < 1e-3);
    }

    #[test]
    fn heavy_plate_armor_matches_spec_table() {
        // Spec: armor_heavy_plate (-80% kinetic + -30% mobility / 18 kg)
        let a = ppe_preset(ARMOR_HEAVY_PLATE_ID).unwrap();
        assert!((a.kinetic_damage_reduction - 0.80).abs() < 1e-3);
        assert!((a.mobility_modifier + 0.30).abs() < 1e-3);
    }

    #[test]
    fn radiation_suit_matches_spec_table() {
        // Spec: radiation_suit (-90% radiation tick / 8 kg)
        let r = ppe_preset(RADIATION_SUIT_ID).unwrap();
        assert!((r.radiation_tick_reduction - 0.90).abs() < 1e-3);
    }

    #[test]
    fn modular_plate_carrier_accepts_four_plates() {
        let c = ppe_preset(ARMOR_MODULAR_PLATE_CARRIER_ID).unwrap();
        assert_eq!(c.plate_insert_count, 4);
    }

    #[test]
    fn eva_suit_is_sealed() {
        let s = ppe_preset(EVA_SUIT_ID).unwrap();
        assert!(s.sealed);
    }
}
