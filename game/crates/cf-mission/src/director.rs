//! M2 — Mission director (minimal).
//!
//! Per the M2 spec's "## Files" section, `cf-mission/src/director.rs` is the
//! canonical home for the timer + objective-completion tick logic. The
//! current implementation lives inside `cf-mission/src/lib.rs::step`; this
//! module re-exports the public surface so consumers that import from
//! `cf_mission::director::*` (per the spec file path) compile cleanly.
//!
//! M2 ships a MINIMAL director:
//! - Single mission, 1-2 objectives in sequence.
//! - Timer-based loss condition.
//! - No pacing graph (M13+ adds Warframe-style flow graph + intensity inputs).
//! - No commander AI on the enemy side.
//! - No reinforcements / waves (M9+ adds simple wave; M13+ adds full
//!   reinforcement budget).
//! - No save/load mid-mission (M13+ adds save schema + resume).

pub use crate::{
    MissionLifecycle, MissionResult, MissionState, MissionTickInputs, MissionTickReport, ObjectiveProgressUpdate,
};

/// canonical `cf_mission::step` so consumers that import per the spec path
/// `cf_mission::director::step` resolve cleanly.
pub fn step(state: &mut MissionState, inputs: MissionTickInputs<'_>) -> MissionTickReport {
    crate::step(state, inputs)
}
