//! **M15** § Flask containers + throw + drink mechanics.
//!
//! Per the M15 spec § "Flask system (Noita-inspired)":
//! - Carryable container holding 50-200 mL of any liquid
//! - Flask kinds: water, oil, acid, fuel, heal_potion, poison, alchemy_mix
//! - Throw: flask breaks on impact → splash damage + tile coverage with
//!   contained liquid
//! - Drink: consume contents for self-effect (heal_potion = +50 HP;
//!   poison = -50 HP)
//! - Combat use:
//!   - Throw water at fire → extinguish
//!   - Throw acid at enemy → contact damage + tile coverage
//!   - Throw oil → flammable trap (ignite later with fire)
//!
//! Events emitted:
//! - `flask.thrown { flask_id, kind, contents_material, pos, splash_radius, tick }`
//! - `flask.consumed { flask_id, kind, drinker_id, health_delta, tick }`
//!
//! Mass / volume conservation: flask volume → tile pixel count via the
//! `pixels_per_ml` constant (8 pixels/mL). A 100 mL acid flask paints
//! 800 pixels (circle radius ≈ 16 at 1 px/cell).

#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(
    clippy::module_name_repetitions,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::doc_markdown,
    clippy::missing_const_for_fn,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_lossless,
    clippy::struct_field_names,
    clippy::match_same_arms,
    clippy::unused_self
)]

use serde::{Deserialize, Serialize};

use cf_material::MaterialId;
use cf_terrain::chunked::ChunkedTerrain;

/// Locked launch flask kinds. Per spec § "Flask kinds: water flask, oil
/// flask, acid flask, fuel flask, heal_potion flask, poison flask,
/// alchemy_mix flask".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlaskKind {
    Water,
    Oil,
    Acid,
    Fuel,
    HealPotion,
    Poison,
    AlchemyMix,
}

impl FlaskKind {
    pub fn as_str(self) -> &'static str {
        match self {
            FlaskKind::Water => "water",
            FlaskKind::Oil => "oil",
            FlaskKind::Acid => "acid",
            FlaskKind::Fuel => "fuel",
            FlaskKind::HealPotion => "heal_potion",
            FlaskKind::Poison => "poison",
            FlaskKind::AlchemyMix => "alchemy_mix",
        }
    }

    /// Material id painted on impact (the contained liquid). Maps to
    /// `content/materials/material_registry.json` ids:
    ///   - 13=water, 19=oil, 21=acid, 20=fuel, 13=water (heal proxy),
    ///     66=polluted_water (poison proxy), 23=blood (alchemy_mix proxy).
    pub fn contents_material(self) -> MaterialId {
        match self {
            FlaskKind::Water => 13,
            FlaskKind::Oil => 19,
            FlaskKind::Acid => 21,
            FlaskKind::Fuel => 20,
            FlaskKind::HealPotion => 13,
            FlaskKind::Poison => 66,
            FlaskKind::AlchemyMix => 23,
        }
    }

    /// Default content volume in mL when a flask is hand-crafted from
    /// recipe. Player can refill / partial-fill via the flask station;
    /// the launch range is 50-200 mL.
    pub fn default_volume_ml(self) -> f32 {
        match self {
            FlaskKind::Water => 200.0,
            FlaskKind::Oil => 150.0,
            FlaskKind::Acid => 100.0,
            FlaskKind::Fuel => 150.0,
            FlaskKind::HealPotion => 50.0,
            FlaskKind::Poison => 50.0,
            FlaskKind::AlchemyMix => 100.0,
        }
    }

    /// Max volume a flask of this kind can hold. Stays inside the 50-200
    /// mL spec band.
    pub fn max_volume_ml(self) -> f32 {
        200.0
    }
}

/// Pixels of tile coverage produced per mL of contents on impact.
/// 8 px/mL × 100 mL acid = 800 pixels (≈16 radius). Determined per
/// playtest tuning; modders can override via mission config.
pub const PIXELS_PER_ML: f32 = 8.0;

