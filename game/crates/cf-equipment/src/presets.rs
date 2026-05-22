//! Stable preset registries: rifle [`RifleSpec`] presets, [`RoleRecord`] role
//! records, [`Loadout`] LOAD-A fixtures, and the [`available_fire_modes_for`]
//! ladder used by `act.player.cycle_fire_mode`.

use std::collections::BTreeMap;

use crate::fire_mode::FireMode;
use crate::fire_modes::AdvancedFireMode;
use crate::loadout::Loadout;
use crate::magazine::RoundKind;
use crate::rifle_spec::{
    default_loudness_scalar, default_recoil_decay_rate, RifleSpec, CARBINE_M5_POWERED_ID,
    RIFLE_M1_DEFAULT_ID, RIFLE_M5_MECH_HEAVY_ID,
};
use crate::role::{AiPolicyHint, RoleKind, RoleRecord};
use crate::weapon::m6_weapon_presets;

pub(crate) fn rifle_m1_default() -> RifleSpec {
    RifleSpec {
        preset_id: RIFLE_M1_DEFAULT_ID.to_string(),
        fire_interval_seconds: 0.1,
        mag_capacity: 30,
        reload_seconds: 1.5,
        recoil_impulse: 25.0,
        muzzle_forward_offset: 12.0,
        muzzle_vertical_offset: 4.0,
        projectile_speed: 1200.0,
        damage_per_hit: 12.0,
        projectile_lifetime_seconds: 1.5,
        recoil_decay_rate: default_recoil_decay_rate(),
        loudness: default_loudness_scalar(),
        inherits_firer_velocity: true,
        particle_count: 1,
        spread_radians: 0.0,
        tracer_round_to_total_ratio: 0,
        ai_fire_vel: 1200.0,
        ai_penetration: 0.0,
        ai_life_time: 1.5,
        ai_blast_radius: 0.0,
        fire_mode: FireMode::Semi,
        primary_round: RoundKind::Regular,
        bullet_mass_kg: 0.05,
        bullet_sharpness: 0.8,
    }
}

pub const SHOTGUN_M1_DEFAULT_ID: &str = "shotgun_m1_default";

fn shotgun_m1_default() -> RifleSpec {
    RifleSpec {
        preset_id: SHOTGUN_M1_DEFAULT_ID.to_string(),
        fire_interval_seconds: 0.7,
        mag_capacity: 6,
        reload_seconds: 2.5,
        recoil_impulse: 60.0,
        muzzle_forward_offset: 12.0,
        muzzle_vertical_offset: 4.0,
        projectile_speed: 900.0,
        damage_per_hit: 8.0,
        projectile_lifetime_seconds: 0.6,
        recoil_decay_rate: default_recoil_decay_rate(),
        loudness: 1.3,
        inherits_firer_velocity: true,
        particle_count: 8,
        spread_radians: 0.15,
        tracer_round_to_total_ratio: 0,
        ai_fire_vel: 900.0,
        ai_penetration: 0.0,
        ai_life_time: 0.6,
        ai_blast_radius: 0.0,
        fire_mode: FireMode::Semi,
        primary_round: RoundKind::Pellet,
        bullet_mass_kg: 0.05,
        bullet_sharpness: 0.8,
    }
}

/// (`RTTRatio` per CCCP `Magazine`). Same baseline as the default rifle but
/// with `tracer_round_to_total_ratio=4` (every 4th shot is a tracer).
pub const RIFLE_M1_TRACER_ID: &str = "rifle_m1_tracer";

fn rifle_m1_tracer() -> RifleSpec {
    let mut spec = rifle_m1_default();
    spec.preset_id = RIFLE_M1_TRACER_ID.to_string();
    spec.tracer_round_to_total_ratio = 4;
    spec
}

