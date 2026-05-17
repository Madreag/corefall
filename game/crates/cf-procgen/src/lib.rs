//! M9B: procedural-content generators owned by Corefall.
//!
//! The launch surface for this milestone is the
//! [`trench_generator`] module, which builds WWI-style zigzag trench
//! polylines per `specs/active/M9B.md` § "Zigzag pattern procgen
//! generator". The same crate also exposes the
//! [`trench_generator::ruin_procgen`] pass that decorates a ruined-biome
//! map with 2–4 decayed trench template instances (per spec § "Procgen
//! rotted trenches in PvE ruins (M43)").
//!
//! Per project AGENTS.md, sim crates do not use `thread_rng()` — the
//! kernel threads seeded `rand_xoshiro::Xoshiro256StarStar` generators
//! through the [`trench_generator::ZigzagInput`] struct.

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
    clippy::if_not_else,
    clippy::unreadable_literal,
    clippy::needless_range_loop,
    clippy::many_single_char_names
)]

pub mod trench_generator;

pub use trench_generator::{
    generate_trench_polyline, ruin_procgen, Endpoint, EndpointFacing, GeneratorError, Kink,
    KinkAngle, PolylineHash, ResolvedZigzag, RuinPlacement, RuinProcgenInput, RuinProcgenOutput,
    ZigzagInput,
};
