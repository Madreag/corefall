//! Free helper fns + types extracted from engine.rs.
//!
//! Extracted from engine.rs.

#![allow(unused_imports, dead_code, clippy::too_many_arguments)]

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;

use chrono::{DateTime, Utc};
use serde_json::{json, Value};

use cf_actor::sim::{step as actor_step, ActorSimState, ActorTickOutcome, StepDeps, StepReport};
use cf_actor::{ActorId, ActorWorld, ControlIntent, IntentSource, ItemSlot, Vec2};
use cf_replay::{
    diagnostics, ArtifactItem, BuildInfo, BundleInputs, CapabilitiesBlock, CaptureConfig,
    ChecksumConfig, PerfSample, Recorder, RunManifest, SceneInfo, SettingsBlock, TestRecord,
    CONTROL_SCHEMA_VERSION, EVENT_ENVELOPE_VERSION, MANIFEST_SCHEMA_VERSION, SCENARIO_SCHEMA_VERSION,
};
use cf_sim_core::{
    checksum::{sim_state_v1, CHECKSUM_ALGORITHM, CHECKSUM_SCOPE},
    ids::{iso_hyphen_safe, make_run_id},
    Rng, SimClock, SimConfig, Tick, WallClock,
};

use crate::engine::*;
use crate::scenario::Scenario;
use crate::server::{async_trait, CommandResult, ControlCommand, EngineHandle, SettingsPatch};
use crate::state::{ActorView, ObserveFrame, ObserveSettings, RunStatus};
use crate::{Settings, SCHEMA_VERSION};

/// Build the `summary.json.tests[]` entries from the scenario's
/// `expected_tests` manifest field. Each entry's `result` is exit-code-driven
/// (engine-wide pass/fail), `evidence_event_ids` is the run's first+last event
/// id pair, and `notes` is a stable per-milestone rationale. If the scenario
/// declares no expected tests we synthesize a single milestone-level smoke
/// row so the array is never empty.
pub(crate) fn build_test_records(
    expected_tests: &[String],
    milestone: &str,
    result: &str,
    evidence_event_ids: &[String],
) -> Vec<TestRecord> {
    let normalized = milestone.trim().to_lowercase();
    let notes = match normalized.as_str() {
        "" | "m0" => "M0 fixed-tick smoke + run-bundle parity per spec/native-implementation-backlog.",
        "m1" => "M1 actor controller round-trip (move + jump + aim + fire + reload + select_item).",
        "m1.5" => "M1.5 micro breach fun slice (dig outer wall, kill guard, reach extraction).",
        "m2" => "M2 chunked-terrain dig path (dirt fast / concrete slow / metal_nohook + anchor refused).",
        "m2.5" => {
            "M2.5 micro reactor defense fun slice (dirt-shield strategic choice; reactor protected or destroyed)."
        }
        "m3a" => "M3A event recorder core (snapshot.* + expected_outcome contract + cf-headless replay verifier).",
        _ => "Milestone-scope acceptance per spec/native-implementation-backlog.",
    };
    if expected_tests.is_empty() {
        let id = match normalized.as_str() {
            "" | "m0" => "M0-SMOKE-01",
            "m1" => "M1-SMOKE-01",
            "m1.5" => "M1.5-SMOKE-01",
            "m2" => "M2-SMOKE-01",
            "m2.5" => "M2.5-SMOKE-01",
            "m3a" => "M3A-SMOKE-01",
            _ => "MILESTONE-SMOKE-01",
        };
        return vec![TestRecord {
            id: id.to_string(),
            result: result.to_string(),
            evidence_event_ids: evidence_event_ids.to_vec(),
            notes: Some(notes.to_string()),
        }];
    }
    expected_tests
        .iter()
        .map(|id| TestRecord {
            id: id.clone(),
            result: result.to_string(),
            evidence_event_ids: evidence_event_ids.to_vec(),
            notes: Some(notes.to_string()),
        })
        .collect()
}

/// Discover capture artifacts on disk at run-bundle write time. Returns
/// `(artifacts, evidence_link)` where:
///
/// - `artifacts` lists the recordable items inside `<run>/captures/`:
///   `capture_manifest.json`, `summary_grid.png`, every `grid_NNN.png`, and
///   one `capture_frames` summary entry counting the frame_*.png files.
/// - `evidence_link` is `"captures/"` when any capture artifact is present so
///   `notes.md`'s evidence-link list reflects the on-disk shape.
///
/// `summary_grid.png` may not exist at write_run_bundle time (the cf-e2e
/// composer adds it AFTER cf-app exits); `capture_grid.py` patches
/// `summary.json.artifacts.items[]` post-hoc to add the grid PNGs in that
/// case. This helper covers the in-process path (frames + manifest) and is
/// idempotent with the post-hoc patcher.
pub(crate) fn discover_run_artifacts(run_bundle_dir: &Path) -> (Vec<ArtifactItem>, Option<String>) {
    let captures_dir = run_bundle_dir.join("captures");
    if !captures_dir.is_dir() {
        return (Vec::new(), None);
    }
    let mut items: Vec<ArtifactItem> = Vec::new();
    let manifest_path = captures_dir.join("capture_manifest.json");
    if manifest_path.is_file() {
        items.push(ArtifactItem {
            kind: "capture_manifest".to_string(),
            path: "captures/capture_manifest.json".to_string(),
        });
    }
    let summary_grid = captures_dir.join("summary_grid.png");
    if summary_grid.is_file() {
        items.push(ArtifactItem {
            kind: "summary_grid".to_string(),
            path: "captures/summary_grid.png".to_string(),
        });
        let summary_grid_json = captures_dir.join("summary_grid.json");
        if summary_grid_json.is_file() {
            items.push(ArtifactItem {
                kind: "summary_grid_json".to_string(),
                path: "captures/summary_grid.json".to_string(),
            });
        }
    }
    let mut grids: Vec<String> = Vec::new();
    let mut frames: u64 = 0;
    if let Ok(read_dir) = std::fs::read_dir(&captures_dir) {
        for entry in read_dir.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy().to_string();
            if name_str.starts_with("grid_") && name_str.ends_with(".png") {
                grids.push(name_str);
            } else if name_str.starts_with("frame_") && name_str.ends_with(".png") {
                frames += 1;
            }
        }
    }
    grids.sort();
    for g in grids {
        items.push(ArtifactItem {
            kind: "capture_grid".to_string(),
            path: format!("captures/{g}"),
        });
    }
    if frames > 0 {
        items.push(ArtifactItem {
            kind: "capture_frames".to_string(),
            path: format!("captures/ ({frames} frame_*.png)"),
        });
    }
    let link = if items.is_empty() {
        None
    } else {
        Some("captures/".to_string())
    };
    (items, link)
}

