//! Disease spec registry: per-disease metadata (lifecycle timings, cure
//! recipe, vaccine, isolation class, R0) for all 17 launch diseases, plus a
//! `content/diseases/*.ron` loader with a hardcoded boot fallback.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{
    DiseaseKind, IsolationClass, ItemId, OriginId, PathogenClass, TransmissionVector, TreatmentKind,
    IN_GAME_YEAR_SECONDS, PARTIAL_COURSE_RESISTANCE_DRIVE_CHANCE,
};

const HOUR: f32 = 3_600.0;
const DAY: f32 = 86_400.0;
const WEEK: f32 = 604_800.0;
const MONTH: f32 = 2_592_000.0;

/// Consequence of stopping a treatment course before completion.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PartialConsequence {
    /// Disease relapses (returns to Manifest) on an abandoned course.
    pub relapses: bool,
    /// Chance [0,1] the partial course drives a resistant strain.
    pub drives_resistance_chance: f32,
}

impl PartialConsequence {
    pub const NONE: Self = Self {
        relapses: false,
        drives_resistance_chance: 0.0,
    };

    pub const RELAPSE_ONLY: Self = Self {
        relapses: true,
        drives_resistance_chance: 0.0,
    };

    pub const ANTIBIOTIC: Self = Self {
        relapses: true,
        drives_resistance_chance: PARTIAL_COURSE_RESISTANCE_DRIVE_CHANCE,
    };
}

/// Per-disease cure protocol. Mirrors the spec § "Cure recipe schema".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CureRecipe {
    pub treatment_kind: TreatmentKind,
    pub item_required: Option<ItemId>,
    pub dose_count: u8,
    pub dose_interval_hours: f32,
    pub success_chance: f32,
    pub partial_course_consequence: PartialConsequence,
    /// Origins the cure is effective for. Empty = all origins.
    #[serde(default)]
    pub origin_compatibility: BTreeSet<OriginId>,
}

impl CureRecipe {
    /// Self-limiting (bed rest / fluids) — no item, always succeeds.
    pub fn self_limiting(treatment_kind: TreatmentKind) -> Self {
        Self {
            treatment_kind,
            item_required: None,
            dose_count: 0,
            dose_interval_hours: 0.0,
            success_chance: 1.0,
            partial_course_consequence: PartialConsequence::NONE,
            origin_compatibility: BTreeSet::new(),
        }
    }

    /// Fraction [0,1] of the required course that has been completed.
    pub fn completion_fraction(&self, doses_taken: u8) -> f32 {
        if self.dose_count == 0 {
            return 1.0;
        }
        (doses_taken as f32 / self.dose_count as f32).min(1.0)
    }

    pub fn origin_compatible(&self, origin: OriginId) -> bool {
        self.origin_compatibility.is_empty() || self.origin_compatibility.contains(&origin)
    }
}

/// How a vaccine is procured. `Standard` = available off-the-shelf;
/// `DelayedManufacture` = must be manufactured after the strain appears
/// (pandemic vaccine).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VaccineProcurement {
    Standard,
    DelayedManufacture,
}

impl VaccineProcurement {
    pub fn as_str(self) -> &'static str {
        match self {
            VaccineProcurement::Standard => "standard",
            VaccineProcurement::DelayedManufacture => "delayed_manufacture",
        }
    }
}

/// Per-disease vaccine. Procurement + side-effects + immunity duration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VaccineSpec {
    pub vaccine_id: ItemId,
    pub display_name: String,
    pub immunity_duration_seconds: f32,
    pub side_effect_chance: f32,
    pub doses_required: u8,
    pub procurement: VaccineProcurement,
    /// Manufacture lead time (seconds) before a `DelayedManufacture` vaccine
    /// is available. 0 for `Standard`.
    #[serde(default)]
    pub manufacture_lead_seconds: f32,
}

impl VaccineSpec {
    fn standard(id: &str, name: &str) -> Self {
        Self {
            vaccine_id: id.to_string(),
            display_name: name.to_string(),
            immunity_duration_seconds: 5.0 * IN_GAME_YEAR_SECONDS,
            side_effect_chance: 0.05,
            doses_required: 1,
            procurement: VaccineProcurement::Standard,
            manufacture_lead_seconds: 0.0,
        }
    }
}

/// One row of a disease's `transmission_vector_table`: a vector it can be
/// acquired through + the multiplier applied to `r0_per_exposure_event` for
/// that vector.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TransmissionVectorEntry {
    pub vector: TransmissionVector,
    pub relative_r0: f32,
}

impl TransmissionVectorEntry {
    pub fn new(vector: TransmissionVector, relative_r0: f32) -> Self {
        Self { vector, relative_r0 }
    }
}

