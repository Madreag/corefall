//! Run-bundle manifest writers.
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

impl M0Engine {
    pub fn write_run_bundle(&self, ended_at: DateTime<Utc>, exit_code: i32) -> Result<PathBuf, cf_replay::BundleError> {
        // M2 (extended): every bundle written from the engine — including mid-run
        // `runbundle.write` that fires before `record_run_finished` — must contain at
        // least one `determinism.sim_checksum` event so `summary.json.final_sim_checksum`
        // is never null on a valid bundle.
        self.emit_final_checksum();
        let manifest = self.build_manifest();
        let perf = self.perf_sample();
        let result = if exit_code == 0 { "pass" } else { "fail" };
        let evidence_ids = self.first_and_last_event_ids();
        let tests = build_test_records(
            &self.config.expected_tests,
            &self.config.milestone,
            result,
            &evidence_ids,
        );
        let (artifacts, capture_evidence_link) = discover_run_artifacts(&self.run_bundle_dir);
        let mut evidence_links = vec![
            "events.jsonl".to_string(),
            "summary.json".to_string(),
            "run_manifest.json".to_string(),
        ];
        if let Some(link) = capture_evidence_link {
            evidence_links.push(link);
        }
        let inputs = BundleInputs {
            recorder: &self.recorder,
            manifest,
            started_at: self.started_at,
            ended_at,
            exit_code,
            result: result.to_string(),
            blockers: vec![],
            next_actions: next_actions_for_milestone(&self.config.milestone),
            tests,
            artifacts,
            assumptions_tested: self.config.assumptions_tested.clone(),
            good: vec![],
            bad: vec![],
            meh: vec![],
            evidence_links,
            notes_extra: notes_addendum_for_milestone(&self.config.milestone),
            perf: Some(perf),
        };
        cf_replay::write_run_bundle(&self.run_bundle_dir, inputs)?;
        Ok(self.run_bundle_dir.clone())
    }

    pub(crate) fn first_and_last_event_ids(&self) -> Vec<String> {
        let events = self.recorder.snapshot_events();
        match (events.first(), events.last()) {
            (Some(first), Some(last)) if first.event_id != last.event_id => {
                vec![first.event_id.clone(), last.event_id.clone()]
            }
            (Some(only), _) => vec![only.event_id.clone()],
            _ => vec![],
        }
    }

