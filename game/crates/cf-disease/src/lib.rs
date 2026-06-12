//! M16B — canonical disease registry, per-disease lifecycle FSM, cure +
//! vaccine recipe, isolation protocol, R0 spread model, and the per-origin
//! susceptibility matrix (10 races × 17 diseases).
//!
//! This crate owns the data + deterministic kernels; consumers wire them:
//!   - `cf-actor::diseases` holds per-actor multi-disease state.
//!   - `cf-environment::germ_spread` drives R0 transmission per vector.
//!   - `cf-environment::room_grading` classifies quarantine rooms.
//!   - `cf-equipment::{cures,vaccines,medical_scanner}` ship the items.
//!   - `cf-storyteller::pandemic` registers the pandemic narrative beat.
//!
//! Determinism: no `thread_rng`. Stochastic outcomes (lethality, partial
//! course resistance) derive from a seeded `deterministic_roll` keyed by
//! (seed, actor_id, disease, salt) so identical inputs reproduce identical
//! infected/recovered/death counts.

#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(
    clippy::module_name_repetitions,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::doc_markdown,
    clippy::missing_const_for_fn,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_lossless,
    clippy::struct_field_names,
    clippy::struct_excessive_bools,
    clippy::match_same_arms,
    clippy::unused_self,
    clippy::similar_names,
    clippy::too_many_lines,
    clippy::too_many_arguments,
    clippy::needless_pass_by_value,
    clippy::should_implement_trait,
    clippy::map_unwrap_or,
    clippy::derivable_impls
)]

use serde::{Deserialize, Serialize};

pub mod lifecycle;
pub mod registry;
pub mod susceptibility;

pub use lifecycle::{
    ActorDisease, ActorDiseases, DiseaseDiedEvent, DiseaseDiagnosedEvent, DiseaseExposedEvent,
    DiseaseQuarantineEnteredEvent, DiseaseRecoveredEvent, DiseaseRelapsedEvent, DiseaseStage,
    DiseaseStageChangedEvent, DiseaseVaccinatedEvent, LifecycleOutput, RelapseReason,
    TreatmentProgress,
};
pub use registry::{
    CureRecipe, DiseaseLoadError, DiseaseRegistry, DiseaseSpec, PartialConsequence,
    TransmissionVectorEntry, VaccineProcurement, VaccineSpec,
};
pub use susceptibility::{SusceptibilityLoadError, SusceptibilityMatrix};

/// `item_required` on a [`CureRecipe`] is a content id string — the same
/// string space as `cf_equipment::ItemId` — kept local so cf-disease stays
/// a dependency-free leaf crate.
pub type ItemId = String;

/// Item id for the Medical Scanner T1 device (M14H consumer).
pub const MEDICAL_SCANNER_T1_ID: &str = "medical_scanner_t1";

/// Default in-game seconds in one in-game year (used for vaccine duration).
pub const IN_GAME_YEAR_SECONDS: f32 = 31_536_000.0;

/// Default pandemic detection threshold: infected/total fraction.
pub const PANDEMIC_INFECTED_FRACTION_THRESHOLD: f32 = 0.10;

/// Default contiguous window (seconds) the fraction must hold to declare a
/// pandemic — 24 in-game hours.
pub const PANDEMIC_WINDOW_SECONDS: f32 = 86_400.0;

/// Antibiotic course completion threshold — fraction of doses required for
/// a full cure (below this, the course is "partial").
pub const ANTIBIOTIC_COURSE_COMPLETION_THRESHOLD: f32 = 0.80;

/// Default chance a partial antibiotic course drives a resistant strain.
pub const PARTIAL_COURSE_RESISTANCE_DRIVE_CHANCE: f32 = 0.25;

/// Sepsis trigger: a wound with `dirt_pct` above this + age above the age
/// threshold escalates to sepsis.
pub const SEPSIS_DIRT_PCT_THRESHOLD: f32 = 0.6;
/// Sepsis trigger age (seconds): 24 in-game hours untreated.
pub const SEPSIS_AGE_SECONDS_THRESHOLD: f32 = 86_400.0;

/// 17 launch diseases. Variant order is the stable serialization order;
/// new diseases append at the end.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum DiseaseKind {
    Slimelung = 0,
    FoodPoisoning = 1,
    RadiationSickness = 2,
    CommonCold = 3,
    Flu = 4,
    Pneumonia = 5,
    Tuberculosis = 6,
    Cholera = 7,
    Typhoid = 8,
    Rabies = 9,
    Tetanus = 10,
    BubonicPlague = 11,
    Anthrax = 12,
    Cancer = 13,
    MentalIllness = 14,
    Sepsis = 15,
    InfluenzaPandemic = 16,
}

