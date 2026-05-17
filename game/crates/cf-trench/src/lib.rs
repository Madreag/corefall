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

pub mod breastwork;
pub mod collapse;
pub mod cover_change;
pub mod cover_state;
pub mod damage_routing;
pub mod dig_validation;
pub mod drainage;
pub mod modules;
pub mod parapet_raised_forward_compat;
pub mod segment;

pub use breastwork::{
    apply_round_to_breastwork, cover_state_post_breach, run_breach_sequence,
    BreastworkHitOutcome, BREASTWORK_MAX_HP, ROUND_DAMAGE_J,
};
pub use collapse::{
    collapse_tick, run_revetment_audit, variant_supports_collapse, CollapseCause,
    CollapseEnv, CollapseTickOutcome, COLLAPSE_INTEGRITY_FLOOR,
    REVETMENT_AUDIT_WINDOW_TICKS, REVETMENT_INTEGRITY_FLOOR, SOFT_DIRT_THRESHOLD,
    SOFT_DIRT_DECAY_PER_TICK, STARTING_INTEGRITY,
};
pub use cover_change::{
    cover_state_change, CoverStateChangeCause, CoverStateChangeEvent,
};
pub use cover_state::{cover_state, cover_state_fire_step, CoverState, TrenchStance};
pub use damage_routing::{
    damage_route_for, DamageRoute, DamageZone,
};
pub use dig_validation::{
    dig_substrate_validate, DigSubstrateOutcome, DEEP_HARDNESS_THRESHOLD,
    SUBSTRATE_TOO_HARD_REASON, WARNING_EVENT_KIND as DIG_DOWNGRADE_EVENT_KIND,
};
pub use drainage::{
    drainage_sump_tick, run_drainage_window, DrainageEnv, DrainageTickOutcome,
    FLUSH_FLOOR_PX, FLUSH_THRESHOLD_PX, RAIN_ACCUMULATION_PER_TICK_PX,
};
pub use modules::{ModuleSpec, TrenchModule};
pub use parapet_raised_forward_compat::{
    parapet_raised_dig_validate, warning_event_kind as parapet_raised_warning_event_kind,
    ParapetRaisedRequiresM9c,
};
pub use segment::{
    InMemorySegments, SegmentSpec, SegmentVariant, TrenchSegment, TrenchSegmentLookup,
    TrenchSegmentRuntime,
};
