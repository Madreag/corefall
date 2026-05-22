//! Save/load/migrate methods on M0Engine.
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
    pub fn snapshot_world_save(&self) -> cf_save::WorldSave {
        let state = self.state.read().expect("engine state poisoned");
        let world_tick = state.clock.tick().0;
        let mut actors = Vec::new();
        let mut wound_lists: BTreeMap<u64, cf_wound::ActorWoundList> = BTreeMap::new();
        if let Some(sim) = state.actor_state.as_ref() {
            for actor in sim.world.actors.values() {
                let rifle = sim.rifles.get(&actor.id);
                let reload_remaining = rifle.and_then(|r| {
                    if r.reload_remaining_ticks > 0 {
                        Some(r.reload_remaining_ticks)
                    } else {
                        None
                    }
                });
                actors.push(cf_save::SaveBlob {
                    schema_version: cf_save::CURRENT_SAVE_SCHEMA_VERSION,
                    actor_id: actor.id.0,
                    team: actor.team.clone(),
                    origin_id: actor.origin_id.clone(),
                    position: [actor.position.x, actor.position.y],
                    velocity: [actor.velocity.x, actor.velocity.y],
                    aim: [actor.aim.x, actor.aim.y],
                    hp: actor.hp,
                    hp_max: actor.hp_max,
                    on_ground: actor.on_ground,
                    status: format!("{:?}", actor.status),
                    selected_slot: actor.inventory.selected.0,
                    rifle_preset: rifle.map(|r| r.spec.preset_id.clone()),
                    rifle_ammo: rifle.map(|r| r.ammo_in_mag),
                    rifle_reload_remaining_ticks: reload_remaining,
                    chassis: actor.chassis.clone(),
                    gear_dropped_by_limb_loss: actor.gear_dropped_by_limb_loss,
                    chassis_detached: actor.chassis_detached,
                    afflictions: actor.afflictions.iter().map(|a| format!("{:?}", a.kind)).collect(),
                    crouch_active: actor.crouch_active,
                    climb_active: actor.climb_active,
                    jet_active: actor.jet_active,
                    mod_payload: std::collections::BTreeMap::new(),
                });
                wound_lists.insert(actor.id.0, actor.m14g_wound_list.clone());
            }
        }
        let mut mod_payload = std::collections::BTreeMap::new();
        let m14_state = M14SaveExtension::capture(&state, wound_lists);
        match serde_json::to_value(&m14_state) {
            Ok(v) => {
                mod_payload.insert(M14_SAVE_EXTENSION_KEY.to_string(), v);
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "snapshot_world_save: failed to serialize M14 save extension; emitting empty mod_payload"
                );
            }
        }
        cf_save::WorldSave {
            schema_version: cf_save::CURRENT_SAVE_SCHEMA_VERSION,
            world_tick,
            actors,
            terrain_chunks: Vec::new(),
            projectiles: Vec::new(),
            mod_payload,
        }
    }

    /// captured [`cf_save::WorldSave`]. Reverses [`Self::snapshot_world_save`]:
    /// rewires every per-actor field captured in the SaveBlob (HP, status,
    /// position, chassis incl. M14C ERA flags) AND the M14C/D/E/F/G runtime
    /// state stashed in `mod_payload["corefall.m14_state"]`. Returns `true`
    /// on success.
    ///
    /// After load the engine's clock points at `save.world_tick`, so a
    /// subsequent [`Self::snapshot_world_save`] call (with zero
    /// intervening `drive_tick`) produces a WorldSave byte-equal to the
    /// one passed in.
    pub fn load_world_save(&self, save: &cf_save::WorldSave) -> bool {
        let mut state = match self.state.write() {
            Ok(s) => s,
            Err(_) => return false,
        };
        state.clock.set_tick(cf_sim_core::Tick(save.world_tick));
        self.current_tick
            .store(save.world_tick, std::sync::atomic::Ordering::Relaxed);
        if let Some(sim) = state.actor_state.as_mut() {
            for blob in &save.actors {
                let actor_key = cf_actor::ActorId(blob.actor_id);
                if let Some(actor) = sim.world.actors.get_mut(&actor_key) {
                    actor.team = blob.team.clone();
                    actor.origin_id = blob.origin_id.clone();
                    actor.position = cf_actor::Vec2::new(blob.position[0], blob.position[1]);
                    actor.velocity = cf_actor::Vec2::new(blob.velocity[0], blob.velocity[1]);
                    actor.aim = cf_actor::Vec2::new(blob.aim[0], blob.aim[1]);
                    actor.hp = blob.hp;
                    actor.hp_max = blob.hp_max;
                    actor.on_ground = blob.on_ground;
                    actor.chassis = blob.chassis.clone();
                    actor.gear_dropped_by_limb_loss = blob.gear_dropped_by_limb_loss;
                    actor.chassis_detached = blob.chassis_detached;
                    actor.crouch_active = blob.crouch_active;
                    actor.climb_active = blob.climb_active;
                    actor.jet_active = blob.jet_active;
                }
            }
        }
        if let Some(value) = save.mod_payload.get(M14_SAVE_EXTENSION_KEY) {
            match serde_json::from_value::<M14SaveExtension>(value.clone()) {
                Ok(ext) => ext.apply(&mut state),
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "load_world_save: failed to deserialize M14 save extension; engine state left at initial scenario values"
                    );
                }
            }
        }
        true
    }

    /// and emit `system.save_completed`. On success, pushes the
    /// "Quicksaved" HUD banner; on failure, pushes the
    /// [`Self::push_save_failure_banner`] plain-language modal-style
    /// banner so the player sees the error without a panic.
    pub fn quicksave(&self, dir: &std::path::Path) -> Result<cf_save::quicksave::QuicksaveOutcome, cf_save::SaveError> {
        let save = self.snapshot_world_save();
        let tick = Tick(self.current_tick.load(std::sync::atomic::Ordering::Relaxed));
        let sim_time_ms = tick.0 as f64 * (1000.0 / f64::from(self.config.tick_rate_hz.max(1)));
        let result = crate::m4b_save::fire_quicksave(
            &self.recorder,
            tick,
            sim_time_ms,
            &self.last_save_cache,
            dir,
            &save,
            "quicksave",
        );
        match &result {
            Ok(out) => self.push_save_success_banner("save_quicksaved", "Quicksaved", out.wall_clock_ms, tick.0),
            Err(err) => self.push_save_failure_banner(err, tick.0),
        }
        result
    }

    /// the current schema, emit `system.save_loaded` (and `save_migrated` if
    /// the load triggered a migration step). On corruption surfaces the
    /// plain-language modal `"Save file appears corrupted ..."` per spec
    /// Acceptance Criterion 3; on future-version surfaces
    /// `"This save was created in a newer game version (vN.M.P) ..."` per
    /// Acceptance Criterion 2.
    pub fn quickload(&self, dir: &std::path::Path) -> Result<cf_save::quicksave::QuickloadOutcome, cf_save::SaveError> {
        let tick = Tick(self.current_tick.load(std::sync::atomic::Ordering::Relaxed));
        let sim_time_ms = tick.0 as f64 * (1000.0 / f64::from(self.config.tick_rate_hz.max(1)));
        let result = crate::m4b_save::fire_quickload(&self.recorder, tick, sim_time_ms, &self.last_save_cache, dir);
        match &result {
            Ok(out) => {
                self.push_save_success_banner("save_loaded", "Quickloaded", out.wall_clock_ms, tick.0);
                if let (Some(from), Some(to)) = (out.migrated_from, out.migrated_to) {
                    self.push_migration_banner(from, to, tick.0);
                }
            }
            Err(err) => self.push_save_failure_banner(err, tick.0),
        }
        result
    }

    /// emitted event with `kind: "autosave"`.
    pub fn autosave(&self, dir: &std::path::Path) -> Result<cf_save::quicksave::QuicksaveOutcome, cf_save::SaveError> {
        let save = self.snapshot_world_save();
        let tick = Tick(self.current_tick.load(std::sync::atomic::Ordering::Relaxed));
        let sim_time_ms = tick.0 as f64 * (1000.0 / f64::from(self.config.tick_rate_hz.max(1)));
        let result = crate::m4b_save::fire_quicksave(
            &self.recorder,
            tick,
            sim_time_ms,
            &self.last_save_cache,
            dir,
            &save,
            "autosave",
        );
        match &result {
            Ok(out) => self.push_save_success_banner("save_autosaved", "Autosaved", out.wall_clock_ms, tick.0),
            Err(err) => self.push_save_failure_banner(err, tick.0),
        }
        result
    }

    /// "Autosaved" banner into the HUD queue. cf-app's HUD renders this
    /// in the standard banner stack.
    pub(crate) fn push_save_success_banner(&self, id: &str, label: &str, wall_clock_ms: u32, tick: u64) {
        let mut state = match self.state.write() {
            Ok(s) => s,
            Err(_) => return,
        };
        push_banner_dedup(
            &mut state.hud_banners,
            crate::state::HudBannerView {
                id: id.to_string(),
                severity: "info".to_string(),
                label: format!("{label} ({wall_clock_ms} ms)"),
                raised_at_tick: tick,
                expires_at_tick: Some(tick + 180), // ~3 s @ 60 Hz
                accessibility_id: format!("hud.banner.{id}"),
            },
        );
    }

    /// version save modal" — push a critical-severity HUD banner whose
    /// label matches the spec's prescribed modal text for each
    /// [`cf_save::SaveError`] variant.
    pub(crate) fn push_save_failure_banner(&self, err: &cf_save::SaveError, tick: u64) {
        let (id, label) = match err {
            cf_save::SaveError::ChecksumMismatch { .. } => (
                "save_corrupted",
                "Save file appears corrupted (checksum mismatch). Try another slot.".to_string(),
            ),
            cf_save::SaveError::UnsupportedFutureVersion { found, .. } => (
                "save_future_version",
                format!(
                    "This save was created in a newer game version ({}). Update Corefall to load it.",
                    found.as_string()
                ),
            ),
            cf_save::SaveError::MigrationFailed { from, to, reason } => (
                "save_migration_failed",
                format!(
                    "Save migration failed ({} -> {}): {reason}",
                    from.as_string(),
                    to.as_string()
                ),
            ),
            other => ("save_error", format!("Save failed: {other}")),
        };
        let mut state = match self.state.write() {
            Ok(s) => s,
            Err(_) => return,
        };
        push_banner_dedup(
            &mut state.hud_banners,
            crate::state::HudBannerView {
                id: id.to_string(),
                severity: "critical".to_string(),
                label,
                raised_at_tick: tick,
                expires_at_tick: Some(tick + 600), // ~10 s @ 60 Hz (critical sticks longer)
                accessibility_id: format!("hud.banner.{id}"),
            },
        );
    }

    /// migrated from vA -> vB" banner. The viewer header already surfaces
    /// this for replay bundles; the cf-app HUD surfaces it after a F9
    /// quickload that triggered a migration step.
    pub(crate) fn push_migration_banner(&self, from: cf_save::SaveSchemaVersion, to: cf_save::SaveSchemaVersion, tick: u64) {
        let label = format!("Save migrated from {} -> {}", from.as_string(), to.as_string());
        let mut state = match self.state.write() {
            Ok(s) => s,
            Err(_) => return,
        };
        push_banner_dedup(
            &mut state.hud_banners,
            crate::state::HudBannerView {
                id: "save_migrated".to_string(),
                severity: "info".to_string(),
                label,
                raised_at_tick: tick,
                expires_at_tick: Some(tick + 360),
                accessibility_id: "hud.banner.save_migrated".to_string(),
            },
        );
    }

    pub fn save_migrate(
        &self,
        dir: &std::path::PathBuf,
    ) -> Result<cf_save::quicksave::QuickloadOutcome, cf_save::SaveError> {
        let tick = Tick(self.current_tick.load(std::sync::atomic::Ordering::Relaxed));
        let sim_time_ms = tick.0 as f64 * (1000.0 / f64::from(self.config.tick_rate_hz.max(1)));
        crate::m4b_save::fire_migrate(&self.recorder, tick, sim_time_ms, &self.last_save_cache, dir)
    }

    /// Last quicksave metadata snapshot (for cfctl `observe.save.last`).
    pub fn last_save_snapshot(&self) -> crate::m4b_save::LastSaveMetadata {
        self.last_save_cache.snapshot()
    }
}