impl DiseaseKind {
    pub fn as_str(self) -> &'static str {
        match self {
            DiseaseKind::Slimelung => "slimelung",
            DiseaseKind::FoodPoisoning => "food_poisoning",
            DiseaseKind::RadiationSickness => "radiation_sickness",
            DiseaseKind::CommonCold => "common_cold",
            DiseaseKind::Flu => "flu",
            DiseaseKind::Pneumonia => "pneumonia",
            DiseaseKind::Tuberculosis => "tuberculosis",
            DiseaseKind::Cholera => "cholera",
            DiseaseKind::Typhoid => "typhoid",
            DiseaseKind::Rabies => "rabies",
            DiseaseKind::Tetanus => "tetanus",
            DiseaseKind::BubonicPlague => "bubonic_plague",
            DiseaseKind::Anthrax => "anthrax",
            DiseaseKind::Cancer => "cancer",
            DiseaseKind::MentalIllness => "mental_illness",
            DiseaseKind::Sepsis => "sepsis",
            DiseaseKind::InfluenzaPandemic => "influenza_pandemic",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        Some(match s {
            "slimelung" => DiseaseKind::Slimelung,
            "food_poisoning" => DiseaseKind::FoodPoisoning,
            "radiation_sickness" => DiseaseKind::RadiationSickness,
            "common_cold" => DiseaseKind::CommonCold,
            "flu" => DiseaseKind::Flu,
            "pneumonia" => DiseaseKind::Pneumonia,
            "tuberculosis" => DiseaseKind::Tuberculosis,
            "cholera" => DiseaseKind::Cholera,
            "typhoid" => DiseaseKind::Typhoid,
            "rabies" => DiseaseKind::Rabies,
            "tetanus" => DiseaseKind::Tetanus,
            "bubonic_plague" => DiseaseKind::BubonicPlague,
            "anthrax" => DiseaseKind::Anthrax,
            "cancer" => DiseaseKind::Cancer,
            "mental_illness" => DiseaseKind::MentalIllness,
            "sepsis" => DiseaseKind::Sepsis,
            "influenza_pandemic" => DiseaseKind::InfluenzaPandemic,
            _ => return None,
        })
    }

    pub fn all() -> &'static [DiseaseKind] {
        &[
            DiseaseKind::Slimelung,
            DiseaseKind::FoodPoisoning,
            DiseaseKind::RadiationSickness,
            DiseaseKind::CommonCold,
            DiseaseKind::Flu,
            DiseaseKind::Pneumonia,
            DiseaseKind::Tuberculosis,
            DiseaseKind::Cholera,
            DiseaseKind::Typhoid,
            DiseaseKind::Rabies,
            DiseaseKind::Tetanus,
            DiseaseKind::BubonicPlague,
            DiseaseKind::Anthrax,
            DiseaseKind::Cancer,
            DiseaseKind::MentalIllness,
            DiseaseKind::Sepsis,
            DiseaseKind::InfluenzaPandemic,
        ]
    }

    /// Pathogen class drives the per-origin class rules (e.g. photosynthetic
    /// 0.3× to bacterial, 1.5× to fungal).
    pub fn pathogen_class(self) -> PathogenClass {
        match self {
            DiseaseKind::Slimelung => PathogenClass::Fungal,
            DiseaseKind::CommonCold
            | DiseaseKind::Flu
            | DiseaseKind::Rabies
            | DiseaseKind::InfluenzaPandemic => PathogenClass::Viral,
            DiseaseKind::FoodPoisoning
            | DiseaseKind::Pneumonia
            | DiseaseKind::Tuberculosis
            | DiseaseKind::Cholera
            | DiseaseKind::Typhoid
            | DiseaseKind::Tetanus
            | DiseaseKind::BubonicPlague
            | DiseaseKind::Anthrax => PathogenClass::Bacterial,
            DiseaseKind::RadiationSickness => PathogenClass::Radiological,
            DiseaseKind::Cancer => PathogenClass::Neoplastic,
            DiseaseKind::MentalIllness => PathogenClass::Psychological,
            DiseaseKind::Sepsis => PathogenClass::WoundInfection,
        }
    }
}

/// Pathogen taxonomy used by the susceptibility-matrix class rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PathogenClass {
    Viral,
    Bacterial,
    Fungal,
    Parasitic,
    Radiological,
    Neoplastic,
    Psychological,
    WoundInfection,
}

