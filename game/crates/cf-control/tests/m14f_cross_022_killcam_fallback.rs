//! **VAL-CROSS-022** — cf-killcam variants are suppressed on M14E
//! cave-in and M14F wall-rupture deaths.
//!
//! VAL-M14C-013 registers `heat_penetration` + `apfsds_through_module`
//! killcam variants triggered by `armor.heat_jet_traversed` /
//! `armor.apfsds_long_rod_through`. An actor killed by an M14E falling-
//! debris impulse (VAL-M14E-005) or an M14F wall-rupture debris cone
//! (VAL-CROSS-008) must NOT trigger either M14C variant — the killcam
//! falls back to the M14 default debris/fall variant because no HEAT
//! or APFSDS event fired.
//!
//! These tests live in `cf-control/tests/` because they need to drive
//! full headless scenarios (cf-killcam itself has no scenario / engine
//! dependency).

use cf_control::{M0Engine, M0EngineConfig, Scenario};
use cf_killcam::{
    dispatch_pair_contact_variant, dispatch_variant, ApfsdsThroughModulePayload, HeatPenetrationPayload,
    KillcamVariant, KillcamVariantTrigger, ProjectilePairContactPayload, APFSDS_THROUGH_MODULE_VARIANT_ID,
    DEFAULT_REPLAY_INTERCEPTS, HEAT_PENETRATION_VARIANT_ID,
};

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

/// Walk the engine's event log and build the killcam variant queue
/// the dispatcher would produce on each death-event source.
///
/// Per VAL-M14C-013, the M14C variants fire only on
/// `armor.heat_jet_traversed` (HEAT) and `armor.apfsds_long_rod_through`
/// (APFSDS). Per VAL-M14D-019 + VAL-CROSS-004,
/// `collision.projectile_pair_contact` is excluded from the queue by
/// default (`replay_intercepts=false`). Everything else (including
/// cave-in / wall-rupture deaths) falls back to the `Default` variant.
fn build_killcam_queue(events: &[cf_replay::Event]) -> Vec<KillcamVariant> {
    let mut queue = Vec::new();
    for event in events {
        match (event.category.as_str(), event.event_type.as_str()) {
            ("armor", "heat_jet_traversed") => {
                queue.push(dispatch_variant(
                    KillcamVariantTrigger::HeatJetTraversed,
                    Some(HeatPenetrationPayload::default()),
                    None,
                ));
            }
            ("armor", "apfsds_long_rod_through") => {
                queue.push(dispatch_variant(
                    KillcamVariantTrigger::ApfsdsLongRodThrough,
                    None,
                    Some(ApfsdsThroughModulePayload::default()),
                ));
            }
            ("collision", "projectile_pair_contact") => {
                queue.push(dispatch_pair_contact_variant(
                    ProjectilePairContactPayload::default(),
                    DEFAULT_REPLAY_INTERCEPTS,
                ));
            }
            _ => {}
        }
    }
    queue
}

fn count_events(events: &[cf_replay::Event], category: &str, event_type: &str) -> usize {
    events
        .iter()
        .filter(|e| e.category == category && e.event_type == event_type)
        .count()
}

/// **VAL-CROSS-022 (M14E half)**: drive `m14e_tunnel_collapse_drill.ron`
/// with an actor under the 32-px collapse bbox. The collapse must fire
/// AND the killcam variant queue must contain zero M14C HEAT/APFSDS
/// variants — only the default debris/fall variant applies for a
/// cave-in death.
#[test]
fn val_cross_022_m14e_cave_in_does_not_trigger_heat_or_apfsds_variants() {
    let (_engine, events) = drive_scenario("m14e_tunnel_collapse_drill", 600);

    // Sanity: the cave-in fired so the actor was under a real collapse
    // bbox (per VAL-M14E-003).
    let cave_ins = count_events(&events, "terrain", "cave_in_triggered");
    assert!(
        cave_ins >= 1,
        "expected terrain.cave_in_triggered in m14e_tunnel_collapse_drill; got {cave_ins}"
    );

    // Per VAL-M14C-013: M14C killcam variants only fire from
    // `armor.heat_jet_traversed` / `armor.apfsds_long_rod_through`. The
    // cave-in scenario uses no HEAT/APFSDS ammo, so neither event may
    // appear in the log.
    let heat_events = count_events(&events, "armor", "heat_jet_traversed");
    let apfsds_events = count_events(&events, "armor", "apfsds_long_rod_through");
    assert_eq!(
        heat_events, 0,
        "M14E cave-in must not emit armor.heat_jet_traversed; got {heat_events}"
    );
    assert_eq!(
        apfsds_events, 0,
        "M14E cave-in must not emit armor.apfsds_long_rod_through; got {apfsds_events}"
    );

    // Per VAL-CROSS-022: the queue contains zero HEAT / APFSDS variants.
    let queue = build_killcam_queue(&events);
    let heat_variants = queue
        .iter()
        .filter(|v| matches!(v, KillcamVariant::HeatPenetration(_)))
        .count();
    let apfsds_variants = queue
        .iter()
        .filter(|v| matches!(v, KillcamVariant::ApfsdsThroughModule(_)))
        .count();
    assert_eq!(
        heat_variants, 0,
        "killcam queue must not contain heat_penetration variants on cave-in death"
    );
    assert_eq!(
        apfsds_variants, 0,
        "killcam queue must not contain apfsds_through_module variants on cave-in death"
    );

    // The fallback Default variant is what fires for a cave-in death.
    let fallback = dispatch_variant(KillcamVariantTrigger::Default, None, None);
    assert!(fallback.is_default());
    assert!(fallback.id().is_default());
}

