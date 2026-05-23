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
/// Uses the bare cf-hazard world (no terrain affordance gating) because
/// the acceptance contract is "fire spreads to flammable" — full
/// terrain-affordance gating is exercised separately in
/// `fire_spread_gated_by_material_affordance` below.
#[test]
fn fire_spawns_spreads_and_douses() {
    let mut world = cf_hazard::HazardWorld::new();
    let reg = cf_hazard::HazardRegistry::default_registry();
    world.spawn(HazardKind::Fire, [0.0, 0.0], 1.0, 0, None);
    let mut total_spread = 0u32;
    for tick in 1..=120u64 {
        let out = world.tick_grid(&reg, tick, 60);
        total_spread += out.spread.len() as u32;
    }
    assert!(
        total_spread >= 1,
        "fire should have spread to ≥1 neighbor in 2 seconds; got {total_spread}"
    );
    // Apply water counter, then advance one tick → hazard.dissipated.
    let doused = world.apply_counter_radius(HazardKind::Fire, [0.0, 0.0], 5.0);
    assert!(doused >= 1, "water flask must douse ≥1 fire tile");
    let out = world.tick_grid(&reg, 121, 60);
    let any_doused = out
        .dissipated
        .iter()
        .any(|d| d.reason == cf_hazard::DissipationReason::Doused);
    assert!(any_doused, "douse must emit hazard.dissipated{{reason=doused}}");
}

