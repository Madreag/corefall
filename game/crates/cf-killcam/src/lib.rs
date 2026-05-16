//! cf-killcam — M8 killcam stub. Spec § Killcam: 3-second slow-motion
//! replay from the killing enemy's perspective on player death; toggleable
//! per accessibility (`killcam_enabled`). Slow-mo kill cam: 1.5s
//! cinematic camera angle on mission boss final blow.
//!
//! M8 ships the local state machine + transition events; network forward-
//! compat for M36+ multiplayer.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod playback;
pub mod recorder;

pub use playback::{tick, KillcamPhase, KillcamState, KILLCAM_DURATION_MS, SLOW_MO_KILL_CAM_DURATION_MS};
pub use recorder::{start, start_slow_mo_kill_cam, KillcamTrigger};
