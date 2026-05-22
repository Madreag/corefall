//! M7-A: basic personality + mood/stress surface.
//!
//! 20 launch traits + per-actor mood accumulator. M7-B promotes this into
//! the dedicated `cf-personality` crate with the full retention loop and
//! cfctl methods; M7-A ships the surface so the thinking stack can already
//! consume modifier multipliers + retreat-threshold deltas.

use serde::{Deserialize, Serialize};

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersonalityTrait {
    Brave,
    Coward,
    Loyal,
    Disloyal,
    Merciful,
    Cruel,
    Careful,
    Reckless,
    SharpShooter,
    TerribleShooter,
    HotHeaded,
    Calm,
    FastReflexes,
    SlowReflexes,
    GunCollector,
    Peaceful,
    Paranoid,
    LoneWolf,
    SocialGlue,
    Veteran,
}

impl PersonalityTrait {
    pub const ALL: [PersonalityTrait; 20] = [
        PersonalityTrait::Brave,
        PersonalityTrait::Coward,
        PersonalityTrait::Loyal,
        PersonalityTrait::Disloyal,
        PersonalityTrait::Merciful,
        PersonalityTrait::Cruel,
        PersonalityTrait::Careful,
        PersonalityTrait::Reckless,
        PersonalityTrait::SharpShooter,
        PersonalityTrait::TerribleShooter,
        PersonalityTrait::HotHeaded,
        PersonalityTrait::Calm,
        PersonalityTrait::FastReflexes,
        PersonalityTrait::SlowReflexes,
        PersonalityTrait::GunCollector,
        PersonalityTrait::Peaceful,
        PersonalityTrait::Paranoid,
        PersonalityTrait::LoneWolf,
        PersonalityTrait::SocialGlue,
        PersonalityTrait::Veteran,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            PersonalityTrait::Brave => "brave",
            PersonalityTrait::Coward => "coward",
            PersonalityTrait::Loyal => "loyal",
            PersonalityTrait::Disloyal => "disloyal",
            PersonalityTrait::Merciful => "merciful",
            PersonalityTrait::Cruel => "cruel",
            PersonalityTrait::Careful => "careful",
            PersonalityTrait::Reckless => "reckless",
            PersonalityTrait::SharpShooter => "sharp_shooter",
            PersonalityTrait::TerribleShooter => "terrible_shooter",
            PersonalityTrait::HotHeaded => "hot_headed",
            PersonalityTrait::Calm => "calm",
            PersonalityTrait::FastReflexes => "fast_reflexes",
            PersonalityTrait::SlowReflexes => "slow_reflexes",
            PersonalityTrait::GunCollector => "gun_collector",
            PersonalityTrait::Peaceful => "peaceful",
            PersonalityTrait::Paranoid => "paranoid",
            PersonalityTrait::LoneWolf => "lone_wolf",
            PersonalityTrait::SocialGlue => "social_glue",
            PersonalityTrait::Veteran => "veteran",
        }
    }

    pub fn from_str(value: &str) -> Option<PersonalityTrait> {
        Some(match value {
            "brave" => PersonalityTrait::Brave,
            "coward" => PersonalityTrait::Coward,
            "loyal" => PersonalityTrait::Loyal,
            "disloyal" => PersonalityTrait::Disloyal,
            "merciful" => PersonalityTrait::Merciful,
            "cruel" => PersonalityTrait::Cruel,
            "careful" => PersonalityTrait::Careful,
            "reckless" => PersonalityTrait::Reckless,
            "sharp_shooter" => PersonalityTrait::SharpShooter,
            "terrible_shooter" => PersonalityTrait::TerribleShooter,
            "hot_headed" => PersonalityTrait::HotHeaded,
            "calm" => PersonalityTrait::Calm,
            "fast_reflexes" => PersonalityTrait::FastReflexes,
            "slow_reflexes" => PersonalityTrait::SlowReflexes,
            "gun_collector" => PersonalityTrait::GunCollector,
            "peaceful" => PersonalityTrait::Peaceful,
            "paranoid" => PersonalityTrait::Paranoid,
            "lone_wolf" => PersonalityTrait::LoneWolf,
            "social_glue" => PersonalityTrait::SocialGlue,
            "veteran" => PersonalityTrait::Veteran,
            _ => return None,
        })
    }

    pub fn aim_accuracy_modifier(self) -> f32 {
        match self {
            PersonalityTrait::SharpShooter => 1.15,
            PersonalityTrait::TerribleShooter => 0.70,
            PersonalityTrait::Veteran => 1.05,
            _ => 1.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PersonalityProfile {
    pub display_name: String,
    pub backstory: String,
    pub traits: Vec<PersonalityTrait>,
    /// Mood accumulator. [-100, +100]. 0 = neutral. Spec § Mood/stress.
    pub mood: f32,
    /// Stress accumulator. [0, +100]. Increases under sustained combat.
    pub stress: f32,
}

impl PersonalityProfile {
    pub fn has_trait(&self, name: &str) -> bool {
        let Some(t) = PersonalityTrait::from_str(name) else {
            return false;
        };
        self.traits.contains(&t)
    }

    pub fn aim_accuracy_modifier(&self) -> f32 {
        let mut m: f32 = 1.0;
        for t in &self.traits {
            m *= t.aim_accuracy_modifier();
        }
        // Mood stress reduces accuracy.
        let mood_factor = (1.0 + self.mood / 1000.0).clamp(0.85, 1.15);
        (m * mood_factor).clamp(0.5, 1.5)
    }

    /// Apply a mood delta (clamped to [-100, +100]).
    pub fn adjust_mood(&mut self, delta: f32) {
        self.mood = (self.mood + delta).clamp(-100.0, 100.0);
    }

    /// Apply a stress delta (clamped to [0, +100]).
    pub fn adjust_stress(&mut self, delta: f32) {
        self.stress = (self.stress + delta).clamp(0.0, 100.0);
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoodChangedEvent {
    pub actor_id: u64,
    pub delta: f32,
    pub new_mood: f32,
    pub cause: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_twenty_traits_round_trip() {
        for t in PersonalityTrait::ALL.iter() {
            assert_eq!(PersonalityTrait::from_str(t.as_str()), Some(*t));
        }
    }

    #[test]
    fn sharp_shooter_increases_accuracy() {
        let mut p = PersonalityProfile::default();
        p.traits.push(PersonalityTrait::SharpShooter);
        assert!(p.aim_accuracy_modifier() > 1.1);
    }

    #[test]
    fn terrible_shooter_reduces_accuracy() {
        let mut p = PersonalityProfile::default();
        p.traits.push(PersonalityTrait::TerribleShooter);
        assert!(p.aim_accuracy_modifier() < 0.8);
    }

    #[test]
    fn mood_clamped_to_hundred() {
        let mut p = PersonalityProfile::default();
        p.adjust_mood(150.0);
        assert_eq!(p.mood, 100.0);
        p.adjust_mood(-300.0);
        assert_eq!(p.mood, -100.0);
    }
}