/// M5 powered-armor carbine: 12 RPS, 25-round magazine, slightly less damage
/// per shot, faster reload. AI policy hint = Primary.
fn carbine_m5_powered() -> RifleSpec {
    RifleSpec {
        preset_id: CARBINE_M5_POWERED_ID.to_string(),
        fire_interval_seconds: 0.083,
        mag_capacity: 25,
        reload_seconds: 1.2,
        recoil_impulse: 20.0,
        muzzle_forward_offset: 14.0,
        muzzle_vertical_offset: 6.0,
        projectile_speed: 1400.0,
        damage_per_hit: 9.0,
        projectile_lifetime_seconds: 1.5,
        recoil_decay_rate: default_recoil_decay_rate(),
        loudness: default_loudness_scalar(),
        inherits_firer_velocity: true,
        particle_count: 1,
        spread_radians: 0.0,
        tracer_round_to_total_ratio: 0,
        ai_fire_vel: 1400.0,
        ai_penetration: 0.0,
        ai_life_time: 1.5,
        ai_blast_radius: 0.0,
        fire_mode: FireMode::FullAuto,
        primary_round: RoundKind::Regular,
        bullet_mass_kg: 0.05,
        bullet_sharpness: 0.8,
    }
}

/// M5 mech-heavy rifle: 4 RPS, 15-round magazine, much higher damage per
/// shot, slower reload. AI policy hint = Primary.
fn rifle_m5_mech_heavy() -> RifleSpec {
    RifleSpec {
        preset_id: RIFLE_M5_MECH_HEAVY_ID.to_string(),
        fire_interval_seconds: 0.25,
        mag_capacity: 15,
        reload_seconds: 2.5,
        recoil_impulse: 60.0,
        muzzle_forward_offset: 22.0,
        muzzle_vertical_offset: 8.0,
        projectile_speed: 1100.0,
        damage_per_hit: 40.0,
        projectile_lifetime_seconds: 2.0,
        recoil_decay_rate: default_recoil_decay_rate(),
        loudness: 1.5,
        inherits_firer_velocity: true,
        particle_count: 1,
        spread_radians: 0.0,
        tracer_round_to_total_ratio: 4,
        ai_fire_vel: 1100.0,
        ai_penetration: 0.0,
        ai_life_time: 2.0,
        ai_blast_radius: 0.0,
        fire_mode: FireMode::Semi,
        primary_round: RoundKind::Regular,
        bullet_mass_kg: 0.05,
        bullet_sharpness: 0.8,
    }
}

/// Drives `RoundKind::Heat` rounds through the M14C HEAT producer when
/// the magazine is popped.
pub const RPG_LAUNCHER_V1_RIFLE_ID: &str = "rpg_launcher_v1";

/// rifle preset. Drives `RoundKind::Apfsds` rounds through the M14C APFSDS
/// producer when the magazine is popped.
pub const TANK_AUTOCANNON_T3_RIFLE_ID: &str = "tank_autocannon_t3";

/// magnitudes: 1-round magazine, 4.5 s reload, 320 m/s muzzle velocity. The
/// `primary_round = Heat` field ensures the magazine pops a HEAT round on
/// every fire so the cfctl drive of `m14c_heat_vs_era.ron` actually exercises
/// the M14C HEAT producer per the runtime-evidence layer.
fn rpg_launcher_v1_rifle() -> RifleSpec {
    RifleSpec {
        preset_id: RPG_LAUNCHER_V1_RIFLE_ID.to_string(),
        fire_interval_seconds: 1.0,
        mag_capacity: 1,
        reload_seconds: 4.5,
        recoil_impulse: 80.0,
        muzzle_forward_offset: 18.0,
        muzzle_vertical_offset: 6.0,
        projectile_speed: 320.0,
        damage_per_hit: 80.0,
        projectile_lifetime_seconds: 3.0,
        recoil_decay_rate: default_recoil_decay_rate(),
        loudness: 1.8,
        inherits_firer_velocity: true,
        particle_count: 1,
        spread_radians: 0.0,
        tracer_round_to_total_ratio: 0,
        ai_fire_vel: 320.0,
        ai_penetration: 0.0,
        ai_life_time: 3.0,
        ai_blast_radius: 0.0,
        fire_mode: FireMode::Semi,
        primary_round: RoundKind::Heat,
        // HEAT shaped-charge warhead: ~10 kg total mass with explosive
        // filler. Lower sharpness than KE rounds (0.7) because the
        // penetration mechanism is the molten jet, not raw kinetic
        // impulse — but the high mass + high muzzle still punches
        // through walls.
        bullet_mass_kg: 10.0,
        bullet_sharpness: 0.7,
    }
}

