use std::path::PathBuf;

use clap::{Parser, Subcommand};

use cf_control::{is_supported_key_binding_action, is_supported_key_code_name};

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum OutputFormat {
    Json,
    Pretty,
}

#[derive(Debug, Parser)]
#[command(name = "cfctl", about = "AI/dev control client for the Corefall engine.")]
pub struct Cli {
    /// Optional control endpoint. When unset, commands that need a server attempt to
    /// auto-launch `cf-app --headless-smoke --control-api` and connect to it on
    /// `127.0.0.1:<--auto-launch-port>`.
    #[arg(long, global = true)]
    pub connect: Option<String>,
    #[arg(long, global = true, default_value_t = 17890)]
    pub auto_launch_port: u16,
    /// Subcommands marked (server) require a server connection. With `--no-auto-launch` and no
    /// `--connect`, those subcommands fail.
    #[arg(long, global = true)]
    pub no_auto_launch: bool,
    #[command(subcommand)]
    pub command: Cmd,
}

#[derive(Debug, Subcommand)]
pub enum Cmd {
    Observe {
        #[arg(long)]
        once: bool,
        #[arg(long)]
        stream: bool,
        #[arg(long, default_value_t = 10)]
        hz: u32,
        #[arg(long)]
        scenario: Option<String>,
        /// Optional seed override. When omitted, the scenario manifest's seed is used so that
        /// `cfctl observe --inline` matches `cf-app` for the same scenario (shared determinism contract).
        #[arg(long)]
        seed: Option<u64>,
        #[arg(long, default_value_t = 60)]
        tick_rate_hz: u32,
        #[arg(long)]
        settings: bool,
        #[arg(long, value_enum, default_value_t = OutputFormat::Json)]
        format: OutputFormat,
        /// When set, ignore --connect and run a one-shot inline observation. Useful for CI smoke.
        #[arg(long)]
        inline: bool,
    },
    /// Run a scenario inline for N ticks and optionally write a run bundle.
    Run {
        #[arg(long)]
        scenario: String,
        #[arg(long, default_value_t = 300)]
        ticks: u64,
        /// Optional seed override. When omitted, the scenario manifest's seed is used so that
        /// `cfctl run` matches `cf-app` for the same scenario (shared determinism contract).
        #[arg(long)]
        seed: Option<u64>,
        #[arg(long, default_value_t = 60)]
        tick_rate_hz: u32,
        #[arg(long)]
        write_run_bundle: bool,
        #[arg(long)]
        run_bundle_dir: Option<PathBuf>,
        #[arg(long, default_value_t = 1.0)]
        ui_scale: f32,
        #[arg(long)]
        high_contrast: bool,
        #[arg(long)]
        paced: bool,
        /// **M4B § "Tamper-evident competitive replays"** — enable
        /// per-event BLAKE3 chain mode. Tournament-mode bundles publish
        /// `run_manifest.json.ledger_chain_anchor` and every event carries
        /// `prev_event_hash` + `chained_hash_hex` so any third party can
        /// `cf-mod ledger verify --bundle <path>` to confirm the bundle
        /// wasn't tampered with between record + replay.
        #[arg(long, default_value_t = false)]
        ledger_chain: bool,
        /// **M4B § "Delta baseline cadence is enforced"** — override the
        /// default 600-tick (10 s @ 60 Hz) baseline cadence. 0 disables
        /// snapshot emission entirely.
        #[arg(long, default_value_t = cf_save::delta::DEFAULT_BASELINE_CADENCE_TICKS)]
        delta_baseline_cadence_ticks: u64,
    },
    /// Server-driven `scenario load|reset` over JSON-RPC.
    Scenario {
        #[command(subcommand)]
        action: ScenarioAction,
    },
    /// Server-driven `sim.pause`.
    Pause,
    /// Server-driven `sim.step --ticks N`.
    Step {
        #[arg(long, default_value_t = 1)]
        ticks: u64,
    },
    /// Server-driven `act.*` family. M0 only ships `settings-set`; M1 adds the
    /// `act.player.*` family (`move`, `jump`, `aim`, `fire`, `reload`, `select-item`, `reset`).
    Act {
        #[command(subcommand)]
        action: ActAction,
    },
    /// Execute a control script (server-driven). Scripts live in `game/scripts/cfctl/<name>.cfctl.json`.
    Script {
        #[command(subcommand)]
        action: ScriptAction,
    },
    /// M3B replay viewer / cause-chain / debrief / validate over a run bundle.
    /// Proxies to `cf-tools-replay-viewer` so AI agents and dev scripts can
    /// drive every replay-viewer surface through the canonical cfctl entry
    /// point. Audit-flagged MEDIUM on 2026-05-09 (the docs reference
    /// `cfctl replay scrub` but the original CLI lacked it).
    Replay {
        #[command(subcommand)]
        action: ReplayAction,
    },
    /// **M5**: `cfctl inspect actor` — pull the full ChassisView projection
    /// for the player (or a specific actor id) from `observe.once`. Prints
    /// chassis spec_id, stage, pilot_state, every zone with per-layer integrity,
    /// every module with state + bound_zone, destroyed_zones[],
    /// salvaged_module_ids, eject_ticks_remaining/total, weapon_jammed.
    Inspect {
        #[command(subcommand)]
        action: InspectAction,
    },
    /// **M4A**: query the engine's asset-ledger summary projection via
    /// `observe.assets.ledger_summary`. Prints total + per-category / tier
    /// / status counts. Use `--inline` to skip the server and read the
    /// canonical `content/asset_ledger/ledger.jsonl` directly.
    LedgerSummary {
        #[arg(long, value_enum, default_value_t = OutputFormat::Json)]
        format: OutputFormat,
        #[arg(long)]
        inline: bool,
    },
    /// **M4B**: save subsystem CLI surface. Provides `quicksave`, `quickload`,
    /// `autosave-now`, `list`, `inspect <path>`, `migrate <path> --to <version>`
    /// and `last` (proxy for `observe.save.last`).
    Save {
        #[command(subcommand)]
        action: SaveAction,
    },
    Version,
}

