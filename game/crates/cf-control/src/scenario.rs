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

pub use crate::scenario_actor::{
    ScenarioActor, ScenarioChassis, ScenarioEnemy, ScenarioExtraModule, ScenarioInventory,
};
pub use crate::scenario_m14d::{
    LateralWallSpan, ScenarioM14dProjectile, ScenarioMaterialContact, ScenarioThermalZone,
    ScenarioTunnelSpan,
};
pub use crate::scenario_mission::{
    ScenarioBossState, ScenarioMission, ScenarioMissionPhase, ScenarioPhaseState,
    ScenarioReinforcementWave,
};
pub use crate::scenario_objective::{
    ScenarioExtendedObjectiveKind, ScenarioObjective, ScenarioObjectiveGraph,
    ScenarioObjectiveGraphBranch, ScenarioObjectiveGraphNode, ScenarioObjectiveKind,
};
pub use crate::scenario_script::ScenarioScriptStep;
pub use crate::scenario_terrain::{
    ScenarioBreach, ScenarioChunkedTerrain, ScenarioReactor, ScenarioTerrainStamp,
};

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
    /// Optional scenario tags for grouping/filtering (lab/tutorial/mission).
    #[serde(default)]
    pub scenario_tags: Vec<String>,
    /// Optional difficulty preset (e.g. "cakewalk", "tough_crowd", "veteran").
    #[serde(default)]
    pub difficulty_preset: Option<String>,
    /// Optional loadout template reference (M5 spec).
    #[serde(default)]
    pub loadout_template: Option<String>,
    /// Optional loss reason vocabulary for typed loss reasons.
    #[serde(default)]
    pub loss_reason_vocabulary: Vec<String>,
    /// M4A: optional milestone tag override. When `None` the engine derives
    /// the milestone string from the scenario shape (actor world → m1, mission
    /// → m1.5, terrain → m2, reactors → m2.5). When `Some`, the scenario takes
    /// authoritative control of the milestone tag — e.g. `m4a_micro_breach_readability`
    /// reuses the m1.5 micro_breach world but tags the run bundle as M4A so the
    /// `expected_tests` + `notes_addendum_for_milestone` + `next_actions_for_milestone`
    /// chain reflects the actual milestone being proven.
    #[serde(default)]
    pub milestone_override: Option<String>,
    /// engine seeds INACTIVE for `controllable` actors during the script's
    /// onboarding window and refuses lethal damage transitions. M1 surfaces
    /// the flag through the manifest -> engine config -> actor.set_inactive
    /// path; M1.5+ tutorials act on it.
    #[serde(default)]
    pub tutorial_safety: bool,
    /// mission director. When `Some`, the engine seeds `M7AiWorld.phase`
    /// at scenario start and emits `mission.phase_changed` per the
    /// configured second budgets. `None` means the scenario opts out of
    /// the v0.5 phase pacer.
    #[serde(default)]
    pub phase_state: Option<ScenarioPhaseState>,
    /// declarations. Empty means no waves; the engine still ticks
    /// `try_spawn_reinforcement` but the registry produces no events.
    #[serde(default)]
    pub reinforcement_waves: Vec<ScenarioReinforcementWave>,
    /// When `Some`, the engine seeds `M7AiWorld.boss` at scenario start
    /// and routes hits against `actor_id` into `apply_boss_damage`.
    #[serde(default)]
    pub boss_state: Option<ScenarioBossState>,
    /// graph (DiGraph + branching points + optional/parallel objectives).
    /// When `Some`, the engine seeds `M7AiWorld.objective_graph` at
    /// scenario start and emits `mission.objective_branched` /
    /// `mission.optional_offered` per `drain_objective_graph_emissions`.
    #[serde(default)]
    pub objective_graph: Option<ScenarioObjectiveGraph>,
    /// modifiers stacked atop `gravity`. Empty by default; scenarios that
    /// declare gravity wells, low-g labs, magnetic boots, reverse-g rooms,
    /// or damaged generators populate this array.
    #[serde(default)]
    pub gravity_overrides: Vec<cf_mission::ScenarioGravityOverride>,
    /// authored atmosphere cells. Empty by default; scenarios that declare
    /// pipe ruptures, vent fans, or breaches populate this array.
    #[serde(default)]
    pub wind_sources: Vec<cf_mission::ScenarioWindSource>,
    /// composition that feed the wind force + stratification producers.
    /// Empty by default; scenarios opt in by declaring cells (typically
    /// alongside `wind_sources`).
    #[serde(default)]
    pub atmosphere_cells: Vec<cf_mission::ScenarioAtmosCell>,
    /// engine injects into `pending_intent` before the actor sim runs.
    /// Mirrors what a human (or a cfctl runner) would type via
    /// `cfctl.act.player.{move,aim,fire,reload}` so headless cfctl drives
    /// of `m14c_heat_vs_era.ron` / `m14c_apfsds_vs_heavy.ron` can fire a
    /// HEAT / APFSDS round at a deterministic tick without an external
    /// driver. Empty by default; pre-M14C scenarios behave identically
    /// (no scripted intent injection).
    #[serde(default)]
    pub scripted_steps: Vec<ScenarioScriptStep>,
    /// Drives the per-tick projectile-pair CCD pass
    /// (`cf_physics::run_projectile_pair_pass`) between the actor-collision
    /// pass and the terrain pass. Empty by default; only M14D scenarios
    /// (e.g., `m14d_projectile_intercept.ron`) populate it.
    #[serde(default)]
    pub m14d_projectile_pool: Vec<ScenarioM14dProjectile>,
    /// setting. Default false. Setting `true` opts the killcam back into
    /// surfacing `collision.projectile_pair_contact` events.
    #[serde(default)]
    pub m14d_replay_intercepts: bool,
    /// scenario manifest. Empty by default; M14E scenarios populate one
    /// or more ScenarioTunnelSpan rows so the integrity pass + cave-in
    /// roll fire against a known tunnel topology.
    #[serde(default)]
    pub m14e_tunnel_spans: Vec<ScenarioTunnelSpan>,
    /// rely on cave-in determinism (`m14e_tunnel_collapse_drill.ron`)
    /// set this so the engine's cave-in RNG draw is reproducible. Default
    /// 0 falls back to the engine's `seed` field.
    #[serde(default)]
    pub m14e_cave_in_seed_offset: u64,
    /// scenario manifest. Empty by default; M14F scenarios populate
    /// one or more [`LateralWallSpan`] rows so the lateral integrity
    /// pass + bulging→crack_advanced→rupture cascade fires against a
    /// known sidewall topology. Distinct from `m14e_tunnel_spans` so
    /// the ceiling + lateral passes don't share semantics (axes
    /// differ; the underlying `IntegrityField` buffer is still shared
    /// per VAL-CROSS-005).
    #[serde(default)]
    pub m14f_lateral_wall_spans: Vec<LateralWallSpan>,
    /// by the scenario manifest. Each entry models one actor zone in
    /// sustained contact with a tile at a given temperature; the
    /// engine ticks the dwell counter every tick and runs the
    /// [`cf_environment::classify_tile_thermal`] producer to emit
    /// typed [`cf_wound::WoundKind::Burn1st`]/`Burn2nd`/`Burn3rd` (hot)
    /// or [`cf_wound::WoundKind::Frostbite1st`]/`Frostbite2nd`/`Frostbite3rd`
    /// (cold) records.
    #[serde(default)]
    pub m14g_thermal_zones: Vec<ScenarioThermalZone>,
    /// scenario manifest. Each entry models one actor zone in contact
    /// with a hazardous material; the engine ticks one
    /// [`cf_material::classify_reaction`] call per tick at the
    /// supplied intensity and emits typed
    /// [`cf_wound::WoundKind::AcidBurn`] / `ChemicalBurn` records on
    /// the supplied zone.
    #[serde(default)]
    pub m14g_material_contacts: Vec<ScenarioMaterialContact>,
}


