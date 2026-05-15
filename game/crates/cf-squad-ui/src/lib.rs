//! cf-squad-ui — owns the entire three-layer commandable-AI player UX:
//!
//! - **Layer 2** Tab tactical overlay + Priority Editor + Plan Composer
//! - **Layer 3** live tactical: Q-hold context wheel + single-key panic
//!   (M = medic, R = repair, G = grenade) + MMB tag + 'Why?' (Y) key
//!
//! cfctl methods that surface here:
//! `act.player.toggle_tactical_overlay`,
//! `act.player.compose_plan { actor_id, steps[] }`,
//! `act.player.context_wheel_select { actor_id, slot }`,
//! `act.player.panic_call { kind: medic/engineer/grenade }`,
//! `act.player.tag_target { target_id }`,
//! `act.player.query_why { actor_id }` (returns reason_label).
//!
//! The crate keeps every type Bevy-free so cf-control can mutate state
//! deterministically and cf-replay can snapshot/restore the entire
//! commandability layer round-trippably.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod context_wheel;
pub mod panic;
pub mod plan_composer;
pub mod priority_editor;
pub mod tactical_overlay;
pub mod tag;
pub mod why;

pub use context_wheel::{context_wheel_for, ContextOrderKind, ContextWheel, ReticleTarget, WheelSlot, WHEEL_SLOTS_LEN};
pub use panic::{PanicCommand, PanicKind};
pub use plan_composer::{Plan, PlanComposeError, PlanStep, PlanStepKind, MAX_PLAN_STEPS};
pub use priority_editor::{PriorityEditAction, PriorityEditError, PriorityEditorView};
pub use tactical_overlay::{TacticalOverlayState, MULTIPLAYER_TACTICAL_SIM_SPEED_PCT};
pub use tag::{TagInfo, TagState, DEFAULT_TAG_TTL_TICKS, TAG_UTILITY_BONUS};
pub use why::WhyView;