    pub(crate) fn build_manifest(&self) -> RunManifest {
        let mut schemas = BTreeMap::new();
        schemas.insert("control".to_string(), CONTROL_SCHEMA_VERSION);
        schemas.insert("scenario".to_string(), SCENARIO_SCHEMA_VERSION);
        schemas.insert("events".to_string(), EVENT_ENVELOPE_VERSION);

        let live_settings = self.state.read().map(|s| s.settings.clone()).unwrap_or_default();

        RunManifest {
            schema_version: MANIFEST_SCHEMA_VERSION.to_string(),
            run_id: self.recorder.run_id().to_string(),
            prototype_slice: prototype_slice_for_milestone(&self.config.milestone),
            run_mode: self.config.run_mode.clone(),
            milestone: self.config.milestone.clone(),
            build: BuildInfo {
                commit_sha: self.config.commit_sha.clone(),
                worktree_dirty: self.config.worktree_dirty,
                worktree_fingerprint: self.config.worktree_fingerprint.clone(),
                worktree_dirty_files: self.config.worktree_dirty_files.clone(),
                rust_version: self.config.rust_version.clone(),
                bevy_version: self.config.bevy_version.clone(),
                platform: self.config.platform.clone(),
            },
            scene: SceneInfo {
                id: self.config.scenario_id.clone(),
                display_name: self.config.scenario_id.clone(),
                source_path: self.config.scenario_path.display().to_string(),
            },
            seed: self.config.seed,
            started_at_utc: self.started_at.to_rfc3339(),
            duration_target_sec: self.config.duration_ticks as f64 / f64::from(self.config.tick_rate_hz),
            material_schema_version: if self.config.initial_chunked_terrain.is_some() {
                cf_terrain::MATERIAL_SCHEMA_VERSION.to_string()
            } else {
                "n/a-m0".to_string()
            },
            config_hash: self.config.config_hash.clone(),
            assumptions_tested: self.config.assumptions_tested.clone(),
            linked_specs: self.config.linked_specs.clone(),
            expected_tests: self.config.expected_tests.clone(),
            capture_config: if self.config.capture_grid_enabled {
                CaptureConfig {
                    events: true,
                    screenshots: true,
                    captures: true,
                }
            } else {
                CaptureConfig::default()
            },
            schemas,
            capabilities: CapabilitiesBlock {
                debug: self.config.debug_capabilities.iter().any(|c| c == "debug"),
                control_api: self.config.control_api_enabled,
                save_load: false,
                debug_capabilities: self.config.debug_capabilities.clone(),
            },
            settings: SettingsBlock {
                ui_scale: live_settings.ui_scale,
                high_contrast: live_settings.high_contrast,
                captions: live_settings.captions,
                reduced_motion: live_settings.reduced_motion,
                reduced_shake: live_settings.reduced_shake,
                reduced_flash: live_settings.reduced_flash,
                hold_to_confirm: live_settings.hold_to_confirm,
                hold_threshold_ms: live_settings.hold_threshold_ms,
                key_remap_enabled: live_settings.key_remap_enabled,
                key_bindings: live_settings.key_bindings.clone(),
                // M2 audit pass 5 (2026-05-13): persist live difficulty preset
                // into the run manifest so cfctl reproductions don't have to
                // walk observe.settings events to recover the preset id.
                ai_difficulty: live_settings.ai_difficulty.clone(),
                // M1 audit pass 7 (2026-05-13): persist the full feel-cvar
                // suite per spec literal "run_manifest.json.settings reflects
                // the patched values".
                accel: live_settings.accel,
                friction: live_settings.friction,
                gravity: live_settings.gravity,
                jump_force: live_settings.jump_force,
                recoil_decay_per_tick: live_settings.recoil_decay_per_tick,
                sharp_aim_build_ticks: live_settings.sharp_aim_build_ticks,
                walk_threshold: live_settings.walk_threshold,
                reduce_camera_shake_pct: live_settings.reduce_camera_shake_pct,
                tick_rate_hz: self.config.tick_rate_hz,
            },
            // **M4 § Per-scenario checksum cadence**: respect the engine's
            // configured `checksum_cadence_ticks` (which the CLI flag
            // `--checksum-cadence-ticks <N>` plumbs through). Previously
            // the manifest always reported the m0_default cadence (60),
            // so the cf-headless replay verifier couldn't reconstruct the
            // bundle's actual cadence and produced phantom divergences on
            // off-default cadences.
            checksum: ChecksumConfig {
                algorithm: cf_sim_core::checksum::CHECKSUM_ALGORITHM.to_string(),
                scope: cf_sim_core::checksum::CHECKSUM_SCOPE.to_string(),
                cadence_ticks: self.config.checksum_cadence_ticks,
            },
            tick_rate_hz: self.config.tick_rate_hz,
            // M3A-005 / M4: declared lifecycle outcome. The CLI's
            // `--expected-outcome <clean|panic|abort>` flag wins via
            // `expected_outcome_override`. Otherwise, the panic-injection
            // debug path (`cf-app --debug-inject-panic-at-tick`) flips the
            // default to Panic so the produced events match. Everything
            // else defaults to Clean.
            expected_outcome: self.config.expected_outcome_override.unwrap_or_else(|| {
                if self.config.debug_inject_panic_at_tick.is_some() {
                    cf_replay::ExpectedOutcome::Panic
                } else {
                    cf_replay::ExpectedOutcome::Clean
                }
            }),
            // **M4B § "Replays survive a game update"** — record the
            // SaveSchemaVersion this run was produced under. Default
            // `[2, 0, 0]` matches `cf_save::CURRENT_SAVE_SCHEMA_VERSION`.
            save_schema_version: [
                cf_save::CURRENT_SAVE_SCHEMA_VERSION.major,
                cf_save::CURRENT_SAVE_SCHEMA_VERSION.minor,
                cf_save::CURRENT_SAVE_SCHEMA_VERSION.patch,
            ],
            // **M4B § "Delta baseline cadence is enforced"** — honor the
            // engine config (default 600 = 10 s @ 60 Hz, per spec). cf-app
            // / cfctl can override via `--delta-baseline-cadence-ticks`.
            delta_baseline_cadence_ticks: self.config.delta_baseline_cadence_ticks,
            // **M4B § "Tamper-evident competitive replays"** — read the
            // recorder's current chain anchor. `Some(_)` in tournament
            // mode (`--ledger-chain`); `None` for dev bundles. Computed
            // continuously as events are recorded; the value at manifest-
            // write time is the run-end anchor.
            ledger_chain_anchor: self.recorder.chain_anchor(),
        }
    }

}
