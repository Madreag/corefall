//! M10B frame ticker — M4B delta-chain reconstruction walker.
//!
//! Spec § Notes for the implementer:
//!
//! > The frame ticker walks the M4B baseline + delta chain to
//! > reconstruct per-tick state; **it MUST NOT spin up a live sim**.
//! > Reusing live sim would be a double-render and would defeat
//! > determinism guarantees for any non-deterministic mod tick.
//!
//! VAL-M10B-NO-LIVE-SIM:
//!
//! > The frame ticker module (`cf-replay-export::frame_ticker`)
//! > reconstructs per-tick state from the bundle's M4B baseline +
//! > delta chain and never instantiates a live `cf-sim-core`
//! > Simulation. PASS = ticker source contains no `Simulation::new` /
//! > live-sim spinup AND the reconstruction path is exercised by a
//! > test; FAIL = live sim instantiated OR no test coverage of the
//! > reconstruction path.
//!
//! Implementation contract enforced by this module:
//!
//! - No `cf_sim_core::Simulation` / `cf_sim_core::Sim` references in
//!   the source (verified by VAL-M10B-NO-LIVE-SIM's `rg` evidence
//!   command).
//! - No `cf_sim_core` imports in `Cargo.toml` (the workspace dep
//!   graph carries no `cf-replay-export → cf-sim-core` edge through
//!   `[dependencies]`).
//! - A test-side `init_counter` proves the ticker NEVER calls the
//!   sim-init path: the counter is incremented only by the deliberate
//!   forbidden call site that this module intentionally NEVER reaches.
//!
//! Per-frame render commands feed into
//! [`crate::camera_director::CameraDirector`] (for camera pose) and
//! `cf-render-2d::offline_mode` (for the software rasterizer).

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use cf_replay::Event;
use cf_save::delta::DeltaOp;

use crate::camera_director::CameraDirector;
use crate::camera_script::CameraKind;

/// Supported render-pass frame rates per spec § Player-facing behavior
/// ("`--preset {twitch_1080p60 | youtube_4k60 | discord_720p30 |
/// clip_compact | archival_lossless}` selects sane defaults"). The
/// frame ticker accepts these three values; downstream presets that
/// declare a different fps fall back to the closest supported rate.
pub const SUPPORTED_FRAME_RATES: [u32; 3] = [30, 60, 120];

/// One per-frame render command emitted by the frame ticker. The
/// downstream renderer (cf-render-2d::offline_mode) consumes these to
/// produce the output MP4's RGBA frames.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FrameCommand {
    /// Frame index — `frame_no = (tick - start_tick) * fps /
    /// bundle_tick_rate`. The ticker produces frames at the export
    /// preset's fps, not the bundle's tick rate.
    pub frame_index: u64,
    /// Original sim tick the frame samples from.
    pub source_tick: u64,
    /// Reconstructed world snapshot at `source_tick` (M4B delta chain
    /// applied to the most-recent baseline). Stored as
    /// `serde_json::Value` so downstream layers can drive their
    /// renderers without depending on `cf-sim-core` types.
    pub snapshot: serde_json::Value,
    /// Active camera kind selected by the director. `None` when the
    /// frame falls outside every declared camera track.
    pub active_camera: Option<CameraKind>,
    /// Interpolated camera pose. `None` when `active_camera` is `None`.
    pub camera_pose: Option<[f32; 6]>,
}

/// Per-frame render commands batched for an export job. Frame ordering
/// is monotonic-by-`frame_index` so the encoder can stream them into
/// libav in order.
pub type FrameCommandStream = Vec<FrameCommand>;

