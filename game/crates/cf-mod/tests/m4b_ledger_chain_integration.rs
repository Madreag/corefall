//! **M4B § "Ledger chain rejects tampered bundle"** / "Ledger chain passes
//! for a clean tournament bundle" — integration tests for the
//! `cf-mod ledger verify --bundle <path>` surface.
//!
//! Tests build a tiny synthetic bundle (manifest + events.jsonl) and
//! exercise the verifier via the `cf-save::ledger_chain` API directly
//! (the CLI dispatcher just wraps this API). The CLI smoke covers the
//! exit-code contract in the migration-matrix script.

use cf_save::ledger_chain::{new_encoder, verify_chain, ChainedEvent, VerifyOutcome};

fn build_clean_chain(run_id: &str, seed: u64, n: usize) -> Vec<ChainedEvent> {
    let mut enc = new_encoder(run_id, seed);
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let payload = serde_json::to_string(&serde_json::json!({"i": i, "msg": "ok"})).unwrap();
        out.push(enc.append(&format!("ev_{i}"), &payload));
    }
    out
}

#[test]
fn verify_chain_clean_returns_anchor_matching_encoder() {
    let mut events = build_clean_chain("run_abc", 42, 100);
    let outcome = verify_chain("run_abc", 42, &events);
    let first_anchor = match &outcome {
        VerifyOutcome::Clean { events_verified, anchor } => {
            assert_eq!(*events_verified, 100);
            assert_eq!(anchor, &events.last().unwrap().chained_hash_hex);
            anchor.clone()
        }
        other => panic!("expected clean, got {other:?}"),
    };
    let outcome2 = verify_chain("run_abc", 42, &events);
    match &outcome2 {
        VerifyOutcome::Clean { anchor, .. } => assert_eq!(&first_anchor, anchor),
        other => panic!("clean verify must be idempotent, got {other:?}"),
    }
    // Mutate one event payload mid-chain; verify reports tamper.
    let mut tampered = events.clone();
    tampered[50].payload_canonical_json = serde_json::to_string(&serde_json::json!({"oops": true})).unwrap();
    let outcome3 = verify_chain("run_abc", 42, &tampered);
    match outcome3 {
        VerifyOutcome::Tampered { first_break } => {
            assert_eq!(first_break.event_id, "ev_50");
        }
        other => panic!("expected tampered, got {other:?}"),
    }
    // The unmodified `events` still verifies clean.
    drop(events.pop());
    let outcome4 = verify_chain("run_abc", 42, &events);
    assert!(matches!(outcome4, VerifyOutcome::Clean { .. }));
}

#[test]
fn verify_chain_rejects_seed_substitution() {
    let events = build_clean_chain("run_abc", 42, 10);
    let outcome = verify_chain("run_abc", 99, &events);
    assert!(matches!(outcome, VerifyOutcome::Tampered { .. }));
}

#[test]
fn verify_chain_rejects_run_id_substitution() {
    let events = build_clean_chain("run_abc", 42, 10);
    let outcome = verify_chain("run_xyz", 42, &events);
    assert!(matches!(outcome, VerifyOutcome::Tampered { .. }));
}
