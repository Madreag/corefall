//! Per-tick mission I/O — `MissionTickInputs`, `MissionTickReport`,
//! `ObjectiveProgressUpdate`. Split out of `lib.rs` for the 2k-LOC ceiling.
//! Public API is re-exported at the crate root.

use std::collections::BTreeMap;

use cf_actor::{ActorId, ActorState};

use crate::result::MissionResult;

/// Per-tick mission inputs. The engine assembles this from its actor world plus the
/// breach state (broken? in-range? hp_remaining?) and reactor state before calling
/// [`step`].
#[derive(Debug, Clone, Copy)]
pub struct MissionTickInputs<'a> {
    pub tick: u64,
    pub player: Option<&'a ActorState>,
    pub actors: &'a BTreeMap<ActorId, ActorState>,
    /// Map of `breach_id -> broken?`. `true` once the breach is fully carved.
    pub breaches_broken: &'a BTreeMap<String, bool>,
    /// Map of `reactor_id -> destroyed?`. `true` once hp <= 0. Defaults empty.
    pub reactors_destroyed: &'a BTreeMap<String, bool>,
    /// `mission.objective_updated` events at 25/50/75/100% milestones for
    /// `BreachBarrier` objectives. May be left empty; missing ids default to
    /// `0.0` (no progress yet).
    pub breaches_progress: &'a BTreeMap<String, f32>,
}

/// Per-tick report. Every `Vec` carries objective ids; the engine turns each into a
/// `mission.objective_*` event in tick order.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MissionTickReport {
    pub objective_started: Vec<String>,
    pub objective_completed: Vec<String>,
    pub objective_failed: Vec<String>,
    /// quartile) crossed on this tick. `progress` is the milestone value
    /// (0.25, 0.5, 0.75, or 1.0). The engine emits one `mission.objective_updated`
    /// event per entry.
    pub objective_updated: Vec<ObjectiveProgressUpdate>,
    /// Set on the tick the mission resolves (`Won` or `Lost`).
    pub final_result: Option<MissionResult>,
}

/// The engine turns each into a `mission.objective_updated` event with a
/// payload of `{ objective_id, progress }`.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectiveProgressUpdate {
    pub objective_id: String,
    pub progress: f32,
}
