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
    clippy::needless_range_loop,
    // m10b-3 raw-string tests use `r#"..."#` because some RON test
    // fixtures embed double quotes; pedantic flags the hash-pair when
    // a specific fixture happens not to use `"`, but the consistency
    // is more valuable than the byte saving.
    clippy::needless_raw_string_hashes,
    // m10b-3 sample-rate constants (48_000 used as `48_000_u64`-ish
    // literals in audio-tick math) read more naturally without
    // separator for the math operand; allowing in this crate.
    clippy::unreadable_literal,
    // m10b-3 audio + chapter-derivation lookups use `map(|v|
    // v.as_str()).unwrap_or("")` to keep the read sites flat against
    // the `payload.get(...)?` pattern; the `.and_then` alternative
    // adds line noise without saving allocations.
    clippy::map_unwrap_or,
    // m10b-3 keeps the inherent renderer lifetime explicit on the
    // [`CauseChainOverlay::render`] receiver because the borrow
    // depends on the events slice; clippy's `'_` suggestion obscures
    // the binding.
    clippy::needless_lifetimes,
    // m10b-3 overlay_graph emits an `orphan_warning` with `&self`
    // intentionally — production callers route the warning through
    // an instance whose composition state may inform the message in
    // a future iteration. Tests for clean-uninstall exercise this
    // explicitly.
    clippy::unused_self,
    // m10b-3 chapter-timeline tests + audio_base_mix use abs-diff
    // arithmetic via the `if-then-else` form; switching to
    // `abs_diff` reads identically and isn't worth the lint churn
    // because both terms come from u32 sample / pixel positions.
    clippy::manual_abs_diff
)]

pub mod audio_base_mix;
pub mod audit_events;
pub mod camera_director;
pub mod camera_script;
pub mod cause_chain_walker;
pub mod chapter_derivation;
pub mod chapter_markers;
pub mod commentary;
pub mod default_output_path;
pub mod encoder_session;
pub mod ffmpeg_bridge;
pub mod frame_ticker;
pub mod overlay_cause_chain;
pub mod overlay_chapter_timeline;
pub mod overlay_graph;
pub mod overlay_hud;
pub mod overlay_kill_feed;
pub mod overlay_watermark;
pub mod preset_registry;
pub mod slow_mo;

