//! M16C acceptance — mental-health (Pain + 8 conditions + PTSD + addiction +
//! trauma + therapy) integration against the live M0Engine. Each test drives
//! the real engine tick (or the explicit-tick psych pass for time-gated
//! lifecycle paths) and asserts on engine state + the recorded event stream.

use cf_control::{M0Engine, M0EngineConfig, Scenario};
use cf_mental_health::{mh_roll, ConditionKind, SALT_OUTCOME, SALT_PANIC};
use cf_replay::Event;
use cf_wound::WoundKind;

fn locate_scenario(id: &str) -> std::path::PathBuf {
    let workspace = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    workspace.join("content").join("scenarios").join(format!("{id}.ron"))
}

fn engine_for(id: &str) -> M0Engine {
    let path = locate_scenario(id);
    let scenario = Scenario::load_from_file(&path).expect("scenario parses");
    let config = M0EngineConfig::for_loaded_scenario(&scenario, path);
    M0Engine::new(config)
}

fn engine_for_seed(id: &str, seed: u64) -> M0Engine {
    let path = locate_scenario(id);
    let scenario = Scenario::load_from_file(&path).expect("scenario parses");
    let mut config = M0EngineConfig::for_loaded_scenario(&scenario, path);
    config.seed = seed;
    M0Engine::new(config)
}

fn events(engine: &M0Engine) -> Vec<Event> {
    engine.recorder().snapshot_events()
}

fn has_event(engine: &M0Engine, category: &str, event_type: &str) -> bool {
    events(engine)
        .iter()
        .any(|e| e.category == category && e.event_type == event_type)
}

fn first_event(engine: &M0Engine, category: &str, event_type: &str) -> Option<Event> {
    events(engine)
        .into_iter()
        .find(|e| e.category == category && e.event_type == event_type)
}

/// 12h+ of ticks at 60 Hz (withdrawal absence threshold).
fn twelve_hours_ticks() -> u64 {
    (12.0 * 3600.0 * 60.0) as u64 + 600
}

/// 14d+ of ticks at 60 Hz (SSRI onset).
fn fourteen_days_ticks() -> u64 {
    (14.0 * 86_400.0 * 60.0) as u64 + 3_600
}

// ===========================================================================
// Scenario 1 — Pain stack drives aim wobble.
// ===========================================================================
#[test]
fn pain_stack_drives_aim_wobble() {
    let engine = engine_for("m16c_addiction_loop");
    engine.drive_tick();
    // 3 wounds: LacerationModerate(0.5), Burn2nd(0.6), FractureSimple(0.4).
    engine.m14g_inject_wound(1, WoundKind::LacerationModerate, "torso", 0.5);
    engine.m14g_inject_wound(1, WoundKind::Burn2nd, "torso", 0.6);
    engine.m14g_inject_wound(1, WoundKind::FractureSimple, "left_leg", 0.4);
    // Drive past a recompute-due tick (every 5 ticks).
    for _ in 0..8 {
        engine.drive_tick();
    }
    // The M16 affliction.pain shows severity ~0.6.
    let aff = engine.m16_actor_afflictions(1);
    let pain = aff
        .iter()
        .find(|(k, _)| k == "pain")
        .unwrap_or_else(|| panic!("pain affliction present; got {aff:?}"));
    assert!((pain.1 - 0.6).abs() < 0.05, "pain severity {} ~ 0.6", pain.1);
    // pain.stack_changed fired with new_stack = 18 and aim wobble 1.6.
    let ev = first_event(&engine, "pain", "stack_changed").expect("pain.stack_changed fired");
    let new_stack = ev.payload.get("new_stack").and_then(|v| v.as_f64()).unwrap();
    let wobble = ev.payload.get("aim_wobble_multiplier").and_then(|v| v.as_f64()).unwrap();
    assert!((new_stack - 18.0).abs() < 0.5, "new_stack {new_stack} ~ 18");
    assert!((wobble - 1.6).abs() < 0.02, "aim wobble {wobble} ~ 1.6");
}

