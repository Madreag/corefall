//! M17 completeness — the per-origin mechanics beyond the 13 headline
//! scenarios: full death-trigger tables, resource-driven action/mobility
//! degradation, power action costs, special gas reactions, KO incapacitation,
//! drain-rate events, AI doctrine, and settings gating.

use cf_control::{M0Engine, M0EngineConfig, Scenario};
use cf_replay::Event;

fn engine() -> M0Engine {
    let workspace = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let path = workspace
        .join("content")
        .join("scenarios")
        .join("m17_origin_resource_model.ron");
    let scenario = Scenario::load_from_file(&path).expect("scenario parses");
    let config = M0EngineConfig::for_loaded_scenario(&scenario, path);
    M0Engine::new(config)
}

fn events(engine: &M0Engine) -> Vec<Event> {
    engine.recorder().snapshot_events()
}

fn has_event_for(engine: &M0Engine, category: &str, event_type: &str, actor_id: u64) -> bool {
    events(engine).iter().any(|e| {
        e.category == category
            && e.event_type == event_type
            && e.payload.get("actor_id").and_then(|v| v.as_u64()) == Some(actor_id)
    })
}

fn res(engine: &M0Engine, actor_id: u64) -> serde_json::Value {
    engine.m17_actor_resources(actor_id)
}

fn f(v: &serde_json::Value, key: &str) -> f64 {
    v.get(key).and_then(|x| x.as_f64()).unwrap_or(f64::NAN)
}

// ===========================================================================
// Human blood loss — graduated status: STABLE → UNSTABLE → DOWNED → DYING.
// ===========================================================================
#[test]
fn human_blood_loss_graduated_death_triggers() {
    let engine = engine();
    engine.drive_tick();

    // blood < 30% → UNSTABLE.
    engine.m17_set_resource(1, "blood", 5000.0 * 0.25);
    engine.drive_tick();
    assert_eq!(res(&engine, 1).get("status").and_then(|v| v.as_str()), Some("unstable"));

    // blood < 10% → DOWNED (savable critical) + bleed.
    engine.m17_set_resource(1, "blood", 5000.0 * 0.05);
    engine.drive_tick();
    assert_eq!(res(&engine, 1).get("status").and_then(|v| v.as_str()), Some("downed"));

    // blood = 0 → DYING (terminal).
    engine.m17_set_resource(1, "blood", 0.0);
    engine.drive_tick();
    assert_eq!(res(&engine, 1).get("status").and_then(|v| v.as_str()), Some("dying"));
}

// ===========================================================================
// Robot oil — joints seize at oil = 0 (mobility → 0).
// ===========================================================================
#[test]
fn robot_oil_seizes_joints() {
    let engine = engine();
    engine.drive_tick();

    // oil < 30% → mobility degraded (×0.75).
    engine.m17_set_resource(3, "oil", 5000.0 * 0.2);
    engine.drive_tick();
    let m = f(&res(&engine, 3), "mobility_mult");
    assert!(m < 1.0 && m > 0.0, "oil < 30% degrades mobility; got {m}");

    // oil = 0 → seized (mobility 0).
    engine.m17_set_resource(3, "oil", 0.0);
    engine.drive_tick();
    assert_eq!(f(&res(&engine, 3), "mobility_mult"), 0.0, "seized joints");
}

// ===========================================================================
// Robot power degradation — < 30% slower action, < 10% reserve fire-lock.
// ===========================================================================
#[test]
fn robot_power_degradation_action_speed() {
    let engine = engine();
    engine.drive_tick();

    // power < 30% → action speed × 0.5.
    engine.m17_set_resource(3, "power", 100.0 * 0.25);
    engine.drive_tick();
    assert!(
        (f(&res(&engine, 3), "action_speed_factor") - 0.5).abs() < 1e-6,
        "power < 30% halves action speed"
    );
    assert!(engine.m17_can_fire(3), "still able to fire at 25% power");

    // power < 10% → reserve mode: fire locked.
    engine.m17_set_resource(3, "power", 100.0 * 0.05);
    engine.drive_tick();
    assert!(!engine.m17_can_fire(3), "reserve mode locks fire");
}

// ===========================================================================
// Overclock raises action speed (and therefore fire cadence).
// ===========================================================================
#[test]
fn overclock_raises_action_speed_factor() {
    let engine = engine();
    engine.drive_tick();
    assert!(engine.m17_request_overclock(3, 3));
    engine.drive_tick();
    assert!(
        (f(&res(&engine, 3), "action_speed_factor") - 1.5).abs() < 1e-6,
        "tier-3 overclock = 1.5x action speed"
    );
}

