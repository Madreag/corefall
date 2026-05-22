//! M3B: `cf-tools-replay-viewer` binary.
//!
//! Subcommands:
//! - `view <bundle> [--at-tick N] [--filter cat,cat2] [--tail-len N] [--paused]`
//! - `cause-chain <bundle> [--event-id ID | --event-type T] [--max-depth N] [--json]`
//! - `debrief <bundle> [--json] [--output PATH]`
//! - `validate <bundle>` (load + reject corrupt bundles; exit non-zero on failure).

use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

use cf_tools_replay_viewer::{
    bundle::Bundle,
    cause_chain::{
        render_markdown as cause_render_markdown, render_markdown_multi as cause_render_multi, trace as cause_trace,
        trace_default_triggers, DEFAULT_MAX_DEPTH,
    },
    debrief::{
        compose as debrief_compose, render_json as debrief_render_json, render_markdown as debrief_render_markdown,
    },
    summary::SweepSummary,
    thinking_timeline::{
        build_timeline as thinking_build_timeline, render_json as thinking_render_json,
        render_markdown as thinking_render_markdown, slice_window as thinking_slice_window,
    },
    viewer::{render_markdown as viewer_render_markdown, watch_tail, ViewerState, DEFAULT_TAIL_LEN},
};

#[derive(Debug, Parser)]
#[command(
    name = "cf-tools-replay-viewer",
    about = "M3B: replay viewer + cause-chain + debrief over a run bundle.",
    long_about = "M3B replay viewer.\n\nUsage shapes:\n  cf-tools-replay-viewer <bundle>             # short form: equivalent to `debrief <bundle>` (the milestone's E2E shape).\n  cf-tools-replay-viewer view <bundle> [...]\n  cf-tools-replay-viewer cause-chain <bundle> [...]\n  cf-tools-replay-viewer debrief <bundle> [...]\n  cf-tools-replay-viewer validate <bundle>\n"
)]
struct Cli {
    /// Optional bundle path for the no-subcommand shorthand:
    /// `cf-tools-replay-viewer <bundle>` is equivalent to
    /// `cf-tools-replay-viewer debrief <bundle>`. The roadmap's authoritative
    /// E2E command (`cargo run -p cf-tools-replay-viewer -- prototype_runs/native/<m2_5_run>`)
    /// uses this shape. Audit-flagged HIGH on 2026-05-09: bare-bundle invocation
    /// previously exited 2 because clap required a subcommand.
    #[arg(value_name = "BUNDLE_DIR")]
    bundle_dir: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Cmd>,
}

