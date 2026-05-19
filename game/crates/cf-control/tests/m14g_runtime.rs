//! **M14G runtime-evidence acceptance tests** — drive each M14C/M14D/
//! M14E/M14F/M14G scenario end-to-end through `M0Engine::drive_tick`
//! and assert the typed M14G `wound.created` emit path actually fires
//! at the runtime-evidence layer. Per AGENTS.md, scrutiny mandates
//! runtime evidence — unit tests on the producer surface alone are not
//! sufficient.
//!
//! Each test cites the VAL-CROSS-* / VAL-M14G-* assertion it satisfies.

use cf_control::{M0Engine, M0EngineConfig, Scenario};

fn locate_scenario(id: &str) -> std::path::PathBuf {
    let workspace = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    workspace.join("content").join("scenarios").join(format!("{id}.ron"))
}

fn drive_scenario(scenario_id: &str, ticks: u64) -> (M0Engine, Vec<cf_replay::Event>) {
    let path = locate_scenario(scenario_id);
    let scenario = Scenario::load_from_file(&path).expect("scenario parses");
    let config = M0EngineConfig::for_loaded_scenario(&scenario, path);
    let engine = M0Engine::new(config);
    engine.record_run_started();
    for _ in 0..ticks {
        if engine.drive_tick().is_none() {
            break;
        }
    }
    let events = engine.recorder().snapshot_events();
    (engine, events)
}

fn wound_kinds(events: &[cf_replay::Event]) -> Vec<String> {
    events
        .iter()
        .filter(|e| e.category == "wound" && e.event_type == "created")
        .filter_map(|e| e.payload.get("kind").and_then(|v| v.as_str()).map(String::from))
        .collect()
}

fn count_wounds_of_kind(events: &[cf_replay::Event], kind: &str) -> usize {
    wound_kinds(events).iter().filter(|k| k.as_str() == kind).count()
}

fn count_legacy_wound_added(events: &[cf_replay::Event]) -> usize {
    events
        .iter()
        .filter(|e| e.category == "combat" && e.event_type == "wound_added")
        .count()
}

/// VAL-CROSS-001 runtime evidence: cfctl drive of `m14c_heat_vs_era`
/// emits ≥ 1 `wound.created kind=Burn3rd` AND ≥ 1 `kind=GunshotThrough`
/// on crew_torso.
#[test]
fn val_m14g_runtime_heat_round_cluster() {
    let (_engine, events) = drive_scenario("m14c_heat_vs_era", 600);
    let burn3rd = count_wounds_of_kind(&events, "Burn3rd");
    let gunshot_through = count_wounds_of_kind(&events, "GunshotThrough");
    assert!(
        burn3rd >= 1,
        "expected ≥ 1 Burn3rd wound from HEAT cluster, got {burn3rd}"
    );
    assert!(
        gunshot_through >= 1,
        "expected ≥ 1 GunshotThrough wound from HEAT cluster, got {gunshot_through}"
    );
    let crew_torso_count = events
        .iter()
        .filter(|e| e.category == "wound" && e.event_type == "created")
        .filter(|e| {
            e.payload
                .get("kind")
                .and_then(|v| v.as_str())
                .map(|k| k == "GunshotThrough")
                .unwrap_or(false)
        })
        .filter(|e| {
            e.payload
                .get("zone")
                .and_then(|v| v.as_str())
                .map(|z| z == "crew_torso")
                .unwrap_or(false)
        })
        .count();
    assert!(
        crew_torso_count >= 1,
        "GunshotThrough must land on crew_torso (got {crew_torso_count})"
    );
}

/// VAL-CROSS-002 runtime evidence: cfctl drive of
/// `m14c_apfsds_vs_heavy` emits ≥ 3 `wound.created kind=ShrapnelThrough`
/// across the traversed modules.
#[test]
fn val_m14g_runtime_apfsds_shrapnel_through() {
    let (_engine, events) = drive_scenario("m14c_apfsds_vs_heavy", 600);
    let shrapnel_through = count_wounds_of_kind(&events, "ShrapnelThrough");
    assert!(
        shrapnel_through >= 3,
        "expected ≥ 3 ShrapnelThrough wounds from APFSDS path; got {shrapnel_through}"
    );
}

/// VAL-CROSS-009 runtime evidence: cfctl drive of
/// `m14d_projectile_intercept` emits ≥ 3 `wound.created kind=ShrapnelEmbedded`
/// on a fuze-triggered grenade's blast-radius actor.
#[test]
fn val_m14g_runtime_fuze_grenade_shrapnel_embedded() {
    let (_engine, events) = drive_scenario("m14d_projectile_intercept", 600);
    let embedded = count_wounds_of_kind(&events, "ShrapnelEmbedded");
    assert!(
        embedded >= 3,
        "expected ≥ 3 ShrapnelEmbedded wounds from fuze grenade detonation; got {embedded}"
    );
}

