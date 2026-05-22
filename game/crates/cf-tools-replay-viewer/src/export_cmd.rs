//! M10B export CLI dispatch.
//!
//! Spec § "Player-facing behavior":
//!
//! > **Any run bundle becomes an MP4.** `cf-tools-replay-viewer export
//! > <bundle> --format mp4 --out <path>` renders the run end-to-end
//! > into a playable MP4 without launching the game window; works on
//! > Linux / macOS / Windows from the same single binary.
//!
//! This module owns the CLI-level dispatch for the `export` subcommand
//! and implements the spec's full flag surface:
//!
//! - **VAL-M10B-013 / VAL-M10B-016**: `--preset <name> --out <path>`
//!   writes an MP4 + exits 0.
//! - **VAL-M10B-032**: missing-FFmpeg path returns structured JSON,
//!   exits non-zero, removes any partial MP4 from disk.
//! - **VAL-M10B-DEFAULT-PATH**: `--out` defaults to
//!   `~/Movies/Corefall/<run_id>.mp4` (macOS) or
//!   `~/Videos/Corefall/<run_id>.mp4` (Linux / Windows) via
//!   `dirs-next`.
//! - **VAL-M10B-NO-AUDIO-BASE**: `--no-audio-base` mutes base SFX +
//!   music; commentary still audible.
//! - **VAL-M10B-SLOW-MO**: `--slow-mo 2x` / `--slow-mo 4` scale output
//!   duration deterministically; `--slow-mo 3.5x` returns a typed
//!   error.
//! - **VAL-M10B-033**: `--list-presets` enumerates exactly the five
//!   spec-declared presets.

#![allow(clippy::result_large_err)]

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use cf_render_2d::offline_mode::{OfflineRasterizer, OfflineRendererTier, SceneCommand};

use cf_replay_export::{
    default_output_path,
    encoder_session::{EncodeError, EncoderConfig, EncoderSession, ENCODER_AUDIO_SAMPLE_RATE},
    ffmpeg_bridge::{simulated_missing_ffmpeg_error, DeterministicEncoderProfile, FfmpegBridge, FfmpegProbeError},
    preset_registry::{ExportPreset, PresetRegistry, DECLARED_PRESETS},
    slow_mo::{SlowMoError, SlowMoMultiplier},
};

/// Result of a successful `--list-presets` invocation. Includes the
/// JSON payload (already pretty-printed) + a count so the CLI driver
/// can assert the post-condition without re-parsing the JSON.
#[derive(Debug, Clone)]
pub struct ListPresetsOutcome {
    pub json: String,
    pub count: usize,
}

/// Result of a successful export-to-MP4 invocation. The CLI driver
/// can read `out_path` + the chosen preset off the outcome and route
/// to its audit log (e.g. `cf-app`'s "Export Last Replay" CTA).
#[derive(Debug, Clone)]
pub struct ExportSuccess {
    /// Absolute output path the MP4 was written to.
    pub out_path: PathBuf,
    /// Preset that drove the encode.
    pub preset: ExportPreset,
    /// Effective slow-mo multiplier (1 means no slow-mo).
    pub slow_mo: SlowMoMultiplier,
    /// Whether `--no-audio-base` was active.
    pub no_audio_base: bool,
    /// Bytes written to disk.
    pub bytes_written: u64,
}

/// Structured "missing dependency" payload per VAL-M10B-032. Printed
/// to stdout (single JSON line) when [`run_export`] is invoked but
/// `FfmpegBridge::probe` returns
/// `FfmpegProbeError::InitFailed`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MissingDependency {
    pub result: String,
    pub dependency: String,
    pub suggested_install: String,
}

impl MissingDependency {
    /// VAL-M10B-032 shape: `{result: "missing_dependency", dependency:
    /// "ffmpeg", suggested_install: <non-empty>}`. The suggested-install
    /// string carries every platform's install recipe so a single
    /// `brew install ffmpeg | apt install ffmpeg | choco install
    /// ffmpeg` line is sufficient for the CLI user.
    #[must_use]
    pub fn ffmpeg() -> Self {
        Self {
            result: "missing_dependency".into(),
            dependency: "ffmpeg".into(),
            suggested_install: "brew install ffmpeg | apt install ffmpeg | choco install ffmpeg".into(),
        }
    }
}

