//! M7B acceptance scenarios — verifies the deep squad-command grammar,
//! formation orders, breach chain, doctrine veto, brain-hop, bounding
//! retreat, and per-archetype BT contracts end-to-end against the
//! engine-side `M7BSquadWorld`.

#![allow(clippy::manual_contains)]

use serde_json::Value;

use cf_ai::{
    archetype_bt::{node_ids_for, ArchetypeBtKind},
    formation::FormationKind,
    squad_command_grammar::{builtin_registry, CommandIssue, DoctrineCompatMatrix},
    squad_state::{BoundingPhase, BreachChainStep, RoleAssignmentResult},
    DoctrineMode, SquadRoleHint, VerbArgValue,
};
use cf_control::m7b_squad::{M7BSquadWorld, PLAYER_SQUAD_ID};

// ============================================================================
// Scenario: Verb registry enumerates 50+ named squad commands
// ============================================================================

#[test]
fn verb_registry_enumerates_at_least_50_named_squad_commands() {
    let r = builtin_registry();
    assert!(r.len() >= 50, "verb registry has only {} entries", r.len());
    for def in r.iter() {
        assert!(!def.verb_id.is_empty(), "verb_id must be non-empty");
        assert!(!def.display_name.is_empty(), "display_name must be non-empty");
        assert!(!def.valid_target.is_empty(), "valid_target predicate label must be non-empty");
        let _ = def.family; // ensures the family enum is present
    }
    let matrix = DoctrineCompatMatrix::builtin();
    assert!(matrix.veto_reason(DoctrineMode::Defensive, "press_attack").is_some());
}

#[test]
fn dump_squad_state_view_lists_registry_formations_and_archetype_bts() {
    let world = M7BSquadWorld::new();
    let view = world.dump_state_view(PLAYER_SQUAD_ID);
    assert!(view.get("verb_registry_count").and_then(Value::as_u64).unwrap_or(0) >= 50);
    let verbs = view.get("verb_registry").and_then(Value::as_array).expect("verbs");
    assert!(verbs.len() >= 50);
    let formations = view.get("formations").and_then(Value::as_array).expect("formations");
    assert_eq!(formations.len(), 9, "9 formation kinds must be listed");
    let bts = view.get("archetype_bt").and_then(Value::as_array).expect("archetype_bt");
    assert_eq!(bts.len(), 6, "6 archetype BTs must be listed");
    for bt in bts {
        assert!(bt.get("node_count").and_then(Value::as_u64).unwrap_or(0) >= 30);
    }
}

// ============================================================================
// Scenario: Wedge formation resolves slots and survives a member drop
// ============================================================================

#[test]
fn wedge_formation_resolves_five_slots_and_collapses_on_kia() {
    let mut world = M7BSquadWorld::new();
    let squad = world.ensure_squad(PLAYER_SQUAD_ID, DoctrineMode::Defensive);
    squad.assign_role(1, SquadRoleHint::SquadLeader);
    squad.assign_role(2, SquadRoleHint::Rifleman);
    squad.assign_role(3, SquadRoleHint::Rifleman);
    squad.assign_role(4, SquadRoleHint::Heavy);
    squad.assign_role(5, SquadRoleHint::Heavy);
    let out = world.set_formation(PLAYER_SQUAD_ID, FormationKind::Wedge, [0.0, 0.0], 0.0, Some(1), 100);
    assert_eq!(out.new_kind, FormationKind::Wedge);
    assert_eq!(out.assignment_payloads.len(), 5);

    // Member dies, formation should collapse on next solve.
    let squad = world.squad_mut(PLAYER_SQUAD_ID).expect("squad");
    assert!(squad.on_member_kia(5));
    // Force a reslove at a later tick (within the 2s window).
    let solved = squad.solve_slots([0.0, 0.0], 0.0, 220);
    assert_eq!(solved.len(), 4, "only 4 surviving members map to slots");
}