impl PathogenClass {
    pub fn as_str(self) -> &'static str {
        match self {
            PathogenClass::Viral => "viral",
            PathogenClass::Bacterial => "bacterial",
            PathogenClass::Fungal => "fungal",
            PathogenClass::Parasitic => "parasitic",
            PathogenClass::Radiological => "radiological",
            PathogenClass::Neoplastic => "neoplastic",
            PathogenClass::Psychological => "psychological",
            PathogenClass::WoundInfection => "wound_infection",
        }
    }

    /// "Biological" classes that robots / drones are immune to.
    pub fn is_biological(self) -> bool {
        matches!(
            self,
            PathogenClass::Viral
                | PathogenClass::Bacterial
                | PathogenClass::Fungal
                | PathogenClass::Parasitic
                | PathogenClass::Neoplastic
                | PathogenClass::WoundInfection
        )
    }
}

/// Transmission vector for one exposure event. `as_str` mirrors the
/// `vector` field in `cf-replay/schemas/event/disease_exposed.json`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransmissionVector {
    Airborne,
    Foodborne,
    Waterborne,
    CloseContact,
    VectorBorne,
    WoundContact,
    WoundInfection,
    SporeExposure,
    RadiationDose,
    ToxinAccumulation,
    StressAccumulator,
}

impl TransmissionVector {
    pub fn as_str(self) -> &'static str {
        match self {
            TransmissionVector::Airborne => "airborne",
            TransmissionVector::Foodborne => "foodborne",
            TransmissionVector::Waterborne => "waterborne",
            TransmissionVector::CloseContact => "close_contact",
            TransmissionVector::VectorBorne => "vector_borne",
            TransmissionVector::WoundContact => "wound_contact",
            TransmissionVector::WoundInfection => "wound_infection",
            TransmissionVector::SporeExposure => "spore_exposure",
            TransmissionVector::RadiationDose => "radiation_dose",
            TransmissionVector::ToxinAccumulation => "toxin_accumulation",
            TransmissionVector::StressAccumulator => "stress_accumulator",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        Some(match s {
            "airborne" => TransmissionVector::Airborne,
            "foodborne" => TransmissionVector::Foodborne,
            "waterborne" => TransmissionVector::Waterborne,
            "close_contact" => TransmissionVector::CloseContact,
            "vector_borne" => TransmissionVector::VectorBorne,
            "wound_contact" => TransmissionVector::WoundContact,
            "wound_infection" => TransmissionVector::WoundInfection,
            "spore_exposure" => TransmissionVector::SporeExposure,
            "radiation_dose" => TransmissionVector::RadiationDose,
            "toxin_accumulation" => TransmissionVector::ToxinAccumulation,
            "stress_accumulator" => TransmissionVector::StressAccumulator,
            _ => return None,
        })
    }

    /// True when the vector spreads person-to-person (counts toward R0).
    pub fn is_contagious(self) -> bool {
        matches!(
            self,
            TransmissionVector::Airborne
                | TransmissionVector::CloseContact
                | TransmissionVector::Waterborne
        )
    }
}

/// Isolation room-class requirement. `as_str` is the `room_class` field on
/// `disease.quarantine_entered` (`"A"` / `"B"` / `"C"` / `"n/a"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IsolationClass {
    /// No human-to-human transmission — quarantine not required.
    NotApplicable,
    /// Class C — foodborne / contact, low R0: isolation cot + dedicated mealware.
    ClassC,
    /// Class B — bodily fluid, moderate R0: medical bay + surface sterilization.
    ClassB,
    /// Class A — airborne, high R0: sealed room + analyzer + filter + airlock.
    ClassA,
}

impl IsolationClass {
    pub fn as_str(self) -> &'static str {
        match self {
            IsolationClass::NotApplicable => "n/a",
            IsolationClass::ClassC => "C",
            IsolationClass::ClassB => "B",
            IsolationClass::ClassA => "A",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        Some(match s {
            "n/a" | "none" | "not_applicable" => IsolationClass::NotApplicable,
            "C" | "c" | "class_c" => IsolationClass::ClassC,
            "B" | "b" | "class_b" => IsolationClass::ClassB,
            "A" | "a" | "class_a" => IsolationClass::ClassA,
            _ => return None,
        })
    }

    /// True when a room graded `room_class` satisfies the isolation
    /// requirement of `self` (higher class subsumes lower).
    pub fn satisfied_by(self, room_class: IsolationClass) -> bool {
        room_class >= self
    }

    /// Class A (airborne, high R0) auto-quarantines on M19E atmospheric
    /// detection; lower classes require a manual/triage quarantine order.
    pub fn auto_quarantine_on_detection(self) -> bool {
        self == IsolationClass::ClassA
    }
}

