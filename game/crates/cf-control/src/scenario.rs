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
use cf_mission::{LossConditions, Objective, ObjectiveKind, ObjectiveStatus, Reactor};
use cf_terrain::{material_id_from_name, BreachStrip, ChunkedTerrain, MaterialId, TerrainStamp};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scenario {
    pub schema_version: u32,
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub seed: u64,
    pub duration_ticks: Option<u64>,
    pub region: ScenarioRegion,
    pub gravity: f32,
    /// Y coordinate of the world floor (M1 stand-in for chunked terrain). Defaults to 0.
    #[serde(default)]
    pub floor_y: f32,
    #[serde(default)]
    pub teams: Vec<serde_json::Value>,
    /// Typed M1 actor entries. Empty for M0 scenarios.
    #[serde(default)]
    pub actors: Vec<ScenarioActor>,
    /// M1.5: ordered objective list. Empty when no objectives are required.
    #[serde(default)]
    pub objectives: Vec<ScenarioObjective>,
    /// M1.5: optional mission shape. `None` => no mission state machine; the
    /// scenario runs as a sandbox.
    #[serde(default)]
    pub mission: Option<ScenarioMission>,
    /// M1.5: ordered list of soft-breach strips (M2 will replace with chunked terrain).
    #[serde(default)]
    pub breaches: Vec<ScenarioBreach>,
    /// M2: optional chunked pixel terrain. When present the engine prefers
    /// chunked terrain for `act.player.dig`; legacy `breaches[]` still emit
    /// `terrain.*` events for backward compat with M1.5 evidence.
    #[serde(default)]
    pub terrain: Option<ScenarioChunkedTerrain>,
    /// M2.5: ordered list of reactors. Each reactor is a damageable static
    /// actor the player must defend.
    #[serde(default)]
    pub reactors: Vec<ScenarioReactor>,
    #[serde(default)]
    pub director: Option<serde_json::Value>,
    pub capabilities: ScenarioCapabilities,
    #[serde(default)]
    pub save_fields: Vec<String>,
    #[serde(default)]
    pub expected_tests: Vec<String>,
    #[serde(default)]
    pub notes: String,
    /// M4A: optional milestone tag override. When `None` the engine derives
    /// the milestone string from the scenario shape (actor world → m1, mission
    /// → m1.5, terrain → m2, reactors → m2.5). When `Some`, the scenario takes
    /// authoritative control of the milestone tag — e.g. `m4a_micro_breach_readability`
    /// reuses the m1.5 micro_breach world but tags the run bundle as M4A so the
    /// `expected_tests` + `notes_addendum_for_milestone` + `next_actions_for_milestone`
    /// chain reflects the actual milestone being proven.
    #[serde(default)]
    pub milestone_override: Option<String>,
}

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
    /// **M5**: optional chassis attachment (`infantry`, `powered_armor`,
    /// `light_mech` or a mod-supplied spec id).
    #[serde(default)]
    pub chassis: Option<ScenarioChassis>,
    /// **M5**: optional origin tag (`human`, `robot`, `android`).
    #[serde(default)]
    pub origin_id: Option<String>,
}

/// **M5** scenario manifest entry for chassis attachment. Resolves to a
/// runtime [`cf_chassis::ChassisState`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioChassis {
    pub spec_id: String,
    #[serde(default)]
    pub tutorial_safety: bool,
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

/// One M1.5 objective row. Discriminator strings match `cf-mission::ObjectiveKind`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioObjective {
    pub id: String,
    pub kind: ScenarioObjectiveKind,
    #[serde(default)]
    pub optional: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ScenarioObjectiveKind {
    BreachBarrier { target: String },
    NeutralizeActor { target: u64 },
    ReachZone { min: (f32, f32), max: (f32, f32) },
    DefendReactor { target: String },
}

impl ScenarioObjective {
    pub fn into_objective(self) -> Objective {
        let kind = match self.kind {
            ScenarioObjectiveKind::BreachBarrier { target } => ObjectiveKind::BreachBarrier { target },
            ScenarioObjectiveKind::NeutralizeActor { target } => ObjectiveKind::NeutralizeActor { target },
            ScenarioObjectiveKind::ReachZone { min, max } => ObjectiveKind::ReachZone {
                min: [min.0, min.1],
                max: [max.0, max.1],
            },
            ScenarioObjectiveKind::DefendReactor { target } => ObjectiveKind::DefendReactor { target },
        };
        Objective {
            id: self.id,
            kind,
            optional: self.optional,
            status: ObjectiveStatus::Pending,
        }
    }
}

