//! cf-material — canonical material registry.
//!
//! M2 ships:
//! - [`MaterialDef`] — full per-material data record (identity, physical,
//!   affordances, interaction, hazard, visual, path, future M5.6 fields).
//! - [`MaterialRegistry`] — loaded from
//!   `content/materials/material_registry.json` (schema_version=1).
//! - JSON loader + validator with structured errors so `cf-mod validate
//!   content/materials/` can reject unknown fields, missing required
//!   fields, duplicate ids, and schema-version mismatches.
//!
//! The DR-007 launch set has 8 materials (`air, dirt, concrete, metal_nohook,
//! hazard, loose_fill, repair_fill, anchor`); M5.6 expands to the active
//! material kernel using the `serde(default)` future-compat fields.
//!
//! Runtime affordance lookup (per-pixel: solid, diggable, hardness, etc.)
//! lives in `cf-terrain` so the chunked terrain doesn't pull `serde_json`
//! into its hot path; this crate is the data-driven source of truth for
//! validators, modding tools, and `cfctl inspect.material.<id>`.

#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(
    clippy::module_name_repetitions,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::doc_markdown,
    clippy::missing_const_for_fn,
    clippy::struct_excessive_bools,
    clippy::cast_precision_loss,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::manual_find,
    clippy::manual_let_else,
    clippy::single_match_else,
    clippy::redundant_closure,
    clippy::redundant_closure_for_method_calls,
    clippy::format_in_format_args,
    clippy::useless_format,
    clippy::too_many_lines,
    clippy::too_many_arguments,
    clippy::similar_names,
    clippy::new_without_default
)]

use std::{collections::BTreeMap, path::Path};

use serde::{Deserialize, Serialize};

pub mod alchemy;
pub mod kernel;
pub mod loader;
pub mod phase;
pub mod precipitation;
pub mod reactions;
pub mod registry;
pub mod thermal_sources;

pub use thermal_sources::{ThermalSource, ThermalSourceLoadError, ThermalSourceTable};

pub use alchemy::{
    default_alchemy_registry, step_station, try_invoke_recipe, AlchemyInput, AlchemyRecipe, AlchemyRegistry,
    AlchemyStation, QueuedInvocation, RecipeCompletion, RecipeInvocation, RecipeInvokeError,
};
pub use kernel::{
    kernel_step, kernel_step_no_movement, KernelStepReport, MaterialKernel, SLEEP_IDLE_THRESHOLD_TICKS,
};
pub use loader::{load_registry_from_file, validate_registry_json, RegistryValidationError, RegistryValidationReport};
pub use phase::{
    default_phase_registry, phase_transition_event, PhaseDirection, PhaseRegistry, PhaseState, PhaseTransition,
    PhaseTransitionEvent,
};
pub use precipitation::{
    evaluate_steam_nucleation, material_id_to_name, pressure_rate_multiplier, saturation_rate_per_tick,
    saturation_rate_per_tick_with_pressure, update_cloud_cell, update_cloud_cell_with_pressure, AmbientWorld,
    CloudCell, PhaseNucleatedEvent, PrecipitationConfig, PrecipitationConfigLoadError, PrecipitationCycle,
    PrecipitationInputs, PrecipitationStartedEvent, ACID_RAIN_POLLUTANT_FRACTION_MIN, NUCLEATION_ALTITUDE_PX,
    NUCLEATION_PRESSURE_MIN_KPA, NUCLEATION_TEMP_K_MAX, PRECIPITATION_SATURATION_THRESHOLD, PRECIPITATION_TICK_GATE,
    PRESSURE_MULTIPLIER_RANGE, REFERENCE_PRESSURE_KPA,
};
pub use reactions::{
    classify_reaction, default_reaction_registry, evaluate_reaction_pair, reaction_event,
    reaction_event_with_emissions, MaterialReaction, ReactionRegistry, ReactionTriggeredEvent, ReactionWoundEmit,
    EMISSION_DROPPED,
};

/// Material schema version stamped into every registry JSON file. M2 ships
/// v1; M5.6 will bump when the active material kernel starts reading the
/// future-compat fields.
pub const MATERIAL_SCHEMA_VERSION: u32 = 1;

/// One pixel's material id. Re-exported via cf-terrain so consumers use the
/// same type.
pub type MaterialId = u8;

