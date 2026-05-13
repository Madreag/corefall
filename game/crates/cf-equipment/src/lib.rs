//! M1+M5: weapon presets, per-actor weapon state, and the M5 role-record schema.
//!
//! - M1 owns the [`RifleSpec`] preset + [`RifleState`] state machine. The engine
//!   ticks one rifle per actor each fixed step; the state machine emits structured
//!   outcomes (`fired`, `reloaded`, `dry_fire`) that the caller turns into
//!   `weapon.*` events.
//! - **M5** lands the full **role-record** schema ([`RoleRecord`]) + the [`Loadout`]
//!   registry + AI policy hints + jam-chance / origin-compatibility metadata.
//!   The M1 rifle still works as before; under the hood every rifle preset is now
//!   ALSO exposed through [`role_record`]/[`loadouts()`] so chassis sockets, AI
//!   doctrine, and modding tools all see the same role-record contract.
//!
//! The M5 contract is the **minimum bar** per AGENTS.md: a `RoleRecord` carries
//! every field the chassis grammar (cf-chassis), AI (cf-ai), and HUD/inspect
//! (cfctl) need to reason about a piece of equipment without a screenshot.

#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(
    clippy::module_name_repetitions,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::doc_markdown,
    clippy::struct_excessive_bools,
    clippy::derivable_impls,
    clippy::missing_const_for_fn
)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

// M1 re-audit (2026-05-13): spec lists `cf-equipment/src/projectile.rs` as a
// separate file. The helper lives there now; re-exported here for ergonomics.
pub mod projectile;
pub use projectile::ProjectileSpawnParams;

/// **M1**: how the weapon's fire button is consumed.
///
/// - `Semi`: exactly one shot per `intent.fire` press (the press is latched in
///   `RifleState::semi_latched` and released only when the player releases
///   the trigger). Holding fire fires once, then waits.
/// - `FullAuto`: as long as `intent.fire` is held the rifle fires at
///   `fire_interval_seconds` cadence.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FireMode {
    Semi = 0,
    FullAuto = 1,
}

impl Default for FireMode {
    fn default() -> Self {
        // M1's default rifle is semi-automatic per CCCP `HDFirearm` defaults.
        FireMode::Semi
    }
}

impl FireMode {
    pub fn as_str(self) -> &'static str {
        match self {
            FireMode::Semi => "semi",
            FireMode::FullAuto => "full_auto",
        }
    }
}

fn default_fire_mode() -> FireMode {
    FireMode::Semi
}

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
    /// **M1**: per-tick decay of the recoil_accumulator (CCCP `HDFirearm.cpp:891`).
    /// Default 0.05 = subtract 0.05 toward zero per tick.
    #[serde(default = "default_recoil_decay_rate")]
    pub recoil_decay_rate: f32,
    /// **M1**: per-shot loudness radius scalar (multiplied with the damage-scaled
    /// alarm radius). 1.0 = baseline; higher = louder. CCCP `HDFirearm.cpp:948`.
    #[serde(default = "default_loudness_scalar")]
    pub loudness: f32,
    /// **M1**: when true, the spawned projectile inherits a fraction of the
    /// firer's velocity (running-and-gunning shots arc). When false (mortar-
    /// style), only the muzzle velocity is used. Default true.
    #[serde(default = "default_inherits_firer_velocity")]
    pub inherits_firer_velocity: bool,
    /// **M1**: number of projectile particles spawned per shot (CCCP
    /// `Round.ParticleCount`). 1 = single round; >1 = shotgun-style spread.
    /// M1 ships a single rifle (=1); the field is data so future presets can
    /// describe pellet weapons without code changes.
    #[serde(default = "default_particle_count")]
    pub particle_count: u32,
    /// **M1**: cone spread in radians applied to multi-particle shots
    /// (CCCP `Round.Spread`). 0 = no spread; ~0.15 ≈ ±9° pellet cone.
    #[serde(default)]
    pub spread_radians: f32,
    /// **M1**: tracer round cadence (CCCP `Magazine.RTTRatio`). 1 in N
    /// projectiles uses the tracer visual preset. 0 = no tracers. M1's
    /// default rifle ships without tracers (=0).
    #[serde(default)]
    pub tracer_round_to_total_ratio: u32,
    /// **M1 AI**: ai_fire_vel hint (CCCP `Round::Create` AI default). Defaults
    /// to `projectile_speed` so AI threat estimation matches the live shot.
    #[serde(default)]
    pub ai_fire_vel: f32,
    /// **M1 AI**: ai_penetration hint (CCCP `Round::Create`). Defaults to 0;
    /// future presets fill in mass * sharpness * fire_vel.
    #[serde(default)]
    pub ai_penetration: f32,
    /// **M1 AI**: ai_life_time hint (CCCP `Round::Create`). Defaults to the
    /// first particle's lifetime (= `projectile_lifetime_seconds`).
    #[serde(default)]
    pub ai_life_time: f32,
    /// **M1 AI**: ai_blast_radius hint. 0 for non-explosive rifles; future
    /// grenade / rocket presets set this for AI avoidance.
    #[serde(default)]
    pub ai_blast_radius: f32,
    /// **M1**: fire-mode discriminator (Semi or FullAuto). Default Semi so M1's
    /// canonical rifle keeps its single-press semantics. New presets opt in to
    /// FullAuto by setting this field.
    #[serde(default = "default_fire_mode")]
    pub fire_mode: FireMode,
}