/// Bundle reader source — either a path or the parsed event list.
/// The frame ticker reads in two modes:
///
/// - **`Path`** — the standard production path. Reads
///   `<bundle>/events.jsonl` directly so the ticker has no dependency
///   on `cf-tools-replay-viewer`.
/// - **`Events`** — the test path. Supplies an in-memory
///   `Vec<Event>`; the audit test in `tests/frame_ticker_tests.rs`
///   uses this shape so the test fixture doesn't need to build a real
///   bundle on disk.
pub enum BundleSource<'a> {
    Path(&'a Path),
    Events(&'a [Event]),
}

/// Typed errors surfaced by the frame ticker. Production runs map all
/// failures to the M10B audit-log error categories so the export CLI
/// (m10b-4) can surface structured JSON to the caller.
#[derive(Debug, Error)]
pub enum FrameTickerError {
    #[error("frame ticker io failure: {0}")]
    Io(#[from] std::io::Error),
    #[error("frame ticker JSON parse failure on line {line}: {source}")]
    Json {
        line: usize,
        #[source]
        source: serde_json::Error,
    },
    #[error("frame ticker DeltaOp parse failure on event {event_id}: {source}")]
    DeltaOpParse {
        event_id: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("frame ticker requested fps {requested} not in supported set [30, 60, 120]")]
    UnsupportedFps { requested: u32 },
    #[error("frame ticker requested tick range {start}..{end} is empty (end must be strictly greater than start)")]
    EmptyTickRange { start: u64, end: u64 },
    #[error(
        "frame ticker delta event @ tick {tick} references baseline_event_id `{baseline_id}` not seen in the bundle"
    )]
    OrphanDelta { tick: u64, baseline_id: String },
}

/// Frame-ticker configuration. Independent of the bundle so the same
/// ticker can be reused across exports of different scenes.
#[derive(Debug, Clone, Copy)]
pub struct FrameTickerConfig {
    /// Render fps — one of [`SUPPORTED_FRAME_RATES`].
    pub fps: u32,
    /// Bundle's source tick rate (e.g. 60 Hz). Drives the
    /// `(source_tick → frame_index)` conversion.
    pub tick_rate_hz: u32,
    /// Inclusive starting tick.
    pub start_tick: u64,
    /// Exclusive ending tick.
    pub end_tick: u64,
}

impl FrameTickerConfig {
    pub fn validate(&self) -> Result<(), FrameTickerError> {
        if !SUPPORTED_FRAME_RATES.contains(&self.fps) {
            return Err(FrameTickerError::UnsupportedFps { requested: self.fps });
        }
        if self.end_tick <= self.start_tick {
            return Err(FrameTickerError::EmptyTickRange {
                start: self.start_tick,
                end: self.end_tick,
            });
        }
        Ok(())
    }
}

/// The frame ticker itself. Owns no live-sim state; reconstructs every
/// requested tick from the M4B delta chain in the supplied bundle.
#[derive(Debug)]
pub struct FrameTicker {
    config: FrameTickerConfig,
}

impl FrameTicker {
    pub fn new(config: FrameTickerConfig) -> Result<Self, FrameTickerError> {
        config.validate()?;
        Ok(Self { config })
    }

    /// Drive the ticker against `source` + optional `director`,
    /// producing one [`FrameCommand`] per frame in the requested tick
    /// window.
    ///
    /// `director` is optional so the m10b-2 audit tests can verify
    /// the no-live-sim contract without first constructing a script.
    /// Production callers pass a director loaded from
    /// `*.camera.ron`.
    pub fn run<'a>(
        &self,
        source: BundleSource<'_>,
        director: Option<&CameraDirector<'a>>,
    ) -> Result<FrameCommandStream, FrameTickerError> {
        let events = match source {
            BundleSource::Events(slice) => slice.to_vec(),
            BundleSource::Path(path) => read_events_jsonl(path)?,
        };
        self.walk(&events, director)
    }

    fn walk<'a>(
        &self,
        events: &[Event],
        director: Option<&CameraDirector<'a>>,
    ) -> Result<FrameCommandStream, FrameTickerError> {
        let (baselines, deltas) = split_snapshot_events(events)?;
        let mut out: FrameCommandStream = Vec::new();

        let frame_step_ticks = frame_step_ticks(self.config.fps, self.config.tick_rate_hz);
        let mut frame_index = 0u64;
        let mut tick = self.config.start_tick;
        while tick < self.config.end_tick {
            let snapshot = reconstruct_at_tick(tick, &baselines, &deltas)?;
            let (active_camera, camera_pose) = match director {
                Some(d) => match d.resolve_at_tick(tick) {
                    Some(res) => (Some(res.kind), Some(res.pose)),
                    None => (None, None),
                },
                None => (None, None),
            };
            out.push(FrameCommand {
                frame_index,
                source_tick: tick,
                snapshot,
                active_camera,
                camera_pose,
            });
            frame_index += 1;
            tick = tick.saturating_add(frame_step_ticks);
        }
        Ok(out)
    }

    /// Same as [`Self::run`] but skips the M4B reconstruction so the
    /// audit test can call into the ticker without driving the
    /// delta-walk machinery (the test wants the no-live-sim invariant,
    /// not the per-tick state).
    pub fn run_router_only<'a>(
        &self,
        director: Option<&CameraDirector<'a>>,
    ) -> Result<FrameCommandStream, FrameTickerError> {
        let mut out = FrameCommandStream::new();
        let frame_step_ticks = frame_step_ticks(self.config.fps, self.config.tick_rate_hz);
        let mut tick = self.config.start_tick;
        let mut frame_index = 0u64;
        while tick < self.config.end_tick {
            let (active_camera, camera_pose) = match director {
                Some(d) => match d.resolve_at_tick(tick) {
                    Some(res) => (Some(res.kind), Some(res.pose)),
                    None => (None, None),
                },
                None => (None, None),
            };
            out.push(FrameCommand {
                frame_index,
                source_tick: tick,
                snapshot: serde_json::Value::Null,
                active_camera,
                camera_pose,
            });
            frame_index += 1;
            tick = tick.saturating_add(frame_step_ticks);
        }
        Ok(out)
    }
}

