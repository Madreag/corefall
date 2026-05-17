//! M6C: medical SKU registry (12 SKUs).
//!
//! Per M6C § "Medical (12 new beyond M6 medkit)" — these are the M14H
//! consumer surface; the actual application of medical effects lives in
//! the M16 affliction system. This module only defines the SKUs + a
//! per-SKU [`MedicalApplication`] descriptor that the engine can route to
//! the appropriate affliction.

pub mod defibrillator;

use serde::{Deserialize, Serialize};

pub const FIELD_BANDAGE_ID: &str = "field_bandage";
pub const TRAUMA_PACK_ID: &str = "trauma_pack";
pub const TOURNIQUET_ID: &str = "tourniquet";
pub const SUTURES_ID: &str = "sutures";
pub const SPLINT_ID: &str = "splint";
pub const SURGERY_KIT_ID: &str = "surgery_kit";
pub const DEFIBRILLATOR_ID: &str = "defibrillator";
pub const CPR_COMPRESSIONS_ID: &str = "cpr_compressions";
pub const TRANSFUSION_BAG_ID: &str = "transfusion_bag";
pub const IV_FLUIDS_ID: &str = "iv_fluids";
pub const OXYGEN_THERAPY_ID: &str = "oxygen_therapy";
pub const MEDICAL_SCANNER_T1_ID: &str = "medical_scanner_t1";

/// Categorical medical effect kind. Maps 1:1 onto M16 afflictions.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MedicalEffectKind {
    StopBleeding = 0,
    HealMinorTrauma = 1,
    HealMajorTrauma = 2,
    CloseLaceration = 3,
    StabilizeFracture = 4,
    SurgicalRepair = 5,
    DefibRevive = 6,
    CprStabilize = 7,
    RestoreBlood = 8,
    RehydrateFluids = 9,
    OxygenSupplement = 10,
    DiagnosticScan = 11,
}

impl MedicalEffectKind {
    pub fn as_str(self) -> &'static str {
        match self {
            MedicalEffectKind::StopBleeding => "stop_bleeding",
            MedicalEffectKind::HealMinorTrauma => "heal_minor_trauma",
            MedicalEffectKind::HealMajorTrauma => "heal_major_trauma",
            MedicalEffectKind::CloseLaceration => "close_laceration",
            MedicalEffectKind::StabilizeFracture => "stabilize_fracture",
            MedicalEffectKind::SurgicalRepair => "surgical_repair",
            MedicalEffectKind::DefibRevive => "defib_revive",
            MedicalEffectKind::CprStabilize => "cpr_stabilize",
            MedicalEffectKind::RestoreBlood => "restore_blood",
            MedicalEffectKind::RehydrateFluids => "rehydrate_fluids",
            MedicalEffectKind::OxygenSupplement => "oxygen_supplement",
            MedicalEffectKind::DiagnosticScan => "diagnostic_scan",
        }
    }
}

/// Per-SKU descriptor consumed by M14H + M16.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MedicalPreset {
    pub id: String,
    pub display_name: String,
    pub kind: MedicalEffectKind,
    /// Application time in seconds.
    pub apply_seconds: f32,
    /// HP restored on apply (0.0 - 1.0 fraction of max HP).
    pub hp_restore_fraction: f32,
    /// Affliction id this SKU clears (M16 consumer; empty = none).
    pub clears_affliction_id: String,
    /// True when the SKU can target downed actors (defibrillator, cpr).
    pub revives_downed: bool,
    /// Mass in kg.
    pub mass_kg: f32,
}

#[must_use]
pub fn m6c_medical_presets() -> Vec<MedicalPreset> {
    vec![
        field_bandage(),
        trauma_pack(),
        tourniquet(),
        sutures(),
        splint(),
        surgery_kit(),
        defibrillator(),
        cpr_compressions(),
        transfusion_bag(),
        iv_fluids(),
        oxygen_therapy(),
        medical_scanner_t1(),
    ]
}

fn field_bandage() -> MedicalPreset {
    MedicalPreset {
        id: FIELD_BANDAGE_ID.to_string(),
        display_name: "Field Bandage".to_string(),
        kind: MedicalEffectKind::StopBleeding,
        apply_seconds: 2.5,
        hp_restore_fraction: 0.10,
        clears_affliction_id: "bleeding_light".to_string(),
        revives_downed: false,
        mass_kg: 0.1,
    }
}

fn trauma_pack() -> MedicalPreset {
    MedicalPreset {
        id: TRAUMA_PACK_ID.to_string(),
        display_name: "Trauma Pack".to_string(),
        kind: MedicalEffectKind::HealMajorTrauma,
        apply_seconds: 4.0,
        hp_restore_fraction: 0.30,
        clears_affliction_id: "bleeding_heavy".to_string(),
        revives_downed: false,
        mass_kg: 0.4,
    }
}