/// Full per-disease spec. Loaded from `content/diseases/<id>.ron` with a
/// hardcoded boot fallback.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiseaseSpec {
    pub kind: DiseaseKind,
    pub pathogen_class: PathogenClass,
    /// Headline acquisition vector (the default vector for contact spread +
    /// the first entry of `transmission_vector_table`).
    pub primary_vector: TransmissionVector,
    /// Every vector the disease can be acquired through, with per-vector R0
    /// multipliers. Diseases with a single route carry one entry.
    pub transmission_vector_table: Vec<TransmissionVectorEntry>,
    /// Duration of the Incubating stage (seconds).
    pub incubation_seconds: f32,
    /// Duration of the Prodromal stage (seconds). 0 → skip to Manifest.
    pub prodromal_seconds: f32,
    /// Duration of the Manifest stage (seconds). `f32::INFINITY` = indefinite.
    pub manifest_seconds: f32,
    /// Probability [0,1] of death if left untreated through Manifest.
    pub lethality_untreated: f32,
    /// When true, `lethality_untreated` is scaled by the exposure dose
    /// (radiation sickness — "scales with dose").
    #[serde(default)]
    pub lethality_scales_with_dose: bool,
    /// If not cured, the disease becomes Chronic rather than resolving.
    pub becomes_chronic: bool,
    /// Survivors can become asymptomatic Carriers (typhoid).
    pub can_become_carrier: bool,
    /// Spreads human-to-human (false → rabies, tetanus, cancer, etc.).
    pub human_to_human: bool,
    pub isolation_class: IsolationClass,
    /// Base reproduction proxy per exposure event (contact-driven spread).
    pub r0_per_exposure_event: f32,
    pub cure: CureRecipe,
    pub vaccine: Option<VaccineSpec>,
    /// Vaccine/cure only effective before Manifest (rabies post-exposure).
    #[serde(default)]
    pub cure_only_pre_manifest: bool,
}

