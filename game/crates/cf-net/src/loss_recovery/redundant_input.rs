//! M8B § Redundant-input encoding — piggyback the last N inputs on every
//! datagram. A dropped input datagram is recovered from the next
//! datagram's redundant tail; the server's authoritative tick is not
//! stalled and no rollback is triggered.
//!
//! Wire shape (encoded via the bincode 2.x fixed-int + little-endian
//! config in `protocol::frame_v01`):
//!
//! - `window_ticks: u8` — the number of redundant entries that follow.
//!   Default `REDUNDANT_INPUT_DEFAULT_WINDOW` = 3 per the M8B Notes.
//! - `entries: Vec<RedundantInputEntry>` — bincode-encoded as u64
//!   length prefix + per-entry serialization (u64 tick + Vec<u8>
//!   intent_bytes).
//!
//! The server maintains a per-client `tick -> ingested` BTreeSet. On
//! each incoming datagram (head + redundant tail), every entry whose
//! tick has not yet been ingested triggers `net.input_resent_redundant`
//! into the recorder.

use serde::{Deserialize, Serialize};

pub const REDUNDANT_INPUT_DEFAULT_WINDOW: u8 = 3;

/// One entry in the redundant-input tail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedundantInputEntry {
    pub tick: u64,
    pub intent_bytes: Vec<u8>,
}

/// The redundant tail attached to every InputCommandRedundant datagram.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedundantInputTail {
    pub window_ticks: u8,
    pub entries: Vec<RedundantInputEntry>,
}

impl RedundantInputTail {
    pub fn new() -> Self {
        Self {
            window_ticks: REDUNDANT_INPUT_DEFAULT_WINDOW,
            entries: Vec::with_capacity(REDUNDANT_INPUT_DEFAULT_WINDOW as usize),
        }
    }

    pub fn with_window(window_ticks: u8) -> Self {
        Self {
            window_ticks,
            entries: Vec::with_capacity(window_ticks as usize),
        }
    }

    /// Append an input to the tail. Trims oldest entries to maintain
    /// `window_ticks` length.
    pub fn push(&mut self, tick: u64, intent_bytes: Vec<u8>) {
        self.entries.push(RedundantInputEntry { tick, intent_bytes });
        while self.entries.len() > self.window_ticks as usize {
            self.entries.remove(0);
        }
    }
}

impl Default for RedundantInputTail {
    fn default() -> Self {
        Self::new()
    }
}

/// ticks the server has already ingested + which ticks would otherwise
/// have stalled but are recovered from the redundant tail.
#[derive(Debug, Clone, Default)]
pub struct RedundantInputLedger {
    pub ingested_ticks: std::collections::BTreeSet<u64>,
    pub recovered_ticks: std::collections::BTreeSet<u64>,
}

impl RedundantInputLedger {
    pub fn new() -> Self {
        Self::default()
    }

    /// Ingest the head + tail of a datagram. Returns the list of ticks
    /// that were recovered from the redundant tail (i.e., ticks not
    /// previously ingested). Each recovered tick triggers a
    /// `net.input_resent_redundant` recorder event in the caller.
    pub fn ingest(
        &mut self,
        head_tick: u64,
        _head_intent_bytes: &[u8],
        tail: &RedundantInputTail,
    ) -> Vec<u64> {
        let mut newly_recovered = Vec::new();
        self.ingested_ticks.insert(head_tick);
        for entry in &tail.entries {
            if self.ingested_ticks.insert(entry.tick) {
                self.recovered_ticks.insert(entry.tick);
                newly_recovered.push(entry.tick);
            }
        }
        newly_recovered
    }

    /// datagram loss"**: returns true if the supplied tick was missed
    /// in the head but recovered from a tail.
    pub fn was_recovered(&self, tick: u64) -> bool {
        self.recovered_ticks.contains(&tick)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RedundantInputError {
    #[error("redundant input decode: {0}")]
    Decode(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tail_trims_to_window_ticks() {
        let mut tail = RedundantInputTail::with_window(3);
        tail.push(100, vec![0x01]);
        tail.push(101, vec![0x02]);
        tail.push(102, vec![0x03]);
        tail.push(103, vec![0x04]);
        assert_eq!(tail.entries.len(), 3);
        assert_eq!(tail.entries[0].tick, 101);
        assert_eq!(tail.entries[2].tick, 103);
    }

    #[test]
    fn ledger_recovers_dropped_head_from_tail() {
        // Simulate: client sent ticks 700 + 701; tick 700 datagram was dropped.
        // The server receives tick 701's datagram which carries 700 + 701 + 702 (tail size 3).
        // Tick 700 must be recovered from the tail.
        let mut ledger = RedundantInputLedger::new();
        let mut tail = RedundantInputTail::with_window(3);
        tail.push(698, vec![]);
        tail.push(699, vec![]);
        tail.push(700, vec![]);
        let recovered = ledger.ingest(701, &[], &tail);
        // Head 701 ingested; tail 698, 699, 700 all recovered (newly seen).
        assert!(ledger.ingested_ticks.contains(&701));
        assert!(ledger.ingested_ticks.contains(&700));
        assert!(ledger.was_recovered(700));
        assert!(ledger.was_recovered(699));
        assert!(ledger.was_recovered(698));
        assert_eq!(recovered.len(), 3);
    }

    #[test]
    fn ledger_does_not_double_ingest() {
        let mut ledger = RedundantInputLedger::new();
        let mut tail = RedundantInputTail::with_window(2);
        tail.push(100, vec![]);
        let _ = ledger.ingest(101, &[], &tail);
        // Re-ingesting: 100 + 101 already in; new tail tick 99 is a new recovery.
        let mut tail2 = RedundantInputTail::with_window(2);
        tail2.push(99, vec![]);
        tail2.push(100, vec![]);
        let recovered = ledger.ingest(101, &[], &tail2);
        // Only 99 is newly recovered; 100 + 101 already ingested.
        assert_eq!(recovered, vec![99]);
    }

    #[test]
    fn five_percent_loss_recovery_at_60hz() {
        // stream of 1100 input frames; drop every 20th packet (5% loss).
        // Each surviving packet carries a 3-tick redundant tail. The server
        // must end up with every tick in the first 1000 ingested (later
        // ticks may be on the trailing edge of the stream + dropped without
        // a following datagram to recover from). No rollback should be
        // triggered for the recovered window.
        let mut ledger = RedundantInputLedger::new();
        let mut dropped: u32 = 0;
        for tick in 0u64..1100 {
            if tick % 20 == 19 {
                dropped += 1;
                continue;
            }
            let mut tail = RedundantInputTail::with_window(3);
            for back in 1..=3u64 {
                if tick >= back {
                    tail.push(tick - back, vec![]);
                }
            }
            ledger.ingest(tick, &[], &tail);
        }
        let mut missing = Vec::new();
        for tick in 0u64..1000 {
            if !ledger.ingested_ticks.contains(&tick) {
                missing.push(tick);
            }
        }
        assert!(
            missing.is_empty(),
            "5% loss simulation left {} ticks un-ingested in [0..1000): {:?} (dropped={})",
            missing.len(),
            missing,
            dropped
        );
        assert!(dropped > 0, "test must drop some datagrams to be meaningful");
    }
}
