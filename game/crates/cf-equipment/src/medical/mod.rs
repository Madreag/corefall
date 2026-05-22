//! **M14H** extends the M6C medical SKU registry from 12 to 22 SKUs, one
//! per M14H treatment producer. The M6C SKUs remain at their canonical
//! IDs; the M14H additions cover antibiotic courses (T1/T2), antidotes
//! (universal/organophosphate), anti-radiation chelation, painkillers,
//! anti-anxiety benzo, combat stim, cauterize tool, hospital bed object.
//!
//! `m14h_medical_presets()` returns all 22 SKUs in canonical order.

pub mod defibrillator;

use serde::{Deserialize, Serialize};

// ----- M6C: 12 base SKUs -----
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

// ----- M14H: 10 additional SKUs -----
pub const CAUTERIZE_TOOL_ID: &str = "cauterize_tool";
pub const ANTIBIOTIC_COURSE_T1_ID: &str = "antibiotic_course_t1";
pub const ANTIBIOTIC_COURSE_T2_ID: &str = "antibiotic_course_t2";
pub const ANTIDOTE_UNIVERSAL_T1_ID: &str = "antidote_universal_t1";
pub const ANTIDOTE_ORGANOPHOSPHATE_ID: &str = "antidote_organophosphate";
pub const ANTI_RADIATION_CHELATION_ID: &str = "anti_radiation_chelation";
pub const PAINKILLER_OPIOID_T1_ID: &str = "painkiller_opioid_t1";
pub const ANTI_ANXIETY_BENZO_T1_ID: &str = "anti_anxiety_benzo_t1";
pub const COMBAT_STIM_T1_ID: &str = "combat_stim_t1";
pub const HOSPITAL_BED_ID: &str = "hospital_bed";

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
    // M14H additions:
    CauterizeBleed = 12,
    AntibioticCourse = 13,
    AntidoteMild = 14,
    AntidoteNerve = 15,
    AntiRadiationChelation = 16,
    PainkillerOpioid = 17,
    AntiAnxietyBenzo = 18,
    CombatStim = 19,
    HospitalBed = 20,
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
            MedicalEffectKind::CauterizeBleed => "cauterize_bleed",
            MedicalEffectKind::AntibioticCourse => "antibiotic_course",
            MedicalEffectKind::AntidoteMild => "antidote_mild",
            MedicalEffectKind::AntidoteNerve => "antidote_nerve",
            MedicalEffectKind::AntiRadiationChelation => "anti_radiation_chelation",
            MedicalEffectKind::PainkillerOpioid => "painkiller_opioid",
            MedicalEffectKind::AntiAnxietyBenzo => "anti_anxiety_benzo",
            MedicalEffectKind::CombatStim => "combat_stim",
            MedicalEffectKind::HospitalBed => "hospital_bed",
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

/// SKUs spanning the M6C 12 base SKUs + 10 M14H additions. Each entry
/// corresponds 1:1 to a `cf_treatment::TreatmentKind` producer.
#[must_use]
pub fn m14h_medical_presets() -> Vec<MedicalPreset> {
    let mut v = m6c_medical_presets();
    v.push(cauterize_tool());
    v.push(antibiotic_course_t1());
    v.push(antibiotic_course_t2());
    v.push(antidote_universal_t1());
    v.push(antidote_organophosphate());
    v.push(anti_radiation_chelation());
    v.push(painkiller_opioid_t1());
    v.push(anti_anxiety_benzo_t1());
    v.push(combat_stim_t1());
    v.push(hospital_bed());
    v
}

