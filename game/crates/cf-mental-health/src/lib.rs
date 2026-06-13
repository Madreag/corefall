//! M16C — canonical mental-health system: the 8-condition lifecycle FSM, the
//! per-condition trigger evaluators (witness-death window, stim-dose window),
//! the treatment-efficacy roller (therapy + medication), the psych-medication
//! registry, and the comorbidity matrix.
//!
//! This crate owns the data + deterministic kernels; consumers wire them:
//!   - `cf-actor::traits` registers the recovery / chronic / refractory traits.
//!   - `cf-equipment::{psych_meds,stims}` ship the medication + stim items.
//!   - `cf-storyteller::trauma_event` registers witness-death trauma beats.
//!   - `cf-ai::medic_doctrine` drives the medic `TreatPsych` priority.
//!   - `cf-ui::psych_dashboard` renders per-actor mental-health state.
//!
//! Lifecycle FSM: `Triggered → Acute → (Subacute | Chronic) → (Remission |
//! Refractory)` — isomorphic to the cf-disease `DiseaseStage` lifecycle and
//! shared with the M16B `mental_illness` disease entry.
//!
//! Determinism: no `thread_rng`. Stochastic outcomes (treated/natural
//! resolution, panic-attack timing, relapse, therapy efficacy) derive from a
//! seeded `mh_roll` keyed by (seed, actor_id, condition, salt) so identical
//! inputs reproduce identical event streams. Per-tick rolls fold the tick into
//! the salt so a fixed seed reproduces panic-attack *timing* (acceptance
//! scenario 9).

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

pub mod comorbidity;
pub mod conditions;
pub mod treatment;

pub use comorbidity::{ComorbidityLoadError, ComorbidityMatrix, ComorbidityPair};
pub use conditions::{
    ActorCondition, ActorMentalHealth, AddictionDevelopedEvent, ComorbidityDetectedEvent,
    ConditionKind, ConditionLoadError, ConditionRegistry, ConditionSpec, ConditionStage,
    ConditionTriggeredEvent, DoseWindow, MedicationStartedEvent, PanicAttackEvent, PsychOutput,
    PsychRelapsedEvent, PsychStageChangedEvent, RemissionAchievedEvent, TherapySessionEvent,
    TriggerReason, WithdrawalStartedEvent, WitnessWindow,
};
pub use treatment::{
    default_psych_med_catalog, load_psych_med_dir, psych_med_for, therapy_efficacy_roll,
    PsychMedClass, PsychMedItemSpec, PsychMedLoadError, TreatmentPlan,
};

// ---------------------------------------------------------------------------
// Trait-id conventions (M14I / M41 consumers). The per-condition trait strings
// are built from these prefixes by `ConditionKind::{recovered,chronic,
// refractory}_trait`.
// ---------------------------------------------------------------------------

/// Prefix for the recovery trait granted on remission (`recovered_from_ptsd`).
pub const RECOVERED_FROM_PREFIX: &str = "recovered_from_";
/// Prefix for the chronic trait granted on chronic entry (`chronic_depression`).
pub const CHRONIC_PREFIX: &str = "chronic_";
/// Prefix for the refractory trait granted on refractory entry
/// (`refractory_ptsd`).
pub const REFRACTORY_PREFIX: &str = "refractory_";

// ---------------------------------------------------------------------------
// Time constants (in-game seconds). The mental-health clocks run in in-game
// time; the engine converts ticks → seconds via `ticks_to_seconds`.
// ---------------------------------------------------------------------------

/// In-game seconds in one in-game hour.
pub const HOUR_SECONDS: f32 = 3_600.0;
/// In-game seconds in one in-game day.
pub const DAY_SECONDS: f32 = 86_400.0;

// ---------------------------------------------------------------------------
// Trigger thresholds (spec § Tunable defaults).
// ---------------------------------------------------------------------------

/// PTSD witness trigger: deaths within the witness window.
pub const WITNESS_DEATH_THRESHOLD: u32 = 3;
/// PTSD witness window (seconds): 3+ deaths within 60s.
pub const WITNESS_WINDOW_SECONDS: f32 = 60.0;

/// Addiction trigger: combat-stim doses within the addiction window.
pub const ADDICTION_DOSE_THRESHOLD: u32 = 7;
/// Addiction window (seconds): 7 doses within 30 in-game days.
pub const ADDICTION_WINDOW_SECONDS: f32 = 30.0 * DAY_SECONDS;

