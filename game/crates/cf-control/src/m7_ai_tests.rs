//! Tests for m7_ai.rs (extracted to keep m7_ai.rs under 2000 LOC).

#![allow(unused_imports)]

#[cfg(test)]
mod tests {
    use crate::m7_ai::*;
    use cf_actor::{ActorId, ActorState, Inventory, Status, Vec2};
    use cf_ai::*;
    use cf_audio::ChatterCategory;
    use cf_mission::MissionPhase;
    use cf_priority::{PersonalityModifier, QuickPresetId, RoleTemplate};
    use serde_json::json;
    use std::collections::BTreeMap;

    fn actor(id: u64, x: f32, y: f32, status: Status) -> ActorState {
        let mut a = ActorState::player(ActorId(id), "test", Vec2::new(x, y), 100.0, Inventory::default());
        a.status = status;
        a
    }

    #[test]
    fn nearest_medic_picks_closest_alive() {
        let mut world = M7AiWorld::new();
        world.assign_archetype(ActorId(1), Archetype::Medic);
        world.assign_archetype(ActorId(2), Archetype::Medic);
        let mut actors = BTreeMap::new();
        actors.insert(ActorId(1), actor(1, 100.0, 0.0, Status::Stable));
        actors.insert(ActorId(2), actor(2, 10.0, 0.0, Status::Stable));
        actors.insert(ActorId(99), actor(99, 0.0, 0.0, Status::Dying));
        let pick = nearest_medic(&world.bots, &actors, ActorId(99), 500.0);
        assert!(pick.is_some());
        assert_eq!(pick.unwrap().0, ActorId(2));
    }

    #[test]
    fn begin_auto_triage_only_for_medic_archetype() {
        let mut bot = BotState::new(Archetype::Rifleman);
        let r = begin_auto_triage(&mut bot, ActorId(1), ActorId(2), 100, 60);
        assert!(r.is_none());

        let mut bot = BotState::new(Archetype::Medic);
        let r = begin_auto_triage(&mut bot, ActorId(1), ActorId(2), 100, 60);
        assert!(r.is_some());
        assert!(bot.auto_triage.is_some());
    }

    #[test]
    fn complete_auto_triage_terminates_mission() {
        let mut bot = BotState::new(Archetype::Medic);
        let _ = begin_auto_triage(&mut bot, ActorId(1), ActorId(2), 100, 60);
        let payload = complete_auto_triage(&mut bot, 360, 60);
        assert!(payload.is_some());
        assert!(bot.auto_triage.as_ref().unwrap().is_terminal());
    }

    #[test]
    fn auto_repair_progress_increments_counter() {
        let mut bot = BotState::new(Archetype::Engineer);
        let _ = begin_auto_repair(&mut bot, ActorId(1), ActorId(2), "leg_left", 100, 60);
        progress_auto_repair(&mut bot, 360, 5.0);
        progress_auto_repair(&mut bot, 420, 5.0);
        assert_eq!(bot.auto_repair.as_ref().unwrap().progressed_ticks, 2);
    }

    /// returns the medic + target id once the mission's reach deadline
    /// elapses. Before the deadline, the scan is empty even when a
    /// mission is in flight.
    #[test]
    fn ready_triage_completions_fires_after_reach_deadline() {
        let mut world = M7AiWorld::new();
        world.assign_archetype(ActorId(1), Archetype::Medic);
        let bot = world.bot_mut(ActorId(1)).expect("medic bot");
        let _ = begin_auto_triage(bot, ActorId(1), ActorId(99), 100, 60);
        // Reach deadline = 100 + 360 = 460. Before that → empty.
        assert!(ready_triage_completions(&world, 459).is_empty());
        // At/after 460 → returns the (medic, target) pair.
        let ready = ready_triage_completions(&world, 460);
        assert_eq!(ready, vec![(ActorId(1), ActorId(99))]);
    }

    /// fires once when the engineer's first_tick_deadline lands AND the
    /// mission has not yet recorded a repair tick. Subsequent calls
    /// after a repair tick has been recorded return empty (engine drives
    /// follow-up progressions through the natural mission lifecycle).
    #[test]
    fn ready_repair_progressions_fires_once_per_mission() {
        let mut world = M7AiWorld::new();
        world.assign_archetype(ActorId(2), Archetype::Engineer);
        let bot = world.bot_mut(ActorId(2)).expect("engineer bot");
        let _ = begin_auto_repair(bot, ActorId(2), ActorId(99), "leg_left", 100, 60);
        // first_tick_deadline = 100 + 480 = 580. Before that → empty.
        assert!(ready_repair_progressions(&world, 579).is_empty());
        // At 580 → returns the triple.
        let ready = ready_repair_progressions(&world, 580);
        assert_eq!(ready, vec![(ActorId(2), ActorId(99), "leg_left".to_string())]);
        // After progressing once, ready returns empty.
        let bot = world.bot_mut(ActorId(2)).expect("engineer bot");
        progress_auto_repair(bot, 580, AUTO_REPAIR_AMOUNT_PER_TICK);
        assert!(ready_repair_progressions(&world, 600).is_empty());
    }