/// A flask in inventory or in the world. `flask_id` is the engine's
/// stable record id (M4 record-id contract).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Flask {
    pub flask_id: u64,
    pub kind: FlaskKind,
    pub volume_ml: f32,
}

impl Flask {
    pub fn new(flask_id: u64, kind: FlaskKind) -> Self {
        Self {
            flask_id,
            kind,
            volume_ml: kind.default_volume_ml(),
        }
    }

    pub fn with_volume(flask_id: u64, kind: FlaskKind, volume_ml: f32) -> Self {
        Self {
            flask_id,
            kind,
            volume_ml: volume_ml.clamp(0.0, kind.max_volume_ml()),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.volume_ml <= 1e-3
    }
}

/// Recorded `flask.thrown` event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FlaskThrownEvent {
    pub flask_id: u64,
    pub thrower_id: u64,
    pub kind: FlaskKind,
    pub contents_material: MaterialId,
    pub impact_pos: [f32; 2],
    pub volume_ml: f32,
    pub splash_radius_px: f32,
    pub splash_pixel_budget: u32,
    pub tick: u64,
}

/// Recorded `flask.consumed` event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FlaskConsumedEvent {
    pub flask_id: u64,
    pub drinker_id: u64,
    pub kind: FlaskKind,
    pub health_delta: f32,
    /// Optional affliction kind (e.g. `"poisoned"`); mods extend via
    /// the M16 affliction registry. `None` for neutral kinds (water).
    pub applied_affliction: Option<String>,
    pub tick: u64,
}

/// Drink outcome for a flask kind. Per spec literal:
/// > Drink: consume contents for self-effect (heal_potion = +50 HP;
/// > poison = -50 HP)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DrinkEffect {
    pub health_delta: f32,
    pub applied_affliction: Option<String>,
}

/// Resolve the drink effect per spec. Modders can override by intercepting
/// the `flask.consumed` event payload.
#[must_use]
pub fn drink_effect(kind: FlaskKind, volume_ml: f32) -> DrinkEffect {
    let scale = volume_ml / 100.0;
    match kind {
        FlaskKind::HealPotion => DrinkEffect {
            health_delta: 50.0 * scale,
            applied_affliction: None,
        },
        FlaskKind::Poison => DrinkEffect {
            health_delta: -50.0 * scale,
            applied_affliction: Some("poisoned".to_string()),
        },
        FlaskKind::Water => DrinkEffect {
            health_delta: 2.0 * scale,
            applied_affliction: None,
        },
        FlaskKind::Acid => DrinkEffect {
            health_delta: -30.0 * scale,
            applied_affliction: Some("acid_burn".to_string()),
        },
        FlaskKind::Oil => DrinkEffect {
            health_delta: -5.0 * scale,
            applied_affliction: Some("nauseous".to_string()),
        },
        FlaskKind::Fuel => DrinkEffect {
            health_delta: -20.0 * scale,
            applied_affliction: Some("nauseous".to_string()),
        },
        FlaskKind::AlchemyMix => DrinkEffect {
            health_delta: 0.0,
            applied_affliction: Some("mixed".to_string()),
        },
    }
}

/// Outcome of a throw attempt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThrowOutcome {
    pub event: FlaskThrownEvent,
    /// Tile-coverage budget in pixels — the engine paints this many
    /// pixels of `contents_material` in a circle around `impact_pos`.
    pub pixel_budget: u32,
    /// Radius the engine should use when painting the splash. Caller
    /// scales for visual taste; this is the deterministic anchor.
    pub splash_radius_px: f32,
}

/// Outcome of a drink attempt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DrinkOutcome {
    pub event: FlaskConsumedEvent,
    pub effect: DrinkEffect,
}

/// Errors when invoking flask actions.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum FlaskActionError {
    #[error("flask is empty")]
    Empty,
}

