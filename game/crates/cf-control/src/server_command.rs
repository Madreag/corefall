//! ControlCommand enum
//!
//! Extracted from server.rs.

#![allow(unused_imports, dead_code, clippy::too_many_arguments)]

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
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

use cf_actor::IntentSource;

use crate::envelope::*;
use crate::server::*;
use crate::Settings;

#[derive(Debug, Clone)]
pub enum ControlCommand {
    ScenarioLoad {
        scenario: String,
        seed: Option<u64>,
    },
    ScenarioReset,
    Pause,
    Resume,
    Step {
        ticks: u64,
    },
    RunForTicks {
        ticks: u64,
        write_run_bundle: bool,
    },
    ActPlayerMove {
        x: f32,
        y: f32,
        source: IntentSource,
    },
    ActPlayerJump {
        source: IntentSource,
    },
    ActPlayerAim {
        x: f32,
        y: f32,
        source: IntentSource,
    },
    ActPlayerFire {
        pressed: bool,
        /// (`heat` / `apfsds` / `regular` / `tracer` / etc.). `None` =
        /// use the weapon's default round per existing M6 behavior.
        ammo_kind: Option<cf_equipment::RoundKind>,
        source: IntentSource,
    },
    ActPlayerReload {
        source: IntentSource,
    },
    ActPlayerSelectItem {
        slot: u32,
        source: IntentSource,
    },
    ActPlayerReset {
        source: IntentSource,
    },
    /// M1.5: dig the soft-breach strip in front of the player. `target` is an
    /// optional explicit breach id; `None` => pick the nearest in-range strip.
    ActPlayerDig {
        target: Option<String>,
        source: IntentSource,
    },
    /// Samples the chunked terrain material at the target and emits
    /// `terrain.anchor_material_result` with `result="accepted"` (anchorable
    /// material) or `result="refused"` (non-anchorable, with `reason` label).
    /// See `specs/active/M3.md` § Re-opened gaps, MAT-T-06.
    ActPlayerAnchor {
        x: f64,
        y: f64,
        tool_id: Option<String>,
        source: IntentSource,
    },
    /// M4A: ACC-A-04 keyboard/controller focus traversal.
    /// `direction = "next" | "prev" | "set:<node_id>" | "clear"`.
    /// Drives the canonical `HUD_FOCUSABLE_NODES` cursor in the engine; the
    /// new focus state surfaces in `observe.accessibility.focused_node` +
    /// `focus_cycle`. cf-app's keyboard layer + cfctl + cf-e2e all dispatch
    /// through this same path.
    ActInputFocus {
        direction: FocusDirection,
        source: IntentSource,
    },
    /// the hit `target_node_id` via the HUD layout and emits a
    /// `ux.mouse_clicked` event. Non-finite coords reject at the dispatch
    /// boundary.
    ActInputMouseClick {
        x: f32,
        y: f32,
        source: IntentSource,
    },
    /// the hover `hover_node_id` via the HUD layout and emits a
    /// `ux.mouse_moved` event. Non-finite coords reject at the dispatch
    /// boundary.
    ActInputMouseMove {
        x: f32,
        y: f32,
        source: IntentSource,
    },
    /// BP3 self-play floor + pause-overlay cycling. Per the M11 spec
    /// § "Pause + slowdown overlay": "Triggered via `act.input.key_press
    /// { action: 'pause' }` (cycles through modes)". `action` is one of
    /// `pause`, `game_speed_cycle`, `accessibility_overlay`, `tactical_overlay`,
    /// `photo_mode`, `debug_overlay`, `mini_map_toggle`, `compass_toggle`,
    /// `damage_direction_toggle`, `captions_toggle`. Unknown actions reject
    /// with reason `unknown_key_action`.
    ActInputKeyPress {
        action: String,
        source: IntentSource,
    },
    ActPlayerCrouch {
        active: bool,
        source: IntentSource,
    },
    /// owns physical climb resolution).
    ActPlayerClimb {
        active: bool,
        source: IntentSource,
    },
    /// nominal/degraded — Warning + Failed reject).
    ActPlayerJet {
        active: bool,
        source: IntentSource,
    },
    ActPlayerEject {
        source: IntentSource,
    },
    ActPlayerQuickActionSlot {
        slot: u8,
        source: IntentSource,
    },
    ActPlayerQuickActionToggle {
        source: IntentSource,
    },
    ActPlayerQuickActionRadial {
        active: bool,
        source: IntentSource,
    },
    ActPlayerQuickActionSlice {
        slice: u8,
        source: IntentSource,
    },
    ActPlayerWeaponCycle {
        direction: i8,
        source: IntentSource,
    },
    /// `reason` carries the operator label (`field_kit`, `repair_drone`, etc.).
    ActChassisRepair {
        zone: Option<String>,
        module_id: Option<String>,
        reason: String,
        source: IntentSource,
    },
    /// `chassis.salvaged_modules`.
    ActChassisSalvage {
        reason: String,
        source: IntentSource,
    },
    ActChassisClearJam {
        source: IntentSource,
    },
    /// friendly actor; the prior actor stays at its position as a
    /// mission-critical AI fallback.
    ActPlayerBrainHop {
        target_actor_id: u64,
        source: IntentSource,
    },
    ActPlayerActivateAbility {
        ability: String,
        source: IntentSource,
    },
    ActInputCameraAnchor {
        mode: String,
        source: IntentSource,
    },
    ActPlayerSetDroneMode {
        mode: String,
        source: IntentSource,
    },
    ActPlayerAttachModifier {
        modifier: String,
        source: IntentSource,
    },
    ActPlayerDetachModifier {
        modifier: String,
        source: IntentSource,
    },
    /// a chassis actor (1500ms transition).
    ActPlayerBoard {
        chassis_actor_id: u64,
        source: IntentSource,
    },
    /// out of the current chassis (1500ms transition).
    ActPlayerDisembark {
        source: IntentSource,
    },
    /// asks the sim to build `sharp_aim_progress`; `active=false` releases.
    ActPlayerSharpAim {
        active: bool,
        source: IntentSource,
    },
    /// `unsupported_in_m1`; M1.5 swaps in real abort logic without rewiring
    /// the cfctl surface.
    ActPlayerAbort {
        source: IntentSource,
    },
    /// pause path). Emits `mission.objective_paused`.
    ActMissionPause {
        source: IntentSource,
    },
    ActMissionResume {
        source: IntentSource,
    },
    /// debrief prompt, future pause menu) has captured input. While
    /// captured, all `act.player.*` commands are rejected with
    /// `controls_captured` and the CONTROLS CAPTURED HUD zone surfaces.
    ActInputCaptureControls {
        captured: bool,
        capturer: Option<String>,
        source: IntentSource,
    },
    ActToggleMaterialOverlay {
        mode: Option<String>,
        source: IntentSource,
    },
    /// (sprint, slide, vault, lean, stealth kill, knife throw, weapon swap,
    /// drop / pickup, signals, mark waypoint, deploy bipod, cycle fire mode,
    /// cook / throw grenade, melee bash / kick, use tool, suppressor
    /// attach / detach, set facing). Engine reads the inner action + updates
    /// `ActorState` flags + records the matching control event.
    ActM6 {
        action: crate::m6_actions::M6Action,
        source: IntentSource,
    },
    /// broadcasts to all followers.
    ActSquadIssueCommand {
        bot_actor: Option<u64>,
        kind: crate::m6_actions::SquadCommandKindOverWire,
        waypoint: Option<(f32, f32)>,
        source: IntentSource,
    },
    /// them to the default `FollowLeader`. Re-emits `squad.command_issued`
    /// with `kind="follow_leader"` so the replay stream stays linear.
    ActSquadCancelCommand {
        actor_id: u64,
        source: IntentSource,
    },
    /// (clamps to 0..=9). Spec § Smart commandable AI — Per-task override.
    /// Mutates `M7AiWorld.bots[actor].stack.priority` AND emits
    /// `ai.priority_table_changed`.
    ActPlayerSetPriority {
        actor_id: u64,
        task: String,
        weight: u8,
        source: IntentSource,
    },
    /// Manual). Spec § Smart commandable AI — Layer 1 Autonomy mode.
    /// Mutates `M7AiWorld.bots[actor].stack.autonomy` AND emits
    /// `ai.autonomy_mode_changed`.
    ActPlayerSetAutonomyMode {
        actor_id: u64,
        mode: String,
        source: IntentSource,
    },
    /// 6 spec-mandated role templates. Spec § Smart commandable AI — 6
    /// role templates. Emits `ai.role_template_applied`.
    ActPlayerApplyRoleTemplate {
        actor_id: u64,
        template_id: String,
        source: IntentSource,
    },
    /// defend / overwatch / rescue / salvage). Emits
    /// `ai.quick_preset_applied`.
    ActPlayerApplyQuickPreset {
        actor_id: u64,
        preset_id: String,
        source: IntentSource,
    },
    /// Spec § "50+ named squad verbs in a data-driven registry".
    ActSquadIssue {
        squad_id: u64,
        verb_id: String,
        args: Vec<serde_json::Value>,
        source: IntentSource,
    },
    /// formation kinds with per-actor slot resolution".
    ActSquadSetFormation {
        squad_id: u64,
        formation_kind: String,
        source: IntentSource,
    },
    /// "Per-member role assignment is sticky + loadout-aware".
    ActSquadAssignRole {
        squad_id: u64,
        member_actor_id: u64,
        role: String,
        source: IntentSource,
    },
    /// registry, formation catalog, and archetype-BT node counts.
    SrvDumpSquadState {
        squad_id: u64,
        source: IntentSource,
    },
    // === M8 cfctl surface ===
    ActCameraSetMode {
        mode: String,
        source: IntentSource,
    },
    /// records the cause label (`melee_hit`, `ap_round_hit`, etc.).
    ActCameraHitStop {
        duration_ms: u32,
        trigger: String,
        actor_id: Option<u64>,
        source: IntentSource,
    },
    /// Equivalent to `act.camera.set_mode { mode: "scope" }` but encodes
    /// player intent specifically.
    ActCameraScopeZoom {
        source: IntentSource,
    },
    /// transitions to FreeLook anchored at `cursor`; when false it
    /// returns to Follow.
    ActCameraFreeLookToggle {
        active: bool,
        cursor: Option<(f32, f32)>,
        max_distance: f32,
        source: IntentSource,
    },
    /// cf-control mirrors the sim pause + emits `photo_mode.entered`.
    ActPhotoEnter {
        source: IntentSource,
    },
    ActPhotoExit {
        source: IntentSource,
    },
    /// color_grade / cyberpunk_neon).
    ActPhotoCycleFilter {
        source: IntentSource,
    },
    /// the actual PNG export happens in cf-app via cf-photo::export_png).
    ActPhotoShoot {
        source: IntentSource,
    },
    /// rewind, positive = forward).
    ActReplayScrub {
        delta_seconds: f32,
        source: IntentSource,
    },
    ActReplayBookmark {
        label: String,
        source: IntentSource,
    },
    /// pathfinding | collision | material | physics | sound | squad`).
    ActDebugToggleOverlay {
        overlay: String,
        source: IntentSource,
    },
    /// `ux.hud_layout_changed`.
    ActUiSetHudLayout {
        node: String,
        x: f32,
        y: f32,
        source: IntentSource,
    },
    /// `ux.preset_saved`.
    ActUiSavePreset {
        name: String,
        source: IntentSource,
    },
    /// `ux.tactical_overlay_toggled`. `multiplayer` controls the
    /// sim-speed cap (single-player pauses; multiplayer = 25%).
    ActPlayerToggleTacticalOverlay {
        multiplayer: bool,
        source: IntentSource,
    },
    /// Emits `ai.plan_composed`.
    ActPlayerComposePlan {
        actor_id: u64,
        steps: Vec<String>,
        source: IntentSource,
    },
    /// Emits `ai.context_wheel_selected`. `slot` is 0..=7.
    /// `target_kind` selects the per-target slot ordering per spec
    /// § Q-hold context wheel (one of `none` / `squadmate` / `door` /
    /// `enemy` / `terrain_breach` / `hazard` / `reactor_module`). When
    /// the kind needs an entity id (`squadmate` / `door` / `enemy` /
    /// `hazard` / `reactor_module`) the caller supplies `target_id`.
    /// Missing or unknown values fall back to `ReticleTarget::None`.
    ActPlayerContextWheelSelect {
        actor_id: u64,
        slot: u8,
        target_kind: String,
        target_id: Option<u64>,
        source: IntentSource,
    },
    /// or `grenade`. Emits `ai.panic_call_emitted`.
    ActPlayerPanicCall {
        kind: String,
        source: IntentSource,
    },
    /// engine raises Utility weight by +0.5 for engaging the target.
    ActPlayerTagTarget {
        target_id: u64,
        source: IntentSource,
    },
    /// ringbuffer head as a HUD popup. Emits `ai.reason_query_returned`.
    ActPlayerQueryWhy {
        actor_id: u64,
        source: IntentSource,
    },
    /// (`void` / `nearest_actor` / `door` / `item`). Emits
    /// `ux.pie_menu_opened`. Slows sim to 20% in single-player; 100% in
    /// multiplayer.
    ActPlayerPieMenuOpen {
        target_kind: String,
        target_id: Option<u64>,
        multiplayer: bool,
        source: IntentSource,
    },
    /// `ux.pie_menu_slice_chosen` on a valid pick, OR
    /// `ux.pie_menu_slice_rejected { slice, reason }` when the slice is
    /// disabled in the current context. `reason` is optional and
    /// supplied by the caller (cf-app keyboard layer) when it has
    /// pre-validated the slice; otherwise the dispatcher reports
    /// `ok=true` (valid pick) by default.
    ActPlayerPieMenuSelect {
        slot: u8,
        reason: Option<String>,
        source: IntentSource,
    },
    /// `ux.pie_menu_closed` with the open-duration in ticks.
    ActPlayerPieMenuClose {
        source: IntentSource,
    },
    SettingsSet {
        changes: Box<SettingsPatch>,
    },
    RunBundleWrite {
        id_override: Option<String>,
    },
    Shutdown {
        write_run_bundle: bool,
    },
    /// origin. Loads `content/trench_templates/<id>.trench.ron` through
    /// the cf-content loader, instantiates it via
    /// `TrenchTemplate::instantiate`, and emits
    /// `trench.template_dropped` with `template_sha256` (64 hex chars),
    /// `segment_count`, and `placed_fortifications[]` per
    /// VAL-M9B-TEMPLATE-002. Optional placeholders that don't resolve to
    /// a currently-shipped M9C asset emit
    /// `trench.template_missing_fortification` warning events per
    /// VAL-M9B-TEMPLATE-004 (the template still places).
    ActPlayerDropTrenchTemplate {
        id: String,
        origin: (i32, i32),
        source: IntentSource,
    },
    /// trench segment at the player's current tile. `variant` is one of
    /// the 6 declared cross-section variants; `tool_id` selects the
    /// dig tool (entrenching_tool T0, or pickaxe T1/T2/T3 from the
    /// M30B-tier ladder); `substrate_hardness` is `[0.0, 1.0]` from
    /// cf-material — gating the `deep` variant per VAL-M9B-DIG-003.
    /// `strict=true` makes hard-substrate `deep` requests reject
    /// outright; `false` (default) falls back to `shallow_scrape` with a
    /// `trench.segment_variant_downgraded` warning event.
    ActPlayerDigTrenchSegment {
        variant: String,
        tool_id: Option<String>,
        substrate_hardness: f32,
        strict: bool,
        source: IntentSource,
    },
    /// embedded module on a built trench segment. `module_id` is one of
    /// the 6 declared modules (`duckboard`, `fire_step`, `breastwork`,
    /// `drainage_sump`, `revetment`, `corner_traverse`).
    ActPlayerPlaceTrenchModule {
        module_id: String,
        segment_id: u64,
        source: IntentSource,
    },
    /// damaged trench module. Consumes the declared per-module
    /// resources (wood/iron); emits `trench.module_repaired`.
    ActPlayerRepairTrenchModule {
        module_id: String,
        segment_id: u64,
        source: IntentSource,
    },
    /// canonical PascalCase TreatmentKind id; the engine resolves it via
    /// `cf_treatment::TreatmentKind::from_str`.
    ActPlayerTreat {
        kind: String,
        target_actor_id: u64,
        source: IntentSource,
    },
    ActPlayerScan {
        target_actor_id: u64,
        source: IntentSource,
    },
    /// in cardiac arrest.
    ActPlayerCprRound {
        target_actor_id: u64,
        source: IntentSource,
    },
    ActPlayerDefib {
        target_actor_id: u64,
        source: IntentSource,
    },
    ActPlayerSurgeryStart {
        target_actor_id: u64,
        wounds_to_treat: u32,
        surgeon_t1: bool,
        seed: Option<u64>,
        source: IntentSource,
    },
    ActPlayerTriageSelect {
        target_actor_id: Option<u64>,
        source: IntentSource,
    },
    ActPlayerInstallProsthetic {
        target_actor_id: u64,
        kind: String,
        zone: String,
        source: IntentSource,
    },
    ActPlayerMaintainProsthetic {
        target_actor_id: u64,
        zone: String,
        source: IntentSource,
    },
    ActPlayerRetireVeteran {
        target_actor_id: u64,
        source: IntentSource,
    },
    ActPlayerVault {
        source: IntentSource,
    },
    ActPlayerWallJump {
        source: IntentSource,
    },
    ActPlayerFireGrapple {
        target_x: f32,
        target_y: f32,
        source: IntentSource,
    },
    ActPlayerRopeInput {
        climb: f32,
        swing: f32,
        source: IntentSource,
    },
    ActPlayerReleaseRope {
        source: IntentSource,
    },
    ActPlayerZiplineClip {
        line_id: u64,
        source: IntentSource,
    },
    ActPlayerZiplineBrake {
        engaged: bool,
        source: IntentSource,
    },
    ActPlayerMount {
        critter_id: u64,
        source: IntentSource,
    },
    ActPlayerDismount {
        source: IntentSource,
    },
}