    #[test]
    fn phase_advance_emits_payload_at_deadline() {
        let mut world = M7AiWorld::new();
        world.init_phase(0);
        let deadline_tick = 30 * 60;
        let r = advance_phase(&mut world, deadline_tick + 1, 60, "elapsed");
        assert!(r.is_some());
        assert_eq!(world.phase.as_ref().unwrap().current, MissionPhase::Buildup);
    }

    /// counter once per filtered DYING transition.
    #[test]
    fn track_kills_increments_cumulative_count() {
        let mut world = M7AiWorld::new();
        let outcomes = vec![ActorId(2), ActorId(3), ActorId(4)];
        let count = track_kills(&mut world, &outcomes, |id| id.0 != 4);
        assert_eq!(count, 2);
        assert_eq!(world.kill_count, 2);
        let count2 = track_kills(&mut world, &outcomes, |_| true);
        assert_eq!(count2, 5);
    }

    /// the active phase + cumulative kills satisfy a registered wave.
    /// Idempotent — a re-tick after spawn returns None.
    #[test]
    fn try_spawn_reinforcement_fires_once_per_wave() {
        let mut world = M7AiWorld::new();
        world.init_phase(0);
        let _ = advance_phase(&mut world, 30 * 60 + 1, 60, "elapsed");
        world.reinforcements.push(cf_mission::ReinforcementWave::new(
            "alpha",
            MissionPhase::Buildup,
            3,
            [400.0, 0.0],
        ));
        assert!(try_spawn_reinforcement(&mut world, 2, 1900).is_none());
        let payload = try_spawn_reinforcement(&mut world, 3, 1900).expect("alpha wave fires");
        assert_eq!(payload.get("wave_id").unwrap(), &json!("alpha"));
        assert_eq!(payload.get("phase").unwrap(), &json!("buildup"));
        assert!(try_spawn_reinforcement(&mut world, 5, 2000).is_none());
    }

    /// surfaces the canonical phase-change + the canonical ability for
    /// the new phase in one call. Phase 1 → 2 fires `shield`; the latch
    /// prevents a second emission while still in Phase 2.
    #[test]
    fn boss_damage_emits_phase_and_ability_on_threshold_crossing() {
        let mut world = M7AiWorld::new();
        world.boss = Some(cf_mission::BossState::new(42, "spotter", 100.0));
        let emit = apply_boss_damage_and_ability(&mut world, 30.0, 100);
        assert!(emit.phase_changed.is_some(), "Phase 1 → 2 fires phase_changed");
        let ability = emit.ability.expect("Phase 2 fires `shield`");
        assert_eq!(ability.get("phase").unwrap(), &json!("phase_2"));
        assert_eq!(ability.get("ability").unwrap(), &json!("shield"));
        // Re-application that does not cross the next threshold fires nothing.
        let emit2 = apply_boss_damage_and_ability(&mut world, 5.0, 110);
        assert!(emit2.phase_changed.is_none());
        assert!(emit2.ability.is_none());
        // Phase 2 → 3 fires `enraged`.
        let emit3 = apply_boss_damage_and_ability(&mut world, 50.0, 200);
        assert_eq!(
            emit3.ability.expect("phase 3 ability").get("ability").unwrap(),
            &json!("enraged")
        );
    }

    /// each optional objective exactly once when its dependencies clear.
    #[test]
    fn objective_graph_optional_offered_emits_once_per_objective() {
        let mut world = M7AiWorld::new();
        let mut graph = cf_mission::ObjectiveGraph::default();
        graph.push(cf_mission::ObjectiveNode {
            id: "kill_engineer".into(),
            kind: cf_mission::ExtendedObjectiveKind::Optional {
                inner_id: "kill_engineer_inner".into(),
            },
            depends_on: vec![],
            parallel: false,
            optional: true,
            branch_label: String::new(),
            status: cf_mission::ObjectiveNodeStatus::Pending,
        });
        world.objective_graph = Some(graph);
        let emit = drain_objective_graph_emissions(&mut world, 100);
        assert_eq!(emit.optional_offered.len(), 1);
        assert_eq!(
            emit.optional_offered[0].get("objective_id").unwrap(),
            &json!("kill_engineer")
        );
        // Second tick: latched, no re-emit.
        let emit2 = drain_objective_graph_emissions(&mut world, 200);
        assert!(emit2.optional_offered.is_empty());
    }

    /// the chosen branch when a BranchingPoint has a `chosen_branch`.
    #[test]
    fn objective_graph_branched_emits_on_chosen_branch() {
        let mut world = M7AiWorld::new();
        let mut graph = cf_mission::ObjectiveGraph::default();
        graph.branches.push(cf_mission::BranchingPoint {
            id: "chokepoint".into(),
            branch_a_id: "kill_path".into(),
            branch_b_id: "sneak_path".into(),
            chosen_branch: Some("sneak_path".into()),
            offered_tick: Some(150),
        });
        world.objective_graph = Some(graph);
        let emit = drain_objective_graph_emissions(&mut world, 200);
        assert_eq!(emit.objective_branched.len(), 1);
        let payload = &emit.objective_branched[0];
        assert_eq!(payload.get("branching_point_id").unwrap(), &json!("chokepoint"));
        assert_eq!(payload.get("chosen_branch").unwrap(), &json!("sneak_path"));
        assert_eq!(payload.get("other_branch").unwrap(), &json!("kill_path"));
        // Latched on the second tick.
        let emit2 = drain_objective_graph_emissions(&mut world, 250);
        assert!(emit2.objective_branched.is_empty());
    }

