//! M2 — Reactive guard state machine.
//!
//! Per the M2 spec's "## Files" section, `cf-ai/src/guard_state.rs` is the
//! canonical home for the `GuardState` enum + transition function with
//! reason labels (`Idle → Alert → Engaged → Retreating → Dying → Dead`).
//! The current implementation lives in `cf-ai/src/lib.rs`; this module
//! re-exports the public surface so consumers that import per the spec path
//! `cf_ai::guard_state::*` resolve cleanly.

pub use crate::{GuardState, GuardStateTransition};
