//! M16C — psych-medication equipment items. The 8 medication families + dose
//! schedules live in `cf-mental-health` (the canonical registry); this module
//! is the equipment-layer view: it re-exports the catalog and adds the
//! per-dose side-effect roll (addiction onset for the habit-forming classes).
//!
//! Mirrors `cf-equipment::cures` (which wraps the cf-disease cure registry).

use cf_mental_health::{PsychMedClass, PsychMedItemSpec};

pub use cf_mental_health::{
    default_psych_med_catalog, load_psych_med_dir, psych_med_for, PsychMedItemSpec as PsychMed,
    PsychMedClass as PsychMedFamily, PsychMedLoadError,
};

/// A side effect rolled when a psych-med dose is administered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PsychMedSideEffect {
    /// No adverse effect this dose.
    None,
    /// The dose pushed the carrier into chemical dependency.
    AddictionOnset,
}

/// Deterministic [0,1) roll for a med dose, keyed by (seed, actor, med class,
/// dose index). Local SplitMix64 finaliser — same kernel shape as
/// `cf_mental_health::mh_roll`, keyed by the med class rather than a condition.
fn dose_roll(seed: u64, actor_id: u64, class: PsychMedClass, dose_index: u32) -> f32 {
    let mut z = seed
        ^ actor_id.wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (class as u64).wrapping_mul(0xD1B5_4A32_D192_ED03)
        ^ u64::from(dose_index).wrapping_mul(0x94D0_49BB_1331_11EB);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 11) as f32 / ((1u64 << 53) as f32)
}

/// Roll the side effect for one dose of `item`. For habit-forming classes
/// (benzo / opioid / stimulant) an `addiction_risk_per_dose` chance returns
/// [`PsychMedSideEffect::AddictionOnset`]; otherwise [`PsychMedSideEffect::None`].
/// Deterministic — the same (seed, actor, dose) reproduces the outcome.
pub fn dose_side_effect(
    seed: u64,
    actor_id: u64,
    item: &PsychMedItemSpec,
    dose_index: u32,
) -> PsychMedSideEffect {
    if item.addiction_risk_per_dose > 0.0
        && dose_roll(seed, actor_id, item.class, dose_index) < item.addiction_risk_per_dose
    {
        PsychMedSideEffect::AddictionOnset
    } else {
        PsychMedSideEffect::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eight_med_families_present() {
        assert_eq!(default_psych_med_catalog().len(), 8);
    }

    #[test]
    fn ssri_dose_never_causes_addiction() {
        let cat = default_psych_med_catalog();
        let ssri = psych_med_for(&cat, cf_mental_health::ConditionKind::Depression).unwrap();
        for dose in 0..100 {
            assert_eq!(dose_side_effect(42, 7, ssri, dose), PsychMedSideEffect::None);
        }
    }

    #[test]
    fn benzo_dose_addiction_is_deterministic_and_possible() {
        let cat = default_psych_med_catalog();
        let benzo = cat.iter().find(|m| m.class == PsychMedClass::Benzo).unwrap();
        assert!(benzo.addiction_risk_per_dose > 0.0);
        // Determinism: identical inputs reproduce the outcome.
        let a = dose_side_effect(99, 3, benzo, 5);
        let b = dose_side_effect(99, 3, benzo, 5);
        assert_eq!(a, b);
        // Across many doses at least one onset occurs (4% risk).
        let any = (0..500).any(|d| dose_side_effect(99, 3, benzo, d) == PsychMedSideEffect::AddictionOnset);
        assert!(any, "a 4%/dose risk must fire within 500 doses");
    }
}