/// Treatment modality for a [`CureRecipe`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TreatmentKind {
    None,
    BedRest,
    Rehydration,
    Antibiotic,
    Antiviral,
    Chelation,
    Antitoxin,
    Immunoglobulin,
    Inhaler,
    Chemotherapy,
    PostExposureVaccine,
    Therapy,
    SupportiveCare,
}

impl TreatmentKind {
    pub fn as_str(self) -> &'static str {
        match self {
            TreatmentKind::None => "none",
            TreatmentKind::BedRest => "bed_rest",
            TreatmentKind::Rehydration => "rehydration",
            TreatmentKind::Antibiotic => "antibiotic",
            TreatmentKind::Antiviral => "antiviral",
            TreatmentKind::Chelation => "chelation",
            TreatmentKind::Antitoxin => "antitoxin",
            TreatmentKind::Immunoglobulin => "immunoglobulin",
            TreatmentKind::Inhaler => "inhaler",
            TreatmentKind::Chemotherapy => "chemotherapy",
            TreatmentKind::PostExposureVaccine => "post_exposure_vaccine",
            TreatmentKind::Therapy => "therapy",
            TreatmentKind::SupportiveCare => "supportive_care",
        }
    }
}

/// 10 launch origins (races). Mirrors the disease-relevant origin naming
/// used by `content/balance/afflictions_registry_full.json`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum OriginId {
    Human = 0,
    Android = 1,
    Robot = 2,
    Drone = 3,
    HeavyBiomech = 4,
    MethaneBreather = 5,
    Crystalline = 6,
    Aqueous = 7,
    Photosynthetic = 8,
    Insectoid = 9,
}

impl OriginId {
    pub fn as_str(self) -> &'static str {
        match self {
            OriginId::Human => "human",
            OriginId::Android => "android",
            OriginId::Robot => "robot",
            OriginId::Drone => "drone",
            OriginId::HeavyBiomech => "heavy_biomech",
            OriginId::MethaneBreather => "methane_breather",
            OriginId::Crystalline => "crystalline",
            OriginId::Aqueous => "aqueous",
            OriginId::Photosynthetic => "photosynthetic",
            OriginId::Insectoid => "insectoid",
        }
    }

    /// Tolerant parse — maps FRE/world aliases onto the canonical 10.
    pub fn from_str(s: &str) -> Self {
        match s {
            "android" | "android_synthetic" => OriginId::Android,
            "robot" | "robotic_drone" | "synth" => OriginId::Robot,
            "drone" => OriginId::Drone,
            "heavy_biomech" | "powered_organic" => OriginId::HeavyBiomech,
            "methane" | "methane_breather" => OriginId::MethaneBreather,
            "crystalline" | "crystalline_helios" | "silica_xenofauna" | "silicon" => {
                OriginId::Crystalline
            }
            "aqueous" | "aqueous_kindred" => OriginId::Aqueous,
            "photosynthetic" | "photosynth" => OriginId::Photosynthetic,
            "insectoid" | "insectoid_swarm" => OriginId::Insectoid,
            _ => OriginId::Human,
        }
    }

    pub fn all() -> &'static [OriginId] {
        &[
            OriginId::Human,
            OriginId::Android,
            OriginId::Robot,
            OriginId::Drone,
            OriginId::HeavyBiomech,
            OriginId::MethaneBreather,
            OriginId::Crystalline,
            OriginId::Aqueous,
            OriginId::Photosynthetic,
            OriginId::Insectoid,
        ]
    }

    /// True for fully synthetic origins (immune to every biological disease).
    pub fn is_synthetic(self) -> bool {
        matches!(self, OriginId::Robot | OriginId::Drone)
    }
}

impl Default for OriginId {
    fn default() -> Self {
        OriginId::Human
    }
}

/// Deterministic [0,1) roll keyed by (seed, actor, disease, salt). Uses a
/// SplitMix64 finaliser so identical inputs reproduce identical outcomes —
/// no `thread_rng`.
pub fn deterministic_roll(seed: u64, actor_id: u64, kind: DiseaseKind, salt: u64) -> f32 {
    let mut z = seed
        ^ actor_id.wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (kind as u64).wrapping_mul(0xD1B5_4A32_D192_ED03)
        ^ salt.wrapping_mul(0x94D0_49BB_1331_11EB);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 11) as f32 / ((1u64 << 53) as f32)
}