#[derive(Debug, Subcommand)]
pub enum SaveAction {
    /// **M4B § F5 quicksave** — write the current world to
    /// `<dir>/quicksave.cfsave`. Default `<dir>` = `./saves/quicksave`.
    Quicksave {
        #[arg(long, default_value = "saves/quicksave")]
        dir: PathBuf,
    },
    /// **M4B § F9 quickload** — read + migrate `<dir>/quicksave.cfsave`.
    Quickload {
        #[arg(long, default_value = "saves/quicksave")]
        dir: PathBuf,
    },
    /// **M4B § "Mission autosave fires every 60 seconds"** — force a
    /// one-shot autosave even when the timer hasn't elapsed.
    AutosaveNow {
        #[arg(long, default_value = "saves/quicksave")]
        dir: PathBuf,
    },
    /// **M4B**: list every `.cfsave` directory under `dir` with its
    /// schema_version + blake3 + size. Useful for AI agents auditing the
    /// save library.
    List {
        #[arg(long, default_value = "saves")]
        dir: PathBuf,
    },
    /// **M4B § "cf-headless save inspect"** — print schema_version + delta
    /// chain depth + ledger anchor for a single `<path>.cfsave`.
    Inspect { path: PathBuf },
    /// **M4B § "cf-headless save migrate"** — migrate a single
    /// `<path>.cfsave` to `--to <major.minor.patch>` (default: current
    /// build's schema).
    Migrate {
        path: PathBuf,
        #[arg(long)]
        to: Option<String>,
    },
    /// **M4B § "observe.save.last"** — proxy that returns the last save
    /// metadata snapshot the running engine has tracked.
    Last,
}

