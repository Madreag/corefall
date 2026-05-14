//! M6: melee weapons (4 + Kick + Shoulder check).
//!
//! Per spec § "4 melee weapons":
//! - Rifle bash (blunt + 30% knockdown)
//! - Knife stab (piercing + bleed chance)
//! - Hatchet (heavy chop)
//! - Baton (high knockdown chance)
//! + Kick (close-range, 60% knockdown)
//! + Shoulder check (during sprint, 80% knockdown)

pub mod baton;
pub mod hatchet;
pub mod knife;
pub mod rifle_bash;

use serde::{Deserialize, Serialize};

pub const RIFLE_BASH_M6_DEFAULT_ID: &str = "melee_rifle_bash_m6";
pub const KNIFE_M6_DEFAULT_ID: &str = "melee_knife_m6";
pub const HATCHET_M6_DEFAULT_ID: &str = "melee_hatchet_m6";
pub const BATON_M6_DEFAULT_ID: &str = "melee_baton_m6";

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeleeKind {
    RifleBash = 0,
    Knife = 1,
    Hatchet = 2,
    Baton = 3,
    Kick = 4,
    ShoulderCheck = 5,
}

impl MeleeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            MeleeKind::RifleBash => "rifle_bash",
            MeleeKind::Knife => "knife",
            MeleeKind::Hatchet => "hatchet",
            MeleeKind::Baton => "baton",
            MeleeKind::Kick => "kick",
            MeleeKind::ShoulderCheck => "shoulder_check",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeleePreset {
    pub id: String,
    pub display_name: String,
    pub kind: MeleeKind,
    pub damage: f32,
    /// 0..1 knockdown probability on hit (deterministic; engine threads RNG).
    pub knockdown_chance: f32,
    /// 0..1 bleed-affliction chance.
    pub bleed_chance: f32,
    /// World-units reach.
    pub reach: f32,
    /// Animation duration (seconds).
    pub animation_seconds: f32,
    /// Damage kind label for resistance routing (`blunt`, `piercing`, `slash`).
    pub damage_kind: String,
    pub mass_kg: f32,
}

#[must_use]
pub fn m6_melee_presets() -> Vec<MeleePreset> {
    vec![
        rifle_bash::rifle_bash_m6_default(),
        knife::knife_m6_default(),
        hatchet::hatchet_m6_default(),
        baton::baton_m6_default(),
        kick_default(),
        shoulder_check_default(),
    ]
}

fn kick_default() -> MeleePreset {
    MeleePreset {
        id: "melee_kick_m6".to_string(),
        display_name: "Kick".to_string(),
        kind: MeleeKind::Kick,
        damage: 8.0,
        knockdown_chance: 0.6,
        bleed_chance: 0.0,
        reach: 16.0,
        animation_seconds: 0.35,
        damage_kind: "blunt".to_string(),
        mass_kg: 0.0,
    }
}

fn shoulder_check_default() -> MeleePreset {
    MeleePreset {
        id: "melee_shoulder_check_m6".to_string(),
        display_name: "Shoulder Check".to_string(),
        kind: MeleeKind::ShoulderCheck,
        damage: 10.0,
        knockdown_chance: 0.8,
        bleed_chance: 0.0,
        reach: 12.0,
        animation_seconds: 0.4,
        damage_kind: "blunt".to_string(),
        mass_kg: 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_six_entries() {
        let v = m6_melee_presets();
        assert_eq!(v.len(), 6);
    }

    #[test]
    fn four_named_melee_present() {
        let v = m6_melee_presets();
        assert!(v.iter().any(|m| m.kind == MeleeKind::RifleBash));
        assert!(v.iter().any(|m| m.kind == MeleeKind::Knife));
        assert!(v.iter().any(|m| m.kind == MeleeKind::Hatchet));
        assert!(v.iter().any(|m| m.kind == MeleeKind::Baton));
    }

    #[test]
    fn shoulder_check_higher_knockdown_than_rifle_bash() {
        let v = m6_melee_presets();
        let sc = v.iter().find(|m| m.kind == MeleeKind::ShoulderCheck).unwrap();
        let rb = v.iter().find(|m| m.kind == MeleeKind::RifleBash).unwrap();
        assert!(sc.knockdown_chance > rb.knockdown_chance);
    }
}
