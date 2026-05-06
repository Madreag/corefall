//! RON scenario loader. The full schema lives in `spec/prototype-roadmap.md`
//! Scenario Manifest Schema; M0/M1 implement a subset:
//!
//! - M0 ships engine bootstrap (no actors, empty regions).
//! - M1 adds typed `actors[]` entries (player + optional dummies) and a `floor_y`
//!   so `cf-physics` can resolve ground collisions.

use std::path::Path;

use serde::{Deserialize, Serialize};

use cf_actor::{ActorId, ActorState, Inventory, InventoryItem, ItemSlot, Vec2};
use cf_equipment::{rifle_preset, RifleState};

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
    #[serde(default)]
    pub objectives: Vec<serde_json::Value>,
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
/// (target practice, friendlies). Chassis-grade actors land in M5.
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
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScenarioInventory {
    /// Optional rifle preset id; resolved against `cf-equipment::rifle_preset`.
    #[serde(default)]
    pub rifle: Option<String>,
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
        source: ron::error::SpannedError,
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
        actor
    }

    pub fn rifle_state(&self) -> Option<RifleState> {
        let preset = self.inventory.rifle.as_deref()?;
        rifle_preset(preset).map(RifleState::new)
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
            source,
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
        assert!(player.rifle_state().is_some());
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
}
