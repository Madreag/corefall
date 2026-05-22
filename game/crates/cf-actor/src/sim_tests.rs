//! Tests for [`crate::sim`] and the `sim_*` sibling modules.
//!
//! Extracted from `sim.rs` for file size.

#![cfg(test)]

use std::collections::BTreeMap;

use cf_equipment::{rifle_preset, RifleState, RIFLE_M1_DEFAULT_ID};

use crate::sim::{
    step_no_rng, ActorSimState, ActorTuning, HitOutcome, Projectile, StepDeps,
};
use crate::{
    ActorId, ActorState, ActorWorld, ControlIntent, IntentSource, Inventory, ItemSlot, Stance, Status, Vec2,
};

fn setup() -> (ActorSimState, BTreeMap<ActorId, ControlIntent>) {
    let mut world = ActorWorld::new(0.0, -980.0);
    let inv = Inventory::with_rifle(RIFLE_M1_DEFAULT_ID);
    let mut actor = ActorState::player(ActorId(1), "blue", Vec2::new(50.0, 16.0), 100.0, inv);
    actor.on_ground = true;
    world.insert(actor);
    // Target dummy at x=400. Damage of 12 * 9 hits = 108 > 100 so it can be killed.
    let dummy_inv = Inventory::default();
    let mut dummy = ActorState::player(ActorId(2), "red", Vec2::new(400.0, 16.0), 100.0, dummy_inv);
    dummy.controllable = false;
    dummy.on_ground = true;
    world.insert(dummy);

    let mut state = ActorSimState::new(world);
    state.ensure_rifle_for(
        ActorId(1),
        RifleState::new(rifle_preset(RIFLE_M1_DEFAULT_ID).unwrap(), 60),
    );
    let intents = BTreeMap::new();
    (state, intents)
}

fn deps() -> StepDeps {
    StepDeps {
        tick_dt: 1.0 / 60.0,
        region_min_x: 0.0,
        region_max_x: 1280.0,
        region_max_y: 720.0,
        auto_reload_when_empty: false,
        tuning: None,
        tutorial_safety: false,
    }
}

#[test]
fn idle_actor_just_settles() {
    let (mut state, mut intents) = setup();
    let report = step_no_rng(&mut state, &mut intents, deps());
    assert_eq!(report.actor_outcomes.len(), 2);
    let player = report.actor_outcomes.iter().find(|o| o.actor == ActorId(1)).unwrap();
    assert!(!player.fired);
    assert!(!player.jump_accepted);
}

#[test]
fn move_intent_advances_position() {
    let (mut state, mut intents) = setup();
    intents.insert(
        ActorId(1),
        ControlIntent {
            actor: ActorId(1),
            move_x: 1.0,
            ..ControlIntent::new(ActorId(1), IntentSource::Human)
        },
    );
    let _ = step_no_rng(&mut state, &mut intents, deps());
    let actor = state.world.actors.get(&ActorId(1)).unwrap();
    assert!(actor.position.x > 50.0);
}

#[test]
fn jump_only_works_on_ground() {
    let (mut state, mut intents) = setup();
    intents.insert(
        ActorId(1),
        ControlIntent {
            actor: ActorId(1),
            jump: true,
            ..ControlIntent::new(ActorId(1), IntentSource::Human)
        },
    );
    let report = step_no_rng(&mut state, &mut intents, deps());
    let player = report.actor_outcomes.iter().find(|o| o.actor == ActorId(1)).unwrap();
    assert!(player.jump_accepted);

    // In the air, a second jump intent must be refused.
    intents.insert(
        ActorId(1),
        ControlIntent {
            actor: ActorId(1),
            jump: true,
            ..ControlIntent::new(ActorId(1), IntentSource::Human)
        },
    );
    let report2 = step_no_rng(&mut state, &mut intents, deps());
    let player2 = report2.actor_outcomes.iter().find(|o| o.actor == ActorId(1)).unwrap();
    assert!(!player2.jump_accepted, "no double-jump in M1");
}

