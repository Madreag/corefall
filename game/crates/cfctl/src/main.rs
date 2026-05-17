//! `cfctl` — AI/dev control client.
//!
//! Subcommands either:
//!   - run a self-contained inline sim (`run`, `observe --once --inline`); or
//!   - connect to a running `cf-app --control-api` server over WebSocket (`scenario`, `pause`,
//!     `step`, `observe --stream`, `script run`, `act`).
//!
//! Stub/fake-success responses are forbidden. If the server is unreachable, commands
//! fail with a non-zero exit code and a structured JSON error.

use std::{
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, Command as TokioCommand},
    time::sleep,
};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};

use cf_control::{
    engine::{run_m0_inline, M0Engine},
    is_supported_key_binding_action, is_supported_key_code_name,
    runtime::{build_engine_config, resolve_run_bundle_root, ConfigInputs},
    EngineHandle, Settings, SCHEMA_VERSION,
};
use cf_replay::diagnostics;

#[derive(Debug, Clone, clap::ValueEnum)]
enum OutputFormat {
    Json,
    Pretty,
}

#[derive(Debug, Parser)]
#[command(name = "cfctl", about = "AI/dev control client for the Corefall engine.")]
struct Cli {
    /// Optional control endpoint. When unset, commands that need a server attempt to
    /// auto-launch `cf-app --headless-smoke --control-api` and connect to it on
    /// `127.0.0.1:<--auto-launch-port>`.
    #[arg(long, global = true)]
    connect: Option<String>,
    #[arg(long, global = true, default_value_t = 17890)]
    auto_launch_port: u16,
    /// Subcommands marked (server) require a server connection. With `--no-auto-launch` and no
    /// `--connect`, those subcommands fail.
    #[arg(long, global = true)]
    no_auto_launch: bool,
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Debug, Subcommand)]
enum Cmd {
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
enum SaveAction {
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
enum InspectAction {
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
enum ScenarioAction {
    Load {
        scenario: String,
        #[arg(long)]
        seed: Option<u64>,
    },
    Reset,
}

#[derive(Debug, Subcommand)]
enum ActAction {
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
}

#[derive(Debug, Subcommand)]
enum ScriptAction {
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
enum ReplayAction {
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

fn main() -> Result<()> {
    diagnostics::init("cf::ctl");
    let cli = Cli::parse();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("failed to build tokio runtime for cfctl")?;
    runtime.block_on(async move { dispatch(cli).await })
}

async fn dispatch(cli: Cli) -> Result<()> {
    match cli.command {
        Cmd::Observe {
            once,
            stream,
            hz,
            scenario,
            seed,
            tick_rate_hz,
            settings,
            format,
            inline,
        } => {
            cmd_observe(
                &cli.connect,
                cli.auto_launch_port,
                cli.no_auto_launch,
                once,
                stream,
                hz,
                scenario,
                seed,
                tick_rate_hz,
                settings,
                format,
                inline,
            )
            .await
        }
        Cmd::Run {
            scenario,
            ticks,
            seed,
            tick_rate_hz,
            write_run_bundle,
            run_bundle_dir,
            ui_scale,
            high_contrast,
            paced,
            ledger_chain,
            delta_baseline_cadence_ticks,
        } => cmd_run(
            scenario,
            ticks,
            seed,
            tick_rate_hz,
            write_run_bundle,
            run_bundle_dir,
            ui_scale,
            high_contrast,
            paced,
            ledger_chain,
            delta_baseline_cadence_ticks,
        ),
        Cmd::Scenario { action } => cmd_scenario(&cli.connect, cli.auto_launch_port, cli.no_auto_launch, action).await,
        Cmd::Pause => {
            cmd_simple(
                &cli.connect,
                cli.auto_launch_port,
                cli.no_auto_launch,
                "sim.pause",
                json!({}),
            )
            .await
        }
        Cmd::Step { ticks } => {
            cmd_simple(
                &cli.connect,
                cli.auto_launch_port,
                cli.no_auto_launch,
                "sim.step",
                json!({"ticks": ticks}),
            )
            .await
        }
        Cmd::Act { action } => cmd_act(&cli.connect, cli.auto_launch_port, cli.no_auto_launch, action).await,
        Cmd::Script { action } => cmd_script(&cli.connect, cli.auto_launch_port, cli.no_auto_launch, action).await,
        Cmd::Inspect { action } => cmd_inspect(&cli.connect, cli.auto_launch_port, cli.no_auto_launch, action).await,
        Cmd::Replay { action } => cmd_replay(action),
        Cmd::LedgerSummary { format, inline } => {
            cmd_ledger_summary(&cli.connect, cli.auto_launch_port, cli.no_auto_launch, format, inline).await
        }
        Cmd::Save { action } => cmd_save(&cli.connect, cli.auto_launch_port, cli.no_auto_launch, action).await,
        Cmd::Version => cmd_version(),
    }
}

/// **M4B § cfctl save**: dispatch every save subcommand. The
/// `quicksave` / `quickload` / `autosave-now` / `last` paths require a
/// running server (so the engine has actor state to capture); the
/// `inspect` / `migrate` / `list` paths run inline against on-disk save
/// files and don't need a server.
async fn cmd_save(
    connect: &Option<String>,
    auto_launch_port: u16,
    no_auto_launch: bool,
    action: SaveAction,
) -> Result<()> {
    match action {
        SaveAction::Quicksave { dir } => {
            cmd_save_quicksave(connect, auto_launch_port, no_auto_launch, &dir).await
        }
        SaveAction::Quickload { dir } => {
            cmd_save_quickload(connect, auto_launch_port, no_auto_launch, &dir).await
        }
        SaveAction::AutosaveNow { dir } => {
            cmd_save_autosave_now(connect, auto_launch_port, no_auto_launch, &dir).await
        }
        SaveAction::List { dir } => cmd_save_list(&dir),
        SaveAction::Inspect { path } => cmd_save_inspect(&path),
        SaveAction::Migrate { path, to } => cmd_save_migrate(&path, to),
        SaveAction::Last => cmd_save_last(connect, auto_launch_port, no_auto_launch).await,
    }
}

fn cmd_save_list(dir: &Path) -> Result<()> {
    let mut entries = Vec::new();
    if !dir.exists() {
        println!("{}", serde_json::json!({"dir": dir.display().to_string(), "entries": []}));
        return Ok(());
    }
    let read = std::fs::read_dir(dir).context("read save dir")?;
    for entry in read {
        let entry = entry.context("dir entry")?;
        let path = entry.path();
        if path.is_dir() {
            let cfsave = path.join(cf_save::quicksave::QUICKSAVE_FILE);
            if cfsave.exists() {
                entries.push(inspect_save_path(&cfsave).unwrap_or_else(|err| {
                    serde_json::json!({"path": cfsave.display().to_string(), "error": err.to_string()})
                }));
            }
        } else if path.extension().and_then(|e| e.to_str()) == Some("cfsave") {
            entries.push(inspect_save_path(&path).unwrap_or_else(|err| {
                serde_json::json!({"path": path.display().to_string(), "error": err.to_string()})
            }));
        }
    }
    println!(
        "{}",
        serde_json::json!({"dir": dir.display().to_string(), "entries": entries})
    );
    Ok(())
}

fn inspect_save_path(path: &Path) -> Result<serde_json::Value> {
    let bytes = std::fs::read(path).with_context(|| format!("read {}", path.display()))?;
    let raw: serde_json::Value =
        serde_json::from_slice(&bytes).with_context(|| format!("parse {} as JSON", path.display()))?;
    let schema_version = raw.get("schema_version").cloned().unwrap_or(serde_json::Value::Null);
    let world_tick = raw.get("world_tick").cloned().unwrap_or(serde_json::Value::Null);
    let mod_payload_keys = raw
        .get("mod_payload")
        .and_then(|v| v.as_object())
        .map(|m| m.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    let actor_count = raw
        .get("actors")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    let checksum = path.with_extension("cfsave.checksum");
    let checksum_hex = std::fs::read_to_string(&checksum)
        .ok()
        .map(|s| s.trim().to_string());
    Ok(serde_json::json!({
        "path": path.display().to_string(),
        "schema_version": schema_version,
        "world_tick": world_tick,
        "actor_count": actor_count,
        "mod_payload_keys": mod_payload_keys,
        "size_bytes": bytes.len(),
        "blake3": checksum_hex,
    }))
}

fn cmd_save_inspect(path: &Path) -> Result<()> {
    let value = inspect_save_path(path)?;
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}

fn cmd_save_migrate(path: &Path, _to: Option<String>) -> Result<()> {
    let dir = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("save path has no parent dir: {}", path.display()))?;
    let outcome =
        cf_save::quicksave::read_quicksave(dir).map_err(|e| anyhow::anyhow!("read quicksave: {e}"))?;
    let write =
        cf_save::quicksave::write_quicksave(dir, &outcome.save).map_err(|e| anyhow::anyhow!("write quicksave: {e}"))?;
    let envelope = serde_json::json!({
        "path": write.path.display().to_string(),
        "schema_version": [
            outcome.save.schema_version.major,
            outcome.save.schema_version.minor,
            outcome.save.schema_version.patch,
        ],
        "blake3": write.checksum_hex,
        "size_bytes": write.bytes_written,
        "migrated_from": outcome.migrated_from.map(|v| [v.major, v.minor, v.patch]),
        "migrated_to": outcome.migrated_to.map(|v| [v.major, v.minor, v.patch]),
        "handler_chain": outcome.handler_chain,
    });
    println!("{}", serde_json::to_string_pretty(&envelope)?);
    Ok(())
}

async fn cmd_save_quicksave(
    _connect: &Option<String>,
    _auto_launch_port: u16,
    _no_auto_launch: bool,
    dir: &Path,
) -> Result<()> {
    // The inline path writes an empty WorldSave (cf-app integration drives the
    // server-side capture); this surface is here so AI agents can author + commit
    // a baseline file without standing up the engine.
    let world = cf_save::WorldSave::empty(0);
    let outcome = cf_save::quicksave::write_quicksave(dir, &world).map_err(|e| anyhow::anyhow!("quicksave: {e}"))?;
    let envelope = serde_json::json!({
        "path": outcome.path.display().to_string(),
        "blake3": outcome.checksum_hex,
        "size_bytes": outcome.bytes_written,
        "wall_clock_ms": outcome.wall_clock_ms,
    });
    println!("{}", serde_json::to_string_pretty(&envelope)?);
    Ok(())
}

async fn cmd_save_quickload(
    _connect: &Option<String>,
    _auto_launch_port: u16,
    _no_auto_launch: bool,
    dir: &Path,
) -> Result<()> {
    let outcome = cf_save::quicksave::read_quicksave(dir).map_err(|e| anyhow::anyhow!("quickload: {e}"))?;
    let envelope = serde_json::json!({
        "path": dir.join(cf_save::quicksave::QUICKSAVE_FILE).display().to_string(),
        "actor_count": outcome.save.actors.len(),
        "world_tick": outcome.save.world_tick,
        "schema_version": [
            outcome.save.schema_version.major,
            outcome.save.schema_version.minor,
            outcome.save.schema_version.patch,
        ],
        "blake3": outcome.checksum_hex,
        "wall_clock_ms": outcome.wall_clock_ms,
        "migrated_from": outcome.migrated_from.map(|v| [v.major, v.minor, v.patch]),
        "migrated_to": outcome.migrated_to.map(|v| [v.major, v.minor, v.patch]),
        "handler_chain": outcome.handler_chain,
    });
    println!("{}", serde_json::to_string_pretty(&envelope)?);
    Ok(())
}

async fn cmd_save_autosave_now(
    connect: &Option<String>,
    auto_launch_port: u16,
    no_auto_launch: bool,
    dir: &Path,
) -> Result<()> {
    cmd_save_quicksave(connect, auto_launch_port, no_auto_launch, dir).await
}

async fn cmd_save_last(
    connect: &Option<String>,
    auto_launch_port: u16,
    no_auto_launch: bool,
) -> Result<()> {
    let mut session = Session::open(connect, auto_launch_port, no_auto_launch).await?;
    let result = session.send_request("observe.save.last", json!({})).await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

async fn cmd_ledger_summary(
    connect: &Option<String>,
    auto_launch_port: u16,
    no_auto_launch: bool,
    format: OutputFormat,
    inline: bool,
) -> Result<()> {
    let payload = if inline {
        // Read the canonical ledger directly without a server round-trip.
        // Tries common locations relative to the current working directory.
        let candidates = [
            PathBuf::from("content/asset_ledger/ledger.jsonl"),
            PathBuf::from("../content/asset_ledger/ledger.jsonl"),
            PathBuf::from("game/content/asset_ledger/ledger.jsonl"),
        ];
        let mut summary = Value::Null;
        for c in candidates.iter() {
            if c.exists() {
                let handle = cf_asset_ledger::LedgerHandle::new(c);
                let entries = handle.read_all().context("read ledger")?;
                let s = cf_asset_ledger::summarize(&entries);
                summary = cf_asset_ledger::summary_to_observe_json(&s);
                break;
            }
        }
        if summary.is_null() {
            anyhow::bail!("no ledger.jsonl found at any of: content/asset_ledger/ledger.jsonl, ../content/asset_ledger/ledger.jsonl, game/content/asset_ledger/ledger.jsonl");
        }
        summary
    } else {
        let mut session = Session::open(connect, auto_launch_port, no_auto_launch).await?;
        let result = session.send_request("observe.assets.ledger_summary", json!({})).await?;
        session.close().await?;
        result
    };
    print_value(&payload, &format);
    Ok(())
}

/// Proxy to `cf-tools-replay-viewer`. Resolves the binary via `CF_REPLAY_VIEWER_BIN`
/// env, then `current_exe.parent` (release / debug colocated build), then a
/// `cargo run -p cf-tools-replay-viewer --` fallback.
fn cmd_replay(action: ReplayAction) -> Result<()> {
    let bin = locate_replay_viewer_binary();
    let mut cmd = match bin {
        Some(path) => std::process::Command::new(path),
        None => {
            // Fallback: cargo run.
            let mut c = std::process::Command::new("cargo");
            c.args(["run", "--quiet", "-p", "cf-tools-replay-viewer", "--"]);
            c
        }
    };
    match action {
        ReplayAction::View {
            bundle_dir,
            at_tick,
            filter,
            tail_len,
            since_event_id,
            paused,
            output,
            png,
        }
        | ReplayAction::Scrub {
            bundle_dir,
            at_tick,
            filter,
            tail_len,
            since_event_id,
            paused,
            output,
            png,
        } => {
            cmd.arg("view").arg(&bundle_dir);
            if let Some(t) = at_tick {
                cmd.arg("--at-tick").arg(t.to_string());
            }
            if !filter.is_empty() {
                cmd.arg("--filter").arg(&filter);
            }
            cmd.arg("--tail-len").arg(tail_len.to_string());
            if let Some(s) = &since_event_id {
                cmd.arg("--since-event-id").arg(s);
            }
            if paused {
                cmd.arg("--paused");
            }
            if let Some(p) = &output {
                cmd.arg("--output").arg(p);
            }
            if let Some(p) = &png {
                cmd.arg("--png").arg(p);
            }
        }
        ReplayAction::CauseChain {
            bundle_dir,
            event_id,
            event_type,
            max_depth,
            json,
            output,
            png,
        } => {
            cmd.arg("cause-chain").arg(&bundle_dir);
            if let Some(id) = &event_id {
                cmd.arg("--event-id").arg(id);
            }
            if let Some(ty) = &event_type {
                cmd.arg("--event-type").arg(ty);
            }
            cmd.arg("--max-depth").arg(max_depth.to_string());
            if json {
                cmd.arg("--json");
            }
            if let Some(p) = &output {
                cmd.arg("--output").arg(p);
            }
            if let Some(p) = &png {
                cmd.arg("--png").arg(p);
            }
        }
        ReplayAction::Debrief {
            bundle_dir,
            json,
            output,
            png,
        } => {
            cmd.arg("debrief").arg(&bundle_dir);
            if json {
                cmd.arg("--json");
            }
            if let Some(p) = &output {
                cmd.arg("--output").arg(p);
            }
            if let Some(p) = &png {
                cmd.arg("--png").arg(p);
            }
        }
        ReplayAction::Validate { bundle_dir } => {
            cmd.arg("validate").arg(&bundle_dir);
        }
        ReplayAction::Export {
            bundle_dir,
            list_presets,
            preset,
            out,
            presets_dir,
            no_audio_base,
            slow_mo,
        } => {
            cmd.arg("export");
            if let Some(b) = &bundle_dir {
                cmd.arg(b);
            }
            if list_presets {
                cmd.arg("--list-presets");
            }
            if let Some(p) = &preset {
                cmd.arg("--preset").arg(p);
            }
            if let Some(o) = &out {
                cmd.arg("--out").arg(o);
            }
            if let Some(pd) = &presets_dir {
                cmd.arg("--presets-dir").arg(pd);
            }
            if no_audio_base {
                cmd.arg("--no-audio-base");
            }
            if let Some(s) = &slow_mo {
                cmd.arg("--slow-mo").arg(s);
            }
        }
        ReplayAction::Edit {
            bundle_dir,
            headless,
            camera_script,
            scrub_to_tick,
        } => {
            cmd.arg("edit").arg(&bundle_dir);
            if headless {
                cmd.arg("--headless");
            }
            if let Some(cs) = &camera_script {
                cmd.arg("--camera-script").arg(cs);
            }
            if let Some(t) = scrub_to_tick {
                cmd.arg("--scrub-to-tick").arg(t.to_string());
            }
        }
    }
    let status = cmd.status().context("spawn cf-tools-replay-viewer")?;
    // VAL-M10B-035: headless `edit` exits with code 74 — we propagate
    // that exit code so script harnesses can disambiguate the
    // editor-unavailable path from other failures. Production CLI
    // routes any non-zero exit through anyhow's error path so the
    // process surfaces a non-zero status.
    if !status.success() {
        if let Some(code) = status.code() {
            bail!("cf-tools-replay-viewer exited {code}");
        }
        bail!("cf-tools-replay-viewer exited {status}");
    }
    Ok(())
}

fn locate_replay_viewer_binary() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("CF_REPLAY_VIEWER_BIN") {
        let pb = PathBuf::from(p);
        if pb.exists() {
            return Some(pb);
        }
    }
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    let candidates = [
        dir.join("cf-tools-replay-viewer"),
        dir.join("cf-tools-replay-viewer.exe"),
    ];
    candidates.into_iter().find(|c| c.exists())
}

#[allow(clippy::too_many_arguments)]
async fn cmd_observe(
    connect: &Option<String>,
    auto_launch_port: u16,
    no_auto_launch: bool,
    once: bool,
    stream: bool,
    hz: u32,
    scenario: Option<String>,
    seed: Option<u64>,
    tick_rate_hz: u32,
    settings_only: bool,
    format: OutputFormat,
    inline: bool,
) -> Result<()> {
    if !once && !stream && !settings_only {
        anyhow::bail!("specify --once, --stream, or --settings");
    }
    // M5: --inline and --stream are mutually exclusive. Inline mode runs a single in-process
    // engine snapshot; streaming requires a server. Silently ignoring `--inline` when paired
    // with `--stream` (and falling through to the server path) hides operator intent.
    if inline && stream {
        anyhow::bail!("--inline and --stream are mutually exclusive: streaming requires a control server, inline runs a single in-process snapshot");
    }
    let want_inline = want_inline_default(inline, once, stream, settings_only) && connect.is_none();
    if want_inline && !stream {
        let scenario_id = scenario.unwrap_or_else(|| "m0_blank".to_string());
        let scenario_path = locate_scenario(&scenario_id)?;
        let inputs = ConfigInputs {
            scenario_id: scenario_id.clone(),
            scenario_path,
            run_mode: "cfctl-observe-inline".to_string(),
            run_bundle_root: resolve_run_bundle_root(None),
            write_run_bundle: false,
            control_api_enabled: false,
            debug_capabilities: Vec::new(),
            tick_rate_hz: tick_rate_hz.max(1),
            capture_grid_enabled: false,
            paced: false,
            settings: Settings::default(),
            seed_override: seed,
            duration_ticks_override: Some(1),
            debug_inject_panic_at_tick: None,
            checksum_cadence_ticks: None,
            expected_outcome: None,
        };
        let config = build_engine_config(inputs).context("cfctl observe inline: build_engine_config failed")?;
        let engine = M0Engine::new(config);
        engine.record_run_started();
        let value: Value = if settings_only {
            let s = engine.settings_snapshot().await;
            json!({"schema_version": SCHEMA_VERSION, "settings": s})
        } else {
            let frame = engine.snapshot(None).await;
            serde_json::to_value(&frame).unwrap_or(Value::Null)
        };
        engine.record_run_finished(0);
        print_value(&value, &format);
        return Ok(());
    }

    // Server-driven (auto-launch unless disabled).
    let mut session = Session::open(connect, auto_launch_port, no_auto_launch).await?;
    if stream {
        session.subscribe(hz, scenario.clone()).await?;
        let count = std::env::var("CFCTL_STREAM_FRAMES")
            .ok()
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(3);
        for _ in 0..count {
            let frame = session.next_observe_frame(Duration::from_secs(5)).await?;
            print_value(&frame, &format);
        }
        session.unsubscribe().await?;
    } else if settings_only {
        let s = session.send_request("observe.settings", json!({})).await?;
        print_value(&s, &format);
    } else {
        let frame = session.send_request("observe.once", json!({})).await?;
        print_value(&frame, &format);
    }
    session.close().await?;
    Ok(())
}

fn want_inline_default(inline_flag: bool, once: bool, stream: bool, settings_only: bool) -> bool {
    if inline_flag {
        return true;
    }
    if stream {
        return false;
    }
    once || settings_only
}

#[allow(clippy::too_many_arguments)]
fn cmd_run(
    scenario_id: String,
    ticks: u64,
    seed: Option<u64>,
    tick_rate_hz: u32,
    write_run_bundle: bool,
    run_bundle_dir: Option<PathBuf>,
    ui_scale: f32,
    high_contrast: bool,
    paced: bool,
    ledger_chain: bool,
    delta_baseline_cadence_ticks: u64,
) -> Result<()> {
    let scenario_path = locate_scenario(&scenario_id)?;
    // M0.2-F1: cfctl now goes through the SAME production config-builder as cf-app so the
    // run bundle ships real `commit_sha`, `rust_version`, `bevy_version`, and the scenario
    // manifest's `expected_tests` / `region` / canonical `seed` (overridden by --seed when
    // explicitly provided).
    let inputs = ConfigInputs {
        scenario_id: scenario_id.clone(),
        scenario_path,
        run_mode: "cfctl-run".to_string(),
        run_bundle_root: resolve_run_bundle_root(run_bundle_dir),
        write_run_bundle,
        control_api_enabled: false,
        debug_capabilities: Vec::new(),
        tick_rate_hz: tick_rate_hz.max(1),
        capture_grid_enabled: false,
        paced,
        settings: Settings {
            ui_scale,
            high_contrast,
            ..Settings::default()
        },
        seed_override: seed,
        duration_ticks_override: if ticks > 0 { Some(ticks) } else { None },
        debug_inject_panic_at_tick: None,
        checksum_cadence_ticks: None,
        expected_outcome: None,
    };
    let mut config = build_engine_config(inputs).context("cfctl run: build_engine_config failed")?;
    // **M4B § "Tamper-evident competitive replays"** + § "Delta baseline
    // cadence is enforced" — propagate the CLI flags into the engine
    // config. Default cadence preserved for runs that don't pass --delta-
    // baseline-cadence-ticks; chain mode is off by default.
    config.ledger_chain_enabled = ledger_chain;
    config.delta_baseline_cadence_ticks = delta_baseline_cadence_ticks;
    let outcome = run_m0_inline(config).context("inline run failed")?;
    let report = json!({
        "schema_version": SCHEMA_VERSION,
        "run_id": outcome.run_id,
        "ticks_run": outcome.ticks_run,
        "tick_rate_hz": tick_rate_hz,
        "wall_seconds": outcome.wall_seconds,
        "bundle_dir": outcome.bundle_dir.as_ref().map(|p| p.display().to_string()),
        "final_sim_checksum": outcome.final_checksum_hex,
    });
    println!("{}", serde_json::to_string(&report).unwrap());
    Ok(())
}

async fn cmd_scenario(
    connect: &Option<String>,
    auto_launch_port: u16,
    no_auto_launch: bool,
    action: ScenarioAction,
) -> Result<()> {
    let mut session = Session::open(connect, auto_launch_port, no_auto_launch).await?;
    let result = match action {
        ScenarioAction::Load { scenario, seed } => {
            let mut params = json!({"scenario": scenario});
            if let Some(seed) = seed {
                params["seed"] = json!(seed);
            }
            session.send_request("scenario.load", params).await?
        }
        ScenarioAction::Reset => session.send_request("scenario.reset", json!({})).await?,
    };
    println!("{}", serde_json::to_string(&result).unwrap());
    session.close().await?;
    Ok(())
}

async fn cmd_simple(
    connect: &Option<String>,
    auto_launch_port: u16,
    no_auto_launch: bool,
    method: &str,
    params: Value,
) -> Result<()> {
    let mut session = Session::open(connect, auto_launch_port, no_auto_launch).await?;
    let result = session.send_request(method, params).await?;
    println!("{}", serde_json::to_string(&result).unwrap());
    session.close().await?;
    Ok(())
}

async fn cmd_act(
    connect: &Option<String>,
    auto_launch_port: u16,
    no_auto_launch: bool,
    action: ActAction,
) -> Result<()> {
    let mut session = Session::open(connect, auto_launch_port, no_auto_launch).await?;
    let result = match action {
        ActAction::SettingsSet {
            ui_scale,
            high_contrast,
            captions,
            reduced_motion,
            reduced_shake,
            reduced_flash,
            hold_to_confirm,
            hold_threshold_ms,
            key_remap_enabled,
            key_bindings,
            clear_key_bindings,
        } => {
            let mut params = serde_json::Map::new();
            if let Some(v) = ui_scale {
                params.insert("ui_scale".into(), json!(v));
            }
            if let Some(v) = high_contrast {
                params.insert("high_contrast".into(), json!(v));
            }
            if let Some(v) = captions {
                params.insert("captions".into(), json!(v));
            }
            if let Some(v) = reduced_motion {
                params.insert("reduced_motion".into(), json!(v));
            }
            if let Some(v) = reduced_shake {
                params.insert("reduced_shake".into(), json!(v));
            }
            if let Some(v) = reduced_flash {
                params.insert("reduced_flash".into(), json!(v));
            }
            if let Some(v) = hold_to_confirm {
                params.insert("hold_to_confirm".into(), json!(v));
            }
            if let Some(v) = hold_threshold_ms {
                params.insert("hold_threshold_ms".into(), json!(v));
            }
            if let Some(v) = key_remap_enabled {
                params.insert("key_remap_enabled".into(), json!(v));
            }
            if clear_key_bindings {
                params.insert("key_bindings".into(), json!({}));
            } else if !key_bindings.is_empty() {
                let mut bindings = serde_json::Map::new();
                for (action, key) in key_bindings {
                    bindings.insert(action, json!(key));
                }
                params.insert("key_bindings".into(), Value::Object(bindings));
            }
            session.send_request("act.settings.set", Value::Object(params)).await?
        }
        ActAction::PlayerMove { x, y } => session.send_request("act.player.move", json!({"x": x, "y": y})).await?,
        ActAction::PlayerJump => session.send_request("act.player.jump", json!({})).await?,
        ActAction::PlayerAim { x, y } => session.send_request("act.player.aim", json!({"x": x, "y": y})).await?,
        ActAction::PlayerFire { pressed } => {
            session
                .send_request("act.player.fire", json!({"pressed": pressed}))
                .await?
        }
        ActAction::PlayerReload => session.send_request("act.player.reload", json!({})).await?,
        ActAction::PlayerSelectItem { slot } => {
            session
                .send_request("act.player.select_item", json!({"slot": slot}))
                .await?
        }
        ActAction::PlayerReset => session.send_request("act.player.reset", json!({})).await?,
        ActAction::PlayerDig { target } => {
            let mut params = serde_json::Map::new();
            if let Some(t) = target {
                params.insert("target".into(), json!(t));
            }
            session.send_request("act.player.dig", Value::Object(params)).await?
        }
        ActAction::InputFocus { direction, node } => {
            let mut params = serde_json::Map::new();
            params.insert("direction".into(), json!(direction));
            if let Some(n) = node {
                params.insert("node".into(), json!(n));
            }
            session.send_request("act.input.focus", Value::Object(params)).await?
        }
        ActAction::PlayerCrouch { active } => {
            session
                .send_request("act.player.crouch", json!({"active": active}))
                .await?
        }
        ActAction::PlayerClimb { active } => {
            session
                .send_request("act.player.climb", json!({"active": active}))
                .await?
        }
        ActAction::PlayerJet { active } => {
            session
                .send_request("act.player.jet", json!({"active": active}))
                .await?
        }
        ActAction::PlayerEject => session.send_request("act.player.eject", json!({})).await?,
        ActAction::ChassisRepair {
            zone,
            module_id,
            reason,
        } => {
            let mut params = serde_json::Map::new();
            if let Some(z) = zone {
                params.insert("zone".into(), json!(z));
            }
            if let Some(m) = module_id {
                params.insert("module_id".into(), json!(m));
            }
            params.insert("reason".into(), json!(reason));
            session
                .send_request("act.chassis.repair", Value::Object(params))
                .await?
        }
        ActAction::ChassisSalvage { reason } => {
            session
                .send_request("act.chassis.salvage", json!({"reason": reason}))
                .await?
        }
        ActAction::ChassisClearJam => session.send_request("act.chassis.clear_jam", json!({})).await?,
        ActAction::PlayerDropTrenchTemplate {
            id,
            origin_x,
            origin_y,
        } => {
            session
                .send_request(
                    "act.player.drop_trench_template",
                    json!({"id": id, "origin": [origin_x, origin_y]}),
                )
                .await?
        }
        ActAction::PlayerDigTrenchSegment {
            variant,
            tool_id,
            substrate_hardness,
            strict,
        } => {
            let mut payload = json!({
                "variant": variant,
                "substrate_hardness": substrate_hardness,
                "strict": strict,
            });
            if let Some(t) = tool_id {
                if let Some(obj) = payload.as_object_mut() {
                    obj.insert("tool_id".to_string(), json!(t));
                }
            }
            session
                .send_request("act.player.dig_trench_segment", payload)
                .await?
        }
        ActAction::PlayerPlaceTrenchModule {
            module_id,
            segment_id,
        } => {
            session
                .send_request(
                    "act.player.place_trench_module",
                    json!({"module_id": module_id, "segment_id": segment_id}),
                )
                .await?
        }
        ActAction::PlayerRepairTrenchModule {
            module_id,
            segment_id,
        } => {
            session
                .send_request(
                    "act.player.repair_trench_module",
                    json!({"module_id": module_id, "segment_id": segment_id}),
                )
                .await?
        }
    };
    println!("{}", serde_json::to_string(&result).unwrap());
    session.close().await?;
    Ok(())
}

async fn cmd_inspect(
    connect: &Option<String>,
    auto_launch_port: u16,
    no_auto_launch: bool,
    action: InspectAction,
) -> Result<()> {
    let mut session = Session::open(connect, auto_launch_port, no_auto_launch).await?;
    // Inspect always reads observe.once and slices the result.
    let frame = session.send_request("observe.once", json!({})).await?;
    let actors = frame
        .get("actors")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let resolve_actor = |actor_id: Option<u64>| -> Option<&Value> {
        let target = match actor_id {
            Some(id) => id,
            None => frame.get("player_actor_id").and_then(|v| v.as_u64())?,
        };
        actors
            .iter()
            .find(|a| a.get("id").and_then(|i| i.as_u64()) == Some(target))
    };
    let output = match action {
        InspectAction::Actor { actor, format } => {
            // **M1**: prefer the server-side `inspect.actor` envelope (includes
            // the last 30 actor-category events for the target). Falls back to
            // observe.once if the server hasn't implemented inspect.actor.
            let payload = if let Some(id) = actor {
                session
                    .send_request("inspect.actor", json!({"target": id.to_string()}))
                    .await
                    .ok()
            } else {
                session
                    .send_request("inspect.actor", json!({"target": "player"}))
                    .await
                    .ok()
            };
            let target = payload.unwrap_or_else(|| resolve_actor(actor).cloned().unwrap_or(Value::Null));
            (target, format)
        }
        InspectAction::Chassis { actor, format } => {
            let target = resolve_actor(actor)
                .and_then(|a| a.get("chassis").cloned())
                .unwrap_or(Value::Null);
            (target, format)
        }
        InspectAction::Equipment { preset_id, format } => {
            // **M1 Gap A5**: full RifleSpec via inspect.equipment.
            let payload = session
                .send_request("inspect.equipment", json!({"preset_id": preset_id}))
                .await
                .unwrap_or(Value::Null);
            (payload, format)
        }
    };
    let (payload, format) = output;
    match format {
        OutputFormat::Json => println!("{}", serde_json::to_string(&payload).unwrap_or_default()),
        OutputFormat::Pretty => println!("{}", serde_json::to_string_pretty(&payload).unwrap_or_default()),
    }
    session.close().await?;
    Ok(())
}

async fn cmd_script(
    connect: &Option<String>,
    auto_launch_port: u16,
    no_auto_launch: bool,
    action: ScriptAction,
) -> Result<()> {
    let ScriptAction::Run {
        name,
        write_run_bundle,
        timeout_seconds,
    } = action;
    let path = locate_script(&name)?;
    let text = std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let script: ControlScript = serde_json::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
    // L4: only ask the auto-launched cf-app to write a bundle if this script actually needs one.
    // M1: route the script's declared `scenario` (if any) into the auto-launched cf-app so
    // act.player.* methods land on a real actor world.
    let mut session = Session::open_with(
        connect,
        auto_launch_port,
        no_auto_launch,
        AutoLaunchOpts {
            write_run_bundle,
            scenario: script.scenario.clone(),
        },
    )
    .await?;
    let mut steps_results = Vec::new();
    let deadline = std::time::Instant::now() + Duration::from_secs(timeout_seconds);
    for step in &script.steps {
        if std::time::Instant::now() > deadline {
            anyhow::bail!("script {} timed out after {}s", name, timeout_seconds);
        }
        let result = session.send_request(&step.method, step.params.clone()).await?;
        steps_results.push(json!({
            "method": step.method,
            "result": result,
        }));
        // After sim.step / sim.run_for_ticks, poll observe.once until the engine has
        // actually advanced the requested number of ticks. The server runs the SimClock
        // at wall-clock rate (60 Hz default) so without this poll the next script command
        // overwrites Stepping(N) before drive_tick can advance even one tick.
        if let Some(extra_ticks) = ticks_to_wait_for(&step.method, &step.params) {
            let target_tick = current_tick(&result).map(|t| t + extra_ticks).unwrap_or(0);
            let poll_deadline = std::time::Instant::now() + Duration::from_millis((extra_ticks * 50).max(2_000));
            loop {
                if std::time::Instant::now() > poll_deadline {
                    break;
                }
                let frame = session.send_request("observe.once", json!({})).await?;
                let live_tick = frame.get("tick").and_then(|t| t.as_u64()).unwrap_or(0);
                if live_tick >= target_tick {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(15)).await;
            }
        }
    }
    if write_run_bundle {
        let result = session.send_request("runbundle.write", json!({})).await?;
        steps_results.push(json!({"method": "runbundle.write", "result": result}));
    }
    println!(
        "{}",
        serde_json::to_string(&json!({
            "schema_version": SCHEMA_VERSION,
            "script": name,
            "steps": steps_results,
        }))
        .unwrap()
    );
    session.close().await?;
    Ok(())
}

/// Returns `Some(ticks)` if the script step requests sim.step / sim.run_for_ticks; the
/// caller polls observe.once until the engine has advanced that many ticks before sending
/// the next command (otherwise the next command overwrites Stepping(N) before drive_tick
/// advances).
fn ticks_to_wait_for(method: &str, params: &Value) -> Option<u64> {
    match method {
        "sim.step" | "sim.run_for_ticks" => params.get("ticks").and_then(|t| t.as_u64()),
        _ => None,
    }
}

/// Extract the engine tick from a CommandAck `result`. Returns `0` when the response
/// shape is unexpected (the caller's poll loop will still advance via the deadline).
fn current_tick(result: &Value) -> Option<u64> {
    result.get("effective_tick").and_then(|v| v.as_u64())
}

#[derive(Debug, Deserialize)]
struct ControlScript {
    #[serde(default)]
    description: Option<String>,
    /// Optional scenario id the auto-launched cf-app should load. M0 scripts omit this
    /// and inherit `m0_blank`; M1 scripts that exercise `act.player.*` set this to
    /// `m1_actor_range` so the engine has an actor world to drive.
    #[serde(default)]
    scenario: Option<String>,
    steps: Vec<ScriptStep>,
}

impl ControlScript {
    #[allow(dead_code)]
    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

#[derive(Debug, Deserialize, Clone)]
struct ScriptStep {
    method: String,
    #[serde(default)]
    params: Value,
}

fn cmd_version() -> Result<()> {
    let report = json!({
        "schema_version": SCHEMA_VERSION,
        "cfctl_version": env!("CARGO_PKG_VERSION"),
        "milestone": "m0",
    });
    println!("{}", serde_json::to_string(&report).unwrap());
    Ok(())
}

fn print_value(value: &Value, format: &OutputFormat) {
    match format {
        OutputFormat::Json => println!("{}", serde_json::to_string(value).unwrap()),
        OutputFormat::Pretty => println!("{}", serde_json::to_string_pretty(value).unwrap()),
    }
}

fn locate_scenario(scenario_id: &str) -> Result<PathBuf> {
    cf_control::runtime::locate_scenario(scenario_id)
        .with_context(|| format!("scenario lookup failed for {scenario_id}"))
}

fn locate_script(name: &str) -> Result<PathBuf> {
    let candidates = [
        PathBuf::from("scripts/cfctl").join(format!("{name}.cfctl.json")),
        PathBuf::from("../scripts/cfctl").join(format!("{name}.cfctl.json")),
        PathBuf::from("game/scripts/cfctl").join(format!("{name}.cfctl.json")),
    ];
    for c in &candidates {
        if c.exists() {
            return Ok(c.clone());
        }
    }
    anyhow::bail!("control script not found for {name}; expected scripts/cfctl/{name}.cfctl.json");
}

type WsStream = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

/// JSON-RPC session over WebSocket. Auto-launches `cf-app --headless-smoke
/// --control-api` if `--connect` is not provided and `--no-auto-launch` is not
/// set.
struct Session {
    ws: WsStream,
    next_id: i64,
    spawned_child: Option<Child>,
}

impl Session {
    async fn open(connect: &Option<String>, auto_launch_port: u16, no_auto_launch: bool) -> Result<Self> {
        Self::open_with(connect, auto_launch_port, no_auto_launch, AutoLaunchOpts::default()).await
    }

