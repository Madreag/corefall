//! M8B § Integration test — in-process NAT emulator for ICE-lite +
//! STUN + TURN flow.
//!
//! Maps to spec § Acceptance:
//! - "ICE-lite + STUN punches a symmetric-NAT to symmetric-NAT pair"
//! - "TURN relay engages when ICE-lite fails"

use cf_net::nat::{
    candidate_pair::{Candidate, CandidateKind},
    ice_lite::{IceLiteAgent, NatBehavior},
    stun_client::{StunBindingResponse, StunClient, StunTransactionId},
    turn_relay::TurnRelayClient,
    NatTraversalMethod, NatTraversalPath,
};

fn host(addr: &str, port: u16) -> Candidate {
    Candidate {
        kind: CandidateKind::Host,
        address: addr.into(),
        port,
        priority: 0xFFFF_0000,
    }
}

fn srflx(addr: &str, port: u16) -> Candidate {
    Candidate {
        kind: CandidateKind::ServerReflexive,
        address: addr.into(),
        port,
        priority: 0x7FFF_0000,
    }
}

#[test]
fn ice_lite_punches_symmetric_nat_to_symmetric_nat_pair() {
    // Client A behind a symmetric NAT (10.0.0.1 → 203.0.113.1:49001).
    let stun = StunClient::new("stun.corefall.example", 3478);
    let resp_a = stun.discover(StunTransactionId([0u8; 12]), |tx, _, _| StunBindingResponse {
        transaction_id: tx,
        mapped_address: "203.0.113.1".into(),
        mapped_port: 49001,
    });
    // Client B behind a symmetric NAT (10.0.0.2 → 203.0.113.2:49002).
    let resp_b = stun.discover(StunTransactionId([1u8; 12]), |tx, _, _| StunBindingResponse {
        transaction_id: tx,
        mapped_address: "203.0.113.2".into(),
        mapped_port: 49002,
    });

    let mut agent_a = IceLiteAgent::new_controlling(host("10.0.0.1", 5000));
    agent_a.set_server_reflexive(srflx(&resp_a.mapped_address, resp_a.mapped_port));
    let remote_b = vec![host("10.0.0.2", 5000), srflx(&resp_b.mapped_address, resp_b.mapped_port)];
    let outcome = agent_a.run_connectivity_check(&remote_b, NatBehavior::Symmetric);
    assert!(matches!(outcome.method, NatTraversalMethod::IceLite));
    assert!(matches!(outcome.path, NatTraversalPath::Direct));
    let chosen = outcome.chosen_pair.expect("srflx-srflx pair succeeds");
    assert_eq!(chosen.local.kind, CandidateKind::ServerReflexive);
    assert_eq!(chosen.remote.kind, CandidateKind::ServerReflexive);
    // Total elapsed (per pair = 100ms in the model; up to 4 pairs attempted) ≤ 4000ms.
    assert!(outcome.elapsed_ms <= 4000);
}

#[test]
fn turn_relay_engages_when_ice_lite_fails() {
    let mut agent_a = IceLiteAgent::new_controlling(host("10.0.0.1", 5000));
    agent_a.set_server_reflexive(srflx("203.0.113.1", 49001));
    let remote_b = vec![host("10.0.0.2", 5000), srflx("203.0.113.2", 49002)];
    let ice_outcome = agent_a.run_connectivity_check(&remote_b, NatBehavior::SymmetricPortRestricted);
    assert!(ice_outcome.chosen_pair.is_none(), "ICE-lite must fail to trigger TURN");

    // Fall back to TURN.
    let turn = TurnRelayClient::new("turn.corefall.example", 3478, "anon", "corefall");
    let outcome = turn.allocate(|_| ("198.51.100.7".into(), 50000, 1500));
    assert!(matches!(outcome.method, NatTraversalMethod::TurnRelay));
    assert!(matches!(outcome.path, NatTraversalPath::Relay));
    assert!(outcome.relayed_address == "198.51.100.7");
    assert!(outcome.relayed_port == 50000);
}

#[test]
fn ice_lite_no_nat_picks_host_pair() {
    let mut agent_a = IceLiteAgent::new_controlling(host("10.0.0.1", 5000));
    agent_a.set_server_reflexive(srflx("203.0.113.1", 49001));
    let remote_b = vec![host("10.0.0.2", 5000), srflx("203.0.113.2", 49002)];
    let outcome = agent_a.run_connectivity_check(&remote_b, NatBehavior::None);
    let chosen = outcome.chosen_pair.unwrap();
    assert_eq!(chosen.local.kind, CandidateKind::Host);
}