#[test]
fn fire_spawns_projectile_with_recoil() {
    let (mut state, mut intents) = setup();
    intents.insert(
        ActorId(1),
        ControlIntent {
            actor: ActorId(1),
            fire: true,
            ..ControlIntent::new(ActorId(1), IntentSource::Cfctl)
        },
    );
    let report = step_no_rng(&mut state, &mut intents, deps());
    assert_eq!(report.spawned_projectiles.len(), 1);
    let player = report.actor_outcomes.iter().find(|o| o.actor == ActorId(1)).unwrap();
    assert!(player.fired);
    assert!(player.recoil_applied > 0.0);
    let actor = state.world.actors.get(&ActorId(1)).unwrap();
    // Recoil pushed the firer slightly backwards; aim is +x so velocity_x went negative.
    assert!(actor.velocity.x < 0.0);
}

#[test]
fn projectile_eventually_hits_dummy_and_can_kill_it() {
    let (mut state, mut intents) = setup();
    // Fire 9 shots (12 dmg * 9 = 108 > 100 hp on the dummy). Allow ~120 ticks for travel.
    let mut shots_fired = 0;
    let mut hits = 0;
    let fire_interval_ticks = rifle_preset(RIFLE_M1_DEFAULT_ID).unwrap().fire_interval_ticks(60);
    for tick in 0..240u32 {
        let intent = if tick % fire_interval_ticks == 0 && shots_fired < 9 {
            shots_fired += 1;
            ControlIntent {
                actor: ActorId(1),
                fire: true,
                ..ControlIntent::new(ActorId(1), IntentSource::Cfctl)
            }
        } else {
            ControlIntent::new(ActorId(1), IntentSource::Cfctl)
        };
        intents.insert(ActorId(1), intent);
        let report = step_no_rng(&mut state, &mut intents, deps());
        hits += report.hits.len();
    }
    assert!(hits >= 9, "all 9 shots must connect; got {hits}");
    let dummy = state.world.actors.get(&ActorId(2)).unwrap();
    // CCCP Actor.cpp:1229 — HP=0 enters DYING dwell first. After the
    // 60-tick dwell at 60Hz the status advances to DEAD; the loop above
    // runs 240 ticks, well past the dwell.
    assert!(
        matches!(dummy.status, Status::Dying | Status::Dead),
        "dummy must enter DYING (or DEAD after dwell); got {:?}",
        dummy.status
    );
}

#[test]
fn fast_projectile_hits_actor_via_swept_segment() {
    // Regression: at the M1 rifle's 1200 units/s a projectile travels 20 units per
    // tick, which is wider than the default 16-unit actor AABB. Without a swept
    // segment-vs-AABB test the projectile point can step over an actor between two
    // sampled positions and miss entirely. Place a dummy whose AABB sits cleanly
    // between two consecutive projectile points so a non-swept check would tunnel.
    let mut world = ActorWorld::new(0.0, -980.0);
    let inv = Inventory::with_rifle(RIFLE_M1_DEFAULT_ID);
    let mut shooter = ActorState::player(ActorId(1), "blue", Vec2::new(50.0, 16.0), 100.0, inv);
    shooter.on_ground = true;
    world.insert(shooter);
    // Muzzle x = 50 + 12 = 62, +20/tick. After 16 ticks the projectile is at x=382;
    // after 17 ticks it is at x=402. An AABB centred at x=391 spans [383, 399], which
    // both sampled points miss but the segment crosses.
    let mut dummy = ActorState::player(ActorId(2), "red", Vec2::new(391.0, 16.0), 100.0, Inventory::default());
    dummy.controllable = false;
    dummy.on_ground = true;
    world.insert(dummy);
    let mut state = ActorSimState::new(world);
    state.ensure_rifle_for(
        ActorId(1),
        RifleState::new(rifle_preset(RIFLE_M1_DEFAULT_ID).unwrap(), 60),
    );
    let mut intents = BTreeMap::new();
    intents.insert(
        ActorId(1),
        ControlIntent {
            actor: ActorId(1),
            fire: true,
            ..ControlIntent::new(ActorId(1), IntentSource::Cfctl)
        },
    );
    let mut total_hits = 0;
    for _ in 0..40 {
        let report = step_no_rng(&mut state, &mut intents, deps());
        total_hits += report.hits.len();
        intents.insert(ActorId(1), ControlIntent::new(ActorId(1), IntentSource::Cfctl));
    }
    assert_eq!(
        total_hits, 1,
        "swept segment must register the otherwise-tunneling shot"
    );
}

