//! M8A § Files / cf-control — ECS component decomposition scaffold.
//!
//! M8A spec § Architecture rules rule 1: "No subsystem holds the entire
//! engine state behind one mutex."
//!
//! The existing `RwLock<EngineMutable>` mega-mutex at engine.rs:600
//! remains in place at M8A to preserve M1-M8 byte-identical replay
//! behavior (per the backfill-discipline lock). This module exposes the
//! ECS-component decomposition that the M9+ engine-host integration
//! migrates onto, one subsystem at a time. The components mirror the
//! shape of `EngineMutable`'s sub-fields so the migration is a
//! mechanical refactor with deterministic round-trip.
//!
//! Decomposition:
//!
//! - `ActorWorldComponent`  — the M1 actor world snapshot + mutations.
//! - `ChunkedTerrainComponent` — M3 terrain chunks + dirty tracking.
//! - `MissionComponent` — M2 mission director state.
//! - `RecorderComponent` — M4 event recorder + shard merge.
//! - `ProjectilePoolComponent` — M1 projectile pool (SlotMap).
//! - `ReactorWorldComponent` — M9 reactor world (forward-compat).

use serde::{Deserialize, Serialize};

/// **M8A**: marker types only at M8A. The actual sub-state lives in
/// `cf_control::EngineMutable` until M9+ engine-host refactor migrates
/// each subsystem one at a time.
///
/// Each component is intentionally empty at M8A; cf-app and the M9+
/// engine-host wrap with `#[derive(Component)]` newtypes that hold the
/// actual data references via `Arc<RwLock<...>>` until the migration
/// completes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ActorWorldComponent;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ChunkedTerrainComponent;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct MissionComponent;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RecorderComponent;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ProjectilePoolComponent;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ReactorWorldComponent;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn components_are_zero_sized() {
        assert_eq!(std::mem::size_of::<ActorWorldComponent>(), 0);
        assert_eq!(std::mem::size_of::<ChunkedTerrainComponent>(), 0);
        assert_eq!(std::mem::size_of::<MissionComponent>(), 0);
        assert_eq!(std::mem::size_of::<RecorderComponent>(), 0);
        assert_eq!(std::mem::size_of::<ProjectilePoolComponent>(), 0);
        assert_eq!(std::mem::size_of::<ReactorWorldComponent>(), 0);
    }
}
