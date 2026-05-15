//! M8A § Backfill discipline — cfctl JSON-RPC wire-shape lock.
//!
//! M8A's parallel-determinism refactor MUST NOT change the cfctl wire
//! shape. `SCHEMA_VERSION` stays frozen at its M0-M8 value; M8A's
//! changes are all internal (ECS components, scheduler graph, per-thread
//! recorder shards). cfctl handlers continue to read/write via World
//! queries (M9+ migration) instead of `RwLock<EngineMutable>`.
//!
//! This test asserts `SCHEMA_VERSION == 2` (the M0-M8 baseline) so any
//! accidental bump by an M8A or downstream PR fails CI immediately.

use cf_control::schemas::{SCHEMA_VERSION, SCHEMA_VERSION_MIN};

#[test]
fn m8a_locks_cfctl_schema_version_at_2() {
    assert_eq!(
        SCHEMA_VERSION, 2,
        "M8A frozen cfctl wire shape: SCHEMA_VERSION must stay at 2"
    );
}

#[test]
fn m8a_locks_cfctl_schema_version_min_at_1() {
    assert_eq!(
        SCHEMA_VERSION_MIN, 1,
        "M8A frozen cfctl wire shape: SCHEMA_VERSION_MIN must stay at 1"
    );
}

#[test]
fn m8a_ecs_components_exist() {
    // Spec § Files / cf-control — ECS component decomposition scaffold.
    // The components are zero-sized marker types at M8A; M9+ wraps them
    // with real Bevy components in cf-app.
    use cf_control::components::{
        ActorWorldComponent, ChunkedTerrainComponent, MissionComponent, ProjectilePoolComponent, ReactorWorldComponent,
        RecorderComponent,
    };
    let _aw = ActorWorldComponent;
    let _ct = ChunkedTerrainComponent;
    let _m = MissionComponent;
    let _r = RecorderComponent;
    let _pp = ProjectilePoolComponent;
    let _rw = ReactorWorldComponent;
}

#[test]
fn m8a_engine_world_wrapper_constructible() {
    use cf_control::world::EngineWorld;
    let _world = EngineWorld::new();
}