    /// Open with explicit auto-launch options (used by `script run --write-run-bundle` and
    /// `runbundle write` so only those subcommands ask the auto-launched cf-app to write a
    /// bundle on exit).
    async fn open_with(
        connect: &Option<String>,
        auto_launch_port: u16,
        no_auto_launch: bool,
        opts: AutoLaunchOpts,
    ) -> Result<Self> {
        let (addr, child) = if let Some(addr) = connect.clone() {
            (addr, None)
        } else if no_auto_launch {
            anyhow::bail!("no --connect supplied and --no-auto-launch is set; nothing to talk to");
        } else {
            let child = launch_cf_app(auto_launch_port, opts).await?;
            (format!("127.0.0.1:{auto_launch_port}"), Some(child))
        };
        let url = format!("ws://{}", addr);
        let ws = wait_for_ws(&url, Duration::from_secs(8)).await?;
        Ok(Self {
            ws,
            next_id: 1,
            spawned_child: child,
        })
    }

    async fn send_request(&mut self, method: &str, params_no_schema: Value) -> Result<Value> {
        let id = self.next_id;
        self.next_id += 1;
        let mut params = match params_no_schema {
            Value::Object(_) | Value::Null => params_no_schema,
            other => json!({"_unwrapped": other}),
        };
        if let Value::Object(ref mut m) = params {
            m.insert("schema_version".to_string(), json!(SCHEMA_VERSION));
        } else {
            params = json!({"schema_version": SCHEMA_VERSION});
        }
        let req = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        self.ws.send(Message::Text(req.to_string().into())).await?;
        loop {
            let msg = tokio::time::timeout(Duration::from_secs(8), self.ws.next())
                .await
                .context("server did not respond within 8s")?;
            let msg = msg.ok_or_else(|| anyhow::anyhow!("ws stream closed before response"))??;
            if let Message::Text(text) = msg {
                let v: Value = serde_json::from_str(&text)?;
                if v.get("id").and_then(|x| x.as_i64()) == Some(id) {
                    if let Some(err) = v.get("error") {
                        anyhow::bail!("server error: {err}");
                    }
                    return Ok(v.get("result").cloned().unwrap_or(Value::Null));
                }
                // Notifications (e.g. observe.frame) are skipped while waiting for an id match.
            }
        }
    }