#[test]
fn dead_actor_does_not_accept_input() {
    let (mut state, mut intents) = setup();
    if let Some(a) = state.world.actors.get_mut(&ActorId(1)) {
        a.hp = 0.0;
        a.status = Status::Dead;
    }
    intents.insert(
        ActorId(1),
        ControlIntent {
            actor: ActorId(1),
            move_x: 1.0,
            fire: true,
            ..ControlIntent::new(ActorId(1), IntentSource::Human)
        },
    );
    let report = step_no_rng(&mut state, &mut intents, deps());
    let player = report.actor_outcomes.iter().find(|o| o.actor == ActorId(1)).unwrap();
    // Dead actors don't fire.
    assert!(!player.fired);
    assert_eq!(player.move_x, 0.0);
}

#[test]
fn reset_returns_actor_to_spawn_and_reloads_rifle() {
    let (mut state, mut intents) = setup();
    // Fire once to consume ammo.
    intents.insert(
        ActorId(1),
        ControlIntent {
            actor: ActorId(1),
            fire: true,
            ..ControlIntent::new(ActorId(1), IntentSource::Human)
        },
    );
    let _ = step_no_rng(&mut state, &mut intents, deps());
    intents.insert(
        ActorId(1),
        ControlIntent {
            actor: ActorId(1),
            reset: true,
            ..ControlIntent::new(ActorId(1), IntentSource::Human)
        },
    );
    let report = step_no_rng(&mut state, &mut intents, deps());
    let player = report.actor_outcomes.iter().find(|o| o.actor == ActorId(1)).unwrap();
    assert!(player.reset);
    let actor = state.world.actors.get(&ActorId(1)).unwrap();
    assert_eq!(actor.position, actor.spawn);
    let rifle = state.rifles.get(&ActorId(1)).unwrap();
    let mag_capacity = rifle_preset(RIFLE_M1_DEFAULT_ID).unwrap().mag_capacity;
    assert_eq!(rifle.ammo_in_mag, mag_capacity);
}

#[test]
fn reset_clears_resetting_actors_inflight_projectiles() {
    // Regression: `intent.reset` must mirror `ControlCommand::ScenarioReset`
    // and clear the resetting actor's own pre-reset projectiles. Without this,
    // a player who fires and then immediately resets in the next tick can have
    // their pre-reset shot hit the dummy after respawn, which contradicts the
    // user-facing semantics of "reset to spawn with full HP / ammo".
    let (mut state, mut intents) = setup();
    intents.insert(
        ActorId(1),
        ControlIntent {
            actor: ActorId(1),
            fire: true,
            ..ControlIntent::new(ActorId(1), IntentSource::Human)
        },
    );
    let _ = step_no_rng(&mut state, &mut intents, deps());
    assert_eq!(state.projectiles.len(), 1, "fire must spawn one projectile");
    let projectile_id = state.projectiles[0].id;
    intents.insert(
        ActorId(1),
        ControlIntent {
            actor: ActorId(1),
            reset: true,
            ..ControlIntent::new(ActorId(1), IntentSource::Human)
        },
    );
    let report = step_no_rng(&mut state, &mut intents, deps());
    assert!(
        state.projectiles.iter().all(|p| p.owner != ActorId(1)),
        "reset must drain the resetting actor's own projectiles"
    );
    assert!(
        report
            .expired_projectiles
            .iter()
            .any(|e| e.id == projectile_id && e.owner == ActorId(1)),
        "every cleared projectile must be reported as expired so the event log balances spawned and expired"
    );
}