/// Throw a flask. Consumes its volume and produces a [`ThrowOutcome`].
/// The caller is responsible for routing the resulting `pixel_budget`
/// to the chunked-terrain CA paint call so the splash actually lands.
///
/// On a successful throw, the flask's `volume_ml` is set to 0.
pub fn throw_flask(
    flask: &mut Flask,
    thrower_id: u64,
    impact_pos: [f32; 2],
    tick: u64,
) -> Result<ThrowOutcome, FlaskActionError> {
    if flask.is_empty() {
        return Err(FlaskActionError::Empty);
    }
    let volume = flask.volume_ml;
    let pixel_budget = (volume * PIXELS_PER_ML).max(1.0) as u32;
    // Splash radius from area: budget ≈ π r² → r = sqrt(budget/π).
    let splash_radius_px = ((pixel_budget as f32) / std::f32::consts::PI).sqrt().max(2.0);
    let event = FlaskThrownEvent {
        flask_id: flask.flask_id,
        thrower_id,
        kind: flask.kind,
        contents_material: flask.kind.contents_material(),
        impact_pos,
        volume_ml: volume,
        splash_radius_px,
        splash_pixel_budget: pixel_budget,
        tick,
    };
    flask.volume_ml = 0.0;
    Ok(ThrowOutcome {
        event,
        pixel_budget,
        splash_radius_px,
    })
}

/// **M15** § Paint a flask's splash on the chunked terrain. Called by
/// the engine immediately after [`throw_flask`] to realize the splash
/// pixels per the M15 spec § "flask glue" ownership for cf-material.
///
/// Replaces every non-static pixel within `splash_radius_px` of the
/// impact position with the flask's `contents_material`. Static
/// materials (concrete, metal, anchor) are preserved; only passable
/// pixels (air, liquids, gases) are overwritten.
///
/// Returns the number of pixels painted. The chunked terrain's dirty-
/// region contract is preserved: every write routes through
/// `set_material_pixel` (which marks the chunk dirty), and we call
/// `add_updated_material_area` once at the end for the renderer
/// upload pass.
pub fn paint_splash(terrain: &mut ChunkedTerrain, outcome: &ThrowOutcome, tick: u64) -> u32 {
    let radius = outcome.splash_radius_px.max(1.0);
    let r2 = radius * radius;
    let cx = outcome.event.impact_pos[0];
    let cy = outcome.event.impact_pos[1];
    let min = [cx - radius, cy - radius];
    let max = [cx + radius, cy + radius];
    let x0 = (min[0].floor() as i64).max(0);
    let y0 = (min[1].floor() as i64).max(0);
    let x1 = (max[0].ceil() as i64)
        .max(0)
        .min(terrain.width_px as i64);
    let y1 = (max[1].ceil() as i64)
        .max(0)
        .min(terrain.height_px as i64);
    let contents = outcome.event.contents_material;
    let mut painted: u32 = 0;
    let mut budget = outcome.pixel_budget;
    for py in y0..y1 {
        for px in x0..x1 {
            if budget == 0 {
                break;
            }
            let dx = (px as f32 + 0.5) - cx;
            let dy = (py as f32 + 0.5) - cy;
            if dx * dx + dy * dy > r2 {
                continue;
            }
            let existing = terrain.material_at(px, py);
            if !is_paintable(existing) {
                continue;
            }
            if terrain.set_material_pixel(px, py, contents, tick) {
                painted += 1;
                budget = budget.saturating_sub(1);
            }
        }
    }
    if painted > 0 {
        terrain.add_updated_material_area(min, max);
    }
    painted
}

/// Materials that the flask splash CAN overwrite. Solids that are
/// integral to the structure (concrete, metal, anchor, dirt, etc.)
/// stay; only passable / mobile materials are repainted.
fn is_paintable(id: MaterialId) -> bool {
    use cf_terrain::ca::{ca_movement_class, CaMovementClass};
    matches!(
        ca_movement_class(id),
        CaMovementClass::Air | CaMovementClass::Liquid | CaMovementClass::Gas | CaMovementClass::Powder
    )
}