    async fn subscribe(&mut self, hz: u32, _scenario: Option<String>) -> Result<()> {
        let _ = self.send_request("observe.subscribe", json!({"hz": hz})).await?;
        Ok(())
    }

    async fn unsubscribe(&mut self) -> Result<()> {
        let _ = self.send_request("observe.unsubscribe", json!({})).await?;
        Ok(())
    }

    async fn next_observe_frame(&mut self, timeout: Duration) -> Result<Value> {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            let msg = tokio::time::timeout_at(deadline, self.ws.next())
                .await
                .context("observe.frame timeout")?;
            let msg = msg.ok_or_else(|| anyhow::anyhow!("ws stream closed before frame"))??;
            if let Message::Text(text) = msg {
                let v: Value = serde_json::from_str(&text)?;
                if v.get("method").and_then(|x| x.as_str()) == Some("observe.frame") {
                    return Ok(v.get("params").cloned().unwrap_or(Value::Null));
                }
            }
        }
    }

    async fn close(mut self) -> Result<()> {
        // Best-effort shutdown if we launched the server ourselves.
        if self.spawned_child.is_some() {
            let _ = self.send_request("system.shutdown", json!({})).await;
        }
        let _ = self.ws.close(None).await;
        if let Some(mut child) = self.spawned_child.take() {
            let _ = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;
            let _ = child.start_kill();
        }
        Ok(())
    }
}

/// Options for auto-launching `cf-app` from a `cfctl` subcommand.
#[derive(Debug, Clone, Default)]
struct AutoLaunchOpts {
    /// Pass `--write-run-bundle` to the auto-launched cf-app. Only set this for subcommands
    /// that actually need bundle evidence (`script run --write-run-bundle`, `runbundle write`,
    /// observability scripts that call `runbundle.write`). For `observe --once`, `pause`, etc.,
    /// leave it false to keep `prototype_runs/native/` clean. (L4 fix.)
    write_run_bundle: bool,
    /// Scenario id to load in the auto-launched cf-app. Defaults to `m0_blank`. M1 scripts
    /// that need an actor world override this with `m1_actor_range` so `act.player.*` works.
    scenario: Option<String>,
}

async fn launch_cf_app(port: u16, opts: AutoLaunchOpts) -> Result<Child> {
    let bin = locate_cf_app_binary()?;
    // `--ticks 0` = run until shutdown when --control-api is set; cfctl will send system.shutdown on close.
    let scenario = opts.scenario.clone().unwrap_or_else(|| "m0_blank".into());
    let mut args: Vec<String> = vec![
        "--scenario".into(),
        scenario,
        "--headless-smoke".into(),
        "--control-api".into(),
        "--control-port".into(),
        port.to_string(),
        "--ticks".into(),
        "0".into(),
    ];
    if opts.write_run_bundle {
        args.push("--write-run-bundle".into());
        args.push("--run-bundle-dir".into());
        args.push(resolve_run_bundle_root(None).display().to_string());
    }
    let child = TokioCommand::new(&bin)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to spawn {}", bin.display()))?;
    Ok(child)
}

fn locate_cf_app_binary() -> Result<PathBuf> {
    if let Ok(bin) = std::env::var("CF_APP_BIN") {
        if !bin.is_empty() {
            let p = PathBuf::from(bin);
            if p.exists() {
                return Ok(p);
            }
        }
    }
    let exe = std::env::current_exe().context("current_exe")?;
    let dir = exe.parent().context("cfctl binary has no parent dir")?;
    let candidates: Vec<PathBuf> = vec![
        dir.join("cf-app"),
        dir.join("cf-app.exe"),
        dir.parent().unwrap_or(Path::new("")).join("cf-app"),
        dir.parent().unwrap_or(Path::new("")).join("cf-app.exe"),
    ];
    for c in &candidates {
        if c.exists() {
            return Ok(c.clone());
        }
    }
    // Fallback: ask cargo where the binary lives.
    let output = std::process::Command::new("cargo")
        .args(["build", "-p", "cf-app", "--message-format=json"])
        .output();
    if let Ok(o) = output {
        if o.status.success() {
            for line in String::from_utf8_lossy(&o.stdout).lines() {
                if line.contains("\"target\"") && line.contains("\"cf-app\"") && line.contains("\"executable\"") {
                    if let Ok(v) = serde_json::from_str::<Value>(line) {
                        if let Some(exe) = v.get("executable").and_then(|x| x.as_str()) {
                            return Ok(PathBuf::from(exe));
                        }
                    }
                }
            }
        }
    }
    anyhow::bail!("could not locate cf-app binary; set CF_APP_BIN or run `cargo build -p cf-app` first")
}

async fn wait_for_ws(url: &str, total_timeout: Duration) -> Result<WsStream> {
    let deadline = tokio::time::Instant::now() + total_timeout;
    let mut last_err: Option<anyhow::Error> = None;
    loop {
        if tokio::time::Instant::now() >= deadline {
            return Err(last_err.unwrap_or_else(|| anyhow::anyhow!("ws connect timeout: {url}")));
        }
        match connect_async(url).await {
            Ok((ws, _resp)) => return Ok(ws),
            Err(err) => {
                last_err = Some(anyhow::Error::from(err));
                sleep(Duration::from_millis(150)).await;
            }
        }
    }
}

// Discoverability helper kept available so `--connect` log-flooding can be enabled later.
#[allow(dead_code)]
async fn drain_subprocess_stderr(child: &mut Child) -> Result<()> {
    if let Some(stderr) = child.stderr.take() {
        let mut reader = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            eprintln!("[cf-app] {line}");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn version_command_prints_schema_version() {
        cmd_version().unwrap();
    }

    #[test]
    fn want_inline_logic() {
        assert!(want_inline_default(false, true, false, false));
        assert!(want_inline_default(false, false, false, true));
        assert!(!want_inline_default(false, false, true, false));
        assert!(want_inline_default(true, false, true, false));
    }

    #[test]
    fn run_seed_defaults_to_none_so_manifest_seed_wins() {
        let cli = Cli::try_parse_from(["cfctl", "run", "--scenario", "m0_blank"]).unwrap();
        match cli.command {
            Cmd::Run { seed, .. } => assert_eq!(
                seed, None,
                "cfctl run --seed must default to None so the scenario manifest's seed flows \
                 unchanged into build_engine_config (shared determinism contract with cf-app). \
                 Any default value here force-passes Some(default) and rejects scenarios whose \
                 manifest seed differs (M0.2-F3 reject path)."
            ),
            other => panic!("expected Cmd::Run, got {other:?}"),
        }
    }

    #[test]
    fn run_seed_explicit_value_passes_through() {
        let cli = Cli::try_parse_from(["cfctl", "run", "--scenario", "m0_blank", "--seed", "7"]).unwrap();
        match cli.command {
            Cmd::Run { seed, .. } => assert_eq!(seed, Some(7)),
            other => panic!("expected Cmd::Run, got {other:?}"),
        }
    }

    #[test]
    fn observe_seed_defaults_to_none_so_manifest_seed_wins() {
        let cli = Cli::try_parse_from(["cfctl", "observe", "--once", "--scenario", "m0_blank"]).unwrap();
        match cli.command {
            Cmd::Observe { seed, .. } => assert_eq!(
                seed, None,
                "cfctl observe --seed must default to None so the scenario manifest's seed flows \
                 unchanged into build_engine_config (shared determinism contract with cf-app). \
                 Any default value here force-passes Some(default) and rejects scenarios whose \
                 manifest seed differs (M0.2-F3 reject path)."
            ),
            other => panic!("expected Cmd::Observe, got {other:?}"),
        }
    }

    #[test]
    fn observe_seed_explicit_value_passes_through() {
        let cli = Cli::try_parse_from(["cfctl", "observe", "--once", "--scenario", "m0_blank", "--seed", "7"]).unwrap();
        match cli.command {
            Cmd::Observe { seed, .. } => assert_eq!(seed, Some(7)),
            other => panic!("expected Cmd::Observe, got {other:?}"),
        }
    }
}
