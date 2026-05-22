//! M8B § Rollback ring buffer — 6-frame fixed-size store of per-tick
//! inputs + per-tick world-state hashes. The driver uses this to detect
//! the first-divergent-frame when the server's authoritative input set
//! differs from the client's prediction.

use serde::{Deserialize, Serialize};

/// 6-frame budget.
pub const ROLLBACK_WINDOW_FRAMES: usize = 6;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputFrame {
    pub tick: u64,
    pub intent_bytes: Vec<u8>,
    /// BLAKE3-32 of the post-tick world state. Used by the driver to
    /// detect a misprediction at minimal cost.
    pub world_state_hash: [u8; 32],
}

impl InputFrame {
    pub fn new(tick: u64, intent_bytes: Vec<u8>, world_state_hash: [u8; 32]) -> Self {
        Self {
            tick,
            intent_bytes,
            world_state_hash,
        }
    }
}

/// Fixed-size ring buffer of `ROLLBACK_WINDOW_FRAMES` entries.
/// Deterministic + canonical (BTreeMap-ish behavior — newer ticks
/// overwrite older ones but ordering is preserved).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackRingBuffer {
    entries: Vec<InputFrame>,
}

impl Default for RollbackRingBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl RollbackRingBuffer {
    pub fn new() -> Self {
        Self {
            entries: Vec::with_capacity(ROLLBACK_WINDOW_FRAMES),
        }
    }

    pub fn push(&mut self, frame: InputFrame) {
        self.entries.push(frame);
        while self.entries.len() > ROLLBACK_WINDOW_FRAMES {
            self.entries.remove(0);
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &InputFrame> {
        self.entries.iter()
    }

    pub fn get_by_tick(&self, tick: u64) -> Option<&InputFrame> {
        self.entries.iter().find(|f| f.tick == tick)
    }

    /// Find the first tick at which the buffer's recorded
    /// `world_state_hash` diverges from the supplied authoritative
    /// hash set. Returns `None` if all frames match.
    pub fn first_divergent_tick(&self, authoritative: &[(u64, [u8; 32])]) -> Option<u64> {
        for (tick, hash) in authoritative {
            if let Some(frame) = self.get_by_tick(*tick) {
                if frame.world_state_hash != *hash {
                    return Some(*tick);
                }
            }
        }
        None
    }

    /// Commit-order: returns the frames in chronological (tick-ascending)
    /// order. Caller MUST use this for resimulation ordering.
    pub fn commit_order(&self) -> Vec<&InputFrame> {
        let mut sorted: Vec<&InputFrame> = self.entries.iter().collect();
        sorted.sort_by_key(|f| f.tick);
        sorted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locked_at_six_frames() {
        assert_eq!(ROLLBACK_WINDOW_FRAMES, 6);
    }

    #[test]
    fn buffer_trims_to_window_size() {
        let mut rb = RollbackRingBuffer::new();
        for tick in 0u64..10 {
            rb.push(InputFrame::new(tick, vec![tick as u8], [0u8; 32]));
        }
        assert_eq!(rb.len(), 6);
        assert_eq!(rb.commit_order().first().unwrap().tick, 4);
        assert_eq!(rb.commit_order().last().unwrap().tick, 9);
    }

    #[test]
    fn first_divergent_tick_finds_mismatch() {
        let mut rb = RollbackRingBuffer::new();
        for tick in 614..=620u64 {
            let mut h = [0u8; 32];
            h[0] = tick as u8;
            rb.push(InputFrame::new(tick, vec![], h));
        }
        // Authoritative: tick 614..618 match, tick 619 differs.
        let mut auth = Vec::new();
        for tick in 614..=619u64 {
            let mut h = [0u8; 32];
            h[0] = tick as u8;
            if tick == 619 {
                h[0] = 0xFF;
            }
            auth.push((tick, h));
        }
        assert_eq!(rb.first_divergent_tick(&auth), Some(619));
    }

    #[test]
    fn no_divergence_returns_none() {
        let mut rb = RollbackRingBuffer::new();
        for tick in 100..=105u64 {
            let mut h = [0u8; 32];
            h[0] = tick as u8;
            rb.push(InputFrame::new(tick, vec![], h));
        }
        let mut auth = Vec::new();
        for tick in 100..=105u64 {
            let mut h = [0u8; 32];
            h[0] = tick as u8;
            auth.push((tick, h));
        }
        assert!(rb.first_divergent_tick(&auth).is_none());
    }
}
