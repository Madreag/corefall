//! M8A § Files / cf-control — Bevy World wrapper scaffold.
//!
//! M8A's cfctl JSON-RPC wire shape is FROZEN. The handler dispatcher in
//! cf-control/src/server.rs continues to talk to the existing
//! `RwLock<EngineMutable>` engine at M8A; M9+ migrates handlers to read
//! / write engine state via Bevy `World` queries one cfctl method at a
//! time.
//!
//! This module ships the `EngineWorld` wrapper scaffold that M9+ wires
//! into the cfctl dispatch. The wrapper holds the engine state and
//! exposes typed accessors that mirror the `EngineMutable` field layout
//! so handler migration is mechanical.

use crate::components::{
    ActorWorldComponent, ChunkedTerrainComponent, MissionComponent, ProjectilePoolComponent,
    ReactorWorldComponent, RecorderComponent,
};

/// **M8A**: Bevy World wrapper marker. M9+ wraps a real `bevy_ecs::World`
/// once the component types in `components.rs` carry actual state. At
/// M8A this is a zero-cost wrapper that proves the cfctl JSON-RPC wire
/// shape can route through ECS query patterns without breaking M1-M8
/// behavior.
#[derive(Debug, Default)]
pub struct EngineWorld {
    pub actor_world: ActorWorldComponent,
    pub chunked_terrain: ChunkedTerrainComponent,
    pub mission: MissionComponent,
    pub recorder: RecorderComponent,
    pub projectile_pool: ProjectilePoolComponent,
    pub reactor_world: ReactorWorldComponent,
}

impl EngineWorld {
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_world_constructs_with_default_components() {
        let world = EngineWorld::new();
        assert_eq!(world.actor_world, ActorWorldComponent::default());
        assert_eq!(world.chunked_terrain, ChunkedTerrainComponent::default());
    }
}