pub(crate) fn default_m14d_radius() -> f32 {
    1.0
}

pub(crate) fn default_m14d_mass_kg() -> f32 {
    0.01
}


pub(crate) fn default_ceiling_thickness() -> u32 {
    4
}

pub(crate) fn default_vibration_modifier() -> f32 {
    1.0
}


pub(crate) fn default_wall_thickness() -> u32 {
    4
}

pub(crate) fn default_lateral_yield_strength() -> u16 {
    50
}

pub(crate) fn default_lateral_topology() -> String {
    "mineshaft".to_string()
}

pub(crate) fn default_sealed_room_pressure() -> f32 {
    101.0
}



pub(crate) fn default_material_intensity() -> f32 {
    0.5
}







pub(crate) fn default_extra_module_hp() -> f32 {
    30.0
}


pub(crate) fn parse_module_kind(s: &str) -> Option<cf_chassis::ModuleKind> {
    match s.to_ascii_lowercase().as_str() {
        "era" => Some(cf_chassis::ModuleKind::Era),
        "ammo_rack" => Some(cf_chassis::ModuleKind::AmmoRack),
        "engine" => Some(cf_chassis::ModuleKind::Engine),
        "fuel_tank" => Some(cf_chassis::ModuleKind::FuelTank),
        "weapon_mount" => Some(cf_chassis::ModuleKind::WeaponMount),
        "jet" => Some(cf_chassis::ModuleKind::Jet),
        "shield" => Some(cf_chassis::ModuleKind::Shield),
        "sensor" => Some(cf_chassis::ModuleKind::Sensor),
        "repair_drone" => Some(cf_chassis::ModuleKind::RepairDrone),
        _ => None,
    }
}