// ===========================================================================
// M5/M16 consumer — Pain degrades aim + move speed through the engine bridge.
// ===========================================================================
#[test]
fn pain_degrades_aim_and_move_speed() {
    let engine = engine_for("m16c_addiction_loop");
    engine.drive_tick();
    // Baseline: unafflicted actor has identity combat modifiers.
    let (speed0, spread0) = engine.m16c_actor_combat_modifiers(1);
    assert!((speed0 - 1.0).abs() < 1e-6 && spread0.abs() < 1e-6, "unafflicted = identity");
    // 3 wounds → Pain severity ~0.6.
    engine.m14g_inject_wound(1, WoundKind::LacerationModerate, "torso", 0.5);
    engine.m14g_inject_wound(1, WoundKind::Burn2nd, "torso", 0.6);
    engine.m14g_inject_wound(1, WoundKind::FractureSimple, "left_leg", 0.4);
    for _ in 0..8 {
        engine.drive_tick();
    }
    let (speed, spread) = engine.m16c_actor_combat_modifiers(1);
    // Wounds drive BOTH Pain (severity 0.6 → ×0.82) and Bleeding (×~0.925),
    // so the combined move-speed multiplier ≈ 0.76 (< baseline 1.0). Aim
    // spread is positive (Pain wobble). This proves the aggregator multiplies
    // across all active afflictions and the engine bridge applies it.
    assert!(speed < 0.85 && speed > 0.65, "afflicted move-speed multiplier {speed} in (0.65,0.85)");
    assert!(spread > 0.0, "pain adds aim-spread bonus {spread}");
}

// ===========================================================================
// Scenario 2 + 8 — PTSD triggers from squad wipe; robot witness is filtered.
// ===========================================================================
#[test]
fn ptsd_triggers_from_squad_wipe_and_robot_is_filtered() {
    let engine = engine_for("m16c_squad_wipe_triggers_ptsd");
    engine.drive_tick();
    // Fatally wound the 3 human squadmates (actors 2, 3, 4).
    for sid in [2u64, 3, 4] {
        engine.m16c_apply_combat_damage(sid, 9_999.0);
    }
    // One tick: the psych pass detects the 3 deaths → the human leader (1)
    // witnesses 3 within 60s → PTSD; the robot squadmate (5) is immune.
    engine.drive_tick();

    let leader = engine.m16c_mental_health_summary(1);
    assert!(
        leader.iter().any(|(c, s)| c == "ptsd" && s == "acute"),
        "human leader has acute PTSD; got {leader:?}"
    );
    assert!(has_event(&engine, "psych", "condition_triggered"), "psych.condition_triggered fired");

    // Scenario 8 — the robot witness never develops a condition.
    let robot = engine.m16c_mental_health_summary(5);
    assert!(robot.is_empty(), "robot mental_health stays empty; got {robot:?}");
}

// ===========================================================================
// Scenario 3 — Addiction develops from sustained stim use; withdrawal on absence.
// ===========================================================================
#[test]
fn addiction_and_withdrawal_from_stim_use() {
    let engine = engine_for("m16c_addiction_loop");
    engine.drive_tick();
    // 7 combat-stim doses → addiction on the 7th.
    let mut developed = false;
    for _ in 0..7 {
        developed = engine.m16c_use_combat_stim(1);
    }
    assert!(developed, "the 7th dose develops addiction");
    assert!(engine.m16c_has_trait(1, "chronic_addiction"), "chronic_addiction trait granted");
    assert!(has_event(&engine, "psych", "addiction_developed"), "psych.addiction_developed fired");
    assert!(
        engine.m16c_mental_health_summary(1).iter().any(|(c, _)| c == "addiction"),
        "addiction condition present"
    );

    // Dose absent > 12h → withdrawal (2× aim wobble).
    engine.m16c_drive_psych_tick(twelve_hours_ticks());
    let wev = first_event(&engine, "psych", "withdrawal_started").expect("psych.withdrawal_started fired");
    let wobble = wev.payload.get("aim_wobble_multiplier").and_then(|v| v.as_f64()).unwrap();
    assert!((wobble - 2.0).abs() < 1e-6, "withdrawal aim wobble {wobble} = 2.0");
    assert!(
        engine.m16c_mental_health_summary(1).iter().any(|(c, _)| c == "withdrawal"),
        "withdrawal condition present"
    );
}