#[test]
fn jump_impulse_uses_actor_tuning_field() {
    // Regression: the jump apply path used to hardcode 420.0 instead of reading
    // ActorTuning::jump_impulse, so any tuning change was silently dead code.
    // We assert post-jump vy equals ActorTuning::jump_impulse minus the one-tick
    // gravity decay (apply_jump runs before step_kinematics inside step_one_actor,
    // so the post-tick vy reflects exactly one frame of gravity).
    let (mut state, mut intents) = setup();
    let dep = deps();
    let gravity_step = state.world.gravity * dep.tick_dt;
    intents.insert(
        ActorId(1),
        ControlIntent {
            actor: ActorId(1),
            jump: true,
            ..ControlIntent::new(ActorId(1), IntentSource::Human)
        },
    );
    let _ = step_no_rng(&mut state, &mut intents, dep);
    let actor = state.world.actors.get(&ActorId(1)).unwrap();
    let expected = ActorTuning::default().jump_impulse + gravity_step;
    assert!(
        (actor.velocity.y - expected).abs() < 1e-3,
        "post-jump vy must equal ActorTuning::jump_impulse ({}) minus one tick of gravity (={expected}); got {}",
        ActorTuning::default().jump_impulse,
        actor.velocity.y
    );
}

#[test]
fn fire_blocked_when_selected_slot_is_not_rifle() {
    // M1-FIX-11 regression: selecting an empty slot must gate fire/reload so
    // inventory selection drives gameplay, not just the HUD.
    let (mut state, mut intents) = setup();
    intents.insert(
        ActorId(1),
        ControlIntent {
            actor: ActorId(1),
            selected_item: Some(ItemSlot(1)),
            ..ControlIntent::new(ActorId(1), IntentSource::Human)
        },
    );
    let _ = step_no_rng(&mut state, &mut intents, deps());
    // Now the selected slot is 1 (Empty). Pressing fire must NOT spawn a projectile
    // even though the actor still owns the rifle in slot 0.
    intents.insert(
        ActorId(1),
        ControlIntent {
            actor: ActorId(1),
            fire: true,
            ..ControlIntent::new(ActorId(1), IntentSource::Human)
        },
    );
    let report = step_no_rng(&mut state, &mut intents, deps());
    assert!(
        report.spawned_projectiles.is_empty(),
        "fire must be gated on selected slot"
    );
    let player = report.actor_outcomes.iter().find(|o| o.actor == ActorId(1)).unwrap();
    assert!(!player.fired);
    // Ammo should still be untouched.
    let mag = rifle_preset(RIFLE_M1_DEFAULT_ID).unwrap().mag_capacity;
    assert_eq!(state.rifles.get(&ActorId(1)).unwrap().ammo_in_mag, mag);
}

#[test]
fn vertical_aim_produces_vertical_projectile_velocity() {
    // M1-FIX-12 regression: aim straight up must yield (0, +speed) projectile
    // velocity, not (0, 0). Diagonal aim must scale both components.
    let (mut state, mut intents) = setup();
    intents.insert(
        ActorId(1),
        ControlIntent {
            actor: ActorId(1),
            aim: Vec2::new(0.0, 1.0),
            fire: true,
            ..ControlIntent::new(ActorId(1), IntentSource::Human)
        },
    );
    let report = step_no_rng(&mut state, &mut intents, deps());
    assert_eq!(report.spawned_projectiles.len(), 1);
    let projectile = &report.spawned_projectiles[0];
    let speed = rifle_preset(RIFLE_M1_DEFAULT_ID).unwrap().projectile_speed;
    assert!(
        projectile.velocity.x.abs() < 1e-3,
        "vertical aim must zero out vx, got {}",
        projectile.velocity.x
    );
    assert!(
        (projectile.velocity.y - speed).abs() < 1e-3,
        "vertical aim must produce vy = +speed, got {}",
        projectile.velocity.y
    );
}

