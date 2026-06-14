//! M17 acceptance — origin reaction + per-origin resource model against the
//! live M0Engine. Each test drives the real engine tick (or the explicit M17
//! origin pass) and asserts on engine state + the recorded event stream.
//!
//! Actor cast (scenario `m17_origin_resource_model`):
//!   1 = human, 2 = android, 3 = robot (team red_robot), 4 = heavy_biomech,
//!   5 = methane_breather.

use cf_control::{M0Engine, M0EngineConfig, Scenario};
use cf_replay::Event;

fn locate_scenario(id: &str) -> std::path::PathBuf {
    let workspace = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    workspace.join("content").join("scenarios").join(format!("{id}.ron"))
}

fn engine() -> M0Engine {
    let path = locate_scenario("m17_origin_resource_model");
    let scenario = Scenario::load_from_file(&path).expect("scenario parses");
    let config = M0EngineConfig::for_loaded_scenario(&scenario, path);
    M0Engine::new(config)
}

fn events(engine: &M0Engine) -> Vec<Event> {
    engine.recorder().snapshot_events()
}

fn event_for_actor(engine: &M0Engine, category: &str, event_type: &str, actor_id: u64) -> Option<Event> {
    events(engine).into_iter().find(|e| {
        e.category == category
            && e.event_type == event_type
            && e.payload.get("actor_id").and_then(|v| v.as_u64()) == Some(actor_id)
    })
}

fn has_event(engine: &M0Engine, category: &str, event_type: &str) -> bool {
    events(engine).iter().any(|e| e.category == category && e.event_type == event_type)
}

