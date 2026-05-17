//! M9B: declarative content kernel (loaders + validators).
//!
//! The launch surface for this milestone is the
//! [`trench_templates`] module: a loader + validator for `.trench.ron`
//! authored-content files per `specs/active/M9B.md` § "Per-zone trench
//! templates (CC parity)". The same kernel exposes the placeholder
//! grammar for M9C-owned fortifications (forward-compat per the spec
//! notes — "load gracefully with a `missing fortification` warning
//! event").

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
    clippy::cast_precision_loss,
    clippy::cast_lossless,
    clippy::return_self_not_must_use,
    clippy::items_after_statements,
    clippy::derivable_impls,
    clippy::struct_excessive_bools,
    clippy::needless_pass_by_value,
    clippy::too_many_lines,
    clippy::match_same_arms,
    clippy::similar_names,
    clippy::if_not_else
)]

pub mod trench_templates;

pub use trench_templates::{
    placeholder_warning_label, template_sha256, FortificationPlaceholder, FortificationResolution,
    InstantiatedTemplate, MissingFortificationWarning, PlacedFortification, PlacedSegment,
    SegmentOverride, TemplateLoadError, TemplateZone, TrenchTemplate, TrenchTemplateInstantiation,
    KNOWN_FORTIFICATION_IDS,
};