#[test]
fn formation_collapse_step_chains_through_diamond_column_single_file() {
    assert_eq!(
        FormationKind::Wedge.collapse_step(),
        Some(FormationKind::Diamond)
    );
    assert_eq!(
        FormationKind::Diamond.collapse_step(),
        Some(FormationKind::Column)
    );
    assert_eq!(
        FormationKind::Column.collapse_step(),
        Some(FormationKind::SingleFile)
    );
    assert_eq!(FormationKind::SingleFile.collapse_step(), None);
}

// ============================================================================
// Scenario: Stack-Breach-Frag-Advance chain composes end-to-end
// ============================================================================

#[test]
fn breach_chain_fires_four_steps_in_order_then_completes() {
    let mut world = M7BSquadWorld::new();
    let start = world.start_breach_chain(PLAYER_SQUAD_ID, 42, "left", vec![1, 2, 3, 4], 100);
    assert_eq!(start.get("door_id").and_then(Value::as_u64), Some(42));
    assert_eq!(start.get("side").and_then(Value::as_str), Some("left"));
    // Sectors-of-fire pre-assigned per spec § "per-actor sectors-of-fire
    // are pre-assigned per slot (Stack-1 ahead, Stack-2 left, Stack-3
    // right, Stack-4 rear)".
    let sectors = start
        .get("sectors_of_fire")
        .and_then(Value::as_array)
        .expect("sectors_of_fire on start event");
    assert_eq!(sectors.len(), 4);

    let step1 = world.advance_breach_chain_with_actors(PLAYER_SQUAD_ID, 101, &[1, 2, 3, 4]);
    let p1 = step1.step_payload.expect("step 1 payload");
    assert_eq!(p1.get("step").and_then(Value::as_str), Some("breach"));
    assert!(step1.complete_payload.is_none());
    assert!(p1.get("sectors_of_fire").and_then(Value::as_array).is_some());

    let step2 = world.advance_breach_chain_with_actors(PLAYER_SQUAD_ID, 102, &[1, 2, 3, 4]);
    let p2 = step2.step_payload.expect("step 2 payload");
    assert_eq!(p2.get("step").and_then(Value::as_str), Some("frag"));

    let step3 = world.advance_breach_chain_with_actors(PLAYER_SQUAD_ID, 103, &[1, 2, 3, 4]);
    let p3 = step3.step_payload.expect("step 3 payload");
    assert_eq!(p3.get("step").and_then(Value::as_str), Some("advance"));

    let final_res = world.advance_breach_chain_with_actors(PLAYER_SQUAD_ID, 104, &[1, 2, 3, 4]);
    assert!(final_res.step_payload.is_none());
    let complete = final_res.complete_payload.expect("complete payload");
    assert_eq!(complete.get("door_id").and_then(Value::as_u64), Some(42));
}

#[test]
fn breach_chain_step_ordinals_monotonic() {
    let expected = [
        BreachChainStep::Stack,
        BreachChainStep::Breach,
        BreachChainStep::Frag,
        BreachChainStep::Advance,
    ];
    for (i, step) in expected.iter().enumerate() {
        assert_eq!(step.ordinal() as usize, i, "step {step:?} ordinal mismatch");
    }
}

// ============================================================================
// Scenario: Suppress vs Overwatch issue distinct BT subtrees
// ============================================================================

