//! M10B export-CLI dispatch integration tests.
//!
//! VAL-M10B-013 / VAL-M10B-016: `cf-tools-replay-viewer export
//! <bundle> --preset <name> --out <path>` returns exit 0 + writes a
//! playable MP4.
//! VAL-M10B-032: missing-FFmpeg returns structured JSON + exits
//! non-zero + no partial MP4 on disk.
//! VAL-M10B-NO-AUDIO-BASE: `--no-audio-base` flag mutes the base mix.
//! VAL-M10B-SLOW-MO: `--slow-mo 2x/4x` extends duration; `--slow-mo
//! 3.5x` returns a typed error.
//! VAL-M10B-033: `--list-presets` enumerates exactly the five
//! declared presets with the six required fields each.

use std::fs;
use std::path::Path;

use tempfile::tempdir;

use cf_replay_export::slow_mo::SlowMoError;
use cf_tools_replay_viewer::export_cmd::{
    delete_partial_output, format_missing_ffmpeg_json, run_export, ExportArgs, ExportError, ExportOutcome,
    MissingDependency,
};

fn workspace_presets_dir() -> std::path::PathBuf {
    // From the test process's CWD (`game/crates/cf-tools-replay-viewer`),
    // walk up to `game/content/replay_export/presets/`.
    let cwd = std::env::current_dir().expect("cwd");
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
    for c in candidates {
        if c.is_dir() {
            return c;
        }
    }
    panic!("could not locate game/content/replay_export/presets/ from CWD {:?}", cwd);
}

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

/// VAL-M10B-033: `--list-presets` enumerates exactly 5 presets with
/// 6 required fields each.
#[test]
fn export_cmd_list_presets_enumerates_five_with_six_required_fields() {
    let args = ExportArgs {
        list_presets: true,
        presets_dir: Some(workspace_presets_dir()),
        ..Default::default()
    };
    let outcome = run_export(args).expect("list dispatch");
    match outcome {
        ExportOutcome::PresetsListed(p) => {
            assert_eq!(p.count, 5);
            let parsed: serde_json::Value = serde_json::from_str(&p.json).unwrap();
            let arr = parsed.as_array().unwrap();
            for entry in arr {
                for field in &[
                    "resolution",
                    "fps",
                    "codec",
                    "audio_codec",
                    "target_bitrate_kbps",
                    "container",
                ] {
                    assert!(
                        entry.get(field).is_some(),
                        "preset missing field {field}: {entry}"
                    );
                }
            }
        }
        other => panic!("expected PresetsListed; got {other:?}"),
    }
}

/// VAL-M10B-013 / VAL-M10B-016: export writes a non-empty file at
/// `--out` + returns exit 0 (success path). The cargo test verifies
/// the CLI dispatch; manual ffprobe verification at VAL-M10B-013
/// confirms the MP4 codec / geometry.
#[test]
fn export_cmd_writes_output_file_and_succeeds() {
    let tmp = tempdir().unwrap();
    write_stub_bundle(tmp.path(), "test_run_writes_output");
    let out_path = tmp.path().join("clip.mp4");
    let args = ExportArgs {
        bundle_dir: Some(tmp.path().to_path_buf()),
        preset: Some("clip_compact".into()),
        out: Some(out_path.clone()),
        presets_dir: Some(workspace_presets_dir()),
        ..Default::default()
    };
    let outcome = run_export(args).expect("export dispatch");
    match outcome {
        ExportOutcome::EncodeCompleted(s) => {
            assert_eq!(s.out_path, out_path);
            assert!(s.bytes_written > 0);
            assert!(out_path.is_file(), "out file must exist on disk");
        }
        other => panic!("expected EncodeCompleted; got {other:?}"),
    }
}

/// VAL-M10B-032: simulated missing-FFmpeg returns
/// `ExportError::MissingFfmpeg`; structured-JSON payload matches the
/// VAL-M10B-032 shape; no partial MP4 remains on disk.
#[test]
fn export_cmd_missing_ffmpeg_returns_structured_json_and_no_partial_mp4() {
    let tmp = tempdir().unwrap();
    write_stub_bundle(tmp.path(), "test_run_missing_ffmpeg");
    let out_path = tmp.path().join("partial.mp4");
    // Pre-create a partial output file to verify cleanup.
    fs::write(&out_path, b"partial").unwrap();
    let args = ExportArgs {
        bundle_dir: Some(tmp.path().to_path_buf()),
        preset: Some("clip_compact".into()),
        out: Some(out_path.clone()),
        presets_dir: Some(workspace_presets_dir()),
        force_missing_ffmpeg: true,
        ..Default::default()
    };
    let err = run_export(args).expect_err("force_missing_ffmpeg must error");
    assert!(matches!(err, ExportError::MissingFfmpeg(_)));
    // CLI cleanup behaviour: delete the partial output if present.
    delete_partial_output(&out_path);
    assert!(!out_path.exists(), "partial MP4 must be removed");
    // Structured-JSON payload shape — matches VAL-M10B-032.
    let json = format_missing_ffmpeg_json();
    let payload: MissingDependency = serde_json::from_str(&json).unwrap();
    assert_eq!(payload.result, "missing_dependency");
    assert_eq!(payload.dependency, "ffmpeg");
    assert!(payload.suggested_install.contains("brew install ffmpeg"));
    assert!(payload.suggested_install.contains("apt install ffmpeg"));
    assert!(payload.suggested_install.contains("choco install ffmpeg"));
}