#[derive(Debug, Subcommand)]
enum Cmd {
    /// M3B-001 + M10: render the viewer shell (event tail / category /
    /// actor / event-type filter / tick scrubber / pause-step indicator)
    /// at the given anchor.
    View {
        bundle_dir: PathBuf,
        /// Inclusive tick anchor; events with tick <= at-tick are visible.
        /// Default: end of run.
        #[arg(long)]
        at_tick: Option<u64>,
        /// Comma-separated category list. Empty / unset means all categories.
        #[arg(long, default_value = "")]
        filter: String,
        /// Tail length cap.
        #[arg(long, default_value_t = DEFAULT_TAIL_LEN)]
        tail_len: usize,
        /// Last seen event id (events after this are highlighted).
        #[arg(long)]
        since_event_id: Option<String>,
        /// Surface the paused state in the viewer header.
        #[arg(long)]
        paused: bool,
        /// Optional output path; default writes markdown to stdout.
        #[arg(long)]
        output: Option<PathBuf>,
        /// Optional PNG output path. Renders the markdown via
        /// `game/tools/markdown_to_png.py` (Pillow). Implies `--output`
        /// when `--output` is unset (writes a sibling `.md` next to
        /// `--png`). Audit-flagged BLOCKER on 2026-05-09: roadmap requires
        /// "Viewer capture in bundle" evidence, which is now this PNG.
        #[arg(long)]
        png: Option<PathBuf>,
        /// `actor_id`/`source_id` or payload `actor_id`/`shooter`/`target`
        /// matches the requested integer.
        #[arg(long)]
        actor: Option<u64>,
        /// exactly matches the requested string.
        #[arg(long)]
        event_type: Option<String>,
        /// `events.jsonl` and emit new events as plain-language sentences
        /// as they're appended (Ctrl-C to exit). Bypasses the markdown
        /// renderer.
        #[arg(long)]
        watch: bool,
        #[arg(long, default_value_t = 100u64)]
        watch_interval_ms: u64,
        /// Default: unbounded.
        #[arg(long)]
        watch_max_iterations: Option<u64>,
        /// (`accessibility.*` + `ux.*` categories). Composes with `--filter`
        /// by adding the categories to the union; takes effect even when
        /// `--filter` is empty.
        #[arg(long, default_value_t = false)]
        accessibility: bool,
    },
    /// M3B-002: walk the parent_event_id chain back from a terminal event
    /// (or from every default trigger if neither --event-id nor --event-type
    /// is provided).
    CauseChain {
        bundle_dir: PathBuf,
        /// Trace from this specific event id.
        #[arg(long, conflicts_with = "event_type")]
        event_id: Option<String>,
        /// Trace from the first event of this type.
        #[arg(long, conflicts_with = "event_id")]
        event_type: Option<String>,
        #[arg(long, default_value_t = DEFAULT_MAX_DEPTH)]
        max_depth: usize,
        /// Emit JSON instead of markdown.
        #[arg(long, default_value_t = false)]
        json: bool,
        /// Optional output path; default writes to stdout.
        #[arg(long)]
        output: Option<PathBuf>,
        /// Optional PNG output path; mirrors `--png` on `view` / `debrief`.
        /// Conflicts with `--json` (PNG renders markdown, not JSON).
        #[arg(long, conflicts_with = "json")]
        png: Option<PathBuf>,
    },
    /// M3B-003: render the debrief summary (outcome / objectives / key
    /// events / damage recap / terrain / checksum status).
    Debrief {
        bundle_dir: PathBuf,
        /// Emit JSON instead of markdown.
        #[arg(long, default_value_t = false)]
        json: bool,
        /// Optional output path; default writes to stdout.
        #[arg(long)]
        output: Option<PathBuf>,
        /// Optional PNG output path; mirrors `--png` on `view` / `cause-chain`.
        /// Conflicts with `--json`.
        #[arg(long, conflicts_with = "json")]
        png: Option<PathBuf>,
    },
    /// M3B-001: load + validate a run bundle, print a one-line PASS or
    /// detailed FAIL message, and exit 0 / 1.
    Validate {
        bundle_dir: PathBuf,
        /// instead of a single-line PASS/FAIL stdout. JSON shape:
        /// `{ run_id, status: "pass"|"fail", error: <kind+message>|null, warnings: [...] }`.
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// the bundle (`scenario @ run_id: result=..., ticks=..., ...`).
    Summary {
        bundle_dir: PathBuf,
        /// Emit JSON instead of the one-line text.
        #[arg(long, default_value_t = false)]
        json: bool,
        /// Optional output path; default writes to stdout.
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// AI thinking-stack timeline for `--actor <id>` from the bundle's
    /// `ai.reason_label_changed` + `ai.thinking_layer_invoked` events.
    ThinkingTimeline {
        bundle_dir: PathBuf,
        /// The actor whose thinking timeline to render.
        #[arg(long)]
        actor: u64,
        /// Inclusive tick anchor; only entries at or before this tick
        /// surface. Defaults to "end of run".
        #[arg(long)]
        at_tick: Option<u64>,
        /// Slice the timeline to the last N entries at or before `at_tick`.
        /// Spec § Per-bot thinking timeline: "last 10 ticks before death".
        #[arg(long)]
        last_n: Option<usize>,
        /// Emit JSON instead of markdown.
        #[arg(long, default_value_t = false)]
        json: bool,
        /// Optional output path; default writes to stdout.
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// via the `cf-replay-export` pipeline.
    ///
    /// `--list-presets` (m10b-1) introspects the preset registry; the
    /// full encode shape (m10b-4) is `export <bundle> --preset <name>
    /// --out <path>` with optional `--no-audio-base` + `--slow-mo
    /// <N>` flags.
    Export {
        /// Optional bundle path. Unused by `--list-presets` (the
        /// preset registry is data-only); required by the encode
        /// shape `export <bundle> --preset <name> --out <path>`.
        #[arg(value_name = "BUNDLE_DIR")]
        bundle_dir: Option<PathBuf>,
        /// presets with the 6 required fields each. Output is JSON for
        /// easy consumption by `jq -e '. | length == 5'`.
        #[arg(long, default_value_t = false)]
        list_presets: bool,
        /// Optional preset name (e.g. `twitch_1080p60`). Default:
        /// `clip_compact` — matches the cf-app debrief CTA + the
        /// Discord-25MB tier.
        #[arg(long)]
        preset: Option<String>,
        /// Optional output path. Default: `~/Movies/Corefall/<run_id>.mp4`
        /// (macOS) or `~/Videos/Corefall/<run_id>.mp4` (Linux/Windows)
        /// resolved via dirs-next per VAL-M10B-DEFAULT-PATH.
        #[arg(long)]
        out: Option<PathBuf>,
        /// Override the preset content directory. Defaults to
        /// `game/content/replay_export/presets/` relative to the
        /// workspace root (resolved by walking up from CWD).
        #[arg(long)]
        presets_dir: Option<PathBuf>,
        /// music mix; commentary remains audible.
        #[arg(long, default_value_t = false)]
        no_audio_base: bool,
        /// `4x` extends output duration deterministically. Non-integer
        /// values (`3.5x`, `1.5`) are rejected with a typed error.
        #[arg(long)]
        slow_mo: Option<String>,
    },
    ///
    /// timeline + scrub + trim + multi-camera angle selector opens.
    /// In headless mode (`--headless`, or stdin not a TTY) prints a
    /// structured JSON envelope to stdout and exits with the
    /// documented `74` exit code so script harnesses can disambiguate
    /// the editor-unavailable path from other failures.
    Edit {
        #[arg(value_name = "BUNDLE_DIR")]
        bundle_dir: PathBuf,
        /// Force headless mode regardless of TTY detection. Used by
        /// `cfctl replay edit --headless` + the test suite.
        #[arg(long, default_value_t = false)]
        headless: bool,
        /// Optional `*.camera.ron` path; the multi-camera angle
        /// selector pre-loads the script's tracks on open.
        #[arg(long)]
        camera_script: Option<PathBuf>,
        /// Optional initial scrub tick — the editor's timeline cursor
        /// lands here on open.
        #[arg(long)]
        scrub_to_tick: Option<u64>,
    },
}

fn init_diagnostics() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn,cf_tools_replay_viewer=info")),
        )
        .with_target(true)
        .try_init();
}

fn main() -> Result<()> {
    init_diagnostics();
    let cli = Cli::parse();
    // Reject the conflicting "both bare bundle AND subcommand" invocation.
    if cli.bundle_dir.is_some() && cli.command.is_some() {
        bail!("either pass a bare bundle path (shorthand for `debrief <bundle>`) OR a subcommand, not both");
    }
    let command = match (cli.bundle_dir, cli.command) {
        (Some(bundle_dir), None) => {
            // Bare-bundle shorthand: equivalent to `debrief <bundle>`.
            // Matches the authoritative E2E command in the roadmap +
            // backlog (`cargo run -p cf-tools-replay-viewer -- <bundle>`).
            Cmd::Debrief {
                bundle_dir,
                json: false,
                output: None,
                png: None,
            }
        }
        (None, Some(cmd)) => cmd,
        (None, None) => {
            bail!("missing required argument: pass a bundle path or a subcommand. See --help.");
        }
        (Some(_), Some(_)) => unreachable!("guarded above"),
    };
    match command {
        Cmd::View {
            bundle_dir,
            at_tick,
            filter,
            tail_len,
            since_event_id,
            paused,
            output,
            png,
            actor,
            event_type,
            watch,
            watch_interval_ms,
            watch_max_iterations,
            accessibility,
        } => {
            if watch {
                // M10 watch mode: bypass markdown, tail events.jsonl,
                // emit plain-language sentences.
                let events_path = bundle_dir.join("events.jsonl");
                let mut stdout = std::io::stdout();
                let _ = watch_tail(&events_path, &mut stdout, watch_interval_ms, watch_max_iterations);
                return Ok(());
            }
            let bundle = load_bundle(&bundle_dir)?;
            // bundle has a ledger_chain_anchor (tournament mode), refuse
            // to render its events if the chain is broken. Dev-mode
            // bundles (no anchor) pass through unchanged.
            if let Some(cf_save::ledger_chain::VerifyOutcome::Tampered { first_break }) =
                verify_bundle_chain(&bundle)
            {
                eprintln!(
                    "FAIL bundle_dir={} ledger_chain_tampered_at={} expected={} actual={}",
                    bundle_dir.display(),
                    first_break.event_id,
                    first_break.expected_hash,
                    first_break.actual_hash
                );
                bail!("ledger chain verification failed; refusing to render tampered bundle");
            }
            // ACC-A audit trail categories (accessibility, ux) to the union.
            let effective_filter = if accessibility {
                if filter.trim().is_empty() {
                    "accessibility,ux".to_string()
                } else {
                    format!("{},accessibility,ux", filter)
                }
            } else {
                filter.clone()
            };
            let state = ViewerState {
                at_tick: at_tick.unwrap_or(u64::MAX),
                filter: ViewerState::parse_filter(&effective_filter),
                tail_len,
                since_event_id,
                paused,
                actor_id_filter: actor,
                event_type_filter: event_type,
            };
            let md = viewer_render_markdown(&bundle, &state);
            write_output(output.as_deref(), &md)?;
            render_png_if_requested(&md, png.as_deref())
        }
        Cmd::CauseChain {
            bundle_dir,
            event_id,
            event_type,
            max_depth,
            json,
            output,
            png,
        } => {
            let bundle = load_bundle(&bundle_dir)?;
            let chain_text = if let Some(id) = event_id {
                let trigger = bundle
                    .event_by_id(&id)
                    .ok_or_else(|| anyhow!("event_id '{id}' not found in bundle"))?;
                let chain = cause_trace(&bundle, trigger, max_depth);
                if json {
                    let v = chain_to_json(&chain);
                    serde_json::to_string_pretty(&v)?
                } else {
                    let mut out = format!("# Cause Chain — `{}`\n\n", bundle.manifest.run_id);
                    out.push_str(&cause_render_markdown(&chain));
                    out
                }
            } else if let Some(ty) = event_type {
                let trigger = bundle
                    .first_event_of_type(&ty)
                    .ok_or_else(|| anyhow!("no event of type '{ty}' in bundle"))?;
                let chain = cause_trace(&bundle, trigger, max_depth);
                if json {
                    let v = chain_to_json(&chain);
                    serde_json::to_string_pretty(&v)?
                } else {
                    let mut out = format!("# Cause Chain — `{}`\n\n", bundle.manifest.run_id);
                    out.push_str(&cause_render_markdown(&chain));
                    out
                }
            } else {
                let chains = trace_default_triggers(&bundle, max_depth);
                if json {
                    let arr: Vec<serde_json::Value> = chains.iter().map(chain_to_json).collect();
                    serde_json::to_string_pretty(&arr)?
                } else {
                    cause_render_multi(&bundle, &chains)
                }
            };
            write_output(output.as_deref(), &chain_text)?;
            if !json {
                render_png_if_requested(&chain_text, png.as_deref())?;
            }
            Ok(())
        }
        Cmd::Debrief {
            bundle_dir,
            json,
            output,
            png,
        } => {
            let bundle = load_bundle(&bundle_dir)?;
            let debrief = debrief_compose(&bundle);
            let text = if json {
                serde_json::to_string_pretty(&debrief_render_json(&debrief))?
            } else {
                debrief_render_markdown(&debrief)
            };
            write_output(output.as_deref(), &text)?;
            if !json {
                render_png_if_requested(&text, png.as_deref())?;
            }
            Ok(())
        }
        Cmd::Validate { bundle_dir, output } => match Bundle::load(&bundle_dir) {
            Ok(bundle) => {
                // bundle"** — run the BLAKE3 ledger chain check before
                // declaring PASS. When the chain anchor is set + the chain
                // doesn't verify, downgrade to FAIL.
                let chain_outcome = verify_bundle_chain(&bundle);
                // audit log"** — record the outcome regardless of pass/fail
                // so the audit trail is preserved.
                if let Some(out) = &chain_outcome {
                    let _ = write_chain_audit_event(&bundle_dir, &bundle.manifest.run_id, out);
                }
                if let Some(cf_save::ledger_chain::VerifyOutcome::Tampered { first_break }) = &chain_outcome {
                    let json = serde_json::json!({
                        "run_id": bundle.manifest.run_id,
                        "status": "fail",
                        "error": format!(
                            "ledger chain tampered at event_id={}",
                            first_break.event_id
                        ),
                        "first_break": {
                            "event_id": first_break.event_id,
                            "expected_hash": first_break.expected_hash,
                            "actual_hash": first_break.actual_hash,
                        },
                        "warnings": [],
                    });
                    if let Some(out_path) = output.as_deref() {
                        write_output(Some(out_path), &serde_json::to_string_pretty(&json)?)?;
                    }
                    eprintln!(
                        "FAIL bundle_dir={} ledger_chain_tampered_at={}",
                        bundle_dir.display(),
                        first_break.event_id
                    );
                    bail!("ledger chain verification failed");
                }
                if let Some(out_path) = output.as_deref() {
                    let json = serde_json::json!({
                        "run_id": bundle.manifest.run_id,
                        "status": "pass",
                        "error": serde_json::Value::Null,
                        "warnings": [],
                        "events_total": bundle.summary.event_counts.total,
                        "first_tick": bundle.summary.first_tick,
                        "last_tick": bundle.summary.last_tick,
                        "ledger_chain": match &chain_outcome {
                            Some(cf_save::ledger_chain::VerifyOutcome::Clean { events_verified, anchor }) => {
                                serde_json::json!({"result": "clean", "events_verified": events_verified, "anchor": anchor})
                            }
                            Some(cf_save::ledger_chain::VerifyOutcome::EmptyChain) => {
                                serde_json::json!({"result": "empty_chain"})
                            }
                            _ => serde_json::Value::Null,
                        },
                    });
                    write_output(Some(out_path), &serde_json::to_string_pretty(&json)?)?;
                } else {
                    println!(
                        "PASS bundle_dir={} run_id={} events={} ticks={}..{}",
                        bundle.bundle_dir.display(),
                        bundle.manifest.run_id,
                        bundle.summary.event_counts.total,
                        bundle
                            .summary
                            .first_tick
                            .map(|t| t.to_string())
                            .unwrap_or_else(|| "n/a".into()),
                        bundle
                            .summary
                            .last_tick
                            .map(|t| t.to_string())
                            .unwrap_or_else(|| "n/a".into()),
                    );
                }
                Ok(())
            }
            Err(e) => {
                if let Some(out_path) = output.as_deref() {
                    let json = serde_json::json!({
                        "run_id": serde_json::Value::Null,
                        "status": "fail",
                        "error": format!("{e}"),
                        "warnings": [],
                    });
                    let _ = write_output(Some(out_path), &serde_json::to_string_pretty(&json)?);
                }
                eprintln!("FAIL bundle_dir={} error={}", bundle_dir.display(), e);
                bail!("validation failed");
            }
        },
        Cmd::Summary {
            bundle_dir,
            json,
            output,
        } => {
            let bundle = load_bundle(&bundle_dir)?;
            let summary = SweepSummary::from_bundle(&bundle);
            let text = if json {
                serde_json::to_string_pretty(&summary.render_json())?
            } else {
                summary.render_text()
            };
            // Ensure stdout always terminates with a newline so sweep
            // pipelines that grep per line behave sensibly.
            let text_with_nl = if text.ends_with('\n') {
                text
            } else {
                format!("{text}\n")
            };
            write_output(output.as_deref(), &text_with_nl)?;
            Ok(())
        }
        Cmd::ThinkingTimeline {
            bundle_dir,
            actor,
            at_tick,
            last_n,
            json,
            output,
        } => {
            let bundle = load_bundle(&bundle_dir)?;
            let entries = thinking_build_timeline(&bundle, actor);
            let sliced = thinking_slice_window(&entries, at_tick, last_n);
            let text = if json {
                serde_json::to_string_pretty(&thinking_render_json(actor, &sliced))?
            } else {
                thinking_render_markdown(actor, &sliced)
            };
            write_output(output.as_deref(), &text)?;
            Ok(())
        }
        Cmd::Export {
            bundle_dir,
            list_presets,
            preset,
            out,
            presets_dir,
            no_audio_base,
            slow_mo,
        } => run_export_dispatch(
            bundle_dir,
            list_presets,
            preset,
            out,
            presets_dir,
            no_audio_base,
            slow_mo,
        ),
        Cmd::Edit {
            bundle_dir,
            headless,
            camera_script,
            scrub_to_tick,
        } => run_edit_dispatch(bundle_dir, headless, camera_script, scrub_to_tick),
    }
}

/// dispatches the `export` subcommand through [`cf_tools_replay_viewer::export_cmd::run_export`].
fn run_export_dispatch(
    bundle_dir: Option<PathBuf>,
    list_presets: bool,
    preset: Option<String>,
    out: Option<PathBuf>,
    presets_dir: Option<PathBuf>,
    no_audio_base: bool,
    slow_mo: Option<String>,
) -> Result<()> {
    use cf_tools_replay_viewer::export_cmd::{
        delete_partial_output, format_missing_ffmpeg_json, run_export, ExportArgs, ExportError, ExportOutcome,
    };
    let args = ExportArgs {
        bundle_dir: bundle_dir.clone(),
        preset,
        out: out.clone(),
        list_presets,
        presets_dir,
        no_audio_base,
        slow_mo,
        dry_run: false,
        force_missing_ffmpeg: false,
    };
    match run_export(args) {
        Ok(ExportOutcome::PresetsListed(p)) => {
            println!("{}", p.json);
            Ok(())
        }
        Ok(ExportOutcome::EncodeCompleted(s)) => {
            tracing::info!(
                target: "cf_tools_replay_viewer::export",
                bytes = s.bytes_written,
                preset = %s.preset.name,
                slow_mo = s.slow_mo.value(),
                no_audio_base = s.no_audio_base,
                "export complete: {}",
                s.out_path.display()
            );
            println!(
                "{}",
                serde_json::json!({
                    "result": "export_complete",
                    "out_path": s.out_path.display().to_string(),
                    "preset": s.preset.name,
                    "codec": s.preset.codec.as_str(),
                    "container": s.preset.container.as_str(),
                    "slow_mo": s.slow_mo.value(),
                    "no_audio_base": s.no_audio_base,
                    "bytes_written": s.bytes_written,
                })
            );
            Ok(())
        }
        Ok(ExportOutcome::DryRun(_)) => unreachable!("dry_run=false above"),
        Err(ExportError::MissingFfmpeg(_)) => {
            if let Some(out_path) = out.as_deref() {
                delete_partial_output(out_path);
            }
            println!("{}", format_missing_ffmpeg_json());
            bail!("missing FFmpeg / libav dependency; see structured JSON above");
        }
        Err(err) => Err(anyhow!(err)),
    }
}

/// [`cf_tools_replay_viewer::edit_cmd::run_edit`].
fn run_edit_dispatch(
    bundle_dir: PathBuf,
    headless: bool,
    camera_script: Option<PathBuf>,
    scrub_to_tick: Option<u64>,
) -> Result<()> {
    use cf_tools_replay_viewer::edit_cmd::{run_edit, EditArgs, EditOutcome};
    let args = EditArgs {
        bundle_dir: Some(bundle_dir),
        headless,
        camera_script,
        scrub_to_tick,
    };
    match run_edit(args)? {
        EditOutcome::Headless(env) => {
            println!("{}", serde_json::to_string_pretty(&env).unwrap_or_default());
            // Propagate the documented headless exit code via std::process::exit;
            // anyhow's Err route would surface a generic non-zero we'd rather avoid.
            std::process::exit(env.exit_code);
        }
        EditOutcome::Interactive {
            bundle,
            opened_at_tick,
            initial_tracks,
        } => {
            tracing::info!(
                target: "cf_tools_replay_viewer::edit",
                bundle = %bundle.display(),
                opened_at_tick,
                track_count = initial_tracks.len(),
                "editor opened interactively"
            );
            println!(
                "{}",
                serde_json::json!({
                    "result": "editor_open",
                    "bundle": bundle.display().to_string(),
                    "opened_at_tick": opened_at_tick,
                    "track_count": initial_tracks.len(),
                })
            );
            Ok(())
        }
    }
}

fn load_bundle(path: &Path) -> Result<Bundle> {
    Bundle::load(path).with_context(|| format!("load run bundle {}", path.display()))
}

/// append one structured entry per validation run. The schema matches
/// `cf-replay/schemas/event/ledger_chain_verified.json`. The audit log
/// lives at `<bundle>/ledger_chain_audit.jsonl` (shared with `cf-mod
/// ledger verify --bundle`) so every verification by either tool persists.
fn write_chain_audit_event(
    bundle_dir: &Path,
    run_id: &str,
    outcome: &cf_save::ledger_chain::VerifyOutcome,
) -> std::io::Result<()> {
    use std::io::Write as _;
    let path = bundle_dir.join("ledger_chain_audit.jsonl");
    let envelope = match outcome {
        cf_save::ledger_chain::VerifyOutcome::Clean { events_verified, anchor } => {
            serde_json::json!({
                "event_type": "ledger_chain_verified",
                "run_id": run_id,
                "result": "clean",
                "events_verified": events_verified,
                "anchor": anchor,
                "verifier": "cf-tools-replay-viewer",
            })
        }
        cf_save::ledger_chain::VerifyOutcome::Tampered { first_break } => serde_json::json!({
            "event_type": "ledger_chain_verified",
            "run_id": run_id,
            "result": "tampered",
            "first_break": {
                "event_id": first_break.event_id,
                "expected_hash": first_break.expected_hash,
                "actual_hash": first_break.actual_hash,
            },
            "verifier": "cf-tools-replay-viewer",
        }),
        cf_save::ledger_chain::VerifyOutcome::EmptyChain => serde_json::json!({
            "event_type": "ledger_chain_verified",
            "run_id": run_id,
            "result": "empty_chain",
            "verifier": "cf-tools-replay-viewer",
        }),
    };
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    writeln!(f, "{}", serde_json::to_string(&envelope)?)?;
    Ok(())
}

/// `cf-tools-replay-viewer validate` and the view header. Returns `None`
/// when the bundle has no `ledger_chain_anchor` (dev mode); otherwise
/// returns the structured outcome from `cf_save::ledger_chain::verify_chain`.
fn verify_bundle_chain(bundle: &Bundle) -> Option<cf_save::ledger_chain::VerifyOutcome> {
    let anchor = bundle.manifest.ledger_chain_anchor.as_deref()?;
    if anchor.is_empty() {
        return None;
    }
    let mut chained = Vec::with_capacity(bundle.events.len());
    for event in &bundle.events {
        let payload_canonical_json = serde_json::to_string(&event.payload).ok()?;
        chained.push(cf_save::ledger_chain::ChainedEvent {
            event_id: event.event_id.clone(),
            payload_canonical_json,
            prev_event_hash: event.prev_event_hash.clone(),
            chained_hash_hex: event.chained_hash_hex.clone().unwrap_or_default(),
        });
    }
    Some(cf_save::ledger_chain::verify_chain(
        &bundle.manifest.run_id,
        bundle.manifest.seed,
        &chained,
    ))
}

fn write_output(output: Option<&Path>, text: &str) -> Result<()> {
    match output {
        Some(p) => {
            if let Some(parent) = p.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent)
                        .with_context(|| format!("create parent dir {}", parent.display()))?;
                }
            }
            std::fs::write(p, text).with_context(|| format!("write {}", p.display()))?;
        }
        None => {
            print!("{text}");
        }
    }
    Ok(())
}

