//! M8B § Integration tests for loss recovery.
//!
//! Maps to spec § Acceptance:
//! - "Redundant-input encoding recovers from single-datagram loss"
//! - "Reed-Solomon FEC recovers a single-byte-corrupted reliable payload"
//! - "Lossy networks stay smooth" (5% packet loss on 100 ms RTT)

use cf_net::loss_recovery::fec::{decode_fec_group, encode_fec_group, FecShard};
use cf_net::loss_recovery::redundant_input::{RedundantInputLedger, RedundantInputTail};

#[test]
fn redundant_input_recovers_dropped_tick_700() {
    // **M8B § Acceptance**: client sends ticks at 60Hz with last-3 tail.
    // Tick 700 datagram is dropped; tick 701 carries 698/699/700 in tail.
    // Server's authoritative tick 700 must not stall.
    let mut ledger = RedundantInputLedger::new();
    // First, ingest 698 and 699 normally.
    let mut tail = RedundantInputTail::with_window(3);
    tail.push(696, vec![]);
    tail.push(697, vec![]);
    tail.push(698, vec![]);
    ledger.ingest(699, &[], &tail);

    // Tick 700 is dropped. Then tick 701 arrives with tail 698/699/700.
    let mut tail_701 = RedundantInputTail::with_window(3);
    tail_701.push(698, vec![]);
    tail_701.push(699, vec![]);
    tail_701.push(700, vec![]);
    let recovered = ledger.ingest(701, &[], &tail_701);

    assert!(ledger.ingested_ticks.contains(&700), "tick 700 recovered from tail");
    assert!(recovered.contains(&700), "tick 700 is newly recovered in this ingest call");
    assert!(ledger.was_recovered(700));
}

#[test]
fn five_percent_loss_on_60hz_does_not_stall_authoritative_stream() {
    // **M8B § Acceptance "Lossy networks stay smooth"**: 5% packet loss
    // on a 60Hz stream with last-3 redundant tail; the server's
    // authoritative tick stream completes without missing any tick.
    let mut ledger = RedundantInputLedger::new();
    for tick in 0u64..1100 {
        if tick % 20 == 19 {
            continue; // drop 5% of datagrams
        }
        let mut tail = RedundantInputTail::with_window(3);
        for back in 1..=3u64 {
            if tick >= back {
                tail.push(tick - back, vec![]);
            }
        }
        ledger.ingest(tick, &[], &tail);
    }
    // Within the recoverable window (the first 1000 ticks), no tick is missing.
    for tick in 0u64..1000 {
        assert!(
            ledger.ingested_ticks.contains(&tick),
            "tick {tick} un-ingested after 5% loss simulation"
        );
    }
}

#[test]
fn fec_recovers_single_lost_shard_k4_m2() {
    // **M8B § Acceptance "Reed-Solomon FEC recovers a single-byte-
    // corrupted reliable payload"**: k=4 + m=2. We model corruption as
    // erasure (the receiver knows which shard is bad). With 1 of 6
    // shards lost, the remaining 5 reconstruct the original payload.
    let payload = b"M8B FEC reliable event batch payload (small, under 8 kB)".to_vec();
    let group = encode_fec_group(&payload, 4, 2, 999).unwrap();
    let surviving: Vec<FecShard> = group
        .shards
        .iter()
        .filter(|s| s.shard_index != 2)
        .cloned()
        .collect();
    let recovered = decode_fec_group(&surviving, 4, 2, payload.len()).unwrap();
    assert_eq!(recovered, payload);
}

#[test]
fn fec_handles_random_payloads_round_trip() {
    // Stress: 50 random-ish payloads, each with a random shard erased.
    let mut state: u64 = 0xCAFE_BABE_DEAD_BEEFu64;
    for _ in 0..50 {
        let len = (state % 256) as usize + 16;
        let mut payload = Vec::with_capacity(len);
        for _ in 0..len {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            payload.push((state >> 33) as u8);
        }
        let group = encode_fec_group(&payload, 4, 2, 1).unwrap();
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let erase = (state % 6) as u8;
        let surviving: Vec<FecShard> = group.shards.iter().filter(|s| s.shard_index != erase).cloned().collect();
        let recovered = decode_fec_group(&surviving, 4, 2, payload.len()).unwrap();
        assert_eq!(recovered, payload, "erasure of shard {erase}");
    }
}