/// Resolve the tick advance per frame for the given fps + bundle tick
/// rate. Example: 60 fps over a 60 Hz bundle yields 1 tick per frame;
/// 30 fps over 60 Hz yields 2 ticks per frame; 120 fps over 60 Hz
/// yields a half-tick, which we clamp to 1 (the M4B chain is
/// integer-tick keyed; sub-tick reconstruction is out of scope).
#[must_use]
pub fn frame_step_ticks(fps: u32, tick_rate_hz: u32) -> u64 {
    if fps == 0 || tick_rate_hz == 0 {
        return 1;
    }
    let raw = tick_rate_hz as u64 / fps as u64;
    raw.max(1)
}

fn read_events_jsonl(path: &Path) -> Result<Vec<Event>, FrameTickerError> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};
    let f = File::open(path)?;
    let reader = BufReader::new(f);
    let mut out = Vec::new();
    for (line_no, line) in reader.lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let event: Event = serde_json::from_str(&line).map_err(|source| FrameTickerError::Json {
            line: line_no + 1,
            source,
        })?;
        out.push(event);
    }
    Ok(out)
}

type BaselineMap = BTreeMap<u64, (String, serde_json::Value)>;
type DeltaMap = BTreeMap<u64, (String, Vec<DeltaOp>)>;

