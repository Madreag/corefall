//! M8B § Loss recovery — redundant-input piggyback + Reed-Solomon FEC.
//!
//! Per M8B Notes for the implementer:
//!
//! - Redundant-input encoding piggybacks the last 3 inputs on each
//!   datagram (configurable `net.redundant_input.window_ticks`). The
//!   cost is +192 bytes/datagram at the default; the recovery benefit on
//!   a 5% loss link is enormous.
//! - Reed-Solomon FEC is used ONLY on small reliable payloads (event
//!   batches < 8 kB). Larger payloads rely on QUIC's own stream
//!   retransmit, which is more efficient.

pub mod fec;
pub mod redundant_input;

pub use fec::{decode_fec_group, encode_fec_group, FecError, FecGroup, FecShard};
pub use redundant_input::{RedundantInputEntry, RedundantInputTail, REDUNDANT_INPUT_DEFAULT_WINDOW};