    /// dependencies are NOT offered until the dependency completes.
    #[test]
    fn objective_graph_optional_waits_for_dependencies() {
        let mut world = M7AiWorld::new();
        let mut graph = cf_mission::ObjectiveGraph::default();
        graph.push(cf_mission::ObjectiveNode {
            id: "primary".into(),
            kind: cf_mission::ExtendedObjectiveKind::KillN {
                target_class: "rifleman".into(),
                count: 1,
            },
            depends_on: vec![],
            parallel: false,
            optional: false,
            branch_label: String::new(),
            status: cf_mission::ObjectiveNodeStatus::Pending,
        });
        graph.push(cf_mission::ObjectiveNode {
            id: "bonus".into(),
            kind: cf_mission::ExtendedObjectiveKind::Optional {
                inner_id: "bonus_inner".into(),
            },
            depends_on: vec!["primary".into()],
            parallel: false,
            optional: true,
            branch_label: String::new(),
            status: cf_mission::ObjectiveNodeStatus::Pending,
        });
        world.objective_graph = Some(graph);
        let emit = drain_objective_graph_emissions(&mut world, 100);
        assert!(
            emit.optional_offered.is_empty(),
            "bonus is gated until primary completes"
        );
        if let Some(g) = world.objective_graph.as_mut() {
            g.mark_completed("primary");
        }
        let emit2 = drain_objective_graph_emissions(&mut world, 200);
        assert_eq!(emit2.optional_offered.len(), 1);
        assert_eq!(emit2.optional_offered[0].get("objective_id").unwrap(), &json!("bonus"));
    }

    /// drain_boss_phase_ability returns None until a phase change.
    #[test]
    fn boss_phase_1_has_no_ability() {
        let mut world = M7AiWorld::new();
        world.boss = Some(cf_mission::BossState::new(1, "spotter", 100.0));
        assert!(drain_boss_phase_ability(&mut world, 0).is_none());
    }

    /// PriorityTable AND keeps the utility scorer's cached priority in
    /// sync so the next tick scores against the new weight.
    #[test]
    fn set_priority_mutates_state_and_utility_cache() {
        let mut world = M7AiWorld::new();
        world.assign_archetype(ActorId(7), Archetype::Medic);
        let r = world.set_priority(ActorId(7), TaskType::TriageDownedAlly, 3);
        assert!(r.is_ok());
        let (old, new) = r.unwrap();
        assert_eq!(old, 9, "Medic role template starts TriageDownedAlly at 9");
        assert_eq!(new, 3);
        let bot = world.bot(ActorId(7)).unwrap();
        assert_eq!(bot.stack.priority.get(TaskType::TriageDownedAlly), 3);
        assert_eq!(bot.stack.utility.priority.get(TaskType::TriageDownedAlly), 3);
    }

    #[test]
    fn set_priority_clamps_to_nine() {
        let mut world = M7AiWorld::new();
        world.assign_archetype(ActorId(7), Archetype::Sniper);
        let r = world.set_priority(ActorId(7), TaskType::SharpshootTarget, 250);
        assert!(r.is_ok());
        assert_eq!(r.unwrap().1, 9);
    }

    #[test]
    fn set_priority_rejects_unknown_actor() {
        let mut world = M7AiWorld::new();
        let r = world.set_priority(ActorId(99), TaskType::EngageVisibleEnemy, 5);
        assert!(r.is_err());
    }

    #[test]
    fn set_autonomy_returns_old_mode() {
        let mut world = M7AiWorld::new();
        world.assign_archetype(ActorId(7), Archetype::Rifleman);
        let prev = world.set_autonomy(ActorId(7), AutonomyMode::Manual);
        assert_eq!(prev, Some(AutonomyMode::FullAuto));
        assert_eq!(world.bot(ActorId(7)).unwrap().stack.autonomy, AutonomyMode::Manual);
    }

    #[test]
    fn apply_role_template_swaps_priority_and_archetype() {
        let mut world = M7AiWorld::new();
        world.assign_archetype(ActorId(7), Archetype::Rifleman);
        let r = world.apply_role_template(ActorId(7), RoleTemplate::Medic);
        assert!(r.is_some());
        let bot = world.bot(ActorId(7)).unwrap();
        assert_eq!(bot.archetype, Archetype::Medic);
        assert_eq!(bot.stack.priority.get(TaskType::TriageDownedAlly), 9);
    }