/// Required canonical material ids in the DR-007 launch set. The registry
/// loader enforces these IDs are present and that no others sneak in past
/// schema v1.
pub const LAUNCH_MATERIAL_IDS: &[MaterialId] = &[0, 1, 2, 3, 4, 5, 6, 7];

/// Canonical material name per id. Used by the loader to enforce the
/// DR-007 launch order (id=0 → "air", id=1 → "dirt", etc.).
pub const LAUNCH_MATERIAL_NAMES: &[(MaterialId, &str)] = &[
    (0, "air"),
    (1, "dirt"),
    (2, "concrete"),
    (3, "metal_nohook"),
    (4, "hazard"),
    (5, "loose_fill"),
    (6, "repair_fill"),
    (7, "anchor"),
];

/// Full MaterialDef record. Mirrors the M2 spec's "MaterialDef with identity,
/// physical, affordances, interaction, hazard, visual, path, future" sections.
///
/// `serde(default)` on the future-compat fields lets M5.6 add new kernel
/// behaviors without bumping schema_version.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaterialDef {
    /// Stable id (0..=255). Reserved range 0..=7 = DR-007 launch set.
    pub id: MaterialId,
    /// Snake-case name (`"air"`, `"metal_nohook"`).
    pub name: String,
    /// Player-facing label used by HUD legend + tooltips.
    pub display_name: String,
    /// Per-pixel integrity. Drives carve cost AND projectile penetration
    /// formula (`impulse² > integrity²` per CCCP `SceneMan.cpp:571`).
    pub hardness: f32,
    /// Digger tool can carve through.
    pub diggable: bool,
    /// Anchor / tether tool can attach.
    pub anchorable: bool,
    /// Damages actors on contact when true.
    ///
    /// M3 audit pass 7 (2026-05-13): spec literal field name is
    /// `damage_on_touch`; `hazard` retained as the canonical Rust
    /// identifier with `damage_on_touch` accepted as a serde alias so
    /// scenario manifests authored against the spec literal deserialize
    /// cleanly.
    #[serde(alias = "damage_on_touch")]
    pub hazard: bool,
    /// AI path-cost contribution (1.0 baseline; > 1.0 expensive; >= 999 impassable).
    pub path_cost: f32,
    /// Density in kg / pixel — drives spawn-debris mass.
    pub density: f32,
    /// Hex color (RGB) for the HUD legend + integrity overlay.
    pub color_hex: String,
    /// Short human-readable origin reference (CCCP / OpenLieroX / spec
    /// citation). Helps future tuners trace the value.
    pub description: String,

    // --- M2 spec extension: explicit affordance flags (forward-compat with OpenLieroX taxonomy)
    #[serde(default)]
    pub drillable: Option<bool>,
    #[serde(default)]
    pub blastable: Option<bool>,
    #[serde(default)]
    pub beam_cuttable: Option<bool>,
    #[serde(default)]
    pub projectile_passable: Option<bool>,
    #[serde(default)]
    pub actor_passable: Option<bool>,
    #[serde(default)]
    pub blocks_line_of_sight: Option<bool>,
    #[serde(default)]
    pub damage_per_tick: Option<f32>,
    #[serde(default)]
    pub damage_kind: Option<String>,
    #[serde(default)]
    pub stickiness: Option<f32>,
    #[serde(default)]
    pub restitution: Option<f32>,
    #[serde(default)]
    pub friction: Option<f32>,
    #[serde(default)]
    pub structural_integrity: Option<f32>,
    #[serde(default)]
    pub priority: Option<i32>,
    #[serde(default)]
    pub piling: Option<bool>,
    /// **M14E** § Per-material structural-support strength. Drives the
    /// per-chunk integrity field's "locked" baseline: integrity at any
    /// pixel anchored to a load-bearing material is at least this value
    /// (default 0 = no support; concrete=50; reinforced=200; support_beam=500).
    /// Missing entries fall back to 0 via [`structural_support_strength_for`].
    #[serde(default)]
    pub structural_support_strength: Option<u16>,
    /// **M14F** § Per-material lateral yield strength. Drives the
    /// lateral integrity field's bulge/crack/rupture cascade — the
    /// 90°-rotated sibling of `structural_support_strength`. Spec
    /// literal: concrete=50, brick=30, steel=200, wood=15, dirt=10;
    /// default falls back to the material's compressive strength via
    /// [`lateral_yield_strength_for`].
    #[serde(default)]
    pub lateral_yield_strength: Option<u16>,
    #[serde(default)]
    pub settle_material: Option<MaterialId>,
    #[serde(default)]
    pub spawn_material: Option<MaterialId>,
    #[serde(default)]
    pub is_scrap: Option<bool>,
    #[serde(default)]
    pub uses_own_color: Option<bool>,
    #[serde(default)]
    pub ui_overlay_color: Option<String>,
    #[serde(default)]
    pub render_priority: Option<i32>,

    // --- M5.6 future-compat fields (active material kernel + reaction table)
    #[serde(default)]
    pub heat_capacity: Option<f32>,
    #[serde(default)]
    pub thermal_conductivity: Option<f32>,
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub ignition_temperature: Option<f32>,
    #[serde(default)]
    pub burn_rate: Option<f32>,
    #[serde(default)]
    pub oxygen_requirement: Option<f32>,
    #[serde(default)]
    pub burn_products: Vec<MaterialId>,
    #[serde(default)]
    pub phase_changes: Vec<PhaseChange>,
    #[serde(default)]
    pub conductivity: Option<f32>,
    #[serde(default)]
    pub wetting: Option<f32>,
    #[serde(default)]
    pub reaction_tags: Vec<String>,
    #[serde(default)]
    pub ai_affordances: Vec<String>,

    // --- M12B acoustic registry fields. Modders extend by adding rows with
    // these four fields; the cf-audio backend reads from cf-material::registry
    // only. Missing fields fall back to the canonical "dirt" row (see
    // [`AcousticDefaults`]). Per M12B spec § "Per-material acoustic registry".
    #[serde(default)]
    pub echo_coefficient: Option<f32>,
    #[serde(default)]
    pub decay_band: Option<String>,
    #[serde(default)]
    pub acoustic_transmission_loss_db: Option<f32>,
    #[serde(default)]
    pub low_pass_cutoff_hz: Option<f32>,
}

