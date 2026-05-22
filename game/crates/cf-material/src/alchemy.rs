//! **M15** § Alchemy recipe registry + station logic.
//!
//! Per spec § "Alchemy table for crafting reactions":
//! - Alchemy station in scenarios (place + interact)
//! - Combine multiple materials → new compound material
//! - Recipes (data-driven):
//!   - Iron + coal + heat → steel
//!   - Water + sulfur + saltpeter → gunpowder
//!   - Lead + radioactive_ore + heat → uranium
//!   - Copper + zinc + heat → brass
//! - Recipe unlocks via M25 progression (research)
//!
//! Events emitted:
//! - `alchemy.recipe_invoked { recipe, station_id, tick }`
//! - `alchemy.recipe_completed { recipe, output, station_id, tick }`

use serde::{Deserialize, Serialize};

use crate::MaterialId;

/// One ingredient input to a recipe. `units` is a per-recipe abstract
/// quantity (e.g. 1 = "one tile of iron"); the M25 progression / mining
/// scope determines what one unit physically maps to.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AlchemyInput {
    pub material: MaterialId,
    pub units: u32,
}

impl AlchemyInput {
    pub const fn new(material: MaterialId, units: u32) -> Self {
        Self { material, units }
    }
}

/// One alchemy recipe. Recipes are data-driven; mods can append.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlchemyRecipe {
    /// Stable id (e.g. `"recipe.steel"`, `"recipe.gunpowder"`).
    pub id: String,
    /// Player-facing label.
    pub display_name: String,
    /// Required input materials. Order does NOT matter for matching — the
    /// matcher compares as a multiset.
    pub inputs: Vec<AlchemyInput>,
    /// Optional heat requirement (Kelvin). The station must be at or
    /// above this temperature before the recipe will fire.
    #[serde(default)]
    pub heat_required_k: Option<f32>,
    /// Output material id.
    pub output: MaterialId,
    /// Output units produced per invocation.
    pub output_units: u32,
    /// Ticks the station spends "crafting" before firing
    /// `alchemy.recipe_completed`. Recipes with `cooldown_ticks == 0`
    /// fire `invoked` + `completed` on the same tick.
    pub cooldown_ticks: u32,
    /// Optional gating: a research-tier or build-point id from M25 that
    /// must be unlocked before the recipe is available.
    #[serde(default)]
    pub unlock_requirement: Option<String>,
}

impl AlchemyRecipe {
    /// True when `available_inputs` (as a multiset) contains every
    /// ingredient with at least the required units AND `temperature_k`
    /// satisfies `heat_required_k`.
    #[must_use]
    pub fn matches(&self, available_inputs: &[AlchemyInput], temperature_k: f32) -> bool {
        if let Some(h) = self.heat_required_k {
            if temperature_k < h {
                return false;
            }
        }
        for req in &self.inputs {
            let total: u32 = available_inputs
                .iter()
                .filter(|x| x.material == req.material)
                .map(|x| x.units)
                .sum();
            if total < req.units {
                return false;
            }
        }
        true
    }
}

/// Alchemy recipe registry. Loaded at scenario start; frozen for the
/// duration of the run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlchemyRegistry {
    pub schema_version: u32,
    pub recipes: Vec<AlchemyRecipe>,
}

impl AlchemyRegistry {
    pub const SCHEMA_VERSION: u32 = 1;

    pub fn new(recipes: Vec<AlchemyRecipe>) -> Self {
        Self {
            schema_version: Self::SCHEMA_VERSION,
            recipes,
        }
    }

    pub fn len(&self) -> usize {
        self.recipes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.recipes.is_empty()
    }

    pub fn by_id(&self, id: &str) -> Option<&AlchemyRecipe> {
        self.recipes.iter().find(|r| r.id == id)
    }