    #[test]
    fn apply_quick_preset_shifts_weights() {
        let mut world = M7AiWorld::new();
        world.assign_archetype(ActorId(7), Archetype::Rifleman);
        let r = world.apply_quick_preset(ActorId(7), QuickPresetId::Attack);
        assert!(r.is_some());
        let bot = world.bot(ActorId(7)).unwrap();
        // Rifleman base EngageVisibleEnemy = 7; +2 = 9.
        assert_eq!(bot.stack.priority.get(TaskType::EngageVisibleEnemy), 9);
        // HoldCover = 6; -2 = 4.
        assert_eq!(bot.stack.priority.get(TaskType::HoldCover), 4);
    }

    #[test]
    fn priority_table_view_lists_all_22_weights() {
        let mut world = M7AiWorld::new();
        world.assign_archetype(ActorId(7), Archetype::Medic);
        let view = world.priority_table_view(ActorId(7)).unwrap();
        let weights = view.get("weights").unwrap().as_object().unwrap();
        assert_eq!(weights.len(), 22);
        assert_eq!(weights.get("triage_downed_ally").unwrap(), &json!(9));
        assert_eq!(view.get("role").unwrap(), &json!("medic"));
    }

    #[test]
    fn autonomy_view_carries_mode_and_cap() {
        let mut world = M7AiWorld::new();
        world.assign_archetype(ActorId(7), Archetype::Rifleman);
        world.set_autonomy(ActorId(7), AutonomyMode::Standard);
        let view = world.autonomy_view(ActorId(7)).unwrap();
        assert_eq!(view.get("mode").unwrap(), &json!("standard"));
        assert_eq!(view.get("auto_action_cap").unwrap(), &json!(3));
    }

    /// (round-trip preserves weights). Spec § PriorityTable persists.
    #[test]
    fn priority_table_round_trips_through_snapshot_restore() {
        let mut world = M7AiWorld::new();
        world.assign_archetype(ActorId(1), Archetype::Sniper);
        world.assign_archetype(ActorId(2), Archetype::Engineer);
        world.set_priority(ActorId(1), TaskType::SharpshootTarget, 9).unwrap();
        world.set_priority(ActorId(2), TaskType::SetTrap, 8).unwrap();
        let snap = world.snapshot_actor_priorities();

        // Mutate after snapshot.
        world.set_priority(ActorId(1), TaskType::SharpshootTarget, 1).unwrap();
        world.set_priority(ActorId(2), TaskType::SetTrap, 1).unwrap();

        // Restore — weights return to snapshot values.
        let restored = world.restore_actor_priorities(&snap);
        assert_eq!(restored, 2);
        assert_eq!(
            world
                .bot(ActorId(1))
                .unwrap()
                .stack
                .priority
                .get(TaskType::SharpshootTarget),
            9
        );
        assert_eq!(world.bot(ActorId(2)).unwrap().stack.priority.get(TaskType::SetTrap), 8);
    }

    #[test]
    fn try_emit_chatter_gates_within_cooldown_window() {
        let mut world = M7AiWorld::new();
        world.assign_archetype(ActorId(7), Archetype::Medic);
        let first = world.try_emit_chatter(ActorId(7), ChatterCategory::Triage, "Treating Jenkins", 100, 60);
        assert!(first.is_some(), "first emission opens the slot");
        let (event, _info) = first.unwrap();
        assert_eq!(event.text, "Treating Jenkins");
        assert_eq!(event.voice_id, "voice.medic.default");
        // Same category 200 ticks later (3.33s) — still in 4s cooldown.
        let second = world.try_emit_chatter(ActorId(7), ChatterCategory::Triage, "Re-Treating", 200, 60);
        assert!(second.is_none(), "still in cooldown");
        // 4.0s later — boundary case (240 ticks @ 60 Hz). 100 + 240 = 340.
        let third = world.try_emit_chatter(ActorId(7), ChatterCategory::Triage, "Treating again", 340, 60);
        assert!(third.is_some(), "boundary-tick emission is allowed");
    }

    #[test]
    fn personality_modifier_recorded_on_apply() {
        let mut world = M7AiWorld::new();
        world.assign_archetype(ActorId(7), Archetype::Rifleman);
        let r = world.apply_personality_modifier(ActorId(7), PersonalityModifier::Aggressive);
        assert!(r.is_some());
        let bot = world.bot(ActorId(7)).unwrap();
        assert_eq!(bot.personality_modifier, PersonalityModifier::Aggressive);
        // Rifleman EngageVisibleEnemy = 7; +2 (Aggressive) = 9.
        assert_eq!(bot.stack.priority.get(TaskType::EngageVisibleEnemy), 9);
    }

    #[test]
    fn priority_table_changed_payload_shape() {
        let v = priority_table_changed_payload(7, TaskType::TriageDownedAlly, 9, 3);
        assert_eq!(v.get("actor_id").unwrap(), &json!(7));
        assert_eq!(v.get("task").unwrap(), &json!("triage_downed_ally"));
        assert_eq!(v.get("old_weight").unwrap(), &json!(9));
        assert_eq!(v.get("new_weight").unwrap(), &json!(3));
    }