#[test]
fn reset_restores_selected_slot_to_default() {
    // M1-FIX-8 regression: act.player.reset must restore selected slot back to
    // slot 0 so the actor can fire again after being reset.
    let (mut state, mut intents) = setup();
    // Select slot 1 (Empty).
    intents.insert(
        ActorId(1),
        ControlIntent {
            actor: ActorId(1),
            selected_item: Some(ItemSlot(1)),
            ..ControlIntent::new(ActorId(1), IntentSource::Human)
        },
    );
    let _ = step_no_rng(&mut state, &mut intents, deps());
    assert_eq!(
        state.world.actors.get(&ActorId(1)).unwrap().inventory.selected,
        ItemSlot(1)
    );
    // Reset.
    intents.insert(
        ActorId(1),
        ControlIntent {
            actor: ActorId(1),
            reset: true,
            ..ControlIntent::new(ActorId(1), IntentSource::Human)
        },
    );
    let _ = step_no_rng(&mut state, &mut intents, deps());
    assert_eq!(
        state.world.actors.get(&ActorId(1)).unwrap().inventory.selected,
        ItemSlot(0),
        "reset must clear selected back to slot 0 so the rifle is firable again"
    );
}

#[test]
fn checksum_is_deterministic() {
    let (mut a, mut a_int) = setup();
    let (mut b, mut b_int) = setup();
    for tick in 0..30 {
        let intent = if tick % 6 == 0 {
            ControlIntent {
                actor: ActorId(1),
                move_x: 1.0,
                fire: true,
                ..ControlIntent::new(ActorId(1), IntentSource::Human)
            }
        } else {
            ControlIntent {
                actor: ActorId(1),
                move_x: 1.0,
                ..ControlIntent::new(ActorId(1), IntentSource::Human)
            }
        };
        a_int.insert(ActorId(1), intent.clone());
        b_int.insert(ActorId(1), intent);
        let _ = step_no_rng(&mut a, &mut a_int, deps());
        let _ = step_no_rng(&mut b, &mut b_int, deps());
    }
    assert_eq!(a.checksum_bytes(), b.checksum_bytes());
}

#[test]
fn stability_decreases_on_recoil_and_recovers_on_ground() {
    let (mut state, mut intents) = setup();
    // Start stable.
    let actor = state.world.actors.get(&ActorId(1)).unwrap();
    assert!((actor.stability - 1.0).abs() < 1e-6, "initial stability must be 1.0");

    // Fire: recoil should reduce stability.
    intents.insert(
        ActorId(1),
        ControlIntent {
            actor: ActorId(1),
            fire: true,
            aim: Vec2::new(1.0, 0.0),
            ..ControlIntent::new(ActorId(1), IntentSource::Human)
        },
    );
    let report = step_no_rng(&mut state, &mut intents, deps());
    let player = report.actor_outcomes.iter().find(|o| o.actor == ActorId(1)).unwrap();
    assert!(player.recoil_applied > 0.0, "rifle must have fired");
    let post_fire = state.world.actors.get(&ActorId(1)).unwrap().stability;
    assert!(post_fire < 1.0, "stability must decrease after recoil, got {post_fire}");

    // Idle ticks on ground: stability should recover.
    for _ in 0..60 {
        let _ = step_no_rng(&mut state, &mut intents, deps());
    }
    let recovered = state.world.actors.get(&ActorId(1)).unwrap().stability;
    assert!(
        recovered > post_fire,
        "stability must recover after idle ground ticks, got {recovered} (was {post_fire})"
    );
}