/// M4A HUD-cache snapshot for cf-app. Mirrors `ObserveFrame.banners /
/// captions / tool_validity / accessibility.focused_node / focus_cycle` so
/// cf-app does not have to call the async `snapshot()` path every frame.
#[derive(Debug, Clone, Default)]
pub struct HudCachesSnapshot {
    pub banners: Vec<crate::state::HudBannerView>,
    pub captions: Vec<crate::state::CaptionView>,
    pub tool_validity: crate::state::ToolValidityView,
    pub focused_node: Option<String>,
    pub focus_cycle: u64,
    /// cf-app can update the CAPTURED HUD zone without an async snapshot.
    pub controls_captured_by: Option<String>,
}

/// Snapshot of the actor world for cf-app's Bevy bridge. Cheap to clone; reuses
/// `cf-actor::ActorObservation` which is the public actor projection.
#[derive(Debug, Clone, Default)]
pub struct ActorRenderSnapshot {
    pub tick: u64,
    pub floor_y: f32,
    pub actors: Vec<cf_actor::ActorObservation>,
    pub player_actor_id: Option<u64>,
    pub player_rifle: Option<RifleHudView>,
    /// M1.5: breach strips for the renderer.
    pub breaches: Vec<BreachRenderView>,
    /// M1.5: mission HUD bundle. `None` when scenario has no mission.
    pub mission: Option<MissionHudView>,
    /// M1.5: extraction zone derived from the first `ReachZone` objective.
    pub extraction_zone: Option<ExtractionZoneView>,
    /// M1.5: per-enemy state + tactic projection so the HUD doesn't fabricate values.
    pub enemies: Vec<EnemyHudView>,
    /// for the reactor zone widgets. `None` when no reactor world is
    /// loaded.
    pub reactor: Option<ReactorHudView>,
    /// timer-warning HUD + countdown color. `None` when no mission is
    /// loaded.
    pub timer: Option<TimerHudView>,
}