// ===========================================================================
// Scenario — Origin force feedback emits per actor's origin.
// ===========================================================================
#[test]
fn origin_force_feedback_emits_per_origin() {
    let engine = engine();
    engine.drive_tick();

    // Human hit → pain_jolt, g_load_delta > 0, no internal shock module.
    assert!(engine.m17_inject_hit(1, 40.0, "torso"));
    let ff = event_for_actor(&engine, "origin", "shot_force_feedback", 1).expect("human force feedback");
    assert_eq!(ff.payload.get("feedback_kind").and_then(|v| v.as_str()), Some("pain_jolt"));
    assert_eq!(ff.payload.get("origin_id").and_then(|v| v.as_str()), Some("Human"));
    assert!(ff.payload.get("g_load_delta").and_then(|v| v.as_f64()).unwrap() > 0.0);
    assert!(ff.payload.get("internal_shock_module_id").map(|v| v.is_null()).unwrap_or(true));

    // Robot hit → servo_jolt + frame_ring, g_load_delta == 0, module rolled.
    assert!(engine.m17_inject_hit(3, 40.0, "torso"));
    let rf = event_for_actor(&engine, "origin", "shot_force_feedback", 3).expect("robot force feedback");
    assert_eq!(rf.payload.get("feedback_kind").and_then(|v| v.as_str()), Some("servo_jolt"));
    assert_eq!(rf.payload.get("origin_id").and_then(|v| v.as_str()), Some("Robot"));
    assert_eq!(rf.payload.get("frame_ring").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(rf.payload.get("g_load_delta").and_then(|v| v.as_f64()), Some(0.0));
    assert!(rf.payload.get("internal_shock_module_id").map(|v| !v.is_null()).unwrap_or(false));
    // The robot got an internal-shock dose, not a concussion dose.
    assert!(has_event(&engine, "internal_shock", "dose_changed"));
}

// ===========================================================================
// Scenario — Concussion vignette curve per origin (human full / android cap /
// robot none).
// ===========================================================================
#[test]
fn concussion_vignette_curve_per_origin() {
    let engine = engine();
    engine.drive_tick();

    // Human: a big hit pushes the dose to the KO_Imminent band (85% vignette).
    engine.m17_inject_hit(1, 150.0, "head"); // 150 * 0.6 = 90 → KO_Imminent
    let hr = engine.m17_actor_resources(1);
    assert_eq!(hr.get("concussion_band").and_then(|v| v.as_str()), Some("KO_Imminent"));
    assert_eq!(
        cf_actor::concussion::ConcussionBand::KoImminent.vignette_fraction(),
        0.85
    );

    // Android: the same magnitude is susceptibility-halved AND capped at
    // Moderate (synthetic side resists).
    engine.m17_inject_hit(2, 300.0, "head"); // 300 * 0.6 * 0.5 = 90 raw → capped Moderate
    let ar = engine.m17_actor_resources(2);
    assert_eq!(ar.get("concussion_band").and_then(|v| v.as_str()), Some("Moderate"));

    // Robot: never accrues a concussion band; routes to internal shock.
    engine.m17_inject_hit(3, 150.0, "head");
    let rr = engine.m17_actor_resources(3);
    assert_eq!(rr.get("concussion_band").and_then(|v| v.as_str()), Some("Clear"));
    assert!(rr.get("internal_shock_dose").and_then(|v| v.as_f64()).unwrap() > 0.0);
}

// ===========================================================================
// Scenario — Resource depletion per origin.
// ===========================================================================
#[test]
fn resource_depletion_per_origin() {
    let engine = engine();
    engine.m16_set_survival_mode(true);
    engine.drive_tick();

    // Human low caloric → hunger warning + aim accuracy × 0.85.
    engine.m17_set_resource(1, "caloric", 10.0);
    engine.drive_tick();
    assert!((engine.m17_aim_accuracy_mult(1) - 0.85).abs() < 1e-6, "low caloric → 0.85 aim");
    let afflictions = engine.m16_actor_afflictions(1);
    assert!(
        afflictions.iter().any(|(k, _)| k == "hunger"),
        "low caloric raises hunger; got {afflictions:?}"
    );

    // Robot low power → reserve mode → cannot fire.
    engine.m17_set_resource(3, "power", 5.0); // 5% of 100 kWh < 10% reserve
    engine.drive_tick();
    assert!(!engine.m17_can_fire(3), "robot in reserve mode cannot fire");

    // Robot power == 0 → INERT (recoverable, not dead).
    engine.m17_set_resource(3, "power", 0.0);
    engine.drive_tick();
    assert_eq!(engine.m17_actor_status(3).as_deref(), Some("inert"));
    assert!(!engine.m17_can_fire(3));
    assert!(has_event(&engine, "resource", "cascade_offline"));

    // Repair-tool revive restores power + clears INERT.
    assert!(engine.m17_repair_revive(3));
    assert_eq!(engine.m17_actor_status(3).as_deref(), Some("stable"));
    assert!(has_event(&engine, "resource", "restored"));
}

// ===========================================================================
// Scenario — Helmet breach in vacuum (3× O2 drain → suffocation).
// ===========================================================================
#[test]
fn helmet_breach_in_vacuum() {
    let engine = engine();
    engine.drive_tick();

    assert!(engine.m17_equip_sealed_helmet(1), "sealed helmet equipped");
    engine.m17_set_atmosphere_pressure(1, 5.0); // vacuum
    engine.m17_set_resource(1, "oxygen", 1000.0);

    // Sealed (un-breached) drain over 1 in-game second.
    let o2_a = engine.m17_actor_resources(1).get("oxygen_supply").and_then(|v| v.as_f64()).unwrap();
    for _ in 0..60 {
        engine.drive_tick();
    }
    let o2_b = engine.m17_actor_resources(1).get("oxygen_supply").and_then(|v| v.as_f64()).unwrap();
    let sealed_drain = o2_a - o2_b;
    assert!(sealed_drain > 0.0, "sealed helmet still consumes O2 in vacuum");

    // Breach the helmet → next second drains ~3×.
    assert!(engine.m17_breach_helmet(1));
    assert!(event_for_actor(&engine, "origin", "helmet_breach", 1).is_some());
    let o2_c = engine.m17_actor_resources(1).get("oxygen_supply").and_then(|v| v.as_f64()).unwrap();
    for _ in 0..60 {
        engine.drive_tick();
    }
    let o2_d = engine.m17_actor_resources(1).get("oxygen_supply").and_then(|v| v.as_f64()).unwrap();
    let breach_drain = o2_c - o2_d;
    assert!(
        breach_drain > sealed_drain * 2.0,
        "breach drains ~3× ({breach_drain} vs {sealed_drain})"
    );

    // O2 empty → HP drains + hypoxia stacks.
    engine.m17_set_resource(1, "oxygen", 0.0);
    let hp0 = actor_hp(&engine, 1);
    for _ in 0..3 {
        engine.drive_tick();
    }
    assert!(actor_hp(&engine, 1) < hp0, "oxygen-empty drains HP");
    let aff = engine.m16_actor_afflictions(1);
    assert!(aff.iter().any(|(k, _)| k == "hypoxic"), "hypoxia stacks; got {aff:?}");
}

fn actor_hp(engine: &M0Engine, actor_id: u64) -> f32 {
    engine
        .m17_actor_resources(actor_id)
        .get("hp")
        .and_then(|h| h.as_f64())
        .map(|h| h as f32)
        .unwrap_or(0.0)
}

// ===========================================================================
// Scenario — Robot downclock under sustained heat.
// ===========================================================================
#[test]
fn robot_downclock_under_sustained_heat() {
    let engine = engine();
    engine.drive_tick();

    engine.m17_set_resource(3, "heat", 0.75); // throttle band (0.70)
    engine.drive_tick();
    assert!(has_event(&engine, "chassis", "thermal_throttle_started"));
    assert!(
        (engine.m17_action_speed(3) - 0.5).abs() < 1e-6,
        "throttled action speed = 0.5x"
    );
}

// ===========================================================================
// Scenario — Robot overclock voluntary boost (+ sustained-heat module damage).
// ===========================================================================
#[test]
fn robot_overclock_boost_and_sustained_damage() {
    let engine = engine();
    engine.drive_tick();

    // Overclock tier 2 → boost event + 1.3x action speed (heat still cool).
    assert!(engine.m17_request_overclock(3, 2));
    assert!(has_event(&engine, "chassis", "overclock_started"));
    assert!(
        (engine.m17_action_speed(3) - 1.30).abs() < 1e-6,
        "tier 2 overclock = 1.3x action speed"
    );

    // Sustained overclock cooks the chassis → heat crosses critical → a module
    // takes overheat damage.
    engine.m17_set_resource(3, "heat", 0.68);
    engine.m17_request_overclock(3, 3);
    for _ in 0..360 {
        engine.drive_tick();
    }
    assert!(
        has_event(&engine, "internal_shock", "module_damaged"),
        "sustained critical heat damages a module"
    );
}

// ===========================================================================
// Scenario — Vacuum without helmet → hypoxia (robot immune).
// ===========================================================================
#[test]
fn vacuum_without_helmet_hypoxia_robot_immune() {
    let engine = engine();
    engine.drive_tick();

    // Human, no helmet, vacuum.
    engine.m17_set_atmosphere_pressure(1, 3.0);
    engine.m17_set_resource(1, "oxygen", 5.0);
    // Robot in the same vacuum (oxygen-immune).
    engine.m17_set_atmosphere_pressure(3, 3.0);
    for _ in 0..30 {
        engine.drive_tick();
    }
    let human_aff = engine.m16_actor_afflictions(1);
    assert!(human_aff.iter().any(|(k, _)| k == "hypoxic"), "human suffocates; got {human_aff:?}");
    let robot_aff = engine.m16_actor_afflictions(3);
    assert!(
        !robot_aff.iter().any(|(k, _)| k == "hypoxic"),
        "robot is vacuum-immune; got {robot_aff:?}"
    );
}

// ===========================================================================
// Scenario — Death recap renders per origin (never wrong-origin language).
// ===========================================================================
#[test]
fn death_recap_renders_per_origin() {
    let engine = engine();
    engine.drive_tick();

    // Human bleeds out (blood → 0).
    engine.m17_set_resource(1, "blood", 0.0);
    engine.drive_tick();
    let human_recap = engine.m17_death_recap(1).to_lowercase();
    assert!(human_recap.contains("bled") || human_recap.contains("blood"), "human: {human_recap}");
    assert!(!human_recap.contains("offline") && !human_recap.contains("circuit"));

    // Robot goes offline (power → 0).
    engine.m17_set_resource(3, "power", 0.0);
    engine.drive_tick();
    let robot_recap = engine.m17_death_recap(3).to_lowercase();
    assert!(robot_recap.contains("offline") || robot_recap.contains("power"), "robot: {robot_recap}");
    assert!(!robot_recap.contains("bled") && !robot_recap.contains("blood"));
}

// ===========================================================================
// Scenario — Heart-rate audio mix per concussion band.
// ===========================================================================
#[test]
fn heart_rate_audio_gain_per_band() {
    use cf_actor::concussion::ConcussionBand;
    assert!((ConcussionBand::Mild.heart_rate_gain_bonus() - 0.20).abs() < 1e-6);
    assert!((ConcussionBand::Severe.heart_rate_gain_bonus() - 0.60).abs() < 1e-6);
    assert!(!ConcussionBand::Mild.ducks_ambient());
    assert!(ConcussionBand::Severe.ducks_ambient());
}

// ===========================================================================
// Scenario — Compound TTD floor enforced at default difficulty (≥ 8s).
// ===========================================================================
#[test]
fn compound_ttd_floor_at_default_difficulty() {
    let engine = engine();
    engine.drive_tick();

    // Stack lethal afflictions on the human via the M16 surface (bleeding +
    // burning) so the compound TTD math has lethal sources.
    engine.m16_apply_affliction(1, cf_affliction::M16AfflictionKind::Bleeding, 0.9, "m17_test");
    engine.m16_apply_affliction(1, cf_affliction::M16AfflictionKind::Burning, 0.9, "m17_test");
    engine.drive_tick();

    let ttd = engine.m17_actor_ttd(1);
    let compound = ttd.get("compound_ttd_seconds").and_then(|v| v.as_f64());
    if let Some(c) = compound {
        assert!(c >= 8.0, "compound TTD floored at 8s default; got {c}");
    }
    assert_eq!(ttd.get("compound_floor_seconds").and_then(|v| v.as_f64()), Some(8.0));
}

// ===========================================================================
// Scenario — Per-origin TTD differs for the same wound + difficulty multiplier.
// ===========================================================================
#[test]
fn per_origin_ttd_and_difficulty_multiplier() {
    let engine = engine();

    // Same heart wound, different origins (Tough Crowd baseline).
    assert_eq!(engine.m17_damage_type_ttd("human", "heart_wound", "tough_crowd"), Some(90.0));
    assert_eq!(engine.m17_damage_type_ttd("android", "heart_wound", "tough_crowd"), Some(110.0));
    assert_eq!(engine.m17_damage_type_ttd("heavy_biomech", "heart_wound", "tough_crowd"), Some(180.0));
    // Robot rejects "heart wound" → routes to the primary power cable.
    assert_eq!(engine.m17_damage_type_ttd("robot", "heart_wound", "tough_crowd"), Some(40.0));

    // Difficulty multiplier applies system-wide: Veteran = 0.65×.
    let veteran = engine.m17_damage_type_ttd("human", "heart_wound", "veteran").unwrap();
    assert!((veteran - 90.0 * 0.65).abs() < 0.01, "veteran scales 0.65x; got {veteran}");
    // Hardcore = 0.4× (no compound floor).
    let hardcore = engine.m17_damage_type_ttd("human", "heart_wound", "hardcore").unwrap();
    assert!((hardcore - 90.0 * 0.4).abs() < 0.01);

    // The compound floor drops from 8s (default) to 5s (veteran).
    assert_eq!(cf_actor::AiDifficulty::ToughCrowd.compound_floor_seconds(), 8.0);
    assert_eq!(cf_actor::AiDifficulty::Veteran.compound_floor_seconds(), 5.0);
    assert_eq!(cf_actor::AiDifficulty::Hardcore.compound_floor_seconds(), 0.0);
}
