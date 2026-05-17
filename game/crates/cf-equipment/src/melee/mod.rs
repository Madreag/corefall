//! M6: melee weapons (4 + Kick + Shoulder check) + M6C: 8 new melee SKUs.
//!
//! Per M6 spec § "4 melee weapons":
//! - Rifle bash (blunt + 30% knockdown)
//! - Knife stab (piercing + bleed chance)
//! - Hatchet (heavy chop)
//! - Baton (high knockdown chance)
//! + Kick (close-range, 60% knockdown)
//! + Shoulder check (during sprint, 80% knockdown)
//!
//! Per M6C § "Melee (8 new)":
//! - dagger_combat, katana, sledgehammer, spear, bayonet, axe_hatchet,
//!   stun_baton, pickaxe_combat_variant.

pub mod axe;
pub mod baton;
pub mod bayonet;
pub mod dagger;
pub mod hatchet;
pub mod katana;
pub mod knife;
pub mod pickaxe;
pub mod rifle_bash;
pub mod sledge;
pub mod spear;
pub mod stun_baton;

use serde::{Deserialize, Serialize};

pub const RIFLE_BASH_M6_DEFAULT_ID: &str = "melee_rifle_bash_m6";
pub const KNIFE_M6_DEFAULT_ID: &str = "melee_knife_m6";
pub const HATCHET_M6_DEFAULT_ID: &str = "melee_hatchet_m6";
pub const BATON_M6_DEFAULT_ID: &str = "melee_baton_m6";

// M6C melee SKU ids.
pub const DAGGER_COMBAT_ID: &str = "dagger_combat";
pub const KATANA_ID: &str = "katana";
pub const SLEDGEHAMMER_ID: &str = "sledgehammer";
pub const SPEAR_ID: &str = "spear";
pub const BAYONET_ID: &str = "bayonet";
pub const AXE_HATCHET_ID: &str = "axe_hatchet";
pub const STUN_BATON_ID: &str = "stun_baton";
pub const PICKAXE_COMBAT_VARIANT_ID: &str = "pickaxe_combat_variant";

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
    // M6C additions:
    Dagger = 6,
    Katana = 7,
    Sledgehammer = 8,
    Spear = 9,
    Bayonet = 10,
    Axe = 11,
    StunBaton = 12,
    Pickaxe = 13,
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
            MeleeKind::Dagger => "dagger",
            MeleeKind::Katana => "katana",
            MeleeKind::Sledgehammer => "sledgehammer",
            MeleeKind::Spear => "spear",
            MeleeKind::Bayonet => "bayonet",
            MeleeKind::Axe => "axe",
            MeleeKind::StunBaton => "stun_baton",
            MeleeKind::Pickaxe => "pickaxe",
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
    /// True when this melee weapon must be attached to a host (bayonet).
    #[serde(default)]
    pub requires_host_weapon: bool,
    /// True when the weapon delivers a non-lethal electric jolt (stun_baton).
    #[serde(default)]
    pub non_lethal_jolt: bool,
    /// True when the weapon can also be used as a mining tool (pickaxe).
    #[serde(default)]
    pub can_mine_terrain: bool,
    /// True when the weapon can breach structural cover (sledgehammer).
    #[serde(default)]
    pub structural_breach: bool,
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

/// M6C melee presets (8 SKUs beyond the M6 baseline).
#[must_use]
pub fn m6c_melee_presets() -> Vec<MeleePreset> {
    vec![
        dagger::dagger_combat(),
        katana::katana(),
        sledge::sledgehammer(),
        spear::spear(),
        bayonet::bayonet(),
        axe::axe_hatchet(),
        stun_baton::stun_baton(),
        pickaxe::pickaxe_combat_variant(),
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
        requires_host_weapon: false,
        non_lethal_jolt: false,
        can_mine_terrain: false,
        structural_breach: false,
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
        requires_host_weapon: false,
        non_lethal_jolt: false,
        can_mine_terrain: false,
        structural_breach: false,
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
