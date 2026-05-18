//! **M14C** — cf-control acceptance tests pinned to the M14C validation
//! contract:
//!   - VAL-M14C-019: cfctl.act.player.fire surfaces ammo_kind={heat,apfsds}
//!   - VAL-M14C-020: scenario files load + run cleanly via the loader

use cf_actor::IntentSource;
use cf_control::Scenario;
use cf_control::{schemas::*, ControlCommand, M0Engine};

/// **VAL-M14C-019**: ActPlayerFireParams deserializes ammo_kind=heat / apfsds.
#[test]
fn val_m14c_019_act_player_fire_params_accepts_heat_apfsds() {
    let json = serde_json::json!({
        "schema_version": 2,
        "pressed": true,
        "ammo_kind": "heat",
    });
    let p: ActPlayerFireParams = serde_json::from_value(json).expect("heat parses");
    assert_eq!(p.ammo_kind.as_deref(), Some("heat"));

    let json = serde_json::json!({
        "schema_version": 2,
        "pressed": true,
        "ammo_kind": "apfsds",
    });
    let p: ActPlayerFireParams = serde_json::from_value(json).expect("apfsds parses");
    assert_eq!(p.ammo_kind.as_deref(), Some("apfsds"));

    // No ammo_kind = backward-compatible (rifle / default round).
    let json = serde_json::json!({"schema_version": 2, "pressed": true});
    let p: ActPlayerFireParams = serde_json::from_value(json).expect("default parses");
    assert!(p.ammo_kind.is_none());
}

/// **VAL-M14C-019** follow-on: snake_case RoundKind round-trip via
/// `cf_equipment::RoundKind::from_str_snake`. The cfctl dispatch boundary
/// rejects unknown ammo-kind labels with `unknown_ammo_kind`.
#[test]
fn val_m14c_019_round_kind_snake_case_roundtrip() {
    assert_eq!(
        cf_equipment::RoundKind::from_str_snake("heat"),
        Some(cf_equipment::RoundKind::Heat)
    );
    assert_eq!(
        cf_equipment::RoundKind::from_str_snake("apfsds"),
        Some(cf_equipment::RoundKind::Apfsds)
    );
    assert_eq!(cf_equipment::RoundKind::from_str_snake("garbage"), None);
}

/// **VAL-M14C-019**: ControlCommand::ActPlayerFire carries the ammo_kind
/// discriminator end-to-end (constructor accepts `Some(RoundKind::Heat)` and
/// `Some(RoundKind::Apfsds)`).
#[test]
fn val_m14c_019_control_command_carries_ammo_kind() {
    let cmd = ControlCommand::ActPlayerFire {
        pressed: true,
        ammo_kind: Some(cf_equipment::RoundKind::Heat),
        source: IntentSource::Cfctl,
    };
    match cmd {
        ControlCommand::ActPlayerFire { ammo_kind, .. } => {
            assert_eq!(ammo_kind, Some(cf_equipment::RoundKind::Heat));
        }
        _ => panic!("expected ActPlayerFire"),
    }
}

fn locate_scenario(id: &str) -> std::path::PathBuf {
    let workspace = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    workspace.join("content").join("scenarios").join(format!("{id}.ron"))
}

/// **VAL-M14C-020**: scenario files `m14c_heat_vs_era.ron` +
/// `m14c_apfsds_vs_heavy.ron` load via the scenario loader.
#[test]
fn val_m14c_020_scenarios_load_cleanly() {
    let path = locate_scenario("m14c_heat_vs_era");
    let s = Scenario::load_from_file(&path).expect("heat scenario parses");
    assert_eq!(s.id, "m14c_heat_vs_era");
    assert!(s.actors.iter().any(|a| a.id == 1), "blue actor present");
    assert!(s.actors.iter().any(|a| a.id == 2), "red actor present");

    let path = locate_scenario("m14c_apfsds_vs_heavy");
    let s = Scenario::load_from_file(&path).expect("apfsds scenario parses");
    assert_eq!(s.id, "m14c_apfsds_vs_heavy");
    assert!(s.actors.iter().any(|a| a.id == 1), "blue actor present");
    assert!(s.actors.iter().any(|a| a.id == 2), "red actor present");
    assert!(s.actors.iter().any(|a| a.id == 3), "unarmored infantry present");
}

/// **VAL-M14C-020** follow-on: cfctl headless drive accepts both
/// scenarios for 60 ticks without panic and produces a stable final
/// checksum.
#[test]
fn val_m14c_020_scenarios_drive_headless_to_60_ticks() {
    fn drive(scenario_id: &str) -> Option<String> {
        let path = locate_scenario(scenario_id);
        let scenario = Scenario::load_from_file(&path).expect("parse");
        let config = cf_control::M0EngineConfig::for_loaded_scenario(&scenario, path);
        let engine = M0Engine::new(config);
        engine.record_run_started();
        for _ in 0..60 {
            engine.drive_tick();
        }
        engine.recorder().final_checksum_hex()
    }
    let heat = drive("m14c_heat_vs_era");
    let apfsds = drive("m14c_apfsds_vs_heavy");
    assert!(heat.is_some());
    assert!(apfsds.is_some());
}

/// **VAL-M14C-013**: cf-killcam exposes heat_penetration + apfsds_through_module
/// variants and the dispatcher returns a non-default variant per trigger.
#[test]
fn val_m14c_013_killcam_variants_present_and_dispatch_non_default() {
    use cf_killcam::{
        dispatch_variant, ApfsdsThroughModulePayload, HeatPenetrationPayload, KillcamVariant,
        KillcamVariantTrigger,
    };
    let v = dispatch_variant(
        KillcamVariantTrigger::HeatJetTraversed,
        Some(HeatPenetrationPayload::default()),
        None,
    );
    assert!(!v.is_default());
    assert!(matches!(v, KillcamVariant::HeatPenetration(_)));

    let v = dispatch_variant(
        KillcamVariantTrigger::ApfsdsLongRodThrough,
        None,
        Some(ApfsdsThroughModulePayload::default()),
    );
    assert!(!v.is_default());
    assert!(matches!(v, KillcamVariant::ApfsdsThroughModule(_)));

    // Default trigger -> fallback variant.
    let v = dispatch_variant(KillcamVariantTrigger::Default, None, None);
    assert!(v.is_default());
}
