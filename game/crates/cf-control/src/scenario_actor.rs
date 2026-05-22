//! RON scenario loader. The full schema lives in `spec/prototype-roadmap.md`
//! Scenario Manifest Schema; M0/M1/M1.5 implement a subset:
//!
//! - M0 ships engine bootstrap (no actors, empty regions).
//! - M1 adds typed `actors[]` entries (player + optional dummies) and a `floor_y`
//!   so `cf-physics` can resolve ground collisions.
//! - M1.5 adds `breaches[]`, `objectives[]`, and per-actor `enemy: ReactiveGuard`
//!   parameters so the micro breach scenario can run end-to-end.

use std::path::Path;

use serde::{Deserialize, Serialize};

use cf_actor::{ActorId, ActorState, Inventory, InventoryItem, ItemSlot, Vec2};
use cf_ai::ReactiveGuardParams;
use cf_equipment::{rifle_preset, RifleState};
use cf_mission::{
    BossState, BranchingPoint, ExtendedObjectiveKind, LossConditions, MissionPhase, Objective, ObjectiveGraph,
    ObjectiveKind, ObjectiveNode, ObjectiveNodeStatus, ObjectiveStatus, PhaseState, Reactor, ReinforcementWave,
};
use cf_terrain::{material_id_from_name, BreachStrip, ChunkedTerrain, MaterialId, TerrainStamp};

#[allow(unused_imports)]
use crate::scenario::*;


/// One actor entry in `Scenario.actors`. M1 only models the player + simple dummies
/// (target practice, friendlies). M1.5 adds an optional `enemy` block that turns
/// the actor into a reactive guard. **M5** adds an optional `chassis` block that
/// attaches a full chassis grammar to the actor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioActor {
    pub id: u64,
    pub team: String,
    pub spawn: (f32, f32),
    #[serde(default)]
    pub controllable: bool,
    pub hp: f32,
    #[serde(default)]
    pub inventory: ScenarioInventory,
    /// Half-extents (width, height) of the actor's collision proxy. Defaults to 8x16.
    /// When `chassis` is set, the chassis kind overrides these defaults to fit
    /// the actor silhouette.
    #[serde(default)]
    pub half_extents: Option<(f32, f32)>,
    /// M1.5: optional initial aim direction (defaults to `(1.0, 0.0)`). Reactive
    /// guards face this direction; the AI updates aim every tick from there.
    #[serde(default)]
    pub aim: Option<(f32, f32)>,
    /// M1.5: optional reactive-guard configuration. When `Some`, the engine
    /// drives this actor through `cf-ai::ReactiveGuard`.
    #[serde(default)]
    pub enemy: Option<ScenarioEnemy>,
    /// `light_mech` or a mod-supplied spec id).
    #[serde(default)]
    pub chassis: Option<ScenarioChassis>,
    #[serde(default)]
    pub origin_id: Option<String>,
    /// the [`cf_squad::Squad`] at scenario init and emits one
    /// `squad.member_added` event. Accepted values: `"leader"` /
    /// `"follower"`. `None` for non-squad actors (enemies, dummies).
    #[serde(default)]
    pub squad_role: Option<String>,
    /// squad followers. Currently unused beyond influencing the bot's
    /// default `current_command` (FollowLeader); M7 expands to full
    /// archetypes. Free-form string tag (`"rifleman"` / `"medic"` /
    /// `"engineer"` etc).
    #[serde(default)]
    pub squad_archetype: Option<String>,
}

/// runtime [`cf_chassis::ChassisState`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioChassis {
    pub spec_id: String,
    #[serde(default)]
    pub tutorial_safety: bool,
    /// spec. Each entry maps `kind` (snake_case `ModuleKind` discriminator
    /// — currently only `"era"` is honored) to a `BodyZone` + per-panel
    /// HP + ERA charge. Used by `m14c_heat_vs_era.ron` to bolt an ERA
    /// panel onto the Heavy Trooper torso so the M14C HEAT producer
    /// emits `armor.era_pre_detonated` strictly before
    /// `armor.heat_jet_traversed` (VAL-M14C-009/011).
    #[serde(default)]
    pub extra_modules: Vec<ScenarioExtraModule>,
    /// Optional initial stage override. Accepts `"nominal" | "degraded" |
    /// "critical" | "wreck" | "disabled" | "salvaged"`. When set to `"wreck"`
    /// or `"disabled"`, `act.chassis.salvage` becomes immediately valid against
    /// the spawned chassis. Default = scenario seeds a Nominal chassis.
    #[serde(default)]
    pub initial_stage: Option<String>,
}