#[test]
fn suppress_and_overwatch_produce_distinct_bt_subtrees() {
    // Per acceptance: a Rifleman issuing `Suppress (window)` should engage
    // sustained suppressive fire; `Overwatch (sector)` should hold fire
    // until target enters sector. These map to distinct BT subtrees.
    let suppress = cf_ai::archetype_bt::bt_for(ArchetypeBtKind::Rifleman, cf_ai::TaskType::SuppressFire);
    let suppress_label = suppress.flatten_label();
    assert!(suppress_label.contains("rifleman_suppress_target"));

    // Overwatch is consumed via the BT node id list (verb registry maps
    // overwatch_sector to a hold-and-engage chain).
    let nodes = node_ids_for(ArchetypeBtKind::Rifleman);
    assert!(nodes.iter().any(|n| *n == "rifleman_overwatch_sector"));
    assert!(nodes.iter().any(|n| *n == "rifleman_suppress_target"));
    assert!(nodes.iter().any(|n| *n == "rifleman_suppress_window"));
    // Suppression node ≠ overwatch node (distinct subtree leaves).
    assert_ne!(
        nodes.iter().find(|n| **n == "rifleman_suppress_target"),
        nodes.iter().find(|n| **n == "rifleman_overwatch_sector"),
    );
}

// ============================================================================
// Scenario: Doctrine veto rejects incompatible verb
// ============================================================================

#[test]
fn defensive_doctrine_vetoes_press_attack_with_canonical_label() {
    let mut world = M7BSquadWorld::new();
    world.ensure_squad(PLAYER_SQUAD_ID, DoctrineMode::Defensive);
    let outcome = world.issue_verb(PLAYER_SQUAD_ID, "press_attack", vec![], 1, 100);
    assert!(!outcome.is_accepted());
    assert!(matches!(outcome.outcome, CommandIssue::Vetoed { .. }));
    assert_eq!(
        outcome.payload.get("reason_label").and_then(Value::as_str),
        Some("doctrine_defensive_blocks_press_attack")
    );
    // Previous command persists (none was set, so still None).
    let squad = world.squad(PLAYER_SQUAD_ID).expect("squad");
    assert!(squad.current_command.is_none());
    assert!(squad.last_veto_label.is_some());
}

#[test]
fn re_issue_after_doctrine_switch_accepted() {
    let mut world = M7BSquadWorld::new();
    let squad = world.ensure_squad(PLAYER_SQUAD_ID, DoctrineMode::Defensive);
    squad.doctrine = DoctrineMode::Aggressive;
    let outcome = world.issue_verb(PLAYER_SQUAD_ID, "press_attack", vec![], 1, 101);
    assert!(outcome.is_accepted());
}

// ============================================================================
// Scenario: Commander brain-hop preserves squad doctrine
// ============================================================================

#[test]
fn brain_hop_preserves_squad_doctrine_formation_and_roles() {
    let mut world = M7BSquadWorld::new();
    let squad = world.ensure_squad(PLAYER_SQUAD_ID, DoctrineMode::Aggressive);
    squad.assign_role(1, SquadRoleHint::SquadLeader);
    squad.assign_role(2, SquadRoleHint::Rifleman);
    squad.assign_role(3, SquadRoleHint::Rifleman);
    let _ = squad.try_issue_builtin("advance", vec![], 1, 100);
    let _ = world.set_formation(PLAYER_SQUAD_ID, FormationKind::Wedge, [0.0, 0.0], 0.0, Some(1), 100);

    // Snapshot the squad state before the hop.
    let pre_doctrine = world.squad(PLAYER_SQUAD_ID).unwrap().doctrine;
    let pre_formation = world.squad(PLAYER_SQUAD_ID).unwrap().formation_kind;
    let pre_roles = world.squad(PLAYER_SQUAD_ID).unwrap().role_assignments.clone();
    let pre_command = world.squad(PLAYER_SQUAD_ID).unwrap().current_command.clone();

    let payload = world.brain_hop_payload(PLAYER_SQUAD_ID, 1, 2, 110, true);
    assert_eq!(payload.get("from_actor_id").and_then(Value::as_u64), Some(1));
    assert_eq!(payload.get("to_actor_id").and_then(Value::as_u64), Some(2));
    assert_eq!(payload.get("same_squad").and_then(Value::as_bool), Some(true));

    // Squad state is unchanged.
    let post = world.squad(PLAYER_SQUAD_ID).unwrap();
    assert_eq!(post.doctrine, pre_doctrine);
    assert_eq!(post.formation_kind, pre_formation);
    assert_eq!(post.role_assignments, pre_roles);
    assert_eq!(post.current_command, pre_command);
}