/// M2 chunked terrain manifest entry. The terrain is constructed by:
///
/// 1. Allocate a `ChunkedTerrain` of size `width_px × height_px`.
/// 2. Set the default material from `default_material` (string name).
/// 3. Apply each stamp in declaration order.
///
/// Stamps share the discriminator vocabulary with `cf-terrain::TerrainStamp`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioChunkedTerrain {
    pub width_px: u32,
    pub height_px: u32,
    #[serde(default)]
    pub anchor: Option<(f32, f32)>,
    #[serde(default = "default_material_air")]
    pub default_material: String,
    #[serde(default)]
    pub stamps: Vec<ScenarioTerrainStamp>,
}

fn default_material_air() -> String {
    "air".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ScenarioTerrainStamp {
    FillAabb {
        min: (f32, f32),
        max: (f32, f32),
        material: String,
    },
    FillCircle {
        center: (f32, f32),
        radius: f32,
        material: String,
    },
}

impl From<ScenarioTerrainStamp> for TerrainStamp {
    fn from(s: ScenarioTerrainStamp) -> Self {
        match s {
            ScenarioTerrainStamp::FillAabb { min, max, material } => TerrainStamp::FillAabb {
                min: [min.0, min.1],
                max: [max.0, max.1],
                material,
            },
            ScenarioTerrainStamp::FillCircle {
                center,
                radius,
                material,
            } => TerrainStamp::FillCircle {
                center: [center.0, center.1],
                radius,
                material,
            },
        }
    }
}

impl ScenarioChunkedTerrain {
    /// Build a runtime [`ChunkedTerrain`] from this manifest. Returns an error
    /// if `default_material` or any stamp material name is not in the launch
    /// material set.
    ///
    /// `path` is the scenario file path (used in error messages so reviewers
    /// can find the offending file). Production callers go through
    /// `Scenario::load_from_file -> validate -> for_loaded_scenario` which
    /// already validates materials with the correct path; this method's
    /// strictness exists so direct callers (tests, future tools) never
    /// silently fall back to AIR for unknown defaults.
    pub fn build_terrain(&self, path: &str) -> Result<ChunkedTerrain, ScenarioLoadError> {
        // Devin BUG_pr-review-job 3212186926 (yellow): no `unwrap_or(MATERIAL_AIR)`
        // — return a structured error if the manifest names an unknown
        // material. This matches the strict stamp-material check below.
        let default_id: MaterialId =
            material_id_from_name(&self.default_material).ok_or_else(|| ScenarioLoadError::UnknownTerrainMaterial {
                path: path.to_string(),
                material: self.default_material.clone(),
            })?;
        let mut terrain = ChunkedTerrain::new(self.width_px.max(1), self.height_px.max(1), default_id);
        if let Some((ax, ay)) = self.anchor {
            terrain.anchor = [ax, ay];
        }
        // Validate each stamp's material name first so we fail at load time.
        for stamp in &self.stamps {
            let mat_name = match stamp {
                ScenarioTerrainStamp::FillAabb { material, .. } => material,
                ScenarioTerrainStamp::FillCircle { material, .. } => material,
            };
            if material_id_from_name(mat_name).is_none() {
                // Devin BUG_pr-review-job 3212186980 (yellow): thread the
                // scenario path through so the error message names the
                // offending file instead of producing the previous
                // "scenario  terrain stamp ..." with a blank path.
                return Err(ScenarioLoadError::UnknownTerrainMaterial {
                    path: path.to_string(),
                    material: mat_name.clone(),
                });
            }
        }
        let stamps: Vec<TerrainStamp> = self.stamps.iter().cloned().map(Into::into).collect();
        terrain.apply_stamps(&stamps);
        Ok(terrain)
    }
}

/// M2.5 reactor manifest entry. Becomes a `cf_mission::Reactor` at runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioReactor {
    pub id: String,
    pub position: (f32, f32),
    pub half_extents: (f32, f32),
    pub hp: f32,
}