/// Drink a flask. Consumes its volume and produces a [`DrinkOutcome`].
/// The engine wires `effect.health_delta` into the drinker's HP.
pub fn drink_flask(flask: &mut Flask, drinker_id: u64, tick: u64) -> Result<DrinkOutcome, FlaskActionError> {
    if flask.is_empty() {
        return Err(FlaskActionError::Empty);
    }
    let effect = drink_effect(flask.kind, flask.volume_ml);
    let event = FlaskConsumedEvent {
        flask_id: flask.flask_id,
        drinker_id,
        kind: flask.kind,
        health_delta: effect.health_delta,
        applied_affliction: effect.applied_affliction.clone(),
        tick,
    };
    flask.volume_ml = 0.0;
    Ok(DrinkOutcome { event, effect })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// VAL-M15-flask-001: launch flask kinds cover the 7-kind spec literal.
    #[test]
    fn launch_kinds_match_spec_list() {
        let names: Vec<_> = [
            FlaskKind::Water,
            FlaskKind::Oil,
            FlaskKind::Acid,
            FlaskKind::Fuel,
            FlaskKind::HealPotion,
            FlaskKind::Poison,
            FlaskKind::AlchemyMix,
        ]
        .iter()
        .map(|k| k.as_str())
        .collect();
        assert_eq!(names.len(), 7);
        assert!(names.contains(&"water"));
        assert!(names.contains(&"oil"));
        assert!(names.contains(&"acid"));
        assert!(names.contains(&"fuel"));
        assert!(names.contains(&"heal_potion"));
        assert!(names.contains(&"poison"));
        assert!(names.contains(&"alchemy_mix"));
    }

    /// VAL-M15-flask-002: water flask paints water (material id 13).
    #[test]
    fn water_flask_contents_is_water_material() {
        assert_eq!(FlaskKind::Water.contents_material(), 13);
    }

    /// VAL-M15-flask-003: throw spawns an event with splash radius >= 2.
    #[test]
    fn throw_outcome_has_event_and_paint_budget() {
        let mut flask = Flask::new(7, FlaskKind::Water);
        let outcome = throw_flask(&mut flask, 1, [100.0, 50.0], 42).expect("ok");
        assert_eq!(outcome.event.kind, FlaskKind::Water);
        assert!(outcome.event.splash_radius_px >= 2.0);
        assert!(outcome.event.splash_pixel_budget >= 100);
        assert_eq!(outcome.event.contents_material, 13);
        assert!(flask.is_empty(), "thrown flask must be emptied");
    }

    /// VAL-M15-flask-004: thrown empty flask refuses.
    #[test]
    fn empty_flask_refuses_to_throw() {
        let mut flask = Flask::with_volume(1, FlaskKind::Water, 0.0);
        let res = throw_flask(&mut flask, 1, [0.0, 0.0], 1);
        assert!(matches!(res, Err(FlaskActionError::Empty)));
    }

    /// VAL-M15-flask-005: heal_potion drink restores +50 HP at 100 mL.
    #[test]
    fn heal_potion_drink_restores_50hp_at_100ml() {
        let mut flask = Flask::with_volume(1, FlaskKind::HealPotion, 100.0);
        let outcome = drink_flask(&mut flask, 1, 1).expect("ok");
        assert!((outcome.effect.health_delta - 50.0).abs() < 1e-3);
        assert!(flask.is_empty(), "drunk flask must be emptied");
    }

    /// VAL-M15-flask-006: poison drink deals -50 HP at 100 mL + applies
    /// the "poisoned" affliction.
    #[test]
    fn poison_drink_deals_negative_50hp_and_applies_affliction() {
        let mut flask = Flask::with_volume(1, FlaskKind::Poison, 100.0);
        let outcome = drink_flask(&mut flask, 1, 1).expect("ok");
        assert!((outcome.effect.health_delta + 50.0).abs() < 1e-3);
        assert_eq!(outcome.effect.applied_affliction.as_deref(), Some("poisoned"));
    }

    /// VAL-M15-flask-007: events round-trip via serde.
    #[test]
    fn events_round_trip_via_serde() {
        let mut flask = Flask::new(5, FlaskKind::Acid);
        let outcome = throw_flask(&mut flask, 1, [0.0, 0.0], 10).expect("ok");
        let json = serde_json::to_string(&outcome.event).expect("ser");
        let back: FlaskThrownEvent = serde_json::from_str(&json).expect("de");
        assert_eq!(back, outcome.event);
    }

    /// VAL-M15-flask-008: flask volume clamps to max.
    #[test]
    fn volume_clamps_to_max() {
        let f = Flask::with_volume(1, FlaskKind::Water, 999.0);
        assert!(f.volume_ml <= FlaskKind::Water.max_volume_ml());
    }

    /// VAL-M15-flask-009: default volume sits inside the 50-200 mL band
    /// per spec literal.
    #[test]
    fn default_volume_in_spec_band() {
        for kind in [
            FlaskKind::Water,
            FlaskKind::Oil,
            FlaskKind::Acid,
            FlaskKind::Fuel,
            FlaskKind::HealPotion,
            FlaskKind::Poison,
            FlaskKind::AlchemyMix,
        ] {
            let v = kind.default_volume_ml();
            assert!(v >= 50.0 && v <= 200.0, "{kind:?} default {v} out of 50-200 band");
        }
    }

    /// VAL-M15-flask-010: drink scales with volume — 50 mL heal_potion
    /// restores 25 HP (half the 100 mL anchor).
    #[test]
    fn drink_effect_scales_with_volume() {
        let mut flask = Flask::with_volume(1, FlaskKind::HealPotion, 50.0);
        let outcome = drink_flask(&mut flask, 1, 1).expect("ok");
        assert!((outcome.effect.health_delta - 25.0).abs() < 1e-3);
    }

    /// VAL-M15-flask-011: splash painter overwrites air pixels with the
    /// contents material within the radius.
    #[test]
    fn paint_splash_paints_air_pixels_within_radius() {
        use cf_terrain::chunked::{ChunkedTerrain, MATERIAL_AIR};
        let mut terrain = ChunkedTerrain::new(64, 64, MATERIAL_AIR);
        let mut flask = Flask::new(1, FlaskKind::Water);
        let outcome = throw_flask(&mut flask, 1, [32.0, 32.0], 100).expect("ok");
        let painted = paint_splash(&mut terrain, &outcome, 100);
        assert!(painted > 0, "splash painted at least one pixel");
        // The impact center pixel must be water.
        assert_eq!(terrain.material_at(32, 32), 13, "center pixel is water");
    }

    /// VAL-M15-flask-012: splash painter does NOT overwrite static
    /// solids (concrete, dirt, metal).
    #[test]
    fn paint_splash_preserves_static_solids() {
        use cf_terrain::chunked::{ChunkedTerrain, MATERIAL_AIR, MATERIAL_CONCRETE};
        let mut terrain = ChunkedTerrain::new(64, 64, MATERIAL_AIR);
        terrain.set_material_pixel(32, 32, MATERIAL_CONCRETE, 0);
        let mut flask = Flask::new(1, FlaskKind::Acid);
        let outcome = throw_flask(&mut flask, 1, [32.0, 32.0], 100).expect("ok");
        let _painted = paint_splash(&mut terrain, &outcome, 100);
        assert_eq!(terrain.material_at(32, 32), MATERIAL_CONCRETE, "concrete preserved");
    }

    /// VAL-M15-flask-013: splash painter marks chunks dirty (M3
    /// preservation rule 1).
    #[test]
    fn paint_splash_marks_chunks_dirty() {
        use cf_terrain::chunked::{ChunkedTerrain, MATERIAL_AIR};
        let mut terrain = ChunkedTerrain::new(64, 64, MATERIAL_AIR);
        terrain.clear_dirty();
        let mut flask = Flask::new(1, FlaskKind::Water);
        let outcome = throw_flask(&mut flask, 1, [32.0, 32.0], 100).expect("ok");
        let _ = paint_splash(&mut terrain, &outcome, 100);
        assert!(terrain.dirty_chunk_count() > 0, "splash dirties chunks");
    }
}