// ============================================================================
// Scenario: Bounding retreat alternates cover and movement
// ============================================================================

#[test]
fn bounding_retreat_alternates_cover_and_movement_with_event_per_swap() {
    let mut world = M7BSquadWorld::new();
    let squad = world.ensure_squad(PLAYER_SQUAD_ID, DoctrineMode::Aggressive);
    squad.assign_role(1, SquadRoleHint::SquadLeader);
    squad.assign_role(2, SquadRoleHint::Rifleman);
    squad.assign_role(3, SquadRoleHint::Heavy);
    squad.assign_role(4, SquadRoleHint::Rifleman);
    squad.start_bounding([30.0, 0.0], 100);

    let p1 = world.tick_bounding(PLAYER_SQUAD_ID, Some([30.0, 0.0])).unwrap();
    assert_eq!(p1.get("step_index").and_then(Value::as_u64), Some(1));
    assert_eq!(
        p1.get("new_phase").and_then(Value::as_str),
        Some(BoundingPhase::MoveHalf.as_str())
    );

    let p2 = world.tick_bounding(PLAYER_SQUAD_ID, Some([30.0, 0.0])).unwrap();
    assert_eq!(p2.get("step_index").and_then(Value::as_u64), Some(2));
    assert_eq!(
        p2.get("new_phase").and_then(Value::as_str),
        Some(BoundingPhase::CoverHalf.as_str())
    );

    let p3 = world.tick_bounding(PLAYER_SQUAD_ID, Some([30.0, 0.0])).unwrap();
    assert_eq!(
        p3.get("new_phase").and_then(Value::as_str),
        Some(BoundingPhase::MoveHalf.as_str())
    );
}

// ============================================================================
// Scenario: Per-archetype BT exposes 30+ nodes
// ============================================================================

#[test]
fn each_archetype_bt_exposes_at_least_30_distinct_nodes() {
    for kind in ArchetypeBtKind::ALL {
        let nodes = node_ids_for(kind);
        assert!(
            nodes.len() >= 30,
            "{:?} exposes only {} nodes (floor is 30)",
            kind,
            nodes.len()
        );
        let mut ids: Vec<&str> = nodes.to_vec();
        ids.sort_unstable();
        for w in ids.windows(2) {
            assert_ne!(w[0], w[1], "duplicate node id {:?} in {:?}", w[0], kind);
        }
    }
}

// ============================================================================
// Scenario: Replay determinism across hop + breach
// ============================================================================

#[test]
fn replay_payloads_deterministic_across_two_runs() {
    fn run() -> Vec<Value> {
        let mut world = M7BSquadWorld::new();
        let mut events: Vec<Value> = Vec::new();
        let squad = world.ensure_squad(PLAYER_SQUAD_ID, DoctrineMode::Aggressive);
        squad.assign_role(1, SquadRoleHint::SquadLeader);
        squad.assign_role(2, SquadRoleHint::Pointman);
        squad.assign_role(3, SquadRoleHint::Rifleman);
        squad.assign_role(4, SquadRoleHint::Heavy);

        events.push(world.brain_hop_payload(PLAYER_SQUAD_ID, 1, 2, 100, true));
        events.push(world.start_breach_chain(PLAYER_SQUAD_ID, 42, "left", vec![1, 2, 3, 4], 110));
        for tick in 111..=114 {
            let res = world.advance_breach_chain(PLAYER_SQUAD_ID, tick);
            if let Some(p) = res.step_payload {
                events.push(p);
            }
            if let Some(c) = res.complete_payload {
                events.push(c);
            }
        }
        events
    }

    let a = run();
    let b = run();
    assert_eq!(a, b, "M7B events must be bit-exact across two identical runs");
}