/// `tank_autocannon_t3.ron` magnitudes: 15-round mag, 2.5 s reload, 1600 m/s
/// muzzle velocity (DU long-rod). The `primary_round = Apfsds` field ensures
/// every popped round routes to the M14C APFSDS producer.
fn tank_autocannon_t3_rifle() -> RifleSpec {
    RifleSpec {
        preset_id: TANK_AUTOCANNON_T3_RIFLE_ID.to_string(),
        fire_interval_seconds: 0.2,
        mag_capacity: 15,
        reload_seconds: 2.5,
        recoil_impulse: 80.0,
        muzzle_forward_offset: 22.0,
        muzzle_vertical_offset: 8.0,
        projectile_speed: 1600.0,
        damage_per_hit: 40.0,
        projectile_lifetime_seconds: 2.0,
        recoil_decay_rate: default_recoil_decay_rate(),
        loudness: 1.6,
        inherits_firer_velocity: true,
        particle_count: 1,
        spread_radians: 0.0,
        tracer_round_to_total_ratio: 0,
        ai_fire_vel: 1600.0,
        ai_penetration: 0.0,
        ai_life_time: 2.0,
        ai_blast_radius: 0.0,
        fire_mode: FireMode::Semi,
        primary_round: RoundKind::Apfsds,
        // APFSDS depleted-uranium long-rod penetrator: ~8 kg with a
        // hardened tip (sharpness 0.98). The KE × sharpness² product
        // is what gives APFSDS its through-armor capability.
        bullet_mass_kg: 8.0,
        bullet_sharpness: 0.98,
    }
}

/// All known presets. Keyed by `preset_id` for scenario lookup.
#[must_use]
pub fn rifle_presets() -> BTreeMap<&'static str, RifleSpec> {
    let mut m = BTreeMap::new();
    m.insert(RIFLE_M1_DEFAULT_ID, rifle_m1_default());
    m.insert(CARBINE_M5_POWERED_ID, carbine_m5_powered());
    m.insert(RIFLE_M5_MECH_HEAVY_ID, rifle_m5_mech_heavy());
    m.insert(SHOTGUN_M1_DEFAULT_ID, shotgun_m1_default());
    m.insert(RIFLE_M1_TRACER_ID, rifle_m1_tracer());
    m.insert(RPG_LAUNCHER_V1_RIFLE_ID, rpg_launcher_v1_rifle());
    m.insert(TANK_AUTOCANNON_T3_RIFLE_ID, tank_autocannon_t3_rifle());
    m
}

/// Stable role-record registry. Every rifle preset is also a role record so
/// chassis sockets + AI doctrine + HUD inspect can speak in role-record terms.
#[must_use]
pub fn role_records() -> BTreeMap<&'static str, RoleRecord> {
    let mut m = BTreeMap::new();
    m.insert(
        RIFLE_M1_DEFAULT_ID,
        RoleRecord::from_rifle_spec(
            &rifle_m1_default(),
            RoleKind::Rifle,
            AiPolicyHint::Primary,
            "Service Rifle",
            "spec/equipment-loadout#LOAD-A.rifle_m1_default",
            0.0,
            3.5,
        ),
    );
    m.insert(
        CARBINE_M5_POWERED_ID,
        RoleRecord::from_rifle_spec(
            &carbine_m5_powered(),
            RoleKind::Carbine,
            AiPolicyHint::Primary,
            "Powered Carbine",
            "spec/equipment-loadout#LOAD-A.carbine_m5_powered",
            0.005,
            4.2,
        ),
    );
    m.insert(
        RIFLE_M5_MECH_HEAVY_ID,
        RoleRecord::from_rifle_spec(
            &rifle_m5_mech_heavy(),
            RoleKind::HeavyWeapon,
            AiPolicyHint::Primary,
            "Mech Autocannon",
            "spec/equipment-loadout#LOAD-A.rifle_m5_mech_heavy",
            0.015,
            48.0,
        ),
    );
    m
}

