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
        let mut r = Reactor {
            id: self.id.clone(),
            position: [self.position.0, self.position.1],
            half_extents: [self.half_extents.0, self.half_extents.1],
            hp: self.hp.max(0.0),
            max_hp: self.hp.max(0.0),
            destroyed: false,
            ..Reactor::default()
        };
        // engine never has to lazy-init it on the first projectile hit.
        r.ensure_armor_layers();
        r
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