    #[test]
    fn autonomy_mode_changed_payload_shape() {
        let v = autonomy_mode_changed_payload(7, AutonomyMode::FullAuto, AutonomyMode::Manual);
        assert_eq!(v.get("from").unwrap(), &json!("full_auto"));
        assert_eq!(v.get("to").unwrap(), &json!("manual"));
    }

    #[test]
    fn role_template_applied_payload_shape() {
        let v = role_template_applied_payload(7, RoleTemplate::Medic);
        assert_eq!(v.get("template_id").unwrap(), &json!("medic"));
    }

    #[test]
    fn quick_preset_applied_payload_shape() {
        let v = quick_preset_applied_payload(7, QuickPresetId::Rescue);
        assert_eq!(v.get("preset_id").unwrap(), &json!("rescue"));
    }

    #[test]
    fn faction_allegiance_changed_payload_shape() {
        let v = faction_allegiance_changed_payload(FactionId::Player, FactionId::AiAllied, -30, 45, "friendly_fire");
        assert_eq!(v.get("a").unwrap(), &json!("player"));
        assert_eq!(v.get("b").unwrap(), &json!("ai_allied"));
        assert_eq!(v.get("delta").unwrap(), &json!(-30));
        assert_eq!(v.get("new_value").unwrap(), &json!(45));
        assert_eq!(v.get("cause").unwrap(), &json!("friendly_fire"));
    }

    // -----------------------------------------------------------------
    // transition coverage for the 7 sub-plan events the engine emits via
    // `detect_behavior_transitions`.
    // -----------------------------------------------------------------

    fn signals_baseline(actor_id: u64) -> BehaviorSignals {
        BehaviorSignals {
            actor_id,
            self_position: [0.0, 0.0],
            hp_fraction: 1.0,
            enemy_visible: false,
            under_fire: false,
            reactive_override: false,
            player_actor_id: Some(99),
            player_position: Some([100.0, 0.0]),
            squadmates: vec![2, 3],
            friendly_in_line_of_fire: None,
            current_tick: 100,
            tick_rate_hz: 60,
        }
    }

    #[test]
    fn cover_seeking_started_payload_shape() {
        let event = CoverSeekingEvent {
            actor_id: 7,
            archetype: Archetype::Rifleman,
            reason: CoverSeekingReason::Fired,
            target_position: [1.0, 2.0],
            distance: 3.5,
        };
        let v = cover_seeking_started_payload(&event);
        assert_eq!(v.get("actor_id").unwrap(), &json!(7));
        assert_eq!(v.get("archetype").unwrap(), &json!("rifleman"));
        assert_eq!(v.get("reason").unwrap(), &json!("fired"));
        assert_eq!(v.get("target_position").unwrap(), &json!([1.0, 2.0]));
        assert_eq!(v.get("distance").unwrap(), &json!(3.5));
    }

    #[test]
    fn suppression_started_payload_shape() {
        let event = SuppressionEvent::build(7, 99, Some(8), 60);
        let v = suppression_started_payload(&event);
        assert_eq!(v.get("actor_id").unwrap(), &json!(7));
        assert_eq!(v.get("target_actor_id").unwrap(), &json!(99));
        assert_eq!(v.get("flanker_actor_id").unwrap(), &json!(8));
        assert_eq!(v.get("duration_ticks").unwrap(), &json!(240));
    }

    #[test]
    fn retreat_decision_payload_shape() {
        let event = RetreatDecisionEvent {
            actor_id: 7,
            reason: RetreatReason::HpLow,
            hp_fraction: 0.25,
            tick: 100,
        };
        let v = retreat_decision_payload(&event);
        assert_eq!(v.get("actor_id").unwrap(), &json!(7));
        assert_eq!(v.get("reason").unwrap(), &json!("hp_low"));
        assert_eq!(v.get("hp_fraction").unwrap(), &json!(0.25));
        assert_eq!(v.get("tick").unwrap(), &json!(100));
    }

    #[test]
    fn squad_comm_relayed_payload_shape() {
        let event = SquadCommRelayedEvent {
            originator_actor_id: 7,
            receiver_actor_ids: vec![2, 3],
            target_actor_id: 99,
            target_position: [50.0, 60.0],
            delay_ticks: 30,
        };
        let v = squad_comm_relayed_payload(&event);
        assert_eq!(v.get("originator_actor_id").unwrap(), &json!(7));
        assert_eq!(v.get("receiver_actor_ids").unwrap(), &json!([2, 3]));
        assert_eq!(v.get("target_actor_id").unwrap(), &json!(99));
        assert_eq!(v.get("target_position").unwrap(), &json!([50.0, 60.0]));
        assert_eq!(v.get("delay_ticks").unwrap(), &json!(30));
    }

    #[test]
    fn patrol_waypoint_reached_payload_shape() {
        let event = PatrolWaypointReachedEvent {
            actor_id: 7,
            waypoint_index: 1,
            position: [10.0, 0.0],
            idle_seconds: 7.5,
        };
        let v = patrol_waypoint_reached_payload(&event);
        assert_eq!(v.get("actor_id").unwrap(), &json!(7));
        assert_eq!(v.get("waypoint_index").unwrap(), &json!(1));
        assert_eq!(v.get("position").unwrap(), &json!([10.0, 0.0]));
        assert_eq!(v.get("idle_seconds").unwrap(), &json!(7.5));
    }

