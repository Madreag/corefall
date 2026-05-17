//! M6C: heavy weapons registry (8 SKUs).
//!
//! Per M6C § "Heavy weapons (8 new)":
//! - RPG launcher (HEAT) — M14C HEAT round
//! - Tank autocannon — M14C APFSDS
//! - Mortar (60mm) — indirect fire + crew-served
//! - Recoilless rifle — anti-armor + back-blast
//! - ATGM Javelin — fire-and-forget + top-attack HEAT
//! - Flamethrower — sustained fire spray (M15 fire)
//! - Plasma cannon — exotic; long range
//! - Gauss rifle (anti-materiel) — electromagnetic + high damage

pub mod atgm;
pub mod flamethrower;
pub mod mortar;
pub mod plasma_cannon;
pub mod recoilless;

use serde::{Deserialize, Serialize};

pub const RPG_LAUNCHER_HEAT_ID: &str = "rpg_launcher_heat";
pub const TANK_AUTOCANNON_M14C_ID: &str = "tank_autocannon_m14c";
pub const MORTAR_60MM_ID: &str = "mortar_60mm";
pub const RECOILLESS_RIFLE_ID: &str = "recoilless_rifle";
pub const ATGM_JAVELIN_ID: &str = "atgm_javelin";
pub const FLAMETHROWER_ID: &str = "flamethrower";
pub const PLASMA_CANNON_M48_ID: &str = "plasma_cannon_m48";
pub const GAUSS_RIFLE_ANTI_MATERIEL_ID: &str = "gauss_rifle_anti_materiel";

/// Category of a heavy weapon. Drives ammo-routing, crew-served gates, and
/// HUD widget selection.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HeavyWeaponKind {
    RpgHeat = 0,
    TankAutocannon = 1,
    Mortar = 2,
    Recoilless = 3,
    Atgm = 4,
    Flamethrower = 5,
    PlasmaCannon = 6,
    GaussRifle = 7,
}

impl HeavyWeaponKind {
    pub fn as_str(self) -> &'static str {
        match self {
            HeavyWeaponKind::RpgHeat => "rpg_heat",
            HeavyWeaponKind::TankAutocannon => "tank_autocannon",
            HeavyWeaponKind::Mortar => "mortar",
            HeavyWeaponKind::Recoilless => "recoilless",
            HeavyWeaponKind::Atgm => "atgm",
            HeavyWeaponKind::Flamethrower => "flamethrower",
            HeavyWeaponKind::PlasmaCannon => "plasma_cannon",
            HeavyWeaponKind::GaussRifle => "gauss_rifle",
        }
    }
}

/// Heavy weapon preset descriptor. Per M14C the ammo profile (HEAT / APFSDS
/// / fragment) is carried as a string id so M14C consumers can resolve to
/// the canonical round spec without circular crate deps.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeavyWeaponPreset {
    pub id: String,
    pub display_name: String,
    pub kind: HeavyWeaponKind,
    /// Mass in kg (drives M14A mass aggregation).
    pub mass_kg: f32,
    /// Effective range in world units.
    pub effective_range: f32,
    /// Damage per primary projectile (cosmetic baseline; M14C routes overrides).
    pub damage_per_hit: f32,
    /// Magazine / loader capacity (rounds or canister volume).
    pub mag_capacity: u32,
    /// Reload seconds (full single-round chamber for crew-served weapons).
    pub reload_seconds: f32,
    /// Per-shot recoil impulse in world units / s.
    pub recoil_impulse: f32,
    /// Number of crew members required to operate (1 = solo; 2 = gunner+loader).
    pub crew_required: u8,
    /// True when the weapon consumes ammo from a tank-slot canister rather
    /// than an inline magazine (Flamethrower fuel, Mortar tube feeder).
    pub uses_tank_canister: bool,
    /// Optional id of the M14C ammo profile this weapon launches.
    pub ammo_profile_id: String,
    /// True when the projectile is top-attack (ATGM Javelin).
    pub top_attack: bool,
    /// Lock acquisition time in seconds (0 = no lock required).
    pub lock_seconds: f32,
    /// True when the weapon produces a back-blast cone.
    pub back_blast: bool,
}