#[test]
fn fire_spread_gated_by_material_affordance() {
    use cf_hazard::TileAffordance;
    let mut world = cf_hazard::HazardWorld::new();
    let reg = cf_hazard::HazardRegistry::default_registry();
    world.spawn(HazardKind::Fire, [0.0, 0.0], 1.0, 0, None);
    let solid_only = |_pos: [f32; 2]| TileAffordance::Solid;
    let flammable_only = |_pos: [f32; 2]| TileAffordance::Flammable;
    let mut got_solid = 0u32;
    for tick in 1..=120u64 {
        let out = world.tick_grid_with_affordance(&reg, tick, 60, solid_only);
        got_solid += out.spread.len() as u32;
    }
    assert_eq!(got_solid, 0, "fire must NOT spread to Solid tiles (concrete/dirt)");
    let mut got_flammable = 0u32;
    for tick in 121..=240u64 {
        let out = world.tick_grid_with_affordance(&reg, tick, 60, flammable_only);
        got_flammable += out.spread.len() as u32;
    }
    assert!(
        got_flammable >= 1,
        "fire MUST spread to Flammable tiles; got {got_flammable}"
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

/// VAL-M16-AUDIT-001: per-actor trigger thresholds gate auto-triage
/// reasons (Gherkin scenario 3).
#[test]
fn per_actor_trigger_thresholds_raise_threshold() {
    use cf_affliction::{ActiveAffliction, ActorAfflictions, AutoTriageReason, M16AfflictionKind, M16TriggerThresholds};
    let mut afflictions = ActorAfflictions::default();
    afflictions.active.push(ActiveAffliction {
        kind: M16AfflictionKind::Bleeding,
        severity: 0.5,
        applied_at_tick: 0,
        expected_clear_tick: None,
        source_event_id: None,
    });
    let default = M16TriggerThresholds::default();
    let emergency = M16TriggerThresholds::emergency_only();
    let with_default =
        cf_affliction::auto_triage_reasons(&afflictions, &default, 3, false, 0.9, 0.0, false, false, f32::INFINITY);
    assert!(
        with_default.contains(&AutoTriageReason::BleedingStack3),
        "default thresholds: 3 wounds at 90% HP MUST fire"
    );
    let with_emergency =
        cf_affliction::auto_triage_reasons(&afflictions, &emergency, 3, false, 0.9, 0.0, false, false, f32::INFINITY);
    assert!(
        !with_emergency.contains(&AutoTriageReason::BleedingStack3),
        "emergency thresholds: 3 wounds at 90% HP must NOT fire"
    );
}

/// VAL-M16-AUDIT-002: storyteller registry registers all M16 hazard
/// narrative event ids per spec § "Storyteller integration".
#[test]
fn storyteller_registers_m16_hazard_event_ids() {
    use cf_storyteller::{
        register_m16_narratives, M16NarrativeRegistry, NARRATIVE_EVENT_ID_ACID_POOL_GROWTH,
        NARRATIVE_EVENT_ID_ELECTRIC_ARC_CASCADE, NARRATIVE_EVENT_ID_FIRE_SPREAD,
        NARRATIVE_EVENT_ID_RADIATION_STORM,
    };
    let mut reg = M16NarrativeRegistry::new();
    register_m16_narratives(&mut reg);
    assert!(reg.get(NARRATIVE_EVENT_ID_FIRE_SPREAD).is_some());
    assert!(reg.get(NARRATIVE_EVENT_ID_ELECTRIC_ARC_CASCADE).is_some());
    assert!(reg.get(NARRATIVE_EVENT_ID_ACID_POOL_GROWTH).is_some());
    assert!(reg.get(NARRATIVE_EVENT_ID_RADIATION_STORM).is_some());
}

/// VAL-M16-AUDIT-003: engine surface reports the same registered
/// narrative ids.
#[test]
fn engine_m16_storyteller_event_ids_match_registry() {
    let engine = engine_for("m16_fire_spread_water_dousing");
    let ids = engine.m16_storyteller_event_ids();
    assert!(ids.iter().any(|s| s == "narrative.m16.fire_spread"));
    assert!(ids.iter().any(|s| s == "narrative.m16.radiation_storm"));
    assert_eq!(ids.len(), 6);
}

/// VAL-M16-AUDIT-004: utility scorer +0.4 bonus for TriageDownedAlly
/// shifts the Medic's chosen task.
#[test]
fn m16_utility_bonus_shifts_medic_choice() {
    use cf_ai::{utility::base_utility, ThinkingContext};
    use cf_ai::task::TaskType;
    let mut ctx = ThinkingContext::stub();
    ctx.downed_ally_in_squad = true;
    ctx.enemy_visible = true;
    ctx.enemy_distance_normalized = 0.1;
    ctx.m16_triage_bonus = 0.0;
    let without = base_utility(TaskType::TriageDownedAlly, &ctx);
    ctx.m16_triage_bonus = 0.4;
    let with_bonus = base_utility(TaskType::TriageDownedAlly, &ctx);
    assert!(
        with_bonus > without + 0.35,
        "M16 +0.4 bonus must add to TriageDownedAlly base utility (without={without}, with={with_bonus})"
    );
}

/// VAL-M16-AUDIT-005: vacuum exposure affliction kind is in the
/// schema enum + registry + race-aware TTD function.
#[test]
fn vacuum_exposure_kind_round_trips() {
    use cf_affliction::{vacuum_exposure_ttd_seconds, AfflictionRegistry, M16AfflictionKind, Race};
    let reg = AfflictionRegistry::default_registry();
    assert!(reg.specs.contains_key("vacuum_exposure"));
    assert_eq!(M16AfflictionKind::from_str("vacuum_exposure"), Some(M16AfflictionKind::VacuumExposure));
    assert!((vacuum_exposure_ttd_seconds(Race::Human) - 15.0).abs() < 1e-3);
    assert!(vacuum_exposure_ttd_seconds(Race::Methane).is_infinite());
    assert!((vacuum_exposure_ttd_seconds(Race::Crystalline) - 60.0).abs() < 1e-3);
}

/// VAL-M16-AUDIT-006: anomaly_detector item registered in cf_equipment.
#[test]
fn anomaly_detector_item_registered() {
    let spec = cf_equipment::spec_for_id(cf_equipment::sensor::ANOMALY_DETECTOR_ID);
    assert!(spec.is_some(), "anomaly_detector ItemSpec must exist");
}

/// VAL-M16-AUDIT-007: underwater weapons registered + tagged.
#[test]
fn underwater_weapons_registered() {
    assert!(cf_equipment::spec_for_id("harpoon").is_some());
    assert!(cf_equipment::spec_for_id("spear_gun").is_some());
    assert!(cf_swim::is_underwater_weapon("harpoon"));
    assert!(cf_swim::is_underwater_weapon("spear_gun"));
}

/// VAL-M16-AUDIT-008: tile_affordance translator handles spec-named
/// material families.
#[test]
fn tile_affordance_translator_maps_spec_families() {
    use cf_control::m16_tick::tile_affordance_for_material_name as f;
    use cf_hazard::TileAffordance;
    assert!(matches!(f("wood"), TileAffordance::Flammable));
    assert!(matches!(f("oil"), TileAffordance::Flammable));
    assert!(matches!(f("kerosene"), TileAffordance::Flammable));
    assert!(matches!(f("iron"), TileAffordance::Conductive));
    assert!(matches!(f("water"), TileAffordance::Water));
    assert!(matches!(f("concrete"), TileAffordance::Solid));
    assert!(matches!(f("air"), TileAffordance::Empty));
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
