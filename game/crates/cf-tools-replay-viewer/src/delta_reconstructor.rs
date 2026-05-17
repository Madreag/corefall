//! **M4B § "Reads + reconstructs delta chain into per-tick snapshots
//! transparently"** — viewer-side wrapper around `cf_save::delta`.
//!
//! Walks a [`crate::bundle::Bundle`]'s `events.jsonl` for
//! `snapshot.baseline_emitted` + `snapshot.delta_emitted` events and
//! reconstructs per-tick world states on demand. The viewer header surfaces
//! `delta_chain_depth` + `last_baseline_tick`; the cause-chain + debrief
//! panes can call [`reconstruct_at_tick`] to walk forward from the most
//! recent baseline + apply every delta whose tick ≤ requested tick.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::bundle::Bundle;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeltaReconstructorState {
    pub baseline_count: u64,
    pub delta_chain_depth: u64,
    pub last_baseline_tick: Option<u64>,
    pub last_baseline_event_id: Option<String>,
}

/// One reconstructed snapshot at a given tick.
#[derive(Debug, Clone)]
pub struct ReconstructedSnapshot {
    pub tick: u64,
    pub state: serde_json::Value,
}

/// Walk every snapshot event in `bundle` and build the per-baseline summary.
/// This is the surface the viewer header reads.
pub fn summarize(bundle: &Bundle) -> DeltaReconstructorState {
    let mut out = DeltaReconstructorState::default();
    for event in &bundle.events {
        if event.category != "snapshot" {
            continue;
        }
        match event.event_type.as_str() {
            "baseline_emitted" => {
                out.baseline_count += 1;
                out.delta_chain_depth = 0;
                out.last_baseline_tick = Some(event.tick);
                out.last_baseline_event_id = Some(event.event_id.clone());
            }
            "delta_emitted" => {
                out.delta_chain_depth += 1;
            }
            _ => {}
        }
    }
    out
}