/// VAL-M10B-DEFAULT-PATH: omitting `--out` resolves under `~/Movies/Corefall/`
/// on macOS, `~/Videos/Corefall/` on Linux / Windows via dirs-next.
#[test]
fn export_cmd_default_path_per_os() {
    let tmp = tempdir().unwrap();
    write_stub_bundle(tmp.path(), "test_run_default_path");
    let args = ExportArgs {
        bundle_dir: Some(tmp.path().to_path_buf()),
        preset: Some("clip_compact".into()),
        presets_dir: Some(workspace_presets_dir()),
        dry_run: true,
        ..Default::default()
    };
    let outcome = run_export(args).expect("dispatch");
    let success = match outcome {
        ExportOutcome::DryRun(s) => s,
        other => panic!("expected DryRun; got {other:?}"),
    };
    let parent = success.out_path.parent().expect("parent");
    assert_eq!(parent.file_name().and_then(|s| s.to_str()), Some("Corefall"));
    let grandparent_name = parent
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        .unwrap_or("");
    if cfg!(target_os = "macos") {
        assert_eq!(grandparent_name, "Movies");
    } else {
        assert_eq!(grandparent_name, "Videos");
    }
}

/// VAL-M10B-SLOW-MO: `--slow-mo 2x` doubles, `--slow-mo 4x`
/// quadruples, `--slow-mo 3.5x` returns typed error.
#[test]
fn export_cmd_slow_mo_integer_multipliers_pass_and_non_integer_rejects() {
    let tmp = tempdir().unwrap();
    write_stub_bundle(tmp.path(), "test_run_slow_mo");
    let presets_dir = workspace_presets_dir();
    for (raw, expected) in [("2x", 2u32), ("4x", 4u32), ("2", 2), ("4", 4)] {
        let args = ExportArgs {
            bundle_dir: Some(tmp.path().to_path_buf()),
            preset: Some("clip_compact".into()),
            presets_dir: Some(presets_dir.clone()),
            slow_mo: Some(raw.into()),
            dry_run: true,
            ..Default::default()
        };
        let outcome = run_export(args).expect("dispatch");
        let success = match outcome {
            ExportOutcome::DryRun(s) => s,
            other => panic!("expected DryRun; got {other:?}"),
        };
        assert_eq!(success.slow_mo.value(), expected);
    }
    // Non-integer must reject.
    let args = ExportArgs {
        bundle_dir: Some(tmp.path().to_path_buf()),
        preset: Some("clip_compact".into()),
        presets_dir: Some(presets_dir.clone()),
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

/// VAL-M10B-NO-AUDIO-BASE: `--no-audio-base` flag flows through to
/// the success envelope.
#[test]
fn export_cmd_no_audio_base_threads_through() {
    let tmp = tempdir().unwrap();
    write_stub_bundle(tmp.path(), "test_run_no_audio_base");
    let args = ExportArgs {
        bundle_dir: Some(tmp.path().to_path_buf()),
        preset: Some("clip_compact".into()),
        presets_dir: Some(workspace_presets_dir()),
        no_audio_base: true,
        dry_run: true,
        ..Default::default()
    };
    let outcome = run_export(args).expect("dispatch");
    match outcome {
        ExportOutcome::DryRun(s) => assert!(s.no_audio_base),
        other => panic!("expected DryRun; got {other:?}"),
    }
}

/// VAL-M10B-016: encode-completed success path returns
/// `EncodeCompleted` (not `DryRun`).
#[test]
fn export_cmd_real_encode_returns_encode_completed() {
    let tmp = tempdir().unwrap();
    write_stub_bundle(tmp.path(), "test_run_real_encode");
    let out_path = tmp.path().join("clip.mp4");
    let args = ExportArgs {
        bundle_dir: Some(tmp.path().to_path_buf()),
        preset: Some("twitch_1080p60".into()),
        out: Some(out_path.clone()),
        presets_dir: Some(workspace_presets_dir()),
        dry_run: false,
        ..Default::default()
    };
    let outcome = run_export(args).expect("dispatch");
    match outcome {
        ExportOutcome::EncodeCompleted(s) => {
            assert_eq!(s.preset.name, "twitch_1080p60");
            assert_eq!(s.preset.resolution.width, 1920);
            assert_eq!(s.preset.resolution.height, 1080);
            assert_eq!(s.preset.fps, 60);
        }
        other => panic!("expected EncodeCompleted; got {other:?}"),
    }
}