/// **M12B** § Canonical per-material acoustic fallback. When a material row
/// is missing any of the four acoustic fields, lookups return these defaults
/// (the "default dirt" row) so the audio backend always has a deterministic
/// number to plug into the HRTF / reverb / occlusion / low-pass pipeline.
///
/// Per M12B spec § Notes:
/// > `(echo_coefficient=0.2, decay_band=warm_mid, transmission_loss_db=8.0,
/// > low_pass_cutoff_hz=2000)` (the "default dirt" row). Log via
/// > `tracing::warn!` once per missing material id.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AcousticDefaults;

impl AcousticDefaults {
    /// Default echo coefficient (`0.2`).
    pub const ECHO_COEFFICIENT: f32 = 0.2;
    /// Default decay band (`warm_mid`).
    pub const DECAY_BAND: &'static str = "warm_mid";
    /// Default acoustic transmission loss per wall in decibels (`8.0`).
    pub const TRANSMISSION_LOSS_DB: f32 = 8.0;
    /// Default low-pass cutoff in hertz (`2000`).
    pub const LOW_PASS_CUTOFF_HZ: f32 = 2000.0;
}

/// **M12B** § Resolved per-material acoustic profile. Returned by
/// [`MaterialDef::acoustic_profile`] / [`MaterialRegistry::acoustic_for`]. All
/// four fields are guaranteed populated — missing data falls back to the
/// [`AcousticDefaults`] row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AcousticProfile {
    /// 0..=1 — first-reflection amplitude on this surface.
    pub echo_coefficient: f32,
    /// Spectral tilt label — `bright`, `bright_ringing`, `warm_mid`,
    /// `warm_low`, `dampened`, `anechoic`, `bright_short`.
    pub decay_band: &'static str,
    /// Per-wall transmission loss in dB (positive number, applied as
    /// attenuation).
    pub transmission_loss_db: f32,
    /// Low-pass cutoff frequency in Hz for sounds traveling through this
    /// material.
    pub low_pass_cutoff_hz: f32,
}

impl AcousticProfile {
    /// The canonical "default dirt" fallback row.
    #[must_use]
    pub const fn fallback() -> Self {
        Self {
            echo_coefficient: AcousticDefaults::ECHO_COEFFICIENT,
            decay_band: AcousticDefaults::DECAY_BAND,
            transmission_loss_db: AcousticDefaults::TRANSMISSION_LOSS_DB,
            low_pass_cutoff_hz: AcousticDefaults::LOW_PASS_CUTOFF_HZ,
        }
    }

