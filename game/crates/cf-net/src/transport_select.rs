//! M8B § Transport-select — picks per-session transport mode.
//!
//! Per M8B spec § Notes: "The transport-select policy is deterministic:
//! same server mode + same client capabilities → same transport choice.
//! CI gate runs the full matrix."
//!
//! Decision matrix:
//!
//! | Server mode         | Client caps          | Transport                      |
//! |---------------------|----------------------|--------------------------------|
//! | coop_room           | any                  | DedicatedServerAuth            |
//! | pvp_arena           | any                  | DedicatedServerAuth            |
//! | mmo_shard           | any                  | DedicatedServerAuth            |
//! | lobby_directory     | any                  | DedicatedServerAuth (control)  |
//! | lan_room (host)     | n/a                  | HostAuthoritativeLockstep      |
//! | lan_room (guest)    | any                  | HostAuthoritativeLockstep      |
//! | p2p_session         | hosts_p2p_capable    | P2pLockstep                    |

use serde::{Deserialize, Serialize};

/// Server mode advertised by the lobby / handshake. Aligned with M36's
/// `cf-server --mode <X>` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServerMode {
    CoopRoom,
    PvpArena,
    LanRoom,
    MmoShard,
    LobbyDirectory,
    /// Dedicated-server-less P2P session (M40+).
    P2pSession,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LanParticipantRole {
    Host,
    Guest,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ClientCapabilities {
    /// True when the client advertises P2P-capable transport (i.e., it can
    /// open inbound UDP from the peer via ICE-lite).
    pub hosts_p2p_capable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransportSelectInput {
    pub server_mode: ServerMode,
    pub lan_role: Option<LanParticipantRole>,
    pub client_capabilities: ClientCapabilities,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportMode {
    /// Dedicated server is the canonical authority. Client uploads inputs
    /// + receives delta-encoded snapshots.
    DedicatedServerAuth,
    /// LAN host-authoritative lockstep. Host merges all guests' inputs +
    /// broadcasts per tick; every node sim-steps on the same merged
    /// input set.
    HostAuthoritativeLockstep,
    /// Pure peer-to-peer lockstep (M40+). Each peer simulates the same
    /// merged inputs.
    P2pLockstep,
}

impl TransportMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DedicatedServerAuth => "dedicated_server_auth",
            Self::HostAuthoritativeLockstep => "host_authoritative_lockstep",
            Self::P2pLockstep => "p2p_lockstep",
        }
    }
}

/// deterministic decision function.
pub fn select_transport(input: &TransportSelectInput) -> TransportMode {
    match input.server_mode {
        ServerMode::CoopRoom => TransportMode::DedicatedServerAuth,
        ServerMode::PvpArena => TransportMode::DedicatedServerAuth,
        ServerMode::MmoShard => TransportMode::DedicatedServerAuth,
        ServerMode::LobbyDirectory => TransportMode::DedicatedServerAuth,
        ServerMode::LanRoom => TransportMode::HostAuthoritativeLockstep,
        ServerMode::P2pSession => {
            if input.client_capabilities.hosts_p2p_capable {
                TransportMode::P2pLockstep
            } else {
                TransportMode::DedicatedServerAuth
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coop_room_picks_dedicated_server_auth() {
        let m = select_transport(&TransportSelectInput {
            server_mode: ServerMode::CoopRoom,
            lan_role: None,
            client_capabilities: ClientCapabilities::default(),
        });
        assert_eq!(m, TransportMode::DedicatedServerAuth);
    }

    #[test]
    fn lan_room_picks_host_authoritative_lockstep_for_guest_and_host() {
        for role in [LanParticipantRole::Host, LanParticipantRole::Guest] {
            let m = select_transport(&TransportSelectInput {
                server_mode: ServerMode::LanRoom,
                lan_role: Some(role),
                client_capabilities: ClientCapabilities::default(),
            });
            assert_eq!(m, TransportMode::HostAuthoritativeLockstep);
        }
    }

    #[test]
    fn pvp_arena_picks_dedicated_server_auth() {
        let m = select_transport(&TransportSelectInput {
            server_mode: ServerMode::PvpArena,
            lan_role: None,
            client_capabilities: ClientCapabilities::default(),
        });
        assert_eq!(m, TransportMode::DedicatedServerAuth);
    }

    #[test]
    fn p2p_session_with_capable_client_picks_p2p_lockstep() {
        let m = select_transport(&TransportSelectInput {
            server_mode: ServerMode::P2pSession,
            lan_role: None,
            client_capabilities: ClientCapabilities {
                hosts_p2p_capable: true,
            },
        });
        assert_eq!(m, TransportMode::P2pLockstep);
    }

    #[test]
    fn p2p_session_without_capable_client_falls_back_to_dedicated() {
        let m = select_transport(&TransportSelectInput {
            server_mode: ServerMode::P2pSession,
            lan_role: None,
            client_capabilities: ClientCapabilities {
                hosts_p2p_capable: false,
            },
        });
        assert_eq!(m, TransportMode::DedicatedServerAuth);
    }

    /// deterministic per (server_mode, client_caps).
    #[test]
    fn full_matrix_is_deterministic() {
        for sm in [
            ServerMode::CoopRoom,
            ServerMode::PvpArena,
            ServerMode::LanRoom,
            ServerMode::MmoShard,
            ServerMode::LobbyDirectory,
            ServerMode::P2pSession,
        ] {
            for caps in [
                ClientCapabilities {
                    hosts_p2p_capable: false,
                },
                ClientCapabilities {
                    hosts_p2p_capable: true,
                },
            ] {
                let input = TransportSelectInput {
                    server_mode: sm,
                    lan_role: None,
                    client_capabilities: caps.clone(),
                };
                let r1 = select_transport(&input);
                let r2 = select_transport(&input);
                assert_eq!(r1, r2, "deterministic on {sm:?} + {caps:?}");
            }
        }
    }
}