// ===========================================================================
// Android hybrid — blood AND power both critical → downed; both empty → dead.
// ===========================================================================
#[test]
fn android_hybrid_dual_resource_death() {
    let engine = engine();
    engine.drive_tick();

    // blood < 20% AND power < 20% → hybrid-critical (downed).
    engine.m17_set_resource(2, "blood", 4000.0 * 0.15);
    engine.m17_set_resource(2, "power", 60.0 * 0.15);
    engine.drive_tick();
    assert_eq!(res(&engine, 2).get("status").and_then(|v| v.as_str()), Some("downed"));

    // both empty → full destruction (dying → dead).
    engine.m17_set_resource(2, "blood", 0.0);
    engine.m17_set_resource(2, "power", 0.0);
    engine.drive_tick();
    let st = res(&engine, 2).get("status").and_then(|v| v.as_str()).map(String::from);
    assert!(matches!(st.as_deref(), Some("dying") | Some("dead")), "both empty → death; got {st:?}");
}

// ===========================================================================
// Methane breather — oxygen is poison in an O2 atmosphere.
// ===========================================================================
#[test]
fn methane_breather_oxygen_poisoning() {
    let engine = engine();
    engine.drive_tick();
    // Actor 5 (methane breather) sits in the default Earth-like atmosphere
    // (O2 ~21 kPa) — oxygen poisons it.
    let hp0 = f(&res(&engine, 5), "hp");
    for _ in 0..5 {
        engine.drive_tick();
    }
    assert!(f(&res(&engine, 5), "hp") < hp0, "O2 poisons a methane breather");
    assert!(has_event_for(&engine, "affliction", "applied", 5), "oxygen_poisoning affliction fires");
}

// ===========================================================================
// Concussion KO actually incapacitates (knockdown for the blackout window).
// ===========================================================================
#[test]
fn concussion_ko_incapacitates_actor() {
    let engine = engine();
    engine.drive_tick();
    // A huge hit drives the human past the KO threshold.
    engine.m17_inject_hit(1, 300.0, "head");
    let kd = res(&engine, 1)
        .get("knockdown_ticks_remaining")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    assert!(kd > 0, "KO incapacitates the actor (knockdown window); got {kd}");
}

// ===========================================================================
// resource.drain_rate_changed fires when a bleed starts.
// ===========================================================================
#[test]
fn drain_rate_changed_on_new_bleed() {
    let engine = engine();
    engine.drive_tick();
    engine.m16_apply_affliction(1, cf_affliction::M16AfflictionKind::Bleeding, 0.8, "m17_test");
    engine.drive_tick();
    assert!(
        has_event_for(&engine, "resource", "drain_rate_changed", 1),
        "a new bleed changes the blood drain rate"
    );
}

// ===========================================================================
// AI doctrine — low battery emits the power-aware doctrine event.
// ===========================================================================
#[test]
fn ai_doctrine_low_battery_emits() {
    let engine = engine();
    engine.drive_tick();
    engine.m17_set_resource(3, "power", 100.0 * 0.15); // < 20%
    engine.drive_tick();
    let ev = events(&engine)
        .into_iter()
        .find(|e| e.category == "ai" && e.event_type == "m17_doctrine"
            && e.payload.get("actor_id").and_then(|v| v.as_u64()) == Some(3));
    let ev = ev.expect("ai.m17_doctrine fires for a low-battery robot");
    let reason = ev.payload.get("reason").and_then(|v| v.as_str()).unwrap_or("");
    assert!(reason == "low_battery" || reason == "power_shed", "got {reason}");
}

// ===========================================================================
// Settings gate — disabling oxygen simulation stops vacuum O2 drain.
// ===========================================================================
#[test]
fn settings_gate_disables_oxygen_sim() {
    let engine = engine();
    engine.drive_tick();
    engine.m17_set_power_sim(false, true, true); // oxygen sim OFF
    engine.m17_set_atmosphere_pressure(1, 3.0); // vacuum
    engine.m17_set_resource(1, "oxygen", 100.0);
    let o2_before = f(&res(&engine, 1), "oxygen_supply");
    for _ in 0..30 {
        engine.drive_tick();
    }
    assert!(
        (f(&res(&engine, 1), "oxygen_supply") - o2_before).abs() < 1e-3,
        "oxygen sim disabled → no vacuum drain"
    );
}

// ===========================================================================
// Content override — origin profile loads from content/origins/*.json.
// ===========================================================================
#[test]
fn origin_registry_loads_canonical_resources() {
    let engine = engine();
    engine.drive_tick();
    // Seeded from the loaded registry (content/origins/human.json = 5000 mL).
    assert_eq!(f(&res(&engine, 1), "blood_max"), 5000.0);
    assert_eq!(f(&res(&engine, 3), "power_max"), 100.0);
}