#[derive(Debug, Subcommand)]
pub enum InspectAction {
    /// Inspect actor by id; omit `--actor` to inspect the player actor.
    Actor {
        #[arg(long)]
        actor: Option<u64>,
        #[arg(long, value_enum, default_value_t = OutputFormat::Pretty)]
        format: OutputFormat,
    },
    /// Inspect chassis state for actor; omit `--actor` to inspect the player chassis.
    Chassis {
        #[arg(long)]
        actor: Option<u64>,
        #[arg(long, value_enum, default_value_t = OutputFormat::Pretty)]
        format: OutputFormat,
    },
    /// **M1**: Inspect equipment preset (full `RifleSpec`).
    Equipment {
        #[arg(long = "preset", short = 'p')]
        preset_id: String,
        #[arg(long, value_enum, default_value_t = OutputFormat::Pretty)]
        format: OutputFormat,
    },
}

#[derive(Debug, Subcommand)]
pub enum ScenarioAction {
    Load {
        scenario: String,
        #[arg(long)]
        seed: Option<u64>,
    },
    Reset,
}

#[derive(Debug, Subcommand)]
pub enum ActAction {
    SettingsSet {
        #[arg(long)]
        ui_scale: Option<f32>,
        #[arg(long)]
        high_contrast: Option<bool>,
        #[arg(long)]
        captions: Option<bool>,
        #[arg(long)]
        reduced_motion: Option<bool>,
        #[arg(long)]
        reduced_shake: Option<bool>,
        #[arg(long)]
        reduced_flash: Option<bool>,
        /// M4A: enable hold-to-confirm on discrete actions.
        #[arg(long)]
        hold_to_confirm: Option<bool>,
        /// M4A: hold-to-confirm threshold in milliseconds (clamped to [50, 2000] server-side).
        #[arg(long)]
        hold_threshold_ms: Option<u32>,
        /// M4A: enable the live key remap table.
        #[arg(long)]
        key_remap_enabled: Option<bool>,
        /// M4A: rebind a single action -> KeyCode-name pair via `--key-binding action=KeyName`
        /// (e.g. `--key-binding fire=KeyF --key-binding move_left=KeyH`). Repeatable.
        #[arg(long = "key-binding", value_parser = parse_key_binding_kv)]
        key_bindings: Vec<(String, String)>,
        /// M4A: clear all key bindings (overrides any --key-binding flags in the same call).
        #[arg(long)]
        clear_key_bindings: bool,
    },
    /// `act.player.move x=<-1..1>` — M1+ scenarios only.
    PlayerMove {
        #[arg(long)]
        x: f32,
        #[arg(long, default_value_t = 0.0)]
        y: f32,
    },
    /// `act.player.jump` — edge-triggered.
    PlayerJump,
    /// `act.player.aim x=<f32> y=<f32>` — vector is normalized server-side.
    PlayerAim {
        #[arg(long)]
        x: f32,
        #[arg(long)]
        y: f32,
    },
    /// `act.player.fire` — edge-triggered single shot.
    PlayerFire {
        #[arg(long, default_value_t = true)]
        pressed: bool,
    },
    /// `act.player.reload` — edge-triggered.
    PlayerReload,
    /// `act.player.select_item slot=<u32>`.
    PlayerSelectItem {
        #[arg(long)]
        slot: u32,
    },
    /// `act.player.reset` — return to spawn with full HP / ammo.
    PlayerReset,
    /// `act.player.dig` — M1.5 soft-breach dig request.
    PlayerDig {
        /// Optional explicit breach id; otherwise the engine picks the nearest in-range strip.
        #[arg(long)]
        target: Option<String>,
    },
    /// `act.input.focus` — M4A keyboard/controller focus traversal (DR-012 ACC-A-04).
    InputFocus {
        /// Direction to advance focus: `next`, `prev`, `set`, or `clear`.
        #[arg(long)]
        direction: String,
        /// Required when `direction=set`: the canonical HUD focusable-node id (e.g. `hud.silhouette`).
        #[arg(long)]
        node: Option<String>,
    },
    /// **M5**: `act.player.crouch` — sticky crouch toggle.
    PlayerCrouch {
        #[arg(long)]
        active: bool,
    },
    /// **M5**: `act.player.climb` — sticky climb toggle.
    PlayerClimb {
        #[arg(long)]
        active: bool,
    },
    /// **M5**: `act.player.jet` — jet thrust toggle (requires Jet module nominal/degraded).
    PlayerJet {
        #[arg(long)]
        active: bool,
    },
    /// **M5**: `act.player.eject` — trigger pilot eject from a chassis.
    PlayerEject,
    /// **M5**: `act.chassis.repair zone=<head|torso|arm_left|...>` and/or `module_id=<id>`.
    ChassisRepair {
        /// Body zone to repair (e.g. `torso`, `arm_right`, `hand_left`).
        #[arg(long)]
        zone: Option<String>,
        /// Module id to repair (e.g. `jet.pack`, `shield.bubble`, `sensor.scope`).
        #[arg(long)]
        module_id: Option<String>,
        /// Operator label recorded in the chassis event (default: `field_kit`).
        #[arg(long, default_value = "field_kit")]
        reason: String,
    },
    /// **M5**: `act.chassis.salvage` — pull every surviving module from a wrecked chassis.
    ChassisSalvage {
        #[arg(long, default_value = "manual")]
        reason: String,
    },
    /// **M5**: `act.chassis.clear_jam` — manually clear a weapon jam.
    ChassisClearJam,
    /// **M9B-2**: `act.player.drop_trench_template id=<template> origin_x=<i32> origin_y=<i32>`.
    /// Drops the authored trench template at the supplied tile origin
    /// and emits `trench.template_dropped` with the template SHA256 +
    /// segment_count + placed/missing fortification arrays. The template
    /// id resolves against `content/trench_templates/<id>.trench.ron`.
    PlayerDropTrenchTemplate {
        #[arg(long)]
        id: String,
        #[arg(long)]
        origin_x: i32,
        #[arg(long)]
        origin_y: i32,
    },
    /// **M9B-3**: `act.player.dig_trench_segment variant=<id>
    /// [tool_id=<id>] [substrate_hardness=<f32>] [strict]`.
    /// Carves a trench segment with the specified variant. Substrate
    /// hardness ≥ 0.5 on `deep` falls back to `shallow_scrape` with a
    /// `trench.segment_variant_downgraded` warning event, or rejects
    /// outright when `--strict` is supplied.
    PlayerDigTrenchSegment {
        #[arg(long)]
        variant: String,
        #[arg(long)]
        tool_id: Option<String>,
        #[arg(long, default_value_t = 0.0)]
        substrate_hardness: f32,
        #[arg(long, default_value_t = false)]
        strict: bool,
    },
    /// **M9B-3**: `act.player.place_trench_module module_id=<id> segment_id=<u64>`.
    /// Places an embedded module on a built trench segment; emits
    /// `trench.module_placed`.
    PlayerPlaceTrenchModule {
        #[arg(long)]
        module_id: String,
        #[arg(long)]
        segment_id: u64,
    },
    /// **M9B-3**: `act.player.repair_trench_module module_id=<id> segment_id=<u64>`.
    /// Repairs a damaged trench module; emits `trench.module_repaired`.
    PlayerRepairTrenchModule {
        #[arg(long)]
        module_id: String,
        #[arg(long)]
        segment_id: u64,
    },
    /// **M14H**: `act.player.treat kind=<TreatmentKind> target=<actor_id>`.
    /// Applies a treatment producer to the target actor. `kind` is one of
    /// the 22 M14H canonical TreatmentKind names (PascalCase, e.g.
    /// `FieldBandageV1`, `Sutures V1`, `DefibrillatorV1`).
    PlayerTreat {
        #[arg(long)]
        kind: String,
        #[arg(long)]
        target: u64,
    },
    /// **M14H**: `act.player.scan target=<actor_id>` — start a 30s Medical
    /// Scanner read.
    PlayerScan {
        #[arg(long)]
        target: u64,
    },
    /// **M14H**: `act.player.cpr_round target=<actor_id>` — apply one CPR
    /// round.
    PlayerCprRound {
        #[arg(long)]
        target: u64,
    },
    /// **M14H**: `act.player.defib target=<actor_id>` — deliver a defib
    /// shock.
    PlayerDefib {
        #[arg(long)]
        target: u64,
    },
    /// **M14H**: `act.player.surgery_start target=<actor_id>
    /// wounds_to_treat=<u32> [surgeon_t1] [seed=<u64>]`.
    PlayerSurgeryStart {
        #[arg(long)]
        target: u64,
        #[arg(long)]
        wounds_to_treat: u32,
        #[arg(long, default_value_t = false)]
        surgeon_t1: bool,
        #[arg(long)]
        seed: Option<u64>,
    },
    /// **M14H**: `act.player.triage_select target=<actor_id?>` — open the
    /// Patient Detail panel. Omit `target` to clear the selection.
    PlayerTriageSelect {
        #[arg(long)]
        target: Option<u64>,
    },
    /// **M14J**: `act.player.vault` — manual vault override (auto-vault is detect-driven).
    PlayerVault,
    /// **M14J**: `act.player.wall_jump` — wall-jump while in contact grace.
    PlayerWallJump,
    /// **M14J**: `act.player.fire_grapple target_x=<f32> target_y=<f32>`.
    PlayerFireGrapple {
        #[arg(long)]
        target_x: f32,
        #[arg(long)]
        target_y: f32,
    },
    /// **M14J**: `act.player.rope_input climb=<-1..1> [swing=<-1..1>]`.
    PlayerRopeInput {
        #[arg(long)]
        climb: f32,
        #[arg(long, default_value_t = 0.0)]
        swing: f32,
    },
    /// **M14J**: `act.player.release_rope` — release embedded rope.
    PlayerReleaseRope,
    /// **M14J**: `act.player.zipline_clip line_id=<u64>`.
    PlayerZiplineClip {
        #[arg(long)]
        line_id: u64,
    },
    /// **M14J**: `act.player.zipline_brake engaged=<bool>`.
    PlayerZiplineBrake {
        #[arg(long)]
        engaged: bool,
    },
    /// **M14J**: `act.player.mount critter_id=<u64>`.
    PlayerMount {
        #[arg(long)]
        critter_id: u64,
    },
    /// **M14J**: `act.player.dismount`.
    PlayerDismount,
}

