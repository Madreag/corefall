//! M8B § Candidate-pair connectivity check.
//!
//! Per M8B Notes: "Parallel candidate-pair connectivity check with
//! deterministic tiebreak." The agent gathers candidates locally + via
//! STUN; the peer does the same; both sides enumerate pairs and check
//! each one. The first pair that completes the check wins; deterministic
//! tiebreak resolves ties on the same priority (so the gate is byte-
//! identical across re-runs).

use serde::{Deserialize, Serialize};

/// A candidate address + transport priority. Priority follows the
/// RFC 5245 ranking: host > server-reflexive > relayed.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Candidate {
    pub kind: CandidateKind,
    pub address: String,
    pub port: u16,
    pub priority: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CandidateKind {
    Host,
    ServerReflexive,
    Relayed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidatePair {
    pub local: Candidate,
    pub remote: Candidate,
    pub pair_priority: u64,
}

impl CandidatePair {
    pub fn new(local: Candidate, remote: Candidate) -> Self {
        // RFC 5245 § 5.7.2 — pair priority. We use a simplified form
        // suitable for ICE-lite + a deterministic tiebreak by address
        // sort so byte-identical re-runs produce the same chosen pair.
        let g = local.priority.min(remote.priority) as u64;
        let d = local.priority.max(remote.priority) as u64;
        let pair_priority = (g << 32)
            | (d << 1)
            | (if local.priority > remote.priority { 1 } else { 0 });
        Self {
            local,
            remote,
            pair_priority,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PairConnectivityResult {
    Succeeded,
    Failed,
    TimedOut,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairConnectivityCheck {
    pub pair: CandidatePair,
    pub result: PairConnectivityResult,
    pub elapsed_ms: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidatePairOutcome {
    /// The first pair that completed the check with `Succeeded`, ordered
    /// by `pair_priority` descending + deterministic address tiebreak.
    /// `None` when every pair failed or timed out.
    pub chosen: Option<CandidatePair>,
    pub elapsed_ms: u32,
    pub attempts: usize,
}

/// run all pairs "in parallel" and pick the first one that succeeds.
/// In this model the checker is a callback so tests can simulate
/// success / failure / timeout per pair.
pub fn run_pair_checks<F>(pairs: Vec<CandidatePair>, mut check_one: F) -> CandidatePairOutcome
where
    F: FnMut(&CandidatePair) -> PairConnectivityCheck,
{
    let mut sorted = pairs;
    // Sort by pair_priority desc; then by (local.address, local.port,
    // remote.address, remote.port) ascending for a deterministic
    // tiebreak.
    sorted.sort_by(|a, b| {
        b.pair_priority.cmp(&a.pair_priority).then_with(|| {
            (
                &a.local.address,
                a.local.port,
                &a.remote.address,
                a.remote.port,
            )
                .cmp(&(
                    &b.local.address,
                    b.local.port,
                    &b.remote.address,
                    b.remote.port,
                ))
        })
    });
    let mut total_elapsed_ms: u32 = 0;
    let mut attempts: usize = 0;
    for pair in &sorted {
        let check = check_one(pair);
        attempts += 1;
        total_elapsed_ms = total_elapsed_ms.saturating_add(check.elapsed_ms);
        if matches!(check.result, PairConnectivityResult::Succeeded) {
            return CandidatePairOutcome {
                chosen: Some(pair.clone()),
                elapsed_ms: total_elapsed_ms,
                attempts,
            };
        }
    }
    CandidatePairOutcome {
        chosen: None,
        elapsed_ms: total_elapsed_ms,
        attempts,
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
    fn host_pair_beats_reflexive_pair_on_priority() {
        let pairs = vec![
            CandidatePair::new(host("10.0.0.1", 1000), host("10.0.0.2", 1000)),
            CandidatePair::new(srflx("203.0.113.1", 5000), srflx("203.0.113.2", 5000)),
        ];
        // Both succeed; the highest-priority (host-host) pair wins.
        let outcome = run_pair_checks(pairs, |_| PairConnectivityCheck {
            pair: CandidatePair::new(host("0", 0), host("0", 0)),
            result: PairConnectivityResult::Succeeded,
            elapsed_ms: 50,
        });
        let chosen = outcome.chosen.expect("at least one pair succeeded");
        assert_eq!(chosen.local.kind, CandidateKind::Host);
        assert_eq!(chosen.remote.kind, CandidateKind::Host);
    }

    #[test]
    fn srflx_pair_wins_when_host_pair_fails() {
        // Simulates the M8B "symmetric NAT" scenario: host pairs all fail,
        // server-reflexive pair succeeds within budget.
        let host_pair = CandidatePair::new(host("10.0.0.1", 1000), host("10.0.0.2", 1000));
        let srflx_pair = CandidatePair::new(srflx("203.0.113.1", 5000), srflx("203.0.113.2", 5000));
        let pairs = vec![host_pair.clone(), srflx_pair.clone()];
        let outcome = run_pair_checks(pairs, |p| {
            let result = if p.local.kind == CandidateKind::Host {
                PairConnectivityResult::Failed
            } else {
                PairConnectivityResult::Succeeded
            };
            PairConnectivityCheck {
                pair: p.clone(),
                result,
                elapsed_ms: 100,
            }
        });
        let chosen = outcome.chosen.expect("srflx succeeded");
        assert_eq!(chosen.local.kind, CandidateKind::ServerReflexive);
        assert_eq!(outcome.attempts, 2);
    }

    #[test]
    fn all_failed_returns_none() {
        let pairs = vec![CandidatePair::new(host("10.0.0.1", 1000), host("10.0.0.2", 1000))];
        let outcome = run_pair_checks(pairs, |p| PairConnectivityCheck {
            pair: p.clone(),
            result: PairConnectivityResult::Failed,
            elapsed_ms: 50,
        });
        assert!(outcome.chosen.is_none());
    }

    #[test]
    fn tiebreak_is_deterministic_on_equal_priority() {
        let a = host("10.0.0.1", 1000);
        let b = host("10.0.0.2", 1000);
        let c = host("10.0.0.3", 1000);
        let p1 = CandidatePair::new(a.clone(), b.clone());
        let p2 = CandidatePair::new(a.clone(), c.clone());
        // Both pairs have identical priority (same kind = Host on both
        // sides). Tiebreak by (local.address, local.port, remote.address,
        // remote.port) ascending = p1 (remote=10.0.0.2) wins over p2.
        let outcome1 = run_pair_checks(vec![p1.clone(), p2.clone()], |p| {
            PairConnectivityCheck {
                pair: p.clone(),
                result: PairConnectivityResult::Succeeded,
                elapsed_ms: 50,
            }
        });
        let outcome2 = run_pair_checks(vec![p2.clone(), p1.clone()], |p| {
            PairConnectivityCheck {
                pair: p.clone(),
                result: PairConnectivityResult::Succeeded,
                elapsed_ms: 50,
            }
        });
        assert_eq!(outcome1.chosen, outcome2.chosen);
        assert_eq!(outcome1.chosen.unwrap().remote.address, "10.0.0.2");
    }
}
