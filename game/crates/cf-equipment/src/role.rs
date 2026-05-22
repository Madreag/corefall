//! M5 role-record schema: kind, AI hint, origin compat, [`RoleRecord`], and the
//! shared [`FiringProfile`] carried by ranged roles.

use serde::{Deserialize, Serialize};

use crate::fire_mode::default_fire_mode;
use crate::rifle_spec::{
    default_bullet_mass_kg, default_bullet_sharpness, default_inherits_firer_velocity,
    default_loudness_scalar, default_particle_count, default_primary_round,
    default_recoil_decay_rate, RifleSpec,
};

/// Role kind for the M5 role-record schema. Every equippable item is one of
/// these top-level kinds; modders can add new kinds via the `Other` opaque
/// payload after BP8 modding lands.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoleKind {
    Rifle,
    Carbine,
    Sidearm,
    HeavyWeapon,
    MeleeTool,
    Grenade,
    Medkit,
    RepairKit,
    Shield,
    SensorPack,
    UtilityModule,
}

impl RoleKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            RoleKind::Rifle => "rifle",
            RoleKind::Carbine => "carbine",
            RoleKind::Sidearm => "sidearm",
            RoleKind::HeavyWeapon => "heavy_weapon",
            RoleKind::MeleeTool => "melee_tool",
            RoleKind::Grenade => "grenade",
            RoleKind::Medkit => "medkit",
            RoleKind::RepairKit => "repair_kit",
            RoleKind::Shield => "shield",
            RoleKind::SensorPack => "sensor_pack",
            RoleKind::UtilityModule => "utility_module",
        }
    }
}

/// AI policy hint declared by the role record. Lets the AI doctrine choose
/// equipment for the right role without per-item special-casing. See
/// `spec/equipment-loadout` (the M5 role-record fixture).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiPolicyHint {
    /// Default firing role (rifle/carbine).
    Primary,
    /// Backup / close-range role.
    Sidearm,
    /// Area-effect / breaching role.
    AreaDenial,
    /// Sensor / scouting / spotter role.
    Recon,
    /// Healing/repair role.
    Support,
    /// Defensive role (shield / mobility).
    Defense,
}

impl AiPolicyHint {
    pub fn as_str(self) -> &'static str {
        match self {
            AiPolicyHint::Primary => "primary",
            AiPolicyHint::Sidearm => "sidearm",
            AiPolicyHint::AreaDenial => "area_denial",
            AiPolicyHint::Recon => "recon",
            AiPolicyHint::Support => "support",
            AiPolicyHint::Defense => "defense",
        }
    }
}

/// Compatibility tag for origin-gated equipment (DR-014 / M5.8). M5 ships a
/// permissive default (compatible with every origin); M5.8 layers per-origin
/// rejection on top.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OriginCompatibility {
    Universal,
    HumanOnly,
    RobotOnly,
    AndroidOnly,
    BiologicalOnly,
}

impl OriginCompatibility {
    pub fn as_str(self) -> &'static str {
        match self {
            OriginCompatibility::Universal => "universal",
            OriginCompatibility::HumanOnly => "human_only",
            OriginCompatibility::RobotOnly => "robot_only",
            OriginCompatibility::AndroidOnly => "android_only",
            OriginCompatibility::BiologicalOnly => "biological_only",
        }
    }
}

/// Drives chassis socket binding, AI doctrine, HUD inspect, modding, and replay.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoleRecord {
    /// Stable id (`rifle_m1_default`, `carbine_m5_powered`, etc.).
    pub id: String,
    /// Localized display label (English-only at M5; localized at BP12).
    pub display_name: String,
    /// Top-level role kind.
    pub kind: RoleKind,
    /// AI doctrine hint.
    pub ai_policy_hint: AiPolicyHint,
    /// Per-origin compatibility tag (DR-014 / M5.8 hook).
    pub origin_compatibility: OriginCompatibility,
    /// Probability per shot that the weapon jams (0..1). 0 = never jams; 0.01 = 1%.
    pub jam_chance_per_shot: f32,
    /// Probability per shot that a jam clears on its own (0..1). 0 = manual clear required.
    pub jam_clear_chance_per_shot: f32,
    /// Optional rifle-style firing data. `None` for melee/medkit/shield roles.
    pub firing: Option<FiringProfile>,
    /// Mass in kg (drives M5.5 impulse-to-damage routing).
    pub mass_kg: f32,
    /// Provenance string (`spec/equipment-loadout` slice id or mod author).
    pub provenance: String,
    /// Tutorial-safety toggle: when true, weapon may be issued in tutorials.
    pub tutorial_safe: bool,
}

