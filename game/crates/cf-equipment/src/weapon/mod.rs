//! M6: launch-weapon registry (6 weapons) + M6C: 12 firearm SKUs.
//!
//! Per M6 spec § "6 launch weapons" table:
//! - Rifle (M1 default; preserved)
//! - SMG
//! - Shotgun
//! - Sniper
//! - Pistol
//! - GrenadeLauncher
//!
//! Per M6C § "Firearms (12 new beyond M6 baseline)":
//! - revolver_357, submachine_gun_9mm, assault_rifle_t2, sniper_rifle_t2,
//!   shotgun_pump, heavy_machine_gun_50cal, battle_rifle_762,
//!   carbine_compact, dmr_762, lmg_belt_fed, anti_materiel_rifle_127,
//!   squad_automatic_saw.

pub mod anti_materiel;
pub mod assault_rifle_t2;
pub mod battle_rifle;
pub mod carbine;
pub mod dmr;
pub mod grenade_launcher;
pub mod hmg;
pub mod lmg;
pub mod pistol;
pub mod revolver;
pub mod saw;
pub mod shotgun;
pub mod shotgun_pump;
pub mod smg;
pub mod smg9;
pub mod sniper;
pub mod sniper_t2;

use serde::{Deserialize, Serialize};

use crate::fire_modes::AdvancedFireMode;
use crate::RifleSpec;

pub const SMG_M6_DEFAULT_ID: &str = "smg_m6_default";
pub const SHOTGUN_M6_DEFAULT_ID: &str = "shotgun_m6_default";
pub const SNIPER_M6_DEFAULT_ID: &str = "sniper_m6_default";
pub const PISTOL_M6_DEFAULT_ID: &str = "pistol_m6_default";
pub const GRENADE_LAUNCHER_M6_DEFAULT_ID: &str = "grenade_launcher_m6_default";

// M6C firearm SKU ids.
pub const REVOLVER_357_ID: &str = "revolver_357";
pub const SUBMACHINE_GUN_9MM_ID: &str = "submachine_gun_9mm";
pub const ASSAULT_RIFLE_T2_ID: &str = "assault_rifle_t2";
pub const SNIPER_RIFLE_T2_ID: &str = "sniper_rifle_t2";
pub const SHOTGUN_PUMP_ID: &str = "shotgun_pump";
pub const HEAVY_MACHINE_GUN_50CAL_ID: &str = "heavy_machine_gun_50cal";
pub const BATTLE_RIFLE_762_ID: &str = "battle_rifle_762";
pub const CARBINE_COMPACT_ID: &str = "carbine_compact";
pub const DMR_762_ID: &str = "dmr_762";
pub const LMG_BELT_FED_ID: &str = "lmg_belt_fed";
pub const ANTI_MATERIEL_RIFLE_127_ID: &str = "anti_materiel_rifle_127";
pub const SQUAD_AUTOMATIC_SAW_ID: &str = "squad_automatic_saw";

/// Top-level weapon category for M6 + M6C weapon registry.
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
    // M6C additions:
    Revolver = 6,
    Carbine = 7,
    BattleRifle = 8,
    Dmr = 9,
    Lmg = 10,
    Hmg = 11,
    AntiMateriel = 12,
    Saw = 13,
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
            WeaponClass::Revolver => "revolver",
            WeaponClass::Carbine => "carbine",
            WeaponClass::BattleRifle => "battle_rifle",
            WeaponClass::Dmr => "dmr",
            WeaponClass::Lmg => "lmg",
            WeaponClass::Hmg => "hmg",
            WeaponClass::AntiMateriel => "anti_materiel",
            WeaponClass::Saw => "saw",
        }
    }
}

/// M6 weapon descriptor: rifle-spec compatible firing data + class metadata
/// + fire-mode set + per-shot loudness multiplier (extended over M1's scalar).
///
/// **M6C** added `crew_required` + `bipod_compatible` + `vehicle_mountable`
/// fields so the spec-literal firearm descriptors (`heavy_machine_gun_50cal`
/// "crew-served; vehicle-mountable", `squad_automatic_saw` "bipod + sustained
/// suppress") have concrete data the engine can gate on. All three are
/// `#[serde(default)]` so older presets round-trip cleanly.
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
    /// **M6C**: minimum crew size to operate (1 = solo; 2 = gunner+loader).
    /// Default 1 keeps existing M6 presets unchanged.
    #[serde(default = "default_crew_required")]
    pub crew_required: u8,
    /// **M6C**: true when this weapon can deploy a bipod for sustained
    /// suppression. Drives `act.player.deploy_bipod` validity.
    #[serde(default)]
    pub bipod_compatible: bool,
    /// **M6C**: true when this weapon can be vehicle-mounted (HMG, etc).
    #[serde(default)]
    pub vehicle_mountable: bool,
}

fn default_crew_required() -> u8 {
    1
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
            crew_required: default_crew_required(),
            bipod_compatible: false,
            vehicle_mountable: false,
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

/// M6C firearm presets (12 SKUs beyond the M6 baseline).
#[must_use]
pub fn m6c_firearm_presets() -> Vec<WeaponPreset> {
    vec![
        revolver::revolver_357(),
        smg9::submachine_gun_9mm(),
        assault_rifle_t2::assault_rifle_t2(),
        sniper_t2::sniper_rifle_t2(),
        shotgun_pump::shotgun_pump(),
        hmg::heavy_machine_gun_50cal(),
        battle_rifle::battle_rifle_762(),
        carbine::carbine_compact(),
        dmr::dmr_762(),
        lmg::lmg_belt_fed(),
        anti_materiel::anti_materiel_rifle_127(),
        saw::squad_automatic_saw(),
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

    #[test]
    fn m6c_firearm_registry_has_twelve_skus() {
        let v = m6c_firearm_presets();
        assert_eq!(v.len(), 12);
        for p in &v {
            assert!(!p.available_modes.is_empty(), "{} has no modes", p.id);
        }
    }

    #[test]
    fn m6c_firearm_ids_are_unique() {
        use std::collections::BTreeSet;
        let ids: BTreeSet<String> = m6c_firearm_presets().into_iter().map(|p| p.id).collect();
        assert_eq!(ids.len(), 12);
    }

    #[test]
    fn m6c_hmg_is_crew_served_and_vehicle_mountable() {
        // M6C spec literal: "heavy_machine_gun_50cal — crew-served; vehicle-mountable".
        let presets = m6c_firearm_presets();
        let hmg = presets
            .iter()
            .find(|p| p.id == HEAVY_MACHINE_GUN_50CAL_ID)
            .expect("HMG must be registered");
        assert_eq!(hmg.crew_required, 2);
        assert!(hmg.vehicle_mountable);
        assert!(hmg.bipod_compatible);
    }

    #[test]
    fn m6c_saw_is_bipod_compatible() {
        // M6C spec literal: "squad_automatic_saw — bipod + sustained suppress".
        let presets = m6c_firearm_presets();
        let saw = presets
            .iter()
            .find(|p| p.id == SQUAD_AUTOMATIC_SAW_ID)
            .expect("SAW must be registered");
        assert!(saw.bipod_compatible);
    }

    #[test]
    fn other_firearms_default_to_solo_crew() {
        let presets = m6c_firearm_presets();
        for p in &presets {
            if p.id == HEAVY_MACHINE_GUN_50CAL_ID {
                continue;
            }
            assert_eq!(p.crew_required, 1, "{} unexpectedly multi-crewed", p.id);
        }
    }
}
