//! **M14H** § 22 treatment producers.
//!
//! Each producer is identified by a [`TreatmentKind`] enum variant.
//! [`treatment_catalog()`] returns the static catalog of [`TreatmentSpec`]
//! records describing apply time, tool requirements, skill requirements,
//! and risk surfaces per the M14H spec table.

use serde::{Deserialize, Serialize};

pub const BANDAGE_SOAK_THROUGH_SECONDS: f32 = 180.0;

pub const TOURNIQUET_NECROSIS_THRESHOLD_SECONDS: f32 = 90.0 * 60.0;

pub const MEDIC_T1_SKILL_PASS_RATE_X1000: u32 = 700;

pub const SURGEON_T1_SKILL_PASS_RATE_X1000: u32 = 900;

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TreatmentKind {
    FieldBandageV1 = 0,
    TraumaPackV1 = 1,
    TourniquetV1 = 2,
    SuturesV1 = 3,
    CauterizeV1 = 4,
    SplintV1 = 5,
    SurgeryKitV1 = 6,
    DefibrillatorV1 = 7,
    CprManual = 8,
    TransfusionBagV1 = 9,
    IvFluidsV1 = 10,
    OxygenTherapyV1 = 11,
    AntibioticCourseT1 = 12,
    AntibioticCourseT2 = 13,
    AntidoteUniversalT1 = 14,
    AntidoteOrganophosphate = 15,
    AntiRadiationChelation = 16,
    PainkillerOpioidT1 = 17,
    AntiAnxietyBenzoT1 = 18,
    CombatStimT1 = 19,
    MedicalScannerT1 = 20,
    HospitalBedV1 = 21,
}

impl TreatmentKind {
    pub const COUNT: usize = 22;

