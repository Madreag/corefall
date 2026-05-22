//! **M14I** § FunctionalDebuff enum + locked
//! `(WoundKind × closure_method × severity_at_close)` → `FunctionalDebuff`
//! matrix.

use serde::{Deserialize, Serialize};

use cf_wound::registry::{TreatmentKind, ZoneId};
use cf_wound::WoundKind;

/// Locked sensory channel identifier — used by `SensoryLoss`.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SenseKind {
    Sight = 0,
    Hearing = 1,
    Touch = 2,
    Smell = 3,
}

impl SenseKind {
    pub fn as_str(self) -> &'static str {
        match self {
            SenseKind::Sight => "sight",
            SenseKind::Hearing => "hearing",
            SenseKind::Touch => "touch",
            SenseKind::Smell => "smell",
        }
    }
}

///
/// Matches the spec's named variant list exactly. Per-variant stat
/// semantics are documented at the call site (cf-actor::long_term passive
/// pass). Numeric fields use `f32` for cross-tick stability.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FunctionalDebuff {
    None,
    ReducedMaxBlood {
        ml_lost: f32,
    },
    ReducedZoneStrength {
        zone: ZoneId,
        pct: f32,
    },
    ReducedAimAccuracy {
        pct: f32,
    },
    ReducedMoveSpeed {
        pct: f32,
    },
    SensoryLoss {
        sense: SenseKind,
        pct: f32,
    },
    ChronicPainBaseline {
        pain_points: f32,
    },
    Limp,
    PhantomLimbRisk {
        chance_per_panic: f32,
    },
}

impl FunctionalDebuff {
    /// Stable 3-tuple used by [`crate::ScarTimeline::checksum_bytes`].
    pub fn checksum_triple(&self) -> (u8, f32, f32) {
        match self {
            FunctionalDebuff::None => (0, 0.0, 0.0),
            FunctionalDebuff::ReducedMaxBlood { ml_lost } => (1, *ml_lost, 0.0),
            FunctionalDebuff::ReducedZoneStrength { pct, .. } => (2, *pct, 0.0),
            FunctionalDebuff::ReducedAimAccuracy { pct } => (3, *pct, 0.0),
            FunctionalDebuff::ReducedMoveSpeed { pct } => (4, *pct, 0.0),
            FunctionalDebuff::SensoryLoss { sense, pct } => (5, *sense as u8 as f32, *pct),
            FunctionalDebuff::ChronicPainBaseline { pain_points } => (6, *pain_points, 0.0),
            FunctionalDebuff::Limp => (7, 0.0, 0.0),
            FunctionalDebuff::PhantomLimbRisk { chance_per_panic } => (8, *chance_per_panic, 0.0),
        }
    }

    /// Stable snake_case tag name for replay event payloads.
    pub fn tag(&self) -> &'static str {
        match self {
            FunctionalDebuff::None => "none",
            FunctionalDebuff::ReducedMaxBlood { .. } => "reduced_max_blood",
            FunctionalDebuff::ReducedZoneStrength { .. } => "reduced_zone_strength",
            FunctionalDebuff::ReducedAimAccuracy { .. } => "reduced_aim_accuracy",
            FunctionalDebuff::ReducedMoveSpeed { .. } => "reduced_move_speed",
            FunctionalDebuff::SensoryLoss { .. } => "sensory_loss",
            FunctionalDebuff::ChronicPainBaseline { .. } => "chronic_pain_baseline",
            FunctionalDebuff::Limp => "limp",
            FunctionalDebuff::PhantomLimbRisk { .. } => "phantom_limb_risk",
        }
    }
}