// ============================================================================
// Scenario: Verb + formation registries surface to UI
//
// The cf-ui side of this contract is exercised by the unit tests in
// `crates/cf-ui/src/tactical_overlay.rs` + `crates/cf-ui/src/context_wheel.rs`.
// Here we assert the SAME registry the engine emits is the one the UI
// reads (via cf-ai's `builtin_registry`).
// ============================================================================

#[test]
fn engine_dump_uses_same_registry_the_ui_enumerates() {
    let world = M7BSquadWorld::new();
    let view = world.dump_state_view(PLAYER_SQUAD_ID);
    let verbs = view.get("verb_registry").and_then(Value::as_array).expect("verbs");
    let engine_ids: std::collections::BTreeSet<String> = verbs
        .iter()
        .filter_map(|v| v.get("verb_id").and_then(Value::as_str).map(|s| s.to_string()))
        .collect();

    let builtin = builtin_registry();
    let builtin_ids: std::collections::BTreeSet<String> =
        builtin.iter().map(|d| d.verb_id.clone()).collect();
    assert_eq!(
        engine_ids, builtin_ids,
        "engine dump must enumerate the same verb set as the UI registry"
    );
}

// ============================================================================
// Additional invariants — role assignment + reslot determinism
// ============================================================================

#[test]
fn role_assignment_is_sticky_and_tracks_changes() {
    let mut world = M7BSquadWorld::new();
    let res1 = world.assign_role(PLAYER_SQUAD_ID, 5, SquadRoleHint::Rifleman);
    assert!(matches!(res1.result, RoleAssignmentResult::Assigned));
    let res2 = world.assign_role(PLAYER_SQUAD_ID, 5, SquadRoleHint::Rifleman);
    assert!(matches!(res2.result, RoleAssignmentResult::Unchanged));
    let res3 = world.assign_role(PLAYER_SQUAD_ID, 5, SquadRoleHint::Heavy);
    assert!(matches!(
        res3.result,
        RoleAssignmentResult::Changed {
            previous: SquadRoleHint::Rifleman
        }
    ));
    let prev = res3.payload.get("previous_role").and_then(Value::as_str);
    assert_eq!(prev, Some("rifleman"));
}

#[test]
fn slot_solver_deterministic_across_repeated_calls() {
    let mut world = M7BSquadWorld::new();
    let squad = world.ensure_squad(PLAYER_SQUAD_ID, DoctrineMode::Aggressive);
    squad.assign_role(1, SquadRoleHint::SquadLeader);
    squad.assign_role(2, SquadRoleHint::Rifleman);
    squad.assign_role(3, SquadRoleHint::Heavy);
    let a = world
        .set_formation(PLAYER_SQUAD_ID, FormationKind::Wedge, [5.0, 0.0], 0.5, Some(1), 100)
        .assignment_payloads
        .clone();
    let b = world
        .set_formation(PLAYER_SQUAD_ID, FormationKind::Wedge, [5.0, 0.0], 0.5, Some(1), 100)
        .assignment_payloads;
    assert_eq!(a, b);
}

#[test]
fn parser_rejects_well_known_argument_shape_mismatch() {
    let mut world = M7BSquadWorld::new();
    world.ensure_squad(PLAYER_SQUAD_ID, DoctrineMode::Aggressive);
    // breach_door requires a Door arg.
    let bad = world.issue_verb(PLAYER_SQUAD_ID, "breach_door", vec![], 1, 100);
    assert!(matches!(bad.outcome, CommandIssue::Rejected { .. }));
    let good = world.issue_verb(
        PLAYER_SQUAD_ID,
        "breach_door",
        vec![VerbArgValue::Door(42)],
        1,
        101,
    );
    assert!(good.is_accepted());
}

