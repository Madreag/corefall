//! Run-bundle writer: turns recorder + manifest + perf samples into
//! `events.jsonl`, `run_manifest.json`, `summary.json`, `notes.md`.

use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::{
    manifest::RunManifest,
    recorder::Recorder,
    summary::{
        ArtifactItem, ArtifactsBlock, PerfSample, PerformanceBlock, RecorderBlock, RunSummary, TestRecord, VolumeBlock,
    },
    SUMMARY_SCHEMA_VERSION,
};

/// Final state passed into `write_run_bundle`.
pub struct BundleInputs<'a> {
    pub recorder: &'a Recorder,
    pub manifest: RunManifest,
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
    pub exit_code: i32,
    pub result: String,
    pub blockers: Vec<String>,
    pub next_actions: Vec<String>,
    pub tests: Vec<TestRecord>,
    pub artifacts: Vec<ArtifactItem>,
    pub assumptions_tested: Vec<String>,
    pub good: Vec<String>,
    pub bad: Vec<String>,
    pub meh: Vec<String>,
    pub evidence_links: Vec<String>,
    pub notes_extra: String,
    pub perf: Option<PerfSample>,
}

#[derive(Debug, thiserror::Error)]
pub enum BundleError {
    #[error("io error writing {path:?}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("serde error writing {path:?}: {source}")]
    Serde {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
}

pub fn run_bundle_dir_basename(run_id: &str) -> String {
    run_id.to_string()
}

/// Write `events.jsonl`, `run_manifest.json`, `summary.json`, `notes.md` into `bundle_dir`.
pub fn write_run_bundle(bundle_dir: &Path, inputs: BundleInputs<'_>) -> Result<RunSummary, BundleError> {
    fs::create_dir_all(bundle_dir).map_err(|source| BundleError::Io {
        path: bundle_dir.to_path_buf(),
        source,
    })?;

    let events_path = bundle_dir.join("events.jsonl");
    let mut events_writer = BufWriter::new(File::create(&events_path).map_err(|source| BundleError::Io {
        path: events_path.clone(),
        source,
    })?);
    let events = inputs.recorder.snapshot_events();
    let mut bytes_written: u64 = 0;
    for event in &events {
        let line = serde_json::to_string(event).map_err(|source| BundleError::Serde {
            path: events_path.clone(),
            source,
        })?;
        events_writer
            .write_all(line.as_bytes())
            .and_then(|_| events_writer.write_all(b"\n"))
            .map_err(|source| BundleError::Io {
                path: events_path.clone(),
                source,
            })?;
        bytes_written += line.len() as u64 + 1;
    }
    events_writer.flush().map_err(|source| BundleError::Io {
        path: events_path.clone(),
        source,
    })?;

    let manifest_path = bundle_dir.join("run_manifest.json");
    write_pretty_json(&manifest_path, &inputs.manifest)?;

    let counts = inputs.recorder.counts();
    let (first_tick, last_tick) = inputs.recorder.first_last_tick();
    let final_checksum = inputs.recorder.final_checksum_hex();
    let checksum_event_count = inputs.recorder.checksum_event_count();
    let duration_sec = (inputs.ended_at - inputs.started_at).num_milliseconds() as f64 / 1000.0;

    let notes_path = bundle_dir.join("notes.md");
    let notes = render_notes(
        &inputs.manifest,
        &inputs.assumptions_tested,
        &inputs.good,
        &inputs.bad,
        &inputs.meh,
        &inputs.evidence_links,
        &inputs.next_actions,
        &inputs.notes_extra,
    );
    fs::write(&notes_path, notes).map_err(|source| BundleError::Io {
        path: notes_path,
        source,
    })?;

    let performance = match inputs.perf {
        Some(perf) => PerformanceBlock::from(perf),
        None => PerformanceBlock {
            avg_frame_ms: 0.0,
            p99_frame_ms: 0.0,
            avg_tick_ms: 0.0,
            p99_tick_ms: 0.0,
            ticks_run: last_tick.unwrap_or(0),
            wall_seconds: duration_sec,
            tick_rate_hz: inputs.manifest.tick_rate_hz,
            terrain: None,
            subsystems: None,
        },
    };
    let summary = RunSummary {
        schema_version: SUMMARY_SCHEMA_VERSION.to_string(),
        run_id: inputs.manifest.run_id.clone(),
        manifest_run_id: inputs.manifest.run_id.clone(),
        duration_sec,
        result: inputs.result,
        ended_at_utc: inputs.ended_at.to_rfc3339(),
        exit_code: inputs.exit_code,
        event_counts: counts,
        volume: VolumeBlock {
            events_jsonl_bytes: bytes_written,
            event_lines: events.len() as u64,
        },
        performance,
        artifacts: ArtifactsBlock {
            items: inputs.artifacts,
        },
        blockers: inputs.blockers,
        next_actions: inputs.next_actions,
        tests: inputs.tests,
        final_sim_checksum: final_checksum,
        checksum_event_count,
        first_tick,
        last_tick,
        recorder: RecorderBlock {
            peak_buffer_depth: inputs.recorder.peak_buffer_depth(),
            dropped_cosmetic: inputs.recorder.dropped_cosmetic_count(),
            dropped_gameplay: inputs.recorder.dropped_gameplay_count(),
            dropped_total: inputs.recorder.dropped_count(),
        },
    };

    let summary_path = bundle_dir.join("summary.json");
    write_pretty_json(&summary_path, &summary)?;

    Ok(summary)
}

fn write_pretty_json<T: Serialize>(path: &Path, value: &T) -> Result<(), BundleError> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|source| BundleError::Serde {
        path: path.to_path_buf(),
        source,
    })?;
    fs::write(path, bytes).map_err(|source| BundleError::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn render_notes(
    manifest: &RunManifest,
    assumptions: &[String],
    good: &[String],
    bad: &[String],
    meh: &[String],
    evidence_links: &[String],
    next_actions: &[String],
    extra: &str,
) -> String {
    let mut s = String::new();
    s.push_str(&format!(
        "# {} Run Notes — {}\n\n",
        manifest.milestone.to_uppercase(),
        manifest.run_id
    ));
    s.push_str("## Assumptions Tested\n");
    for a in assumptions {
        s.push_str(&format!("- {a}\n"));
    }
    if assumptions.is_empty() {
        s.push_str(&format!("- {}\n", "(none recorded)"));
    }
    s.push('\n');

    s.push_str("## Good\n");
    for g in good {
        s.push_str(&format!("- {g}\n"));
    }
    if good.is_empty() {
        s.push_str("- (none recorded)\n");
    }
    s.push('\n');

    s.push_str("## Bad\n");
    for b in bad {
        s.push_str(&format!("- {b}\n"));
    }
    if bad.is_empty() {
        s.push_str("- (none recorded)\n");
    }
    s.push('\n');

    s.push_str("## Meh\n");
    for m in meh {
        s.push_str(&format!("- {m}\n"));
    }
    if meh.is_empty() {
        s.push_str("- (none recorded)\n");
    }
    s.push('\n');

    s.push_str("## Evidence Links\n");
    for link in evidence_links {
        s.push_str(&format!("- {link}\n"));
    }
    if evidence_links.is_empty() {
        s.push_str("- events.jsonl\n- summary.json\n- run_manifest.json\n");
    }
    s.push('\n');

    s.push_str("## Next Actions\n");
    for n in next_actions {
        s.push_str(&format!("- {n}\n"));
    }
    if next_actions.is_empty() {
        s.push_str("- (none recorded)\n");
    }
    if !extra.is_empty() {
        s.push('\n');
        s.push_str(extra);
        if !extra.ends_with('\n') {
            s.push('\n');
        }
    }
    s
}