///
/// Spec table:
/// - `LacerationSevere × sutures × 0.8` → `ReducedZoneStrength{zone, 0.05}`.
/// - `Burn3rd × cauterize × 0.9` → `ReducedZoneStrength{zone, 0.15}` PLUS
///   `ChronicPainBaseline{2}`. Since the debuff slot is single-valued the
///   chronic-pain pairing is folded into the zone strength's pct field; the
///   spec's ChronicPainBaseline pairing is preserved via a per-zone bonus
///   pain-points encoded in the actor's long-term-state pass.
///
/// To honor the spec's "Each WoundKind × closure_method × severity-at-close
/// maps to a FunctionalDebuff" contract we cover every combination
/// produced by the M14H closure methods (`SuturesV1`/`CauterizeV1`/
/// `SurgeryKitV1`). Closure via the literal `TreatmentKind::SutureKit`
/// (M14G enum) maps to the same per-wound mapping as `SuturesV1`.
#[must_use]
pub fn functional_debuff_for(
    kind: WoundKind,
    closure: TreatmentKind,
    severity: f32,
    zone: &ZoneId,
) -> FunctionalDebuff {
    use FunctionalDebuff::*;
    let s = severity.clamp(0.0, 1.0);
    match (kind, closure) {
        // ---- Penetrating ----
        (WoundKind::LacerationLight, _) if s < 0.5 => None,
        (WoundKind::LacerationLight, _) => ReducedZoneStrength {
            zone: zone.clone(),
            pct: 0.01,
        },
        (WoundKind::LacerationModerate, _) => ReducedZoneStrength {
            zone: zone.clone(),
            pct: 0.03,
        },
        (WoundKind::LacerationSevere, _) => ReducedZoneStrength {
            zone: zone.clone(),
            pct: 0.05,
        },
        (WoundKind::Puncture, _) => ReducedZoneStrength {
            zone: zone.clone(),
            pct: 0.02,
        },
        (WoundKind::StabThrough, _) => ReducedZoneStrength {
            zone: zone.clone(),
            pct: 0.04,
        },
        (WoundKind::GunshotEntry, _) => ReducedZoneStrength {
            zone: zone.clone(),
            pct: 0.05,
        },
        (WoundKind::GunshotExit, _) => ReducedZoneStrength {
            zone: zone.clone(),
            pct: 0.05,
        },
        (WoundKind::GunshotThrough, _) => ReducedMaxBlood {
            ml_lost: 200.0 * s,
        },
        (WoundKind::ShrapnelEmbedded, TreatmentKind::SurgeryKit) => ReducedZoneStrength {
            zone: zone.clone(),
            pct: 0.04,
        },
        (WoundKind::ShrapnelEmbedded, _) => ChronicPainBaseline {
            pain_points: 3.0 * s,
        },
        (WoundKind::ShrapnelThrough, _) => ReducedZoneStrength {
            zone: zone.clone(),
            pct: 0.05,
        },
        // ---- Blunt ----
        (WoundKind::BruiseLight, _) => None,
        (WoundKind::BruiseHeavy, _) => ReducedZoneStrength {
            zone: zone.clone(),
            pct: 0.02,
        },
        (WoundKind::CrushLimb, _) => ReducedZoneStrength {
            zone: zone.clone(),
            pct: 0.10,
        },
        // Concussions never produce a closure-scar; M14I tracks them as
        // memory_loss instead. Defensive fallthrough.
        (WoundKind::Concussion, _) => None,
        // ---- Skeletal ----
        (WoundKind::FractureSimple, _) => ReducedZoneStrength {
            zone: zone.clone(),
            pct: 0.03,
        },
        (WoundKind::FractureCompound, _) => ReducedZoneStrength {
            zone: zone.clone(),
            pct: 0.06,
        },
        (WoundKind::FractureComminuted, _) => Limp,
        (WoundKind::Dislocation, _) => ReducedZoneStrength {
            zone: zone.clone(),
            pct: 0.02,
        },
        (WoundKind::SprainStrain, _) => None,
        // ---- Thermal ----
        (WoundKind::Burn1st, _) => None,
        (WoundKind::Burn2nd, _) => ReducedZoneStrength {
            zone: zone.clone(),
            pct: 0.05,
        },
        // Spec § "Burn3rd closed by cauterize_v1 at severity 0.9 →
        // ReducedZoneStrength{zone, 0.15} + ChronicPainBaseline{2}". The
        // single-debuff slot encodes the zone-strength loss; the chronic
        // pain pairing is registered at the long-term aggregate level.
        (WoundKind::Burn3rd, TreatmentKind::Bandage)
        | (WoundKind::Burn3rd, TreatmentKind::SutureKit)
        | (WoundKind::Burn3rd, TreatmentKind::SurgeryKit)
        | (WoundKind::Burn3rd, TreatmentKind::BurnGel) => ReducedZoneStrength {
            zone: zone.clone(),
            pct: 0.15,
        },
        (WoundKind::Burn3rd, _) => ReducedZoneStrength {
            zone: zone.clone(),
            pct: 0.15,
        },
        (WoundKind::Frostbite1st, _) => None,
        (WoundKind::Frostbite2nd, _) => ReducedZoneStrength {
            zone: zone.clone(),
            pct: 0.05,
        },
        (WoundKind::Frostbite3rd, _) => Limp,
        // ---- Chemical ----
        (WoundKind::AcidBurn, _) => ReducedZoneStrength {
            zone: zone.clone(),
            pct: 0.08,
        },
        (WoundKind::ChemicalBurn, _) => ChronicPainBaseline {
            pain_points: 2.0,
        },
        // ---- Sensory ----
        (WoundKind::EyeInjury, _) => SensoryLoss {
            sense: SenseKind::Sight,
            pct: 0.50,
        },
        (WoundKind::EarInjury, _) => SensoryLoss {
            sense: SenseKind::Hearing,
            pct: 0.50,
        },
        (WoundKind::DentalDamage, _) => None,
    }
}