impl ScenarioReactor {
    pub fn build_reactor(&self) -> Reactor {
        Reactor {
            id: self.id.clone(),
            position: [self.position.0, self.position.1],
            half_extents: [self.half_extents.0, self.half_extents.1],
            hp: self.hp.max(0.0),
            max_hp: self.hp.max(0.0),
            destroyed: false,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScenarioMission {
    /// Time limit in ticks (`0` = no limit). At 60 Hz, 5400 = 90 seconds.
    #[serde(default)]
    pub time_limit_ticks: u64,
    #[serde(default = "default_true")]
    pub player_dead_loses: bool,
}

fn default_true() -> bool {
    true
}

impl ScenarioMission {
    pub fn loss_conditions(&self) -> LossConditions {
        LossConditions {
            player_dead: self.player_dead_loses,
            time_limit_ticks: self.time_limit_ticks,
        }
    }
}

/// One soft-breach strip. M2 will replace these with real chunked terrain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioBreach {
    pub id: String,
    pub bbox_min: (f32, f32),
    pub bbox_max: (f32, f32),
    pub material: String,
    #[serde(default)]
    pub max_hp: Option<f32>,
    #[serde(default)]
    pub hardness: Option<f32>,
    #[serde(default)]
    pub dig_range: Option<f32>,
    /// Set when the strip is permanently un-diggable (e.g. `metal_nohook`). The
    /// dig path emits `terrain.tool_refused` with reason `material_<name>`.
    #[serde(default)]
    pub refusal_reason: Option<String>,
}

impl ScenarioBreach {
    pub fn build_strip(&self) -> BreachStrip {
        let max_hp = self.max_hp.unwrap_or(60.0);
        BreachStrip {
            id: self.id.clone(),
            bbox_min: [self.bbox_min.0, self.bbox_min.1],
            bbox_max: [self.bbox_max.0, self.bbox_max.1],
            material: self.material.clone(),
            max_hp,
            hp: max_hp,
            hardness: self.hardness.unwrap_or(20.0),
            dig_range: self.dig_range.unwrap_or(48.0),
            refusal_reason: self.refusal_reason.clone(),
            broken: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioRegion {
    pub anchor: (f32, f32),
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScenarioCapabilities {
    pub debug: bool,
    pub control_api: bool,
    pub save_load: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum ScenarioLoadError {
    #[error("io error reading scenario {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("ron parse error in scenario {path}: {source}")]
    Ron {
        path: String,
        #[source]
        source: Box<ron::error::SpannedError>,
    },
    #[error("scenario id mismatch: expected {expected}, found {found} in {path}")]
    IdMismatch {
        expected: String,
        found: String,
        path: String,
    },
    #[error("scenario {path} actor {actor_id} references unknown rifle preset `{preset}`")]
    UnknownRiflePreset {
        path: String,
        actor_id: u64,
        preset: String,
    },
    #[error("scenario {path} declares more than one controllable actor; M1 supports a single player")]
    MultiplePlayerActors { path: String },
    #[error("scenario {path} actor {actor_id} has duplicate id with another entry")]
    DuplicateActorId { path: String, actor_id: u64 },
    #[error("scenario {path} breach {breach_id} duplicates another entry")]
    DuplicateBreachId { path: String, breach_id: String },
    #[error("scenario {path} objective {objective_id} references unknown breach `{breach_id}`")]
    ObjectiveUnknownBreach {
        path: String,
        objective_id: String,
        breach_id: String,
    },
    #[error("scenario {path} objective {objective_id} references unknown actor {actor_id}")]
    ObjectiveUnknownActor {
        path: String,
        objective_id: String,
        actor_id: u64,
    },
    #[error("scenario {path} declares duplicate objective id `{objective_id}`")]
    DuplicateObjectiveId { path: String, objective_id: String },
    #[error("scenario {path} terrain stamp references unknown material `{material}`")]
    UnknownTerrainMaterial { path: String, material: String },
    #[error("scenario {path} reactor `{reactor_id}` duplicates another entry")]
    DuplicateReactorId { path: String, reactor_id: String },
    #[error("scenario {path} objective {objective_id} references unknown reactor `{reactor_id}`")]
    ObjectiveUnknownReactor {
        path: String,
        objective_id: String,
        reactor_id: String,
    },
    #[error("scenario {path} reactor `{reactor_id}` declares hp={hp} (must be > 0; a destroyed-on-spawn reactor cannot reset)")]
    ReactorHpNotPositive { path: String, reactor_id: String, hp: f32 },
}

impl ScenarioActor {
    pub fn build_state(&self) -> ActorState {
        self.build_state_with_tick_rate(60)
    }

    /// **M5**: build the actor state including chassis attachment, using the
    /// engine's configured `tick_rate_hz` so the chassis eject window is
    /// real-time stable across 60 Hz / 120 Hz scenarios.
    pub fn build_state_with_tick_rate(&self, tick_rate_hz: u32) -> ActorState {
        let inv = match &self.inventory.rifle {
            Some(preset) => Inventory {
                items: vec![
                    InventoryItem::Rifle { preset: preset.clone() },
                    InventoryItem::Empty,
                    InventoryItem::Empty,
                    InventoryItem::Empty,
                ],
                selected: ItemSlot(0),
            },
            None => Inventory::default(),
        };
        let mut actor = ActorState::player(
            ActorId(self.id),
            &self.team,
            Vec2::new(self.spawn.0, self.spawn.1),
            self.hp,
            inv,
        );
        actor.controllable = self.controllable;
        if let Some((hx, hy)) = self.half_extents {
            actor.half_extents = Vec2::new(hx, hy);
        }
        if let Some((ax, ay)) = self.aim {
            actor.aim = Vec2::new(ax, ay);
        }
        if let Some(origin) = &self.origin_id {
            actor.origin_id = origin.clone();
        }
        if let Some(chassis_def) = &self.chassis {
            if let Some(chassis_state) = chassis_def.build_state(tick_rate_hz) {
                actor.attach_chassis(chassis_state);
            }
        }
        actor
    }

    pub fn rifle_state(&self, tick_rate_hz: u32) -> Option<RifleState> {
        let preset = self.inventory.rifle.as_deref()?;
        rifle_preset(preset).map(|spec| RifleState::new(spec, tick_rate_hz))
    }

    /// True when this actor is configured as a reactive guard (M1.5).
    pub fn is_reactive_guard(&self) -> bool {
        self.enemy.is_some()
    }
}

impl Scenario {
    pub fn load_from_file(path: &Path) -> Result<Self, ScenarioLoadError> {
        let text = std::fs::read_to_string(path).map_err(|source| ScenarioLoadError::Io {
            path: path.display().to_string(),
            source,
        })?;
        let scenario: Scenario = ron::from_str(&text).map_err(|source| ScenarioLoadError::Ron {
            path: path.display().to_string(),
            source: Box::new(source),
        })?;
        scenario.validate(&path.display().to_string())?;
        Ok(scenario)
    }

    fn validate(&self, path: &str) -> Result<(), ScenarioLoadError> {
        let mut player_seen = false;
        let mut ids_seen = std::collections::HashSet::new();
        for actor in &self.actors {
            if !ids_seen.insert(actor.id) {
                return Err(ScenarioLoadError::DuplicateActorId {
                    path: path.to_string(),
                    actor_id: actor.id,
                });
            }
            if actor.controllable {
                if player_seen {
                    return Err(ScenarioLoadError::MultiplePlayerActors { path: path.to_string() });
                }
                player_seen = true;
            }
            if let Some(preset) = &actor.inventory.rifle {
                if rifle_preset(preset).is_none() {
                    return Err(ScenarioLoadError::UnknownRiflePreset {
                        path: path.to_string(),
                        actor_id: actor.id,
                        preset: preset.clone(),
                    });
                }
            }
        }

        // Breach ids are unique.
        let mut breach_ids = std::collections::HashSet::new();
        for breach in &self.breaches {
            if !breach_ids.insert(breach.id.clone()) {
                return Err(ScenarioLoadError::DuplicateBreachId {
                    path: path.to_string(),
                    breach_id: breach.id.clone(),
                });
            }
        }

        // M2.5: reactor ids unique + hp positive.
        let mut reactor_ids = std::collections::HashSet::new();
        for reactor in &self.reactors {
            if !reactor_ids.insert(reactor.id.clone()) {
                return Err(ScenarioLoadError::DuplicateReactorId {
                    path: path.to_string(),
                    reactor_id: reactor.id.clone(),
                });
            }
            // Bugbot 3212274163 (Low): a reactor declared with hp <= 0
            // would start destroyed AND `reset()` would set hp = max_hp = 0
            // so the reactor stays destroyed forever (since
            // `is_destroyed` returns `destroyed || hp <= 0`). Reject at
            // load so scenarios cannot author an unresettable reactor.
            if !reactor.hp.is_finite() || reactor.hp <= 0.0 {
                return Err(ScenarioLoadError::ReactorHpNotPositive {
                    path: path.to_string(),
                    reactor_id: reactor.id.clone(),
                    hp: reactor.hp,
                });
            }
        }

        // Objectives must reference real targets, with unique ids.
        let mut objective_ids = std::collections::HashSet::new();
        for objective in &self.objectives {
            if !objective_ids.insert(objective.id.clone()) {
                return Err(ScenarioLoadError::DuplicateObjectiveId {
                    path: path.to_string(),
                    objective_id: objective.id.clone(),
                });
            }
            match &objective.kind {
                ScenarioObjectiveKind::BreachBarrier { target } => {
                    if !breach_ids.contains(target) {
                        return Err(ScenarioLoadError::ObjectiveUnknownBreach {
                            path: path.to_string(),
                            objective_id: objective.id.clone(),
                            breach_id: target.clone(),
                        });
                    }
                }
                ScenarioObjectiveKind::NeutralizeActor { target } => {
                    if !self.actors.iter().any(|a| a.id == *target) {
                        return Err(ScenarioLoadError::ObjectiveUnknownActor {
                            path: path.to_string(),
                            objective_id: objective.id.clone(),
                            actor_id: *target,
                        });
                    }
                }
                ScenarioObjectiveKind::ReachZone { .. } => {}
                ScenarioObjectiveKind::DefendReactor { target } => {
                    if !reactor_ids.contains(target) {
                        return Err(ScenarioLoadError::ObjectiveUnknownReactor {
                            path: path.to_string(),
                            objective_id: objective.id.clone(),
                            reactor_id: target.clone(),
                        });
                    }
                }
            }
        }

        // M2: validate terrain stamp material names.
        if let Some(terrain) = &self.terrain {
            if material_id_from_name(&terrain.default_material).is_none() {
                return Err(ScenarioLoadError::UnknownTerrainMaterial {
                    path: path.to_string(),
                    material: terrain.default_material.clone(),
                });
            }
            for stamp in &terrain.stamps {
                let mat_name = match stamp {
                    ScenarioTerrainStamp::FillAabb { material, .. } => material,
                    ScenarioTerrainStamp::FillCircle { material, .. } => material,
                };
                if material_id_from_name(mat_name).is_none() {
                    return Err(ScenarioLoadError::UnknownTerrainMaterial {
                        path: path.to_string(),
                        material: mat_name.clone(),
                    });
                }
            }
        }

        Ok(())
    }

    /// True if the scenario contains any typed `actors[]` (M1+); false for M0 scenarios.
    pub fn has_actor_world(&self) -> bool {
        !self.actors.is_empty()
    }

    /// Resolved player actor id (the single `controllable` actor) or `None`.
    pub fn player_actor_id(&self) -> Option<u64> {
        self.actors.iter().find(|a| a.controllable).map(|a| a.id)
    }

    /// True if the scenario carries a mission state machine (objectives + loss
    /// conditions). M0/M1 scenarios are sandbox-only and return false.
    pub fn has_mission(&self) -> bool {
        !self.objectives.is_empty() || self.mission.is_some()
    }

    /// True if the scenario declares chunked terrain (M2+).
    pub fn has_chunked_terrain(&self) -> bool {
        self.terrain.is_some()
    }

    /// True if the scenario declares any reactor (M2.5+).
    pub fn has_reactors(&self) -> bool {
        !self.reactors.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"(
  schema_version: 1,
  id: "m0_blank",
  display_name: "M0 Blank Scene",
  description: "Empty scene used for engine bootstrap and run-bundle smoke.",
  seed: 42,
  duration_ticks: Some(300),
  region: (anchor: (0.0, 0.0), width: 1280.0, height: 720.0),
  gravity: -980.0,
  teams: [],
  actors: [],
  objectives: [],
  director: None,
  capabilities: (
    debug: false,
    control_api: true,
    save_load: false,
  ),
  save_fields: [],
  expected_tests: ["M0-SMOKE-01"],
  notes: "",
)"#;

    #[test]
    fn loads_minimal_scenario() {
        let parsed: Scenario = ron::from_str(SAMPLE).expect("sample must parse");
        assert_eq!(parsed.id, "m0_blank");
        assert_eq!(parsed.seed, 42);
        assert_eq!(parsed.duration_ticks, Some(300));
        assert_eq!(parsed.expected_tests, vec!["M0-SMOKE-01"]);
        assert!(parsed.capabilities.control_api);
        assert!(!parsed.has_actor_world());
        assert!(!parsed.has_mission());
    }

    const M1_SAMPLE: &str = r#"(
  schema_version: 1,
  id: "m1_actor_range",
  display_name: "M1 Actor Range",
  description: "M1 actor controller test scene.",
  seed: 7,
  duration_ticks: Some(18000),
  region: (anchor: (0.0, 0.0), width: 1280.0, height: 720.0),
  gravity: -980.0,
  floor_y: 16.0,
  teams: [],
  actors: [
    (id: 1, team: "blue", spawn: (200.0, 32.0), controllable: true, hp: 100.0,
      inventory: (rifle: Some("rifle_m1_default")), half_extents: Some((8.0, 16.0))),
    (id: 2, team: "red", spawn: (900.0, 32.0), controllable: false, hp: 100.0,
      inventory: (rifle: None)),
  ],
  objectives: [],
  director: None,
  capabilities: (debug: false, control_api: true, save_load: false),
  save_fields: [],
  expected_tests: ["M1-SMOKE-01", "M1-FIRE-01"],
  notes: "M1 actor range fixture.",
)"#;

    #[test]
    fn loads_m1_actor_range_scenario() {
        let parsed: Scenario = ron::from_str(M1_SAMPLE).expect("m1 sample must parse");
        assert_eq!(parsed.id, "m1_actor_range");
        assert_eq!(parsed.actors.len(), 2);
        assert!(parsed.has_actor_world());
        assert_eq!(parsed.player_actor_id(), Some(1));
        let player = &parsed.actors[0];
        assert!(player.controllable);
        assert_eq!(player.inventory.rifle.as_deref(), Some("rifle_m1_default"));
        let player_state = player.build_state();
        assert!(player_state.controllable);
        assert!(player.rifle_state(60).is_some());
    }

    #[test]
    fn rejects_unknown_rifle_preset() {
        let scenario_text = M1_SAMPLE.replace("rifle_m1_default", "rifle_does_not_exist");
        let parsed: Scenario = ron::from_str(&scenario_text).unwrap();
        assert!(parsed.validate("test.ron").is_err());
    }

    #[test]
    fn rejects_two_controllable_actors() {
        let scenario_text = M1_SAMPLE.replace(
            "controllable: false, hp: 100.0,\n      inventory: (rifle: None)",
            "controllable: true, hp: 100.0,\n      inventory: (rifle: None)",
        );
        let parsed: Scenario = ron::from_str(&scenario_text).unwrap();
        assert!(matches!(
            parsed.validate("t.ron"),
            Err(ScenarioLoadError::MultiplePlayerActors { .. })
        ));
    }

    const M1_5_SAMPLE: &str = r#"(
  schema_version: 1,
  id: "micro_breach",
  display_name: "Micro Breach",
  description: "M1.5 micro breach fun slice.",
  seed: 17,
  duration_ticks: Some(5400),
  region: (anchor: (0.0, 0.0), width: 1280.0, height: 720.0),
  gravity: -980.0,
  floor_y: 16.0,
  teams: [],
  actors: [
    (id: 1, team: "blue", spawn: (96.0, 32.0), controllable: true, hp: 100.0,
      inventory: (rifle: Some("rifle_m1_default"))),
    (id: 2, team: "red", spawn: (900.0, 32.0), controllable: false, hp: 80.0,
      inventory: (rifle: None), aim: Some((-1.0, 0.0)),
      enemy: Some((kind: Some("reactive_guard")))),
  ],
  breaches: [
    (id: "outer_wall", bbox_min: (600.0, 16.0), bbox_max: (664.0, 96.0), material: "concrete_soft"),
    (id: "anchor", bbox_min: (760.0, 16.0), bbox_max: (792.0, 96.0), material: "metal_nohook",
      refusal_reason: Some("metal_nohook")),
  ],
  objectives: [
    (id: "breach", kind: { "kind": "breach_barrier", "target": "outer_wall" }),
    (id: "neutralize", kind: { "kind": "neutralize_actor", "target": 2 }),
    (id: "extract", kind: { "kind": "reach_zone", "min": (1180.0, 16.0), "max": (1280.0, 96.0) }),
  ],
  mission: Some((time_limit_ticks: 5400, player_dead_loses: true)),
  director: None,
  capabilities: (debug: false, control_api: true, save_load: false),
  save_fields: [],
  expected_tests: ["M1.5-SMOKE-01"],
  notes: "M1.5 micro breach fixture.",
)"#;

    #[test]
    fn loads_m1_5_scenario_with_breaches_and_objectives() {
        let parsed: Scenario = ron::from_str(M1_5_SAMPLE).expect("m1.5 sample must parse");
        assert_eq!(parsed.id, "micro_breach");
        assert!(parsed.has_actor_world());
        assert!(parsed.has_mission());
        assert_eq!(parsed.breaches.len(), 2);
        assert_eq!(parsed.objectives.len(), 3);
        let mission = parsed.mission.as_ref().unwrap();
        assert_eq!(mission.time_limit_ticks, 5400);
        assert!(mission.player_dead_loses);
        let guard = &parsed.actors[1];
        assert!(guard.is_reactive_guard());
        let _ = guard.enemy.as_ref().unwrap().build_params();
        // Objective-target validation fires.
        let mut bad = parsed.clone();
        bad.objectives[1] = ScenarioObjective {
            id: "neutralize".to_string(),
            kind: ScenarioObjectiveKind::NeutralizeActor { target: 99 },
            optional: false,
        };
        assert!(matches!(
            bad.validate("t.ron"),
            Err(ScenarioLoadError::ObjectiveUnknownActor { .. })
        ));
    }

    #[test]
    fn rejects_reactor_with_zero_or_negative_hp() {
        // Bugbot 3212274163 (Low) regression: a reactor declared with
        // hp <= 0 would start destroyed AND can never be reset (since
        // max_hp = hp = 0 and `is_destroyed` returns `hp <= 0`). The
        // validator MUST reject the scenario at load.
        let scenario = Scenario {
            schema_version: 1,
            id: "test".to_string(),
            display_name: "test".to_string(),
            description: "test".to_string(),
            seed: 0,
            duration_ticks: None,
            region: ScenarioRegion {
                anchor: (0.0, 0.0),
                width: 100.0,
                height: 100.0,
            },
            gravity: -980.0,
            floor_y: 0.0,
            teams: vec![],
            actors: vec![],
            objectives: vec![],
            mission: None,
            breaches: vec![],
            terrain: None,
            reactors: vec![ScenarioReactor {
                id: "core".to_string(),
                position: (50.0, 50.0),
                half_extents: (10.0, 10.0),
                hp: 0.0,
            }],
            director: None,
            capabilities: ScenarioCapabilities::default(),
            save_fields: vec![],
            expected_tests: vec![],
            notes: String::new(),
            milestone_override: None,
        };
        assert!(matches!(
            scenario.validate("t.ron"),
            Err(ScenarioLoadError::ReactorHpNotPositive { .. })
        ));

        // Negative hp also rejected.
        let mut s2 = scenario.clone();
        s2.reactors[0].hp = -10.0;
        assert!(matches!(
            s2.validate("t.ron"),
            Err(ScenarioLoadError::ReactorHpNotPositive { .. })
        ));

        // NaN rejected.
        let mut s3 = scenario;
        s3.reactors[0].hp = f32::NAN;
        assert!(matches!(
            s3.validate("t.ron"),
            Err(ScenarioLoadError::ReactorHpNotPositive { .. })
        ));
    }
}