    /// Find the first recipe whose ingredients are satisfied by
    /// `available_inputs` at the given temperature.
    #[must_use]
    pub fn first_match(&self, available_inputs: &[AlchemyInput], temperature_k: f32) -> Option<&AlchemyRecipe> {
        self.recipes.iter().find(|r| r.matches(available_inputs, temperature_k))
    }
}

/// `content/materials/material_registry.json`:
/// - 13=water, 33=coal, 34=ore_iron, 35=ore_gold, 36=ore_copper,
///   37=ore_uranium, 42=salt, 48=gunpowder, 68=iron, 69=steel.
///
/// are iron+coal+heat → steel, water+salt+saltpeter → gunpowder (we
/// stand in ore_uranium/sulfur via salt for the launch set), and the
/// brass + uranium recipes. M25 will gate them.
#[must_use]
pub fn default_alchemy_registry() -> AlchemyRegistry {
    let raw: &[AlchemyRecipe] = &[
        // Iron + coal + heat → steel.
        AlchemyRecipe {
            id: "recipe.steel".to_string(),
            display_name: "Steel".to_string(),
            inputs: vec![AlchemyInput::new(68, 1), AlchemyInput::new(33, 1)],
            heat_required_k: Some(1700.0),
            output: 69,
            output_units: 1,
            cooldown_ticks: 60,
            unlock_requirement: None,
        },
        // Water + salt + sulfur (using ore_uranium id as proxy oxidizer) → gunpowder.
        AlchemyRecipe {
            id: "recipe.gunpowder".to_string(),
            display_name: "Gunpowder".to_string(),
            inputs: vec![
                AlchemyInput::new(13, 1),
                AlchemyInput::new(42, 1),
                AlchemyInput::new(33, 1),
            ],
            heat_required_k: Some(450.0),
            output: 48,
            output_units: 2,
            cooldown_ticks: 90,
            unlock_requirement: Some("research.alchemy_basic".to_string()),
        },
        // Copper + zinc + heat → brass (no zinc material yet, stand in salt as proxy until M15C).
        AlchemyRecipe {
            id: "recipe.brass".to_string(),
            display_name: "Brass".to_string(),
            inputs: vec![AlchemyInput::new(36, 1), AlchemyInput::new(42, 1)],
            heat_required_k: Some(1350.0),
            output: 69,
            output_units: 1,
            cooldown_ticks: 60,
            unlock_requirement: Some("research.alchemy_basic".to_string()),
        },
        // Lead (proxy: ore_uranium) + radioactive_ore + heat → uranium (proxy: iron alloy).
        AlchemyRecipe {
            id: "recipe.uranium".to_string(),
            display_name: "Uranium Slug".to_string(),
            inputs: vec![AlchemyInput::new(37, 2), AlchemyInput::new(33, 1)],
            heat_required_k: Some(2000.0),
            output: 37,
            output_units: 1,
            cooldown_ticks: 180,
            unlock_requirement: Some("research.alchemy_advanced".to_string()),
        },
        // Iron + ore_iron + heat → steel (alternate path; ore-direct).
        AlchemyRecipe {
            id: "recipe.steel_direct".to_string(),
            display_name: "Steel (ore-direct)".to_string(),
            inputs: vec![AlchemyInput::new(34, 1), AlchemyInput::new(33, 2)],
            heat_required_k: Some(1900.0),
            output: 69,
            output_units: 1,
            cooldown_ticks: 120,
            unlock_requirement: None,
        },
    ];
    AlchemyRegistry::new(raw.to_vec())
}

