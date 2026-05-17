//! M9B: trench-network kernel.
//!
//! `cf-trench` owns the authored-trench surfaces enumerated in
//! `specs/active/M9B.md`:
//!
//! - [`segment`] — [`segment::TrenchSegment`] + [`segment::SegmentVariant`]
//!   (6 cross-section variants).
//! - [`cover_state`] — [`cover_state::CoverState`] enum (`Exposed | Partial |
//!   Full`) and the pure [`cover_state::cover_state`] derivation function.
//!   Cover is **derived, not cached**: every call recomputes the value from
//!   stance × segment per the M9B notes ("at every frame,
//!   `cover_state(actor) = lookup(segment_at(actor.pos)) × actor.stance`").
//! - [`modules`] — [`modules::TrenchModule`] (6 embedded modules: duckboard,
//!   fire_step, breastwork, drainage_sump, revetment, corner_traverse).
//!
//! Downstream crates ( `cf-actor`, `cf-control`, `cf-ai`, `cf-render-2d`,
//! `cf-procgen`, `cf-content`, … ) consume these three modules.

#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(
    clippy::module_name_repetitions,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::missing_const_for_fn,
    clippy::doc_markdown,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_lossless,
    clippy::return_self_not_must_use,
    clippy::items_after_statements,
    clippy::derivable_impls,
    clippy::struct_excessive_bools,
    clippy::needless_pass_by_value,
    clippy::too_many_lines,
    clippy::match_same_arms
)]

pub mod cover_state;
pub mod modules;
pub mod segment;

pub use cover_state::{cover_state, cover_state_fire_step, CoverState, TrenchStance};
pub use modules::{ModuleSpec, TrenchModule};
pub use segment::{SegmentSpec, SegmentVariant, TrenchSegment, TrenchSegmentLookup};
