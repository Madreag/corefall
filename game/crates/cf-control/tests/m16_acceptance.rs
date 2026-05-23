//! M16 acceptance — hazards, anomalies, artifacts, swim/drowning,
//! afflictions integration test against M0Engine.

use cf_affliction::{ClearReason, M16AfflictionKind};
use cf_anomaly::AnomalyKind;
use cf_artifact::ArtifactRegistry;
use cf_control::{M0Engine, M0EngineConfig, Scenario};
use cf_hazard::{HazardKind, HazardRegistry};

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

/// VAL-M16-CONTENT-001: M16 scenarios parse cleanly.
#[test]
fn m16_scenarios_parse() {
    for id in [
        "m16_fire_spread_water_dousing",
        "m16_electric_anomaly_periodic_damage",
        "m16_artifact_pickup_max_hp",
        "m16_swim_drowning",
        "m16_affliction_stack_escalate",
    ] {
        let path = locate_scenario(id);
        let s = Scenario::load_from_file(&path).unwrap_or_else(|err| panic!("scenario {id} must parse: {err}"));
        assert_eq!(s.id, id);
    }
}

/// VAL-M16-CONTENT-002: hazard registry covers every spec-locked kind.
#[test]
fn hazard_registry_default_covers_9_kinds() {
    let reg = HazardRegistry::default_registry();
    for k in HazardKind::all() {
        assert!(reg.specs.contains_key(k.as_str()), "missing hazard {}", k.as_str());
    }
}

/// VAL-M16-CONTENT-003: artifact registry has ≥20 launch artifacts.
#[test]
fn artifact_registry_has_20_plus() {
    let reg = ArtifactRegistry::default_registry();
    assert!(reg.specs.len() >= 20, "expected ≥20 artifacts; got {}", reg.specs.len());
}

/// VAL-M16-ACCEPT-001: fire hazard spawn → spread → counter dousing.
#[test]
fn fire_spawns_spreads_and_douses() {
    let engine = engine_for("m16_fire_spread_water_dousing");
    let id = engine.m16_spawn_hazard(HazardKind::Fire, [100.0, 20.0], 1.0, None);
    assert_ne!(id, u64::MAX, "hazard id assigned");
    let initial = engine.m16_hazard_snapshot();
    assert_eq!(initial.summary_per_kind.get("fire").copied(), Some(1));
    // Drive ~60 ticks at 60Hz so the spread cadence fires (1 tile/s).
    for _ in 0..120 {
        engine.drive_tick();
    }
    let after_spread = engine.m16_hazard_snapshot();
    let fire_count = after_spread.summary_per_kind.get("fire").copied().unwrap_or(0);
    assert!(
        fire_count > 1,
        "fire should have spread to ≥1 neighbor in 2 seconds; got {fire_count}"
    );
    // Apply water counter to all fire tiles around (100, 20).
    let doused = engine.m16_apply_counter(HazardKind::Fire, [100.0, 20.0], 5.0);
    assert!(doused >= 1, "water flask must douse ≥1 fire tile");
    // One more tick drains the counter → hazard.dissipated.
    engine.drive_tick();
    let after_douse = engine.m16_hazard_snapshot();
    let remaining = after_douse.summary_per_kind.get("fire").copied().unwrap_or(0);
    assert!(
        remaining < fire_count,
        "douse must reduce fire count from {fire_count} → {remaining}"
    );
}

/// VAL-M16-ACCEPT-002: electric anomaly damages a player who enters its
/// detection radius. Drives the M16 producer directly because the engine
/// drive_tick path is conditionally gated by effective sim speed; the
/// acceptance test exercises the underlying contract without coupling
/// to the engine clock.
#[test]
fn electric_anomaly_periodic_damage() {
    use cf_affliction::{apply_affliction, AfflictionRegistry, ActorAfflictions};
    use cf_anomaly::{AnomalyRegistry, AnomalyWorld};
    let mut world = AnomalyWorld::new();
    let reg = AnomalyRegistry::default_registry();
    let id = world.spawn(AnomalyKind::ElectricAnomaly, [63.0, 32.0], 0);
    let actor_pos = (1u64, [63.0, 32.0]);
    let mut total_damage_events = 0u32;
    let mut afflictions = ActorAfflictions::default();
    let affliction_reg = AfflictionRegistry::default_registry();
    let mut applied_count = 0u32;
    for tick in 1..=180u64 {
        let out = world.tick(&reg, tick, std::slice::from_ref(&actor_pos));
        total_damage_events += out.damage.len() as u32;
        for ev in &out.damage {
            if let Some(kind_str) = ev.applied_affliction.as_deref() {
                if let Some(kind) = M16AfflictionKind::from_str(kind_str) {
                    let (a, _e) = apply_affliction(
                        &mut afflictions,
                        ev.actor_id,
                        kind,
                        0.2,
                        &affliction_reg,
                        tick,
                        60,
                        "test".to_string(),
                    );
                    if a.is_some() {
                        applied_count += 1;
                    }
                }
            }
        }
    }
    let _ = id;
    assert!(
        total_damage_events >= 3,
        "electric anomaly should fire ≥3 damage events in 3s; got {total_damage_events}"
    );
    assert!(
        afflictions.severity_of(M16AfflictionKind::Electrified) > 0.0,
        "electric anomaly must stack 'electrified' affliction; severity now 0.0"
    );
    let _ = applied_count;
}