/// table mapping treatment SKUs to per-tier ingredients.
#[must_use]
pub fn m14h_medical_recipes() -> Vec<MedicalRecipe> {
    vec![
        MedicalRecipe::cloth_consumable(FIELD_BANDAGE_ID, &[("cloth", 1)]),
        MedicalRecipe::cloth_consumable(TRAUMA_PACK_ID, &[("cloth", 2), ("gauze", 1)]),
        MedicalRecipe::cloth_consumable(TOURNIQUET_ID, &[("strap", 1)]),
        MedicalRecipe::tool(SUTURES_ID, &[("steel", 1), ("thread", 1)]),
        MedicalRecipe::tool(SPLINT_ID, &[("wood", 2), ("cloth", 1)]),
        MedicalRecipe::tool(SURGERY_KIT_ID, &[("steel", 2), ("alcohol", 1)]),
        MedicalRecipe::tool(DEFIBRILLATOR_ID, &[("electronics", 2), ("battery", 1)]),
        MedicalRecipe::consumable(CPR_COMPRESSIONS_ID, &[]),
        MedicalRecipe::consumable(TRANSFUSION_BAG_ID, &[("blood_pack", 1)]),
        MedicalRecipe::consumable(IV_FLUIDS_ID, &[("saline_bag", 1)]),
        MedicalRecipe::tool(OXYGEN_THERAPY_ID, &[("oxygen_tank", 1)]),
        MedicalRecipe::tool(MEDICAL_SCANNER_T1_ID, &[("electronics", 1), ("sensor", 1)]),
        MedicalRecipe::tool(CAUTERIZE_TOOL_ID, &[("steel", 1), ("battery", 1)]),
        MedicalRecipe::consumable(ANTIBIOTIC_COURSE_T1_ID, &[("antibiotic_pill", 14)]),
        MedicalRecipe::consumable(ANTIBIOTIC_COURSE_T2_ID, &[("antibiotic_pill", 21)]),
        MedicalRecipe::consumable(ANTIDOTE_UNIVERSAL_T1_ID, &[("antidote_serum", 1)]),
        MedicalRecipe::consumable(ANTIDOTE_ORGANOPHOSPHATE_ID, &[("atropine_vial", 1)]),
        MedicalRecipe::consumable(ANTI_RADIATION_CHELATION_ID, &[("chelation_serum", 1)]),
        MedicalRecipe::consumable(PAINKILLER_OPIOID_T1_ID, &[("opioid_pill", 1)]),
        MedicalRecipe::consumable(ANTI_ANXIETY_BENZO_T1_ID, &[("benzo_pill", 1)]),
        MedicalRecipe::consumable(COMBAT_STIM_T1_ID, &[("stim_vial", 1)]),
        MedicalRecipe::furniture(HOSPITAL_BED_ID, &[("steel", 4), ("cloth", 2)]),
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

fn cauterize_tool() -> MedicalPreset {
    MedicalPreset {
        id: CAUTERIZE_TOOL_ID.to_string(),
        display_name: "Cauterize Tool".to_string(),
        kind: MedicalEffectKind::CauterizeBleed,
        apply_seconds: 4.0,
        hp_restore_fraction: 0.05,
        clears_affliction_id: "bleeding_light".to_string(),
        revives_downed: false,
        mass_kg: 0.3,
    }
}

fn antibiotic_course_t1() -> MedicalPreset {
    MedicalPreset {
        id: ANTIBIOTIC_COURSE_T1_ID.to_string(),
        display_name: "Antibiotic Course (T1)".to_string(),
        kind: MedicalEffectKind::AntibioticCourse,
        apply_seconds: 5.0,
        hp_restore_fraction: 0.0,
        clears_affliction_id: "bacterial_infection_early".to_string(),
        revives_downed: false,
        mass_kg: 0.15,
    }
}

fn antibiotic_course_t2() -> MedicalPreset {
    MedicalPreset {
        id: ANTIBIOTIC_COURSE_T2_ID.to_string(),
        display_name: "Antibiotic Course (T2)".to_string(),
        kind: MedicalEffectKind::AntibioticCourse,
        apply_seconds: 5.0,
        hp_restore_fraction: 0.0,
        clears_affliction_id: "bacterial_infection_severe".to_string(),
        revives_downed: false,
        mass_kg: 0.2,
    }
}

fn antidote_universal_t1() -> MedicalPreset {
    MedicalPreset {
        id: ANTIDOTE_UNIVERSAL_T1_ID.to_string(),
        display_name: "Universal Antidote (T1)".to_string(),
        kind: MedicalEffectKind::AntidoteMild,
        apply_seconds: 5.0,
        hp_restore_fraction: 0.0,
        clears_affliction_id: "poisoned".to_string(),
        revives_downed: false,
        mass_kg: 0.1,
    }
}

fn antidote_organophosphate() -> MedicalPreset {
    MedicalPreset {
        id: ANTIDOTE_ORGANOPHOSPHATE_ID.to_string(),
        display_name: "Organophosphate Antidote".to_string(),
        kind: MedicalEffectKind::AntidoteNerve,
        apply_seconds: 5.0,
        hp_restore_fraction: 0.0,
        clears_affliction_id: "nerve_agent_exposure".to_string(),
        revives_downed: false,
        mass_kg: 0.1,
    }
}

fn anti_radiation_chelation() -> MedicalPreset {
    MedicalPreset {
        id: ANTI_RADIATION_CHELATION_ID.to_string(),
        display_name: "Anti-Radiation Chelation".to_string(),
        kind: MedicalEffectKind::AntiRadiationChelation,
        apply_seconds: 60.0,
        hp_restore_fraction: 0.0,
        clears_affliction_id: "radiation_dose".to_string(),
        revives_downed: false,
        mass_kg: 0.3,
    }
}

fn painkiller_opioid_t1() -> MedicalPreset {
    MedicalPreset {
        id: PAINKILLER_OPIOID_T1_ID.to_string(),
        display_name: "Opioid Painkiller (T1)".to_string(),
        kind: MedicalEffectKind::PainkillerOpioid,
        apply_seconds: 3.0,
        hp_restore_fraction: 0.0,
        clears_affliction_id: "pain".to_string(),
        revives_downed: false,
        mass_kg: 0.05,
    }
}

fn anti_anxiety_benzo_t1() -> MedicalPreset {
    MedicalPreset {
        id: ANTI_ANXIETY_BENZO_T1_ID.to_string(),
        display_name: "Anti-Anxiety Benzo (T1)".to_string(),
        kind: MedicalEffectKind::AntiAnxietyBenzo,
        apply_seconds: 3.0,
        hp_restore_fraction: 0.0,
        clears_affliction_id: "panic".to_string(),
        revives_downed: false,
        mass_kg: 0.05,
    }
}

fn combat_stim_t1() -> MedicalPreset {
    MedicalPreset {
        id: COMBAT_STIM_T1_ID.to_string(),
        display_name: "Combat Stim (T1)".to_string(),
        kind: MedicalEffectKind::CombatStim,
        apply_seconds: 2.0,
        hp_restore_fraction: 0.0,
        clears_affliction_id: String::new(),
        revives_downed: false,
        mass_kg: 0.1,
    }
}

fn hospital_bed() -> MedicalPreset {
    MedicalPreset {
        id: HOSPITAL_BED_ID.to_string(),
        display_name: "Hospital Bed".to_string(),
        kind: MedicalEffectKind::HospitalBed,
        apply_seconds: 0.0,
        hp_restore_fraction: 0.0,
        clears_affliction_id: "sleep_dep".to_string(),
        revives_downed: false,
        mass_kg: 60.0,
    }
}

///
/// `category` is one of `"consumable"`, `"tool"`, or `"furniture"` and
/// `ingredients` is a flat ingredient → quantity list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MedicalRecipe {
    pub treatment_id: String,
    pub category: String,
    pub ingredients: Vec<(String, u32)>,
}

impl MedicalRecipe {
    fn from_ingredients(
        treatment_id: &str,
        category: &str,
        ingredients: &[(&str, u32)],
    ) -> Self {
        Self {
            treatment_id: treatment_id.to_string(),
            category: category.to_string(),
            ingredients: ingredients
                .iter()
                .map(|(k, v)| (k.to_string(), *v))
                .collect(),
        }
    }

    pub fn consumable(treatment_id: &str, ingredients: &[(&str, u32)]) -> Self {
        Self::from_ingredients(treatment_id, "consumable", ingredients)
    }

    pub fn cloth_consumable(treatment_id: &str, ingredients: &[(&str, u32)]) -> Self {
        Self::from_ingredients(treatment_id, "consumable", ingredients)
    }

    pub fn tool(treatment_id: &str, ingredients: &[(&str, u32)]) -> Self {
        Self::from_ingredients(treatment_id, "tool", ingredients)
    }

    pub fn furniture(treatment_id: &str, ingredients: &[(&str, u32)]) -> Self {
        Self::from_ingredients(treatment_id, "furniture", ingredients)
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

    #[test]
    fn m14h_registry_has_22_items() {
        assert_eq!(m14h_medical_presets().len(), 22);
    }

    #[test]
    fn m14h_recipe_table_has_22_entries() {
        let recipes = m14h_medical_recipes();
        assert_eq!(recipes.len(), 22);
        for r in &recipes {
            assert!(
                !r.treatment_id.is_empty(),
                "recipe must reference a treatment_id"
            );
            assert!(
                ["consumable", "tool", "furniture"].contains(&r.category.as_str()),
                "recipe category must be canonical: got {}",
                r.category
            );
        }
    }

    #[test]
    fn m14h_each_preset_has_unique_id() {
        let v = m14h_medical_presets();
        let mut ids: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
        for p in &v {
            assert!(ids.insert(&p.id), "duplicate id: {}", p.id);
        }
        assert_eq!(ids.len(), 22);
    }

    #[test]
    fn m14h_recipe_treatment_ids_match_presets() {
        let presets = m14h_medical_presets();
        let recipes = m14h_medical_recipes();
        let preset_ids: std::collections::BTreeSet<&str> =
            presets.iter().map(|p| p.id.as_str()).collect();
        for r in &recipes {
            assert!(
                preset_ids.contains(r.treatment_id.as_str()),
                "recipe references unknown treatment id: {}",
                r.treatment_id
            );
        }
    }
}