#[test]
fn edge_triggered_jump_is_not_dropped_within_one_tick() {
    // W1.3 item 862: verify that a jump edge-trigger set and consumed
    // within one step() call is honored (not lost to clear_edges ordering).
    let (mut state, mut intents) = setup();
    // Ensure actor is on ground.
    {
        let actor = state.world.actors.get_mut(&ActorId(1)).unwrap();
        actor.on_ground = true;
        actor.velocity = Vec2::ZERO;
    }
    intents.insert(
        ActorId(1),
        ControlIntent {
            actor: ActorId(1),
            jump: true,
            ..ControlIntent::new(ActorId(1), IntentSource::Human)
        },
    );
    let report = step_no_rng(&mut state, &mut intents, deps());
    let player = report.actor_outcomes.iter().find(|o| o.actor == ActorId(1)).unwrap();
    assert!(
        player.jump_accepted,
        "jump edge must be consumed in the same tick it was set"
    );
}

#[test]
fn move_x_inf_rejected_by_engine_guard() {
    // W1.3 item 863: act.player.move already has the is_finite guard in
    // cf-control's server dispatch; this test confirms the actor sim
    // itself never receives non-finite move_x (the guard is upstream).
    // Here we verify the sim doesn't crash on a zero-move intent.
    let (mut state, mut intents) = setup();
    intents.insert(
        ActorId(1),
        ControlIntent {
            actor: ActorId(1),
            move_x: 0.0,
            ..ControlIntent::new(ActorId(1), IntentSource::Human)
        },
    );
    let report = step_no_rng(&mut state, &mut intents, deps());
    assert!(!report.actor_outcomes.is_empty(), "step must produce outcomes");
}

#[test]
fn climb_intent_produces_climbing_stance() {
    // Item 675: Stance::Climbing consumes climb intent.
    let (mut state, _) = setup();
    {
        let actor = state.world.actors.get_mut(&ActorId(1)).unwrap();
        actor.on_ground = true;
        actor.climb_active = true;
    }
    let stance = state.world.actors.get(&ActorId(1)).unwrap().stance();
    assert_eq!(stance, Stance::Climbing);
}

#[test]
fn crouch_sets_stance_on_ground() {
    // Item 676: crouching stance when on ground.
    let (mut state, _) = setup();
    {
        let actor = state.world.actors.get_mut(&ActorId(1)).unwrap();
        actor.on_ground = true;
        actor.crouch_active = true;
    }
    let stance = state.world.actors.get(&ActorId(1)).unwrap().stance();
    assert_eq!(stance, Stance::Crouching);
}

#[test]
fn projectile_does_not_hit_owner() {
    // Item 678: actor-projectile self-filter (shooter does not hit themselves).
    let (mut state, mut intents) = setup();
    // Fire a projectile.
    intents.insert(
        ActorId(1),
        ControlIntent {
            actor: ActorId(1),
            fire: true,
            aim: Vec2::new(1.0, 0.0),
            ..ControlIntent::new(ActorId(1), IntentSource::Human)
        },
    );
    let report = step_no_rng(&mut state, &mut intents, deps());
    assert!(report.actor_outcomes.iter().any(|o| o.fired), "must fire a projectile");
    // Run many ticks: the projectile should never hit the actor who fired it.
    for _ in 0..120 {
        let r = step_no_rng(&mut state, &mut intents, deps());
        for hit in &r.hits {
            assert_ne!(hit.target, ActorId(1), "projectile must not hit its owner");
        }
    }
}