/// Withdrawal trigger: an addicted actor whose last dose is older than this
/// (seconds) begins withdrawal — "drug absent > N hours".
pub const WITHDRAWAL_ABSENCE_HOURS: f32 = 12.0;
/// Withdrawal absence threshold in seconds.
pub const WITHDRAWAL_ABSENCE_SECONDS: f32 = WITHDRAWAL_ABSENCE_HOURS * HOUR_SECONDS;
/// Withdrawal natural-resolution clock (seconds): resolves over 2 in-game
/// weeks / 14 in-game days (spec § Tunable defaults — "aim-shake duration").
pub const WITHDRAWAL_RESOLVE_SECONDS: f32 = 14.0 * DAY_SECONDS;
/// Aim-wobble multiplier while actively withdrawing (2× wobble).
pub const WITHDRAWAL_AIM_WOBBLE_MULTIPLIER: f32 = 2.0;

// ---------------------------------------------------------------------------
// Treatment constants (spec § Tunable defaults + Therapy NPC).
// ---------------------------------------------------------------------------

/// Therapy session length: 30 in-game minutes.
pub const THERAPY_SESSION_MINUTES: f32 = 30.0;
/// Therapy session length in seconds.
pub const THERAPY_SESSION_SECONDS: f32 = THERAPY_SESSION_MINUTES * 60.0;
/// Therapy sessions required for a treated PTSD remission (scenario 4).
pub const PTSD_THERAPY_SESSIONS: u32 = 10;

/// SSRI therapeutic onset: 14 in-game days before it counts toward remission.
pub const SSRI_ONSET_SECONDS: f32 = 14.0 * DAY_SECONDS;
/// Benzodiazepine per-dose addiction risk (4%).
pub const BENZO_ADDICTION_RISK_PER_DOSE: f32 = 0.04;
/// Combat-stim per-dose addiction risk (7%).
pub const STIM_ADDICTION_RISK_PER_DOSE: f32 = 0.07;

// ---------------------------------------------------------------------------
// Panic-attack constants (spec § Panic Disorder: "freeze for 3–8s").
// ---------------------------------------------------------------------------

/// Minimum panic-attack freeze duration (seconds).
pub const PANIC_FREEZE_MIN_SECONDS: f32 = 3.0;
/// Maximum panic-attack freeze duration (seconds).
pub const PANIC_FREEZE_MAX_SECONDS: f32 = 8.0;

// ---------------------------------------------------------------------------
// RNG salts. Each decorrelates one stochastic stream. The high 32 bits are a
// unique discriminator per salt; per-tick streams XOR the tick (which lands in
// the low 32 bits for any realistic in-game time), so distinct streams stay
// distinct even at the same tick.
// ---------------------------------------------------------------------------

/// Salt — treated / natural resolution outcome (Chronic vs Remission band).
pub const SALT_OUTCOME: u64 = 0x0000_0001_0000_0000;
/// Salt — per-tick panic-attack roll (XORed with the tick).
pub const SALT_PANIC: u64 = 0x0000_0002_0000_0000;
/// Salt — per-tick panic freeze-duration roll (XORed with the tick).
pub const SALT_PANIC_FREEZE: u64 = 0x0000_0003_0000_0000;
/// Salt — per-tick relapse roll (XORed with the tick).
pub const SALT_RELAPSE: u64 = 0x0000_0004_0000_0000;
/// Salt — per-session therapy efficacy roll (XORed with the session index).
pub const SALT_THERAPY: u64 = 0x0000_0005_0000_0000;
/// Salt — comorbidity-onset roll.
pub const SALT_COMORBID: u64 = 0x0000_0006_0000_0000;

/// 10 launch origins (races). Mirrors `cf_disease::OriginId` — duplicated so
/// cf-mental-health stays a dependency-free leaf crate (same idiom M16A/M16B
/// use). Synthetic origins (robot / drone) have no mental health.
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

    /// True for fully synthetic origins (robots / drones have no mental
    /// health — the per-origin mental-health filter, scenario 8).
    pub fn is_synthetic(self) -> bool {
        matches!(self, OriginId::Robot | OriginId::Drone)
    }
}

impl Default for OriginId {
    fn default() -> Self {
        OriginId::Human
    }
}