/// Invoke `game/tools/markdown_to_png.py` with the markdown content piped to
/// stdin, writing a PNG to `png_path`. No-op when `png_path` is `None`.
/// The Python helper uses Pillow to render the markdown as a fixed-width text
/// PNG. This satisfies the M3B "viewer capture in bundle" / "death/failure
/// recap screenshot" / "debrief artifact" evidence targets without requiring
/// a heavy GUI dep (bevy_egui, eframe, etc.).
fn render_png_if_requested(markdown: &str, png_path: Option<&Path>) -> Result<()> {
    let Some(png_path) = png_path else {
        return Ok(());
    };
    if let Some(parent) = png_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).with_context(|| format!("create parent dir {}", parent.display()))?;
        }
    }
    let script = locate_markdown_to_png_script()?;
    let mut child = std::process::Command::new("python3")
        .arg(&script)
        .arg("-")
        .arg("--output")
        .arg(png_path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .with_context(|| format!("spawn python3 {}", script.display()))?;
    {
        use std::io::Write;
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("python3 child stdin unavailable"))?;
        stdin
            .write_all(markdown.as_bytes())
            .context("write markdown to python3 stdin")?;
    }
    let output = child.wait_with_output().context("wait for python3")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "markdown_to_png.py exited {} for {}: {}",
            output.status,
            png_path.display(),
            stderr.trim()
        );
    }
    Ok(())
}