/// Reconstruct the world state at `target_tick`. Walks forward from the
/// most recent `snapshot.baseline_emitted` whose tick is ≤ `target_tick`,
/// applying every `snapshot.delta_emitted` whose tick is in
/// `(baseline_tick, target_tick]`.
pub fn reconstruct_at_tick(bundle: &Bundle, target_tick: u64) -> Option<ReconstructedSnapshot> {
    // Find the most recent baseline at or before target_tick.
    let mut baselines: BTreeMap<u64, (String, serde_json::Value)> = BTreeMap::new();
    let mut deltas: BTreeMap<u64, (String, serde_json::Value)> = BTreeMap::new(); // baseline_event_id, ops
    for event in &bundle.events {
        if event.category != "snapshot" {
            continue;
        }
        match event.event_type.as_str() {
            "baseline_emitted" => {
                if let Some(state) = event.payload.get("state").cloned() {
                    baselines.insert(event.tick, (event.event_id.clone(), state));
                }
            }
            "delta_emitted" => {
                let baseline_event_id = event
                    .payload
                    .get("baseline_event_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let ops = event.payload.get("ops").cloned().unwrap_or(serde_json::Value::Array(vec![]));
                deltas.insert(event.tick, (baseline_event_id, ops));
            }
            _ => {}
        }
    }
    let (baseline_tick, (baseline_event_id, baseline_state)) =
        baselines.range(..=target_tick).next_back().map(|(k, v)| (*k, v.clone()))?;
    let mut cursor = baseline_state;
    for (tick, (baseline_id_ref, ops_value)) in deltas.range((std::ops::Bound::Excluded(baseline_tick), std::ops::Bound::Included(target_tick))) {
        if *baseline_id_ref != baseline_event_id {
            // Delta does not chain from this baseline; skip it (the
            // verifier would have already flagged this in
            // `cf-tools-replay-viewer validate`).
            continue;
        }
        // Parse the ops back into DeltaSnapshot::ops and apply.
        let ops_array = ops_value.as_array().cloned().unwrap_or_default();
        for op_value in ops_array {
            let op: cf_save::delta::DeltaOp = match serde_json::from_value(op_value.clone()) {
                Ok(o) => o,
                Err(_) => continue,
            };
            let _ = cf_save::delta::apply_op(&mut cursor, &op);
        }
        let _ = tick; // silence unused-binding warning
    }
    Some(ReconstructedSnapshot {
        tick: target_tick,
        state: cursor,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_bundle_with_snapshots(events: Vec<cf_replay::Event>) -> Bundle {
        use std::collections::BTreeMap;
        Bundle {
            bundle_dir: std::path::PathBuf::from("/tmp/test"),
            manifest: cf_replay::RunManifest {
                schema_version: cf_replay::MANIFEST_SCHEMA_VERSION.to_string(),
                run_id: "test".to_string(),
                prototype_slice: "M0".to_string(),
                run_mode: "test".to_string(),
                milestone: "m0".to_string(),
                build: cf_replay::BuildInfo::default(),
                scene: cf_replay::SceneInfo {
                    id: "t".to_string(),
                    display_name: "t".to_string(),
                    source_path: "t".to_string(),
                },
                seed: 0,
                started_at_utc: "2026-05-15T00:00:00Z".to_string(),
                duration_target_sec: 1.0,
                material_schema_version: "n/a".to_string(),
                config_hash: "0".to_string(),
                assumptions_tested: vec![],
                linked_specs: vec![],
                expected_tests: vec![],
                capture_config: cf_replay::CaptureConfig::default(),
                schemas: BTreeMap::new(),
                capabilities: cf_replay::CapabilitiesBlock::default(),
                settings: cf_replay::SettingsBlock::default(),
                checksum: cf_replay::ChecksumConfig::m0_default(),
                tick_rate_hz: 60,
                expected_outcome: cf_replay::ExpectedOutcome::Clean,
                save_schema_version: [2, 0, 0],
                delta_baseline_cadence_ticks: 600,
                ledger_chain_anchor: None,
            },
            summary: cf_replay::RunSummary {
                schema_version: cf_replay::SUMMARY_SCHEMA_VERSION.to_string(),
                run_id: "test".to_string(),
                manifest_run_id: "test".to_string(),
                duration_sec: 0.0,
                result: "pass".to_string(),
                ended_at_utc: "2026-05-15T00:00:00Z".to_string(),
                exit_code: 0,
                event_counts: cf_replay::EventCounts {
                    total: events.len() as u64,
                    by_category: BTreeMap::new(),
                    by_type: BTreeMap::new(),
                    by_severity: BTreeMap::new(),
                    dropped_total: 0,
                },
                volume: cf_replay::VolumeBlock::default(),
                performance: cf_replay::PerformanceBlock::default(),
                artifacts: cf_replay::ArtifactsBlock::default(),
                blockers: vec![],
                next_actions: vec![],
                tests: vec![],
                final_sim_checksum: None,
                checksum_event_count: 0,
                first_tick: None,
                last_tick: None,
                recorder: cf_replay::RecorderBlock::default(),
            },
            events,
            event_index: BTreeMap::new(),
        }
    }

    fn ev(tick: u64, event_id: &str, event_type: &str, payload: serde_json::Value) -> cf_replay::Event {
        cf_replay::Event {
            schema_version: cf_replay::EVENT_SCHEMA_VERSION.to_string(),
            run_id: "test".to_string(),
            tick,
            sim_time_ms: tick as f64 * 16.6,
            event_id: event_id.to_string(),
            category: "snapshot".to_string(),
            event_type: event_type.to_string(),
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

    #[test]
    fn summarize_counts_baselines_and_resets_depth() {
        let bundle = make_bundle_with_snapshots(vec![
            ev(0, "b0", "baseline_emitted", serde_json::json!({"state": {"hp": 100}})),
            ev(1, "d1", "delta_emitted", serde_json::json!({"baseline_event_id": "b0", "ops": []})),
            ev(2, "d2", "delta_emitted", serde_json::json!({"baseline_event_id": "b0", "ops": []})),
            ev(600, "b600", "baseline_emitted", serde_json::json!({"state": {"hp": 75}})),
        ]);
        let summary = summarize(&bundle);
        assert_eq!(summary.baseline_count, 2);
        assert_eq!(summary.delta_chain_depth, 0);
        assert_eq!(summary.last_baseline_tick, Some(600));
        assert_eq!(summary.last_baseline_event_id.as_deref(), Some("b600"));
    }

    #[test]
    fn reconstruct_at_tick_applies_deltas_forward_from_baseline() {
        let bundle = make_bundle_with_snapshots(vec![
            ev(0, "b0", "baseline_emitted", serde_json::json!({"state": {"hp": 100, "ammo": 30}})),
            ev(1, "d1", "delta_emitted", serde_json::json!({
                "baseline_event_id": "b0",
                "ops": [{"op": "set", "path": ["hp"], "value": 90}]
            })),
            ev(2, "d2", "delta_emitted", serde_json::json!({
                "baseline_event_id": "b0",
                "ops": [{"op": "set", "path": ["hp"], "value": 75}, {"op": "set", "path": ["ammo"], "value": 28}]
            })),
        ]);
        let s = reconstruct_at_tick(&bundle, 2).unwrap();
        assert_eq!(s.tick, 2);
        assert_eq!(s.state.get("hp").and_then(|v| v.as_f64()), Some(75.0));
        assert_eq!(s.state.get("ammo").and_then(|v| v.as_f64()), Some(28.0));
    }
}
