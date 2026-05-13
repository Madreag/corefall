//! M2 — Objective grammar.
//!
//! Per the M2 spec's "## Files" section, `cf-mission/src/objective.rs` is the
//! canonical home for the `Objective` struct and `ObjectiveKind` enum
//! (ReachZone, KillActor / NeutralizeActor, SurviveTimer, DefendActor /
//! DefendReactor, EscortActor). The current code stores them in
//! `cf-mission/src/lib.rs` for historical reasons; this module re-exports
//! them so consumers that import from `cf_mission::objective::*` (per the
//! spec file path) compile cleanly.
//!
//! M2 ships ReachZone + SurviveTimer as the variants actively used by
//! `micro_breach`. The other variants exist for forward-compat with M2.5
//! (DefendReactor) and M13+ scenarios (NeutralizeActor / EscortActor /
//! BreachBarrier).

pub use crate::{FailSensor, Objective, ObjectiveKind, ObjectiveStatus};
