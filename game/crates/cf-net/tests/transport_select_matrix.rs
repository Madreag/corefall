//! M8B § Integration test — transport-select matrix.
//!
//! Maps to spec § Acceptance:
//! - "Transport-select picks dedicated-server-authority for cf-server connections"
//! - "Transport-select picks host-authoritative lockstep for cf-server lan_room"
//!
//! And § Notes: "The transport-select policy is deterministic: same
//! server mode + same client capabilities → same transport choice."

use cf_net::{
    select_transport, ClientCapabilities, LanParticipantRole, ServerMode, TransportMode,
    TransportSelectInput,
};

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
fn lan_room_picks_host_authoritative_lockstep_for_both_roles() {
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
fn full_matrix_is_deterministic_and_complete() {
    let modes = [
        ServerMode::CoopRoom,
        ServerMode::PvpArena,
        ServerMode::LanRoom,
        ServerMode::MmoShard,
        ServerMode::LobbyDirectory,
        ServerMode::P2pSession,
    ];
    let caps = [
        ClientCapabilities { hosts_p2p_capable: false },
        ClientCapabilities { hosts_p2p_capable: true },
    ];
    for sm in &modes {
        for c in &caps {
            let input = TransportSelectInput {
                server_mode: *sm,
                lan_role: None,
                client_capabilities: c.clone(),
            };
            let r1 = select_transport(&input);
            let r2 = select_transport(&input);
            assert_eq!(r1, r2, "deterministic on ({sm:?}, {c:?})");
        }
    }
}

#[test]
fn p2p_session_with_capable_client_picks_p2p_lockstep() {
    let m = select_transport(&TransportSelectInput {
        server_mode: ServerMode::P2pSession,
        lan_role: None,
        client_capabilities: ClientCapabilities { hosts_p2p_capable: true },
    });
    assert_eq!(m, TransportMode::P2pLockstep);
}

#[test]
fn p2p_session_with_incapable_client_falls_back_to_dedicated() {
    let m = select_transport(&TransportSelectInput {
        server_mode: ServerMode::P2pSession,
        lan_role: None,
        client_capabilities: ClientCapabilities { hosts_p2p_capable: false },
    });
    assert_eq!(m, TransportMode::DedicatedServerAuth);
}
