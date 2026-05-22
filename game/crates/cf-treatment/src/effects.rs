//! **M14H** § Treatment effect resolver.
//!
//! Each of the 22 [`crate::TreatmentKind`] producers maps to a
//! [`TreatmentEffect`] describing what the engine should mutate when the
//! apply-state-machine reaches `Completed`:
//!
//! - Wound flag flips (`bandaged`, `sutured`).
//! - New wound emissions (cauterize → Burn1st, defib shock → Burn1st on
//!   chest, CPR after 3 rounds → BruiseLight on chest).
//! - Visible-state transitions (suture → SutureLine, scab → Scar).
//! - Active-buff insertion (combat_stim, painkiller, anti-anxiety).
//! - Per-zone tourniquet timer start (necrosis after 90 min).
//! - Antibiotic-course tracker start.
//! - Cardiac rhythm restoration (defib success roll).

use serde::{Deserialize, Serialize};

use crate::producers::TreatmentKind;
use cf_wound::WoundKind;

///
/// The engine pattern-matches on this when applying the completed
/// treatment to the patient's actor state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "effect")]
pub enum TreatmentEffect {
    /// Bandage a bleeding wound on `zone`. If `zone` is empty, bandage
    /// the wound with the highest bleed rate.
    BandageBleed { zone: Option<String> },
    /// Trauma-pack severe bleed; bandages + halves bleed rate.
    TraumaPack { zone: Option<String> },
    /// Apply a tourniquet to a limb. Engine records the apply tick on
    /// `ActorState.m14h_tourniquets`; the per-tick aging pass converts
    /// the zone to Necrotic if it stays > 90 min.
    Tourniquet { zone: String },
    /// Suture a wound. Engine sets `sutured = true` + transitions to
    /// `SutureLine` visible state. Fails roll on dirty wound.
    Suture { zone: Option<String> },
    /// Cauterize a wound. Engine closes bleed + emits a Burn1st wound on
    /// the same zone (per spec).
    Cauterize { zone: String },
    /// Splint a fracture. Engine flags the wound's heal-time multiplier.
    Splint { zone: String },
    /// Surgery — per-shrapnel removal during Operate phase. Engine emits
    /// `treatment.applied { kind: SurgeryKitV1 }` per shrapnel removed +
    /// reduces ShrapnelEmbedded severity by 1 band.
    SurgeryRemoveShrapnel,
    /// Defibrillator shock — engine consumes one charge, rolls success
    /// (50% + 10% × consecutive_cpr_rounds), emits a Burn1st wound at
    /// the chest zone. 8s recharge interval is enforced.
    DefibShock,
    /// CPR round — engine increments consecutive_cpr_rounds. After 3+
    /// rounds, emits a BruiseLight wound at the chest zone.
    CprRound,
    /// Transfusion bag — engine restores 500ml blood; per-origin
    /// compatibility roll.
    TransfusionBag,
    /// IV fluids — engine restores hydration; applies the IvFluidsV1 buff.
    IvFluids,
    /// Oxygen therapy — applies the OxygenTherapyV1 buff.
    OxygenTherapy,
    /// Antibiotic course tier 1 (14 doses × 8h).
    AntibioticCourseT1,
    /// Antibiotic course tier 2 (21 doses × 6h).
    AntibioticCourseT2,
    /// Universal antidote — clears mild poison; rejects severe poison.
    AntidoteUniversal,
    /// Organophosphate antidote — clears nerve-agent toxin; injects
    /// cardiac side effect risk.
    AntidoteOrganophosphate,
    /// Anti-radiation chelation — applies the AntiRadiationChelation buff.
    AntiRadiationChelation,
    /// Painkiller (opioid t1) — applies the PainkillerOpioidT1 buff
    /// (Pain reduction by 30 points for 4h).
    PainkillerOpioidT1,
    /// Anti-anxiety benzo t1 — applies AntiAnxietyBenzoT1 buff for 6h.
    AntiAnxietyBenzoT1,
    /// Combat stim t1 — applies CombatStimT1 buff (+20% accuracy + 20%
    /// move speed) for 90s. Crash buff follows.
    CombatStimT1,
    /// Medical scanner — engine emits a `scan.completed` event with the
    /// full diagnostic snapshot.
    MedicalScannerT1,
    /// Hospital bed — applies the HospitalBedV1 (per-tick +50% heal)
    /// buff while the actor stays bedded.
    HospitalBedV1,
}