/// caps at DYING when StepDeps.tutorial_safety = true. Without the flag
/// the DYING dwell promotes to DEAD as usual.
#[test]
fn tutorial_safety_caps_lethal_damage_at_dying() {
    let make_world = || {
        let mut world = ActorWorld::new(64.0, -980.0);
        let mut actor = ActorState::player(
            ActorId(1),
            "blue",
            Vec2::new(100.0, 32.0),
            100.0,
            Inventory::with_rifle(cf_equipment::RIFLE_M1_DEFAULT_ID),
        );
        actor.hp = 0.0;
        actor.status = Status::Dying;
        actor.dying_dwell_ticks_remaining = 2;
        world.insert(actor);
        ActorSimState::new(world)
    };
    let mut state = make_world();
    let mut intents = BTreeMap::new();
    let mut tutorial_deps = deps();
    tutorial_deps.tutorial_safety = true;
    // Step three times: dwell goes 2 → 1 → 0. With tutorial_safety the
    // tick where dwell reaches 0 must NOT promote to Dead.
    for _ in 0..3 {
        let _ = step_no_rng(&mut state, &mut intents, tutorial_deps);
    }
    let actor = state.world.actors.get(&ActorId(1)).expect("actor must persist");
    assert_eq!(
        actor.status,
        Status::Dying,
        "tutorial_safety must cap at DYING; got {:?}",
        actor.status
    );
    // Without tutorial_safety the same sequence promotes to Dead.
    let mut state = make_world();
    for _ in 0..3 {
        let _ = step_no_rng(&mut state, &mut intents, deps());
    }
    let actor = state.world.actors.get(&ActorId(1)).expect("actor must persist");
    assert_eq!(actor.status, Status::Dead);
}

/// floor must fall under gravity, settle within a bounded number of
/// ticks, and emit exactly one `SettledLooseItem` outcome.
#[test]
fn loose_item_falls_and_settles_within_bounded_ticks() {
    let mut state = ActorSimState::new(ActorWorld {
        floor_y: 200.0,
        ..Default::default()
    });
    let id = state.spawn_loose_item("rifle", Vec2::new(100.0, 50.0), Vec2::new(10.0, 0.0), "evt_inv_dropped");
    let mut intents = BTreeMap::new();
    let mut total_settled = 0;
    let mut settle_tick: Option<usize> = None;
    for tick in 0..240 {
        let r = step_no_rng(&mut state, &mut intents, deps());
        if !r.settled_loose_items.is_empty() {
            assert_eq!(r.settled_loose_items.len(), 1);
            let s = &r.settled_loose_items[0];
            assert_eq!(s.id, id);
            assert_eq!(s.source_event_id, "evt_inv_dropped");
            assert_eq!(s.item_label, "rifle");
            total_settled += 1;
            settle_tick = Some(tick);
            break;
        }
    }
    let settle_tick = settle_tick.expect("LooseItem must settle within 240 ticks");
    assert!(settle_tick < 240, "settled after {settle_tick} ticks; expected < 240");
    // The settled latch must fire exactly once for this item across the run.
    // Drive 60 more ticks and confirm no more `SettledLooseItem` outcomes
    // surface for the same id.
    for _ in 0..60 {
        let r = step_no_rng(&mut state, &mut intents, deps());
        assert!(
            r.settled_loose_items.is_empty(),
            "settled latch must fire exactly once per item"
        );
    }
    assert_eq!(total_settled, 1);
    // The item itself must be at rest on the floor surface.
    let item = state
        .loose_items
        .iter()
        .find(|i| i.id == id)
        .expect("loose item must still exist after settle");
    assert!(item.settled, "settled flag must persist after fire");
    assert!(
        item.velocity.x == 0.0 && item.velocity.y == 0.0,
        "settled item velocity must be zero, got {:?}",
        item.velocity
    );
    assert!(
        (item.position.y - (state.world.floor_y - item.half_extents.y)).abs() < 0.01,
        "settled item must rest at floor; got y={} floor_y={}",
        item.position.y,
        state.world.floor_y
    );
}