#[must_use]
pub fn role_record(role_id: &str) -> Option<RoleRecord> {
    role_records().get(role_id).cloned()
}

/// Stable loadout registry (LOAD-A fixtures). Used by scenarios to spawn an
/// actor with a typed loadout.
#[must_use]
pub fn loadouts() -> BTreeMap<&'static str, Loadout> {
    let mut m = BTreeMap::new();
    m.insert(
        "load_a_infantry",
        Loadout {
            id: "load_a_infantry".to_string(),
            display_name: "Infantry Standard".to_string(),
            role_ids: vec![RIFLE_M1_DEFAULT_ID.to_string()],
            provenance: "spec/equipment-loadout#LOAD-A.infantry".to_string(),
        },
    );
    m.insert(
        "load_a_powered_armor",
        Loadout {
            id: "load_a_powered_armor".to_string(),
            display_name: "Powered Armor Combat".to_string(),
            role_ids: vec![CARBINE_M5_POWERED_ID.to_string()],
            provenance: "spec/equipment-loadout#LOAD-A.powered_armor".to_string(),
        },
    );
    m.insert(
        "load_a_light_mech",
        Loadout {
            id: "load_a_light_mech".to_string(),
            display_name: "Light Mech Strike".to_string(),
            role_ids: vec![RIFLE_M5_MECH_HEAVY_ID.to_string()],
            provenance: "spec/equipment-loadout#LOAD-A.light_mech".to_string(),
        },
    );
    m
}

#[must_use]
pub fn loadout(loadout_id: &str) -> Option<Loadout> {
    loadouts().get(loadout_id).cloned()
}

/// Look up a preset by id; returns `None` if unknown so the engine can reject the
/// scenario before tick 0.
#[must_use]
pub fn rifle_preset(preset_id: &str) -> Option<RifleSpec> {
    rifle_presets().get(preset_id).cloned()
}

/// weapon preset id. M1 rifle presets default to the full Single / Burst3 /
/// Auto ladder per spec § "Weapons" (M1 rifle table row "Single / Burst-3 /
/// Auto"). M6 launch-weapon presets surface their declared
/// [`crate::weapon::WeaponPreset::available_modes`]. Unknown presets fall back to
/// `[Single]` so `act.player.cycle_fire_mode` is always well-defined for any
/// equipped weapon — the engine never panics on an unknown preset.
#[must_use]
pub fn available_fire_modes_for(preset_id: &str) -> Vec<AdvancedFireMode> {
    match preset_id {
        RIFLE_M1_DEFAULT_ID | RIFLE_M1_TRACER_ID | CARBINE_M5_POWERED_ID | RIFLE_M5_MECH_HEAVY_ID => vec![
            AdvancedFireMode::Single,
            AdvancedFireMode::Burst3,
            AdvancedFireMode::Auto,
        ],
        SHOTGUN_M1_DEFAULT_ID => vec![AdvancedFireMode::Single, AdvancedFireMode::Pump],
        _ => {
            if let Some(preset) = m6_weapon_presets().into_iter().find(|p| p.id == preset_id) {
                preset.available_modes
            } else {
                vec![AdvancedFireMode::Single]
            }
        }
    }
}