fn default_recoil_decay_rate() -> f32 {
    0.05
}

fn default_loudness_scalar() -> f32 {
    1.0
}

fn default_inherits_firer_velocity() -> bool {
    true
}

fn default_particle_count() -> u32 {
    1
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

/// Stable id for the M1 default rifle preset. Use [`rifle_preset`] to materialize the
/// owned [`RifleSpec`].
pub const RIFLE_M1_DEFAULT_ID: &str = "rifle_m1_default";
/// Stable id for the M5 heavy mech rifle preset (slower, more damage). Used by
/// the LightMech chassis reference loadout.
pub const RIFLE_M5_MECH_HEAVY_ID: &str = "rifle_m5_mech_heavy";
/// Stable id for the M5 powered-armor combat carbine preset (faster, lower damage).
/// Used by the PoweredArmor chassis reference loadout.
pub const CARBINE_M5_POWERED_ID: &str = "carbine_m5_powered";

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

/// **M5 role-record**: the canonical data model for one piece of equipment.
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
        }
    }
}

/// A loadout is a named set of role records (e.g., "infantry default" = rifle +
/// medkit). M5 ships LOAD-A (Loadout A) fixture stubs; M8+ owns mod-loadable
/// loadouts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Loadout {
    pub id: String,
    pub display_name: String,
    /// Ordered list of role-record ids. The first entry is treated as the
    /// primary weapon by AI doctrine.
    pub role_ids: Vec<String>,
    pub provenance: String,
}

fn rifle_m1_default() -> RifleSpec {
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
    }
}

/// **M1**: shotgun preset. Multi-particle round with cone spread.
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
    }
}

/// **M1**: tracer-rich preset used to verify the 1-in-N tracer cadence
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

/// Per-actor rifle state machine. Carries the configured `tick_rate_hz` so timings
/// derived from `RifleSpec` (in seconds) resolve to a stable tick budget at both
/// 60 Hz and 120 Hz simulations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RifleState {
    pub spec: RifleSpec,
    /// Tick rate the engine ticks this rifle at; used to convert `spec.*_seconds`
    /// to tick counts. Always ≥ 1 (clamped at construction).
    pub tick_rate_hz: u32,
    pub ammo_in_mag: u32,
    /// Ticks until the rifle can fire again. 0 = ready.
    pub fire_cooldown_ticks: u32,
    /// Ticks remaining in an in-progress reload. 0 = idle.
    pub reload_remaining_ticks: u32,
    /// **M1 / Semi**: latched after a Semi-mode shot until the trigger is
    /// released. Prevents the next held-tick from re-firing. Cleared when
    /// `RifleTickInputs::fire_pressed=false`.
    #[serde(default)]
    pub semi_latched: bool,
    /// **M1**: per-mag shot index, starting at 0 on reload. Drives the
    /// tracer cadence: shot index N produces a tracer when
    /// `(N % tracer_round_to_total_ratio) == (tracer_round_to_total_ratio - 1)`
    /// for non-zero ratios (so the LAST shot in each cycle is the tracer, per
    /// CCCP Magazine semantics). Reset to 0 by `reset()` and on reload completion.
    #[serde(default)]
    pub shot_index_in_mag: u32,
}

