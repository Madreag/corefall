//! M8A § Files / cf-net — NEW crate for network protocol + transport +
//! server + client + snapshot + rollback. **M8B** locks the deep network
//! protocol layer: v0.1 byte-pinned wire layout, semver gate, rollback
//! prediction + resimulate, loss recovery (redundant input + Reed-Solomon
//! FEC), NAT punch-through (ICE-lite + STUN + TURN relay), and
//! deterministic transport-select policy.
//!
//! cf-net is the wire-protocol crate. Trust-tier / ops / persistence /
//! anti-cheat live in the sibling cf-server* crates. cf-net's job:
//!
//! - **`protocol`** — locked wire frames v0.1; NetFrame + NetPayload
//!   tagged union (Handshake / InputCommand / SnapshotDelta /
//!   EventBatch / ChecksumProbe / Ping / Pong / Disconnect /
//!   InputCommandRedundant / FecShard / NatTraversalOutcome /
//!   RollbackWindow).
//! - **`protocol::frame_v01`** — locked v0.1 byte-pinned layout +
//!   encode_frame / decode_frame.
//! - **`protocol::semver`** — major / minor / patch negotiation.
//! - **`transport`** — QUIC via `quinn` + WebSocket fallback for in-
//!   browser spectator clients (M11+ scope).
//! - **`server`** — authoritative loop; one canonical sim instance;
//!   delta-encoded snapshot per client; per-client bandwidth p99 <
//!   50 kB/s during heavy combat.
//! - **`client`** — input prediction + reconciliation; rollback on
//!   prediction mismatch.
//! - **`snapshot`** — delta-encoded world snapshots per spec § Snapshot
//!   field contract (Powder-Toy-derived).
//! - **`rollback`** — rollback netcode primitives; 6-frame budget at p99
//!   ≤ 8 ms total resimulation; reuses deterministic sim core.
//! - **`loss_recovery`** — redundant-input piggyback + Reed-Solomon FEC
//!   for small reliable payloads.
//! - **`nat`** — ICE-lite candidate gathering + STUN discovery + TURN
//!   relay fallback + parallel candidate-pair check with deterministic
//!   tiebreak.
//! - **`transport_select`** — deterministic per-session transport
//!   selection (`DedicatedServerAuth` / `HostAuthoritativeLockstep` /
//!   `P2pLockstep`).

#![allow(
    clippy::needless_range_loop,
    clippy::unnecessary_cast,
    clippy::bool_assert_comparison
)]

pub mod client;
pub mod loss_recovery;
pub mod nat;
pub mod protocol;
pub mod recovery_events;
pub mod rollback;
pub mod server;
pub mod snapshot;
pub mod stream_routing;
pub mod transport;
pub mod transport_select;

pub use protocol::semver::{Semver, PROTOCOL_SEMVER};
pub use protocol::{NetFrame, NetPayload, PROTOCOL_VERSION};
pub use server::ServerConfig;
pub use snapshot::SnapshotCadence;
pub use stream_routing::{
    binding_for, binding_for_payload, TransportBinding, RELIABLE_STREAM_CONTROL, RELIABLE_STREAM_EVENT_LOG,
    RELIABLE_STREAM_FEC,
};
pub use transport_select::{
    select_transport, ClientCapabilities, LanParticipantRole, ServerMode, TransportMode, TransportSelectInput,
};

#[derive(Debug, thiserror::Error)]
pub enum NetError {
    #[error("protocol version mismatch: server={server} client={client}")]
    ProtocolVersionMismatch { server: u16, client: u16 },
    #[error("frame too large: {0} bytes > 1450 max")]
    FrameTooLarge(usize),
    #[error("content hash mismatch on join")]
    ContentHashMismatch,
    /// session is closed with `NetError::Transport("tls handshake
    /// mismatch")` literal per spec. TLS-bound semver / application
    /// Handshake skew is the canonical trigger.
    #[error("transport error: {0}")]
    Transport(String),
    #[error("deserialize: {0}")]
    Deserialize(String),
}

pub type NetResult<T> = Result<T, NetError>;

/// canonical error string for a TLS-bound handshake mismatch (downgrade
/// attempt). Producers wrap this in [`NetError::Transport`] per spec
/// literal: `NetError::Transport("tls handshake mismatch")`.
pub const TLS_HANDSHAKE_MISMATCH_REASON: &str = "tls handshake mismatch";

/// Build the canonical [`NetError`] for a downgrade attack rejection.
pub fn tls_handshake_mismatch_error() -> NetError {
    NetError::Transport(TLS_HANDSHAKE_MISMATCH_REASON.to_string())
}

impl From<protocol::semver::DowngradeAttackError> for NetError {
    fn from(_: protocol::semver::DowngradeAttackError) -> Self {
        tls_handshake_mismatch_error()
    }
}