    #[test]
    fn friendly_fire_avoidance_payload_shape() {
        let event = FriendlyFireAvoidanceEvent {
            actor_id: 7,
            friendly_actor_id: 8,
            kind: FriendlyFireKind::LineOfFire,
        };
        let v = friendly_fire_avoidance_payload(&event);
        assert_eq!(v.get("actor_id").unwrap(), &json!(7));
        assert_eq!(v.get("friendly_actor_id").unwrap(), &json!(8));
        assert_eq!(v.get("kind").unwrap(), &json!("line_of_fire"));
    }

    #[test]
    fn high_ground_preference_applied_payload_shape() {
        let event = HighGroundEvent {
            actor_id: 7,
            target_position: [3.0, 8.0],
            elevation_gain: 8.0,
        };
        let v = high_ground_preference_applied_payload(&event);
        assert_eq!(v.get("actor_id").unwrap(), &json!(7));
        assert_eq!(v.get("target_position").unwrap(), &json!([3.0, 8.0]));
        assert_eq!(v.get("elevation_gain").unwrap(), &json!(8.0));
    }

    /// **A1 production-path coverage**: cover-seeking transition fires
    /// when the bot's chosen task flips into HoldCover under a fire
    /// signal. Subsequent ticks at the same task do NOT re-fire.
    #[test]
    fn detect_emits_cover_seeking_on_transition_into_cover() {
        let mut bot = BotState::new(Archetype::Rifleman);
        let mut sig = signals_baseline(7);
        sig.under_fire = true;
        let emit = detect_behavior_transitions(&mut bot, TaskType::HoldCover, &sig);
        assert!(emit.cover_seeking_started.is_some());
        let emit2 = detect_behavior_transitions(&mut bot, TaskType::HoldCover, &sig);
        assert!(
            emit2.cover_seeking_started.is_none(),
            "no re-fire on second tick at same task"
        );
    }

    /// **A2 production-path coverage**: suppression-started fires on
    /// transition into SuppressFire and carries the player as target.
    #[test]
    fn detect_emits_suppression_on_transition_into_suppress() {
        let mut bot = BotState::new(Archetype::Rifleman);
        let sig = signals_baseline(7);
        let emit = detect_behavior_transitions(&mut bot, TaskType::SuppressFire, &sig);
        let payload = emit.suppression_started.expect("suppression payload");
        assert_eq!(payload.get("actor_id").unwrap(), &json!(7));
        assert_eq!(payload.get("target_actor_id").unwrap(), &json!(99));
    }

    /// **A3 production-path coverage**: retreat-decision fires when the
    /// bot transitions into RetreatToCover with HP below threshold.
    #[test]
    fn detect_emits_retreat_decision_when_hp_low() {
        let mut bot = BotState::new(Archetype::Rifleman);
        let mut sig = signals_baseline(7);
        sig.hp_fraction = 0.20;
        let emit = detect_behavior_transitions(&mut bot, TaskType::RetreatToCover, &sig);
        let payload = emit.retreat_decision.expect("retreat payload");
        assert_eq!(payload.get("reason").unwrap(), &json!("hp_low"));
        assert_eq!(payload.get("actor_id").unwrap(), &json!(7));
    }

    /// **A4 production-path coverage**: squad-comm relay schedules on
    /// the lost→spotted transition AND fires after the 0.5s delay
    /// (30 ticks @ 60Hz) carrying the squadmates as receivers.
    #[test]
    fn detect_emits_squad_comm_relayed_after_delay() {
        let mut bot = BotState::new(Archetype::Rifleman);
        let mut sig = signals_baseline(7);
        sig.enemy_visible = true;
        sig.current_tick = 100;
        let emit_first = detect_behavior_transitions(&mut bot, TaskType::EngageVisibleEnemy, &sig);
        assert!(
            emit_first.squad_comm_relayed.is_empty(),
            "no relay emitted before delay elapses"
        );
        sig.current_tick = 130;
        let emit_after = detect_behavior_transitions(&mut bot, TaskType::EngageVisibleEnemy, &sig);
        assert_eq!(emit_after.squad_comm_relayed.len(), 1);
        let payload = &emit_after.squad_comm_relayed[0];
        assert_eq!(payload.get("originator_actor_id").unwrap(), &json!(7));
        assert_eq!(payload.get("receiver_actor_ids").unwrap(), &json!([2, 3]));
        assert_eq!(payload.get("delay_ticks").unwrap(), &json!(30));
    }

    /// **A5 production-path coverage**: patrol-waypoint-reached fires
    /// on the tick the bot's idle countdown expires AND the cursor
    /// advances. The default route has 2 waypoints.
    #[test]
    fn detect_emits_patrol_waypoint_when_idle_expires() {
        let mut bot = BotState::new(Archetype::Rifleman);
        let sig = signals_baseline(7);
        let emit = detect_behavior_transitions(&mut bot, TaskType::Patrol, &sig);
        let payload = emit.patrol_waypoint_reached.expect("patrol payload");
        assert_eq!(payload.get("actor_id").unwrap(), &json!(7));
        assert!(payload.get("position").is_some());
    }

