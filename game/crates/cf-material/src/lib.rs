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
    clippy::too_many_lines
)]

use std::{collections::BTreeMap, path::Path};

use serde::{Deserialize, Serialize};

pub mod loader;

pub use loader::{load_registry_from_file, validate_registry_json, RegistryValidationError, RegistryValidationReport};

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

impl MaterialDef {
    /// **M12B** § Resolved acoustic profile for this material. Missing
    /// fields fall back to [`AcousticProfile::fallback`].
    #[must_use]
    pub fn acoustic_profile(&self) -> AcousticProfile {
        let fb = AcousticProfile::fallback();
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
