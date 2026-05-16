//! M6: launch-weapon registry (6 weapons).
//!
//! Per spec § "6 launch weapons" table:
//! - Rifle (M1 default; preserved)
//! - SMG
//! - Shotgun
//! - Sniper
//! - Pistol
//! - GrenadeLauncher

pub mod grenade_launcher;
pub mod pistol;
pub mod shotgun;
pub mod smg;
pub mod sniper;

use serde::{Deserialize, Serialize};

use crate::fire_modes::AdvancedFireMode;
use crate::RifleSpec;

pub const SMG_M6_DEFAULT_ID: &str = "smg_m6_default";
pub const SHOTGUN_M6_DEFAULT_ID: &str = "shotgun_m6_default";
pub const SNIPER_M6_DEFAULT_ID: &str = "sniper_m6_default";
pub const PISTOL_M6_DEFAULT_ID: &str = "pistol_m6_default";
pub const GRENADE_LAUNCHER_M6_DEFAULT_ID: &str = "grenade_launcher_m6_default";

/// Top-level weapon category for M6 weapon registry.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeaponClass {
    Rifle = 0,
    Smg = 1,
    Shotgun = 2,
    Sniper = 3,
    Pistol = 4,
    GrenadeLauncher = 5,
}

impl WeaponClass {
    pub fn as_str(self) -> &'static str {
        match self {
            WeaponClass::Rifle => "rifle",
            WeaponClass::Smg => "smg",
            WeaponClass::Shotgun => "shotgun",
            WeaponClass::Sniper => "sniper",
            WeaponClass::Pistol => "pistol",
            WeaponClass::GrenadeLauncher => "grenade_launcher",
        }
    }
}

/// M6 weapon descriptor: rifle-spec compatible firing data + class metadata
/// + fire-mode set + per-shot loudness multiplier (extended over M1's scalar).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WeaponPreset {
    pub id: String,
    pub display_name: String,
    pub class: WeaponClass,
    pub firing: RifleSpec,
    pub available_modes: Vec<AdvancedFireMode>,
    pub default_mode: AdvancedFireMode,
    /// Mass in kg.
    pub mass_kg: f32,
    /// Effective range in world units (cosmetic / HUD).
    pub effective_range: f32,
}

impl WeaponPreset {
    pub fn new(
        id: &str,
        display_name: &str,
        class: WeaponClass,
        firing: RifleSpec,
        modes: Vec<AdvancedFireMode>,
        mass_kg: f32,
        effective_range: f32,
    ) -> Self {
        let default_mode = modes.first().copied().unwrap_or(AdvancedFireMode::Single);
        Self {
            id: id.to_string(),
            display_name: display_name.to_string(),
            class,
            firing,
            available_modes: modes,
            default_mode,
            mass_kg: mass_kg.max(0.0),
            effective_range: effective_range.max(0.0),
        }
    }
}

/// M6 launch weapon registry (excluding the M1 rifle preset, which lives in
/// the original `rifle_presets` map).
#[must_use]
pub fn m6_weapon_presets() -> Vec<WeaponPreset> {
    vec![
        smg::smg_m6_default(),
        shotgun::shotgun_m6_default(),
        sniper::sniper_m6_default(),
        pistol::pistol_m6_default(),
        grenade_launcher::grenade_launcher_m6_default(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_count_at_least_five_new() {
        let v = m6_weapon_presets();
        assert!(v.len() >= 5);
    }

    #[test]
    fn each_preset_has_modes() {
        for p in m6_weapon_presets() {
            assert!(!p.available_modes.is_empty(), "{} has no modes", p.id);
        }
    }
}