// ============================================================================
// Audit-pass gap fixes — slot-broken emission + 2s reslot cadence +
// per-verb doctrine_compat row + Cover Me distinct BT subtree.
// ============================================================================

#[test]
fn slot_broken_emission_when_actor_wanders() {
    let mut world = M7BSquadWorld::new();
    let squad = world.ensure_squad(PLAYER_SQUAD_ID, DoctrineMode::Defensive);
    squad.assign_role(1, SquadRoleHint::SquadLeader);
    squad.assign_role(2, SquadRoleHint::Rifleman);
    let _ = world.set_formation(PLAYER_SQUAD_ID, FormationKind::Wedge, [0.0, 0.0], 0.0, Some(1), 100);
    let mut positions = std::collections::BTreeMap::new();
    positions.insert(1, [9999.0, 9999.0]);
    positions.insert(2, [0.0, 0.0]);
    let payloads = world.detect_and_report_broken_slots(PLAYER_SQUAD_ID, &positions, 220);
    assert!(!payloads.is_empty(), "wandering actor must surface slot-broken event");
    let p = &payloads[0];
    assert_eq!(p.get("reason").and_then(Value::as_str), Some("out_of_range"));
    assert_eq!(p.get("next_solve_tick").and_then(Value::as_u64), Some(220));
}

#[test]
fn periodic_reslot_fires_at_two_second_cadence() {
    let mut world = M7BSquadWorld::new();
    let squad = world.ensure_squad(PLAYER_SQUAD_ID, DoctrineMode::Aggressive);
    squad.assign_role(1, SquadRoleHint::SquadLeader);
    squad.assign_role(2, SquadRoleHint::Rifleman);
    let _ = squad.try_issue_builtin("advance", vec![], 99, 100);
    let _ = world.set_formation(PLAYER_SQUAD_ID, FormationKind::Wedge, [0.0, 0.0], 0.0, Some(1), 100);
    // Just below cadence at 60 Hz (118 ticks < 120).
    let early = world.tick_periodic_reslot(PLAYER_SQUAD_ID, [1.0, 0.0], 0.0, Some(1), 218, 60);
    assert!(early.is_none(), "below 2s should not re-solve");
    let late = world.tick_periodic_reslot(PLAYER_SQUAD_ID, [1.0, 0.0], 0.0, Some(1), 220, 60);
    assert!(late.is_some(), "at 2s should re-solve");
}

#[test]
fn periodic_reslot_skipped_when_squad_idle() {
    let mut world = M7BSquadWorld::new();
    let squad = world.ensure_squad(PLAYER_SQUAD_ID, DoctrineMode::Defensive);
    squad.assign_role(1, SquadRoleHint::SquadLeader);
    let _ = world.set_formation(PLAYER_SQUAD_ID, FormationKind::Wedge, [0.0, 0.0], 0.0, Some(1), 100);
    let res = world.tick_periodic_reslot(PLAYER_SQUAD_ID, [1.0, 0.0], 0.0, Some(1), 9999, 60);
    assert!(res.is_none(), "idle squad must skip reslot");
}

#[test]
fn dump_state_view_exposes_per_verb_doctrine_compat_row() {
    let world = M7BSquadWorld::new();
    let view = world.dump_state_view(PLAYER_SQUAD_ID);
    let verbs = view.get("verb_registry").and_then(Value::as_array).expect("verbs");
    for verb in verbs {
        let id = verb.get("verb_id").and_then(Value::as_str).unwrap_or("");
        let row = verb.get("doctrine_compat").and_then(Value::as_object);
        assert!(row.is_some(), "verb {id} missing doctrine_compat row");
        let row = row.unwrap();
        assert!(row.contains_key("defensive"));
        assert!(row.contains_key("aggressive"));
        assert!(row.contains_key("scout"));
    }
}