    pub const ALL: [TreatmentKind; Self::COUNT] = [
        TreatmentKind::FieldBandageV1,
        TreatmentKind::TraumaPackV1,
        TreatmentKind::TourniquetV1,
        TreatmentKind::SuturesV1,
        TreatmentKind::CauterizeV1,
        TreatmentKind::SplintV1,
        TreatmentKind::SurgeryKitV1,
        TreatmentKind::DefibrillatorV1,
        TreatmentKind::CprManual,
        TreatmentKind::TransfusionBagV1,
        TreatmentKind::IvFluidsV1,
        TreatmentKind::OxygenTherapyV1,
        TreatmentKind::AntibioticCourseT1,
        TreatmentKind::AntibioticCourseT2,
        TreatmentKind::AntidoteUniversalT1,
        TreatmentKind::AntidoteOrganophosphate,
        TreatmentKind::AntiRadiationChelation,
        TreatmentKind::PainkillerOpioidT1,
        TreatmentKind::AntiAnxietyBenzoT1,
        TreatmentKind::CombatStimT1,
        TreatmentKind::MedicalScannerT1,
        TreatmentKind::HospitalBedV1,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            TreatmentKind::FieldBandageV1 => "field_bandage_v1",
            TreatmentKind::TraumaPackV1 => "trauma_pack_v1",
            TreatmentKind::TourniquetV1 => "tourniquet_v1",
            TreatmentKind::SuturesV1 => "sutures_v1",
            TreatmentKind::CauterizeV1 => "cauterize_v1",
            TreatmentKind::SplintV1 => "splint_v1",
            TreatmentKind::SurgeryKitV1 => "surgery_kit_v1",
            TreatmentKind::DefibrillatorV1 => "defibrillator_v1",
            TreatmentKind::CprManual => "cpr_manual",
            TreatmentKind::TransfusionBagV1 => "transfusion_bag_v1",
            TreatmentKind::IvFluidsV1 => "iv_fluids_v1",
            TreatmentKind::OxygenTherapyV1 => "oxygen_therapy_v1",
            TreatmentKind::AntibioticCourseT1 => "antibiotic_course_t1",
            TreatmentKind::AntibioticCourseT2 => "antibiotic_course_t2",
            TreatmentKind::AntidoteUniversalT1 => "antidote_universal_t1",
            TreatmentKind::AntidoteOrganophosphate => "antidote_organophosphate",
            TreatmentKind::AntiRadiationChelation => "anti_radiation_chelation",
            TreatmentKind::PainkillerOpioidT1 => "painkiller_opioid_t1",
            TreatmentKind::AntiAnxietyBenzoT1 => "anti_anxiety_benzo_t1",
            TreatmentKind::CombatStimT1 => "combat_stim_t1",
            TreatmentKind::MedicalScannerT1 => "medical_scanner_t1",
            TreatmentKind::HospitalBedV1 => "hospital_bed_v1",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        for v in &Self::ALL {
            if v.as_str() == s {
                return Some(*v);
            }
        }
        None
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolRequirement {
    None,
    SutureKit,
    HotMetalOrPlasma,
    SplintMaterial,
    SurgeryTableAndSurgeon,
    IvKit,
    OxygenTank,
    ScannerDevice,
    BedObject,
}

impl ToolRequirement {
    pub fn as_str(self) -> &'static str {
        match self {
            ToolRequirement::None => "none",
            ToolRequirement::SutureKit => "suture_kit",
            ToolRequirement::HotMetalOrPlasma => "hot_metal_or_plasma",
            ToolRequirement::SplintMaterial => "splint_material",
            ToolRequirement::SurgeryTableAndSurgeon => "surgery_table_and_surgeon",
            ToolRequirement::IvKit => "iv_kit",
            ToolRequirement::OxygenTank => "oxygen_tank",
            ToolRequirement::ScannerDevice => "scanner_device",
            ToolRequirement::BedObject => "bed_object",
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillRequirement {
    None,
    MedicT1,
    MedicT2,
    SurgeonT1,
}

impl SkillRequirement {
    pub fn as_str(self) -> &'static str {
        match self {
            SkillRequirement::None => "none",
            SkillRequirement::MedicT1 => "medic_t1",
            SkillRequirement::MedicT2 => "medic_t2",
            SkillRequirement::SurgeonT1 => "surgeon_t1",
        }
    }

    /// True if the patient's actor satisfies this skill requirement.
    pub fn satisfied_by(self, has_medic_t1: bool, has_medic_t2: bool, has_surgeon_t1: bool) -> bool {
        match self {
            SkillRequirement::None => true,
            SkillRequirement::MedicT1 => has_medic_t1 || has_medic_t2 || has_surgeon_t1,
            SkillRequirement::MedicT2 => has_medic_t2,
            SkillRequirement::SurgeonT1 => has_surgeon_t1,
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskKind {
    None,
    NecrosisIfUnreleased,
    FailureRollOnDirtyWound,
    InfectionIfDirty,
    BleedOutDuringSurgery,
    BurnAtChestPerShock,
    BruisePerExtendedCpr,
    BloodReactionRoll,
    InfiltrationRisk,
    ResistanceIfIncomplete,
    ResistanceAndSideEffects,
    IncompatibleWithSeverePoison,
    CardiacSideEffects,
    NauseaSideEffect,
    AddictionPerM16C,
}

impl RiskKind {
    pub fn as_str(self) -> &'static str {
        match self {
            RiskKind::None => "none",
            RiskKind::NecrosisIfUnreleased => "necrosis_if_unreleased",
            RiskKind::FailureRollOnDirtyWound => "failure_roll_on_dirty_wound",
            RiskKind::InfectionIfDirty => "infection_if_dirty",
            RiskKind::BleedOutDuringSurgery => "bleed_out_during_surgery",
            RiskKind::BurnAtChestPerShock => "burn_at_chest_per_shock",
            RiskKind::BruisePerExtendedCpr => "bruise_per_extended_cpr",
            RiskKind::BloodReactionRoll => "blood_reaction_roll",
            RiskKind::InfiltrationRisk => "infiltration_risk",
            RiskKind::ResistanceIfIncomplete => "resistance_if_incomplete",
            RiskKind::ResistanceAndSideEffects => "resistance_and_side_effects",
            RiskKind::IncompatibleWithSeverePoison => "incompatible_with_severe_poison",
            RiskKind::CardiacSideEffects => "cardiac_side_effects",
            RiskKind::NauseaSideEffect => "nausea_side_effect",
            RiskKind::AddictionPerM16C => "addiction_per_m16c",
        }
    }
}

///
/// Equivalent to a row in the spec's 22-row producer table. Loaded from
/// `content/treatments/<id>.ron` or via `treatment_catalog()` (baked
/// defaults).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TreatmentSpec {
    pub kind: TreatmentKind,
    pub display_name: String,
    pub apply_seconds_min: f32,
    pub apply_seconds_max: f32,
    pub tool: ToolRequirement,
    pub skill: SkillRequirement,
    pub risk: RiskKind,
    /// True if this producer targets cardiac-arrest events (CPR / defib).
    pub revives_downed: bool,
    /// True if origin-aware (robot vs human) — failures emit
    /// `treatment.failed reason="wrong_origin"` per Gherkin scenario 6.
    pub origin_aware: bool,
    /// Optional charges (defibrillator = 4 charges per pack).
    pub charges: Option<u32>,
    /// Course-based producer (antibiotics) — doses per full course.
    pub doses_per_course: Option<u32>,
    /// Course-based producer interval in hours between doses.
    pub dose_interval_hours: Option<f32>,
}

impl TreatmentSpec {
    /// Average apply time in seconds (used for cfctl progress + AI ETA).
    pub fn apply_seconds_avg(&self) -> f32 {
        (self.apply_seconds_min + self.apply_seconds_max) * 0.5
    }
}

fn spec(
    kind: TreatmentKind,
    display_name: &str,
    apply_min: f32,
    apply_max: f32,
    tool: ToolRequirement,
    skill: SkillRequirement,
    risk: RiskKind,
) -> TreatmentSpec {
    TreatmentSpec {
        kind,
        display_name: display_name.to_string(),
        apply_seconds_min: apply_min,
        apply_seconds_max: apply_max,
        tool,
        skill,
        risk,
        revives_downed: false,
        origin_aware: false,
        charges: None,
        doses_per_course: None,
        dose_interval_hours: None,
    }
}

#[must_use]
pub fn treatment_catalog() -> Vec<TreatmentSpec> {
    vec![
        TreatmentSpec {
            origin_aware: true,
            ..spec(
                TreatmentKind::FieldBandageV1,
                "Field Bandage",
                5.0,
                5.0,
                ToolRequirement::None,
                SkillRequirement::None,
                RiskKind::None,
            )
        },
        TreatmentSpec {
            origin_aware: true,
            ..spec(
                TreatmentKind::TraumaPackV1,
                "Trauma Pack",
                15.0,
                15.0,
                ToolRequirement::None,
                SkillRequirement::MedicT1,
                RiskKind::None,
            )
        },
        TreatmentSpec {
            origin_aware: true,
            ..spec(
                TreatmentKind::TourniquetV1,
                "Tourniquet",
                8.0,
                8.0,
                ToolRequirement::None,
                SkillRequirement::None,
                RiskKind::NecrosisIfUnreleased,
            )
        },
        TreatmentSpec {
            origin_aware: true,
            ..spec(
                TreatmentKind::SuturesV1,
                "Sutures",
                30.0,
                30.0,
                ToolRequirement::SutureKit,
                SkillRequirement::MedicT1,
                RiskKind::FailureRollOnDirtyWound,
            )
        },
        TreatmentSpec {
            origin_aware: true,
            ..spec(
                TreatmentKind::CauterizeV1,
                "Cauterize",
                4.0,
                4.0,
                ToolRequirement::HotMetalOrPlasma,
                SkillRequirement::MedicT1,
                RiskKind::InfectionIfDirty,
            )
        },
        TreatmentSpec {
            origin_aware: true,
            ..spec(
                TreatmentKind::SplintV1,
                "Splint",
                20.0,
                20.0,
                ToolRequirement::SplintMaterial,
                SkillRequirement::MedicT1,
                RiskKind::None,
            )
        },
        TreatmentSpec {
            origin_aware: true,
            ..spec(
                TreatmentKind::SurgeryKitV1,
                "Surgery Kit",
                60.0,
                180.0,
                ToolRequirement::SurgeryTableAndSurgeon,
                SkillRequirement::SurgeonT1,
                RiskKind::BleedOutDuringSurgery,
            )
        },
        TreatmentSpec {
            revives_downed: true,
            origin_aware: true,
            charges: Some(4),
            ..spec(
                TreatmentKind::DefibrillatorV1,
                "Defibrillator",
                5.0,
                5.0,
                ToolRequirement::None,
                SkillRequirement::MedicT1,
                RiskKind::BurnAtChestPerShock,
            )
        },
        TreatmentSpec {
            revives_downed: true,
            origin_aware: true,
            ..spec(
                TreatmentKind::CprManual,
                "CPR (Manual)",
                20.0,
                20.0,
                ToolRequirement::None,
                SkillRequirement::None,
                RiskKind::BruisePerExtendedCpr,
            )
        },
        TreatmentSpec {
            origin_aware: true,
            ..spec(
                TreatmentKind::TransfusionBagV1,
                "Transfusion Bag",
                60.0,
                60.0,
                ToolRequirement::IvKit,
                SkillRequirement::MedicT1,
                RiskKind::BloodReactionRoll,
            )
        },
        TreatmentSpec {
            origin_aware: true,
            ..spec(
                TreatmentKind::IvFluidsV1,
                "IV Fluids",
                120.0,
                120.0,
                ToolRequirement::IvKit,
                SkillRequirement::MedicT1,
                RiskKind::InfiltrationRisk,
            )
        },
        TreatmentSpec {
            origin_aware: true,
            ..spec(
                TreatmentKind::OxygenTherapyV1,
                "Oxygen Therapy",
                60.0,
                60.0,
                ToolRequirement::OxygenTank,
                SkillRequirement::MedicT1,
                RiskKind::None,
            )
        },
        TreatmentSpec {
            origin_aware: true,
            doses_per_course: Some(14),
            dose_interval_hours: Some(8.0),
            ..spec(
                TreatmentKind::AntibioticCourseT1,
                "Antibiotic Course (T1)",
                5.0,
                5.0,
                ToolRequirement::None,
                SkillRequirement::MedicT1,
                RiskKind::ResistanceIfIncomplete,
            )
        },
        TreatmentSpec {
            origin_aware: true,
            doses_per_course: Some(21),
            dose_interval_hours: Some(6.0),
            ..spec(
                TreatmentKind::AntibioticCourseT2,
                "Antibiotic Course (T2)",
                5.0,
                5.0,
                ToolRequirement::None,
                SkillRequirement::MedicT2,
                RiskKind::ResistanceAndSideEffects,
            )
        },
        TreatmentSpec {
            origin_aware: true,
            ..spec(
                TreatmentKind::AntidoteUniversalT1,
                "Universal Antidote (T1)",
                5.0,
                5.0,
                ToolRequirement::None,
                SkillRequirement::MedicT1,
                RiskKind::IncompatibleWithSeverePoison,
            )
        },
        TreatmentSpec {
            origin_aware: true,
            ..spec(
                TreatmentKind::AntidoteOrganophosphate,
                "Organophosphate Antidote",
                5.0,
                5.0,
                ToolRequirement::None,
                SkillRequirement::MedicT1,
                RiskKind::CardiacSideEffects,
            )
        },
        TreatmentSpec {
            origin_aware: true,
            ..spec(
                TreatmentKind::AntiRadiationChelation,
                "Anti-Radiation Chelation",
                60.0,
                60.0,
                ToolRequirement::IvKit,
                SkillRequirement::MedicT1,
                RiskKind::NauseaSideEffect,
            )
        },
        TreatmentSpec {
            origin_aware: true,
            ..spec(
                TreatmentKind::PainkillerOpioidT1,
                "Opioid Painkiller (T1)",
                3.0,
                3.0,
                ToolRequirement::None,
                SkillRequirement::MedicT1,
                RiskKind::AddictionPerM16C,
            )
        },
        TreatmentSpec {
            origin_aware: true,
            ..spec(
                TreatmentKind::AntiAnxietyBenzoT1,
                "Anti-Anxiety Benzo (T1)",
                3.0,
                3.0,
                ToolRequirement::None,
                SkillRequirement::MedicT1,
                RiskKind::AddictionPerM16C,
            )
        },
        TreatmentSpec {
            origin_aware: true,
            ..spec(
                TreatmentKind::CombatStimT1,
                "Combat Stim (T1)",
                2.0,
                2.0,
                ToolRequirement::None,
                SkillRequirement::None,
                RiskKind::AddictionPerM16C,
            )
        },
        TreatmentSpec {
            origin_aware: true,
            ..spec(
                TreatmentKind::MedicalScannerT1,
                "Medical Scanner (T1)",
                30.0,
                30.0,
                ToolRequirement::ScannerDevice,
                SkillRequirement::MedicT1,
                RiskKind::None,
            )
        },
        TreatmentSpec {
            origin_aware: false,
            ..spec(
                TreatmentKind::HospitalBedV1,
                "Hospital Bed",
                0.0,
                0.0,
                ToolRequirement::BedObject,
                SkillRequirement::None,
                RiskKind::None,
            )
        },
    ]
}

/// Look up a treatment spec by kind. Panics if the catalog is malformed
/// (only callable from the bundled baked-defaults path; tests guard the
/// 22-entry invariant).
#[must_use]
pub fn treatment_spec(kind: TreatmentKind) -> TreatmentSpec {
    treatment_catalog()
        .into_iter()
        .find(|s| s.kind == kind)
        .expect("treatment catalog must contain every TreatmentKind")
}

/// `content/treatments/*.ron`. Mirrors `cf-wound::WoundSpecRegistry`.
#[derive(Debug, Clone, Default)]
pub struct TreatmentSpecRegistry {
    pub(crate) by_kind: std::collections::BTreeMap<TreatmentKind, TreatmentSpec>,
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum TreatmentSpecError {
    #[error("io error: {0}")]
    Io(String),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("duplicate TreatmentKind: {0:?}")]
    DuplicateKind(TreatmentKind),
}

impl TreatmentSpecRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load(dir: &std::path::Path) -> Result<Self, TreatmentSpecError> {
        let mut registry = TreatmentSpecRegistry::new();
        let entries = std::fs::read_dir(dir).map_err(|e| TreatmentSpecError::Io(e.to_string()))?;
        let mut paths: Vec<std::path::PathBuf> = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| TreatmentSpecError::Io(e.to_string()))?;
            let p = entry.path();
            if p.extension().and_then(|x| x.to_str()) != Some("ron") {
                continue;
            }
            paths.push(p);
        }
        paths.sort();
        for path in paths {
            let raw =
                std::fs::read_to_string(&path).map_err(|e| TreatmentSpecError::Io(e.to_string()))?;
            let spec: TreatmentSpec = ron::from_str(&raw)
                .map_err(|e| TreatmentSpecError::Parse(format!("{:?}: {}", path, e)))?;
            if registry.by_kind.contains_key(&spec.kind) {
                return Err(TreatmentSpecError::DuplicateKind(spec.kind));
            }
            registry.by_kind.insert(spec.kind, spec);
        }
        Ok(registry)
    }

    pub fn baked_default() -> Self {
        let mut registry = TreatmentSpecRegistry::new();
        for s in treatment_catalog() {
            registry.by_kind.insert(s.kind, s);
        }
        registry
    }

    pub fn get(&self, kind: TreatmentKind) -> Option<&TreatmentSpec> {
        self.by_kind.get(&kind)
    }

    pub fn len(&self) -> usize {
        self.by_kind.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_kind.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&TreatmentKind, &TreatmentSpec)> {
        self.by_kind.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_has_22_producers() {
        let c = treatment_catalog();
        assert_eq!(c.len(), TreatmentKind::COUNT);
        assert_eq!(c.len(), 22);
    }

    #[test]
    fn every_kind_has_a_spec() {
        let c = treatment_catalog();
        for kind in TreatmentKind::ALL.iter() {
            assert!(
                c.iter().any(|s| s.kind == *kind),
                "missing spec for {:?}",
                kind
            );
        }
    }

    #[test]
    fn kind_round_trip_via_str() {
        for kind in TreatmentKind::ALL.iter() {
            let s = kind.as_str();
            let back = TreatmentKind::from_str(s).expect("round-trip");
            assert_eq!(back, *kind);
        }
    }

    #[test]
    fn defib_has_4_charges() {
        let s = treatment_spec(TreatmentKind::DefibrillatorV1);
        assert_eq!(s.charges, Some(4));
        assert!(s.revives_downed);
    }

    #[test]
    fn antibiotic_courses_have_dose_counts() {
        let t1 = treatment_spec(TreatmentKind::AntibioticCourseT1);
        let t2 = treatment_spec(TreatmentKind::AntibioticCourseT2);
        assert_eq!(t1.doses_per_course, Some(14));
        assert_eq!(t2.doses_per_course, Some(21));
        assert_eq!(t1.dose_interval_hours, Some(8.0));
        assert_eq!(t2.dose_interval_hours, Some(6.0));
    }

    #[test]
    fn skill_requirement_satisfied() {
        assert!(SkillRequirement::None.satisfied_by(false, false, false));
        assert!(!SkillRequirement::MedicT1.satisfied_by(false, false, false));
        assert!(SkillRequirement::MedicT1.satisfied_by(true, false, false));
        assert!(SkillRequirement::MedicT1.satisfied_by(false, true, false));
        assert!(SkillRequirement::MedicT1.satisfied_by(false, false, true));
        assert!(!SkillRequirement::SurgeonT1.satisfied_by(true, false, false));
        assert!(SkillRequirement::SurgeonT1.satisfied_by(false, false, true));
    }
}