/// Deterministic [0,1) roll keyed by (seed, actor, condition, salt). Uses a
/// SplitMix64 finaliser so identical inputs reproduce identical outcomes —
/// no `thread_rng`. Byte-for-byte the same structure as
/// `cf_disease::deterministic_roll`, so the two systems share a mixing kernel.
pub fn mh_roll(seed: u64, actor_id: u64, kind: ConditionKind, salt: u64) -> f32 {
    let mut z = seed
        ^ actor_id.wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (kind as u64).wrapping_mul(0xD1B5_4A32_D192_ED03)
        ^ salt.wrapping_mul(0x94D0_49BB_1331_11EB);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 11) as f32 / ((1u64 << 53) as f32)
}

/// Convert a tick count to in-game seconds at `tick_rate_hz` (min 1 Hz).
pub fn ticks_to_seconds(ticks: u64, tick_rate_hz: u32) -> f32 {
    ticks as f32 / tick_rate_hz.max(1) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ten_origins_round_trip() {
        assert_eq!(OriginId::all().len(), 10);
        for &o in OriginId::all() {
            assert_eq!(OriginId::from_str(o.as_str()), o);
        }
        assert_eq!(OriginId::from_str("silica_xenofauna"), OriginId::Crystalline);
        assert_eq!(OriginId::from_str("unknown"), OriginId::Human);
        assert_eq!(OriginId::default(), OriginId::Human);
    }

    #[test]
    fn synthetic_origins_are_robot_and_drone() {
        assert!(OriginId::Robot.is_synthetic());
        assert!(OriginId::Drone.is_synthetic());
        assert!(!OriginId::Human.is_synthetic());
        assert!(!OriginId::Android.is_synthetic());
    }

    #[test]
    fn mh_roll_is_stable_and_bounded() {
        let a = mh_roll(42, 7, ConditionKind::Ptsd, SALT_OUTCOME);
        let b = mh_roll(42, 7, ConditionKind::Ptsd, SALT_OUTCOME);
        assert_eq!(a, b);
        assert!((0.0..1.0).contains(&a));
        assert_ne!(a, mh_roll(43, 7, ConditionKind::Ptsd, SALT_OUTCOME));
        assert_ne!(a, mh_roll(42, 7, ConditionKind::Depression, SALT_OUTCOME));
    }

    #[test]
    fn per_tick_salts_decorrelate_panic_from_freeze() {
        // The panic roll and the freeze-duration roll share the tick but use
        // different salts — they must not be the same value.
        for tick in [0u64, 1, 500, 86_400, 5_000_000] {
            let panic = mh_roll(99, 3, ConditionKind::PanicDisorder, SALT_PANIC ^ tick);
            let freeze = mh_roll(99, 3, ConditionKind::PanicDisorder, SALT_PANIC_FREEZE ^ tick);
            assert_ne!(panic, freeze, "panic/freeze correlated at tick {tick}");
        }
    }

    #[test]
    fn ticks_to_seconds_uses_rate() {
        assert!((ticks_to_seconds(60, 60) - 1.0).abs() < 1e-6);
        assert!((ticks_to_seconds(120, 60) - 2.0).abs() < 1e-6);
        // Zero rate clamps to 1 Hz (never divides by zero).
        assert!((ticks_to_seconds(5, 0) - 5.0).abs() < 1e-6);
    }

    #[test]
    fn trait_prefixes_compose_condition_traits() {
        assert!(ConditionKind::Ptsd.recovered_trait().starts_with(RECOVERED_FROM_PREFIX));
        assert!(ConditionKind::Depression.chronic_trait().starts_with(CHRONIC_PREFIX));
        assert!(ConditionKind::Ptsd.refractory_trait().starts_with(REFRACTORY_PREFIX));
    }

    #[test]
    fn tunable_defaults_match_spec() {
        assert_eq!(WITNESS_DEATH_THRESHOLD, 3);
        assert!((WITNESS_WINDOW_SECONDS - 60.0).abs() < 1e-6);
        assert_eq!(ADDICTION_DOSE_THRESHOLD, 7);
        assert!((ADDICTION_WINDOW_SECONDS - 2_592_000.0).abs() < 1.0);
        assert!((SSRI_ONSET_SECONDS - 1_209_600.0).abs() < 1.0);
        assert!((BENZO_ADDICTION_RISK_PER_DOSE - 0.04).abs() < 1e-6);
        assert!((STIM_ADDICTION_RISK_PER_DOSE - 0.07).abs() < 1e-6);
        assert!((THERAPY_SESSION_MINUTES - 30.0).abs() < 1e-6);
        assert_eq!(PTSD_THERAPY_SESSIONS, 10);
    }
}
