//! M7B: Cortex-Command-style commander hopping.
//!
//! Spec § "Y (gamepad) / Backspace (KBM) enters Brain-Hop, freezes time,
//! surfaces a LOS-radial list of squad members; selecting one transfers M5
//! input control. The previously-held actor reverts to AI under the *same*
//! squad doctrine. Squad state (verb + formation + priority table + roles)
//! lives on the squad, not the held actor, so doctrine survives the hop."
//!
//! `commander_hop.rs` is the algorithm + state-machine side of the hop. The
//! M5 input router consumes [`HopResult`] and re-routes player events; the
//! cf-control engine wires that through. Squad state is not mutated by the
//! hop — that's the whole point. Replay determinism is preserved because
//! the hop is recorded as a single `squad.brain_hop` event.

use serde::{Deserialize, Serialize};

/// **M7B**: per-hop transition state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommanderHopState {
    /// Tick the hop was initiated.
    pub initiated_tick: u64,
    /// Whether time is currently frozen (only true while the LOS-radial
    /// selector is open).
    pub time_frozen: bool,
    /// Actor id that was being held when the hop started.
    pub from_actor_id: u64,
    /// Optional pre-selected target (set when the player has committed in
    /// the radial selector). `None` means the selector is still open.
    pub committed_target: Option<u64>,
}

impl CommanderHopState {
    pub fn open(from_actor_id: u64, tick: u64) -> Self {
        Self {
            initiated_tick: tick,
            time_frozen: true,
            from_actor_id,
            committed_target: None,
        }
    }
}

/// **M7B**: result of resolving a hop. Surfaced as `squad.brain_hop`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HopResult {
    pub from_actor_id: u64,
    pub to_actor_id: u64,
    pub initiated_tick: u64,
    pub completed_tick: u64,
    pub squad_id: u64,
    /// True iff `from_actor_id` and `to_actor_id` were both members of the
    /// same squad at the moment of hop. The engine consumes this for the
    /// "doctrine survives the hop" branch — the recipient inherits the
    /// existing squad command + formation without mutation.
    pub same_squad: bool,
}

/// **M7B**: validate + finalize a brain-hop.
///
/// Inputs come from the engine's resolved world state. The function is
/// pure: it does not mutate any cf-ai state; the engine commits the
/// returned `HopResult` to its actor/world graph and the replay event
/// stream.
pub fn finalize_hop(
    state: &CommanderHopState,
    target_actor_id: u64,
    squad_id: u64,
    same_squad: bool,
    completed_tick: u64,
) -> Result<HopResult, HopError> {
    if state.from_actor_id == target_actor_id {
        return Err(HopError::SameActor);
    }
    if completed_tick < state.initiated_tick {
        return Err(HopError::CompletedBeforeInitiated);
    }
    Ok(HopResult {
        from_actor_id: state.from_actor_id,
        to_actor_id: target_actor_id,
        initiated_tick: state.initiated_tick,
        completed_tick,
        squad_id,
        same_squad,
    })
}

/// **M7B**: hop validation errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HopError {
    SameActor,
    CompletedBeforeInitiated,
}

impl HopError {
    pub fn label(self) -> &'static str {
        match self {
            HopError::SameActor => "brain_hop_same_actor",
            HopError::CompletedBeforeInitiated => "brain_hop_invalid_timing",
        }
    }
}

/// **M7B**: one candidate row produced by [`build_los_radial`]. The UI
/// renders these in a wheel around the held actor; the bearing drives the
/// wheel slot, the distance drives the inner / outer ring.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LosRadialCandidate {
    pub actor_id: u64,
    pub bearing_degrees: f32,
    pub distance: f32,
    pub has_los: bool,
}

/// **M7B**: build the LOS-radial selector candidate list per spec
/// § "surfaces a LOS-radial list of squad members". The caller supplies
/// the held actor's world position + the candidate `(actor_id, position,
/// has_los)` rows; the helper computes bearing + distance and emits a
/// deterministically-sorted list (by bearing, then actor_id) for stable
/// replay rendering.
pub fn build_los_radial(
    holder_pos: [f32; 2],
    candidates: &[(u64, [f32; 2], bool)],
) -> Vec<LosRadialCandidate> {
    let mut out: Vec<LosRadialCandidate> = candidates
        .iter()
        .map(|(id, pos, has_los)| {
            let dx = pos[0] - holder_pos[0];
            let dy = pos[1] - holder_pos[1];
            let distance = (dx * dx + dy * dy).sqrt();
            let bearing_degrees = dy.atan2(dx).to_degrees();
            LosRadialCandidate {
                actor_id: *id,
                bearing_degrees,
                distance,
                has_los: *has_los,
            }
        })
        .collect();
    out.sort_by(|a, b| {
        a.bearing_degrees
            .partial_cmp(&b.bearing_degrees)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.actor_id.cmp(&b.actor_id))
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_freezes_time() {
        let s = CommanderHopState::open(7, 100);
        assert!(s.time_frozen);
        assert_eq!(s.from_actor_id, 7);
    }

    #[test]
    fn finalize_same_actor_rejected() {
        let s = CommanderHopState::open(7, 100);
        let err = finalize_hop(&s, 7, 42, true, 101).unwrap_err();
        assert_eq!(err, HopError::SameActor);
    }

    #[test]
    fn finalize_back_in_time_rejected() {
        let s = CommanderHopState::open(7, 100);
        let err = finalize_hop(&s, 8, 42, true, 50).unwrap_err();
        assert_eq!(err, HopError::CompletedBeforeInitiated);
    }

    #[test]
    fn finalize_records_same_squad() {
        let s = CommanderHopState::open(7, 100);
        let r = finalize_hop(&s, 8, 42, true, 105).unwrap();
        assert_eq!(r.from_actor_id, 7);
        assert_eq!(r.to_actor_id, 8);
        assert_eq!(r.squad_id, 42);
        assert!(r.same_squad);
    }

    #[test]
    fn los_radial_sorts_by_bearing_then_actor_id() {
        let candidates = vec![
            (1, [10.0, 0.0], true),  // bearing 0
            (2, [0.0, 10.0], true),  // bearing 90
            (3, [-10.0, 0.0], false), // bearing 180
            (4, [0.0, -10.0], true), // bearing -90
        ];
        let radial = build_los_radial([0.0, 0.0], &candidates);
        assert_eq!(radial.len(), 4);
        let bearings: Vec<f32> = radial.iter().map(|c| c.bearing_degrees).collect();
        let mut sorted = bearings.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(bearings, sorted, "candidates must be sorted by bearing");
    }

    #[test]
    fn los_radial_records_distance_and_los_flag() {
        let candidates = vec![(7, [3.0, 4.0], false)];
        let radial = build_los_radial([0.0, 0.0], &candidates);
        assert_eq!(radial.len(), 1);
        assert_eq!(radial[0].actor_id, 7);
        assert!((radial[0].distance - 5.0).abs() < 1e-3);
        assert!(!radial[0].has_los);
    }
}