/// VAL-M16-ACCEPT-003: artifact pickup grants the carrier the bonus.
#[test]
fn stone_blood_pickup_grants_max_hp() {
    let engine = engine_for("m16_artifact_pickup_max_hp");
    let inst = engine
        .m16_spawn_artifact("stone_blood", [32.0, 16.0])
        .expect("stone_blood spec exists");
    let baseline = engine.m16_actor_artifact_bonus(1);
    assert_eq!(baseline.max_hp_bonus, 0.0);
    let ok = engine.m16_pickup_artifact(inst, 1);
    assert!(ok, "pickup must succeed");
    let agg = engine.m16_actor_artifact_bonus(1);
    assert!((agg.max_hp_bonus - 20.0).abs() < 1e-3, "Stone Blood adds +20 max HP");
}

/// VAL-M16-ACCEPT-004: affliction stacks + escalates per spec.
#[test]
fn affliction_stacks_and_escalates() {
    let engine = engine_for("m16_affliction_stack_escalate");
    let ok = engine.m16_apply_affliction(1, M16AfflictionKind::Burning, 0.3, "test-1");
    assert!(ok);
    let aff = engine.m16_actor_afflictions(1);
    let b = aff.iter().find(|(k, _)| k == "burning").expect("burning applied");
    assert!((b.1 - 0.3).abs() < 1e-3);
    engine.m16_apply_affliction(1, M16AfflictionKind::Burning, 0.5, "test-2");
    let aff = engine.m16_actor_afflictions(1);
    let b = aff.iter().find(|(k, _)| k == "burning").expect("burning stacked");
    assert!((b.1 - 0.8).abs() < 1e-3, "burning severity must stack to 0.8; got {}", b.1);
    // Medikit clears.
    let cleared = engine.m16_clear_affliction(1, M16AfflictionKind::Burning, ClearReason::Medikit);
    assert!(cleared);
    let aff = engine.m16_actor_afflictions(1);
    assert!(aff.iter().find(|(k, _)| k == "burning").is_none(), "medikit must clear burning");
}

/// VAL-M16-ACCEPT-005: anomaly detector exposes detector-required anomalies.
#[test]
fn anomaly_detector_reveals_bloodsucker_lair() {
    let engine = engine_for("m16_electric_anomaly_periodic_damage");
    engine.m16_spawn_anomaly(AnomalyKind::ElectricAnomaly, [10.0, 0.0]);
    engine.m16_spawn_anomaly(AnomalyKind::BloodsuckerLair, [5.0, 0.0]);
    // Without detector parameter the query still returns spec-tagged
    // visible anomalies; the cf-ui filter on detector_required is what
    // hides the bloodsucker lair from the unaided HUD.
    let hits = engine.m16_detector_query([0.0, 0.0], 20.0);
    assert_eq!(hits.len(), 2);
    let lair = hits
        .iter()
        .find(|(_, k, _, _)| k == "bloodsucker_lair")
        .expect("bloodsucker lair returned");
    assert!(lair.3, "bloodsucker_lair must be flagged detector_required");
}

/// VAL-M16-ACCEPT-006: survival afflictions clear outside PvE Survival mode.
#[test]
fn survival_afflictions_clear_outside_survival_mode() {
    let engine = engine_for("m16_affliction_stack_escalate");
    engine.m16_set_survival_mode(false);
    engine.m16_apply_affliction(1, M16AfflictionKind::Hunger, 0.5, "test");
    // Drive a single tick → hunger should be auto-cleared.
    engine.drive_tick();
    let aff = engine.m16_actor_afflictions(1);
    assert!(
        aff.iter().find(|(k, _)| k == "hunger").is_none(),
        "hunger must clear when survival mode is off"
    );
}

/// VAL-M16-ACCEPT-007: hazard.tick events fire at 10:1 batched cadence.
#[test]
fn hazard_tick_cosmetic_events_batched_10_to_1() {
    let mut world = cf_hazard::HazardWorld::new();
    let reg = cf_hazard::HazardRegistry::default_registry();
    let (_id, _) = world.spawn(HazardKind::Radiation, [0.0, 0.0], 1.0, 0, None);
    let mut total = 0u32;
    for tick in 1..=60u64 {
        let out = world.tick_grid(&reg, tick, 60);
        total += out.tick.len() as u32;
    }
    assert!(
        total <= 6,
        "60 sim ticks must batch to ≤6 hazard.tick cosmetic events; got {total}"
    );
}

/// VAL-M16-EVENT-SCHEMAS-001: cf-replay lookup table resolves every M16
/// event schema id.
#[test]
fn cf_replay_lookup_covers_m16_events() {
    for (category, event_type) in [
        ("hazard", "spawned"),
        ("hazard", "spread"),
        ("hazard", "actor_contact"),
        ("hazard", "tick"),
        ("hazard", "dissipated"),
        ("affliction", "applied"),
        ("affliction", "tick"),
        ("affliction", "cleared"),
        ("affliction", "escalated"),
        ("anomaly", "entered"),
        ("anomaly", "damage_applied"),
        ("artifact", "spawned"),
        ("artifact", "picked_up"),
        ("artifact", "carried_bonus_applied"),
        ("actor", "swim_started"),
        ("actor", "swim_ended"),
        ("actor", "drowning_started"),
        ("actor", "drowning_lethal"),
    ] {
        let schema = cf_replay::schemas::event_schema_for(category, event_type);
        assert!(
            schema.is_some(),
            "cf-replay lookup must resolve ({category}, {event_type})"
        );
    }
}