/// for cf-app's reactor HP bar, pressure line, and VFX sprite. Mirrors
/// the `observe.mission.reactor` cfctl surface so cf-app does not have
/// to call the async path each frame.
#[derive(Debug, Clone)]
pub struct ReactorHudView {
    pub actor_id: String,
    pub hp: f32,
    pub max_hp: f32,
    pub hp_percent: f32,
    pub pressure_state: String,
    pub position: [f32; 2],
    pub mission_critical: bool,
    pub destroyed: bool,
    pub heat_signature_k: f32,
    pub armor_layers: Vec<ReactorArmorLayerView>,
}

/// (External / Internal / Core). cf-app maps `hp_percent` to the 5-tier
/// integrity band when rendering pips.
#[derive(Debug, Clone)]
pub struct ReactorArmorLayerView {
    pub kind: String,
    pub hp: f32,
    pub max_hp: f32,
    pub hp_percent: f32,
    pub hardness: f32,
}

/// for the cf-ui timer-warning widget. cf-app reads `remaining_seconds`
/// to push warnings + update the color band per frame.
#[derive(Debug, Clone, Default)]
pub struct TimerHudView {
    pub remaining_ticks: u64,
    pub total_ticks: u64,
    pub remaining_seconds: u32,
    pub color_state: String,
    pub mission_terminal: bool,
}