    /// Canonical decay-band label table. Returns the stable `&'static str`
    /// matching the spec-locked vocabulary; unknown bands fall back to
    /// [`AcousticDefaults::DECAY_BAND`].
    #[must_use]
    pub fn canonical_band(s: &str) -> &'static str {
        match s {
            "bright" => "bright",
            "bright_ringing" => "bright_ringing",
            "bright_short" => "bright_short",
            "warm_mid" => "warm_mid",
            "warm_low" => "warm_low",
            "dampened" => "dampened",
            "anechoic" => "anechoic",
            _ => AcousticDefaults::DECAY_BAND,
        }
    }
}

/// **M14E** § Canonical per-material `structural_support_strength` for the
/// load-bearing baseline that the per-chunk integrity field uses to lock
/// anchored pixels. Returns the spec-baked values when the material name
/// is recognized; falls back to `0` for unsupported materials.
///
/// Per spec literal:
/// > Add per-material `structural_support_strength` field (default 0;
/// > concrete = 50; reinforced = 200; support_beam = 500).
#[must_use]
pub fn structural_support_strength_for(material_name: &str) -> u16 {
    match material_name {
        "concrete" | "concrete_soft" => 50,
        "reinforced" => 200,
        "support_beam" => 500,
        _ => 0,
    }
}

/// **M14F** § Canonical per-material `lateral_yield_strength` for the
/// lateral wall collapse pass. Returns the spec-baked values when the
/// material name is recognized; otherwise returns `None` so callers can
/// fall back to the material's compressive strength
/// (i.e. [`structural_support_strength_for`] per VAL-M14F-021).
///
/// Per spec literal:
/// > Per-material `lateral_yield_strength` = approximate Stationeers
/// > values: concrete 50, brick 30, steel 200, wood 15, dirt 10
/// > (default = same as compressive for now).
#[must_use]
pub fn lateral_yield_strength_for(material_name: &str) -> Option<u16> {
    match material_name {
        "concrete" | "concrete_soft" => Some(50),
        "brick" => Some(30),
        "steel" | "metal_nohook" => Some(200),
        "wood" => Some(15),
        "dirt" => Some(10),
        _ => None,
    }
}

/// **M14F** § Effective lateral yield strength for a material identified
/// only by name (no full `MaterialDef`). Returns the spec-baked value
/// when known, otherwise falls back to compressive strength
/// ([`structural_support_strength_for`]). Per VAL-M14F-021.
#[must_use]
pub fn lateral_yield_strength_value_for(material_name: &str) -> u16 {
    lateral_yield_strength_for(material_name)
        .unwrap_or_else(|| structural_support_strength_for(material_name))
}

impl MaterialDef {
    /// **M14E** § Resolved structural-support strength for this material.
    /// Uses the explicit registry field when set; otherwise falls back to
    /// [`structural_support_strength_for`].
    #[must_use]
    pub fn structural_support_strength_value(&self) -> u16 {
        self.structural_support_strength
            .unwrap_or_else(|| structural_support_strength_for(self.name.as_str()))
    }

    /// **M14F** § Compressive strength for this material — alias for
    /// [`structural_support_strength_value`]. Surfaced under the
    /// "compressive" name so the lateral-yield default path
    /// (VAL-M14F-021) reads as the spec says: "default = same as
    /// compressive for now".
    #[must_use]
    pub fn compressive_strength(&self) -> u16 {
        self.structural_support_strength_value()
    }

    /// **M14F** § Resolved lateral yield strength for this material.
    /// Resolution order (per VAL-M14F-015 + VAL-M14F-021):
    ///   1. Explicit `lateral_yield_strength` field on the registry row.
    ///   2. Spec-baked override via [`lateral_yield_strength_for`].
    ///   3. Fallback to [`Self::compressive_strength`].
    #[must_use]
    pub fn lateral_yield_strength_value(&self) -> u16 {
        if let Some(v) = self.lateral_yield_strength {
            return v;
        }
        if let Some(v) = lateral_yield_strength_for(self.name.as_str()) {
            return v;
        }
        self.compressive_strength()
    }

