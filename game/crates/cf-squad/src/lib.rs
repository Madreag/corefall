//! M6: squad-of-two surface (1 friendly bot, 4 commands).
//!
//! M6 ships the basic squad management surface; M7 layers full AI archetypes
//! on top of `SquadMember.behavior_role`. The crate keeps the squad state
//! pure (no AI doctrine) so cf-control can dispatch commands and cf-ai can
//! react without circular dependencies.

#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(
    clippy::module_name_repetitions,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::doc_markdown,
    clippy::missing_const_for_fn,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]

pub mod command;
pub mod squad;

pub use command::{SquadCommand, SquadCommandKind};
pub use squad::{Squad, SquadMember, SquadRole};
