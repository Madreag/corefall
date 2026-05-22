//! EngineHandle trait declaration
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
use crate::server::*;
use crate::server_command::*;
use crate::state::*;
use crate::{Settings, SCHEMA_VERSION, SCHEMA_VERSION_MIN};

/// Trait that the engine implements so the server stays decoupled from `cf-app`.
#[async_trait::async_trait]
pub trait EngineHandle: Send + Sync + 'static {
    async fn snapshot(&self, filter: Option<&str>) -> ObserveFrame;
    async fn settings_snapshot(&self) -> Settings;
    async fn dispatch(&self, command: ControlCommand) -> CommandResult;
    /// profile + AI hints + particle/tracer metadata). Default impl returns
    /// `None` for handlers that don't have an equipment registry.
    async fn inspect_equipment(&self, _preset_id: &str) -> Option<serde_json::Value> {
        None
    }
    /// player when `actor_id` is None). Default returns `None`.
    /// projection or `None` if no mission is loaded.
    async fn observe_mission(&self) -> Option<serde_json::Value> {
        None
    }
    /// perception summary + current target + reason) for `actor_id`.
    async fn observe_ai(&self, _actor_id: u64) -> Option<serde_json::Value> {
        None
    }
    /// projection (chunk_count / dirty_chunk_count / material_distribution
    /// / current_overlay_mode / total_carve_events / total_debris_spawned).
    /// `None` when no chunked terrain is loaded.
    async fn observe_terrain(&self) -> Option<serde_json::Value> {
        None
    }
    /// projection `{ actor_id, hp, max_hp, hp_percent, pressure_state,
    /// position, mission_critical, role, armor_layers, heat_signature_k }`.
    /// `None` when no reactor is loaded in the active scenario.
    async fn observe_mission_reactor(&self) -> Option<serde_json::Value> {
        None
    }
    /// projection `{ remaining_ticks, total_ticks, remaining_seconds,
    /// color_state }`. `color_state` is "green" / "yellow" / "red".
    /// `None` when no mission is loaded.
    async fn observe_mission_timer(&self) -> Option<serde_json::Value> {
        None
    }
    /// objectives + last N mission events.
    async fn inspect_mission(&self) -> Option<serde_json::Value> {
        None
    }
    /// grid + last N ai events.
    async fn inspect_ai(&self, _actor_id: u64) -> Option<serde_json::Value> {
        None
    }
    async fn observe_actor(&self, _actor_id: Option<u64>) -> Option<serde_json::Value> {
        None
    }
    /// action bar + radial state. Default returns `None`.
    async fn observe_quick_action(&self, _actor_id: Option<u64>) -> Option<serde_json::Value> {
        None
    }
    /// + stealth_meter + last footstep loudness band + last occlusion
    /// factor + spotted flag. `actor_id=None` resolves to the player. Default
    /// returns `None` for handlers without a perception kernel.
    async fn observe_perception(&self, _actor_id: Option<u64>) -> Option<serde_json::Value> {
        None
    }
    /// per-member current_command + hp + waypoint. Default returns `None`
    /// when no squad is loaded.
    async fn observe_squad(&self) -> Option<serde_json::Value> {
        None
    }
    /// events. Default returns `None`.
    async fn inspect_actor(&self, _target: Option<&str>, _last_n_events: usize) -> Option<serde_json::Value> {
        None
    }
    /// sockets), per-zone integrity (per-layer HP), per-module state, pilot
    /// state and eject window for the requested actor (`"player"` / empty =
    /// the controllable actor). Returns `None` when the actor has no chassis
    /// attached.
    async fn inspect_chassis(&self, _target: Option<&str>) -> Option<serde_json::Value> {
        None
    }

    /// silhouette projection (per-chassis-zone HP). Surfaces the chassis
    /// half of the dual-layer HUD silhouette so the pilot can stay on
    /// `observe.actor.silhouette`.
    async fn observe_chassis_silhouette(&self, _actor_id: Option<u64>) -> Option<serde_json::Value> {
        None
    }
    /// actor-category events. Spec § Reactor as a non-player static actor:
    /// "And cfctl inspect.actor.reactor returns the full ActorState".
    /// Default returns `None`.
    async fn inspect_actor_reactor(&self, _last_n_events: usize) -> Option<serde_json::Value> {
        None
    }
    /// `{ current_phase, phase_started_at_tick, phases_completed,
    /// intensity, spawn_budget, active_objectives }`. Default returns
    /// `None`.
    async fn observe_mission_director(&self) -> Option<serde_json::Value> {
        None
    }
    /// requested chunk coord. Default returns `None`.
    async fn inspect_terrain_chunk(&self, _cx: i32, _cy: i32) -> Option<serde_json::Value> {
        None
    }
    /// returns `None`.
    async fn inspect_material(&self, _id: u16) -> Option<serde_json::Value> {
        None
    }
    /// world-space `(x, y)` — the 9 affordance flags (actor_passable,
    /// projectile_passable, diggable, anchorable, blocks_light,
    /// contact_damage, path_cost, produces_debris, produces_sound) plus
    /// integrity (read from the per-pixel meta grid via
    /// `ChunkedTerrain::pixel_integrity`) plus color_hex (resolved from
    /// the material registry). Powers spec § "Material affordance
    /// tooltip" + the integrity-overlay reticle. Default returns `None`.
    async fn observe_terrain_material_at(&self, _x: f32, _y: f32) -> Option<serde_json::Value> {
        None
    }
    /// by-category / by-tier / by-status / missing-id list). Reads the
    /// canonical `content/asset_ledger/ledger.jsonl` at the workspace
    /// root by default; engines that ship a non-default ledger path can
    /// override. Returns `None` when no ledger file exists.
    async fn observe_assets_ledger_summary(&self) -> Option<serde_json::Value> {
        default_observe_assets_ledger_summary()
    }
    /// quicksave / quickload / migrate snapshot (path, schema_version,
    /// size_bytes, blake3). Default returns the empty placeholder; M0Engine
    /// overrides with the shared [`crate::m4b_save::LastSaveCache`].
    async fn observe_save_last(&self) -> serde_json::Value {
        serde_json::to_value(crate::m4b_save::LastSaveMetadata::fresh()).unwrap_or(serde_json::Value::Null)
    }
    /// grid + role + personality modifier). Default returns `None`.
    async fn observe_priority_table(&self, _actor_id: u64) -> Option<serde_json::Value> {
        None
    }
    /// Default returns `None`.
    async fn observe_autonomy(&self, _actor_id: u64) -> Option<serde_json::Value> {
        None
    }
    /// formation catalog + archetype-BT node counts + per-squad state row).
    /// Powers `srv.dump_squad_state`. Default returns `None`.
    async fn dump_squad_state(&self, _squad_id: u64) -> Option<serde_json::Value> {
        None
    }
    /// source + phase + playhead + duration + active word + camera
    /// offset. Powers `srv.dump_cinematic_state`. Default returns a
    /// "no cinematic" sentinel.
    async fn dump_cinematic_state(&self) -> serde_json::Value {
        json!({
            "schema_version": SCHEMA_VERSION,
            "cinematic_id": null,
            "source": null,
            "phase": "ended",
            "playhead_ms": 0,
            "duration_ms": 0,
            "active": false,
            "blocks_gameplay_input": false,
            "seen_set_count": 0,
        })
    }
    /// playhead ms when the skip was accepted, or an error reason when
    /// it was rejected (no cinematic active / inside confirm window).
    async fn act_player_skip_cinematic(&self) -> Result<u32, String> {
        Err("no_cinematic_active".to_string())
    }
    /// `(paused, ms)` after the toggle.
    async fn act_player_pause_cinematic(&self) -> Result<(bool, u32), String> {
        Err("no_cinematic_active".to_string())
    }
    /// `Codex → Cinematics`. Returns the engine tick at which the
    /// replay kernel was engaged.
    async fn act_player_replay_cinematic(&self, _id: &str) -> Result<u64, String> {
        Err("no_cinematic_replay_support".to_string())
    }
    /// (mode + position + hit_stop_remaining_ms + fov_degrees +
    /// free_look_max_distance + free_look_cursor + deadzone_radius).
    async fn observe_camera(&self) -> serde_json::Value {
        json!({"schema_version": SCHEMA_VERSION, "mode": "follow", "fov_degrees": cf_camera::FOLLOW_FOV_DEGREES, "hit_stop_remaining_ms": 0_u32})
    }
    /// `Settings.language`.
    async fn observe_localization_current_language(&self) -> serde_json::Value {
        json!({"schema_version": SCHEMA_VERSION, "language": "en"})
    }
    /// snake_case id + whether it's currently enabled.
    async fn observe_debug_overlays(&self) -> serde_json::Value {
        json!({"schema_version": SCHEMA_VERSION, "enabled": Vec::<String>::new(), "available": cf_debug::DebugOverlay::ALL.iter().map(|o| o.as_str()).collect::<Vec<_>>()})
    }
    async fn observe_tactical_overlay(&self) -> serde_json::Value {
        json!({"schema_version": SCHEMA_VERSION, "open": false, "sim_speed_pct": 100_u8, "focused_actor_id": serde_json::Value::Null, "open_count": 0_u32})
    }
    /// + weight_bonus + issuer_actor_id).
    async fn observe_tags(&self) -> serde_json::Value {
        json!({"schema_version": SCHEMA_VERSION, "tagged": Vec::<serde_json::Value>::new()})
    }
    /// 21 settings flags + key_bindings + focused_node + captions queue +
    /// banner stack — so the replay viewer + cfctl AI agents see exactly
    /// what the player sees in the HUD.
    async fn observe_accessibility(&self) -> serde_json::Value {
        let settings = self.settings_snapshot().await;
        let v = serde_json::to_value(&settings).unwrap_or(serde_json::Value::Null);
        json!({ "schema_version": SCHEMA_VERSION, "settings": v, "focusable_nodes": Vec::<String>::new() })
    }
    async fn observe_captions(&self) -> serde_json::Value {
        json!({ "schema_version": SCHEMA_VERSION, "queue": Vec::<serde_json::Value>::new() })
    }
    async fn observe_accessibility_banners(&self) -> serde_json::Value {
        json!({ "schema_version": SCHEMA_VERSION, "banners": Vec::<serde_json::Value>::new() })
    }
    /// promoted from observe_frame). Default returns `None`.
    async fn observe_actor_silhouette(&self, _actor_id: Option<u64>) -> Option<serde_json::Value> {
        None
    }
    /// from observe_frame). Default returns `None`.
    async fn observe_actor_module_strip(&self, _actor_id: Option<u64>) -> Option<serde_json::Value> {
        None
    }
    /// `node_id` and applies `predicate` (e.g. `text~=DOWNED`,
    /// `severity=critical`). Returns a JSON `{ pass: bool, observed: <val> }`.
    async fn ui_assert(&self, _node_id: &str, _predicate: &str) -> serde_json::Value {
        json!({ "schema_version": SCHEMA_VERSION, "pass": false, "observed": serde_json::Value::Null })
    }
    /// for the named actor. Default returns `Exposed` (open ground); the
    /// engine override derives the value from stance × current trench
    /// segment.
    async fn observe_actor_cover_state(&self, actor_id: u64) -> serde_json::Value {
        json!({
            "schema_version": SCHEMA_VERSION,
            "actor_id": actor_id,
            "cover_state": "Exposed",
        })
    }
    /// `TrenchSegmentView` object. Default returns `null`.
    async fn observe_trench_segment_at_pos(&self, _x: i32, _y: i32) -> serde_json::Value {
        json!({
            "schema_version": SCHEMA_VERSION,
            "result": serde_json::Value::Null,
        })
    }
}