impl DiseaseSpec {
    /// Hardcoded boot default for one disease. Content RON overrides these.
    pub fn default_for(kind: DiseaseKind) -> Self {
        match kind {
            DiseaseKind::Slimelung => DiseaseSpec {
                kind,
                transmission_vector_table: Self::vectors_for(kind),
                lethality_scales_with_dose: false,
                pathogen_class: PathogenClass::Fungal,
                primary_vector: TransmissionVector::Airborne,
                incubation_seconds: 6.0 * HOUR,
                prodromal_seconds: 1.0 * HOUR,
                manifest_seconds: 24.0 * HOUR,
                lethality_untreated: 0.10,
                becomes_chronic: false,
                can_become_carrier: false,
                human_to_human: true,
                isolation_class: IsolationClass::ClassA,
                r0_per_exposure_event: 2.5,
                cure: CureRecipe {
                    treatment_kind: TreatmentKind::Inhaler,
                    item_required: Some("bronchial_inhaler".to_string()),
                    dose_count: 3,
                    dose_interval_hours: 8.0,
                    success_chance: 0.90,
                    partial_course_consequence: PartialConsequence::RELAPSE_ONLY,
                    origin_compatibility: BTreeSet::new(),
                },
                vaccine: Some(VaccineSpec::standard("cold_slim_vaccine", "Cold-Slim Vaccine")),
                cure_only_pre_manifest: false,
            },
            DiseaseKind::FoodPoisoning => DiseaseSpec {
                kind,
                transmission_vector_table: Self::vectors_for(kind),
                lethality_scales_with_dose: false,
                pathogen_class: PathogenClass::Bacterial,
                primary_vector: TransmissionVector::Foodborne,
                incubation_seconds: 1.0 * HOUR,
                prodromal_seconds: 0.0,
                manifest_seconds: 12.0 * HOUR,
                lethality_untreated: 0.05,
                becomes_chronic: false,
                can_become_carrier: false,
                human_to_human: false,
                isolation_class: IsolationClass::ClassC,
                r0_per_exposure_event: 0.0,
                cure: CureRecipe {
                    treatment_kind: TreatmentKind::Rehydration,
                    item_required: Some("iv_rehydration".to_string()),
                    dose_count: 2,
                    dose_interval_hours: 6.0,
                    success_chance: 0.95,
                    partial_course_consequence: PartialConsequence::RELAPSE_ONLY,
                    origin_compatibility: BTreeSet::new(),
                },
                vaccine: None,
                cure_only_pre_manifest: false,
            },
            DiseaseKind::RadiationSickness => DiseaseSpec {
                kind,
                transmission_vector_table: Self::vectors_for(kind),
                lethality_scales_with_dose: true,
                pathogen_class: PathogenClass::Radiological,
                primary_vector: TransmissionVector::RadiationDose,
                incubation_seconds: 12.0 * HOUR,
                prodromal_seconds: 2.0 * HOUR,
                manifest_seconds: 3.0 * DAY,
                lethality_untreated: 0.30,
                becomes_chronic: false,
                can_become_carrier: false,
                human_to_human: false,
                isolation_class: IsolationClass::ClassB,
                r0_per_exposure_event: 0.0,
                cure: CureRecipe {
                    treatment_kind: TreatmentKind::Chelation,
                    item_required: Some("chelation_injection".to_string()),
                    dose_count: 3,
                    dose_interval_hours: 12.0,
                    success_chance: 0.70,
                    partial_course_consequence: PartialConsequence::RELAPSE_ONLY,
                    origin_compatibility: BTreeSet::new(),
                },
                vaccine: Some(VaccineSpec {
                    vaccine_id: "anti_rad_prophylactic".to_string(),
                    display_name: "Anti-Rad Prophylactic".to_string(),
                    immunity_duration_seconds: 1.0 * DAY,
                    side_effect_chance: 0.10,
                    doses_required: 1,
                    procurement: VaccineProcurement::Standard,
                    manufacture_lead_seconds: 0.0,
                }),
                cure_only_pre_manifest: false,
            },
            DiseaseKind::CommonCold => DiseaseSpec {
                kind,
                transmission_vector_table: Self::vectors_for(kind),
                lethality_scales_with_dose: false,
                pathogen_class: PathogenClass::Viral,
                primary_vector: TransmissionVector::CloseContact,
                incubation_seconds: 12.0 * HOUR,
                prodromal_seconds: 6.0 * HOUR,
                manifest_seconds: 5.0 * DAY,
                lethality_untreated: 0.0,
                becomes_chronic: false,
                can_become_carrier: false,
                human_to_human: true,
                isolation_class: IsolationClass::ClassC,
                r0_per_exposure_event: 1.5,
                cure: CureRecipe::self_limiting(TreatmentKind::BedRest),
                vaccine: None,
                cure_only_pre_manifest: false,
            },
            DiseaseKind::Flu => DiseaseSpec {
                kind,
                transmission_vector_table: Self::vectors_for(kind),
                lethality_scales_with_dose: false,
                pathogen_class: PathogenClass::Viral,
                primary_vector: TransmissionVector::Airborne,
                incubation_seconds: 24.0 * HOUR,
                prodromal_seconds: 12.0 * HOUR,
                manifest_seconds: 7.0 * DAY,
                lethality_untreated: 0.01,
                becomes_chronic: false,
                can_become_carrier: false,
                human_to_human: true,
                isolation_class: IsolationClass::ClassA,
                r0_per_exposure_event: 2.0,
                cure: CureRecipe {
                    treatment_kind: TreatmentKind::Antiviral,
                    item_required: Some("antiviral_t1".to_string()),
                    dose_count: 5,
                    dose_interval_hours: 12.0,
                    success_chance: 0.90,
                    partial_course_consequence: PartialConsequence::RELAPSE_ONLY,
                    origin_compatibility: BTreeSet::new(),
                },
                vaccine: Some(VaccineSpec::standard("flu_vaccine", "Flu Vaccine")),
                cure_only_pre_manifest: false,
            },
            DiseaseKind::Pneumonia => DiseaseSpec {
                kind,
                transmission_vector_table: Self::vectors_for(kind),
                lethality_scales_with_dose: false,
                pathogen_class: PathogenClass::Bacterial,
                primary_vector: TransmissionVector::Airborne,
                incubation_seconds: 24.0 * HOUR,
                prodromal_seconds: 12.0 * HOUR,
                manifest_seconds: 10.0 * DAY,
                lethality_untreated: 0.30,
                becomes_chronic: false,
                can_become_carrier: false,
                human_to_human: true,
                isolation_class: IsolationClass::ClassB,
                r0_per_exposure_event: 1.6,
                cure: CureRecipe {
                    treatment_kind: TreatmentKind::Antibiotic,
                    item_required: Some("antibiotic_course_t1".to_string()),
                    dose_count: 14,
                    dose_interval_hours: 12.0,
                    success_chance: 0.95,
                    partial_course_consequence: PartialConsequence::ANTIBIOTIC,
                    origin_compatibility: BTreeSet::new(),
                },
                vaccine: None,
                cure_only_pre_manifest: false,
            },
            DiseaseKind::Tuberculosis => DiseaseSpec {
                kind,
                transmission_vector_table: Self::vectors_for(kind),
                lethality_scales_with_dose: false,
                pathogen_class: PathogenClass::Bacterial,
                primary_vector: TransmissionVector::Airborne,
                incubation_seconds: 4.0 * WEEK,
                prodromal_seconds: 1.0 * WEEK,
                manifest_seconds: 60.0 * DAY,
                lethality_untreated: 0.50,
                becomes_chronic: true,
                can_become_carrier: true,
                human_to_human: true,
                isolation_class: IsolationClass::ClassA,
                r0_per_exposure_event: 3.0,
                cure: CureRecipe {
                    treatment_kind: TreatmentKind::Antibiotic,
                    item_required: Some("antibiotic_course_t2".to_string()),
                    dose_count: 180,
                    dose_interval_hours: 24.0,
                    success_chance: 0.85,
                    partial_course_consequence: PartialConsequence::ANTIBIOTIC,
                    origin_compatibility: BTreeSet::new(),
                },
                vaccine: Some(VaccineSpec::standard("bcg_vaccine", "BCG Vaccine")),
                cure_only_pre_manifest: false,
            },
            DiseaseKind::Cholera => DiseaseSpec {
                kind,
                transmission_vector_table: Self::vectors_for(kind),
                lethality_scales_with_dose: false,
                pathogen_class: PathogenClass::Bacterial,
                primary_vector: TransmissionVector::Waterborne,
                incubation_seconds: 4.0 * HOUR,
                prodromal_seconds: 0.0,
                manifest_seconds: 5.0 * DAY,
                lethality_untreated: 0.50,
                becomes_chronic: false,
                can_become_carrier: true,
                human_to_human: true,
                isolation_class: IsolationClass::ClassB,
                r0_per_exposure_event: 1.8,
                cure: CureRecipe {
                    treatment_kind: TreatmentKind::Rehydration,
                    item_required: Some("iv_rehydration".to_string()),
                    dose_count: 6,
                    dose_interval_hours: 8.0,
                    success_chance: 0.90,
                    partial_course_consequence: PartialConsequence::ANTIBIOTIC,
                    origin_compatibility: BTreeSet::new(),
                },
                vaccine: Some(VaccineSpec::standard("cholera_vaccine", "Cholera Vaccine")),
                cure_only_pre_manifest: false,
            },
            DiseaseKind::Typhoid => DiseaseSpec {
                kind,
                transmission_vector_table: Self::vectors_for(kind),
                lethality_scales_with_dose: false,
                pathogen_class: PathogenClass::Bacterial,
                primary_vector: TransmissionVector::Waterborne,
                incubation_seconds: 1.0 * WEEK,
                prodromal_seconds: 2.0 * DAY,
                manifest_seconds: 4.0 * WEEK,
                lethality_untreated: 0.20,
                becomes_chronic: false,
                can_become_carrier: true,
                human_to_human: true,
                isolation_class: IsolationClass::ClassB,
                r0_per_exposure_event: 1.7,
                cure: CureRecipe {
                    treatment_kind: TreatmentKind::Antibiotic,
                    item_required: Some("antibiotic_course_t1".to_string()),
                    dose_count: 14,
                    dose_interval_hours: 12.0,
                    success_chance: 0.85,
                    partial_course_consequence: PartialConsequence::ANTIBIOTIC,
                    origin_compatibility: BTreeSet::new(),
                },
                vaccine: Some(VaccineSpec::standard("typhoid_vaccine", "Typhoid Vaccine")),
                cure_only_pre_manifest: false,
            },
            DiseaseKind::Rabies => DiseaseSpec {
                kind,
                transmission_vector_table: Self::vectors_for(kind),
                lethality_scales_with_dose: false,
                pathogen_class: PathogenClass::Viral,
                primary_vector: TransmissionVector::VectorBorne,
                incubation_seconds: 2.0 * MONTH,
                prodromal_seconds: 2.0 * DAY,
                manifest_seconds: 7.0 * DAY,
                lethality_untreated: 1.0,
                becomes_chronic: false,
                can_become_carrier: false,
                human_to_human: false,
                isolation_class: IsolationClass::NotApplicable,
                r0_per_exposure_event: 0.0,
                cure: CureRecipe {
                    treatment_kind: TreatmentKind::PostExposureVaccine,
                    item_required: Some("rabies_vaccine".to_string()),
                    dose_count: 4,
                    dose_interval_hours: 72.0,
                    success_chance: 1.0,
                    partial_course_consequence: PartialConsequence::RELAPSE_ONLY,
                    origin_compatibility: BTreeSet::new(),
                },
                vaccine: Some(VaccineSpec::standard("rabies_vaccine", "Rabies Vaccine")),
                cure_only_pre_manifest: true,
            },
            DiseaseKind::Tetanus => DiseaseSpec {
                kind,
                transmission_vector_table: Self::vectors_for(kind),
                lethality_scales_with_dose: false,
                pathogen_class: PathogenClass::Bacterial,
                primary_vector: TransmissionVector::WoundContact,
                incubation_seconds: 7.0 * DAY,
                prodromal_seconds: 1.0 * DAY,
                manifest_seconds: 10.0 * DAY,
                lethality_untreated: 0.50,
                becomes_chronic: false,
                can_become_carrier: false,
                human_to_human: false,
                isolation_class: IsolationClass::ClassC,
                r0_per_exposure_event: 0.0,
                cure: CureRecipe {
                    treatment_kind: TreatmentKind::Immunoglobulin,
                    item_required: Some("immunoglobulin_tetanus".to_string()),
                    dose_count: 1,
                    dose_interval_hours: 0.0,
                    success_chance: 0.80,
                    partial_course_consequence: PartialConsequence::RELAPSE_ONLY,
                    origin_compatibility: BTreeSet::new(),
                },
                vaccine: Some(VaccineSpec::standard("tetanus_toxoid", "Tetanus Toxoid")),
                cure_only_pre_manifest: false,
            },
            DiseaseKind::BubonicPlague => DiseaseSpec {
                kind,
                transmission_vector_table: Self::vectors_for(kind),
                lethality_scales_with_dose: false,
                pathogen_class: PathogenClass::Bacterial,
                primary_vector: TransmissionVector::VectorBorne,
                incubation_seconds: 3.0 * DAY,
                prodromal_seconds: 1.0 * DAY,
                manifest_seconds: 7.0 * DAY,
                lethality_untreated: 0.60,
                becomes_chronic: false,
                can_become_carrier: false,
                human_to_human: true,
                isolation_class: IsolationClass::ClassA,
                r0_per_exposure_event: 2.2,
                cure: CureRecipe {
                    treatment_kind: TreatmentKind::Antibiotic,
                    item_required: Some("antibiotic_course_t2".to_string()),
                    dose_count: 10,
                    dose_interval_hours: 12.0,
                    success_chance: 0.85,
                    partial_course_consequence: PartialConsequence::ANTIBIOTIC,
                    origin_compatibility: BTreeSet::new(),
                },
                vaccine: Some(VaccineSpec::standard("plague_vaccine", "Plague Vaccine")),
                cure_only_pre_manifest: false,
            },
            DiseaseKind::Anthrax => DiseaseSpec {
                kind,
                transmission_vector_table: Self::vectors_for(kind),
                lethality_scales_with_dose: false,
                pathogen_class: PathogenClass::Bacterial,
                primary_vector: TransmissionVector::SporeExposure,
                incubation_seconds: 1.0 * DAY,
                prodromal_seconds: 6.0 * HOUR,
                manifest_seconds: 4.0 * DAY,
                lethality_untreated: 0.80,
                becomes_chronic: false,
                can_become_carrier: false,
                human_to_human: false,
                isolation_class: IsolationClass::ClassA,
                r0_per_exposure_event: 0.5,
                cure: CureRecipe {
                    treatment_kind: TreatmentKind::Antitoxin,
                    item_required: Some("antitoxin_anthrax".to_string()),
                    dose_count: 7,
                    dose_interval_hours: 12.0,
                    success_chance: 0.70,
                    partial_course_consequence: PartialConsequence::ANTIBIOTIC,
                    origin_compatibility: BTreeSet::new(),
                },
                vaccine: Some(VaccineSpec::standard("anthrax_vaccine", "Anthrax Vaccine")),
                cure_only_pre_manifest: false,
            },
            DiseaseKind::Cancer => DiseaseSpec {
                kind,
                transmission_vector_table: Self::vectors_for(kind),
                lethality_scales_with_dose: false,
                pathogen_class: PathogenClass::Neoplastic,
                primary_vector: TransmissionVector::ToxinAccumulation,
                incubation_seconds: 6.0 * MONTH,
                prodromal_seconds: 1.0 * MONTH,
                manifest_seconds: 6.0 * MONTH,
                lethality_untreated: 0.70,
                becomes_chronic: true,
                can_become_carrier: false,
                human_to_human: false,
                isolation_class: IsolationClass::NotApplicable,
                r0_per_exposure_event: 0.0,
                cure: CureRecipe {
                    treatment_kind: TreatmentKind::Chemotherapy,
                    item_required: Some("chemotherapy_kit".to_string()),
                    dose_count: 6,
                    dose_interval_hours: 720.0,
                    success_chance: 0.50,
                    partial_course_consequence: PartialConsequence::RELAPSE_ONLY,
                    origin_compatibility: BTreeSet::new(),
                },
                vaccine: None,
                cure_only_pre_manifest: false,
            },
            DiseaseKind::MentalIllness => DiseaseSpec {
                kind,
                transmission_vector_table: Self::vectors_for(kind),
                lethality_scales_with_dose: false,
                pathogen_class: PathogenClass::Psychological,
                primary_vector: TransmissionVector::StressAccumulator,
                incubation_seconds: 2.0 * WEEK,
                prodromal_seconds: 1.0 * WEEK,
                manifest_seconds: 30.0 * DAY,
                lethality_untreated: 0.0,
                becomes_chronic: true,
                can_become_carrier: false,
                human_to_human: false,
                isolation_class: IsolationClass::NotApplicable,
                r0_per_exposure_event: 0.0,
                cure: CureRecipe {
                    treatment_kind: TreatmentKind::Therapy,
                    item_required: None,
                    dose_count: 0,
                    dose_interval_hours: 0.0,
                    success_chance: 0.0,
                    partial_course_consequence: PartialConsequence::NONE,
                    origin_compatibility: BTreeSet::new(),
                },
                vaccine: None,
                cure_only_pre_manifest: false,
            },
            DiseaseKind::Sepsis => DiseaseSpec {
                kind,
                transmission_vector_table: Self::vectors_for(kind),
                lethality_scales_with_dose: false,
                pathogen_class: PathogenClass::WoundInfection,
                primary_vector: TransmissionVector::WoundInfection,
                incubation_seconds: 0.0,
                prodromal_seconds: 0.0,
                manifest_seconds: 5.0 * HOUR,
                lethality_untreated: 0.80,
                becomes_chronic: false,
                can_become_carrier: false,
                human_to_human: false,
                isolation_class: IsolationClass::NotApplicable,
                r0_per_exposure_event: 0.0,
                cure: CureRecipe {
                    treatment_kind: TreatmentKind::Antibiotic,
                    item_required: Some("antibiotic_course_t2".to_string()),
                    dose_count: 5,
                    dose_interval_hours: 6.0,
                    success_chance: 0.60,
                    partial_course_consequence: PartialConsequence::ANTIBIOTIC,
                    origin_compatibility: BTreeSet::new(),
                },
                vaccine: None,
                cure_only_pre_manifest: false,
            },
            DiseaseKind::InfluenzaPandemic => DiseaseSpec {
                kind,
                transmission_vector_table: Self::vectors_for(kind),
                lethality_scales_with_dose: false,
                pathogen_class: PathogenClass::Viral,
                primary_vector: TransmissionVector::Airborne,
                incubation_seconds: 18.0 * HOUR,
                prodromal_seconds: 8.0 * HOUR,
                manifest_seconds: 8.0 * DAY,
                lethality_untreated: 0.30,
                becomes_chronic: false,
                can_become_carrier: false,
                human_to_human: true,
                isolation_class: IsolationClass::ClassA,
                r0_per_exposure_event: 4.0,
                cure: CureRecipe {
                    treatment_kind: TreatmentKind::Antiviral,
                    item_required: Some("antiviral_t1".to_string()),
                    dose_count: 5,
                    dose_interval_hours: 12.0,
                    success_chance: 0.70,
                    partial_course_consequence: PartialConsequence::ANTIBIOTIC,
                    origin_compatibility: BTreeSet::new(),
                },
                vaccine: Some(VaccineSpec {
                    vaccine_id: "pandemic_vaccine".to_string(),
                    display_name: "Pandemic Vaccine".to_string(),
                    immunity_duration_seconds: 5.0 * IN_GAME_YEAR_SECONDS,
                    side_effect_chance: 0.08,
                    doses_required: 2,
                    procurement: VaccineProcurement::DelayedManufacture,
                    manufacture_lead_seconds: 14.0 * DAY,
                }),
                cure_only_pre_manifest: false,
            },
        }
    }