    /// **A6 production-path coverage**: friendly-fire-avoidance fires
    /// when the bot is shooting AND a friendly is in the line of fire.
    /// Throttled to one emission per friendly until cleared.
    #[test]
    fn detect_emits_friendly_fire_avoidance_when_friend_in_los() {
        let mut bot = BotState::new(Archetype::Rifleman);
        let mut sig = signals_baseline(7);
        sig.friendly_in_line_of_fire = Some(8);
        let emit = detect_behavior_transitions(&mut bot, TaskType::EngageVisibleEnemy, &sig);
        let payload = emit.friendly_fire_avoidance.expect("ff payload");
        assert_eq!(payload.get("friendly_actor_id").unwrap(), &json!(8));
        assert_eq!(payload.get("kind").unwrap(), &json!("line_of_fire"));
        // Re-tick with the same friendly still blocking — no re-fire.
        let emit2 = detect_behavior_transitions(&mut bot, TaskType::EngageVisibleEnemy, &sig);
        assert!(emit2.friendly_fire_avoidance.is_none());
    }

    /// **A7 production-path coverage**: high-ground-preference-applied
    /// fires when a Sniper transitions into SharpshootTarget while
    /// standing on positive elevation.
    #[test]
    fn detect_emits_high_ground_preference_for_sniper_on_elevation() {
        let mut bot = BotState::new(Archetype::Sniper);
        let mut sig = signals_baseline(7);
        sig.self_position = [10.0, 25.0];
        let emit = detect_behavior_transitions(&mut bot, TaskType::SharpshootTarget, &sig);
        let payload = emit.high_ground_preference_applied.expect("high-ground payload");
        assert_eq!(payload.get("actor_id").unwrap(), &json!(7));
        assert_eq!(payload.get("elevation_gain").unwrap(), &json!(25.0));
    }

    /// **A7 negative**: Rifleman archetype does NOT fire high-ground
    /// even on the same transition / elevation (high-ground is
    /// Sniper/Spotter-only per spec).
    #[test]
    fn detect_does_not_emit_high_ground_for_rifleman() {
        let mut bot = BotState::new(Archetype::Rifleman);
        let mut sig = signals_baseline(7);
        sig.self_position = [10.0, 25.0];
        let emit = detect_behavior_transitions(&mut bot, TaskType::SharpshootTarget, &sig);
        assert!(emit.high_ground_preference_applied.is_none());
    }

    // -----------------------------------------------------------------
    // faction delta coverage. Verifies the helpers mutate the world
    // accumulators correctly and return ready-to-record payloads.
    // -----------------------------------------------------------------

    #[test]
    fn adjust_actor_mood_applies_delta_and_returns_payload() {
        let mut world = M7AiWorld::new();
        world.assign_archetype(ActorId(7), Archetype::Rifleman);
        let payload = adjust_actor_mood(&mut world, ActorId(7), MOOD_DELTA_ALLY_KILLED, "ally_killed")
            .expect("ally_killed mood delta returns payload");
        assert_eq!(payload.get("actor_id").unwrap(), &json!(7));
        assert_eq!(payload.get("delta").unwrap(), &json!(MOOD_DELTA_ALLY_KILLED));
        assert_eq!(payload.get("new_mood").unwrap(), &json!(MOOD_DELTA_ALLY_KILLED));
        assert_eq!(payload.get("cause").unwrap(), &json!("ally_killed"));
        let bot = world.bot(ActorId(7)).expect("bot exists");
        assert!((bot.personality.mood - MOOD_DELTA_ALLY_KILLED).abs() < f32::EPSILON);
    }

    #[test]
    fn adjust_actor_mood_clamps_to_minus_hundred() {
        let mut world = M7AiWorld::new();
        world.assign_archetype(ActorId(7), Archetype::Rifleman);
        // Bottom-out: 8 ally_killed events = -120, clamped to -100.
        for _ in 0..8 {
            let _ = adjust_actor_mood(&mut world, ActorId(7), MOOD_DELTA_ALLY_KILLED, "ally_killed");
        }
        let bot = world.bot(ActorId(7)).unwrap();
        assert!((bot.personality.mood + 100.0).abs() < f32::EPSILON);
    }

    #[test]
    fn adjust_actor_mood_returns_none_for_untracked_actor() {
        let mut world = M7AiWorld::new();
        assert!(adjust_actor_mood(&mut world, ActorId(42), MOOD_DELTA_WOUNDED, "wounded").is_none());
    }

