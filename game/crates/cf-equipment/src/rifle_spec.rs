//! M1 [`RifleSpec`] preset shape + serde defaults + per-tick-rate conversions.

use serde::{Deserialize, Serialize};

use crate::fire_mode::{default_fire_mode, FireMode};
use crate::magazine::RoundKind;

/// Stable id for the M1 default rifle preset. Use [`crate::rifle_preset`] to materialize the
/// owned [`RifleSpec`].
pub const RIFLE_M1_DEFAULT_ID: &str = "rifle_m1_default";
/// Stable id for the M5 heavy mech rifle preset (slower, more damage). Used by
/// the LightMech chassis reference loadout.
pub const RIFLE_M5_MECH_HEAVY_ID: &str = "rifle_m5_mech_heavy";
/// Stable id for the M5 powered-armor combat carbine preset (faster, lower damage).
/// Used by the PoweredArmor chassis reference loadout.
pub const CARBINE_M5_POWERED_ID: &str = "carbine_m5_powered";

/// Spec for one rifle preset. Loaded from a hard-coded registry in M1; M5 introduces
/// the full role-record schema (`cf-equipment::RoleRecord`) and a `content/equipment/`
/// data path.
///
/// Timings are stored in seconds, NOT ticks, so the same preset behaves identically
/// at 60 Hz and 120 Hz. Use [`RifleSpec::fire_interval_ticks`] etc. to derive tick
/// counts for the configured `tick_rate_hz`. This honours the AGENTS.md
/// "No-Compromise Performance Defaults" rule (no hardcoded 60 Hz constants).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RifleSpec {
    pub preset_id: String,
    /// Seconds between consecutive shots. `0.1` = 10 RPS.
    pub fire_interval_seconds: f32,
    pub mag_capacity: u32,
    /// Seconds the actor spends reloading. `1.5` = 1.5 s.
    pub reload_seconds: f32,
    /// Horizontal recoil impulse applied to the firer's velocity_x (units / s).
    pub recoil_impulse: f32,
    /// Distance forward of the actor centre to spawn the projectile (world units).
    pub muzzle_forward_offset: f32,
    /// Vertical offset above the actor centre (world units; positive = up).
    pub muzzle_vertical_offset: f32,
    /// Speed of the projectile (world units / s). Pure horizontal in M1; M5 wires aim.
    pub projectile_speed: f32,
    /// Damage applied to the first hit body (M1 keeps damage instantaneous; M5 routes
    /// through the chassis grammar).
    pub damage_per_hit: f32,
    /// Seconds of flight time before the projectile expires if it never hits.
    pub projectile_lifetime_seconds: f32,
    /// Default 0.05 = subtract 0.05 toward zero per tick.
    #[serde(default = "default_recoil_decay_rate")]
    pub recoil_decay_rate: f32,
    /// alarm radius). 1.0 = baseline; higher = louder. CCCP `HDFirearm.cpp:948`.
    #[serde(default = "default_loudness_scalar")]
    pub loudness: f32,
    /// firer's velocity (running-and-gunning shots arc). When false (mortar-
    /// style), only the muzzle velocity is used. Default true.
    #[serde(default = "default_inherits_firer_velocity")]
    pub inherits_firer_velocity: bool,
    /// `Round.ParticleCount`). 1 = single round; >1 = shotgun-style spread.
    /// M1 ships a single rifle (=1); the field is data so future presets can
    /// describe pellet weapons without code changes.
    #[serde(default = "default_particle_count")]
    pub particle_count: u32,
    /// (CCCP `Round.Spread`). 0 = no spread; ~0.15 ≈ ±9° pellet cone.
    #[serde(default)]
    pub spread_radians: f32,
    /// projectiles uses the tracer visual preset. 0 = no tracers. M1's
    /// default rifle ships without tracers (=0).
    #[serde(default)]
    pub tracer_round_to_total_ratio: u32,
    /// to `projectile_speed` so AI threat estimation matches the live shot.
    #[serde(default)]
    pub ai_fire_vel: f32,
    /// future presets fill in mass * sharpness * fire_vel.
    #[serde(default)]
    pub ai_penetration: f32,
    /// first particle's lifetime (= `projectile_lifetime_seconds`).
    #[serde(default)]
    pub ai_life_time: f32,
    /// grenade / rocket presets set this for AI avoidance.
    #[serde(default)]
    pub ai_blast_radius: f32,
    /// canonical rifle keeps its single-press semantics. New presets opt in to
    /// FullAuto by setting this field.
    #[serde(default = "default_fire_mode")]
    pub fire_mode: FireMode,
    /// per-shot override (`ControlIntent::ammo_kind`) is provided. Defaults
    /// to [`RoundKind::Regular`] so every pre-M14C preset preserves byte-
    /// identical behavior. The M14C tank-grade presets (`rpg_launcher_v1`,
    /// `tank_autocannon_t3`) set this to `Heat` / `Apfsds` respectively so
    /// the cfctl drive of `m14c_heat_vs_era.ron` / `m14c_apfsds_vs_heavy.ron`
    /// actually fires the tank-grade round per the runtime-evidence layer
    /// required by VAL-M14C-007/008/009/010/011/012/019/020/023/026.
    #[serde(default = "default_primary_round")]
    pub primary_round: RoundKind,

    /// **Per-projectile mass** (kg). Drives the
    /// `cf-physics::try_penetrate` impulse formula
    /// `impulse² = (mass × velocity × sharpness)²` (CCCP
    /// `SceneMan::TryPenetrate:571`). Default 0.05 kg = 50 g rifle bullet
    /// baseline so legacy presets behave identically to the pre-spec-
    /// extension hardcoded value. Tank-grade rounds override (APFSDS
    /// long-rod ~7-9 kg; HEAT shaped-charge ~10-15 kg; 5.56 NATO ~4 g
    /// = 0.004 kg).
    #[serde(default = "default_bullet_mass_kg")]
    pub bullet_mass_kg: f32,

    /// **Per-projectile sharpness** in [0, 1] — the penetration-formula
    /// multiplier capturing aerodynamic shape + hardened tip. Default
    /// 0.8 = ogive-tipped rifle round baseline (preserves the legacy
    /// hardcoded value). APFSDS long-rod ≈ 0.98 (depleted-uranium dart);
    /// HEAT ≈ 0.7 (shaped-charge jet); blunt slug ≈ 0.4; rubber bullet
    /// ≈ 0.2.
    #[serde(default = "default_bullet_sharpness")]
    pub bullet_sharpness: f32,
}