    /// Per-disease transmission vector table. Single-route diseases carry one
    /// entry; multi-route diseases (typhoid waterborne/food, pneumonia
    /// airborne/chest-wound, plague critter/pneumonic, cancer radiation/toxin)
    /// list every acquisition vector with its relative R0.
    pub fn vectors_for(kind: DiseaseKind) -> Vec<TransmissionVectorEntry> {
        use TransmissionVector as V;
        let v = TransmissionVectorEntry::new;
        match kind {
            DiseaseKind::Slimelung => vec![v(V::Airborne, 1.0)],
            DiseaseKind::FoodPoisoning => vec![v(V::Foodborne, 1.0)],
            DiseaseKind::RadiationSickness => vec![v(V::RadiationDose, 1.0)],
            DiseaseKind::CommonCold => vec![v(V::CloseContact, 1.0)],
            DiseaseKind::Flu => vec![v(V::Airborne, 1.0)],
            DiseaseKind::Pneumonia => vec![v(V::Airborne, 1.0), v(V::WoundContact, 0.4)],
            DiseaseKind::Tuberculosis => vec![v(V::Airborne, 1.0)],
            DiseaseKind::Cholera => vec![v(V::Waterborne, 1.0)],
            DiseaseKind::Typhoid => vec![v(V::Waterborne, 1.0), v(V::Foodborne, 0.7)],
            DiseaseKind::Rabies => vec![v(V::VectorBorne, 1.0)],
            DiseaseKind::Tetanus => vec![v(V::WoundContact, 1.0)],
            DiseaseKind::BubonicPlague => vec![v(V::VectorBorne, 1.0), v(V::Airborne, 0.3)],
            DiseaseKind::Anthrax => vec![v(V::SporeExposure, 1.0)],
            DiseaseKind::Cancer => vec![v(V::ToxinAccumulation, 1.0), v(V::RadiationDose, 1.0)],
            DiseaseKind::MentalIllness => vec![v(V::StressAccumulator, 1.0)],
            DiseaseKind::Sepsis => vec![v(V::WoundInfection, 1.0)],
            DiseaseKind::InfluenzaPandemic => vec![v(V::Airborne, 1.0)],
        }
    }

