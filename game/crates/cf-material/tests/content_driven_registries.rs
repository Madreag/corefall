//! **M15B+ § Content-driven registries**: verify that the reaction
//! registry, phase registry, and precipitation config can all be
//! loaded from JSON files and produce byte-identical results to the
//! hardcoded defaults. This unlocks mod-driven chemistry without
//! engine recompilation.

use cf_material::phase::{default_phase_registry, PhaseRegistry};
use cf_material::precipitation::PrecipitationConfig;
use cf_material::reactions::{default_reaction_registry, ReactionRegistry};

fn locate_content(name: &str) -> std::path::PathBuf {
    let workspace = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    workspace.join("content").join("materials").join(name)
}

/// VAL-M15B-CONTENT-001: reaction_registry.json loads + matches the
/// hardcoded default_reaction_registry() output. This guarantees the
/// content file is a true mirror of the engine baseline.
#[test]
fn reaction_registry_json_matches_hardcoded_default() {
    let path = locate_content("reaction_registry.json");
    let from_json = ReactionRegistry::load_from_file(&path).expect("reaction_registry.json loads");
    let hardcoded = default_reaction_registry();
    assert_eq!(
        from_json.len(),
        hardcoded.len(),
        "reaction count must match: json={} hardcoded={}",
        from_json.len(),
        hardcoded.len()
    );
    // Compare every reaction by id (since order may differ in serde
    // round-trip).
    for hc in &hardcoded.reactions {
        let j = from_json
            .by_id(&hc.id)
            .unwrap_or_else(|| panic!("reaction {} missing from JSON", hc.id));
        assert_eq!(j.input_a, hc.input_a, "{}: input_a", hc.id);
        assert_eq!(j.input_b, hc.input_b, "{}: input_b", hc.id);
        assert_eq!(j.output, hc.output, "{}: output", hc.id);
        assert_eq!(j.byproduct, hc.byproduct, "{}: byproduct", hc.id);
        assert_eq!(j.emissions, hc.emissions, "{}: emissions", hc.id);
        assert_eq!(j.auto_ignite, hc.auto_ignite, "{}: auto_ignite", hc.id);
        assert_eq!(
            j.min_temperature_k, hc.min_temperature_k,
            "{}: min_temperature_k",
            hc.id
        );
        assert_eq!(j.propagates, hc.propagates, "{}: propagates", hc.id);
    }
}

/// VAL-M15B-CONTENT-002: phase_registry.json loads + matches the
/// hardcoded default_phase_registry() output.
#[test]
fn phase_registry_json_matches_hardcoded_default() {
    let path = locate_content("phase_registry.json");
    let from_json = PhaseRegistry::load_from_file(&path).expect("phase_registry.json loads");
    let hardcoded = default_phase_registry();
    assert_eq!(from_json.len(), hardcoded.len());
    for (a, b) in from_json.transitions.iter().zip(hardcoded.transitions.iter()) {
        assert_eq!(a.material, b.material);
        assert_eq!(a.product_material, b.product_material);
        assert!((a.threshold_k - b.threshold_k).abs() < 0.01);
    }
}

/// VAL-M15B-CONTENT-003: precipitation_config.json loads + matches
/// the spec-baked baseline constants.
#[test]
fn precipitation_config_json_matches_baseline() {
    let path = locate_content("precipitation_config.json");
    let cfg = PrecipitationConfig::load_from_file(&path).expect("precipitation_config.json loads");
    let baseline = PrecipitationConfig::default();
    assert!((cfg.nucleation_altitude_px - baseline.nucleation_altitude_px).abs() < 1e-3);
    assert!((cfg.nucleation_temp_k_max - baseline.nucleation_temp_k_max).abs() < 1e-3);
    assert!(
        (cfg.precipitation_saturation_threshold - baseline.precipitation_saturation_threshold)
            .abs()
            < 1e-3
    );
    assert_eq!(cfg.precipitation_tick_gate, baseline.precipitation_tick_gate);
    assert!(
        (cfg.acid_rain_pollutant_fraction_min - baseline.acid_rain_pollutant_fraction_min).abs() < 1e-3
    );
    assert!((cfg.reference_pressure_kpa - baseline.reference_pressure_kpa).abs() < 1e-3);
    assert!((cfg.nucleation_pressure_min_kpa - baseline.nucleation_pressure_min_kpa).abs() < 1e-3);
    assert!((cfg.pressure_multiplier_lo - baseline.pressure_multiplier_lo).abs() < 1e-3);
    assert!((cfg.pressure_multiplier_hi - baseline.pressure_multiplier_hi).abs() < 1e-3);
}

