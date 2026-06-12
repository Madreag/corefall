//! **M14I** — Storyteller narrative-event substrate.
//!
//! Canonical owner of narrative-event-id registration for M14I's
//! retirement flow. The actual storyteller orchestration is owned by a
//! downstream milestone (M48 narrative director); this crate ships the
//! minimum surface that M14I needs:
//!
//! - [`NarrativeEventKind`] — locked enum of M14I narrative events.
//! - [`retirement_event`] — module exposing the canonical retirement
//!   narrative event id + a registration helper.
//!
//! Consumers (M48 storyteller / M41 veteran roster / cf-ui veteran
//! dossier) read these ids to surface narrative beats when a veteran
//! retires.

#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(
    clippy::module_name_repetitions,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::doc_markdown,
    clippy::missing_const_for_fn,
    clippy::unused_self,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

pub mod retirement_event;
pub mod m16_event;
pub mod pandemic;

pub use retirement_event::{
    register_retirement_narrative, RetirementNarrative, RetirementNarrativeRegistry,
    NARRATIVE_EVENT_ID_VETERAN_RETIRED,
};
pub use m16_event::{
    register_m16_narratives, M16NarrativeKind, M16NarrativeRegistration, M16NarrativeRegistry,
    NARRATIVE_EVENT_ID_ACID_POOL_GROWTH, NARRATIVE_EVENT_ID_ANOMALY_ENCOUNTERED,
    NARRATIVE_EVENT_ID_DROWNING_LETHAL, NARRATIVE_EVENT_ID_ELECTRIC_ARC_CASCADE,
    NARRATIVE_EVENT_ID_FIRE_SPREAD, NARRATIVE_EVENT_ID_RADIATION_STORM,
};
pub use pandemic::{
    declare_base_lockdown, register_pandemic_narratives, BaseLockdown, PandemicDeclaredEvent,
    PandemicNarrativeRegistration, PandemicNarrativeRegistry, PandemicTracker,
    NARRATIVE_EVENT_ID_PANDEMIC_DECLARED, PANDEMIC_LOCKDOWN_CHATTER,
};

use serde::{Deserialize, Serialize};

/// milestones (M48 storyteller) without breaking serde compatibility.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NarrativeEventKind {
    /// Veteran retirement — emitted when an actor commits to retirement
    /// via `act.player.retire_veteran`.
    VeteranRetired = 0,
}

impl NarrativeEventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            NarrativeEventKind::VeteranRetired => "veteran_retired",
        }
    }
}