    /// True when the disease can be acquired through `vector`.
    pub fn has_vector(&self, vector: TransmissionVector) -> bool {
        self.transmission_vector_table.iter().any(|e| e.vector == vector)
    }

    /// Effective R0 for a given vector (base × the table's relative_r0).
    /// Returns 0 when the disease doesn't use that vector.
    pub fn r0_for_vector(&self, vector: TransmissionVector) -> f32 {
        self.transmission_vector_table
            .iter()
            .find(|e| e.vector == vector)
            .map(|e| self.r0_per_exposure_event * e.relative_r0)
            .unwrap_or(0.0)
    }

    /// The contagious (person-to-person) vector used for contact spread, if
    /// the disease has one.
    pub fn contagious_vector(&self) -> Option<TransmissionVector> {
        self.transmission_vector_table
            .iter()
            .map(|e| e.vector)
            .find(|v| v.is_contagious())
    }

    /// Total seconds from exposure to the outcome decision at end of Manifest.
    pub fn total_course_seconds(&self) -> f32 {
        self.incubation_seconds + self.prodromal_seconds + self.manifest_seconds
    }
}

/// Disease registry — `DiseaseKind` id → spec. Loaded from
/// `content/diseases/*.ron` with a hardcoded boot fallback.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DiseaseRegistry {
    pub specs: BTreeMap<String, DiseaseSpec>,
}