impl ScenarioChassis {
    pub fn build_state(&self, tick_rate_hz: u32) -> Option<cf_chassis::ChassisState> {
        let mut state = cf_chassis::chassis_spec(&self.spec_id)
            .map(|spec| cf_chassis::ChassisState::from_spec(&spec, tick_rate_hz, self.tutorial_safety))?;
        // `era` kind is honored — the panel is bolted onto the configured
        // body zone with the requested HP + era_charge_kg + one-shot
        // consumable flag set true so HEAT impacts trigger ERA
        // pre-detonation.
        for extra in &self.extra_modules {
            if let Some(module) = extra.build_module() {
                state.modules.push(module);
            }
        }
        if let Some(stage) = self.initial_stage.as_deref() {
            let target = match stage.to_ascii_lowercase().as_str() {
                "nominal" => Some(cf_chassis::ChassisStage::Nominal),
                "degraded" => Some(cf_chassis::ChassisStage::Degraded),
                "module_warning" => Some(cf_chassis::ChassisStage::ModuleWarning),
                "module_failed" => Some(cf_chassis::ChassisStage::ModuleFailed),
                "weapon_jammed" => Some(cf_chassis::ChassisStage::WeaponJammed),
                "armor_cracked" => Some(cf_chassis::ChassisStage::ArmorCracked),
                "disabled" => Some(cf_chassis::ChassisStage::Disabled),
                "pilot_injured" => Some(cf_chassis::ChassisStage::PilotInjured),
                "eject" => Some(cf_chassis::ChassisStage::Eject),
                "bail_too_late" => Some(cf_chassis::ChassisStage::BailTooLate),
                "wreck" | "wrecked" => Some(cf_chassis::ChassisStage::Wreck),
                "gibbed" => Some(cf_chassis::ChassisStage::Gibbed),
                _ => None,
            };
            if let Some(target_stage) = target {
                state.force_stage(target_stage);
            }
        }
        Some(state)
    }
}

/// on top of the base spec. M14C ships exactly one kind — `era` — for the
/// HEAT-vs-ERA scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioExtraModule {
    /// Snake-case `ModuleKind` discriminator (`"era"`, `"ammo_rack"`, etc.).
    pub kind: String,
    /// Stable module id used as the event payload `module_id`.
    pub id: String,
    /// Body zone the module is bound to (`"torso"`, `"head"`, etc.).
    pub zone: String,
    /// Module HP at spawn time.
    #[serde(default = "default_extra_module_hp")]
    pub hp_max: f32,
    /// `era_charge_kg × 0.7` HEAT penetration reduction formula.
    #[serde(default)]
    pub era_charge_kg: Option<f32>,
}

impl ScenarioExtraModule {
    /// Build the matching [`cf_chassis::ChassisModule`]. Returns `None` for
    /// unknown kind / zone identifiers so the scenario loader can reject
    /// the manifest cleanly.
    pub fn build_module(&self) -> Option<cf_chassis::ChassisModule> {
        let kind = parse_module_kind(&self.kind)?;
        let zone = parse_body_zone(&self.zone)?;
        let mut module = cf_chassis::ChassisModule::new(&self.id, kind, zone, self.hp_max);
        if matches!(kind, cf_chassis::ModuleKind::Era) {
            module = module.with_era(self.era_charge_kg.unwrap_or(1.0), true);
        }
        Some(module)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScenarioInventory {
    /// Optional rifle preset id; resolved against `cf-equipment::rifle_preset`.
    #[serde(default)]
    pub rifle: Option<String>,
}

/// Reactive-guard parameters for one actor. Defaults match
/// [`ReactiveGuardParams::default`] so scenarios can override only the fields
/// they need.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ScenarioEnemy {
    pub kind: Option<String>,
    pub sight_radius: Option<f32>,
    pub sight_cone_degrees: Option<f32>,
    pub aim_settle_seconds: Option<f32>,
    pub miss_chance: Option<f32>,
    pub alert_dwell_seconds: Option<f32>,
    pub burst_shots: Option<u32>,
    pub burst_pause_seconds: Option<f32>,
    pub damage_per_hit: Option<f32>,
    pub projectile_speed: Option<f32>,
    pub projectile_lifetime_seconds: Option<f32>,
    pub mag_capacity: Option<u32>,
    pub reload_seconds: Option<f32>,
    pub muzzle_forward_offset: Option<f32>,
    pub muzzle_vertical_offset: Option<f32>,
    /// pipeline. The engine recognises `"AI-TRENCH-A-01"` (M9B trench
    /// garrison doctrine); other values are forward-compat placeholders
    /// and are ignored by the current engine.
    pub doctrine: Option<String>,
}

impl ScenarioEnemy {
    pub fn build_params(&self) -> ReactiveGuardParams {
        let mut p = ReactiveGuardParams::default();
        if let Some(v) = self.sight_radius {
            p.sight_radius = v;
        }
        if let Some(v) = self.sight_cone_degrees {
            p.sight_cone_degrees = v;
        }
        if let Some(v) = self.aim_settle_seconds {
            p.aim_settle_seconds = v;
        }
        if let Some(v) = self.miss_chance {
            p.miss_chance = v;
        }
        if let Some(v) = self.alert_dwell_seconds {
            p.alert_dwell_seconds = v;
        }
        if let Some(v) = self.burst_shots {
            p.burst_shots = v;
        }
        if let Some(v) = self.burst_pause_seconds {
            p.burst_pause_seconds = v;
        }
        if let Some(v) = self.damage_per_hit {
            p.damage_per_hit = v;
        }
        if let Some(v) = self.projectile_speed {
            p.projectile_speed = v;
        }
        if let Some(v) = self.projectile_lifetime_seconds {
            p.projectile_lifetime_seconds = v;
        }
        if let Some(v) = self.mag_capacity {
            p.mag_capacity = v;
        }
        if let Some(v) = self.reload_seconds {
            p.reload_seconds = v;
        }
        if let Some(v) = self.muzzle_forward_offset {
            p.muzzle_forward_offset = v;
        }
        if let Some(v) = self.muzzle_vertical_offset {
            p.muzzle_vertical_offset = v;
        }
        p
    }
}