impl RoleRecord {
    /// Build a role record from a [`RifleSpec`] preset + supplemental metadata.
    pub fn from_rifle_spec(
        spec: &RifleSpec,
        kind: RoleKind,
        ai_policy_hint: AiPolicyHint,
        display_name: &str,
        provenance: &str,
        jam_chance_per_shot: f32,
        mass_kg: f32,
    ) -> Self {
        let firing = FiringProfile {
            fire_interval_seconds: spec.fire_interval_seconds,
            mag_capacity: spec.mag_capacity,
            reload_seconds: spec.reload_seconds,
            recoil_impulse: spec.recoil_impulse,
            muzzle_forward_offset: spec.muzzle_forward_offset,
            muzzle_vertical_offset: spec.muzzle_vertical_offset,
            projectile_speed: spec.projectile_speed,
            damage_per_hit: spec.damage_per_hit,
            projectile_lifetime_seconds: spec.projectile_lifetime_seconds,
            bullet_mass_kg: spec.bullet_mass_kg,
            bullet_sharpness: spec.bullet_sharpness,
        };
        Self {
            id: spec.preset_id.clone(),
            display_name: display_name.to_string(),
            kind,
            ai_policy_hint,
            origin_compatibility: OriginCompatibility::Universal,
            jam_chance_per_shot: jam_chance_per_shot.clamp(0.0, 1.0),
            jam_clear_chance_per_shot: 0.0,
            firing: Some(firing),
            mass_kg: mass_kg.max(0.0),
            provenance: provenance.to_string(),
            tutorial_safe: true,
        }
    }

    /// Returns true when the role can be mounted by an actor of the given origin tag.
    pub fn compatible_with_origin(&self, origin_id: &str) -> bool {
        match self.origin_compatibility {
            OriginCompatibility::Universal => true,
            OriginCompatibility::HumanOnly => origin_id == "human",
            OriginCompatibility::RobotOnly => origin_id == "robot",
            OriginCompatibility::AndroidOnly => origin_id == "android",
            OriginCompatibility::BiologicalOnly => matches!(origin_id, "human" | "biological"),
        }
    }
}

/// Per-shot firing profile carried by ranged role records. Mirrors [`RifleSpec`]
/// for backward compat — the rifle preset registry now delegates to this.
///
/// projectile-vs-terrain penetration formula configurable per-RON.
/// Tank-grade rounds (HEAT shaped-charge, APFSDS long-rod) override
/// the rifle baseline so heavy weapons punch through walls per the
/// real CCCP `SceneMan::TryPenetrate:571` formula
/// `impulse² = (mass × velocity × sharpness)²`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FiringProfile {
    pub fire_interval_seconds: f32,
    pub mag_capacity: u32,
    pub reload_seconds: f32,
    pub recoil_impulse: f32,
    pub muzzle_forward_offset: f32,
    pub muzzle_vertical_offset: f32,
    pub projectile_speed: f32,
    pub damage_per_hit: f32,
    pub projectile_lifetime_seconds: f32,
    /// **Per-projectile mass** (kg). Optional — defaults to 0.05 kg
    /// (50 g rifle baseline) when omitted. Drives the penetration
    /// formula. Tank rounds set this to ~8 kg (APFSDS) or ~10 kg
    /// (HEAT) to override the default.
    #[serde(default = "default_bullet_mass_kg")]
    pub bullet_mass_kg: f32,
    /// **Per-projectile sharpness** in [0, 1]. Optional — defaults to
    /// 0.8 (ogive-tipped rifle round). APFSDS long-rod ≈ 0.98; HEAT
    /// ≈ 0.7; blunt slug ≈ 0.4.
    #[serde(default = "default_bullet_sharpness")]
    pub bullet_sharpness: f32,
}

impl FiringProfile {
    pub fn into_rifle_spec(self, preset_id: String) -> RifleSpec {
        RifleSpec {
            preset_id,
            fire_interval_seconds: self.fire_interval_seconds,
            mag_capacity: self.mag_capacity,
            reload_seconds: self.reload_seconds,
            recoil_impulse: self.recoil_impulse,
            muzzle_forward_offset: self.muzzle_forward_offset,
            muzzle_vertical_offset: self.muzzle_vertical_offset,
            projectile_speed: self.projectile_speed,
            damage_per_hit: self.damage_per_hit,
            projectile_lifetime_seconds: self.projectile_lifetime_seconds,
            recoil_decay_rate: default_recoil_decay_rate(),
            loudness: default_loudness_scalar(),
            inherits_firer_velocity: default_inherits_firer_velocity(),
            particle_count: default_particle_count(),
            spread_radians: 0.0,
            tracer_round_to_total_ratio: 0,
            ai_fire_vel: self.projectile_speed,
            ai_penetration: 0.0,
            ai_life_time: self.projectile_lifetime_seconds,
            ai_blast_radius: 0.0,
            fire_mode: default_fire_mode(),
            primary_round: default_primary_round(),
            bullet_mass_kg: self.bullet_mass_kg,
            bullet_sharpness: self.bullet_sharpness,
        }
    }
}
