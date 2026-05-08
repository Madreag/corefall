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
use cf_mission::{LossConditions, Objective, ObjectiveKind, ObjectiveStatus};
use cf_terrain::BreachStrip;

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
    #[serde(default)]
    pub director: Option<serde_json::Value>,
    pub capabilities: ScenarioCapabilities,
    #[serde(default)]
    pub save_fields: Vec<String>,
    #[serde(default)]
    pub expected_tests: Vec<String>,
    #[serde(default)]
    pub notes: String,
}

/// One actor entry in `Scenario.actors`. M1 only models the player + simple dummies
/// (target practice, friendlies). M1.5 adds an optional `enemy` block that turns
/// the actor into a reactive guard. Chassis-grade actors land in M5.
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
        };
        Objective {
            id: self.id,
            kind,
            optional: self.optional,
            status: ObjectiveStatus::Pending,
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
}

impl ScenarioActor {
    pub fn build_state(&self) -> ActorState {
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
}