    /// **M12B** § Resolved acoustic profile for this material. Missing
    /// fields fall back to [`AcousticProfile::fallback`].
    ///
    /// Per M12B spec § Notes for the implementer: "missing acoustic
    /// fields fall back to `(echo_coefficient=0.2, decay_band=warm_mid,
    /// transmission_loss_db=8.0, low_pass_cutoff_hz=2000)` (the 'default
    /// dirt' row). Log via `tracing::warn!` once per missing material
    /// id.". The once-per-id throttle lives in [`warn_missing_acoustic`].
    #[must_use]
    pub fn acoustic_profile(&self) -> AcousticProfile {
        let fb = AcousticProfile::fallback();
        if self.echo_coefficient.is_none()
            || self.decay_band.is_none()
            || self.acoustic_transmission_loss_db.is_none()
            || self.low_pass_cutoff_hz.is_none()
        {
            warn_missing_acoustic(self.id, self.name.as_str());
        }
        AcousticProfile {
            echo_coefficient: self
                .echo_coefficient
                .unwrap_or(fb.echo_coefficient)
                .clamp(0.0, 1.0),
            decay_band: self
                .decay_band
                .as_deref()
                .map_or(fb.decay_band, AcousticProfile::canonical_band),
            transmission_loss_db: self.acoustic_transmission_loss_db.unwrap_or(fb.transmission_loss_db).max(0.0),
            low_pass_cutoff_hz: self.low_pass_cutoff_hz.unwrap_or(fb.low_pass_cutoff_hz).max(20.0),
        }
    }
}

/// **M12B** § Once-per-material warning helper. Tracks seen ids so the
/// log spam is bounded — the warning fires exactly once per missing
/// material id (per process). Test-only `reset_acoustic_warning_cache`
/// drains the state between tests.
fn warn_missing_acoustic(id: MaterialId, name: &str) {
    use std::sync::Mutex;
    static SEEN: std::sync::OnceLock<Mutex<std::collections::BTreeSet<MaterialId>>> = std::sync::OnceLock::new();
    let seen = SEEN.get_or_init(|| Mutex::new(std::collections::BTreeSet::new()));
    if let Ok(mut set) = seen.lock() {
        if set.insert(id) {
            tracing::warn!(
                target = "cf-material::acoustic",
                material_id = id,
                material_name = name,
                "missing one or more M12B acoustic fields; falling back to default-dirt acoustic row"
            );
        }
    }
}

/// Phase-change rule for the M5.6 active material kernel. `From → To` when
/// the local temperature crosses `threshold_kelvin`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhaseChange {
    pub from_state: String,
    pub to_state: String,
    pub threshold_kelvin: f32,
    #[serde(default)]
    pub product_material: Option<MaterialId>,
}

/// The canonical material registry. Loaded once from JSON at scenario start
/// and frozen for the duration of the run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaterialRegistry {
    pub schema_version: u32,
    #[serde(default)]
    pub description: String,
    pub materials: Vec<MaterialDef>,
}

impl MaterialRegistry {
    pub fn find_by_id(&self, id: MaterialId) -> Option<&MaterialDef> {
        self.materials.iter().find(|m| m.id == id)
    }

    pub fn find_by_name(&self, name: &str) -> Option<&MaterialDef> {
        self.materials.iter().find(|m| m.name == name)
    }

    /// Run the v1 validator over the loaded registry. Returns every error
    /// found (caller can decide whether any error is fatal).
    pub fn validate(&self) -> Vec<RegistryValidationError> {
        loader::validate_registry(self)
    }

