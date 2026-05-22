//! Tests for server.rs
//!
//! Extracted from server.rs.

#![allow(unused_imports, dead_code, clippy::too_many_arguments)]

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::net::SocketAddr;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use schemars::JsonSchema;
use async_trait::async_trait;
use tokio::sync::{mpsc, oneshot, broadcast};
use tokio::time::{sleep, timeout};
use futures_util::{SinkExt, StreamExt};

use tokio::sync::Mutex;

use cf_actor::IntentSource;

use crate::envelope::*;
use crate::schemas::*;
use crate::server::*;
use crate::server_command::*;
use crate::server_engine_handle::*;
use crate::state::*;
use crate::{Settings, SCHEMA_VERSION, SCHEMA_VERSION_MIN};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::RunStatus;

    #[derive(Default)]
    struct StubEngine;

    #[async_trait::async_trait]
    impl EngineHandle for StubEngine {
        async fn snapshot(&self, _filter: Option<&str>) -> ObserveFrame {
            ObserveFrame {
                schema_version: SCHEMA_VERSION,
                run_id: "stub".to_string(),
                tick: 0,
                sim_time_ms: 0.0,
                run_status: RunStatus::Paused,
                scenario: "m0_blank".to_string(),
                events_since: 0,
                events: vec![],
                settings: ObserveSettings {
                    schema_version: SCHEMA_VERSION,
                    settings: Settings::default(),
                },
                actors: vec![],
                player_actor_id: None,
                mission: None,
                breaches: vec![],
                enemies: vec![],
                terrain: None,
                reactors: vec![],
                banners: vec![],
                captions: vec![],
                tool_validity: None,
                accessibility: crate::state::AccessibilityView::default(),
                controls_capture: crate::state::ControlsCaptureView::default(),
                trench_segment_at_pos: None,
                cells: vec![],
                gravity_vectors: vec![],
                ropes: vec![],
                ziplines: vec![],
                mount_links: vec![],
            }
        }
        async fn settings_snapshot(&self) -> Settings {
            Settings::default()
        }
        async fn dispatch(&self, _command: ControlCommand) -> CommandResult {
            CommandResult::accepted(0)
        }
    }

    #[tokio::test]
    async fn schema_mismatch_returns_invalid_params() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "act.player.move",
            "params": {"schema_version": 99, "x": 1.0}
        });
        let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
            .await
            .unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
        let error = parsed.error.expect("schema mismatch produces an error");
        assert_eq!(error.code, error_codes::INVALID_PARAMS);
        let data = error.data.unwrap();
        assert_eq!(data.get("reason").unwrap(), "schema_version_mismatch");
        // The error reports the current server SCHEMA_VERSION; future M5+ bumps
        // must update through the constant, not a literal — keeps the contract
        // consistent across the test surface.
        assert_eq!(data.get("server_version").unwrap(), SCHEMA_VERSION);
        assert_eq!(data.get("client_version").unwrap(), 99);
    }

    /// M0.2-F2: Every M0 method must reject a request with missing `params.schema_version`.
    /// Pre-fix, several handlers (`act.settings.set`, `runbundle.write`, `system.shutdown`,
    /// `observe.subscribe/unsubscribe`, `observe.once`) silently defaulted when the field
    /// was absent. Now `check_schema_version` requires it before any handler runs.
    #[tokio::test]
    async fn missing_schema_version_rejects_every_m0_method() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let methods_with_params: &[(&str, serde_json::Value)] = &[
            ("scenario.load", json!({"scenario": "m0_blank"})),
            ("scenario.reset", json!({})),
            ("sim.pause", json!({})),
            ("sim.resume", json!({})),
            ("sim.step", json!({"ticks": 1})),
            ("sim.run_for_ticks", json!({"ticks": 30})),
            ("observe.once", json!({})),
            ("observe.subscribe", json!({"hz": 10})),
            ("observe.unsubscribe", json!({})),
            ("observe.settings", json!({})),
            ("act.player.move", json!({"x": 1.0, "y": 0.0})),
            ("act.settings.set", json!({"ui_scale": 2.0})),
            ("runbundle.write", json!({})),
            ("system.shutdown", json!({})),
        ];
        for (method, params) in methods_with_params {
            let req = json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params});
            let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
                .await
                .unwrap();
            let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
            let error = parsed
                .error
                .unwrap_or_else(|| panic!("{method} must reject missing schema_version"));
            assert_eq!(
                error.code,
                error_codes::INVALID_PARAMS,
                "{method} must return -32602 (InvalidParams) on missing schema_version, got code {}",
                error.code
            );
            let data = error.data.unwrap();
            assert_eq!(
                data.get("reason").unwrap(),
                "schema_version_missing",
                "{method} returned wrong reason on missing schema_version"
            );
        }
    }

    /// M0.2-F2: A request that omits `params` entirely must also reject. (clap-driven
    /// JSON-RPC clients commonly send `{"method": "...", "id": ...}` without params for
    /// no-arg methods; the server must still demand schema_version.)
    #[tokio::test]
    async fn missing_params_object_rejects() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let req = json!({"jsonrpc": "2.0", "id": 1, "method": "sim.pause"});
        let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
            .await
            .unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
        let error = parsed.error.expect("missing params must reject");
        assert_eq!(error.code, error_codes::INVALID_PARAMS);
        assert_eq!(error.data.unwrap().get("reason").unwrap(), "schema_version_missing");
    }

    #[tokio::test]
    async fn unknown_method_returns_method_not_found() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "act.player.fly",
            "params": {"schema_version": 1}
        });
        let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
            .await
            .unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
        let error = parsed.error.expect("unknown method returns error");
        assert_eq!(error.code, error_codes::METHOD_NOT_FOUND);
    }

    /// dispatch returns a stable empty view (live engine wires at M9+).
    #[tokio::test]
    async fn m8b_observe_net_session_transport_dispatches() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "observe.net.session_transport",
            "params": {"schema_version": SCHEMA_VERSION}
        });
        let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
            .await
            .unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
        let result = parsed.result.expect("session_transport returns success");
        assert_eq!(result.get("schema_version").unwrap(), 1);
        assert!(result.get("session_id").is_some(), "view has session_id field");
        assert!(result.get("transport_mode").is_some());
        assert!(result.get("traversal_method").is_some());
        assert!(result.get("traversal_path").is_some());
    }

    #[tokio::test]
    async fn m8b_observe_net_rollback_stats_dispatches() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "observe.net.rollback_stats",
            "params": {"schema_version": SCHEMA_VERSION}
        });
        let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
            .await
            .unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
        let result = parsed.result.expect("rollback_stats returns success");
        assert!(result.get("recent_windows").is_some());
        assert_eq!(result.get("windows_within_budget").unwrap(), 0);
        assert_eq!(result.get("windows_over_budget").unwrap(), 0);
    }

    #[tokio::test]
    async fn m8b_observe_net_loss_recovery_dispatches() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "observe.net.loss_recovery",
            "params": {"schema_version": SCHEMA_VERSION}
        });
        let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
            .await
            .unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
        let result = parsed.result.expect("loss_recovery returns success");
        assert_eq!(result.get("redundant_input_window_ticks").unwrap(), 3);
    }

    #[tokio::test]
    async fn m8b_admin_net_force_relay_accepts_toggle() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "admin.net.force_relay",
            "params": {"schema_version": SCHEMA_VERSION, "enabled": true}
        });
        let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
            .await
            .unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
        let result = parsed.result.expect("force_relay returns success");
        assert_eq!(result.get("status").unwrap(), "accepted");
        assert_eq!(result.get("force_relay_enabled").unwrap(), true);
    }

    #[tokio::test]
    async fn observe_once_returns_frame() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "observe.once",
            "params": {"schema_version": SCHEMA_VERSION}
        });
        let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
            .await
            .unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
        let result = parsed.result.expect("observe.once returns success");
        assert_eq!(result.get("scenario").unwrap(), "m0_blank");
        // observe.once returns an ObserveFrame whose schema_version field equals
        // the current server SCHEMA_VERSION constant; the test reads through the
        // constant so a future M5+ bump cannot silently drift the contract.
        assert_eq!(result.get("schema_version").unwrap(), SCHEMA_VERSION);
    }

    #[tokio::test]
    async fn settings_set_dispatches_patch() {
        struct CaptureEngine {
            patch: Mutex<Option<Box<SettingsPatch>>>,
        }
        #[async_trait::async_trait]
        impl EngineHandle for CaptureEngine {
            async fn snapshot(&self, _filter: Option<&str>) -> ObserveFrame {
                StubEngine.snapshot(_filter).await
            }
            async fn settings_snapshot(&self) -> Settings {
                Settings::default()
            }
            async fn dispatch(&self, command: ControlCommand) -> CommandResult {
                if let ControlCommand::SettingsSet { changes } = command {
                    *self.patch.lock().await = Some(changes);
                }
                CommandResult::accepted(0)
            }
        }
        let engine = CaptureEngine {
            patch: Mutex::new(None),
        };
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "act.settings.set",
            "params": {"schema_version": 1, "ui_scale": 1.5, "high_contrast": true}
        });
        let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
            .await
            .unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
        assert!(parsed.result.is_some());
        let captured = engine.patch.lock().await.clone().unwrap();
        assert_eq!(captured.ui_scale, Some(1.5));
        assert_eq!(captured.high_contrast, Some(true));
        assert_eq!(captured.captions, None);
    }

    /// Contract-integrity regression: every M0 handler must reject unknown fields instead
    /// of accepting a request whose extra data was silently ignored.
    #[tokio::test]
    async fn unknown_params_reject_every_m0_method() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let methods_with_params: &[(&str, serde_json::Value)] = &[
            (
                "scenario.load",
                json!({"schema_version": 1, "scenario": "m0_blank", "unexpected": true}),
            ),
            ("scenario.reset", json!({"schema_version": 1, "unexpected": true})),
            ("sim.pause", json!({"schema_version": 1, "unexpected": true})),
            ("sim.resume", json!({"schema_version": 1, "unexpected": true})),
            ("sim.step", json!({"schema_version": 1, "ticks": 1, "unexpected": true})),
            (
                "sim.run_for_ticks",
                json!({"schema_version": 1, "ticks": 30, "unexpected": true}),
            ),
            ("observe.once", json!({"schema_version": 1, "unexpected": true})),
            (
                "observe.subscribe",
                json!({"schema_version": 1, "hz": 10, "unexpected": true}),
            ),
            ("observe.unsubscribe", json!({"schema_version": 1, "unexpected": true})),
            ("observe.settings", json!({"schema_version": 1, "unexpected": true})),
            (
                "act.player.move",
                json!({"schema_version": 1, "x": 1.0, "unexpected": true}),
            ),
            (
                "act.settings.set",
                json!({"schema_version": 1, "ui_scale": 2.0, "unexpected": true}),
            ),
            ("runbundle.write", json!({"schema_version": 1, "unexpected": true})),
            ("system.shutdown", json!({"schema_version": 1, "unexpected": true})),
        ];
        for (method, params) in methods_with_params {
            let req = json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params});
            let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
                .await
                .unwrap();
            let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
            let error = parsed
                .error
                .unwrap_or_else(|| panic!("{method} must reject unknown params"));
            assert_eq!(
                error.code,
                error_codes::INVALID_PARAMS,
                "{method} must reject unknown params"
            );
            assert!(
                parsed.result.is_none(),
                "{method} must not return success with unknown params"
            );
        }
    }

    #[tokio::test]
    async fn unsupported_m0_params_reject_before_dispatch() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let cases: &[(&str, serde_json::Value, &str)] = &[
            (
                "sim.step",
                json!({"schema_version": 1, "ticks": 0}),
                "ticks_must_be_positive",
            ),
            (
                "sim.run_for_ticks",
                json!({"schema_version": 1, "ticks": 0}),
                "ticks_must_be_positive",
            ),
            (
                "observe.once",
                json!({"schema_version": 1, "filter": "system"}),
                "observe_filter_not_supported_in_m0",
            ),
            (
                "observe.subscribe",
                json!({"schema_version": 1, "hz": 0}),
                "observe_hz_out_of_range",
            ),
            (
                "observe.subscribe",
                json!({"schema_version": 1, "hz": 241}),
                "observe_hz_out_of_range",
            ),
            (
                "observe.subscribe",
                json!({"schema_version": 1, "filter": "system"}),
                "observe_filter_not_supported_in_m0",
            ),
            ("act.settings.set", json!({"schema_version": 1}), "settings_patch_empty"),
            (
                "runbundle.write",
                json!({"schema_version": 1, "id_override": "manual"}),
                "runbundle_id_override_not_supported_in_m0",
            ),
        ];
        for (method, params, reason) in cases {
            let req = json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params});
            let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
                .await
                .unwrap();
            let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
            let error = parsed.error.unwrap_or_else(|| panic!("{method} must reject {params}"));
            assert_eq!(error.code, error_codes::INVALID_PARAMS, "{method} wrong error code");
            assert_eq!(
                error.data.unwrap().get("reason").unwrap(),
                reason,
                "{method} wrong reason"
            );
        }
    }

    /// Regression test for the PR #26 review's 🔴 finding: the original
    /// implementation used `Notify::notify_waiters()` which is non-sticky —
    /// if signaled while no `.notified().await` was active, the signal was
    /// silently lost. This test triggers shutdown BEFORE any receiver
    /// observes it, then asserts that a receiver created later still
    /// observes the signal (i.e., `watch::<bool>` is sticky and cannot lose
    /// the signal in a race window).
    #[tokio::test]
    async fn shutdown_signal_is_sticky_across_subscribe_after_trigger() {
        let (tx, rx) = shutdown_signal();
        // Trigger shutdown BEFORE anyone awaits — this is the failure mode
        // of the original Notify-based implementation.
        trigger_shutdown(&tx);
        // A new receiver created at any later time observes `true`.
        let mut late_rx = rx;
        // `wait_for_shutdown` should resolve essentially immediately because
        // the value is already `true`. A 100 ms timeout is generous; failure
        // here means the signal was lost.
        tokio::time::timeout(std::time::Duration::from_millis(100), wait_for_shutdown(&mut late_rx))
            .await
            .expect("wait_for_shutdown must resolve when value is already true (PR #26 sticky-shutdown regression)");
    }

    /// Regression test for the same root cause but in the snapshot-await
    /// race window: the observation loop checks `*shutdown.borrow()` at the
    /// top of every iteration and races every long await against the
    /// shutdown receiver inside a `select!`. This test simulates the race
    /// by triggering shutdown WHILE another task is mid-await on
    /// `wait_for_shutdown` and proves the await resolves cleanly.
    #[tokio::test]
    async fn shutdown_signal_unblocks_inflight_wait() {
        let (tx, rx) = shutdown_signal();
        let mut rx_for_task = rx.clone();
        let waiter = tokio::spawn(async move {
            wait_for_shutdown(&mut rx_for_task).await;
        });
        // Give the task a moment to enter the await.
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        trigger_shutdown(&tx);
        // The waiter should resolve well within 100 ms.
        tokio::time::timeout(std::time::Duration::from_millis(100), waiter)
            .await
            .expect("in-flight wait_for_shutdown must resolve when signal fires (PR #26 sticky-shutdown regression)")
            .expect("waiter task should not panic");
    }

    /// summary (total + by_category + by_tier + by_status + non-fresh
    /// arrays) backed by the engine handle's projection. Default impl
    /// returns either the canonical ledger's contents or an empty
    /// projection when no ledger is present.
    #[tokio::test]
    async fn observe_assets_ledger_summary_returns_summary() {
        struct LedgerEngine;
        #[async_trait::async_trait]
        impl EngineHandle for LedgerEngine {
            async fn snapshot(&self, filter: Option<&str>) -> ObserveFrame {
                StubEngine.snapshot(filter).await
            }
            async fn settings_snapshot(&self) -> Settings {
                Settings::default()
            }
            async fn dispatch(&self, _c: ControlCommand) -> CommandResult {
                CommandResult::accepted(0)
            }
            async fn observe_assets_ledger_summary(&self) -> Option<serde_json::Value> {
                Some(json!({
                    "schema_version": 1,
                    "total_entries": 5,
                    "live_entries": 4,
                    "superseded_entries": 1,
                    "by_category": {"WeaponSprite": 4},
                    "by_tier": {"Tier1_SVG": 4},
                    "by_status": {"Fresh": 4},
                    "missing": [],
                    "drifted": [],
                    "failed": [],
                    "stale": [],
                }))
            }
        }
        let engine = LedgerEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "observe.assets.ledger_summary",
            "params": {"schema_version": SCHEMA_VERSION}
        });
        let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
            .await
            .unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
        let result = parsed.result.expect("ledger summary returns success");
        assert_eq!(result.get("schema_version").unwrap(), 1);
        assert_eq!(result.get("total_entries").unwrap(), 5);
        assert_eq!(result.get("live_entries").unwrap(), 4);
        assert!(result.get("by_category").unwrap().is_object());
    }

    /// returns an empty-but-well-formed projection so callers don't have
    /// to special-case the missing-file case.
    #[tokio::test]
    async fn observe_assets_ledger_summary_falls_back_to_empty() {
        struct EmptyEngine;
        #[async_trait::async_trait]
        impl EngineHandle for EmptyEngine {
            async fn snapshot(&self, filter: Option<&str>) -> ObserveFrame {
                StubEngine.snapshot(filter).await
            }
            async fn settings_snapshot(&self) -> Settings {
                Settings::default()
            }
            async fn dispatch(&self, _c: ControlCommand) -> CommandResult {
                CommandResult::accepted(0)
            }
            async fn observe_assets_ledger_summary(&self) -> Option<serde_json::Value> {
                None
            }
        }
        let engine = EmptyEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "observe.assets.ledger_summary",
            "params": {"schema_version": SCHEMA_VERSION}
        });
        let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
            .await
            .unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
        let result = parsed.result.expect("ledger summary returns success");
        assert_eq!(result.get("total_entries").unwrap(), 0);
        assert!(result.get("missing").unwrap().is_array());
    }

    #[tokio::test]
    async fn runbundle_write_rejects_path_traversal() {
        // distinct rejection reasons:
        //   - `absolute_path_rejected` for ids starting with `/`
        //   - `path_traversal_rejected` for `..` or `\`
        let engine = StubEngine;
        let hz = std::sync::Arc::new(tokio::sync::Mutex::new(None::<u32>));
        let filter = std::sync::Arc::new(tokio::sync::Mutex::new(None::<String>));
        let cases: &[(serde_json::Value, &str)] = &[
            (
                json!({"schema_version": 1, "id_override": "../../../etc/passwd"}),
                "path_traversal_rejected",
            ),
            (
                json!({"schema_version": 1, "id_override": "foo/bar"}),
                "path_traversal_rejected",
            ),
            (
                json!({"schema_version": 1, "id_override": "foo\\bar"}),
                "path_traversal_rejected",
            ),
            (
                json!({"schema_version": 1, "id_override": "/absolute/path"}),
                "absolute_path_rejected",
            ),
            (
                json!({"schema_version": 1, "id_override": "..\\windows\\system32"}),
                "path_traversal_rejected",
            ),
        ];
        for (params, expected_reason) in cases {
            let req = json!({"jsonrpc": "2.0", "id": 1, "method": "runbundle.write", "params": params});
            let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
                .await
                .unwrap();
            let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
            let error = parsed
                .error
                .unwrap_or_else(|| panic!("runbundle.write must reject {params}"));
            assert_eq!(error.code, error_codes::INVALID_PARAMS);
            assert_eq!(
                error.data.unwrap().get("reason").unwrap(),
                expected_reason,
                "wrong reason for {params}",
            );
        }
    }

    /// is routable on the cf-control server. With a well-formed
    /// payload the dispatcher returns an `accepted` ack — never the
    /// generic `-32601 MethodNotFound` error.
    #[tokio::test]
    async fn act_player_drop_trench_template_routes() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let req = json!({
            "jsonrpc": "2.0",
            "id": 7,
            "method": "act.player.drop_trench_template",
            "params": {
                "schema_version": SCHEMA_VERSION,
                "id": "wwi_frontline_a",
                "origin": [50, 30],
            }
        });
        let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
            .await
            .unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
        assert!(parsed.error.is_none(), "expected success, got error: {:?}", parsed.error);
        let result = parsed.result.expect("dispatched response carries result");
        assert_eq!(result.get("status").and_then(|v| v.as_str()), Some("accepted"));
    }

    /// `act.player.drop_trench_template` rejects an empty template id
    /// before reaching the engine handler — surfaces
    /// `template_id_empty` reason.
    #[tokio::test]
    async fn act_player_drop_trench_template_rejects_empty_id() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let req = json!({
            "jsonrpc": "2.0",
            "id": 8,
            "method": "act.player.drop_trench_template",
            "params": {
                "schema_version": SCHEMA_VERSION,
                "id": "",
                "origin": [0, 0],
            }
        });
        let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
            .await
            .unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
        let error = parsed.error.expect("empty id must reject");
        assert_eq!(error.code, error_codes::INVALID_PARAMS);
        assert_eq!(
            error.data.unwrap().get("reason").and_then(|v| v.as_str()),
            Some("template_id_empty")
        );
    }

    /// methods route on the cf-control server (the assertion list:
    /// `act.player.dig_trench_segment`, `act.player.place_trench_module`,
    /// `act.player.drop_trench_template`, `act.player.repair_trench_module`,
    /// `observe.actor.cover_state`, `observe.trench_segment_at_pos`).
    #[tokio::test]
    async fn m9b_cfctl_dispatch() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let routes: &[(&str, serde_json::Value)] = &[
            (
                "act.player.dig_trench_segment",
                json!({
                    "schema_version": SCHEMA_VERSION,
                    "variant": "standard",
                    "tool_id": "entrenching_tool",
                    "substrate_hardness": 0.2,
                }),
            ),
            (
                "act.player.place_trench_module",
                json!({
                    "schema_version": SCHEMA_VERSION,
                    "module_id": "duckboard",
                    "segment_id": 7u64,
                }),
            ),
            (
                "act.player.repair_trench_module",
                json!({
                    "schema_version": SCHEMA_VERSION,
                    "module_id": "duckboard",
                    "segment_id": 7u64,
                }),
            ),
            (
                "act.player.drop_trench_template",
                json!({
                    "schema_version": SCHEMA_VERSION,
                    "id": "wwi_frontline_a",
                    "origin": [50, 30],
                }),
            ),
            (
                "observe.actor.cover_state",
                json!({"schema_version": SCHEMA_VERSION, "actor_id": 0u64}),
            ),
            (
                "observe.trench_segment_at_pos",
                json!({"schema_version": SCHEMA_VERSION, "x": 0, "y": 0}),
            ),
        ];
        for (method, params) in routes {
            let req = json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": method,
                "params": params,
            });
            let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
                .await
                .unwrap();
            let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
            assert!(
                parsed.result.is_some() || parsed.error.as_ref().is_some_and(|e| e.code != error_codes::METHOD_NOT_FOUND),
                "{method} must not return -32601 MethodNotFound"
            );
            if let Some(error) = parsed.error {
                assert_ne!(
                    error.code,
                    error_codes::METHOD_NOT_FOUND,
                    "{method} returned MethodNotFound; routes table out of date"
                );
            }
        }
    }

    /// variant ids upstream of dispatch.
    #[tokio::test]
    async fn act_player_dig_trench_segment_rejects_empty_variant() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "act.player.dig_trench_segment",
            "params": {
                "schema_version": SCHEMA_VERSION,
                "variant": "",
                "substrate_hardness": 0.0,
            }
        });
        let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
            .await
            .unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
        let error = parsed.error.expect("empty variant must reject");
        assert_eq!(error.code, error_codes::INVALID_PARAMS);
        assert_eq!(
            error.data.unwrap().get("reason").and_then(|v| v.as_str()),
            Some("variant_empty")
        );
    }

    /// returns one of the three declared values.
    #[tokio::test]
    async fn observe_actor_cover_state_returns_enum_value() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "observe.actor.cover_state",
            "params": {"schema_version": SCHEMA_VERSION, "actor_id": 0u64}
        });
        let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
            .await
            .unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
        let result = parsed.result.expect("observe.actor.cover_state returns success");
        let cover = result.get("cover_state").and_then(|v| v.as_str()).unwrap();
        assert!(
            matches!(cover, "Exposed" | "Partial" | "Full"),
            "cover_state must be one of Exposed | Partial | Full; got {cover:?}"
        );
    }

    /// returns either `null` or an object.
    #[tokio::test]
    async fn observe_trench_segment_at_pos_returns_null_or_object() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "observe.trench_segment_at_pos",
            "params": {"schema_version": SCHEMA_VERSION, "x": 0, "y": 0}
        });
        let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
            .await
            .unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
        let result = parsed.result.expect("observe.trench_segment_at_pos returns success");
        let inner = result.get("result").expect("result key");
        assert!(
            inner.is_null() || inner.is_object(),
            "result must be null or object; got {inner:?}"
        );
    }

    /// by this feature route on the cf-control server — none of them
    /// returns `-32601 MethodNotFound`. Future m9c-3..m9c-6 features
    /// add the remaining 6 cfctl methods to the same dispatch table.
    #[tokio::test]
    async fn m9c_mg_cfctl_routes() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let routes: &[(&str, serde_json::Value)] = &[
            (
                "act.player.crew_fortification",
                json!({"schema_version": SCHEMA_VERSION, "id": 1u64}),
            ),
            (
                "act.player.uncrew_fortification",
                json!({"schema_version": SCHEMA_VERSION, "id": 1u64}),
            ),
            (
                "act.player.deploy_mg_tripod",
                json!({"schema_version": SCHEMA_VERSION, "pos": [10, 5]}),
            ),
            (
                "act.player.pack_mg_tripod",
                json!({"schema_version": SCHEMA_VERSION, "tripod_id": 7u64}),
            ),
        ];
        for (method, params) in routes {
            let req = json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": method,
                "params": params,
            });
            let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
                .await
                .unwrap();
            let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
            if let Some(error) = parsed.error.as_ref() {
                assert_ne!(
                    error.code,
                    error_codes::METHOD_NOT_FOUND,
                    "{method} returned MethodNotFound; routes table out of date"
                );
            }
            assert!(parsed.result.is_some(), "{method} must dispatch to a result");
        }
    }

    /// satisfies VAL-M9C-PACK-TRIPOD-SURFACE alongside the dedicated
    /// `act.player.pack_mg_tripod` method.
    #[tokio::test]
    async fn m9c_deploy_mg_tripod_mode_pack_alias() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "act.player.deploy_mg_tripod",
            "params": {
                "schema_version": SCHEMA_VERSION,
                "mode": "pack",
                "tripod_id": 42u64,
            }
        });
        let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
            .await
            .unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
        assert!(parsed.error.is_none(), "expected success: {:?}", parsed.error);
        let result = parsed.result.unwrap();
        assert_eq!(result.get("mode").and_then(|v| v.as_str()), Some("pack"));
    }

    /// `fortification_id_zero` before reaching the engine handler.
    #[tokio::test]
    async fn m9c_crew_fortification_rejects_zero_id() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "act.player.crew_fortification",
            "params": {"schema_version": SCHEMA_VERSION, "id": 0u64}
        });
        let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
            .await
            .unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
        let error = parsed.error.expect("zero id must reject");
        assert_eq!(error.code, error_codes::INVALID_PARAMS);
        assert_eq!(
            error.data.unwrap().get("reason").and_then(|v| v.as_str()),
            Some("fortification_id_zero")
        );
    }

    /// owned by `m9c-4-minefield-suite-robot-engineer-doctrine` route
    /// on the cf-control server — neither returns `MethodNotFound`.
    #[tokio::test]
    async fn m9c_minefield_cfctl_routes() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let routes: &[(&str, serde_json::Value)] = &[
            (
                "act.player.deploy_minefield_template",
                json!({
                    "schema_version": SCHEMA_VERSION,
                    "id": "proximity_belt_dense",
                    "origin": [10i32, 5i32],
                }),
            ),
            (
                "act.player.disarm_mine",
                json!({
                    "schema_version": SCHEMA_VERSION,
                    "mine_id": 42u64,
                    "actor_id": 7u64,
                }),
            ),
        ];
        for (method, params) in routes {
            let req = json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": method,
                "params": params,
            });
            let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
                .await
                .unwrap();
            let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
            if let Some(error) = parsed.error.as_ref() {
                assert_ne!(
                    error.code,
                    error_codes::METHOD_NOT_FOUND,
                    "{method} returned MethodNotFound; routes table out of date"
                );
            }
            assert!(parsed.result.is_some(), "{method} must dispatch to a result");
        }
    }

    /// empty template id before reaching the engine handler.
    #[tokio::test]
    async fn m9c_deploy_minefield_template_rejects_empty_id() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "act.player.deploy_minefield_template",
            "params": {
                "schema_version": SCHEMA_VERSION,
                "id": "",
                "origin": [0i32, 0i32],
            }
        });
        let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
            .await
            .unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
        let error = parsed.error.expect("empty template id must reject");
        assert_eq!(error.code, error_codes::INVALID_PARAMS);
        assert_eq!(
            error.data.unwrap().get("reason").and_then(|v| v.as_str()),
            Some("template_id_empty")
        );
    }

    /// path (per VAL-M9C-043) — `robot_id` substitutes for `actor_id`.
    #[tokio::test]
    async fn m9c_disarm_mine_accepts_robot_route() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "act.player.disarm_mine",
            "params": {
                "schema_version": SCHEMA_VERSION,
                "mine_id": 1u64,
                "robot_id": 99u64,
            }
        });
        let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
            .await
            .unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
        assert!(parsed.error.is_none(), "robot-routed disarm must accept: {:?}", parsed.error);
        let result = parsed.result.unwrap();
        assert_eq!(result.get("agent").and_then(|v| v.as_str()), Some("robot"));
    }

    /// `actor_id` AND `robot_id` are missing.
    #[tokio::test]
    async fn m9c_disarm_mine_requires_actor_or_robot() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "act.player.disarm_mine",
            "params": {
                "schema_version": SCHEMA_VERSION,
                "mine_id": 1u64,
            }
        });
        let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
            .await
            .unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
        let error = parsed.error.expect("missing actor + robot must reject");
        assert_eq!(error.code, error_codes::INVALID_PARAMS);
        assert_eq!(
            error.data.unwrap().get("reason").and_then(|v| v.as_str()),
            Some("actor_id_or_robot_id_required"),
        );
    }

    /// routes on the cf-control server. The closure-feature worker
    /// asserts none returns `MethodNotFound` (-32601). The four
    /// methods owned by m9c-2 / m9c-4 are re-tested here so the
    /// dispatch table can never silently drop a method.
    #[tokio::test]
    async fn m9c_cfctl_dispatch() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let routes: &[(&str, serde_json::Value)] = &[
            (
                "act.player.crew_fortification",
                json!({"schema_version": SCHEMA_VERSION, "id": 1u64}),
            ),
            (
                "act.player.uncrew_fortification",
                json!({"schema_version": SCHEMA_VERSION, "id": 1u64}),
            ),
            (
                "act.player.deploy_minefield_template",
                json!({
                    "schema_version": SCHEMA_VERSION,
                    "id": "proximity_belt_dense",
                    "origin": [10i32, 5i32],
                }),
            ),
            (
                "act.player.disarm_mine",
                json!({
                    "schema_version": SCHEMA_VERSION,
                    "mine_id": 1u64,
                    "actor_id": 7u64,
                }),
            ),
            (
                "act.player.cut_wire",
                json!({
                    "schema_version": SCHEMA_VERSION,
                    "wire_id": 1u64,
                    "actor_id": 2u64,
                }),
            ),
            (
                "act.player.repair_fortification",
                json!({"schema_version": SCHEMA_VERSION, "id": 1u64}),
            ),
            (
                "act.player.deploy_mg_tripod",
                json!({"schema_version": SCHEMA_VERSION, "pos": [10, 5]}),
            ),
            (
                "act.player.mark_spotter_target",
                json!({
                    "schema_version": SCHEMA_VERSION,
                    "spotter_id": 3u64,
                    "target_id": 9u64,
                }),
            ),
            (
                "act.player.power_fence",
                json!({"schema_version": SCHEMA_VERSION, "fence_id": 1u64}),
            ),
            (
                "act.player.unpower_fence",
                json!({"schema_version": SCHEMA_VERSION, "fence_id": 1u64}),
            ),
        ];
        assert_eq!(
            routes.len(),
            10,
            "VAL-M9C-010 contract: exactly 10 new M9C cfctl methods must dispatch"
        );
        for (method, params) in routes {
            let req = json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": method,
                "params": params,
            });
            let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
                .await
                .unwrap();
            let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
            if let Some(error) = parsed.error.as_ref() {
                assert_ne!(
                    error.code,
                    error_codes::METHOD_NOT_FOUND,
                    "{method} returned MethodNotFound; m9c-6 dispatch table out of date"
                );
            }
            assert!(
                parsed.result.is_some(),
                "{method} must dispatch to a result"
            );
        }
    }

    /// before reaching the engine handler.
    #[tokio::test]
    async fn m9c_cut_wire_rejects_zero_ids() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        for (params, expected_reason) in [
            (
                json!({"schema_version": SCHEMA_VERSION, "wire_id": 0u64, "actor_id": 1u64}),
                "wire_id_zero",
            ),
            (
                json!({"schema_version": SCHEMA_VERSION, "wire_id": 1u64, "actor_id": 0u64}),
                "actor_id_zero",
            ),
        ] {
            let req = json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "act.player.cut_wire",
                "params": params,
            });
            let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
                .await
                .unwrap();
            let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
            let error = parsed.error.expect("zero id must reject");
            assert_eq!(error.code, error_codes::INVALID_PARAMS);
            assert_eq!(
                error.data.unwrap().get("reason").and_then(|v| v.as_str()),
                Some(expected_reason)
            );
        }
    }

    #[tokio::test]
    async fn m9c_repair_fortification_rejects_zero_id() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "act.player.repair_fortification",
            "params": {"schema_version": SCHEMA_VERSION, "id": 0u64}
        });
        let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
            .await
            .unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
        let error = parsed.error.expect("zero id must reject");
        assert_eq!(error.code, error_codes::INVALID_PARAMS);
        assert_eq!(
            error.data.unwrap().get("reason").and_then(|v| v.as_str()),
            Some("fortification_id_zero")
        );
    }

    /// + `cause="breaker_toggled"` so the closure-feature worker
    /// observes the `fence_depowered.cause` value matching the
    /// schema enum.
    #[tokio::test]
    async fn m9c_unpower_fence_emits_breaker_toggled() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "act.player.unpower_fence",
            "params": {"schema_version": SCHEMA_VERSION, "fence_id": 1u64}
        });
        let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
            .await
            .unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
        assert!(parsed.error.is_none(), "expected success: {:?}", parsed.error);
        let result = parsed.result.unwrap();
        assert_eq!(result.get("powered").and_then(|v| v.as_bool()), Some(false));
        assert_eq!(
            result.get("cause").and_then(|v| v.as_str()),
            Some("breaker_toggled")
        );
    }

    /// `MethodNotFound`). Stub engine defaults to "no cinematic
    /// active" so the request shape is exercised end-to-end.
    #[tokio::test]
    async fn m12c_cinematic_cfctl_routes() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        for method in [
            "act.player.skip_cinematic",
            "act.player.pause_cinematic",
            "srv.dump_cinematic_state",
        ] {
            let req = json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": method,
                "params": {"schema_version": SCHEMA_VERSION},
            });
            let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
                .await
                .unwrap();
            let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
            if let Some(err) = &parsed.error {
                assert_ne!(
                    err.code,
                    error_codes::METHOD_NOT_FOUND,
                    "{method} returned MethodNotFound; cfctl routes table out of date"
                );
            }
        }
        // replay_cinematic carries an `id` parameter.
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "act.player.replay_cinematic",
            "params": {"schema_version": SCHEMA_VERSION, "id": "cin_intro"},
        });
        let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
            .await
            .unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
        if let Some(err) = &parsed.error {
            assert_ne!(
                err.code,
                error_codes::METHOD_NOT_FOUND,
                "act.player.replay_cinematic returned MethodNotFound"
            );
        }
    }

    /// sentinel from the stub engine — schema_version + phase ended +
    /// active false.
    #[tokio::test]
    async fn m12c_dump_cinematic_state_sentinel() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "srv.dump_cinematic_state",
            "params": {"schema_version": SCHEMA_VERSION},
        });
        let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
            .await
            .unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
        assert!(parsed.error.is_none(), "expected success: {:?}", parsed.error);
        let result = parsed.result.expect("payload");
        assert_eq!(result.get("phase").and_then(|v| v.as_str()), Some("ended"));
        assert_eq!(result.get("active").and_then(|v| v.as_bool()), Some(false));
    }
}