impl RifleState {
    pub fn new(spec: RifleSpec, tick_rate_hz: u32) -> Self {
        Self {
            ammo_in_mag: spec.mag_capacity,
            fire_cooldown_ticks: 0,
            reload_remaining_ticks: 0,
            tick_rate_hz: tick_rate_hz.max(1),
            spec,
            semi_latched: false,
            shot_index_in_mag: 0,
        }
    }

    /// Cool-down after a shot, in ticks at this rifle's configured tick rate.
    pub fn fire_interval_ticks(&self) -> u32 {
        self.spec.fire_interval_ticks(self.tick_rate_hz)
    }

    /// Full reload duration in ticks at this rifle's configured tick rate.
    pub fn reload_ticks(&self) -> u32 {
        self.spec.reload_ticks(self.tick_rate_hz)
    }

    /// Maximum projectile flight in ticks at this rifle's configured tick rate.
    pub fn projectile_max_flight_ticks(&self) -> u32 {
        self.spec.projectile_max_flight_ticks(self.tick_rate_hz)
    }

    pub fn ready_to_fire(&self) -> bool {
        self.fire_cooldown_ticks == 0 && self.reload_remaining_ticks == 0 && self.ammo_in_mag > 0
    }

    pub fn is_reloading(&self) -> bool {
        self.reload_remaining_ticks > 0
    }

    /// Reset ammo + cooldowns. Used by `act.player.reset` and scenario reload.
    pub fn reset(&mut self) {
        self.ammo_in_mag = self.spec.mag_capacity;
        self.fire_cooldown_ticks = 0;
        self.reload_remaining_ticks = 0;
        self.semi_latched = false;
        self.shot_index_in_mag = 0;
    }

    /// **M1**: returns true if the next shot (index `shot_index_in_mag`) should
    /// emit a tracer projectile per `tracer_round_to_total_ratio`. Deterministic
    /// — same mag index always produces the same answer for the same ratio.
    pub fn next_shot_is_tracer(&self) -> bool {
        let ratio = self.spec.tracer_round_to_total_ratio;
        if ratio == 0 {
            return false;
        }
        // Tracer falls on every Nth shot starting at index `ratio - 1` so a
        // ratio of 4 produces tracers at shots 3, 7, 11, ... (one per group
        // of 4). Matches CCCP `Magazine::RTTRatio` cycling.
        (self.shot_index_in_mag + 1).is_multiple_of(ratio)
    }
}

/// Outcomes of one tick of the rifle state machine. Converted to recorder events by the
/// caller; the data needed for `weapon_fired`, `weapon_reloaded`, etc. is included here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TickOutcomes {
    pub fired_this_tick: bool,
    pub reload_started: bool,
    pub reload_completed: bool,
    pub dry_fire: bool,
    pub recoil_impulse_applied: f32,
    /// **M1**: tracer flag for the shot fired this tick (per CCCP `Magazine.RTTRatio`).
    /// Always false when no shot fired.
    #[serde(default)]
    pub fired_is_tracer: bool,
}

impl TickOutcomes {
    pub const fn empty() -> Self {
        Self {
            fired_this_tick: false,
            reload_started: false,
            reload_completed: false,
            dry_fire: false,
            recoil_impulse_applied: 0.0,
            fired_is_tracer: false,
        }
    }
}

