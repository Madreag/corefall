//! M7-A: retreat decision + event helpers.
//!
//! `ai.retreat_decision` fires when a bot's HP drops below its archetype's
//! retreat threshold OR when the squad's combat effectiveness is broken
//! (squadmate dead). The bot prefers Reload + Search tactics over Attack
//! while retreating.

use serde::{Deserialize, Serialize};

use crate::personality::PersonalityProfile;

/// `ai.retreat_decision.reason` field.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetreatReason {
    HpLow,
    SquadDead,
    OverWhelmed,
    OrderReceived,
}

impl RetreatReason {
    pub fn as_str(self) -> &'static str {
        match self {
            RetreatReason::HpLow => "hp_low",
            RetreatReason::SquadDead => "squad_dead",
            RetreatReason::OverWhelmed => "overwhelmed",
            RetreatReason::OrderReceived => "order_received",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetreatDecisionEvent {
    pub actor_id: u64,
    pub reason: RetreatReason,
    /// HP fraction at the moment of the decision (0.0..=1.0).
    pub hp_fraction: f32,
    /// Tick on which the decision fired.
    pub tick: u64,
}

/// personality traits + mood/stress. Default is 30% (per spec). Brave
/// trait lowers to 15%; coward raises to 50%. Stressed mood raises by 10%.
pub fn effective_retreat_threshold(profile: &PersonalityProfile) -> f32 {
    let mut threshold: f32 = 0.30;
    if profile.has_trait("brave") {
        threshold = 0.15;
    } else if profile.has_trait("coward") {
        threshold = 0.50;
    }
    let mood_term = (-profile.mood / 200.0).clamp(0.0, 0.20);
    (threshold + mood_term).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::personality::PersonalityTrait;

    #[test]
    fn default_threshold_thirty_percent() {
        let p = PersonalityProfile::default();
        assert!((effective_retreat_threshold(&p) - 0.30).abs() < f32::EPSILON);
    }

    #[test]
    fn brave_trait_lowers_threshold() {
        let mut p = PersonalityProfile::default();
        p.traits.push(PersonalityTrait::Brave);
        assert!((effective_retreat_threshold(&p) - 0.15).abs() < f32::EPSILON);
    }

    #[test]
    fn coward_trait_raises_threshold() {
        let mut p = PersonalityProfile::default();
        p.traits.push(PersonalityTrait::Coward);
        assert!((effective_retreat_threshold(&p) - 0.50).abs() < f32::EPSILON);
    }
}