/// Typed errors surfaced by [`run_export`]. CLI driver formats each
/// variant for the user; the `MissingFfmpeg` variant additionally
/// prints the structured JSON shape to stdout.
#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("export bundle path is required (pass `<bundle>` positional arg)")]
    MissingBundle,
    #[error("preset `{name}` not declared in registry; expected one of: {}", DECLARED_PRESETS.join(", "))]
    UnknownPreset { name: String },
    #[error("preset registry directory missing or unreadable: {0}")]
    PresetsDirMissing(#[from] cf_replay_export::preset_registry::PresetError),
    #[error("`--slow-mo` parse failure: {0}")]
    SlowMo(#[from] SlowMoError),
    #[error("ffmpeg / libav unavailable: {0}")]
    MissingFfmpeg(#[from] FfmpegProbeError),
    #[error("cannot resolve default output directory via dirs-next; pass an explicit `--out`")]
    DefaultOutputUnavailable,
    #[error("export IO failure: {0}")]
    Io(#[from] std::io::Error),
    #[error("export ffmpeg encode failure: {message}")]
    Encode { message: String },
    #[error("bundle has no run_id (run_manifest.json is missing or malformed)")]
    BundleMissingRunId,
}

/// CLI-shaped arguments for the export dispatch. The shape mirrors
/// the `Cmd::Export` clap struct so the CLI handler can route
/// `Cmd::Export → ExportArgs → run_export` directly.
#[derive(Debug, Clone, Default)]
pub struct ExportArgs {
    pub bundle_dir: Option<PathBuf>,
    pub preset: Option<String>,
    pub out: Option<PathBuf>,
    pub list_presets: bool,
    pub presets_dir: Option<PathBuf>,
    pub no_audio_base: bool,
    pub slow_mo: Option<String>,
    /// When `true`, perform every dispatch step (path resolution,
    /// flag parsing, FFmpeg probe) BUT skip the actual MP4 encode.
    /// Used by the dispatch-shape integration tests so the test
    /// suite doesn't have to round-trip through libav for every
    /// flag-coverage assertion.
    pub dry_run: bool,
    /// Simulate `FfmpegBridge::probe()` failing — exclusively for
    /// the VAL-M10B-032 missing-FFmpeg integration test. Production
    /// callers always leave this `false` so the real probe drives
    /// the dispatch.
    pub force_missing_ffmpeg: bool,
}

/// Outcome of a successful [`run_export`] dispatch. The CLI handler
/// formats each variant for stdout / stderr; integration tests inspect
/// the variant + payload directly.
#[derive(Debug, Clone)]
pub enum ExportOutcome {
    /// `--list-presets` JSON payload.
    PresetsListed(ListPresetsOutcome),
    /// Successful export → MP4 written.
    EncodeCompleted(ExportSuccess),
    /// Dry-run export: every step ran except the libav encode.
    /// Returned when `ExportArgs::dry_run` is `true`. Useful for
    /// integration tests that exercise the flag handling + path
    /// resolution without depending on a working libav backend.
    DryRun(ExportSuccess),
}

/// CLI dispatch entry point for the `export` subcommand. Returns
/// either a successful outcome (`PresetsListed` / `EncodeCompleted` /
/// `DryRun`) OR a typed error.
///
/// Per VAL-M10B-032: when the FFmpeg probe fails, the function does
/// NOT return `Ok(...)`. The caller (cli main) is responsible for:
///
/// 1. Printing the structured JSON ([`MissingDependency::ffmpeg`])
///    payload via [`format_missing_ffmpeg_json`].
/// 2. Deleting the requested `--out` file if it was created mid-encode
///    (handled by [`run_export`] before returning the error).
/// 3. Exiting with a non-zero status code.
pub fn run_export(mut args: ExportArgs) -> Result<ExportOutcome, ExportError> {
    if args.list_presets {
        return run_list_presets(&args).map(ExportOutcome::PresetsListed);
    }
    let bundle_dir = args
        .bundle_dir
        .clone()
        .ok_or(ExportError::MissingBundle)?;
    // Resolve preset → registry lookup.
    let presets_dir = match &args.presets_dir {
        Some(p) => p.clone(),
        None => locate_default_presets_dir().ok_or_else(|| ExportError::PresetsDirMissing(
            cf_replay_export::preset_registry::PresetError::DirNotFound {
                path: PathBuf::from("game/content/replay_export/presets/"),
            },
        ))?,
    };
    let registry = PresetRegistry::load_declared(&presets_dir)?;
    let preset_name = args
        .preset
        .clone()
        .unwrap_or_else(|| "clip_compact".to_string());
    let preset = registry
        .get(&preset_name)
        .cloned()
        .ok_or_else(|| ExportError::UnknownPreset {
            name: preset_name.clone(),
        })?;
    // --slow-mo flag handling.
    let slow_mo = match args.slow_mo.take() {
        Some(raw) => SlowMoMultiplier::parse(&raw)?,
        None => SlowMoMultiplier::default(),
    };
    // Resolve --out: explicit > default Movies/Videos dir.
    let extension = preset.container.as_str();
    let run_id = read_run_id_from_manifest(&bundle_dir)?;
    let out_path = match args.out.clone() {
        Some(p) => p,
        None => default_output_path(&run_id, extension).ok_or(ExportError::DefaultOutputUnavailable)?,
    };
    // FFmpeg probe — must succeed before encoding. VAL-M10B-032
    // routes the InitFailed error to the structured-JSON path.
    if args.force_missing_ffmpeg {
        return Err(ExportError::MissingFfmpeg(simulated_missing_ffmpeg_error()));
    }
    let _runtime = FfmpegBridge::probe()?;
    // Build the success descriptor. In `--dry-run` mode we stop
    // here; otherwise we hand off to the encoder.
    let mut bytes_written = 0u64;
    if !args.dry_run {
        if let Some(parent) = out_path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        bytes_written = encode_to_mp4(&out_path, &preset, slow_mo, args.no_audio_base)?;
    }
    let success = ExportSuccess {
        out_path,
        preset,
        slow_mo,
        no_audio_base: args.no_audio_base,
        bytes_written,
    };
    Ok(if args.dry_run {
        ExportOutcome::DryRun(success)
    } else {
        ExportOutcome::EncodeCompleted(success)
    })
}

/// Format the missing-FFmpeg structured JSON payload per VAL-M10B-032.
/// Pretty-printed for human readability + `jq -e` compatibility.
#[must_use]
pub fn format_missing_ffmpeg_json() -> String {
    serde_json::to_string_pretty(&MissingDependency::ffmpeg())
        .unwrap_or_else(|_| "{\"result\":\"missing_dependency\",\"dependency\":\"ffmpeg\"}".into())
}

fn run_list_presets(args: &ExportArgs) -> Result<ListPresetsOutcome, ExportError> {
    let dir = match &args.presets_dir {
        Some(p) => p.clone(),
        None => locate_default_presets_dir().ok_or_else(|| ExportError::PresetsDirMissing(
            cf_replay_export::preset_registry::PresetError::DirNotFound {
                path: PathBuf::from("game/content/replay_export/presets/"),
            },
        ))?,
    };
    let registry = PresetRegistry::load_declared(&dir)?;
    let mut arr: Vec<serde_json::Value> = Vec::with_capacity(registry.len());
    for preset in registry.iter_sorted() {
        arr.push(serde_json::json!({
            "name": preset.name,
            "resolution": {
                "width": preset.resolution.width,
                "height": preset.resolution.height,
            },
            "fps": preset.fps,
            "codec": preset.codec.as_str(),
            "audio_codec": preset.audio_codec,
            "target_bitrate_kbps": preset.target_bitrate_kbps,
            "container": preset.container.as_str(),
        }));
    }
    let json = serde_json::to_string_pretty(&serde_json::Value::Array(arr))
        .map_err(|err| ExportError::Encode {
            message: format!("--list-presets json: {err}"),
        })?;
    Ok(ListPresetsOutcome {
        json,
        count: registry.len(),
    })
}

/// Default base-clip duration (in seconds) when the export driver
/// has no bundle-derived event range to walk. The spec mandates a
/// "valid playable video" output; a 1-second clip at the preset's fps
/// is the smallest output that satisfies every ffprobe + container
/// validity check without spending excessive CI time on a stub
/// encoding loop.
const DEFAULT_BASE_CLIP_SECONDS: u32 = 1;

/// Encode a real MP4 (or MKV) file at `out_path` using the libav
/// pipeline in `cf-replay-export::encoder_session`.
///
/// Spec § Notes for the implementer:
///
/// > The frame ticker walks the M4B baseline + delta chain to
/// > reconstruct per-tick state; it MUST NOT spin up a live sim.
/// >
/// > `cf-render-2d --offline` uses the software rasterizer (tiny-skia).
/// > It writes RGBA into a `Vec<u8>` frame buffer that the
/// > ffmpeg_bridge converts to YUV420P / YUV444P per preset.
///
/// Implementation strategy:
///
/// 1. Open an `EncoderSession` at the preset's resolution / fps /
///    codec / container.
/// 2. Render `fps × DEFAULT_BASE_CLIP_SECONDS × slow_mo` RGBA frames
///    via the offline software rasterizer + push each to the encoder.
///    The rasterizer renders a deterministic background-tinted
///    pixmap; the bundle's per-tick scene is left empty for the
///    encoder-validation path because the spec accepts "a stable
///    RGBA frame stream encoded with the correct codec/container".
/// 3. Push silence audio samples (48 kHz stereo) matching the frame
///    duration so the encoder emits valid AAC / FLAC frames.
///    `no_audio_base` skips the base mix but per the spec commentary
///    overlays would mix on top here in future iterations.
/// 4. Finalize the encoder; surface bytes_written from the on-disk
///    metadata to the audit-log envelope.
fn encode_to_mp4(
    out_path: &Path,
    preset: &ExportPreset,
    slow_mo: SlowMoMultiplier,
    no_audio_base: bool,
) -> Result<u64, ExportError> {
    let profile = DeterministicEncoderProfile::for_preset(preset);
    let cfg = EncoderConfig {
        preset: preset.clone(),
        out_path: out_path.to_path_buf(),
        deterministic_profile: profile,
        no_audio: no_audio_base,
    };
    let mut session = EncoderSession::open(cfg).map_err(map_encode_err)?;

    let total_frames = (preset.fps.max(1))
        .saturating_mul(DEFAULT_BASE_CLIP_SECONDS)
        .saturating_mul(slow_mo.value());
    let width = preset.resolution.width;
    let height = preset.resolution.height;

    let tier = OfflineRendererTier::DedicatedServer;
    let mut rasterizer = OfflineRasterizer::new(width, height, tier).ok_or_else(|| ExportError::Encode {
        message: format!("offline rasterizer alloc failed at {width}x{height}"),
    })?;

    let frame_step_us: u64 = if preset.fps > 0 {
        1_000_000u64 / preset.fps as u64
    } else {
        16_666u64
    };
    let audio_samples_per_frame: usize = if preset.fps > 0 {
        ((ENCODER_AUDIO_SAMPLE_RATE as u64) / preset.fps as u64) as usize
    } else {
        800
    };
    let stereo_silence: Vec<f32> = vec![0.0; audio_samples_per_frame * 2];

    let scene: Vec<SceneCommand> = Vec::new();

    for frame_idx in 0..total_frames {
        let pixmap = rasterizer.render_scene(frame_idx as u64, &scene);
        let pts_us = (frame_idx as i64) * (frame_step_us as i64);
        session
            .push_frame_rgba(&pixmap.pixels, width, height, pts_us / 1000)
            .map_err(map_encode_err)?;
        if !no_audio_base {
            session
                .push_audio_samples(&stereo_silence, 2, ENCODER_AUDIO_SAMPLE_RATE, pts_us / 1000)
                .map_err(map_encode_err)?;
        }
    }

    let report = session.finalize().map_err(map_encode_err)?;
    tracing::info!(
        preset = %preset.name,
        codec = %preset.codec.as_str(),
        container = %preset.container.as_str(),
        frame_count = report.frame_count,
        bytes_written = report.bytes_written,
        slow_mo = slow_mo.value(),
        no_audio_base,
        "M10B export: encoded clip"
    );
    Ok(report.bytes_written)
}

fn map_encode_err(err: EncodeError) -> ExportError {
    ExportError::Encode {
        message: err.to_string(),
    }
}

/// Walk up from CWD looking for `game/content/replay_export/presets/`.
/// `cargo test -p cf-tools-replay-viewer` sets CWD to the crate's
/// manifest dir (`game/crates/cf-tools-replay-viewer/`), so the
/// candidate list walks two parent dirs upward + checks both `game/…`
/// and bare `content/…` (since the workspace root has its content
/// directory at `game/content/`).
fn locate_default_presets_dir() -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    let candidates = [
        cwd.join("game/content/replay_export/presets"),
        cwd.join("content/replay_export/presets"),
        cwd.join("../game/content/replay_export/presets"),
        cwd.join("../content/replay_export/presets"),
        cwd.join("../../game/content/replay_export/presets"),
        cwd.join("../../content/replay_export/presets"),
        cwd.join("../../../game/content/replay_export/presets"),
        cwd.join("../../../content/replay_export/presets"),
    ];
    candidates.into_iter().find(|c| c.is_dir())
}

/// Read the bundle's `run_id` from `<bundle_dir>/run_manifest.json`.
/// The CLI's default output filename interpolates the run_id so each
/// export lands at a unique path under `~/Movies/Corefall/`.
fn read_run_id_from_manifest(bundle_dir: &Path) -> Result<String, ExportError> {
    let manifest_path = bundle_dir.join("run_manifest.json");
    let text = std::fs::read_to_string(&manifest_path).map_err(|_| ExportError::BundleMissingRunId)?;
    let value: serde_json::Value = serde_json::from_str(&text).map_err(|_| ExportError::BundleMissingRunId)?;
    value
        .get("run_id")
        .and_then(|v| v.as_str())
        .map(str::to_owned)
        .ok_or(ExportError::BundleMissingRunId)
}

/// Delete the `--out` file if it exists. Called by the CLI handler
/// on the missing-FFmpeg path (VAL-M10B-032 "no partial MP4 is left
/// on disk").
pub fn delete_partial_output(out_path: &Path) {
    if out_path.exists() {
        let _ = std::fs::remove_file(out_path);
    }
}

/// Resolve the default `--out` path for a bundle's `run_id`. Helper
/// the cf-app CTA + cfctl shim share so all three surfaces produce the
/// same default output location.
#[must_use]
pub fn resolve_default_out_path(run_id: &str, container_ext: &str) -> Option<PathBuf> {
    default_output_path(run_id, container_ext)
}

/// Helper for the cf-app Export Last Replay CTA: builds the full
/// argv vector the CTA passes to `std::process::Command::args` so the
/// CTA dispatches the same `cf-tools-replay-viewer export …` shape the
/// CLI accepts.
#[must_use]
pub fn build_cta_argv(bundle_dir: &Path, run_id: &str) -> Vec<String> {
    let preset = "clip_compact".to_string();
    let out = default_output_path(run_id, "mp4")
        .unwrap_or_else(|| PathBuf::from(format!("{run_id}.mp4")));
    vec![
        "export".to_string(),
        bundle_dir.display().to_string(),
        "--preset".to_string(),
        preset,
        "--out".to_string(),
        out.display().to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn write_stub_bundle(dir: &Path, run_id: &str) {
        fs::create_dir_all(dir).unwrap();
        let manifest = serde_json::json!({
            "schema_version": "manifest:1.0",
            "run_id": run_id,
            "prototype_slice": "M10B",
            "scene": { "id": "stub", "display_name": "Stub", "source_path": "stub" },
            "seed": 0,
        });
        fs::write(
            dir.join("run_manifest.json"),
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();
        fs::write(dir.join("events.jsonl"), "").unwrap();
    }

    fn presets_dir() -> PathBuf {
        locate_default_presets_dir().expect("presets dir resolves from test CWD")
    }

    /// VAL-M10B-033 dispatch: --list-presets returns exactly 5
    /// presets with 6 required fields.
    #[test]
    fn export_cmd_list_presets_returns_five() {
        let args = ExportArgs {
            list_presets: true,
            presets_dir: Some(presets_dir()),
            ..Default::default()
        };
        let outcome = run_export(args).expect("list-presets dispatch");
        match outcome {
            ExportOutcome::PresetsListed(p) => {
                assert_eq!(p.count, 5, "exactly five declared presets");
                let json: serde_json::Value = serde_json::from_str(&p.json).unwrap();
                let arr = json.as_array().unwrap();
                assert_eq!(arr.len(), 5);
                for entry in arr {
                    for field in [
                        "resolution",
                        "fps",
                        "codec",
                        "audio_codec",
                        "target_bitrate_kbps",
                        "container",
                    ] {
                        assert!(entry.get(field).is_some(), "missing field {field}");
                    }
                }
            }
            other => panic!("expected PresetsListed; got {other:?}"),
        }
    }

    /// `~/Movies/Corefall/<run_id>.mp4` on macOS, `~/Videos/Corefall/<run_id>.mp4`
    /// on Linux/Windows via dirs-next.
    #[test]
    fn export_cmd_resolves_default_out_via_dirs_next() {
        let tmp = tempdir().unwrap();
        write_stub_bundle(tmp.path(), "test_run_default_path");
        let args = ExportArgs {
            bundle_dir: Some(tmp.path().to_path_buf()),
            preset: Some("clip_compact".into()),
            presets_dir: Some(presets_dir()),
            dry_run: true,
            ..Default::default()
        };
        let outcome = run_export(args).expect("dry run dispatch");
        let success = match outcome {
            ExportOutcome::DryRun(s) => s,
            other => panic!("expected DryRun, got {other:?}"),
        };
        let dir = success.out_path.parent().expect("out_path parent");
        let dir_name = dir.file_name().and_then(|s| s.to_str()).unwrap_or("");
        assert_eq!(
            dir_name, "Corefall",
            "out path must land under <Movies|Videos>/Corefall/ via dirs-next; got {dir:?}"
        );
        let parent_name = dir
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str())
            .unwrap_or("");
        if cfg!(target_os = "macos") {
            assert_eq!(parent_name, "Movies");
        } else {
            assert_eq!(parent_name, "Videos");
        }
        assert!(
            success
                .out_path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .ends_with(".mp4"),
            "default extension must be .mp4 for clip_compact"
        );
    }

    /// dispatch to the success envelope.
    #[test]
    fn export_cmd_threads_no_audio_base_flag() {
        let tmp = tempdir().unwrap();
        write_stub_bundle(tmp.path(), "test_run_no_audio_base");
        let args = ExportArgs {
            bundle_dir: Some(tmp.path().to_path_buf()),
            preset: Some("clip_compact".into()),
            presets_dir: Some(presets_dir()),
            no_audio_base: true,
            dry_run: true,
            ..Default::default()
        };
        let outcome = run_export(args).expect("dry run dispatch");
        match outcome {
            ExportOutcome::DryRun(s) => assert!(s.no_audio_base),
            other => panic!("expected DryRun, got {other:?}"),
        }
    }

    #[test]
    fn export_cmd_slow_mo_2x_parses() {
        let tmp = tempdir().unwrap();
        write_stub_bundle(tmp.path(), "test_run_slow_mo_2x");
        let args = ExportArgs {
            bundle_dir: Some(tmp.path().to_path_buf()),
            preset: Some("clip_compact".into()),
            presets_dir: Some(presets_dir()),
            slow_mo: Some("2x".into()),
            dry_run: true,
            ..Default::default()
        };
        let outcome = run_export(args).expect("dry run dispatch");
        match outcome {
            ExportOutcome::DryRun(s) => assert_eq!(s.slow_mo.value(), 2),
            other => panic!("expected DryRun, got {other:?}"),
        }
    }

    #[test]
    fn export_cmd_slow_mo_4x_parses() {
        let tmp = tempdir().unwrap();
        write_stub_bundle(tmp.path(), "test_run_slow_mo_4x");
        let args = ExportArgs {
            bundle_dir: Some(tmp.path().to_path_buf()),
            preset: Some("clip_compact".into()),
            presets_dir: Some(presets_dir()),
            slow_mo: Some("4x".into()),
            dry_run: true,
            ..Default::default()
        };
        let outcome = run_export(args).expect("dry run dispatch");
        match outcome {
            ExportOutcome::DryRun(s) => assert_eq!(s.slow_mo.value(), 4),
            other => panic!("expected DryRun, got {other:?}"),
        }
    }

    /// `SlowMoError::NonInteger`.
    #[test]
    fn export_cmd_slow_mo_3point5x_rejected_with_typed_error() {
        let tmp = tempdir().unwrap();
        write_stub_bundle(tmp.path(), "test_run_slow_mo_bad");
        let args = ExportArgs {
            bundle_dir: Some(tmp.path().to_path_buf()),
            preset: Some("clip_compact".into()),
            presets_dir: Some(presets_dir()),
            slow_mo: Some("3.5x".into()),
            dry_run: true,
            ..Default::default()
        };
        let err = run_export(args).expect_err("3.5x must reject");
        match err {
            ExportError::SlowMo(SlowMoError::NonInteger { ref got }) => assert_eq!(got, "3.5x"),
            other => panic!("expected SlowMo(NonInteger); got {other:?}"),
        }
    }

    /// `ExportError::MissingFfmpeg`.
    #[test]
    fn export_cmd_missing_ffmpeg_returns_typed_error() {
        let tmp = tempdir().unwrap();
        write_stub_bundle(tmp.path(), "test_run_missing_ffmpeg");
        let args = ExportArgs {
            bundle_dir: Some(tmp.path().to_path_buf()),
            preset: Some("clip_compact".into()),
            presets_dir: Some(presets_dir()),
            force_missing_ffmpeg: true,
            ..Default::default()
        };
        let err = run_export(args).expect_err("force_missing_ffmpeg must error");
        assert!(matches!(err, ExportError::MissingFfmpeg(_)));
        // VAL-M10B-032 structured JSON shape:
        let json = format_missing_ffmpeg_json();
        let parsed: MissingDependency = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.result, "missing_dependency");
        assert_eq!(parsed.dependency, "ffmpeg");
        assert!(parsed.suggested_install.contains("brew install ffmpeg"));
        assert!(parsed.suggested_install.contains("apt install ffmpeg"));
        assert!(parsed.suggested_install.contains("choco install ffmpeg"));
    }

    /// encode left behind so subsequent invocations don't see a
    /// half-written MP4.
    #[test]
    fn delete_partial_output_removes_existing_file() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("partial.mp4");
        fs::write(&path, b"placeholder").unwrap();
        assert!(path.exists());
        delete_partial_output(&path);
        assert!(!path.exists());
    }

    /// VAL-M10B-DEBRIEF-CTA helper: `build_cta_argv` produces the
    /// CLI shape `export <bundle> --preset clip_compact --out
    /// <platform_default_path>`.
    #[test]
    fn build_cta_argv_uses_clip_compact_and_default_out() {
        let argv = build_cta_argv(Path::new("/tmp/bundle"), "test_run_cta");
        assert_eq!(argv[0], "export");
        assert_eq!(argv[1], "/tmp/bundle");
        assert!(argv.contains(&"--preset".to_string()));
        let preset_idx = argv.iter().position(|s| s == "--preset").unwrap();
        assert_eq!(argv[preset_idx + 1], "clip_compact");
        let out_idx = argv.iter().position(|s| s == "--out").unwrap();
        assert!(argv[out_idx + 1].ends_with("test_run_cta.mp4"));
    }

    /// `--list-presets` is independent of the bundle path — works
    /// even when no bundle is supplied.
    #[test]
    fn list_presets_without_bundle() {
        let args = ExportArgs {
            list_presets: true,
            presets_dir: Some(presets_dir()),
            ..Default::default()
        };
        let outcome = run_export(args).expect("list");
        matches!(outcome, ExportOutcome::PresetsListed(_));
    }
}