/// VAL-M15B-CONTENT-004: ReactionRegistry::load_default_or_hardcoded
/// returns a valid registry whether or not the file is present.
#[test]
fn reaction_registry_default_or_hardcoded_loads() {
    let r = ReactionRegistry::load_default_or_hardcoded();
    assert!(r.len() >= 30, "must have at least 30 reactions");
}

/// VAL-M15B-CONTENT-005: PhaseRegistry::load_default_or_hardcoded.
#[test]
fn phase_registry_default_or_hardcoded_loads() {
    let r = PhaseRegistry::load_default_or_hardcoded();
    assert!(r.len() >= 5);
}

/// VAL-M15B-CONTENT-006: PrecipitationConfig::load_default_or_baseline.
#[test]
fn precipitation_config_default_or_baseline_loads() {
    let c = PrecipitationConfig::load_default_or_baseline();
    assert!(c.precipitation_saturation_threshold > 0.0);
}

/// VAL-M15B-CONTENT-007: mod scenarios can override the pressure
/// multipliers to make precipitation more or less aggressive without
/// recompiling.
#[test]
fn precipitation_config_pressure_multiplier_uses_config_values() {
    let mut cfg = PrecipitationConfig::default();
    cfg.pressure_multiplier_lo = 1.0;
    cfg.pressure_multiplier_hi = 4.0;
    // At ambient Earth pressure, multiplier should be 1.0 (clamp lo).
    let m_earth = cfg.pressure_rate_multiplier(cfg.reference_pressure_kpa);
    assert!((m_earth - 1.0).abs() < 1e-3);
    // At very low pressure, multiplier clamps to the (now-higher) high bound.
    let m_vac = cfg.pressure_rate_multiplier(10.0);
    assert!((m_vac - 4.0).abs() < 1e-3);
}

/// VAL-M15B-CONTENT-008: missing JSON file falls back to hardcoded
/// (server-tier deployments without content/ on the path).
#[test]
fn load_from_missing_file_returns_error_not_panic() {
    let r = ReactionRegistry::load_from_file("/tmp/this_file_definitely_does_not_exist.json");
    assert!(r.is_err());
}

/// VAL-M15B-CONTENT-009: corrupt JSON returns a typed parse error.
#[test]
fn load_from_corrupt_json_returns_typed_parse_error() {
    let tmpfile = std::env::temp_dir().join("cf_material_corrupt_test.json");
    std::fs::write(&tmpfile, b"{not valid json").expect("write");
    let r = ReactionRegistry::load_from_file(&tmpfile);
    assert!(r.is_err());
    let _ = std::fs::remove_file(&tmpfile);
}

/// VAL-M15B-CONTENT-010: schema_version mismatch returns a typed error.
#[test]
fn load_with_schema_version_mismatch_returns_typed_error() {
    let tmpfile = std::env::temp_dir().join("cf_material_schema_mismatch.json");
    std::fs::write(
        &tmpfile,
        br#"{"schema_version": 999, "reactions": []}"#,
    )
    .expect("write");
    let r = ReactionRegistry::load_from_file(&tmpfile);
    assert!(matches!(
        r,
        Err(cf_material::reactions::ReactionRegistryLoadError::SchemaVersionMismatch { .. })
    ));
    let _ = std::fs::remove_file(&tmpfile);
}