#[must_use]
pub fn m6c_heavy_presets() -> Vec<HeavyWeaponPreset> {
    vec![
        rpg_launcher_heat(),
        tank_autocannon_m14c(),
        mortar_60mm(),
        recoilless_rifle(),
        atgm_javelin(),
        flamethrower_preset(),
        plasma_cannon_m48(),
        gauss_rifle_anti_materiel(),
    ]
}

fn rpg_launcher_heat() -> HeavyWeaponPreset {
    HeavyWeaponPreset {
        id: RPG_LAUNCHER_HEAT_ID.to_string(),
        display_name: "RPG Launcher (HEAT)".to_string(),
        kind: HeavyWeaponKind::RpgHeat,
        mass_kg: 7.5,
        effective_range: 500.0,
        damage_per_hit: 180.0,
        mag_capacity: 1,
        reload_seconds: 4.5,
        recoil_impulse: 120.0,
        crew_required: 1,
        uses_tank_canister: false,
        ammo_profile_id: "heat_rocket_85mm".to_string(),
        top_attack: false,
        lock_seconds: 0.0,
        back_blast: true,
    }
}

fn tank_autocannon_m14c() -> HeavyWeaponPreset {
    HeavyWeaponPreset {
        id: TANK_AUTOCANNON_M14C_ID.to_string(),
        display_name: "Tank Autocannon (APFSDS)".to_string(),
        kind: HeavyWeaponKind::TankAutocannon,
        mass_kg: 95.0,
        effective_range: 1800.0,
        damage_per_hit: 220.0,
        mag_capacity: 30,
        reload_seconds: 6.0,
        recoil_impulse: 220.0,
        crew_required: 1,
        uses_tank_canister: false,
        ammo_profile_id: "apfsds_30mm".to_string(),
        top_attack: false,
        lock_seconds: 0.0,
        back_blast: false,
    }
}

fn mortar_60mm() -> HeavyWeaponPreset {
    HeavyWeaponPreset {
        id: MORTAR_60MM_ID.to_string(),
        display_name: "60mm Mortar".to_string(),
        kind: HeavyWeaponKind::Mortar,
        mass_kg: 22.0,
        effective_range: 2200.0,
        damage_per_hit: 140.0,
        mag_capacity: 1,
        reload_seconds: 5.0,
        recoil_impulse: 60.0,
        crew_required: 2,
        uses_tank_canister: false,
        ammo_profile_id: "mortar_shell_60mm".to_string(),
        top_attack: false,
        lock_seconds: 0.0,
        back_blast: false,
    }
}

fn recoilless_rifle() -> HeavyWeaponPreset {
    HeavyWeaponPreset {
        id: RECOILLESS_RIFLE_ID.to_string(),
        display_name: "Recoilless Rifle".to_string(),
        kind: HeavyWeaponKind::Recoilless,
        mass_kg: 14.5,
        effective_range: 1000.0,
        damage_per_hit: 200.0,
        mag_capacity: 1,
        reload_seconds: 5.5,
        recoil_impulse: 0.0,
        crew_required: 1,
        uses_tank_canister: false,
        ammo_profile_id: "recoilless_84mm".to_string(),
        top_attack: false,
        lock_seconds: 0.0,
        back_blast: true,
    }
}

fn atgm_javelin() -> HeavyWeaponPreset {
    HeavyWeaponPreset {
        id: ATGM_JAVELIN_ID.to_string(),
        display_name: "ATGM Javelin".to_string(),
        kind: HeavyWeaponKind::Atgm,
        mass_kg: 22.0,
        effective_range: 2500.0,
        damage_per_hit: 360.0,
        mag_capacity: 1,
        reload_seconds: 8.0,
        recoil_impulse: 0.0,
        crew_required: 1,
        uses_tank_canister: false,
        ammo_profile_id: "atgm_tandem_heat".to_string(),
        top_attack: true,
        lock_seconds: atgm::ATGM_LOCK_ACQUISITION_SECONDS,
        back_blast: false,
    }
}

