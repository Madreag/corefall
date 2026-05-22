//! M8B § ICE-lite candidate gathering + connectivity-check orchestration.
//!
//! Per M8B Notes: "ICE-lite (not full ICE) per IETF RFC 5245: the cf-net
//! client is the controlling agent, the cf-net server is the controlled
//! agent + always has a server-reflexive candidate."

use serde::{Deserialize, Serialize};

use crate::nat::candidate_pair::{
    run_pair_checks, Candidate, CandidateKind, CandidatePair, PairConnectivityCheck,
    PairConnectivityResult,
};
use crate::nat::{NatTraversalMethod, NatTraversalPath};

/// ICE-lite role per RFC 5245. The cf-net client is always Controlling;
/// the cf-net server is always Controlled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IceRole {
    Controlling,
    Controlled,
}

/// The peer's NAT behavior type. Drives the test scenario; production
/// learns it from the STUN exchange.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NatBehavior {
    /// No NAT — direct host connectivity works.
    None,
    /// Symmetric NAT — host pairs fail but server-reflexive succeeds.
    Symmetric,
    /// Symmetric NAT with port restriction — even server-reflexive
    /// fails (so the agent falls back to TURN relay).
    SymmetricPortRestricted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IceLiteOutcome {
    pub method: NatTraversalMethod,
    pub path: NatTraversalPath,
    pub elapsed_ms: u32,
    pub chosen_pair: Option<CandidatePair>,
}

pub struct IceLiteAgent {
    pub role: IceRole,
    pub local_host: Candidate,
    pub local_srflx: Option<Candidate>,
}

impl IceLiteAgent {
    pub fn new_controlling(local_host: Candidate) -> Self {
        Self {
            role: IceRole::Controlling,
            local_host,
            local_srflx: None,
        }
    }

    pub fn new_controlled(local_host: Candidate, local_srflx: Candidate) -> Self {
        Self {
            role: IceRole::Controlled,
            local_host,
            local_srflx: Some(local_srflx),
        }
    }

    pub fn set_server_reflexive(&mut self, srflx: Candidate) {
        self.local_srflx = Some(srflx);
    }

    /// symmetric-NAT pair"**: enumerate (local, remote) candidate pairs
    /// + run connectivity checks; return the chosen pair (or None when
    ///   every pair fails → caller falls back to TURN).
    pub fn enumerate_pairs(&self, remote_candidates: &[Candidate]) -> Vec<CandidatePair> {
        let mut pairs = Vec::new();
        let locals = std::iter::once(&self.local_host).chain(self.local_srflx.iter());
        for local in locals {
            for remote in remote_candidates {
                // Pairing is type-symmetric: host-host, srflx-srflx,
                // srflx-host, host-srflx all allowed.
                pairs.push(CandidatePair::new(local.clone(), remote.clone()));
            }
        }
        pairs
    }

    /// Drive the connectivity check using a behavior model (drives the
    /// per-pair check decision tree).
    pub fn run_connectivity_check(
        &self,
        remote_candidates: &[Candidate],
        peer_behavior: NatBehavior,
    ) -> IceLiteOutcome {
        let pairs = self.enumerate_pairs(remote_candidates);
        let outcome = run_pair_checks(pairs, |p| pair_check_under_behavior(p, peer_behavior));
        if let Some(chosen) = outcome.chosen.clone() {
            IceLiteOutcome {
                method: NatTraversalMethod::IceLite,
                path: NatTraversalPath::Direct,
                elapsed_ms: outcome.elapsed_ms,
                chosen_pair: Some(chosen),
            }
        } else {
            IceLiteOutcome {
                method: NatTraversalMethod::IceLite,
                path: NatTraversalPath::Direct,
                elapsed_ms: outcome.elapsed_ms,
                chosen_pair: None,
            }
        }
    }
}

/// Per-pair check model parameterized by the peer's NAT behavior. Real
/// networking is wired here at M9+; this function captures the logical
/// scenario tree used by M8B's acceptance tests.
fn pair_check_under_behavior(p: &CandidatePair, behavior: NatBehavior) -> PairConnectivityCheck {
    let elapsed_ms = 100u32;
    let result = match (p.local.kind, p.remote.kind, behavior) {
        (_, _, NatBehavior::None) => PairConnectivityResult::Succeeded,
        // Symmetric NAT: host-host pairs all fail because the inside-NAT
        // host candidates aren't routable from outside. ServerReflexive
        // pairs succeed because STUN gives a public-side mapping.
        (CandidateKind::Host, _, NatBehavior::Symmetric)
        | (_, CandidateKind::Host, NatBehavior::Symmetric) => PairConnectivityResult::Failed,
        (CandidateKind::ServerReflexive, CandidateKind::ServerReflexive, NatBehavior::Symmetric) => {
            PairConnectivityResult::Succeeded
        }
        // Symmetric + port restriction: every direct path fails.
        (_, _, NatBehavior::SymmetricPortRestricted) => PairConnectivityResult::Failed,
        _ => PairConnectivityResult::Failed,
    };
    PairConnectivityCheck {
        pair: p.clone(),
        result,
        elapsed_ms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn ice_lite_finds_srflx_pair_under_symmetric_nat() {
        let mut agent_a = IceLiteAgent::new_controlling(host("10.0.0.1", 5000));
        agent_a.set_server_reflexive(srflx("203.0.113.1", 49001));
        let remote = vec![host("10.0.0.2", 5000), srflx("203.0.113.2", 49002)];
        let outcome = agent_a.run_connectivity_check(&remote, NatBehavior::Symmetric);
        assert!(matches!(outcome.method, NatTraversalMethod::IceLite));
        assert!(matches!(outcome.path, NatTraversalPath::Direct));
        let chosen = outcome.chosen_pair.expect("srflx-srflx must succeed");
        assert_eq!(chosen.local.kind, CandidateKind::ServerReflexive);
        assert_eq!(chosen.remote.kind, CandidateKind::ServerReflexive);
    }

    #[test]
    fn ice_lite_fails_under_port_restricted_symmetric_nat() {
        let mut agent_a = IceLiteAgent::new_controlling(host("10.0.0.1", 5000));
        agent_a.set_server_reflexive(srflx("203.0.113.1", 49001));
        let remote = vec![srflx("203.0.113.2", 49002)];
        let outcome = agent_a.run_connectivity_check(&remote, NatBehavior::SymmetricPortRestricted);
        assert!(outcome.chosen_pair.is_none(), "ICE-lite must fail to trigger TURN fallback");
    }

    #[test]
    fn ice_lite_no_nat_picks_host_pair() {
        let mut agent_a = IceLiteAgent::new_controlling(host("10.0.0.1", 5000));
        agent_a.set_server_reflexive(srflx("203.0.113.1", 49001));
        let remote = vec![host("10.0.0.2", 5000)];
        let outcome = agent_a.run_connectivity_check(&remote, NatBehavior::None);
        let chosen = outcome.chosen_pair.unwrap();
        assert_eq!(chosen.local.kind, CandidateKind::Host);
    }
}