    #[test]
    fn record_shot_for_stress_pumps_stress_after_ten_shots() {
        let mut world = M7AiWorld::new();
        world.assign_archetype(ActorId(7), Archetype::Rifleman);
        let tick_rate = 60;
        // First 9 shots in window: no transition yet.
        for i in 0..9 {
            let payload = record_shot_for_stress(&mut world, ActorId(7), 100 + i * 5, tick_rate);
            assert!(payload.is_none(), "no threshold crossed after {} shots", i + 1);
        }
        // 10th shot in the same 5-second window: pumps stress one band.
        let payload =
            record_shot_for_stress(&mut world, ActorId(7), 145, tick_rate).expect("10th shot pumps stress band");
        assert_eq!(payload.get("threshold").unwrap(), &json!("stressed"));
        assert_eq!(payload.get("direction").unwrap(), &json!("entered"));
        assert_eq!(payload.get("actor_id").unwrap(), &json!(7));
        let stress_value = payload.get("stress_value").and_then(|v| v.as_f64()).unwrap();
        assert!(stress_value >= 25.0);
    }

    #[test]
    fn record_shot_for_stress_does_not_re_pump_within_same_burst() {
        let mut world = M7AiWorld::new();
        world.assign_archetype(ActorId(7), Archetype::Rifleman);
        let tick_rate = 60;
        for i in 0..10 {
            let _ = record_shot_for_stress(&mut world, ActorId(7), 100 + i * 5, tick_rate);
        }
        // 11th and 12th shots inside the same burst: no re-pump.
        let p11 = record_shot_for_stress(&mut world, ActorId(7), 152, tick_rate);
        let p12 = record_shot_for_stress(&mut world, ActorId(7), 156, tick_rate);
        assert!(p11.is_none(), "11th shot stays latched");
        assert!(p12.is_none(), "12th shot stays latched");
    }

    #[test]
    fn record_shot_for_stress_resets_latch_when_window_drains() {
        let mut world = M7AiWorld::new();
        world.assign_archetype(ActorId(7), Archetype::Rifleman);
        let tick_rate = 60;
        for i in 0..10 {
            let _ = record_shot_for_stress(&mut world, ActorId(7), 100 + i * 5, tick_rate);
        }
        // Jump way past the 5-second window: latch should clear before next pump.
        let later_tick = 100 + 5 * 60 * 10;
        let payload = record_shot_for_stress(&mut world, ActorId(7), later_tick, tick_rate);
        assert!(payload.is_none(), "single shot after window drain doesn't re-pump");
        let bot = world.bot(ActorId(7)).expect("bot exists");
        assert!(!bot.sustained_combat_latched, "latch reset by window drain");
    }

    #[test]
    fn adjust_faction_relationships_friendly_fire_drops_player_ally_to_negative() {
        let mut world = M7AiWorld::new();
        // Default: Player ↔ AiAllied = +75.
        let payload = adjust_faction_relationships(
            &mut world,
            FactionId::Player,
            FactionId::AiAllied,
            FACTION_DELTA_FRIENDLY_FIRE,
            "friendly_fire_received",
        )
        .expect("friendly fire delta returns payload");
        assert_eq!(payload.get("a").unwrap(), &json!("player"));
        assert_eq!(payload.get("b").unwrap(), &json!("ai_allied"));
        assert_eq!(payload.get("delta").unwrap(), &json!(FACTION_DELTA_FRIENDLY_FIRE));
        assert_eq!(payload.get("new_value").unwrap(), &json!(45));
        // Apply again — same factions cross into hostile territory.
        let payload2 = adjust_faction_relationships(
            &mut world,
            FactionId::Player,
            FactionId::AiAllied,
            -100,
            "friendly_fire_received",
        )
        .unwrap();
        let new_value = payload2.get("new_value").and_then(|v| v.as_i64()).unwrap();
        assert!(new_value <= -50, "matrix went hostile");
    }

    #[test]
    fn adjust_faction_relationships_returns_none_on_self_pair() {
        let mut world = M7AiWorld::new();
        let payload =
            adjust_faction_relationships(&mut world, FactionId::Player, FactionId::Player, -30, "self_pair_skip");
        assert!(payload.is_none(), "self-pair never adjusts");
    }

    #[test]
    fn is_friendly_fire_detects_same_faction_and_allies() {
        let world = M7AiWorld::new();
        assert!(is_friendly_fire(&world, FactionId::AiEnemy, FactionId::AiEnemy));
        assert!(is_friendly_fire(&world, FactionId::Player, FactionId::AiAllied));
        assert!(!is_friendly_fire(&world, FactionId::Player, FactionId::AiEnemy));
    }

    #[test]
    fn faction_for_actor_returns_bot_or_player() {
        let mut world = M7AiWorld::new();
        world.assign_archetype(ActorId(7), Archetype::Rifleman);
        if let Some(bot) = world.bot_mut(ActorId(7)) {
            bot.faction = FactionId::AiAllied;
        }
        assert_eq!(
            faction_for_actor(&world, ActorId(7), Some(ActorId(99))),
            Some(FactionId::AiAllied)
        );
        assert_eq!(
            faction_for_actor(&world, ActorId(99), Some(ActorId(99))),
            Some(FactionId::Player)
        );
        assert_eq!(faction_for_actor(&world, ActorId(42), Some(ActorId(99))), None);
    }
}