impl DiseaseRegistry {
    pub fn default_registry() -> Self {
        let mut specs = BTreeMap::new();
        for &k in DiseaseKind::all() {
            specs.insert(k.as_str().to_string(), DiseaseSpec::default_for(k));
        }
        Self { specs }
    }

    pub fn lookup(&self, kind: DiseaseKind) -> &DiseaseSpec {
        self.specs
            .get(kind.as_str())
            .expect("disease registry must contain every kind")
    }

    pub fn get(&self, kind: DiseaseKind) -> Option<&DiseaseSpec> {
        self.specs.get(kind.as_str())
    }

    /// Loads every `content/diseases/*.ron` file (ignoring files prefixed
    /// with `_`, e.g. the susceptibility matrix). Falls back to
    /// `default_for` for kinds whose RON file is missing.
    pub fn load_dir(dir: &Path) -> Result<Self, DiseaseLoadError> {
        let mut reg = Self::default_registry();
        if !dir.exists() {
            return Ok(reg);
        }
        let read_dir =
            fs::read_dir(dir).map_err(|e| DiseaseLoadError::Io(dir.to_path_buf(), e.to_string()))?;
        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("ron") {
                continue;
            }
            if path
                .file_name()
                .and_then(|s| s.to_str())
                .map(|n| n.starts_with('_'))
                .unwrap_or(false)
            {
                continue;
            }
            let body = fs::read_to_string(&path)
                .map_err(|e| DiseaseLoadError::Io(path.clone(), e.to_string()))?;
            match ron::from_str::<DiseaseSpec>(&body) {
                Ok(spec) => {
                    reg.specs.insert(spec.kind.as_str().to_string(), spec);
                }
                Err(e) => {
                    tracing::warn!(path = %path.display(), error = %e, "disease spec parse failed; keeping default");
                    return Err(DiseaseLoadError::Parse(path.clone(), e.to_string()));
                }
            }
        }
        Ok(reg)
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum DiseaseLoadError {
    #[error("io error reading {0:?}: {1}")]
    Io(PathBuf, String),
    #[error("parse error in {0:?}: {1}")]
    Parse(PathBuf, String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_registry_has_all_17() {
        let reg = DiseaseRegistry::default_registry();
        assert_eq!(reg.specs.len(), 17);
        for &k in DiseaseKind::all() {
            assert!(reg.specs.contains_key(k.as_str()), "missing {}", k.as_str());
        }
    }

    #[test]
    fn pneumonia_uses_14_dose_antibiotic_course() {
        let reg = DiseaseRegistry::default_registry();
        let spec = reg.lookup(DiseaseKind::Pneumonia);
        assert_eq!(spec.cure.dose_count, 14);
        assert_eq!(spec.cure.treatment_kind, TreatmentKind::Antibiotic);
        assert!(spec.cure.partial_course_consequence.relapses);
        assert!(spec.cure.partial_course_consequence.drives_resistance_chance > 0.0);
    }

    #[test]
    fn rabies_is_not_human_to_human_and_cure_pre_manifest_only() {
        let reg = DiseaseRegistry::default_registry();
        let spec = reg.lookup(DiseaseKind::Rabies);
        assert!(!spec.human_to_human);
        assert!(spec.cure_only_pre_manifest);
        assert_eq!(spec.isolation_class, IsolationClass::NotApplicable);
        assert!((spec.lethality_untreated - 1.0).abs() < 1e-6);
    }

    #[test]
    fn pandemic_vaccine_is_delayed_manufacture() {
        let reg = DiseaseRegistry::default_registry();
        let spec = reg.lookup(DiseaseKind::InfluenzaPandemic);
        let v = spec.vaccine.as_ref().expect("pandemic vaccine exists");
        assert_eq!(v.procurement, VaccineProcurement::DelayedManufacture);
        assert!(v.manufacture_lead_seconds > 0.0);
    }

    #[test]
    fn completion_fraction_threshold() {
        let reg = DiseaseRegistry::default_registry();
        let spec = reg.lookup(DiseaseKind::Pneumonia);
        // 11 of 14 doses = 0.785 < 0.80 threshold → partial.
        assert!(spec.cure.completion_fraction(11) < crate::ANTIBIOTIC_COURSE_COMPLETION_THRESHOLD);
        assert!(spec.cure.completion_fraction(14) >= crate::ANTIBIOTIC_COURSE_COMPLETION_THRESHOLD);
    }

    #[test]
    fn every_disease_has_a_transmission_vector_table_with_primary() {
        let reg = DiseaseRegistry::default_registry();
        for &k in DiseaseKind::all() {
            let spec = reg.lookup(k);
            assert!(
                !spec.transmission_vector_table.is_empty(),
                "{} has an empty transmission_vector_table",
                k.as_str()
            );
            assert!(
                spec.has_vector(spec.primary_vector),
                "{} table missing its primary_vector",
                k.as_str()
            );
        }
    }

    #[test]
    fn multi_vector_diseases_declare_all_routes() {
        let reg = DiseaseRegistry::default_registry();
        // Typhoid: waterborne + food.
        let typhoid = reg.lookup(DiseaseKind::Typhoid);
        assert!(typhoid.has_vector(TransmissionVector::Waterborne));
        assert!(typhoid.has_vector(TransmissionVector::Foodborne));
        // Cancer: radiation OR toxin.
        let cancer = reg.lookup(DiseaseKind::Cancer);
        assert!(cancer.has_vector(TransmissionVector::RadiationDose));
        assert!(cancer.has_vector(TransmissionVector::ToxinAccumulation));
        // Plague: critter (primary) + weaker pneumonic (airborne).
        let plague = reg.lookup(DiseaseKind::BubonicPlague);
        assert!(plague.has_vector(TransmissionVector::VectorBorne));
        assert!(plague.r0_for_vector(TransmissionVector::Airborne) < plague.r0_for_vector(TransmissionVector::VectorBorne));
    }

    #[test]
    fn radiation_lethality_scales_with_dose_flag_set() {
        let reg = DiseaseRegistry::default_registry();
        assert!(reg.lookup(DiseaseKind::RadiationSickness).lethality_scales_with_dose);
        assert!(!reg.lookup(DiseaseKind::Pneumonia).lethality_scales_with_dose);
    }

    #[test]
    fn food_poisoning_cure_is_a_real_cure_file() {
        let reg = DiseaseRegistry::default_registry();
        assert_eq!(
            reg.lookup(DiseaseKind::FoodPoisoning).cure.item_required.as_deref(),
            Some("iv_rehydration")
        );
    }

    #[test]
    fn spec_round_trips_through_ron() {
        for &k in DiseaseKind::all() {
            let spec = DiseaseSpec::default_for(k);
            let s = ron::to_string(&spec).unwrap();
            let back: DiseaseSpec = ron::from_str(&s).unwrap();
            assert_eq!(spec, back, "round-trip failed for {}", k.as_str());
        }
    }

    #[test]
    fn on_disk_content_loads() {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../content/diseases");
        if !dir.exists() {
            return;
        }
        let reg = DiseaseRegistry::load_dir(&dir).expect("content/diseases must parse");
        assert_eq!(reg.specs.len(), 17, "all 17 disease RON files must load");
        // The matrix file (prefixed `_`) is skipped by the spec loader.
        let matrix_path = dir.join("_susceptibility_matrix.ron");
        if matrix_path.exists() {
            let m = crate::SusceptibilityMatrix::load_file(&matrix_path)
                .expect("susceptibility matrix must parse");
            assert_eq!(m.grid.len(), 10);
        }
    }
}
