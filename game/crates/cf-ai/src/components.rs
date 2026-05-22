//! M8A § Files / cf-ai — ECS component scaffold.
//!
//! M8A's parallel-determinism refactor exposes `GuardComponent` as a
//! standalone Bevy-ready type. The cf-ai crate stays
//! determinism-locked (no Bevy dependency); cf-app and the M9+ engine
//! host wrap with `#[derive(Component)]` newtypes.
//!
//! BotMemory + PriorityTable + reason_label_recent are already shipped at
//! M7 (cf-ai/src/{bot_memory,priority,reason_label}.rs); M8A's
//! contribution is the budget retune (2.0 ms → 4.0 ms p99; see
//! `cf-ai/src/constants.rs`) + the snapshot/restore round-trip
//! verification in `cf-ai/src/systems.rs`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct GuardId(pub u32);

/// lives in cf-ai::sim and cf-ai::thinking_stack; this component is the
/// surface the M9+ engine host queries via ECS.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct GuardComponent {
    pub id: GuardId,
    pub alive: bool,
    pub last_tick_reason_label: String,
}