pub(crate) fn default_recoil_decay_rate() -> f32 {
    0.05
}

pub(crate) fn default_loudness_scalar() -> f32 {
    1.0
}

pub(crate) fn default_inherits_firer_velocity() -> bool {
    true
}

pub(crate) fn default_particle_count() -> u32 {
    1
}

pub(crate) fn default_primary_round() -> RoundKind {
    RoundKind::Regular
}

/// **No-compromise default**: 50 g rifle bullet baseline (= 0.05 kg).
/// Preserves byte-identical behavior with pre-extension RifleSpec presets
/// per the per-projectile penetration formula.
pub(crate) fn default_bullet_mass_kg() -> f32 {
    0.05
}

/// **No-compromise default**: 0.8 sharpness for an ogive-tipped rifle
/// round. Preserves byte-identical behavior with the pre-extension
/// engine.rs hardcoded constant.
pub(crate) fn default_bullet_sharpness() -> f32 {
    0.8
}

impl RifleSpec {
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    fn seconds_to_ticks(seconds: f32, tick_rate_hz: u32) -> u32 {
        let rate = tick_rate_hz.max(1);
        let ticks = (f64::from(seconds.max(0.0)) * f64::from(rate)).round();
        if ticks < 1.0 {
            1
        } else if ticks > f64::from(u32::MAX) {
            u32::MAX
        } else {
            ticks as u32
        }
    }

    /// Ticks between consecutive shots at the given tick rate. Always ≥ 1.
    pub fn fire_interval_ticks(&self, tick_rate_hz: u32) -> u32 {
        Self::seconds_to_ticks(self.fire_interval_seconds, tick_rate_hz)
    }

    /// Ticks for one full reload at the given tick rate. Always ≥ 1.
    pub fn reload_ticks(&self, tick_rate_hz: u32) -> u32 {
        Self::seconds_to_ticks(self.reload_seconds, tick_rate_hz)
    }

    /// Maximum projectile flight ticks at the given tick rate. Always ≥ 1.
    pub fn projectile_max_flight_ticks(&self, tick_rate_hz: u32) -> u32 {
        Self::seconds_to_ticks(self.projectile_lifetime_seconds, tick_rate_hz)
    }
}