pub(crate) fn parse_body_zone(s: &str) -> Option<cf_chassis::BodyZone> {
    match s.to_ascii_lowercase().as_str() {
        "head" => Some(cf_chassis::BodyZone::Head),
        "torso" => Some(cf_chassis::BodyZone::Torso),
        "arm_right" => Some(cf_chassis::BodyZone::ArmRight),
        "arm_left" => Some(cf_chassis::BodyZone::ArmLeft),
        "leg_right" => Some(cf_chassis::BodyZone::LegRight),
        "leg_left" => Some(cf_chassis::BodyZone::LegLeft),
        "backpack" => Some(cf_chassis::BodyZone::Backpack),
        "forearm_right" => Some(cf_chassis::BodyZone::ForearmRight),
        "forearm_left" => Some(cf_chassis::BodyZone::ForearmLeft),
        "hand_right" => Some(cf_chassis::BodyZone::HandRight),
        "hand_left" => Some(cf_chassis::BodyZone::HandLeft),
        "shin_right" => Some(cf_chassis::BodyZone::ShinRight),
        "shin_left" => Some(cf_chassis::BodyZone::ShinLeft),
        "foot_right" => Some(cf_chassis::BodyZone::FootRight),
        "foot_left" => Some(cf_chassis::BodyZone::FootLeft),
        _ => None,
    }
}








pub(crate) fn default_material_air() -> String {
    "air".to_string()
}







pub(crate) fn default_true() -> bool {
    true
}





pub(crate) fn default_setup_seconds() -> f32 {
    30.0
}

pub(crate) fn default_buildup_seconds() -> f32 {
    60.0
}

pub(crate) fn default_climax_seconds() -> f32 {
    120.0
}



pub(crate) fn default_spawn_count() -> u32 {
    3
}





pub(crate) fn default_boss_phase_2_threshold() -> f32 {
    0.75
}

pub(crate) fn default_boss_phase_3_threshold() -> f32 {
    0.25
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
            scenario_tags: vec![],
            difficulty_preset: None,
            loadout_template: None,
            loss_reason_vocabulary: vec![],
            milestone_override: None,
            tutorial_safety: false,
            phase_state: None,
            reinforcement_waves: vec![],
            boss_state: None,
            objective_graph: None,
            gravity_overrides: vec![],
            wind_sources: vec![],
            atmosphere_cells: vec![],
            scripted_steps: vec![],
            m14d_projectile_pool: vec![],
            m14d_replay_intercepts: false,
            m14e_tunnel_spans: vec![],
            m14e_cave_in_seed_offset: 0,
            m14f_lateral_wall_spans: vec![],
            m14g_thermal_zones: vec![],
            m14g_material_contacts: vec![],
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