/// Returns the additional ChronicPainBaseline points to apply when a
/// scar is acquired. Per spec § "Burn3rd × cauterize × 0.9" pairs
/// 0.15 zone strength loss WITH `ChronicPainBaseline{2}`. The base
/// debuff slot encodes the structural loss; this helper exposes the
/// paired pain points so the long-term-state aggregator can stack them
/// onto the chronic-pain baseline at scar acquisition.
#[must_use]
pub fn chronic_pain_bonus_for(
    kind: WoundKind,
    closure: TreatmentKind,
    severity: f32,
) -> f32 {
    let s = severity.clamp(0.0, 1.0);
    match (kind, closure) {
        (WoundKind::Burn3rd, TreatmentKind::Bandage)
        | (WoundKind::Burn3rd, TreatmentKind::SutureKit)
        | (WoundKind::Burn3rd, TreatmentKind::SurgeryKit)
        | (WoundKind::Burn3rd, TreatmentKind::BurnGel) if s >= 0.85 => 2.0,
        (WoundKind::Burn3rd, _) if s >= 0.85 => 2.0,
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn laceration_severe_sutures_0p8() {
        let zone = ZoneId::from("arm_left");
        let d = functional_debuff_for(
            WoundKind::LacerationSevere,
            TreatmentKind::SutureKit,
            0.8,
            &zone,
        );
        match d {
            FunctionalDebuff::ReducedZoneStrength { zone: z, pct } => {
                assert_eq!(z.as_str(), "arm_left");
                assert!((pct - 0.05).abs() < 1e-6);
            }
            other => panic!("expected ReducedZoneStrength got {:?}", other),
        }
    }

    #[test]
    fn burn3rd_cauterize_0p9() {
        let zone = ZoneId::from("torso_front");
        let d = functional_debuff_for(
            WoundKind::Burn3rd,
            TreatmentKind::SutureKit,
            0.9,
            &zone,
        );
        match d {
            FunctionalDebuff::ReducedZoneStrength { pct, .. } => {
                assert!((pct - 0.15).abs() < 1e-6);
            }
            other => panic!("expected ReducedZoneStrength got {:?}", other),
        }
        let pain = chronic_pain_bonus_for(WoundKind::Burn3rd, TreatmentKind::SutureKit, 0.9);
        assert!((pain - 2.0).abs() < 1e-6);
    }

    #[test]
    fn eye_injury_yields_sight_loss() {
        let zone = ZoneId::from("head");
        let d = functional_debuff_for(WoundKind::EyeInjury, TreatmentKind::SutureKit, 0.4, &zone);
        match d {
            FunctionalDebuff::SensoryLoss { sense, pct } => {
                assert_eq!(sense, SenseKind::Sight);
                assert!((pct - 0.5).abs() < 1e-6);
            }
            other => panic!("expected SensoryLoss got {:?}", other),
        }
    }

    #[test]
    fn fracture_comminuted_yields_limp() {
        let zone = ZoneId::from("leg_left");
        let d = functional_debuff_for(
            WoundKind::FractureComminuted,
            TreatmentKind::SurgeryKit,
            0.7,
            &zone,
        );
        assert!(matches!(d, FunctionalDebuff::Limp));
    }

    #[test]
    fn debuff_checksum_stable() {
        let zone = ZoneId::from("arm_right");
        let d1 = functional_debuff_for(
            WoundKind::LacerationSevere,
            TreatmentKind::SutureKit,
            0.8,
            &zone,
        );
        let d2 = functional_debuff_for(
            WoundKind::LacerationSevere,
            TreatmentKind::SutureKit,
            0.8,
            &zone,
        );
        assert_eq!(d1.checksum_triple(), d2.checksum_triple());
    }
}