    /// Default registry path relative to the game working dir.
    pub fn default_path() -> &'static Path {
        Path::new("content/materials/material_registry.json")
    }

    /// Resolve the registry path: prefer `content/materials/...` then
    /// `../content/materials/...` (so `cargo run` from `game/` and from
    /// the repo root both work).
    pub fn locate_default() -> Option<std::path::PathBuf> {
        for candidate in [
            std::path::PathBuf::from("content/materials/material_registry.json"),
            std::path::PathBuf::from("../content/materials/material_registry.json"),
            std::path::PathBuf::from("game/content/materials/material_registry.json"),
        ] {
            if candidate.exists() {
                return Some(candidate);
            }
        }
        None
    }

    /// Material-name lookup map for fast `name → id` resolution.
    pub fn name_to_id(&self) -> BTreeMap<String, MaterialId> {
        self.materials.iter().map(|m| (m.name.clone(), m.id)).collect()
    }

    /// **M12B** § Acoustic profile lookup by material id. Returns the
    /// resolved [`AcousticProfile`] (with fallbacks applied per
    /// [`AcousticDefaults`]). Returns `AcousticProfile::fallback()` for
    /// unknown ids so audio resolution never panics on a missing material.
    #[must_use]
    pub fn acoustic_for(&self, id: MaterialId) -> AcousticProfile {
        match self.find_by_id(id) {
            Some(m) => m.acoustic_profile(),
            None => AcousticProfile::fallback(),
        }
    }

    /// **M12B** § Acoustic profile lookup by material name (snake_case).
    /// Returns the canonical fallback when the name is unknown.
    #[must_use]
    pub fn acoustic_for_name(&self, name: &str) -> AcousticProfile {
        match self.find_by_name(name) {
            Some(m) => m.acoustic_profile(),
            None => AcousticProfile::fallback(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry_json() -> serde_json::Value {
        serde_json::json!({
            "schema_version": 1,
            "description": "test",
            "materials": [
                {
                    "id": 0,
                    "name": "air",
                    "display_name": "Air",
                    "hardness": 0.0,
                    "diggable": false,
                    "anchorable": false,
                    "hazard": false,
                    "path_cost": 1.0,
                    "density": 0.0,
                    "color_hex": "000000",
                    "description": "empty"
                }
            ]
        })
    }

    #[test]
    fn parses_minimal_registry() {
        let v = registry_json();
        let r: MaterialRegistry = serde_json::from_value(v).expect("parse");
        assert_eq!(r.schema_version, 1);
        assert_eq!(r.materials.len(), 1);
        assert_eq!(r.materials[0].name, "air");
    }

    #[test]
    fn round_trip_preserves_future_compat_fields() {
        let mut v = registry_json();
        let materials = v["materials"].as_array_mut().unwrap();
        materials[0]["heat_capacity"] = serde_json::json!(1005.0);
        materials[0]["ignition_temperature"] = serde_json::json!(900.0);
        let r: MaterialRegistry = serde_json::from_value(v.clone()).expect("parse");
        assert_eq!(r.materials[0].heat_capacity, Some(1005.0));
        assert_eq!(r.materials[0].ignition_temperature, Some(900.0));
        let back = serde_json::to_value(&r).unwrap();
        assert!(back.to_string().contains("heat_capacity"));
    }

    #[test]
    fn rejects_unknown_field() {
        let mut v = registry_json();
        v["materials"][0]["rainbow_color"] = serde_json::json!("red");
        let res: Result<MaterialRegistry, _> = serde_json::from_value(v);
        assert!(res.is_err());
    }

    // M12B § Per-material acoustic registry.

    #[test]
    fn acoustic_profile_defaults_to_fallback_when_fields_missing() {
        let v = registry_json();
        let r: MaterialRegistry = serde_json::from_value(v).expect("parse");
        let profile = r.materials[0].acoustic_profile();
        let fb = AcousticProfile::fallback();
        assert!((profile.echo_coefficient - fb.echo_coefficient).abs() < 1e-6);
        assert_eq!(profile.decay_band, fb.decay_band);
        assert!((profile.transmission_loss_db - fb.transmission_loss_db).abs() < 1e-6);
        assert!((profile.low_pass_cutoff_hz - fb.low_pass_cutoff_hz).abs() < 1e-6);
    }

    #[test]
    fn acoustic_profile_uses_supplied_fields() {
        let mut v = registry_json();
        v["materials"][0]["echo_coefficient"] = serde_json::json!(0.85);
        v["materials"][0]["decay_band"] = serde_json::json!("bright");
        v["materials"][0]["acoustic_transmission_loss_db"] = serde_json::json!(28.0);
        v["materials"][0]["low_pass_cutoff_hz"] = serde_json::json!(800);
        let r: MaterialRegistry = serde_json::from_value(v).expect("parse");
        let profile = r.materials[0].acoustic_profile();
        assert!((profile.echo_coefficient - 0.85).abs() < 1e-6);
        assert_eq!(profile.decay_band, "bright");
        assert!((profile.transmission_loss_db - 28.0).abs() < 1e-6);
        assert!((profile.low_pass_cutoff_hz - 800.0).abs() < 1e-6);
    }

    #[test]
    fn acoustic_profile_clamps_echo_coefficient() {
        let mut v = registry_json();
        v["materials"][0]["echo_coefficient"] = serde_json::json!(5.0);
        let r: MaterialRegistry = serde_json::from_value(v).expect("parse");
        assert!((r.materials[0].acoustic_profile().echo_coefficient - 1.0).abs() < 1e-6);
    }

    #[test]
    fn acoustic_profile_canonicalises_unknown_decay_band() {
        let mut v = registry_json();
        v["materials"][0]["decay_band"] = serde_json::json!("garbage_band");
        let r: MaterialRegistry = serde_json::from_value(v).expect("parse");
        assert_eq!(
            r.materials[0].acoustic_profile().decay_band,
            AcousticDefaults::DECAY_BAND
        );
    }

    #[test]
    fn acoustic_for_unknown_id_returns_fallback() {
        let v = registry_json();
        let r: MaterialRegistry = serde_json::from_value(v).expect("parse");
        let prof = r.acoustic_for(255);
        assert_eq!(prof, AcousticProfile::fallback());
    }

    #[test]
    fn acoustic_for_name_unknown_returns_fallback() {
        let v = registry_json();
        let r: MaterialRegistry = serde_json::from_value(v).expect("parse");
        let prof = r.acoustic_for_name("not_a_material");
        assert_eq!(prof, AcousticProfile::fallback());
    }

    #[test]
    fn registry_module_reexports_canonical_types() {
        // Per M12B spec § Files: `cf-material/src/registry.rs` (MODIFY).
        // The canonical types live in lib.rs for backward-compat; the
        // registry module re-exports them so `cf_material::registry::*`
        // resolves cleanly.
        use crate::registry::{
            AcousticDefaults, AcousticProfile, MaterialDef, MaterialId, MaterialRegistry, PhaseChange,
            MATERIAL_SCHEMA_VERSION,
        };
        // Just smoke-test the imports resolve and one type compiles.
        assert_eq!(MATERIAL_SCHEMA_VERSION, 1);
        let fb = AcousticProfile::fallback();
        assert!((fb.echo_coefficient - AcousticDefaults::ECHO_COEFFICIENT).abs() < 1e-6);
        let _: Option<MaterialDef> = None;
        let _: Option<MaterialRegistry> = None;
        let _: Option<PhaseChange> = None;
        let _: MaterialId = 0;
    }

    /// VAL-M14E-011: per-material structural_support_strength baseline
    /// per spec (default 0 / concrete=50 / reinforced=200 / support_beam=500).
    #[test]
    fn structural_support_strength_baseline_matches_spec() {
        assert_eq!(structural_support_strength_for("air"), 0);
        assert_eq!(structural_support_strength_for("dirt"), 0);
        assert_eq!(structural_support_strength_for("concrete"), 50);
        assert_eq!(structural_support_strength_for("concrete_soft"), 50);
        assert_eq!(structural_support_strength_for("reinforced"), 200);
        assert_eq!(structural_support_strength_for("support_beam"), 500);
        assert_eq!(structural_support_strength_for("unknown_alloy"), 0);
    }

    /// VAL-M14E-011: MaterialDef may carry an explicit override which wins
    /// over the canonical baseline.
    #[test]
    fn material_def_structural_support_strength_uses_override() {
        let mut v = registry_json();
        v["materials"][0]["structural_support_strength"] = serde_json::json!(123);
        let r: MaterialRegistry = serde_json::from_value(v).expect("parse");
        assert_eq!(r.materials[0].structural_support_strength_value(), 123);
    }

    #[test]
    fn material_def_structural_support_strength_falls_back_to_name_table() {
        let mut v = registry_json();
        v["materials"][0]["name"] = serde_json::json!("concrete");
        let r: MaterialRegistry = serde_json::from_value(v).expect("parse");
        assert_eq!(r.materials[0].structural_support_strength_value(), 50);
    }

    /// VAL-M14F-015: per-material lateral_yield_strength baseline matches
    /// the spec literal values (concrete=50, brick=30, steel=200, wood=15,
    /// dirt=10).
    #[test]
    fn lateral_yield_strength_baseline_matches_spec() {
        assert_eq!(lateral_yield_strength_for("concrete"), Some(50));
        assert_eq!(lateral_yield_strength_for("concrete_soft"), Some(50));
        assert_eq!(lateral_yield_strength_for("brick"), Some(30));
        assert_eq!(lateral_yield_strength_for("steel"), Some(200));
        assert_eq!(lateral_yield_strength_for("metal_nohook"), Some(200));
        assert_eq!(lateral_yield_strength_for("wood"), Some(15));
        assert_eq!(lateral_yield_strength_for("dirt"), Some(10));
        assert_eq!(lateral_yield_strength_for("air"), None);
        assert_eq!(lateral_yield_strength_for("unknown_alloy"), None);
    }

    /// VAL-M14F-021: default lateral_yield_strength for materials without
    /// an explicit override equals the material's compressive strength.
    #[test]
    fn lateral_yield_strength_defaults_to_compressive() {
        let v = registry_json();
        let r: MaterialRegistry = serde_json::from_value(v).expect("parse");
        let air = &r.materials[0];
        assert_eq!(
            air.lateral_yield_strength_value(),
            air.compressive_strength(),
            "default lateral_yield_strength must equal compressive_strength"
        );
        assert_eq!(lateral_yield_strength_value_for("unknown_material"), 0);
        assert_eq!(lateral_yield_strength_value_for("concrete"), 50);
        // Confirms an unmapped material with no override returns the
        // compressive baseline.
        assert_eq!(lateral_yield_strength_value_for("air"), 0);
    }

    /// VAL-CROSS-025: cf-material exposes BOTH structural_support_strength
    /// (M14E) and lateral_yield_strength (M14F) independently — no field
    /// shadowing.
    #[test]
    fn structural_support_and_lateral_yield_are_independent_fields() {
        let mut v = registry_json();
        v["materials"][0]["name"] = serde_json::json!("concrete");
        let concrete: MaterialRegistry = serde_json::from_value(v.clone()).expect("parse");
        let m = &concrete.materials[0];
        assert_eq!(m.structural_support_strength_value(), 50);
        assert_eq!(m.lateral_yield_strength_value(), 50);

        v["materials"][0]["name"] = serde_json::json!("steel");
        let steel: MaterialRegistry = serde_json::from_value(v).expect("parse");
        let m = &steel.materials[0];
        assert_eq!(m.structural_support_strength_value(), 0);
        assert_eq!(m.lateral_yield_strength_value(), 200);
    }

    /// VAL-CROSS-025: explicit registry override for lateral_yield_strength
    /// wins over the spec-baked table.
    #[test]
    fn material_def_lateral_yield_strength_uses_override() {
        let mut v = registry_json();
        v["materials"][0]["name"] = serde_json::json!("concrete");
        v["materials"][0]["lateral_yield_strength"] = serde_json::json!(77);
        let r: MaterialRegistry = serde_json::from_value(v).expect("parse");
        assert_eq!(r.materials[0].lateral_yield_strength_value(), 77);
    }

    /// VAL-M14F-023: material lateral-yield ordering is strictly
    /// `wood < brick < concrete < steel`. The spec asserts this in
    /// behaviour — having the values in this strict ordering is a
    /// necessary precondition for that behaviour.
    #[test]
    fn material_yield_strict_ordering_wood_brick_concrete_steel() {
        let wood = lateral_yield_strength_for("wood").unwrap();
        let brick = lateral_yield_strength_for("brick").unwrap();
        let concrete = lateral_yield_strength_for("concrete").unwrap();
        let steel = lateral_yield_strength_for("steel").unwrap();
        assert!(wood < brick, "wood ({wood}) must yield before brick ({brick})");
        assert!(brick < concrete, "brick ({brick}) must yield before concrete ({concrete})");
        assert!(
            concrete < steel,
            "concrete ({concrete}) must yield before steel ({steel})"
        );
    }

    #[test]
    fn acoustic_field_round_trips_through_serde() {
        let mut v = registry_json();
        v["materials"][0]["echo_coefficient"] = serde_json::json!(0.85);
        v["materials"][0]["decay_band"] = serde_json::json!("bright");
        v["materials"][0]["acoustic_transmission_loss_db"] = serde_json::json!(28.0);
        v["materials"][0]["low_pass_cutoff_hz"] = serde_json::json!(800);
        let r: MaterialRegistry = serde_json::from_value(v).expect("parse");
        let back = serde_json::to_value(&r).unwrap();
        let s = back.to_string();
        assert!(s.contains("echo_coefficient"));
        assert!(s.contains("decay_band"));
        assert!(s.contains("acoustic_transmission_loss_db"));
        assert!(s.contains("low_pass_cutoff_hz"));
    }
}