#[derive(Debug, Subcommand)]
pub enum ScriptAction {
    Run {
        name: String,
        #[arg(long)]
        write_run_bundle: bool,
        #[arg(long, default_value_t = 30)]
        timeout_seconds: u64,
    },
}

/// `cfctl replay <action>` proxies to `cf-tools-replay-viewer` so all
/// replay-tooling surfaces are reachable through cfctl. Pass-through args
/// after `--` for advanced flags.
#[derive(Debug, Subcommand)]
pub enum ReplayAction {
    /// Render the viewer at a tick anchor. `cfctl replay view <bundle>` ≡
    /// `cf-tools-replay-viewer view <bundle>`.
    View {
        bundle_dir: PathBuf,
        #[arg(long)]
        at_tick: Option<u64>,
        #[arg(long, default_value = "")]
        filter: String,
        #[arg(long, default_value_t = 32)]
        tail_len: usize,
        #[arg(long)]
        since_event_id: Option<String>,
        #[arg(long)]
        paused: bool,
        #[arg(long)]
        output: Option<PathBuf>,
        #[arg(long)]
        png: Option<PathBuf>,
    },
    /// Alias for `view` to match the CLI Reference's "scrub" naming.
    /// Identical semantics; rendering is anchored at the tick the user
    /// asks for, so re-invoking with a different `--at-tick` is the
    /// "scrub" affordance.
    Scrub {
        bundle_dir: PathBuf,
        #[arg(long)]
        at_tick: Option<u64>,
        #[arg(long, default_value = "")]
        filter: String,
        #[arg(long, default_value_t = 32)]
        tail_len: usize,
        #[arg(long)]
        since_event_id: Option<String>,
        #[arg(long)]
        paused: bool,
        #[arg(long)]
        output: Option<PathBuf>,
        #[arg(long)]
        png: Option<PathBuf>,
    },
    /// Walk the parent_event_id chain. `cfctl replay cause-chain <bundle>`.
    CauseChain {
        bundle_dir: PathBuf,
        #[arg(long, conflicts_with = "event_type")]
        event_id: Option<String>,
        #[arg(long, conflicts_with = "event_id")]
        event_type: Option<String>,
        #[arg(long, default_value_t = 64)]
        max_depth: usize,
        #[arg(long, default_value_t = false)]
        json: bool,
        #[arg(long)]
        output: Option<PathBuf>,
        #[arg(long, conflicts_with = "json")]
        png: Option<PathBuf>,
    },
    /// Render the debrief. `cfctl replay debrief <bundle>`.
    Debrief {
        bundle_dir: PathBuf,
        #[arg(long, default_value_t = false)]
        json: bool,
        #[arg(long)]
        output: Option<PathBuf>,
        #[arg(long, conflicts_with = "json")]
        png: Option<PathBuf>,
    },
    /// Validate a bundle. `cfctl replay validate <bundle>`.
    Validate { bundle_dir: PathBuf },
    /// **M10B § VAL-M10B-035**: export the bundle to an MP4 via the
    /// `cf-tools-replay-viewer export` pipeline.
    Export {
        bundle_dir: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        list_presets: bool,
        #[arg(long)]
        preset: Option<String>,
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long)]
        presets_dir: Option<PathBuf>,
        /// **VAL-M10B-NO-AUDIO-BASE**: mute the base SFX + music
        /// mix; commentary remains audible.
        #[arg(long, default_value_t = false)]
        no_audio_base: bool,
        /// **VAL-M10B-SLOW-MO**: integer multiplier (`2x` / `4x`) —
        /// non-integer values rejected with a typed error.
        #[arg(long)]
        slow_mo: Option<String>,
    },
    /// **M10B § VAL-M10B-035**: open the egui editor for a bundle.
    /// Headless mode (TTY-less invocations / `--headless`) prints a
    /// structured envelope to stdout and exits with the documented
    /// `74` code.
    Edit {
        bundle_dir: PathBuf,
        #[arg(long, default_value_t = false)]
        headless: bool,
        #[arg(long)]
        camera_script: Option<PathBuf>,
        #[arg(long)]
        scrub_to_tick: Option<u64>,
    },
}

/// Parser for `--key-binding action=KeyName` flags on `cfctl act settings-set`.
/// M4A: lets a CLI invocation rebind individual actions in the
/// `Settings.key_bindings` table without writing JSON-RPC params by hand.
fn parse_key_binding_kv(s: &str) -> std::result::Result<(String, String), String> {
    let (k, v) = s
        .split_once('=')
        .ok_or_else(|| format!("expected `action=KeyName`, got {s:?}"))?;
    if k.is_empty() || v.is_empty() {
        return Err(format!("expected `action=KeyName`, got {s:?}"));
    }
    if !is_supported_key_binding_action(k) {
        return Err(format!(
            "unsupported action {k:?}; run `cfctl act settings-set --help` for supported actions"
        ));
    }
    if !is_supported_key_code_name(v) {
        return Err(format!(
            "unsupported key name {v:?}; use a stable KeyCode name such as KeyF or Numpad8"
        ));
    }
    Ok((k.to_string(), v.to_string()))
}