/// Deterministic sliding-window pandemic monitor. The engine feeds the
/// infected/total ratio each tick; the monitor declares a pandemic once the
/// ratio has held above `threshold_fraction` for a contiguous
/// `window_seconds`. Tick-counter based — never wall-clock — so replays are
/// bit-identical.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PandemicMonitor {
    pub threshold_fraction: f32,
    pub window_seconds: f32,
    /// Tick at which the ratio first crossed the threshold in the current
    /// contiguous run. `None` when below threshold.
    pub above_since_tick: Option<u64>,
    /// True once a pandemic has been declared (latched — declared once).
    pub declared: bool,
}

impl Default for PandemicMonitor {
    fn default() -> Self {
        Self {
            threshold_fraction: PANDEMIC_INFECTED_FRACTION_THRESHOLD,
            window_seconds: PANDEMIC_WINDOW_SECONDS,
            above_since_tick: None,
            declared: false,
        }
    }
}

impl PandemicMonitor {
    pub fn new(threshold_fraction: f32, window_seconds: f32) -> Self {
        Self {
            threshold_fraction,
            window_seconds,
            above_since_tick: None,
            declared: false,
        }
    }

    /// Feed one tick. Returns `true` exactly once — on the tick the pandemic
    /// is first declared.
    pub fn observe(
        &mut self,
        infected: u32,
        total: u32,
        tick: u64,
        tick_rate_hz: u32,
    ) -> bool {
        if self.declared || total == 0 {
            if total == 0 {
                self.above_since_tick = None;
            }
            return false;
        }
        let fraction = infected as f32 / total as f32;
        if fraction > self.threshold_fraction {
            let start = *self.above_since_tick.get_or_insert(tick);
            let elapsed_ticks = tick.saturating_sub(start);
            let window_ticks = (self.window_seconds * tick_rate_hz.max(1) as f32) as u64;
            if elapsed_ticks >= window_ticks {
                self.declared = true;
                return true;
            }
        } else {
            self.above_since_tick = None;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disease_kind_round_trips() {
        for &k in DiseaseKind::all() {
            assert_eq!(DiseaseKind::from_str(k.as_str()), Some(k));
        }
        assert_eq!(DiseaseKind::all().len(), 17);
    }

    #[test]
    fn ten_origins() {
        assert_eq!(OriginId::all().len(), 10);
        assert_eq!(OriginId::from_str("methane_breather"), OriginId::MethaneBreather);
        assert_eq!(OriginId::from_str("silica_xenofauna"), OriginId::Crystalline);
        assert_eq!(OriginId::from_str("unknown"), OriginId::Human);
    }

    #[test]
    fn isolation_class_ordering() {
        assert!(IsolationClass::ClassA > IsolationClass::ClassB);
        assert!(IsolationClass::ClassB > IsolationClass::ClassC);
        assert!(IsolationClass::ClassC.satisfied_by(IsolationClass::ClassA));
        assert!(!IsolationClass::ClassA.satisfied_by(IsolationClass::ClassC));
        assert_eq!(IsolationClass::ClassA.as_str(), "A");
    }

    #[test]
    fn deterministic_roll_is_stable_and_bounded() {
        let a = deterministic_roll(42, 7, DiseaseKind::Pneumonia, 1);
        let b = deterministic_roll(42, 7, DiseaseKind::Pneumonia, 1);
        assert_eq!(a, b);
        assert!((0.0..1.0).contains(&a));
        let c = deterministic_roll(43, 7, DiseaseKind::Pneumonia, 1);
        assert_ne!(a, c);
    }

    #[test]
    fn pandemic_declares_after_window() {
        let mut mon = PandemicMonitor::new(0.10, 86_400.0);
        // 60 Hz; window = 86400 * 60 ticks. Feed > 10% from tick 0.
        let window_ticks = (86_400.0 * 60.0) as u64;
        let mut declared_at = None;
        for tick in 0..=window_ticks + 5 {
            if mon.observe(20, 100, tick, 60) {
                declared_at = Some(tick);
                break;
            }
        }
        assert_eq!(declared_at, Some(window_ticks));
    }

    #[test]
    fn pandemic_resets_when_fraction_drops() {
        let mut mon = PandemicMonitor::new(0.10, 100.0);
        let window_ticks = (100.0 * 60.0) as u64;
        // Hold above for half the window, then drop, then never reaches.
        for tick in 0..window_ticks / 2 {
            assert!(!mon.observe(20, 100, tick, 60));
        }
        // Drop below threshold resets the run.
        assert!(!mon.observe(5, 100, window_ticks / 2, 60));
        assert!(mon.above_since_tick.is_none());
        // Single tick above will not immediately declare.
        assert!(!mon.observe(20, 100, window_ticks / 2 + 1, 60));
    }
}
