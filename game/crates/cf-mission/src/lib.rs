//! M2: minimal objective state machine.
//!
//! (Historical name: M1.5 — renamed to M2 in the canonical roadmap. Code
//! comments may still say "M1.5"; they refer to the same milestone whose
//! spec lives at `specs/done/M2.md`.)
//!
//! This crate owns the `MissionState` + `Objective` types used by the M2 micro
//! breach scenario. The full mission director (typed manifest, command-core, base
//! power, commander AI, comic-noir cards) lands at M7. M2 only needs:
//!
//! - A small ordered list of objectives the player must clear in turn (or fail).
//! - A win/loss decision per tick (player dead, timer expired, or all required
//!   objectives complete).
//! - A `MissionTickReport` the engine turns into `mission.*` recorder events with
//!   the same field shape M7's full director will emit.
//!
//! The crate is deterministic and pure: every public mutator is a `&mut self` step
//! that consumes structured inputs and returns a report, so replay parity is the
//! caller's wiring contract — not this crate's.

#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(
    clippy::module_name_repetitions,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::doc_markdown,
    clippy::missing_const_for_fn,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_lossless,
    clippy::float_cmp,
    clippy::redundant_closure,
    clippy::derivable_impls,
    clippy::wildcard_in_or_patterns,
    clippy::needless_pass_by_value,
    clippy::ref_option,
    clippy::match_wildcard_for_single_variants,
    clippy::trivially_copy_pass_by_ref,
    clippy::too_many_lines,
    clippy::uninlined_format_args,
    clippy::single_match_else,
    clippy::struct_field_names,
    // M7 director v0.5 additions: phases/reinforcement/boss/graph modules
    // trip a handful of additional pedantic lints whose remedy doesn't add
    // clarity (inherent `from_str`, similar names across enums, etc.).
    clippy::should_implement_trait,
    clippy::explicit_iter_loop,
    clippy::needless_lifetimes,
    clippy::similar_names,
    clippy::map_unwrap_or
)]

// M2 spec "## Files" wiring: re-export the canonical types via thin
// modules so consumers that import per the spec path (`cf_mission::objective::*`
// / `cf_mission::director::*`) compile cleanly.
pub mod director;
pub mod m14a_resource_drain;
pub mod m14b_world;
pub mod objective;
pub use m14a_resource_drain::{
    helmet_o2_inhaled_mol_per_tick, skips_unstable_for_origin, stride_drain_for_origin, OriginClass, StrideDrain,
};
pub use m14b_world::{ScenarioAtmosCell, ScenarioGravityOverride, ScenarioWindSource};

// **M7**: Mission director v0.5 — additive multi-objective DiGraph + 4-phase
// pacing + reinforcement waves + mini-boss patterns. The M2 single-vec
// objective list keeps working unchanged; M7 layers the v0.5 graph on top
// so scenarios can opt in.
pub mod boss_phases;
pub mod objective_graph;
pub mod phases;
pub mod reinforcement;

// **M9**: Reactor pressure-state machine + 3-layer armor cascade
// (External / Internal / Core). Forward-compat surface for the M13 chassis
// 15-zone × 3-layer model and the M25+ command-core (DR-027). Lives in a
// sibling module so the lib.rs `Reactor` struct can compose the M9 types
// without bloating the file.
pub mod reactor;

// **M9B**: launch scenario registry (8 trench scenarios under
// `game/content/scenarios/m9b_*.ron`). The module exposes
// `SCENARIO_IDS` + `registry()` + `tick_budget_for(id)` so closure-
// feature verification can enumerate the M9B launch roster without
// scanning the filesystem.
pub mod m9b_scenarios;

// **M9C**: launch scenario registry (10 fortification scenarios under
// `game/content/scenarios/m9c_*.ron`). Same shape as
// `m9b_scenarios`: enumerated registry + per-scenario tick budgets
// (with `m9c_full_strongpoint` budgeted at 3600 ticks per VAL-M9C-050
// / VAL-CROSS-006).
pub mod m9c_scenarios;

pub use reactor::{
    pressure_state_for_hp_percent, ArmorLayerHpEvent, LayerKind, LayerState, PressureState, ReactorDamageReport,
    TIMER_WARNING_THRESHOLDS_S,
};

pub use boss_phases::{BossPhase, BossPhaseChangedEvent, BossSpecialAbilityEvent, BossState};
pub use objective_graph::{
    BranchingPoint, ExtendedObjectiveKind, ObjectiveBranchedEvent, ObjectiveGraph, ObjectiveNode, ObjectiveNodeStatus,
    OptionalOfferedEvent,
};
pub use phases::{DirectorPhaseChangeEvent, MissionPhase, PhaseChangedEvent, PhaseState};
pub use reinforcement::{ReinforcementRegistry, ReinforcementWave, ReinforcementWaveSpawnedEvent};

// Mission core split out of this file for the 2k-LOC ceiling. The public
// surface is re-exported below so `cf_mission::*` paths stay stable.
mod loss;
mod objective_types;
mod reactor_world;
mod result;
mod state;
mod step_engine;
mod tick;
mod view;

pub use loss::{LossConditions, LossReason};
pub use objective_types::{FailSensor, Objective, ObjectiveKind, ObjectiveStatus};
pub use reactor_world::{Reactor, ReactorWorld};
pub use result::{MissionLifecycle, MissionResult};
pub use state::MissionState;
pub use step_engine::step;
pub use tick::{MissionTickInputs, MissionTickReport, ObjectiveProgressUpdate};
pub use view::{MissionView, ObjectiveView};

#[cfg(test)]
mod tests;
