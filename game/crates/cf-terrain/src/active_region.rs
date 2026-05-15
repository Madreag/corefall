//! M8A § Semantic terrain event protocol — active_region wake/sleep
//! state machine.
//!
//! Per M8A spec, the `active_region: bool` flag on Chunk (M3 forward-
//! compat) is enforced at M8A: every write transitions the chunk and its
//! 1-chunk-radius neighbors to `active_region = true`. Chunks idle 300+
//! ticks transition back to `false`. Emit
//! `terrain.chunk_active_region_changed` event on every transition.

use serde::{Deserialize, Serialize};

use crate::constants::CHUNK_SLEEP_IDLE_THRESHOLD_TICKS;

/// **M8A**: chunk wake state. Determines whether the chunk participates
/// in the per-tick CA stepping (M15+) and whether the chunk's mutation
/// path is included in the dirty-chunk batch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ActiveRegionState {
    /// Chunk has not seen an edit recently AND is not the neighbor of a
    /// recently-edited chunk. Skipped in the dirty-chunk batch.
    #[default]
    Sleeping,
    /// Chunk was edited within the last
    /// `CHUNK_SLEEP_IDLE_THRESHOLD_TICKS` ticks OR has a neighbor in
    /// `Awake` state.
    Awake,
}

/// **M8A**: wake-on-edit state transition record. Recorded as a
/// `terrain.chunk_active_region_changed` event for replay determinism.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveRegionTransition {
    pub chunk_coords: (i32, i32),
    pub from_state: ActiveRegionState,
    pub to_state: ActiveRegionState,
    pub tick: u64,
}

/// **M8A**: wake the chunk at `(cx, cy)` and its 1-chunk-radius
/// neighbors. Returns the transitions that occurred (each emitted as a
/// `terrain.chunk_active_region_changed` event by the caller).
///
/// `getter` returns the current state for an arbitrary chunk; `setter`
/// commits the new state. The two are separate so callers can stage the
/// transitions into a snapshot-read / commit-serial pattern.
pub fn wake_chunk_and_neighbors<F, G>(
    cx: i32,
    cy: i32,
    tick: u64,
    mut getter: F,
    mut setter: G,
) -> Vec<ActiveRegionTransition>
where
    F: FnMut(i32, i32) -> ActiveRegionState,
    G: FnMut(i32, i32, ActiveRegionState),
{
    let mut transitions = Vec::new();
    for dy in -1..=1 {
        for dx in -1..=1 {
            let nx = cx + dx;
            let ny = cy + dy;
            let current = getter(nx, ny);
            if current != ActiveRegionState::Awake {
                transitions.push(ActiveRegionTransition {
                    chunk_coords: (nx, ny),
                    from_state: current,
                    to_state: ActiveRegionState::Awake,
                    tick,
                });
                setter(nx, ny, ActiveRegionState::Awake);
            }
        }
    }
    transitions
}

/// **M8A**: transition any chunk that has been idle for more than
/// `CHUNK_SLEEP_IDLE_THRESHOLD_TICKS` ticks to `Sleeping`. Returns the
/// resulting transitions.
pub fn sleep_idle_chunks<F, G, H>(
    tick: u64,
    chunks: &[(i32, i32)],
    last_edit_tick_for: F,
    state_for: G,
    mut setter: H,
) -> Vec<ActiveRegionTransition>
where
    F: Fn(i32, i32) -> u64,
    G: Fn(i32, i32) -> ActiveRegionState,
    H: FnMut(i32, i32, ActiveRegionState),
{
    let mut transitions = Vec::new();
    for &(cx, cy) in chunks {
        let last_edit = last_edit_tick_for(cx, cy);
        if state_for(cx, cy) == ActiveRegionState::Awake
            && tick.saturating_sub(last_edit) >= CHUNK_SLEEP_IDLE_THRESHOLD_TICKS
        {
            transitions.push(ActiveRegionTransition {
                chunk_coords: (cx, cy),
                from_state: ActiveRegionState::Awake,
                to_state: ActiveRegionState::Sleeping,
                tick,
            });
            setter(cx, cy, ActiveRegionState::Sleeping);
        }
    }
    transitions
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn wake_chunk_wakes_neighbors() {
        use std::cell::RefCell;
        let states: RefCell<BTreeMap<(i32, i32), ActiveRegionState>> = RefCell::new(BTreeMap::new());
        let transitions = wake_chunk_and_neighbors(
            0,
            0,
            10,
            |x, y| states.borrow().get(&(x, y)).copied().unwrap_or_default(),
            |x, y, s| {
                states.borrow_mut().insert((x, y), s);
            },
        );
        assert_eq!(transitions.len(), 9);
        let states = states.borrow();
        for ((cx, cy), state) in states.iter() {
            assert!(*cx >= -1 && *cx <= 1 && *cy >= -1 && *cy <= 1);
            assert_eq!(*state, ActiveRegionState::Awake);
        }
    }

    #[test]
    fn sleep_after_threshold_idle() {
        use std::cell::RefCell;
        let states: RefCell<BTreeMap<(i32, i32), ActiveRegionState>> = RefCell::new(BTreeMap::new());
        states.borrow_mut().insert((0, 0), ActiveRegionState::Awake);
        let transitions = sleep_idle_chunks(
            CHUNK_SLEEP_IDLE_THRESHOLD_TICKS + 1,
            &[(0, 0)],
            |_, _| 0,
            |x, y| states.borrow().get(&(x, y)).copied().unwrap_or_default(),
            |x, y, s| {
                states.borrow_mut().insert((x, y), s);
            },
        );
        assert_eq!(transitions.len(), 1);
        assert_eq!(transitions[0].to_state, ActiveRegionState::Sleeping);
    }

    #[test]
    fn no_sleep_before_threshold() {
        use std::cell::RefCell;
        let states: RefCell<BTreeMap<(i32, i32), ActiveRegionState>> = RefCell::new(BTreeMap::new());
        states.borrow_mut().insert((0, 0), ActiveRegionState::Awake);
        let transitions = sleep_idle_chunks(
            CHUNK_SLEEP_IDLE_THRESHOLD_TICKS - 1,
            &[(0, 0)],
            |_, _| 0,
            |x, y| states.borrow().get(&(x, y)).copied().unwrap_or_default(),
            |x, y, s| {
                states.borrow_mut().insert((x, y), s);
            },
        );
        assert!(transitions.is_empty());
    }
}