/// terrain anchor, every dirty chunk's pixel data (then clears the dirty
/// set), the active material-overlay mode, and a tool-validity probe at
/// the player's aim direction.
#[derive(Debug, Clone, Default)]
pub struct TerrainRenderSnapshot {
    pub active: bool,
    pub anchor: [f32; 2],
    pub overlay_mode: String,
    pub dirty_updates: Vec<TerrainChunkUpdate>,
    pub dig_preview: Option<TerrainDigPreview>,
}

#[derive(Debug, Clone)]
pub struct TerrainChunkUpdate {
    pub cx: i32,
    pub cy: i32,
    pub dirty_rect: [u32; 4],
    pub pixels: Vec<cf_terrain::MaterialId>,
}

#[derive(Debug, Clone, Copy)]
pub struct TerrainDigPreview {
    pub position: [f32; 2],
    pub radius: f32,
    pub valid: bool,
    pub material_id: cf_terrain::MaterialId,
}

/// M1.5: HUD-side projection of one reactive guard.
#[derive(Debug, Clone)]
pub struct EnemyHudView {
    pub actor: u64,
    pub state: String,
    pub last_tactic: String,
    /// "ENGAGED", "RELOADING"). cf-app surfaces this above the guard's
    /// sprite when `Settings.ai_debug == true`.
    pub intent_label: String,
    /// AI debug label can anchor to the sprite. `None` when the actor
    /// world isn't loaded yet (boot).
    pub position: Option<[f32; 2]>,
}