/// **VAL-CROSS-022 (M14F half)**: drive `m14f_bunker_siege_wall_fail.ron`
/// with an actor in the wall-rupture debris cone. The rupture must
/// fire AND the killcam variant queue must contain zero M14C
/// HEAT/APFSDS variants — only the default debris/fall variant applies
/// for a wall-rupture death.
#[test]
fn val_cross_022_m14f_wall_rupture_does_not_trigger_heat_or_apfsds_variants() {
    let (_engine, events) = drive_scenario("m14f_bunker_siege_wall_fail", 600);

    // Sanity: the rupture fired so the actor was caught in a real
    // wall-rupture event (per VAL-M14F-010).
    let ruptures = count_events(&events, "terrain", "wall_rupture");
    assert!(
        ruptures >= 1,
        "expected terrain.wall_rupture in m14f_bunker_siege_wall_fail; got {ruptures}"
    );

    // Per VAL-M14C-013: HEAT/APFSDS variants require the corresponding
    // armor traversal events. None may fire from a wall-rupture
    // scenario.
    let heat_events = count_events(&events, "armor", "heat_jet_traversed");
    let apfsds_events = count_events(&events, "armor", "apfsds_long_rod_through");
    assert_eq!(
        heat_events, 0,
        "M14F wall-rupture must not emit armor.heat_jet_traversed; got {heat_events}"
    );
    assert_eq!(
        apfsds_events, 0,
        "M14F wall-rupture must not emit armor.apfsds_long_rod_through; got {apfsds_events}"
    );

    // Per VAL-CROSS-022 + VAL-CROSS-004: the queue contains zero
    // HEAT / APFSDS / non-default variants. (Pair-contact variants are
    // excluded by default per VAL-M14D-019.)
    let queue = build_killcam_queue(&events);
    let heat_variants = queue
        .iter()
        .filter(|v| matches!(v, KillcamVariant::HeatPenetration(_)))
        .count();
    let apfsds_variants = queue
        .iter()
        .filter(|v| matches!(v, KillcamVariant::ApfsdsThroughModule(_)))
        .count();
    assert_eq!(
        heat_variants, 0,
        "killcam queue must not contain heat_penetration variants on wall-rupture death"
    );
    assert_eq!(
        apfsds_variants, 0,
        "killcam queue must not contain apfsds_through_module variants on wall-rupture death"
    );

    // Default trigger returns the fallback variant for a wall-rupture
    // death. The killcam queue collapses to this fallback only.
    let fallback = dispatch_variant(KillcamVariantTrigger::Default, None, None);
    assert!(fallback.is_default());
}

/// **VAL-CROSS-022** sanity check: confirm the M14C variant ids are
/// still distinct from the default fallback. Guards against a
/// regression that would alias them to id=0 (which would make the
/// negative assertions above vacuously pass).
#[test]
fn val_cross_022_heat_apfsds_variant_ids_remain_non_default() {
    assert!(!HEAT_PENETRATION_VARIANT_ID.is_default());
    assert!(!APFSDS_THROUGH_MODULE_VARIANT_ID.is_default());
    assert_ne!(HEAT_PENETRATION_VARIANT_ID, APFSDS_THROUGH_MODULE_VARIANT_ID);
}