/// VAL-CROSS-007 runtime evidence: cfctl drive of
/// `m14e_tunnel_collapse_drill` emits ≥ 1 `wound.created` of a
/// skeletal kind on the actor underneath.
#[test]
fn val_m14g_runtime_cave_in_fractures() {
    let (_engine, events) = drive_scenario("m14e_tunnel_collapse_drill", 600);
    let kinds = wound_kinds(&events);
    let skeletal_count = kinds
        .iter()
        .filter(|k| matches!(k.as_str(), "FractureSimple" | "FractureCompound" | "FractureComminuted"))
        .count();
    assert!(
        skeletal_count >= 1,
        "expected ≥ 1 fracture wound from cave-in falling debris; got kinds={:?}",
        kinds
    );
}

/// VAL-CROSS-008 runtime evidence: cfctl drive of
/// `m14f_bunker_siege_wall_fail` emits ≥ 1 `wound.created` of
/// {FractureSimple, FractureCompound, FractureComminuted, CrushLimb,
/// BruiseHeavy} on the actor in the debris path.
#[test]
fn val_m14g_runtime_wall_rupture_wounds() {
    let (_engine, events) = drive_scenario("m14f_bunker_siege_wall_fail", 600);
    let kinds = wound_kinds(&events);
    let qualifying_count = kinds
        .iter()
        .filter(|k| {
            matches!(
                k.as_str(),
                "FractureSimple"
                    | "FractureCompound"
                    | "FractureComminuted"
                    | "CrushLimb"
                    | "BruiseHeavy"
            )
        })
        .count();
    assert!(
        qualifying_count >= 1,
        "expected ≥ 1 fracture/crush/bruise wound from wall rupture; got kinds={:?}",
        kinds
    );
}

/// VAL-M14G-013/014/030 runtime evidence: cfctl drive of the
/// composite scenario (which carries `m14g_thermal_zones`) emits the
/// expected burn + frostbite kind ladders via the engine.
#[test]
fn val_m14g_runtime_thermal_wounds_emit_via_engine() {
    let (_engine, events) = drive_scenario("m14g_whole_mission_determinism", 600);
    let burns = [
        count_wounds_of_kind(&events, "Burn1st"),
        count_wounds_of_kind(&events, "Burn2nd"),
        count_wounds_of_kind(&events, "Burn3rd"),
    ];
    let frostbites = [
        count_wounds_of_kind(&events, "Frostbite1st"),
        count_wounds_of_kind(&events, "Frostbite2nd"),
        count_wounds_of_kind(&events, "Frostbite3rd"),
    ];
    for (i, count) in burns.iter().enumerate() {
        assert!(
            *count >= 1,
            "expected ≥ 1 Burn{}st/2nd/3rd from thermal pass; ladder={:?}",
            i + 1,
            burns
        );
    }
    for (i, count) in frostbites.iter().enumerate() {
        assert!(
            *count >= 1,
            "expected ≥ 1 Frostbite{} from thermal pass; ladder={:?}",
            i + 1,
            frostbites
        );
    }
}

/// VAL-M14G-027 runtime evidence: ZERO `combat.wound_added` events
/// fire across all five scenarios. The legacy placeholder is gone.
#[test]
fn val_m14g_runtime_no_legacy_combat_wound_added() {
    let scenarios = [
        "m14c_heat_vs_era",
        "m14c_apfsds_vs_heavy",
        "m14d_projectile_intercept",
        "m14e_tunnel_collapse_drill",
        "m14f_bunker_siege_wall_fail",
        "m14g_whole_mission_determinism",
    ];
    for scenario in &scenarios {
        let (_engine, events) = drive_scenario(scenario, 600);
        let legacy = count_legacy_wound_added(&events);
        assert_eq!(
            legacy, 0,
            "{scenario} must emit ZERO legacy combat.wound_added events; got {legacy}"
        );
    }
}

/// VAL-M14G-029 runtime evidence: cfctl drive of the composite
/// scenario emits ≥ 1 `wound.created kind=AcidBurn` and ≥ 1
/// `kind=ChemicalBurn` from the per-tick material-contact pass.
#[test]
fn val_m14g_runtime_material_contact_wounds_emit_via_engine() {
    let (_engine, events) = drive_scenario("m14g_whole_mission_determinism", 600);
    let acid = count_wounds_of_kind(&events, "AcidBurn");
    let chemical = count_wounds_of_kind(&events, "ChemicalBurn");
    assert!(
        acid >= 1,
        "expected ≥ 1 AcidBurn from material contact pass; got {acid}"
    );
    assert!(
        chemical >= 1,
        "expected ≥ 1 ChemicalBurn from material contact pass; got {chemical}"
    );
}