// ===========================================================================
// Scenario 4 + 7 — Therapy + SSRI → remission; recovered trait persists.
// ===========================================================================
#[test]
fn therapy_plus_ssri_reaches_remission_and_persists_trait() {
    // Choose a seed where actor 1's PTSD outcome roll lands in the remission
    // band (>= chronic_chance 0.30).
    let mut seed = 0u64;
    for s in 1..1_000_000u64 {
        if mh_roll(s, 1, ConditionKind::Ptsd, SALT_OUTCOME) >= 0.30 {
            seed = s;
            break;
        }
    }
    assert_ne!(seed, 0, "found a remission-band seed");
    let engine = engine_for_seed("m16c_addiction_loop", seed);
    engine.drive_tick();
    assert!(engine.m16c_trigger_condition(1, "ptsd", "witness_deaths"));
    for _ in 0..10 {
        engine.m16c_record_therapy_session(1, "ptsd");
    }
    assert!(engine.m16c_start_medication(1, "ptsd", "ssri"));
    assert!(has_event(&engine, "psych", "therapy_session"), "psych.therapy_session fired");
    assert!(has_event(&engine, "psych", "medication_started"), "psych.medication_started fired");

    // 14d on SSRI → onset met → treated remission.
    engine.m16c_drive_psych_tick(fourteen_days_ticks());
    assert!(has_event(&engine, "psych", "remission_achieved"), "psych.remission_achieved fired");
    assert!(
        engine.m16c_mental_health_summary(1).iter().any(|(c, s)| c == "ptsd" && s == "remission"),
        "PTSD in remission"
    );
    // Scenario 7 — the recovery trait persists onto the actor's TraitSet.
    assert!(engine.m16c_has_trait(1, "recovered_from_ptsd"), "recovered_from_ptsd trait persists");
}

// ===========================================================================
// Scenario 5 — Panic attack fires + freezes for 3-8s.
// ===========================================================================
#[test]
fn panic_attack_fires_for_panic_disorder() {
    let seed = 0x5151_5151_u64;
    let engine = engine_for_seed("m16c_addiction_loop", seed);
    engine.drive_tick();
    assert!(engine.m16c_trigger_condition(1, "panic_disorder", "panic_threshold"));
    // Find the first tick where the seeded panic roll fires for actor 1.
    let panic_tick = first_panic_tick(seed, 1);
    engine.m16c_drive_psych_tick(panic_tick);
    let ev = first_event(&engine, "psych", "panic_attack").expect("psych.panic_attack fired");
    let freeze = ev.payload.get("freeze_seconds").and_then(|v| v.as_f64()).unwrap();
    assert!((3.0..=8.0).contains(&freeze), "freeze {freeze} in [3,8]s");
}

fn first_panic_tick(seed: u64, actor: u64) -> u64 {
    for t in 1..5_000_000u64 {
        if mh_roll(seed, actor, ConditionKind::PanicDisorder, SALT_PANIC ^ t) < 0.000_50 {
            return t;
        }
    }
    panic!("no panic tick found within 5M ticks");
}

// ===========================================================================
// Scenario 6 — Insomnia prevents sleep.
// ===========================================================================
#[test]
fn insomnia_prevents_sleep() {
    let engine = engine_for("m16c_addiction_loop");
    engine.drive_tick();
    assert_eq!(engine.m16c_can_sleep(1), None, "no insomnia → sleep allowed");
    assert!(engine.m16c_trigger_condition(1, "insomnia", "sleep_deprivation"));
    assert_eq!(
        engine.m16c_can_sleep(1),
        Some("insomnia".to_string()),
        "active insomnia rejects sleep with reason=insomnia"
    );
}

// ===========================================================================
// Scenario 9 — Determinism: same seed reproduces panic-attack timing.
// ===========================================================================
#[test]
fn determinism_same_seed_reproduces_panic_timing() {
    let seed = 0xD00D_D00D_u64;
    let panic_tick = first_panic_tick(seed, 1);
    let run = || {
        let engine = engine_for_seed("m16c_addiction_loop", seed);
        engine.drive_tick();
        engine.m16c_trigger_condition(1, "panic_disorder", "panic_threshold");
        engine.m16c_drive_psych_tick(panic_tick);
        first_event(&engine, "psych", "panic_attack")
            .and_then(|e| e.payload.get("freeze_until_tick").and_then(|v| v.as_u64()))
    };
    let a = run();
    let b = run();
    assert!(a.is_some(), "panic fired");
    assert_eq!(a, b, "identical seed reproduces the panic freeze-until tick");
}
