//! M8B § cf-server side of the NAT punch flow + protocol-semver gate
//! at join time.
//!
//! Per M8B spec § Crates / modules touched — `cf-server` MODIFY (from
//! M36): "Server side of the NAT punch flow + protocol semver gate at
//! join."
//!
//! The server runs as the "controlled" ICE-lite agent: it always has a
//! server-reflexive candidate (via STUN discovery on startup) plus a
//! TURN relay binding ready for fallback. On every join the server:
//!
//! 1. Verifies the client's advertised semver matches the server's
//!    major + minor (`negotiate` from `cf_net::protocol::semver`).
//! 2. If accepted, the server picks the per-session transport via
//!    `cf_net::select_transport` (deterministic).
//! 3. Performs ICE-lite candidate-pair connectivity check with the
//!    client over the lobby control channel.
//! 4. On ICE failure (4 sec timeout), engages TURN relay fallback.
//! 5. Emits `net.nat_traversal_outcome` to the run bundle + the lobby
//!    so the client surfaces "connecting via direct" vs "connecting
//!    via relay" in the status line.

use cf_net::nat::{
    candidate_pair::CandidatePair,
    ice_lite::{IceLiteAgent, NatBehavior},
    IceLiteOutcome, NatTraversalMethod, NatTraversalPath, TurnRelayClient, ICE_LITE_TIMEOUT_MS,
};
use cf_net::protocol::semver::{negotiate, NegotiationOutcome, Semver, PROTOCOL_SEMVER};
use cf_net::transport_select::{
    select_transport, ClientCapabilities, LanParticipantRole, ServerMode, TransportMode, TransportSelectInput,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JoinRequest {
    pub session_id: String,
    pub client_semver: Semver,
    pub client_features: Vec<String>,
    pub client_capabilities: ClientCapabilities,
    pub lan_role: Option<LanParticipantRole>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum JoinOutcome {
    Accepted {
        session_id: String,
        session_semver: Semver,
        granted_features: Vec<String>,
        transport_mode: TransportMode,
        traversal_method: NatTraversalMethod,
        traversal_path: NatTraversalPath,
        elapsed_ms: u32,
    },
    Rejected {
        session_id: String,
        reason: String,
        server_semver: Semver,
        client_semver: Semver,
        download_url: String,
    },
}

/// **M8B § Acceptance "Online match join time stays under 6 seconds even
/// behind double-NAT routers"**: server-side join flow.
///
/// The `peer_behavior` argument lets tests model the NAT scenarios
/// (None / Symmetric / SymmetricPortRestricted). Production wires a real
/// STUN exchange + drives this state machine.
pub struct NatPunchServer {
    pub server_mode: ServerMode,
    pub server_features: Vec<String>,
    pub turn_client: TurnRelayClient,
    pub server_host: cf_net::nat::candidate_pair::Candidate,
    pub server_srflx: cf_net::nat::candidate_pair::Candidate,
    pub download_url: String,
}

impl NatPunchServer {
    pub fn process_join(
        &self,
        request: &JoinRequest,
        peer_candidates: &[cf_net::nat::candidate_pair::Candidate],
        peer_behavior: NatBehavior,
    ) -> JoinOutcome {
        let server_features_ref: Vec<&str> = self.server_features.iter().map(String::as_str).collect();
        let client_features_ref: Vec<&str> = request.client_features.iter().map(String::as_str).collect();
        let negotiation = negotiate(
            PROTOCOL_SEMVER,
            &server_features_ref,
            request.client_semver,
            &client_features_ref,
            &self.download_url,
        );
        let (session_semver, granted_features) = match negotiation {
            NegotiationOutcome::RejectedMajorMismatch {
                server,
                client,
                download_url,
            } => {
                return JoinOutcome::Rejected {
                    session_id: request.session_id.clone(),
                    reason: "protocol_major_mismatch".into(),
                    server_semver: server,
                    client_semver: client,
                    download_url,
                };
            }
            NegotiationOutcome::Accepted {
                session_semver,
                granted_features,
            } => (session_semver, granted_features),
        };
        let transport_mode = select_transport(&TransportSelectInput {
            server_mode: self.server_mode,
            lan_role: request.lan_role,
            client_capabilities: request.client_capabilities.clone(),
        });
        // ICE-lite first.
        let controlled = IceLiteAgent::new_controlled(self.server_host.clone(), self.server_srflx.clone());
        let ice_outcome = controlled.run_connectivity_check(peer_candidates, peer_behavior);
        let (traversal_method, traversal_path, elapsed_ms) = if ice_outcome.chosen_pair.is_some() {
            (ice_outcome.method, ice_outcome.path, ice_outcome.elapsed_ms)
        } else {
            // TURN fallback.
            let outcome = self.turn_client.allocate(|_| ("relay.corefall.example".into(), 50000, 1500));
            (
                outcome.method,
                outcome.path,
                ICE_LITE_TIMEOUT_MS.saturating_add(outcome.elapsed_ms),
            )
        };
        JoinOutcome::Accepted {
            session_id: request.session_id.clone(),
            session_semver,
            granted_features,
            transport_mode,
            traversal_method,
            traversal_path,
            elapsed_ms,
        }
    }
}

/// Helper: build the "outcome event payload" string-form per spec §
/// Schemas → `net.nat_traversal_outcome`.
pub fn nat_traversal_event_payload(outcome: &JoinOutcome) -> Option<serde_json::Value> {
    if let JoinOutcome::Accepted {
        session_id,
        traversal_method,
        traversal_path,
        elapsed_ms,
        ..
    } = outcome
    {
        Some(serde_json::json!({
            "session_id": session_id,
            "method": traversal_method.as_str(),
            "path": traversal_path.as_str(),
            "elapsed_ms": *elapsed_ms,
        }))
    } else {
        None
    }
}

/// Returns the chosen candidate-pair description (`local.kind` →
/// `remote.kind`) for tracing.
pub fn describe_chosen_pair(outcome: &IceLiteOutcome) -> Option<String> {
    outcome.chosen_pair.as_ref().map(|p: &CandidatePair| {
        format!(
            "{:?}_{}_{}_{:?}_{}_{}",
            p.local.kind, p.local.address, p.local.port, p.remote.kind, p.remote.address, p.remote.port
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cf_net::nat::candidate_pair::{Candidate, CandidateKind};

    fn srv() -> NatPunchServer {
        NatPunchServer {
            server_mode: ServerMode::CoopRoom,
            server_features: vec!["fec".into(), "ice_lite".into()],
            turn_client: TurnRelayClient::new("turn.corefall.example", 3478, "anon", "corefall"),
            server_host: Candidate {
                kind: CandidateKind::Host,
                address: "10.0.0.1".into(),
                port: 4040,
                priority: 0xFFFF_0000,
            },
            server_srflx: Candidate {
                kind: CandidateKind::ServerReflexive,
                address: "203.0.113.99".into(),
                port: 49099,
                priority: 0x7FFF_0000,
            },
            download_url: "https://corefall.example/update".into(),
        }
    }

    fn cli_join(semver: Semver) -> JoinRequest {
        JoinRequest {
            session_id: "sess-1".into(),
            client_semver: semver,
            client_features: vec!["fec".into(), "ice_lite".into()],
            client_capabilities: ClientCapabilities::default(),
            lan_role: None,
        }
    }

    #[test]
    fn server_rejects_major_mismatched_client() {
        let s = srv();
        let request = cli_join(Semver::new(0, 2, 0));
        let peer = vec![];
        let outcome = s.process_join(&request, &peer, NatBehavior::None);
        match outcome {
            JoinOutcome::Rejected {
                reason,
                server_semver,
                client_semver,
                ..
            } => {
                assert_eq!(reason, "protocol_major_mismatch");
                assert_eq!(server_semver, PROTOCOL_SEMVER);
                assert_eq!(client_semver, Semver::new(0, 2, 0));
            }
            other => panic!("expected Rejected, got {other:?}"),
        }
    }

    #[test]
    fn server_accepts_matched_client_and_picks_direct_path() {
        let s = srv();
        let request = cli_join(PROTOCOL_SEMVER);
        let peer = vec![Candidate {
            kind: CandidateKind::Host,
            address: "10.0.0.2".into(),
            port: 4040,
            priority: 0xFFFF_0000,
        }];
        let outcome = s.process_join(&request, &peer, NatBehavior::None);
        match outcome {
            JoinOutcome::Accepted {
                traversal_method,
                traversal_path,
                transport_mode,
                ..
            } => {
                assert!(matches!(traversal_method, NatTraversalMethod::IceLite));
                assert!(matches!(traversal_path, NatTraversalPath::Direct));
                assert_eq!(transport_mode, TransportMode::DedicatedServerAuth);
            }
            other => panic!("expected Accepted, got {other:?}"),
        }
    }

    #[test]
    fn server_falls_back_to_turn_relay_when_ice_fails() {
        let s = srv();
        let request = cli_join(PROTOCOL_SEMVER);
        let peer = vec![Candidate {
            kind: CandidateKind::ServerReflexive,
            address: "203.0.113.2".into(),
            port: 49002,
            priority: 0x7FFF_0000,
        }];
        let outcome = s.process_join(&request, &peer, NatBehavior::SymmetricPortRestricted);
        match outcome {
            JoinOutcome::Accepted {
                traversal_method,
                traversal_path,
                ..
            } => {
                assert!(matches!(traversal_method, NatTraversalMethod::TurnRelay));
                assert!(matches!(traversal_path, NatTraversalPath::Relay));
            }
            other => panic!("expected Accepted, got {other:?}"),
        }
    }
}