/// M1.5: render-side projection of a breach strip.
#[derive(Debug, Clone)]
pub struct BreachRenderView {
    pub id: String,
    pub material: String,
    pub bbox_min: [f32; 2],
    pub bbox_max: [f32; 2],
    pub hp: f32,
    pub max_hp: f32,
    pub broken: bool,
    pub refusal_reason: Option<String>,
    /// Maximum distance from the player's centre to the nearest point on the
    /// strip's AABB for the dig to be considered "in range". Mirrors
    /// [`cf_terrain::BreachStrip::dig_range`] so HUD/render consumers can
    /// compute an in-range check that matches the engine's dig contract.
    pub dig_range: f32,
}

/// M1.5: HUD-side projection of mission state.
#[derive(Debug, Clone)]
pub struct MissionHudView {
    pub result: String,
    pub loss_reason: Option<String>,
    pub elapsed_ticks: u64,
    pub time_limit_ticks: u64,
    pub ticks_remaining: Option<u64>,
    pub active_objective: Option<String>,
    pub last_event_label: String,
    /// for the mission-resolved modal. cf-ui renders the CTA button
    /// when `show_replay_cta == true`.
    pub show_me_why_event_id: Option<String>,
    pub show_replay_cta: bool,
}

/// M1.5: extraction zone for the renderer.
#[derive(Debug, Clone)]
pub struct ExtractionZoneView {
    pub objective_id: String,
    pub min: [f32; 2],
    pub max: [f32; 2],
    pub completed: bool,
}

/// Rifle ammo / cooldown / reload bundle for the HUD bridge. Mirrors `cf-ui::HudRifle`
/// without depending on cf-ui.
#[derive(Debug, Clone, Default)]
pub struct RifleHudView {
    pub ammo: u32,
    pub capacity: u32,
    pub fire_cooldown_ticks: u32,
    pub reload_remaining_ticks: u32,
    pub reload_total_ticks: u32,
}

/// Compact projection of the actor world emitted as the payload of `actor.actor_snapshot`
/// events at the configured cadence (60 ticks by default).
pub(crate) struct ActorWorldSnapshot {
    pub(crate) actors: Vec<serde_json::Value>,
    pub(crate) player_actor_id: Option<u64>,
}