/// Inputs for [`tick_rifle`]. `fire_pressed` and `reload_pressed` are edge-triggered
/// per `cf-actor::ControlIntent`; the caller clears them after the tick.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RifleTickInputs {
    pub fire_pressed: bool,
    pub reload_pressed: bool,
    pub auto_reload_when_empty: bool,
}

impl Default for RifleTickInputs {
    fn default() -> Self {
        Self {
            fire_pressed: false,
            reload_pressed: false,
            auto_reload_when_empty: false,
        }
    }
}

/// One fixed-tick step of the rifle. Returns the outcomes the caller should turn into
/// recorder events, plus any recoil impulse the caller should apply to the firer.
///
/// **M1**: honors `RifleSpec::fire_mode`. `Semi` latches after each shot until
/// the trigger is released so a held button fires exactly once. `FullAuto` fires
/// at `fire_interval_seconds` cadence while held.
#[must_use]
pub fn tick_rifle(state: &mut RifleState, inputs: RifleTickInputs) -> TickOutcomes {
    let mut outcomes = TickOutcomes::empty();

    // Release the semi-mode latch as soon as the trigger lifts; subsequent
    // presses are honored. Must run BEFORE the fire check so the very tick
    // the player releases doesn't get a free shot.
    if !inputs.fire_pressed {
        state.semi_latched = false;
    }

    // Advance reload counter first; finishing a reload this tick must take priority over
    // firing so the actor can shoot again on the very next tick.
    if state.reload_remaining_ticks > 0 {
        state.reload_remaining_ticks -= 1;
        if state.reload_remaining_ticks == 0 {
            state.ammo_in_mag = state.spec.mag_capacity;
            state.shot_index_in_mag = 0;
            outcomes.reload_completed = true;
            // Reload finished this tick; the fire check below would otherwise see a
            // zero cooldown and fire on the same tick. Defer firing to the next tick
            // to match the documented "shoot again on the very next tick" semantics.
            return outcomes;
        }
    } else if state.fire_cooldown_ticks > 0 {
        state.fire_cooldown_ticks -= 1;
    }

    // Reload requested (or auto-reload when the magazine just emptied).
    let want_reload =
        inputs.reload_pressed || (inputs.auto_reload_when_empty && state.ammo_in_mag == 0 && !state.is_reloading());
    if want_reload && !state.is_reloading() && state.ammo_in_mag < state.spec.mag_capacity {
        state.reload_remaining_ticks = state.reload_ticks();
        // Cancel the pending fire cooldown; reloading takes over.
        state.fire_cooldown_ticks = 0;
        outcomes.reload_started = true;
    }

    if inputs.fire_pressed && !state.is_reloading() {
        if state.ammo_in_mag == 0 {
            outcomes.dry_fire = true;
        } else if state.fire_cooldown_ticks == 0 {
            // Gate the actual fire on fire_mode + latch.
            let allow_fire = match state.spec.fire_mode {
                FireMode::FullAuto => true,
                FireMode::Semi => !state.semi_latched,
            };
            if allow_fire {
                outcomes.fired_is_tracer = state.next_shot_is_tracer();
                state.ammo_in_mag -= 1;
                state.shot_index_in_mag = state.shot_index_in_mag.saturating_add(1);
                state.fire_cooldown_ticks = state.fire_interval_ticks();
                outcomes.fired_this_tick = true;
                outcomes.recoil_impulse_applied = state.spec.recoil_impulse;
                if state.spec.fire_mode == FireMode::Semi {
                    state.semi_latched = true;
                }
            }
        }
    }

    outcomes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rifle() -> RifleState {
        rifle_at(60)
    }

    fn rifle_at(tick_rate_hz: u32) -> RifleState {
        RifleState::new(rifle_preset(RIFLE_M1_DEFAULT_ID).expect("default preset"), tick_rate_hz)
    }

    #[test]
    fn rifle_starts_loaded_and_ready() {
        let r = rifle();
        let spec = rifle_preset(RIFLE_M1_DEFAULT_ID).unwrap();
        assert!(r.ready_to_fire());
        assert_eq!(r.ammo_in_mag, spec.mag_capacity);
    }

    #[test]
    fn fire_decrements_ammo_and_starts_cooldown() {
        let mut r = rifle();
        let cooldown = r.fire_interval_ticks();
        let mag = r.spec.mag_capacity;
        let outcomes = tick_rifle(
            &mut r,
            RifleTickInputs {
                fire_pressed: true,
                ..Default::default()
            },
        );
        assert!(outcomes.fired_this_tick);
        assert_eq!(r.ammo_in_mag, mag - 1);
        assert_eq!(r.fire_cooldown_ticks, cooldown);
    }

    #[test]
    fn cannot_fire_during_cooldown() {
        let mut r = rifle();
        let mag = r.spec.mag_capacity;
        let _ = tick_rifle(
            &mut r,
            RifleTickInputs {
                fire_pressed: true,
                ..Default::default()
            },
        );
        let blocked = tick_rifle(
            &mut r,
            RifleTickInputs {
                fire_pressed: true,
                ..Default::default()
            },
        );
        assert!(!blocked.fired_this_tick);
        assert_eq!(r.ammo_in_mag, mag - 1);
    }

    #[test]
    fn dry_fire_when_empty() {
        let mut r = rifle();
        let mag = r.spec.mag_capacity;
        let cooldown = r.fire_interval_ticks();
        for _ in 0..mag {
            for _ in 0..cooldown {
                let _ = tick_rifle(
                    &mut r,
                    RifleTickInputs {
                        fire_pressed: false,
                        ..Default::default()
                    },
                );
            }
            let _ = tick_rifle(
                &mut r,
                RifleTickInputs {
                    fire_pressed: true,
                    ..Default::default()
                },
            );
        }
        assert_eq!(r.ammo_in_mag, 0);
        let outcomes = tick_rifle(
            &mut r,
            RifleTickInputs {
                fire_pressed: true,
                ..Default::default()
            },
        );
        assert!(outcomes.dry_fire);
        assert!(!outcomes.fired_this_tick);
    }

    #[test]
    fn reload_takes_full_duration() {
        let mut r = rifle();
        let mag = r.spec.mag_capacity;
        let reload = r.reload_ticks();
        let _ = tick_rifle(
            &mut r,
            RifleTickInputs {
                fire_pressed: true,
                ..Default::default()
            },
        );
        let started = tick_rifle(
            &mut r,
            RifleTickInputs {
                reload_pressed: true,
                ..Default::default()
            },
        );
        assert!(started.reload_started);
        for _ in 0..(reload - 1) {
            let _ = tick_rifle(&mut r, RifleTickInputs::default());
            assert!(r.is_reloading());
        }
        let completion = tick_rifle(&mut r, RifleTickInputs::default());
        assert!(completion.reload_completed);
        assert_eq!(r.ammo_in_mag, mag);
        assert!(!r.is_reloading());
    }

    #[test]
    fn auto_reload_when_empty_starts_after_dry_fire() {
        let mut r = rifle();
        r.ammo_in_mag = 0;
        let outcomes = tick_rifle(
            &mut r,
            RifleTickInputs {
                fire_pressed: false,
                reload_pressed: false,
                auto_reload_when_empty: true,
            },
        );
        assert!(outcomes.reload_started);
        assert!(r.is_reloading());
    }

    #[test]
    fn reset_returns_full_mag() {
        let spec = rifle_preset(RIFLE_M1_DEFAULT_ID).unwrap();
        let mut r = rifle();
        r.ammo_in_mag = 5;
        r.fire_cooldown_ticks = 3;
        r.reload_remaining_ticks = 30;
        r.reset();
        assert_eq!(r.ammo_in_mag, spec.mag_capacity);
        assert_eq!(r.fire_cooldown_ticks, 0);
        assert_eq!(r.reload_remaining_ticks, 0);
    }

    #[test]
    fn rifle_preset_lookup() {
        assert!(rifle_preset(RIFLE_M1_DEFAULT_ID).is_some());
        assert!(rifle_preset(CARBINE_M5_POWERED_ID).is_some());
        assert!(rifle_preset(RIFLE_M5_MECH_HEAVY_ID).is_some());
        assert!(rifle_preset("nonexistent").is_none());
    }

    #[test]
    fn role_record_registry_covers_every_rifle_preset() {
        for preset_id in [RIFLE_M1_DEFAULT_ID, CARBINE_M5_POWERED_ID, RIFLE_M5_MECH_HEAVY_ID] {
            let r = role_record(preset_id).unwrap_or_else(|| panic!("role record for {preset_id}"));
            assert_eq!(r.id, preset_id);
            assert!(r.firing.is_some(), "role {preset_id} must carry firing data");
            assert!(r.tutorial_safe, "M5 LOAD-A roles default to tutorial-safe");
        }
    }

    #[test]
    fn role_record_origin_compatibility_default_is_universal() {
        let r = role_record(RIFLE_M1_DEFAULT_ID).unwrap();
        assert!(r.compatible_with_origin("human"));
        assert!(r.compatible_with_origin("robot"));
        assert!(r.compatible_with_origin("android"));
    }

    #[test]
    fn loadout_registry_resolves_canonical_load_a_ids() {
        assert!(loadout("load_a_infantry").is_some());
        assert!(loadout("load_a_powered_armor").is_some());
        assert!(loadout("load_a_light_mech").is_some());
        assert!(loadout("missing").is_none());
    }

    #[test]
    fn rifle_spec_roundtrips_through_role_record() {
        let r = role_record(CARBINE_M5_POWERED_ID).unwrap();
        let firing = r.firing.clone().unwrap();
        let spec = firing.into_rifle_spec(r.id.clone());
        assert!((spec.fire_interval_seconds - 0.083).abs() < 1e-6);
        assert_eq!(spec.mag_capacity, 25);
    }

    #[test]
    fn jam_chance_clamped_to_unit_range() {
        let r = RoleRecord::from_rifle_spec(
            &rifle_m1_default(),
            RoleKind::Rifle,
            AiPolicyHint::Primary,
            "Test",
            "test",
            5.0,
            3.5,
        );
        assert!((r.jam_chance_per_shot - 1.0).abs() < 1e-6);
    }

    #[test]
    fn timings_scale_with_tick_rate() {
        // 10 RPS / 1.5 s reload / 1.5 s flight at the canonical M1 preset.
        let spec = rifle_preset(RIFLE_M1_DEFAULT_ID).unwrap();
        // 60 Hz: 6 / 90 / 90.
        assert_eq!(spec.fire_interval_ticks(60), 6);
        assert_eq!(spec.reload_ticks(60), 90);
        assert_eq!(spec.projectile_max_flight_ticks(60), 90);
        // 120 Hz: 12 / 180 / 180.
        assert_eq!(spec.fire_interval_ticks(120), 12);
        assert_eq!(spec.reload_ticks(120), 180);
        assert_eq!(spec.projectile_max_flight_ticks(120), 180);
        // RifleState resolves the same values via its configured tick_rate_hz.
        let r60 = rifle_at(60);
        let r120 = rifle_at(120);
        assert_eq!(r60.fire_interval_ticks(), 6);
        assert_eq!(r120.fire_interval_ticks(), 12);
        assert_eq!(r60.reload_ticks(), 90);
        assert_eq!(r120.reload_ticks(), 180);
    }

    #[test]
    fn semi_mode_fires_once_per_press_even_when_held() {
        let mut r = rifle();
        assert_eq!(r.spec.fire_mode, FireMode::Semi);
        // Press + hold for many ticks: must produce exactly one shot.
        let first = tick_rifle(
            &mut r,
            RifleTickInputs {
                fire_pressed: true,
                ..Default::default()
            },
        );
        assert!(first.fired_this_tick);
        let mut shots_while_held = 0;
        for _ in 0..60 {
            let outcomes = tick_rifle(
                &mut r,
                RifleTickInputs {
                    fire_pressed: true,
                    ..Default::default()
                },
            );
            if outcomes.fired_this_tick {
                shots_while_held += 1;
            }
        }
        assert_eq!(
            shots_while_held, 0,
            "Semi must NOT auto-repeat while held; got {shots_while_held} extra shots"
        );
        // Releasing + re-pressing fires again.
        let _release = tick_rifle(&mut r, RifleTickInputs::default());
        let second = tick_rifle(
            &mut r,
            RifleTickInputs {
                fire_pressed: true,
                ..Default::default()
            },
        );
        assert!(second.fired_this_tick, "Semi must fire on a fresh press after release");
    }

    #[test]
    fn full_auto_mode_fires_at_cadence_while_held() {
        let preset = rifle_preset(CARBINE_M5_POWERED_ID).unwrap();
        assert_eq!(preset.fire_mode, FireMode::FullAuto);
        let mut r = RifleState::new(preset, 60);
        let mut shots = 0;
        for _ in 0..120 {
            let outcomes = tick_rifle(
                &mut r,
                RifleTickInputs {
                    fire_pressed: true,
                    ..Default::default()
                },
            );
            if outcomes.fired_this_tick {
                shots += 1;
            }
        }
        // ~12 RPS × 2 s window = ~24 shots; clamp by mag (25). Should be > 1.
        assert!(shots > 1, "FullAuto must auto-repeat while held; only {shots} shot(s)");
    }

    #[test]
    fn tracer_cadence_one_in_four_yields_three_tracers_in_twelve_shots() {
        let preset = rifle_preset(RIFLE_M1_TRACER_ID).unwrap();
        assert_eq!(preset.tracer_round_to_total_ratio, 4);
        let mut r = RifleState::new(preset, 60);
        let mut tracer_count = 0;
        let mut shots = 0;
        let cooldown = r.fire_interval_ticks();
        while shots < 12 {
            // Release for one tick to clear the Semi latch (preset is Semi).
            let _ = tick_rifle(&mut r, RifleTickInputs::default());
            for _ in 0..cooldown {
                let _ = tick_rifle(&mut r, RifleTickInputs::default());
            }
            let outcomes = tick_rifle(
                &mut r,
                RifleTickInputs {
                    fire_pressed: true,
                    ..Default::default()
                },
            );
            if outcomes.fired_this_tick {
                if outcomes.fired_is_tracer {
                    tracer_count += 1;
                }
                shots += 1;
            }
        }
        assert_eq!(tracer_count, 3, "12 shots @ ratio 4 must yield exactly 3 tracers");
    }

    #[test]
    fn fire_rate_real_time_equivalent_at_60hz_and_120hz() {
        // Drive both 60 Hz and 120 Hz FullAuto rifles for the same wall-clock
        // window and assert the same number of shots fired. Uses the carbine
        // preset (FullAuto, ~12 RPS) since the default rifle is Semi.
        fn shots_in_window(tick_rate_hz: u32, ticks: u32) -> u32 {
            let preset = rifle_preset(CARBINE_M5_POWERED_ID).unwrap();
            let mut r = RifleState::new(preset, tick_rate_hz);
            let mut shots = 0;
            for _ in 0..ticks {
                let outcomes = tick_rifle(
                    &mut r,
                    RifleTickInputs {
                        fire_pressed: true,
                        ..Default::default()
                    },
                );
                if outcomes.fired_this_tick {
                    shots += 1;
                }
            }
            shots
        }
        let shots_60 = shots_in_window(60, 60);
        let shots_120 = shots_in_window(120, 120);
        // FullAuto cadence: same wall-clock window = same shot count cross-rate.
        assert_eq!(shots_60, shots_120, "FullAuto cadence must hold across tick rates");
        // Carbine: 0.083s fire interval ≈ 12 RPS. 1.0 s ≈ 12 shots.
        assert!(
            (11..=13).contains(&shots_60),
            "expected ~12 RPS for carbine FullAuto, got {shots_60} at 60 Hz"
        );
    }
}
