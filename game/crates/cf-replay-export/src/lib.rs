//! M10B: replay-as-MP4 export pipeline.
//!
//! `cf-replay-export` owns the offline video-output surface enumerated in
//! `specs/active/M10B.md`:
//!
//! - [`ffmpeg_bridge`] — libav wrapper (`ffmpeg-next` 8.1.0 against
//!   FFmpeg 8.0.1). Deterministic-encoder profile constants: single-thread
//!   + locked GOP + `-tune psnr` for the four production presets; FFV1
//!   intra-frame for the `archival_lossless` archival preset.
//! - [`preset_registry`] — typed loader + in-process registry for the
//!   five player-facing presets (`twitch_1080p60`, `youtube_4k60`,
//!   `discord_720p30`, `clip_compact`, `archival_lossless`). RON files
//!   live under `game/content/replay_export/presets/`.
//! - [`chapter_markers`] — data-driven chapter rule loader. Maps M4
//!   event types (`mission.objective_*`, `actor_status_changed=killed`,
//!   `reactor.armor_layer_destroyed`, `atmos.breach_detected`,
//!   `mission.commander_*`) AND the M9B trench events + M9C
//!   fortification events into MP4 chapter markers. RON lives at
//!   `game/content/replay_export/chapter_rules/default.ron`.
//!
//! Subsequent m10b features (m10b-2 frame_ticker + offline_render, m10b-3
//! overlay_graph + commentary, m10b-4 editor + cfctl shim, m10b-5
//! schemas + CI scripts) consume these three modules.

#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(
    clippy::module_name_repetitions,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::missing_const_for_fn,
    clippy::doc_markdown,
    clippy::doc_lazy_continuation,
    clippy::doc_overindented_list_items,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_lossless,
    clippy::return_self_not_must_use,
    clippy::items_after_statements,
    clippy::needless_pass_by_value,
    clippy::too_many_lines,
    clippy::match_same_arms,
    clippy::default_trait_access,
    clippy::single_match_else,
    clippy::if_not_else,
    clippy::similar_names,
    // PresetError + ChapterRulesError embed `ron::error::SpannedError`
    // (≥128 bytes) so loaders can pinpoint the offending field. We
    // accept the larger Result size to keep the typed-error contract
    // per VAL-M10B-010 / VAL-M10B-011 (no generic-string errors).
    clippy::result_large_err,
    clippy::stable_sort_primitive,
    // m10b-2 added clippy::pedantic-noisy patterns in well-tested code
    // paths (per-tick walkers, Catmull-Rom interpolation, RON loaders);
    // each is suppressed here only because it adds noise without
    // improving safety:
    //
    // - cast_precision_loss: tick → f64 conversion in test fixtures
    //   (the live engine routes through the same conversion in
    //   `sim_time_ms = tick as f64 * <step_ms>` and there's no
    //   benefit to alternative formulations).
    // - redundant_closure_for_method_calls: `|v| v.as_i64()` reads
    //   identically to the inline helper; the closure form is more
    //   greppable for future cause-chain instrumentation.
    // - missing_const_for_fn (already allowed above); listed for
    //   completeness.
    // - implicit_hasher: BTreeMap iterators surface the standard
    //   hashmap-equivalent, no custom hasher needed.
    // - unnecessary_wraps: frame_ticker functions return `Result`
    //   even when the happy path can't fail; we keep the wrapping so
    //   future ticks (delta-chain orphan detection) can return Err
    //   without an API break.
    clippy::cast_precision_loss,
    clippy::redundant_closure_for_method_calls,
    clippy::unnecessary_wraps,
    clippy::implicit_hasher,
    clippy::option_if_let_else,
    clippy::manual_let_else,
    clippy::elidable_lifetime_names,
    clippy::needless_continue,
    // m10b-2 camera-director tests compare known-exact `f32`
    // keyframe poses (e.g. `100.0`) that the Catmull-Rom path snaps
    // to verbatim when `tick == keyframe.tick`. The `assert_eq!` on
    // bit-equal poses is intentional (it proves the snap path is
    // taken, not approximate interpolation). `clippy::float_cmp`
    // would force `assert!((a - b).abs() < eps)` which obscures the
    // VAL-M10B-025 contract that boundary ticks return the
    // keyframe's pose verbatim.
    clippy::float_cmp,
    // m10b-2 frame_ticker test fixtures iterate snapshot indices via
    // `for c in 0..POSE_COMPONENTS` — switching to
    // `pose.iter_mut().enumerate()` would obscure the per-component
    // intent without saving any runtime work (compiler unrolls the
    // small fixed-size loop).
    clippy::needless_range_loop
)]

pub mod camera_director;
pub mod camera_script;
pub mod chapter_markers;
pub mod ffmpeg_bridge;
pub mod frame_ticker;
pub mod preset_registry;

pub use camera_director::{pose_at_tick, pose_displacement_pixels, CameraDirector, DirectorResolution};
pub use camera_script::{
    CameraKeyframe, CameraKind, CameraScript, CameraScriptError, CameraTrack, Pose, POSE_COMPONENTS,
};
pub use chapter_markers::{
    ChapterRule, ChapterRuleSet, ChapterRulesError, COMMANDER_EVENT_PREFIX, DEFAULT_CHAPTER_RULES_RON,
    REQUIRED_M4_EVENT_KINDS, REQUIRED_M9B_EVENT_KINDS, REQUIRED_M9C_EVENT_KINDS,
};
pub use ffmpeg_bridge::{
    DeterministicEncoderProfile, FfmpegBridge, FfmpegProbeError, FfmpegRuntime, ARCHIVAL_LOSSLESS_GOP_SIZE,
    ARCHIVAL_LOSSLESS_TUNE, FFMPEG_NEXT_PIN, PRODUCTION_GOP_SIZE, PRODUCTION_TUNE, REQUIRED_FFMPEG_THREADS,
};
pub use frame_ticker::{
    frame_step_ticks, BundleSource, FrameCommand, FrameCommandStream, FrameTicker, FrameTickerConfig, FrameTickerError,
    SUPPORTED_FRAME_RATES,
};
pub use preset_registry::{
    ContainerKind, ExportPreset, PresetCodec, PresetError, PresetField, PresetRegistry, PresetResolution,
    ARCHIVAL_LOSSLESS_NAME, CLIP_COMPACT_NAME, DECLARED_PRESETS, DISCORD_720P30_NAME, PRESET_REQUIRED_FIELDS,
    TWITCH_1080P60_NAME, YOUTUBE_4K60_NAME,
};