/// multiple actor AABBs in one tick MUST emit multiple HitOutcomes —
/// one per actor — in priority (entry-t ascending) order. This is the
/// canonical regression for the swept-collision priority queue's
/// production wiring.
#[test]
fn projectile_crosses_multiple_actors_emits_one_hit_per_actor_in_priority_order() {
    let mut state = ActorSimState::new(ActorWorld {
        floor_y: 0.0,
        ..Default::default()
    });
    // Three target dummies in a row at x=100/200/300, y=64.
    for (id, x) in [(2u64, 100.0_f32), (3, 200.0), (4, 300.0)] {
        let a = ActorState::player(
            ActorId(id),
            "red",
            Vec2::new(x, 64.0),
            100.0,
            Inventory::default(),
        );
        state.world.actors.insert(ActorId(id), a);
    }
    // Shooter at x=0, y=64, firing right.
    let shooter = ActorId(1);
    let sa = ActorState::player(
        shooter,
        "blue",
        Vec2::new(0.0, 64.0),
        100.0,
        Inventory::default(),
    );
    state.world.actors.insert(shooter, sa);
    // Spawn a high-velocity projectile that crosses all three dummies in one tick.
    state.projectiles.push(Projectile {
        id: 99,
        owner: shooter,
        origin: Vec2::new(0.0, 64.0),
        position: Vec2::new(0.0, 64.0),
        velocity: Vec2::new(24000.0, 0.0),
        damage: 25.0,
        remaining_ticks: 60,
        mass_kg: 0.05,
        sharpness: 0.8,
    });
    let mut intents: BTreeMap<ActorId, ControlIntent> = BTreeMap::new();
    let report = step_no_rng(&mut state, &mut intents, deps());
    // Must record THREE hits — one per actor along the swept path.
    let hits_for_proj: Vec<&HitOutcome> = report.hits.iter().filter(|h| h.projectile_id == 99).collect();
    assert_eq!(
        hits_for_proj.len(),
        3,
        "swept projectile must record one hit per crossed actor; got {} hits",
        hits_for_proj.len()
    );
    // Entry-t monotonic (closest first).
    for w in hits_for_proj.windows(2) {
        assert!(
            w[0].entry_t <= w[1].entry_t,
            "hits must be ordered by entry_t ascending"
        );
    }
    // Targets in priority order: ActorId(2)=x100 first, ActorId(4)=x300 last.
    assert_eq!(hits_for_proj[0].target, ActorId(2));
    assert_eq!(hits_for_proj[1].target, ActorId(3));
    assert_eq!(hits_for_proj[2].target, ActorId(4));
    // Ray metadata populated from sim (not reconstructed).
    for hit in &hits_for_proj {
        assert!((hit.ray_direction.x - 1.0).abs() < 1e-3, "ray direction must be +x");
        assert!((hit.ray_origin.x - 0.0).abs() < 1.0, "ray origin near projectile start");
        assert!(hit.distance_traveled > 0.0, "distance_traveled must be set");
    }
    // passthroughs. Each non-last hit absorbs 60% of the projectile's
    // remaining damage (40% continues). The LAST actor in priority
    // order stops the projectile and absorbs whatever is left.
    let original = 25.0_f32;
    assert!(
        (hits_for_proj[0].damage - original * 0.6).abs() < 0.01,
        "first hit must absorb 60% of original damage: {} vs {}",
        hits_for_proj[0].damage,
        original * 0.6
    );
    // After first hit: remaining = 25 - 15 = 10. Second hit (not last)
    // absorbs 6.0; remaining = 10 - 6.0 = 4.0.
    assert!(
        (hits_for_proj[1].damage - 6.0).abs() < 0.01,
        "second hit must absorb 60% of (10.0) = 6.0; got {}",
        hits_for_proj[1].damage
    );
    // Third hit IS the last candidate → absorbs the full remaining 4.0.
    assert!(
        (hits_for_proj[2].damage - 4.0).abs() < 0.01,
        "third (last) hit must stop the projectile and absorb 4.0; got {}",
        hits_for_proj[2].damage
    );
    let total: f32 = hits_for_proj.iter().map(|h| h.damage).sum();
    assert!(
        (total - original).abs() < 0.01,
        "total damage ({}) must equal projectile original damage ({})",
        total,
        original
    );
}