impl TreatmentEffect {
    pub fn chest_zone() -> &'static str {
        "torso_front"
    }

    /// Wound kind emitted by [`TreatmentEffect::Cauterize`].
    pub fn cauterize_burn_kind() -> WoundKind {
        WoundKind::Burn1st
    }

    /// Wound kind emitted by each defib shock (BurnAtChestPerShock).
    pub fn defib_chest_burn_kind() -> WoundKind {
        WoundKind::Burn1st
    }

    /// Wound kind emitted after 3+ CPR rounds (BruisePerExtendedCpr).
    pub fn cpr_chest_bruise_kind() -> WoundKind {
        WoundKind::BruiseLight
    }
}

///
/// `zone_hint` provides the engine with the wound zone when known (e.g.
/// cauterize requires an explicit zone; bandage applies to the most-bleeding
/// wound when omitted).
#[must_use]
pub fn effect_for(kind: TreatmentKind, zone_hint: Option<String>) -> TreatmentEffect {
    use TreatmentKind::*;
    match kind {
        FieldBandageV1 => TreatmentEffect::BandageBleed { zone: zone_hint },
        TraumaPackV1 => TreatmentEffect::TraumaPack { zone: zone_hint },
        TourniquetV1 => TreatmentEffect::Tourniquet {
            zone: zone_hint.unwrap_or_else(|| "leg_left".to_string()),
        },
        SuturesV1 => TreatmentEffect::Suture { zone: zone_hint },
        CauterizeV1 => TreatmentEffect::Cauterize {
            zone: zone_hint.unwrap_or_else(|| "torso_front".to_string()),
        },
        SplintV1 => TreatmentEffect::Splint {
            zone: zone_hint.unwrap_or_else(|| "leg_left".to_string()),
        },
        SurgeryKitV1 => TreatmentEffect::SurgeryRemoveShrapnel,
        DefibrillatorV1 => TreatmentEffect::DefibShock,
        CprManual => TreatmentEffect::CprRound,
        TransfusionBagV1 => TreatmentEffect::TransfusionBag,
        IvFluidsV1 => TreatmentEffect::IvFluids,
        OxygenTherapyV1 => TreatmentEffect::OxygenTherapy,
        AntibioticCourseT1 => TreatmentEffect::AntibioticCourseT1,
        AntibioticCourseT2 => TreatmentEffect::AntibioticCourseT2,
        AntidoteUniversalT1 => TreatmentEffect::AntidoteUniversal,
        AntidoteOrganophosphate => TreatmentEffect::AntidoteOrganophosphate,
        AntiRadiationChelation => TreatmentEffect::AntiRadiationChelation,
        PainkillerOpioidT1 => TreatmentEffect::PainkillerOpioidT1,
        AntiAnxietyBenzoT1 => TreatmentEffect::AntiAnxietyBenzoT1,
        CombatStimT1 => TreatmentEffect::CombatStimT1,
        MedicalScannerT1 => TreatmentEffect::MedicalScannerT1,
        HospitalBedV1 => TreatmentEffect::HospitalBedV1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_has_an_effect() {
        for kind in TreatmentKind::ALL.iter() {
            let e = effect_for(*kind, None);
            // Sanity-check: no panics on the resolver path.
            let _ = format!("{e:?}");
        }
    }

    #[test]
    fn cauterize_zone_default() {
        let e = effect_for(TreatmentKind::CauterizeV1, None);
        assert!(matches!(e, TreatmentEffect::Cauterize { .. }));
    }

    #[test]
    fn cauterize_zone_override() {
        let e = effect_for(
            TreatmentKind::CauterizeV1,
            Some("arm_right".to_string()),
        );
        match e {
            TreatmentEffect::Cauterize { zone } => assert_eq!(zone, "arm_right"),
            _ => panic!("expected Cauterize"),
        }
    }

    #[test]
    fn surgery_emits_shrapnel_removal_effect() {
        let e = effect_for(TreatmentKind::SurgeryKitV1, None);
        assert_eq!(e, TreatmentEffect::SurgeryRemoveShrapnel);
    }
}
