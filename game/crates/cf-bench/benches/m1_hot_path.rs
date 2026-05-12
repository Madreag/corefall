//! **M1 Enhancement P1**: criterion benches for the M1 hot path.
//!
//! Three benchmarks:
//! - `bench_sim_step`: 50 actors, 1000 ticks of `cf_actor::sim::step` with
//!   the player firing while moving. Target: p99 < 0.5 ms per step on a
//!   developer machine at 60 Hz.
//! - `bench_checksum_world`: serialize + checksum a 50-actor world. Should
//!   stay under ~100 µs.
//! - `bench_observe_frame_serialize`: serialize an ActorObservation for the
//!   player. Should stay under ~10 µs.
//!
//! Run with: `cargo bench -p cf-bench --bench m1_hot_path`.

use std::collections::BTreeMap;

use cf_actor::{
    sim::{step_no_rng, ActorSimState, StepDeps},
    ActorId, ActorObservation, ActorState, ActorWorld, ControlIntent, IntentSource, Inventory, Vec2,
};
use cf_equipment::{rifle_preset, RifleState, RIFLE_M1_DEFAULT_ID};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn build_world(actor_count: u32) -> ActorSimState {
    let mut world = ActorWorld::new(0.0, -980.0);
    for i in 0..actor_count {
        let inv = if i == 0 {
            Inventory::with_rifle(RIFLE_M1_DEFAULT_ID)
        } else {
            Inventory::default()
        };
        let spawn = Vec2::new(50.0 + (i as f32) * 24.0, 16.0);
        let mut actor = ActorState::player(ActorId(u64::from(i + 1)), "blue", spawn, 100.0, inv);
        actor.on_ground = true;
        if i != 0 {
            actor.controllable = false;
        }
        world.insert(actor);
    }
    let mut state = ActorSimState::new(world);
    let spec = rifle_preset(RIFLE_M1_DEFAULT_ID).unwrap();
    state.ensure_rifle_for(ActorId(1), RifleState::new(spec, 60));
    state
}

fn bench_sim_step(c: &mut Criterion) {
    let deps = StepDeps {
        tick_dt: 1.0 / 60.0,
        region_min_x: 0.0,
        region_max_x: 1280.0,
        region_max_y: 720.0,
        auto_reload_when_empty: false,
        tuning: None,
        tutorial_safety: false,
    };
    c.bench_function("sim_step_50_actors", |b| {
        b.iter(|| {
            let mut state = build_world(50);
            let mut intents: BTreeMap<ActorId, ControlIntent> = BTreeMap::new();
            for _ in 0..1000 {
                intents.insert(
                    ActorId(1),
                    ControlIntent {
                        actor: ActorId(1),
                        move_x: 1.0,
                        ..ControlIntent::new(ActorId(1), IntentSource::Cfctl)
                    },
                );
                let report = step_no_rng(&mut state, &mut intents, deps);
                black_box(report);
            }
        });
    });
}

fn bench_checksum_world(c: &mut Criterion) {
    let state = build_world(50);
    c.bench_function("checksum_world_50_actors", |b| {
        b.iter(|| {
            let bytes = state.checksum_bytes();
            black_box(bytes);
        });
    });
}

fn bench_observe_frame_serialize(c: &mut Criterion) {
    let state = build_world(1);
    let actor = state.world.actors.values().next().expect("at least one actor");
    let observation = ActorObservation::from(actor);
    c.bench_function("observe_actor_observation_serialize", |b| {
        b.iter(|| {
            let json = serde_json::to_string(&observation).expect("serialize");
            black_box(json);
        });
    });
}

criterion_group!(
    benches,
    bench_sim_step,
    bench_checksum_world,
    bench_observe_frame_serialize
);
criterion_main!(benches);