#[test]
fn cover_me_is_distinct_bt_subtree_from_suppress_and_overwatch() {
    use cf_ai::archetype_bt::{bt_for_squad_verb, ArchetypeBtKind};
    for kind in ArchetypeBtKind::ALL {
        let suppress = bt_for_squad_verb(kind, "suppress_window").expect("suppress");
        let overwatch = bt_for_squad_verb(kind, "overwatch_sector").expect("overwatch");
        let cover_me = bt_for_squad_verb(kind, "cover_me").expect("cover_me");
        let s = suppress.flatten_label();
        let o = overwatch.flatten_label();
        let c = cover_me.flatten_label();
        assert_ne!(s, o, "{kind:?}: suppress + overwatch distinct");
        assert_ne!(s, c, "{kind:?}: suppress + cover_me distinct");
        assert_ne!(o, c, "{kind:?}: overwatch + cover_me distinct");
    }
}

#[test]
fn archetype_bts_round_trip_through_ron_content_files() {
    use cf_ai::archetype_bt::{ArchetypeBtDef, ArchetypeBtKind};
    for (kind, src) in [
        (
            ArchetypeBtKind::Rifleman,
            include_str!("../../../content/ai/archetype_bts/rifleman.ron"),
        ),
        (
            ArchetypeBtKind::Sniper,
            include_str!("../../../content/ai/archetype_bts/sniper.ron"),
        ),
        (
            ArchetypeBtKind::Assault,
            include_str!("../../../content/ai/archetype_bts/assault.ron"),
        ),
        (
            ArchetypeBtKind::Engineer,
            include_str!("../../../content/ai/archetype_bts/engineer.ron"),
        ),
        (
            ArchetypeBtKind::Spotter,
            include_str!("../../../content/ai/archetype_bts/spotter.ron"),
        ),
        (
            ArchetypeBtKind::Heavy,
            include_str!("../../../content/ai/archetype_bts/heavy.ron"),
        ),
    ] {
        let parsed = ArchetypeBtDef::from_ron(src).unwrap_or_else(|e| panic!("{kind:?}: {e}"));
        let builtin = ArchetypeBtDef::from_builtin(kind);
        assert_eq!(parsed, builtin, "{kind:?} RON drifted from builtin");
        assert!(parsed.nodes.len() >= 30, "{kind:?} has <30 nodes");
        assert!(!parsed.squad_verb_subtrees.is_empty(), "{kind:?} has 0 squad verb subtrees");
    }
}

#[test]
fn bounding_step_cover_and_moving_actor_split_by_role() {
    let mut world = M7BSquadWorld::new();
    let squad = world.ensure_squad(PLAYER_SQUAD_ID, DoctrineMode::Aggressive);
    squad.assign_role(1, SquadRoleHint::SquadLeader);
    squad.assign_role(2, SquadRoleHint::Rifleman);
    squad.assign_role(3, SquadRoleHint::Heavy);
    squad.assign_role(4, SquadRoleHint::Pointman);
    squad.start_bounding([50.0, 0.0], 100);
    let p = world.tick_bounding(PLAYER_SQUAD_ID, Some([50.0, 0.0])).unwrap();
    let covering = p.get("cover_actors").and_then(Value::as_array).unwrap();
    let moving = p.get("moving_actors").and_then(Value::as_array).unwrap();
    assert_eq!(covering.len() + moving.len(), 4);
    assert!(!covering.is_empty(), "at least one actor must cover");
    assert!(!moving.is_empty(), "at least one actor must move");
}

#[test]
fn squad_state_schema_file_exists_on_disk() {
    // Spec § Files lists `squad_state.schema.json` as NEW.
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("schemas/v1/squad_state.schema.json");
    assert!(path.exists(), "squad_state.schema.json must exist at {}", path.display());
    let body = std::fs::read_to_string(&path).expect("read schema");
    let parsed: serde_json::Value = serde_json::from_str(&body).expect("parse schema");
    assert_eq!(
        parsed.get("title").and_then(Value::as_str),
        Some("SquadStateView")
    );
}