pub use audio_base_mix::{
    linear_to_dbfs, peak_dbfs_at_tick, synthesis_frequency_hz, synthesize_base_mix,
    synthesize_base_mix_or_silence, synthesize_silent_base_mix, AudioEvent, ENVELOPE_LENGTH_SAMPLES,
    NO_AUDIO_BASE_FLOOR_DBFS, PEAK_THRESHOLD_DBFS,
};
pub use audit_events::{
    emit_export_audit_events, ExportJobMetadata, EVENT_CATEGORY, EVENT_TYPE_CHAPTER_MARKER_EMITTED,
    EVENT_TYPE_EXPORT_COMPLETED, EVENT_TYPE_EXPORT_STARTED,
};
pub use camera_director::{pose_at_tick, pose_displacement_pixels, CameraDirector, DirectorResolution};
pub use camera_script::{
    CameraKeyframe, CameraKind, CameraScript, CameraScriptError, CameraTrack, Pose, POSE_COMPONENTS,
};
pub use cause_chain_walker::{trace as cause_chain_trace, CauseChain, ChainLink, ChainTermination};
pub use chapter_derivation::{counts_by_event_type, interpolate, ChapterDerivation, ChapterMarker};
pub use chapter_markers::{
    ChapterRule, ChapterRuleSet, ChapterRulesError, COMMANDER_EVENT_PREFIX, DEFAULT_CHAPTER_RULES_RON,
    REQUIRED_M4_EVENT_KINDS, REQUIRED_M9B_EVENT_KINDS, REQUIRED_M9C_EVENT_KINDS,
};
pub use commentary::{
    linear_interp_resample, max_drift_frames, sample_offsets_for_clip, CommentaryError, CommentaryScript,
    VoiceClip, CAPTION_LOC_BUNDLE_SIZE_CAP, COMMENTARY_CHANNELS, COMMENTARY_SAMPLE_RATE_HZ,
};
pub use encoder_session::{
    container_extension, EncodeError, EncodeReport, EncoderConfig, EncoderSession, ENCODER_AUDIO_CHANNELS,
    ENCODER_AUDIO_SAMPLE_RATE,
};
pub use ffmpeg_bridge::{
    simulated_missing_ffmpeg_error, DeterministicEncoderProfile, FfmpegBridge, FfmpegProbeError, FfmpegRuntime,
    ARCHIVAL_LOSSLESS_GOP_SIZE, ARCHIVAL_LOSSLESS_TUNE, FFMPEG_NEXT_PIN, PRODUCTION_GOP_SIZE, PRODUCTION_TUNE,
    REQUIRED_FFMPEG_THREADS,
};
pub use frame_ticker::{
    frame_step_ticks, BundleSource, FrameCommand, FrameCommandStream, FrameTicker, FrameTickerConfig, FrameTickerError,
    SUPPORTED_FRAME_RATES,
};
pub use overlay_cause_chain::{
    render_chain as render_cause_chain, render_event_plain as render_cause_chain_event_plain, CauseChainOverlay,
    RenderedLine, CAUSE_CHAIN_AOI_HEIGHT, CAUSE_CHAIN_AOI_WIDTH, CAUSE_CHAIN_AOI_X, CAUSE_CHAIN_AOI_Y,
    CAUSE_CHAIN_FALLBACK_KEY, CAUSE_CHAIN_LINK_DWELL_TICKS, CAUSE_CHAIN_LOCALIZATION_PREFIX,
};
pub use overlay_chapter_timeline::{
    ChapterTimelineOverlay, TickMark, CHAPTER_TIMELINE_AOI_HEIGHT, CHAPTER_TIMELINE_AOI_WIDTH,
    CHAPTER_TIMELINE_AOI_X, CHAPTER_TIMELINE_AOI_Y,
};
pub use overlay_graph::{
    ModOverlayDeclaration, OverlayGraph, OverlayGraphBuilder, OverlayGraphError, OverlayLayer, OverlaySource,
    CAUSE_CHAIN_OVERLAY_NAME, CAUSE_CHAIN_Z_ORDER, CHAPTER_TIMELINE_OVERLAY_NAME, CHAPTER_TIMELINE_Z_ORDER,
    HUD_OVERLAY_NAME, HUD_Z_ORDER, KILL_FEED_OVERLAY_NAME, KILL_FEED_Z_ORDER, OVERLAY_GRAPH_TRACE_TARGET,
    WATERMARK_OVERLAY_NAME, WATERMARK_Z_ORDER,
};
pub use overlay_hud::{HudOverlay, HUD_AOI_HEIGHT, HUD_AOI_WIDTH, HUD_AOI_X, HUD_AOI_Y};
pub use overlay_kill_feed::{
    derive_entries as derive_kill_feed_entries, KillFeedEntry, KillFeedOverlay, KILL_FEED_AOI_HEIGHT,
    KILL_FEED_AOI_WIDTH, KILL_FEED_AOI_X, KILL_FEED_AOI_Y, KILL_FEED_CAUSE_FILTER, KILL_FEED_DWELL_TICKS,
};
pub use overlay_watermark::{
    WatermarkOverlay, WatermarkProvenance, WATERMARK_AOI_HEIGHT, WATERMARK_AOI_WIDTH, WATERMARK_AOI_X,
    WATERMARK_AOI_Y, WATERMARK_FIELD_TRUNCATE,
};
pub use default_output_path::{
    default_output_directory, default_output_filename, default_output_path, CORE_FALL_OUTPUT_SUBDIR,
};
pub use preset_registry::{
    ContainerKind, ExportPreset, PresetCodec, PresetError, PresetField, PresetRegistry, PresetResolution,
    ARCHIVAL_LOSSLESS_NAME, CLIP_COMPACT_NAME, DECLARED_PRESETS, DISCORD_720P30_NAME, PRESET_REQUIRED_FIELDS,
    TWITCH_1080P60_NAME, YOUTUBE_4K60_NAME,
};
pub use slow_mo::{
    SlowMoError, SlowMoMultiplier, DEFAULT_SLOW_MO_MULTIPLIER, MAX_SLOW_MO_MULTIPLIER,
};
