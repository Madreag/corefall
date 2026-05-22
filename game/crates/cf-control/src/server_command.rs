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
        /// **M14C** § optional ammo-kind selector
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
    /// **M3 re-open (2026-05-13)**: place an anchor / tether at world `(x, y)`.
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
    /// **M11**: pointer click at logical screen coords `(x, y)`. Resolves
    /// the hit `target_node_id` via the HUD layout and emits a
    /// `ux.mouse_clicked` event. Non-finite coords reject at the dispatch
    /// boundary.
    ActInputMouseClick {
        x: f32,
        y: f32,
        source: IntentSource,
    },
    /// **M11**: pointer move at logical screen coords `(x, y)`. Resolves
    /// the hover `hover_node_id` via the HUD layout and emits a
    /// `ux.mouse_moved` event. Non-finite coords reject at the dispatch
    /// boundary.
    ActInputMouseMove {
        x: f32,
        y: f32,
        source: IntentSource,
    },
    /// **M11 audit pass (GAP-M11-01 HIGH fix)**: keyed action press for the
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
    /// **M5**: toggle the player actor's crouch stance.
    ActPlayerCrouch {
        active: bool,
        source: IntentSource,
    },
    /// **M5**: toggle the player actor's climb intent (placeholder cue; M5.5
    /// owns physical climb resolution).
    ActPlayerClimb {
        active: bool,
        source: IntentSource,
    },
    /// **M5**: toggle the player actor's jet thrust (requires Jet module
    /// nominal/degraded — Warning + Failed reject).
    ActPlayerJet {
        active: bool,
        source: IntentSource,
    },
    /// **M5**: trigger the chassis eject sequence.
    ActPlayerEject {
        source: IntentSource,
    },
    /// **M14A**: instant slot invocation (keys 1-8).
    ActPlayerQuickActionSlot {
        slot: u8,
        source: IntentSource,
    },
    /// **M14A**: tap-Q quick-toggle to last-used slot.
    ActPlayerQuickActionToggle {
        source: IntentSource,
    },
    /// **M14A**: open/close hold-Q radial picker with sim time-slow.
    ActPlayerQuickActionRadial {
        active: bool,
        source: IntentSource,
    },
    /// **M14A**: commit a radial slice (1-8).
    ActPlayerQuickActionSlice {
        slice: u8,
        source: IntentSource,
    },
    /// **M14A**: mouse-wheel cycle within current slot's category.
    ActPlayerWeaponCycle {
        direction: i8,
        source: IntentSource,
    },
    /// **M5**: repair a chassis zone (`zone` is `head | torso | arm_left | ...`).
    /// `reason` carries the operator label (`field_kit`, `repair_drone`, etc.).
    ActChassisRepair {
        zone: Option<String>,
        module_id: Option<String>,
        reason: String,
        source: IntentSource,
    },
    /// **M5**: salvage a wrecked chassis. Pulls surviving modules into
    /// `chassis.salvaged_modules`.
    ActChassisSalvage {
        reason: String,
        source: IntentSource,
    },
    /// **M5**: manually clear a weapon jam.
    ActChassisClearJam {
        source: IntentSource,
    },
    /// **M13** § "Brain hopping" — transfer control to a different
    /// friendly actor; the prior actor stays at its position as a
    /// mission-critical AI fallback.
    ActPlayerBrainHop {
        target_actor_id: u64,
        source: IntentSource,
    },
    /// **M13** § "Chassis ability slots" — activate one ability.
    ActPlayerActivateAbility {
        ability: String,
        source: IntentSource,
    },
    /// **M13** § "Cockpit camera anchor" — switch camera anchor mode.
    ActInputCameraAnchor {
        mode: String,
        source: IntentSource,
    },
    /// **M13** § "Drone allies" — switch drone ally mode.
    ActPlayerSetDroneMode {
        mode: String,
        source: IntentSource,
    },
    /// **M13** § "Weapon modifier slots" — attach a Noita-style modifier.
    ActPlayerAttachModifier {
        modifier: String,
        source: IntentSource,
    },
    /// **M13** § "Weapon modifier slots" — detach a modifier.
    ActPlayerDetachModifier {
        modifier: String,
        source: IntentSource,
    },
    /// **M13** § "Boarding / disembarking transitions" — start boarding into
    /// a chassis actor (1500ms transition).
    ActPlayerBoard {
        chassis_actor_id: u64,
        source: IntentSource,
    },
    /// **M13** § "Boarding / disembarking transitions" — start disembarking
    /// out of the current chassis (1500ms transition).
    ActPlayerDisembark {
        source: IntentSource,
    },
    /// **M1**: sticky sharp-aim hold (CCCP AHuman.cpp:1779). `active=true`
    /// asks the sim to build `sharp_aim_progress`; `active=false` releases.
    ActPlayerSharpAim {
        active: bool,
        source: IntentSource,
    },
    /// **M1 / Gap S3**: stub for M1.5 mission abort. M1 rejects with
    /// `unsupported_in_m1`; M1.5 swaps in real abort logic without rewiring
    /// the cfctl surface.
    ActPlayerAbort {
        source: IntentSource,
    },
    /// **M1.5**: pause mission objective progress + timer (tutorial-modal
    /// pause path). Emits `mission.objective_paused`.
    ActMissionPause {
        source: IntentSource,
    },
    /// **M1.5**: resume after pause. Emits `mission.objective_resumed`.
    ActMissionResume {
        source: IntentSource,
    },
    /// **M1 / Gap D1**: UI tells the engine an overlay (settings panel,
    /// debrief prompt, future pause menu) has captured input. While
    /// captured, all `act.player.*` commands are rejected with
    /// `controls_captured` and the CONTROLS CAPTURED HUD zone surfaces.
    ActInputCaptureControls {
        captured: bool,
        capturer: Option<String>,
        source: IntentSource,
    },
    /// **M2**: cycle / set the material overlay mode.
    ActToggleMaterialOverlay {
        mode: Option<String>,
        source: IntentSource,
    },
    /// **M6**: umbrella dispatch for the 26 new tactical-controller actions
    /// (sprint, slide, vault, lean, stealth kill, knife throw, weapon swap,
    /// drop / pickup, signals, mark waypoint, deploy bipod, cycle fire mode,
    /// cook / throw grenade, melee bash / kick, use tool, suppressor
    /// attach / detach, set facing). Engine reads the inner action + updates
    /// `ActorState` flags + records the matching control event.
    ActM6 {
        action: crate::m6_actions::M6Action,
        source: IntentSource,
    },
    /// **M6**: issue one of the 4 squad commands to a bot. `bot_actor=None`
    /// broadcasts to all followers.
    ActSquadIssueCommand {
        bot_actor: Option<u64>,
        kind: crate::m6_actions::SquadCommandKindOverWire,
        waypoint: Option<(f32, f32)>,
        source: IntentSource,
    },
    /// **M6**: cancel the named squad member's current command, returning
    /// them to the default `FollowLeader`. Re-emits `squad.command_issued`
    /// with `kind="follow_leader"` so the replay stream stays linear.
    ActSquadCancelCommand {
        actor_id: u64,
        source: IntentSource,
    },
    /// **M7-B**: set a single task weight on an actor's PriorityTable
    /// (clamps to 0..=9). Spec § Smart commandable AI — Per-task override.
    /// Mutates `M7AiWorld.bots[actor].stack.priority` AND emits
    /// `ai.priority_table_changed`.
    ActPlayerSetPriority {
        actor_id: u64,
        task: String,
        weight: u8,
        source: IntentSource,
    },
    /// **M7-B**: set an actor's autonomy mode (FullAuto / Standard /
    /// Manual). Spec § Smart commandable AI — Layer 1 Autonomy mode.
    /// Mutates `M7AiWorld.bots[actor].stack.autonomy` AND emits
    /// `ai.autonomy_mode_changed`.
    ActPlayerSetAutonomyMode {
        actor_id: u64,
        mode: String,
        source: IntentSource,
    },
    /// **M7-B**: replace an actor's role + PriorityTable with one of the
    /// 6 spec-mandated role templates. Spec § Smart commandable AI — 6
    /// role templates. Emits `ai.role_template_applied`.
    ActPlayerApplyRoleTemplate {
        actor_id: u64,
        template_id: String,
        source: IntentSource,
    },
    /// **M7-B**: apply one of the 5 spec-named quick presets (attack /
    /// defend / overwatch / rescue / salvage). Emits
    /// `ai.quick_preset_applied`.
    ActPlayerApplyQuickPreset {
        actor_id: u64,
        preset_id: String,
        source: IntentSource,
    },
    /// **M7B**: issue a verb from the squad-command grammar to a squad.
    /// Spec § "50+ named squad verbs in a data-driven registry".
    ActSquadIssue {
        squad_id: u64,
        verb_id: String,
        args: Vec<serde_json::Value>,
        source: IntentSource,
    },
    /// **M7B**: switch the squad's active formation kind. Spec § "9
    /// formation kinds with per-actor slot resolution".
    ActSquadSetFormation {
        squad_id: u64,
        formation_kind: String,
        source: IntentSource,
    },
    /// **M7B**: assign a sticky role to a squad member. Spec §
    /// "Per-member role assignment is sticky + loadout-aware".
    ActSquadAssignRole {
        squad_id: u64,
        member_actor_id: u64,
        role: String,
        source: IntentSource,
    },
    /// **M7B**: dump the full squad-state JSON view including the verb
    /// registry, formation catalog, and archetype-BT node counts.
    SrvDumpSquadState {
        squad_id: u64,
        source: IntentSource,
    },
    // === M8 cfctl surface ===
    /// **M8**: switch the camera mode (`follow | scope | free_look`).
    ActCameraSetMode {
        mode: String,
        source: IntentSource,
    },
    /// **M8**: trigger a hit-stop pulse (50..200ms; clamped). `trigger`
    /// records the cause label (`melee_hit`, `ap_round_hit`, etc.).
    ActCameraHitStop {
        duration_ms: u32,
        trigger: String,
        actor_id: Option<u64>,
        source: IntentSource,
    },
    /// **M8**: enter sniper scope ADS at the configured `scope_zoom_fov`.
    /// Equivalent to `act.camera.set_mode { mode: "scope" }` but encodes
    /// player intent specifically.
    ActCameraScopeZoom {
        source: IntentSource,
    },
    /// **M8**: toggle free-look (RMB hold). When `active=true` the camera
    /// transitions to FreeLook anchored at `cursor`; when false it
    /// returns to Follow.
    ActCameraFreeLookToggle {
        active: bool,
        cursor: Option<(f32, f32)>,
        max_distance: f32,
        source: IntentSource,
    },
    /// **M8**: enter photo mode. cf-photo's PhotoModeState becomes active;
    /// cf-control mirrors the sim pause + emits `photo_mode.entered`.
    ActPhotoEnter {
        source: IntentSource,
    },
    /// **M8**: exit photo mode.
    ActPhotoExit {
        source: IntentSource,
    },
    /// **M8**: cycle to the next photo filter (none / sepia / b&w /
    /// color_grade / cyberpunk_neon).
    ActPhotoCycleFilter {
        source: IntentSource,
    },
    /// **M8**: capture a photo (records the `photo_mode.shot_taken` event;
    /// the actual PNG export happens in cf-app via cf-photo::export_png).
    ActPhotoShoot {
        source: IntentSource,
    },
    /// **M8**: scrub the replay timeline by `delta_seconds` (negative =
    /// rewind, positive = forward).
    ActReplayScrub {
        delta_seconds: f32,
        source: IntentSource,
    },
    /// **M8**: drop a replay bookmark with the supplied label.
    ActReplayBookmark {
        label: String,
        source: IntentSource,
    },
    /// **M8**: toggle one of the 7 cf-debug overlays (`ai_state |
    /// pathfinding | collision | material | physics | sound | squad`).
    ActDebugToggleOverlay {
        overlay: String,
        source: IntentSource,
    },
    /// **M8**: set a HUD widget's draggable position; emits
    /// `ux.hud_layout_changed`.
    ActUiSetHudLayout {
        node: String,
        x: f32,
        y: f32,
        source: IntentSource,
    },
    /// **M8**: save the current HUD layout under `name`; emits
    /// `ux.preset_saved`.
    ActUiSavePreset {
        name: String,
        source: IntentSource,
    },
    /// **M8**: toggle the Tab tactical overlay; emits
    /// `ux.tactical_overlay_toggled`. `multiplayer` controls the
    /// sim-speed cap (single-player pauses; multiplayer = 25%).
    ActPlayerToggleTacticalOverlay {
        multiplayer: bool,
        source: IntentSource,
    },
    /// **M8**: drop a multi-step plan onto a squadmate (max 8 steps).
    /// Emits `ai.plan_composed`.
    ActPlayerComposePlan {
        actor_id: u64,
        steps: Vec<String>,
        source: IntentSource,
    },
    /// **M8**: pick a slot on the Q-hold context wheel for `actor_id`.
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
    /// **M8**: M / R / G panic surface. `kind` is `medic`, `engineer`,
    /// or `grenade`. Emits `ai.panic_call_emitted`.
    ActPlayerPanicCall {
        kind: String,
        source: IntentSource,
    },
    /// **M8**: MMB tag drop on `target_id`. Emits `ai.target_tagged` +
    /// engine raises Utility weight by +0.5 for engaging the target.
    ActPlayerTagTarget {
        target_id: u64,
        source: IntentSource,
    },
    /// **M8**: 'Why?' (Y) key — surfaces the bot's `reason_label_recent`
    /// ringbuffer head as a HUD popup. Emits `ai.reason_query_returned`.
    ActPlayerQueryWhy {
        actor_id: u64,
        source: IntentSource,
    },
    /// **M8**: open the T-key 8-slice pie menu with target context
    /// (`void` / `nearest_actor` / `door` / `item`). Emits
    /// `ux.pie_menu_opened`. Slows sim to 20% in single-player; 100% in
    /// multiplayer.
    ActPlayerPieMenuOpen {
        target_kind: String,
        target_id: Option<u64>,
        multiplayer: bool,
        source: IntentSource,
    },
    /// **M8**: select a 0..=7 slot on the open pie menu. Emits
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
    /// **M8**: close the pie menu (idempotent). Emits
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
    /// **M9B-2**: drop an authored trench template at the supplied tile
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
    /// **M9B-3 / VAL-M9B-DIG-001..003 / VAL-M9B-CFCTL-001**: dig a
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
    /// **M9B-3 / VAL-M9B-MODULES-002 / VAL-M9B-CFCTL-001**: place an
    /// embedded module on a built trench segment. `module_id` is one of
    /// the 6 declared modules (`duckboard`, `fire_step`, `breastwork`,
    /// `drainage_sump`, `revetment`, `corner_traverse`).
    ActPlayerPlaceTrenchModule {
        module_id: String,
        segment_id: u64,
        source: IntentSource,
    },
    /// **M9B-3 / VAL-M9B-MODULES-003 / VAL-M9B-CFCTL-001**: repair a
    /// damaged trench module. Consumes the declared per-module
    /// resources (wood/iron); emits `trench.module_repaired`.
    ActPlayerRepairTrenchModule {
        module_id: String,
        segment_id: u64,
        source: IntentSource,
    },
    /// **M14H**: apply a treatment producer to a target. `kind` is the
    /// canonical PascalCase TreatmentKind id; the engine resolves it via
    /// `cf_treatment::TreatmentKind::from_str`.
    ActPlayerTreat {
        kind: String,
        target_actor_id: u64,
        source: IntentSource,
    },
    /// **M14H**: start a 30s Medical Scanner read against a target.
    ActPlayerScan {
        target_actor_id: u64,
        source: IntentSource,
    },
    /// **M14H**: apply one CPR round (20s of compressions) to a target
    /// in cardiac arrest.
    ActPlayerCprRound {
        target_actor_id: u64,
        source: IntentSource,
    },
    /// **M14H**: deliver a defibrillator shock to a target.
    ActPlayerDefib {
        target_actor_id: u64,
        source: IntentSource,
    },
    /// **M14H**: begin a 5-phase surgery on a target.
    ActPlayerSurgeryStart {
        target_actor_id: u64,
        wounds_to_treat: u32,
        surgeon_t1: bool,
        seed: Option<u64>,
        source: IntentSource,
    },
    /// **M14H**: open / clear the Patient Detail panel selection.
    ActPlayerTriageSelect {
        target_actor_id: Option<u64>,
        source: IntentSource,
    },
    /// **M14I**: install a prosthetic on a target actor's severed zone.
    ActPlayerInstallProsthetic {
        target_actor_id: u64,
        kind: String,
        zone: String,
        source: IntentSource,
    },
    /// **M14I**: run a maintenance pass on an installed prosthetic.
    ActPlayerMaintainProsthetic {
        target_actor_id: u64,
        zone: String,
        source: IntentSource,
    },
    /// **M14I**: commit an actor's retirement.
    ActPlayerRetireVeteran {
        target_actor_id: u64,
        source: IntentSource,
    },
    /// **M14J**: manual vault override.
    ActPlayerVault {
        source: IntentSource,
    },
    /// **M14J**: wall-jump while in wall-contact grace window.
    ActPlayerWallJump {
        source: IntentSource,
    },
    /// **M14J**: fire grappling-hook gun at a world target.
    ActPlayerFireGrapple {
        target_x: f32,
        target_y: f32,
        source: IntentSource,
    },
    /// **M14J**: continuous rope climb / rappel + swing input.
    ActPlayerRopeInput {
        climb: f32,
        swing: f32,
        source: IntentSource,
    },
    /// **M14J**: release rope; inherit pendulum exit velocity.
    ActPlayerReleaseRope {
        source: IntentSource,
    },
    /// **M14J**: clip onto a deployed zip line.
    ActPlayerZiplineClip {
        line_id: u64,
        source: IntentSource,
    },
    /// **M14J**: engage / release zip-line brake.
    ActPlayerZiplineBrake {
        engaged: bool,
        source: IntentSource,
    },
    /// **M14J**: mount a tamed critter.
    ActPlayerMount {
        critter_id: u64,
        source: IntentSource,
    },
    /// **M14J**: dismount from a critter.
    ActPlayerDismount {
        source: IntentSource,
    },
}