/// fixture; the engine ticks it forward and emits the two events on
/// invoke / complete.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlchemyStation {
    pub station_id: u64,
    pub pos: [f32; 2],
    pub temperature_k: f32,
    pub queued: Option<QueuedInvocation>,
    pub completed: Vec<RecipeCompletion>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueuedInvocation {
    pub recipe_id: String,
    pub remaining_ticks: u32,
    pub started_tick: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeCompletion {
    pub recipe_id: String,
    pub output: MaterialId,
    pub output_units: u32,
    pub completed_tick: u64,
}

impl AlchemyStation {
    pub fn new(station_id: u64, pos: [f32; 2], temperature_k: f32) -> Self {
        Self {
            station_id,
            pos,
            temperature_k,
            queued: None,
            completed: Vec::new(),
        }
    }

    /// True if the station has no in-flight recipe.
    pub fn is_idle(&self) -> bool {
        self.queued.is_none()
    }
}

/// Attempt to invoke a recipe on a station given its current inputs.
/// Returns `Ok(event)` when the invocation queued OR completed; returns
/// `Err(reason)` when the recipe doesn't match or the station is busy.
pub fn try_invoke_recipe(
    station: &mut AlchemyStation,
    registry: &AlchemyRegistry,
    available_inputs: &[AlchemyInput],
    tick: u64,
) -> Result<RecipeInvocation, RecipeInvokeError> {
    if !station.is_idle() {
        return Err(RecipeInvokeError::StationBusy);
    }
    let recipe = registry
        .first_match(available_inputs, station.temperature_k)
        .ok_or(RecipeInvokeError::NoMatch)?;
    if recipe.cooldown_ticks == 0 {
        let completion = RecipeCompletion {
            recipe_id: recipe.id.clone(),
            output: recipe.output,
            output_units: recipe.output_units,
            completed_tick: tick,
        };
        station.completed.push(completion.clone());
        Ok(RecipeInvocation {
            recipe_id: recipe.id.clone(),
            station_id: station.station_id,
            tick,
            completion: Some(completion),
        })
    } else {
        station.queued = Some(QueuedInvocation {
            recipe_id: recipe.id.clone(),
            remaining_ticks: recipe.cooldown_ticks,
            started_tick: tick,
        });
        Ok(RecipeInvocation {
            recipe_id: recipe.id.clone(),
            station_id: station.station_id,
            tick,
            completion: None,
        })
    }
}

/// Step one tick of station cooldown. Returns `Some(completion)` when
/// the recipe just completed.
pub fn step_station(
    station: &mut AlchemyStation,
    registry: &AlchemyRegistry,
    tick: u64,
) -> Option<RecipeCompletion> {
    let queued = station.queued.as_mut()?;
    if queued.remaining_ticks == 0 {
        let recipe_id = queued.recipe_id.clone();
        station.queued = None;
        let recipe = registry.by_id(&recipe_id)?;
        let completion = RecipeCompletion {
            recipe_id,
            output: recipe.output,
            output_units: recipe.output_units,
            completed_tick: tick,
        };
        station.completed.push(completion.clone());
        return Some(completion);
    }
    queued.remaining_ticks = queued.remaining_ticks.saturating_sub(1);
    if queued.remaining_ticks == 0 {
        let recipe_id = queued.recipe_id.clone();
        station.queued = None;
        let recipe = registry.by_id(&recipe_id)?;
        let completion = RecipeCompletion {
            recipe_id,
            output: recipe.output,
            output_units: recipe.output_units,
            completed_tick: tick,
        };
        station.completed.push(completion.clone());
        return Some(completion);
    }
    None
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum RecipeInvokeError {
    #[error("alchemy station is busy with another recipe")]
    StationBusy,
    #[error("no recipe matches the provided inputs at this temperature")]
    NoMatch,
}

/// Returned by [`try_invoke_recipe`]. `completion.is_some()` only when
/// the recipe had cooldown == 0 (instant).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeInvocation {
    pub recipe_id: String,
    pub station_id: u64,
    pub tick: u64,
    pub completion: Option<RecipeCompletion>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// VAL-M15-alchemy-001: launch registry exposes recipe.steel.
    #[test]
    fn steel_recipe_is_present() {
        let r = default_alchemy_registry();
        let recipe = r.by_id("recipe.steel").expect("steel recipe");
        assert_eq!(recipe.output, 69, "steel id");
        assert!(recipe.heat_required_k.unwrap() >= 1500.0);
    }

    /// VAL-M15-alchemy-002: launch registry has 4+ recipes.
    #[test]
    fn launch_registry_has_multiple_recipes() {
        let r = default_alchemy_registry();
        assert!(r.len() >= 4, "expected 4+ recipes, got {}", r.len());
    }

    /// VAL-M15-alchemy-003: invoking with insufficient heat fails.
    #[test]
    fn insufficient_heat_rejects() {
        let r = default_alchemy_registry();
        let mut station = AlchemyStation::new(1, [0.0, 0.0], 300.0);
        let inputs = vec![AlchemyInput::new(68, 1), AlchemyInput::new(33, 1)];
        let res = try_invoke_recipe(&mut station, &r, &inputs, 1);
        assert!(matches!(res, Err(RecipeInvokeError::NoMatch)));
    }

    /// VAL-M15-alchemy-004: invoking with sufficient heat queues + steps complete.
    #[test]
    fn invoke_queues_and_completes_after_cooldown() {
        let r = default_alchemy_registry();
        let mut station = AlchemyStation::new(1, [0.0, 0.0], 1800.0);
        let inputs = vec![AlchemyInput::new(68, 1), AlchemyInput::new(33, 1)];
        let invoke = try_invoke_recipe(&mut station, &r, &inputs, 1).expect("ok");
        assert!(invoke.completion.is_none(), "should queue not instantly complete");
        assert!(!station.is_idle());
        // Step 60 ticks; should complete.
        let mut completion = None;
        for t in 2..70 {
            if let Some(c) = step_station(&mut station, &r, t) {
                completion = Some(c);
                break;
            }
        }
        let c = completion.expect("recipe completed");
        assert_eq!(c.output, 69);
        assert!(station.is_idle());
    }

    /// VAL-M15-alchemy-005: cannot start a second recipe while the
    /// station is busy.
    #[test]
    fn station_busy_rejects_second_invoke() {
        let r = default_alchemy_registry();
        let mut station = AlchemyStation::new(1, [0.0, 0.0], 1800.0);
        let inputs = vec![AlchemyInput::new(68, 1), AlchemyInput::new(33, 1)];
        try_invoke_recipe(&mut station, &r, &inputs, 1).expect("ok");
        let again = try_invoke_recipe(&mut station, &r, &inputs, 2);
        assert!(matches!(again, Err(RecipeInvokeError::StationBusy)));
    }

    /// VAL-M15-alchemy-006: recipe must match all required inputs.
    #[test]
    fn missing_input_rejects() {
        let r = default_alchemy_registry();
        let mut station = AlchemyStation::new(1, [0.0, 0.0], 1800.0);
        let inputs = vec![AlchemyInput::new(68, 1)]; // missing coal
        let res = try_invoke_recipe(&mut station, &r, &inputs, 1);
        assert!(matches!(res, Err(RecipeInvokeError::NoMatch)));
    }

    /// VAL-M15-alchemy-007: registry round-trips via serde.
    #[test]
    fn registry_round_trips() {
        let r = default_alchemy_registry();
        let json = serde_json::to_string(&r).expect("ser");
        let back: AlchemyRegistry = serde_json::from_str(&json).expect("de");
        assert_eq!(back.len(), r.len());
    }

    /// VAL-M15-alchemy-008: recipe.matches respects the temperature gate
    /// and the input multiset.
    #[test]
    fn recipe_matches_evaluates_inputs_and_temp() {
        let r = default_alchemy_registry();
        let recipe = r.by_id("recipe.gunpowder").expect("present");
        let good_inputs = vec![
            AlchemyInput::new(13, 1),
            AlchemyInput::new(42, 1),
            AlchemyInput::new(33, 1),
        ];
        assert!(recipe.matches(&good_inputs, 500.0));
        assert!(!recipe.matches(&good_inputs, 100.0));
        let bad_inputs = vec![AlchemyInput::new(13, 1), AlchemyInput::new(42, 1)];
        assert!(!recipe.matches(&bad_inputs, 500.0));
    }
}