/// Find `game/tools/markdown_to_png.py`. Mirrors `cf-e2e`'s composer-script
/// search: walk up from the binary's CWD looking for `game/tools/`.
fn locate_markdown_to_png_script() -> Result<PathBuf> {
    let candidates = [
        std::env::current_dir()
            .ok()
            .map(|p| p.join("game/tools/markdown_to_png.py")),
        std::env::current_dir().ok().map(|p| p.join("tools/markdown_to_png.py")),
        std::env::current_dir()
            .ok()
            .map(|p| p.join("../game/tools/markdown_to_png.py")),
        std::env::current_dir()
            .ok()
            .map(|p| p.join("../../game/tools/markdown_to_png.py")),
    ];
    for c in candidates.into_iter().flatten() {
        if c.exists() {
            return Ok(c);
        }
    }
    bail!(
        "could not locate game/tools/markdown_to_png.py from CWD {}; \
         pass an absolute path via the future --markdown-to-png-script flag, \
         or run from the repo root",
        std::env::current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "<unknown>".into())
    )
}

fn chain_to_json(chain: &cf_tools_replay_viewer::cause_chain::CauseChain<'_>) -> serde_json::Value {
    let term = match chain.terminated_reason {
        cf_tools_replay_viewer::cause_chain::ChainTermination::RootReached => "root_reached",
        cf_tools_replay_viewer::cause_chain::ChainTermination::ParentMissingFromBundle => "parent_missing_from_bundle",
        cf_tools_replay_viewer::cause_chain::ChainTermination::MaxDepthReached => "max_depth_reached",
        cf_tools_replay_viewer::cause_chain::ChainTermination::CycleDetected => "cycle_detected",
    };
    serde_json::json!({
        "trigger": {
            "event_id": chain.trigger.event_id,
            "tick": chain.trigger.tick,
            "category": chain.trigger.category,
            "event_type": chain.trigger.event_type,
            "payload": chain.trigger.payload,
        },
        "links": chain.links.iter().map(|l| serde_json::json!({
            "depth": l.depth,
            "event_id": l.event.event_id,
            "tick": l.event.tick,
            "category": l.event.category,
            "event_type": l.event.event_type,
            "payload": l.event.payload,
            "parent_event_id": l.event.parent_event_id,
        })).collect::<Vec<_>>(),
        "termination": term,
        "depth": chain.links.len(),
    })
}