fn tourniquet() -> MedicalPreset {
    MedicalPreset {
        id: TOURNIQUET_ID.to_string(),
        display_name: "Tourniquet".to_string(),
        kind: MedicalEffectKind::StopBleeding,
        apply_seconds: 1.5,
        hp_restore_fraction: 0.05,
        clears_affliction_id: "bleeding_arterial".to_string(),
        revives_downed: false,
        mass_kg: 0.05,
    }
}

fn sutures() -> MedicalPreset {
    MedicalPreset {
        id: SUTURES_ID.to_string(),
        display_name: "Sutures".to_string(),
        kind: MedicalEffectKind::CloseLaceration,
        apply_seconds: 5.0,
        hp_restore_fraction: 0.15,
        clears_affliction_id: "laceration".to_string(),
        revives_downed: false,
        mass_kg: 0.1,
    }
}

fn splint() -> MedicalPreset {
    MedicalPreset {
        id: SPLINT_ID.to_string(),
        display_name: "Splint".to_string(),
        kind: MedicalEffectKind::StabilizeFracture,
        apply_seconds: 3.0,
        hp_restore_fraction: 0.05,
        clears_affliction_id: "fracture".to_string(),
        revives_downed: false,
        mass_kg: 0.3,
    }
}

fn surgery_kit() -> MedicalPreset {
    MedicalPreset {
        id: SURGERY_KIT_ID.to_string(),
        display_name: "Surgery Kit".to_string(),
        kind: MedicalEffectKind::SurgicalRepair,
        apply_seconds: 15.0,
        hp_restore_fraction: 0.60,
        clears_affliction_id: "internal_damage".to_string(),
        revives_downed: false,
        mass_kg: 2.5,
    }
}

fn defibrillator() -> MedicalPreset {
    MedicalPreset {
        id: DEFIBRILLATOR_ID.to_string(),
        display_name: "Defibrillator".to_string(),
        kind: MedicalEffectKind::DefibRevive,
        apply_seconds: 3.0,
        hp_restore_fraction: defibrillator::DEFIB_REVIVE_HP_FRACTION,
        clears_affliction_id: "cardiac_arrest".to_string(),
        revives_downed: true,
        mass_kg: 2.0,
    }
}

fn cpr_compressions() -> MedicalPreset {
    MedicalPreset {
        id: CPR_COMPRESSIONS_ID.to_string(),
        display_name: "CPR Compressions".to_string(),
        kind: MedicalEffectKind::CprStabilize,
        apply_seconds: 8.0,
        hp_restore_fraction: 0.10,
        clears_affliction_id: "cardiac_arrest".to_string(),
        revives_downed: true,
        mass_kg: 0.0,
    }
}

fn transfusion_bag() -> MedicalPreset {
    MedicalPreset {
        id: TRANSFUSION_BAG_ID.to_string(),
        display_name: "Transfusion Bag".to_string(),
        kind: MedicalEffectKind::RestoreBlood,
        apply_seconds: 6.0,
        hp_restore_fraction: 0.20,
        clears_affliction_id: "blood_loss".to_string(),
        revives_downed: false,
        mass_kg: 0.6,
    }
}

fn iv_fluids() -> MedicalPreset {
    MedicalPreset {
        id: IV_FLUIDS_ID.to_string(),
        display_name: "IV Fluids".to_string(),
        kind: MedicalEffectKind::RehydrateFluids,
        apply_seconds: 5.0,
        hp_restore_fraction: 0.12,
        clears_affliction_id: "dehydration".to_string(),
        revives_downed: false,
        mass_kg: 0.5,
    }
}

fn oxygen_therapy() -> MedicalPreset {
    MedicalPreset {
        id: OXYGEN_THERAPY_ID.to_string(),
        display_name: "Oxygen Therapy".to_string(),
        kind: MedicalEffectKind::OxygenSupplement,
        apply_seconds: 4.0,
        hp_restore_fraction: 0.08,
        clears_affliction_id: "hypoxia".to_string(),
        revives_downed: false,
        mass_kg: 1.4,
    }
}

fn medical_scanner_t1() -> MedicalPreset {
    MedicalPreset {
        id: MEDICAL_SCANNER_T1_ID.to_string(),
        display_name: "Medical Scanner (T1)".to_string(),
        kind: MedicalEffectKind::DiagnosticScan,
        apply_seconds: 2.0,
        hp_restore_fraction: 0.0,
        clears_affliction_id: String::new(),
        revives_downed: false,
        mass_kg: 0.8,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_twelve_skus() {
        assert_eq!(m6c_medical_presets().len(), 12);
    }

    #[test]
    fn defibrillator_revives_downed() {
        let v = m6c_medical_presets();
        let d = v.iter().find(|p| p.id == DEFIBRILLATOR_ID).unwrap();
        assert!(d.revives_downed);
        assert!((d.hp_restore_fraction - 0.25).abs() < 1e-6);
    }

    #[test]
    fn each_preset_has_unique_id() {
        let v = m6c_medical_presets();
        let mut ids: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
        for p in &v {
            assert!(ids.insert(&p.id), "duplicate id: {}", p.id);
        }
    }
}