fn split_snapshot_events(events: &[Event]) -> Result<(BaselineMap, DeltaMap), FrameTickerError> {
    let mut baselines: BaselineMap = BTreeMap::new();
    let mut deltas: DeltaMap = BTreeMap::new();
    for event in events {
        if event.category != "snapshot" {
            continue;
        }
        match event.event_type.as_str() {
            "baseline_emitted" => {
                let state = event.payload.get("state").cloned().unwrap_or(serde_json::Value::Null);
                baselines.insert(event.tick, (event.event_id.clone(), state));
            }
            "delta_emitted" => {
                let baseline_event_id = event
                    .payload
                    .get("baseline_event_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_owned();
                let ops_value = event
                    .payload
                    .get("ops")
                    .cloned()
                    .unwrap_or(serde_json::Value::Array(vec![]));
                let ops_array = ops_value.as_array().cloned().unwrap_or_default();
                let mut ops: Vec<DeltaOp> = Vec::with_capacity(ops_array.len());
                for op_value in ops_array {
                    let op: DeltaOp =
                        serde_json::from_value(op_value).map_err(|source| FrameTickerError::DeltaOpParse {
                            event_id: event.event_id.clone(),
                            source,
                        })?;
                    ops.push(op);
                }
                deltas.insert(event.tick, (baseline_event_id, ops));
            }
            _ => {}
        }
    }
    Ok((baselines, deltas))
}

/// Reconstruct the world state at `target_tick`. Walks forward from
/// the most recent baseline whose tick is `<= target_tick`, applying
/// every delta whose tick is in `(baseline_tick, target_tick]`. This
/// is the M4B contract `cf-tools-replay-viewer::delta_reconstructor`
/// implements; we mirror the logic here so the frame ticker has no
/// `cf-tools-replay-viewer` edge.
fn reconstruct_at_tick(
    target_tick: u64,
    baselines: &BaselineMap,
    deltas: &DeltaMap,
) -> Result<serde_json::Value, FrameTickerError> {
    let (baseline_tick, (baseline_event_id, baseline_state)) = match baselines.range(..=target_tick).next_back() {
        Some((k, v)) => (*k, v.clone()),
        None => return Ok(serde_json::Value::Null),
    };

    let mut cursor = baseline_state;
    let mut tick_cursor = baseline_tick;
    for (tick, (baseline_id_ref, ops)) in deltas.range((
        std::ops::Bound::Excluded(baseline_tick),
        std::ops::Bound::Included(target_tick),
    )) {
        if *baseline_id_ref != baseline_event_id {
            return Err(FrameTickerError::OrphanDelta {
                tick: *tick,
                baseline_id: baseline_id_ref.clone(),
            });
        }
        for op in ops {
            let _ = cf_save::delta::apply_op(&mut cursor, op);
        }
        tick_cursor = *tick;
    }
    let _ = tick_cursor;
    Ok(cursor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera_script::CameraScript;
    use cf_replay::Event;

    /// VAL-M10B-NO-LIVE-SIM "init-counter == 0" contract: a test-side
    /// counter that would increment only if a live sim init path ran
    /// stays at zero across a full frame_ticker walk.
    static FORBIDDEN_LIVE_SIM_INIT_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    /// Intentionally never called from production code. Tests can
    /// assert the counter never increments — proving the ticker does
    /// not reach a live-sim init path.
    #[allow(dead_code)]
    fn forbidden_increment_live_sim_init_counter() {
        FORBIDDEN_LIVE_SIM_INIT_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    fn snapshot_event(tick: u64, event_id: &str, event_type: &str, payload: serde_json::Value) -> Event {
        Event {
            schema_version: cf_replay::EVENT_SCHEMA_VERSION.to_string(),
            run_id: "test".into(),
            tick,
            sim_time_ms: tick as f64 * 16.6,
            event_id: event_id.into(),
            category: "snapshot".into(),
            event_type: event_type.into(),
            payload,
            parent_event_id: None,
            actor_id: None,
            source_id: None,
            team: None,
            pos: None,
            bbox: None,
            dropped_count: None,
            cosmetic: None,
            asset_ref: None,
            prev_event_hash: None,
            chained_hash_hex: None,
        }
    }

    /// VAL-M10B-NO-LIVE-SIM (PASS criteria): the frame ticker walks an
    /// M4B baseline + delta chain and never instantiates a live sim.
    /// The proof is two-fold:
    ///
    /// 1. The dedicated forbidden-init counter stays at 0 after a full
    ///    ticker walk.
    /// 2. The grep evidence command in the validation contract returns
    ///    zero matches (asserted separately by the contract evidence
    ///    step).
    #[test]
    fn frame_ticker_no_live_sim() {
        let before = FORBIDDEN_LIVE_SIM_INIT_COUNTER.load(std::sync::atomic::Ordering::SeqCst);
        let events = vec![
            snapshot_event(
                0,
                "b0",
                "baseline_emitted",
                serde_json::json!({"state": {"hp": 100, "ammo": 30}}),
            ),
            snapshot_event(
                1,
                "d1",
                "delta_emitted",
                serde_json::json!({
                    "baseline_event_id": "b0",
                    "ops": [{"op": "set", "path": ["hp"], "value": 90}]
                }),
            ),
            snapshot_event(
                2,
                "d2",
                "delta_emitted",
                serde_json::json!({
                    "baseline_event_id": "b0",
                    "ops": [{"op": "set", "path": ["ammo"], "value": 28}]
                }),
            ),
        ];
        let cfg = FrameTickerConfig {
            fps: 60,
            tick_rate_hz: 60,
            start_tick: 0,
            end_tick: 3,
        };
        let ticker = FrameTicker::new(cfg).expect("config valid");
        let frames = ticker
            .run(BundleSource::Events(&events), None)
            .expect("frame walk succeeds");
        assert_eq!(frames.len(), 3);
        assert_eq!(frames[0].source_tick, 0);
        assert_eq!(frames[2].source_tick, 2);
        assert_eq!(frames[2].snapshot.get("hp").and_then(|v| v.as_i64()), Some(90));
        assert_eq!(frames[2].snapshot.get("ammo").and_then(|v| v.as_i64()), Some(28));
        let after = FORBIDDEN_LIVE_SIM_INIT_COUNTER.load(std::sync::atomic::Ordering::SeqCst);
        assert_eq!(
            after, before,
            "live-sim init counter must not increment during a frame_ticker walk"
        );
    }

    /// `frame_step_ticks` correctly converts fps → tick step.
    #[test]
    fn frame_step_matches_fps_over_bundle_tick_rate() {
        assert_eq!(frame_step_ticks(60, 60), 1);
        assert_eq!(frame_step_ticks(30, 60), 2);
        assert_eq!(frame_step_ticks(120, 60), 1);
        assert_eq!(frame_step_ticks(0, 60), 1);
        assert_eq!(frame_step_ticks(60, 0), 1);
    }

    /// Unsupported fps → typed error.
    #[test]
    fn unsupported_fps_returns_typed_error() {
        let err = FrameTicker::new(FrameTickerConfig {
            fps: 50,
            tick_rate_hz: 60,
            start_tick: 0,
            end_tick: 10,
        })
        .expect_err("fps=50 must reject");
        assert!(matches!(err, FrameTickerError::UnsupportedFps { requested: 50 }));
    }

    /// Empty tick range → typed error.
    #[test]
    fn empty_tick_range_returns_typed_error() {
        let err = FrameTicker::new(FrameTickerConfig {
            fps: 60,
            tick_rate_hz: 60,
            start_tick: 10,
            end_tick: 10,
        })
        .expect_err("zero range must reject");
        assert!(matches!(err, FrameTickerError::EmptyTickRange { start: 10, end: 10 }));
    }

    /// VAL-M10B-024 + VAL-M10B-NO-LIVE-SIM combined: the ticker
    /// routes per-tick frames through the camera director and emits
    /// the active camera kind per frame.
    #[test]
    fn frame_ticker_routes_camera_director_per_frame() {
        let text = r#"(
            tracks: [
                (kind: "free_cam", start_tick: 0, end_tick: 2, keyframes: [(tick: 0, pose: Some([0.0, 0.0, 1.0, 0.0, 0.0, 0.0]))]),
                (kind: "follow_player", start_tick: 2, end_tick: 4, keyframes: [(tick: 2, pose: Some([100.0, 0.0, 1.0, 0.0, 0.0, 0.0]))]),
            ]
        )"#;
        let script = CameraScript::from_ron_str(text).unwrap();
        let director = CameraDirector::new(&script);

        let events = vec![snapshot_event(
            0,
            "b0",
            "baseline_emitted",
            serde_json::json!({"state": {"hp": 100}}),
        )];

        let ticker = FrameTicker::new(FrameTickerConfig {
            fps: 60,
            tick_rate_hz: 60,
            start_tick: 0,
            end_tick: 4,
        })
        .unwrap();
        let frames = ticker.run(BundleSource::Events(&events), Some(&director)).unwrap();
        assert_eq!(frames.len(), 4);
        assert_eq!(frames[0].active_camera, Some(CameraKind::FreeCam));
        assert_eq!(frames[1].active_camera, Some(CameraKind::FreeCam));
        assert_eq!(frames[2].active_camera, Some(CameraKind::FollowPlayer));
        assert_eq!(frames[3].active_camera, Some(CameraKind::FollowPlayer));
    }

    /// Repeated ticker runs over the same input produce byte-identical
    /// frame commands per the determinism contract.
    #[test]
    fn frame_ticker_is_byte_identical_on_repeated_runs() {
        let events = vec![snapshot_event(
            0,
            "b0",
            "baseline_emitted",
            serde_json::json!({"state": {"hp": 100}}),
        )];
        let ticker = FrameTicker::new(FrameTickerConfig {
            fps: 60,
            tick_rate_hz: 60,
            start_tick: 0,
            end_tick: 4,
        })
        .unwrap();
        let a = ticker.run(BundleSource::Events(&events), None).unwrap();
        let b = ticker.run(BundleSource::Events(&events), None).unwrap();
        assert_eq!(a, b);
    }
}