fn flamethrower_preset() -> HeavyWeaponPreset {
    HeavyWeaponPreset {
        id: FLAMETHROWER_ID.to_string(),
        display_name: "Flamethrower".to_string(),
        kind: HeavyWeaponKind::Flamethrower,
        mass_kg: 23.0,
        effective_range: 18.0,
        damage_per_hit: 12.0,
        mag_capacity: 0,
        reload_seconds: 0.0,
        recoil_impulse: 8.0,
        crew_required: 1,
        uses_tank_canister: true,
        ammo_profile_id: "fuel_canister_napalm".to_string(),
        top_attack: false,
        lock_seconds: 0.0,
        back_blast: false,
    }
}

fn plasma_cannon_m48() -> HeavyWeaponPreset {
    HeavyWeaponPreset {
        id: PLASMA_CANNON_M48_ID.to_string(),
        display_name: "Plasma Cannon (M48)".to_string(),
        kind: HeavyWeaponKind::PlasmaCannon,
        mass_kg: 18.0,
        effective_range: 900.0,
        damage_per_hit: 180.0,
        mag_capacity: 6,
        reload_seconds: 3.5,
        recoil_impulse: 70.0,
        crew_required: 1,
        uses_tank_canister: true,
        ammo_profile_id: "plasma_bolt".to_string(),
        top_attack: false,
        lock_seconds: 0.0,
        back_blast: false,
    }
}

fn gauss_rifle_anti_materiel() -> HeavyWeaponPreset {
    HeavyWeaponPreset {
        id: GAUSS_RIFLE_ANTI_MATERIEL_ID.to_string(),
        display_name: "Gauss Rifle (Anti-Materiel)".to_string(),
        kind: HeavyWeaponKind::GaussRifle,
        mass_kg: 17.0,
        effective_range: 2400.0,
        damage_per_hit: 260.0,
        mag_capacity: 4,
        reload_seconds: 4.0,
        recoil_impulse: 140.0,
        crew_required: 1,
        uses_tank_canister: true,
        ammo_profile_id: "gauss_slug_127mm".to_string(),
        top_attack: false,
        lock_seconds: 0.0,
        back_blast: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_eight_kinds() {
        let v = m6c_heavy_presets();
        assert_eq!(v.len(), 8);
        let kinds: Vec<HeavyWeaponKind> = v.iter().map(|p| p.kind).collect();
        for k in [
            HeavyWeaponKind::RpgHeat,
            HeavyWeaponKind::TankAutocannon,
            HeavyWeaponKind::Mortar,
            HeavyWeaponKind::Recoilless,
            HeavyWeaponKind::Atgm,
            HeavyWeaponKind::Flamethrower,
            HeavyWeaponKind::PlasmaCannon,
            HeavyWeaponKind::GaussRifle,
        ] {
            assert!(kinds.contains(&k), "missing kind {:?}", k);
        }
    }

    #[test]
    fn mortar_is_crew_served() {
        let v = m6c_heavy_presets();
        let m = v.iter().find(|p| p.kind == HeavyWeaponKind::Mortar).unwrap();
        assert_eq!(m.crew_required, 2);
    }

    #[test]
    fn atgm_is_top_attack_with_3s_lock() {
        let v = m6c_heavy_presets();
        let a = v.iter().find(|p| p.kind == HeavyWeaponKind::Atgm).unwrap();
        assert!(a.top_attack);
        assert!((a.lock_seconds - 3.0).abs() < 1e-3);
    }

    #[test]
    fn flamethrower_uses_tank_canister() {
        let v = m6c_heavy_presets();
        let f = v.iter().find(|p| p.kind == HeavyWeaponKind::Flamethrower).unwrap();
        assert!(f.uses_tank_canister);
        assert_eq!(f.ammo_profile_id, "fuel_canister_napalm");
    }
}
