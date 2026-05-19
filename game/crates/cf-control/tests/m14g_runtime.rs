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
/// initial burn + frostbite degree as `wound.created`, then each
/// subsequent degree-upgrade on the same zone as `wound.escalated`
/// per the spec's Gherkin acceptance scenarios for sustained
/// fire / cold exposure. The composite scenario's thermal zones are
/// `foot_right` (hot, 800 K) and `hand_right` (cold, 250 K); other
/// zones can still legitimately emit Burn3rd via the HEAT-cluster
/// producer (VAL-M14G-022), so the zone-anchored check is scoped to
/// those two zones only.
#[test]
fn val_m14g_runtime_thermal_wounds_emit_via_engine() {
    let (_engine, events) = drive_scenario("m14g_whole_mission_determinism", 600);
    let zone_eq = |e: &&cf_replay::Event, zone: &str| -> bool {
        e.payload
            .get("zone")
            .and_then(|v| v.as_str())
            .map(|z| z == zone)
            .unwrap_or(false)
    };
    let count_created_on_zone = |kind: &str, zone: &str| -> usize {
        events
            .iter()
            .filter(|e| e.category == "wound" && e.event_type == "created")
            .filter(|e| {
                e.payload
                    .get("kind")
                    .and_then(|v| v.as_str())
                    .map(|k| k == kind)
                    .unwrap_or(false)
            })
            .filter(|e| zone_eq(e, zone))
            .count()
    };
    let count_escalated_new_kind_on_zone = |new_kind: &str, zone: &str| -> usize {
        events
            .iter()
            .filter(|e| e.category == "wound" && e.event_type == "escalated")
            .filter(|e| {
                e.payload
                    .get("new_kind")
                    .and_then(|v| v.as_str())
                    .map(|k| k == new_kind)
                    .unwrap_or(false)
            })
            .filter(|e| zone_eq(e, zone))
            .count()
    };
    // First-tier kinds (Burn1st, Frostbite1st) MUST fire as wound.created
    // on the thermal-zone actors.
    assert!(
        count_created_on_zone("Burn1st", "foot_right") >= 1,
        "expected ≥ 1 wound.created kind=Burn1st zone=foot_right from initial fire contact"
    );
    assert!(
        count_created_on_zone("Frostbite1st", "hand_right") >= 1,
        "expected ≥ 1 wound.created kind=Frostbite1st zone=hand_right from initial cold contact"
    );
    // Higher tiers on the thermal zone MUST arrive as wound.escalated
    // upgrades of the same wound — never as a fresh wound.created on
    // the same zone.
    for needle in ["Burn2nd", "Burn3rd"] {
        assert!(
            count_escalated_new_kind_on_zone(needle, "foot_right") >= 1,
            "expected ≥ 1 wound.escalated new_kind={needle} zone=foot_right from sustained burn"
        );
        assert_eq!(
            count_created_on_zone(needle, "foot_right"),
            0,
            "{needle} on foot_right must arrive via wound.escalated, not wound.created"
        );
    }
    for needle in ["Frostbite2nd", "Frostbite3rd"] {
        assert!(
            count_escalated_new_kind_on_zone(needle, "hand_right") >= 1,
            "expected ≥ 1 wound.escalated new_kind={needle} zone=hand_right from sustained cold"
        );
        assert_eq!(
            count_created_on_zone(needle, "hand_right"),
            0,
            "{needle} on hand_right must arrive via wound.escalated, not wound.created"
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

/// VAL-M14G-023 runtime evidence: load `m14g_melee_face.ron`, let the
/// engine settle the scenario, then dispatch one `MeleeShoulderCheck`
/// from actor 1 facing right. The blunt-impulse hit lands on actor 2
/// within shoulder-check reach; the engine's M14G producer routes the
/// hit through `classify_blunt_face_hit` and emits
/// `wound.created kind=DentalDamage severity=0.6 zone=head_front`.
#[test]
fn val_m14g_runtime_blunt_face_dental_damage() {
    let path = locate_scenario("m14g_melee_face");
    let scenario = Scenario::load_from_file(&path).expect("scenario parses");
    let config = M0EngineConfig::for_loaded_scenario(&scenario, path);
    let engine = M0Engine::new(config);
    engine.record_run_started();
    for _ in 0..2 {
        if engine.drive_tick().is_none() {
            break;
        }
    }
    let accepted = engine.m14g_dispatch_melee_shoulder_check();
    assert!(accepted, "MeleeShoulderCheck dispatch must be accepted");
    for _ in 0..3 {
        if engine.drive_tick().is_none() {
            break;
        }
    }
    let events = engine.recorder().snapshot_events();
    let dental_events: Vec<_> = events
        .iter()
        .filter(|e| e.category == "wound" && e.event_type == "created")
        .filter(|e| {
            e.payload
                .get("kind")
                .and_then(|v| v.as_str())
                .map(|k| k == "DentalDamage")
                .unwrap_or(false)
        })
        .collect();
    assert!(
        !dental_events.is_empty(),
        "expected ≥ 1 wound.created kind=DentalDamage; got wound events: {:?}",
        events
            .iter()
            .filter(|e| e.category == "wound")
            .map(|e| (
                e.event_type.clone(),
                e.payload.get("kind").and_then(|v| v.as_str()).map(String::from)
            ))
            .collect::<Vec<_>>()
    );
    for e in &dental_events {
        let zone = e.payload.get("zone").and_then(|v| v.as_str()).unwrap_or("");
        let severity = e.payload.get("severity").and_then(|v| v.as_f64()).unwrap_or(0.0);
        assert_eq!(zone, "head_front", "DentalDamage must land on head_front");
        assert!(
            (severity - 0.6_f64).abs() < 1e-2,
            "DentalDamage severity must be ≈ 0.6; got {severity}"
        );
    }
}
