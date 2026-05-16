//! M8A § Files / cf-net — NEW crate for network protocol + transport +
//! server + client + snapshot + rollback.
//!
//! cf-net is the wire-protocol crate. Trust-tier / ops / persistence /
//! anti-cheat live in the sibling cf-server* crates. cf-net's job:
//!
//! - **`protocol`** — locked wire frames v0.1; NetFrame + NetPayload
//!   tagged union (Handshake / InputCommand / SnapshotDelta /
//!   EventBatch / ChecksumProbe / Ping / Pong / Disconnect).
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
//!
//! M8A ships the protocol + transport contracts + server/client/snapshot
//! /rollback scaffolds; M9+ wires the live engine drive.

pub mod client;
pub mod protocol;
pub mod rollback;
pub mod server;
pub mod snapshot;
pub mod transport;

pub use protocol::{NetFrame, NetPayload, PROTOCOL_VERSION};
pub use server::ServerConfig;
pub use snapshot::SnapshotCadence;

#[derive(Debug, thiserror::Error)]
pub enum NetError {
    #[error("protocol version mismatch: server={server} client={client}")]
    ProtocolVersionMismatch { server: u16, client: u16 },
    #[error("frame too large: {0} bytes > 1450 max")]
    FrameTooLarge(usize),
    #[error("content hash mismatch on join")]
    ContentHashMismatch,
    #[error("transport error: {0}")]
    Transport(String),
    #[error("deserialize: {0}")]
    Deserialize(String),
}

pub type NetResult<T> = Result<T, NetError>;
