use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "cf-mod", about = "Mod and scenario validator/builder.")]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Cmd,
    #[arg(long, global = true)]
    pub(crate) strict: bool,
    #[arg(long, global = true)]
    pub(crate) json: bool,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Cmd {
    /// Walk one or more content/mods directories and validate every scenario manifest found.
    Validate {
        /// Files or directories to validate. If empty, defaults to `content/` (then `../content/`).
        paths: Vec<PathBuf>,
    },
    /// **M1 Gap H2**: validate every event in a run-bundle's `events.jsonl`
    /// against the per-event JSON schemas under `cf-replay/schemas/event/`.
    /// Returns non-zero exit on any payload that fails the schema.
    ValidateBundle {
        /// Path to a run-bundle directory (the one containing `events.jsonl`).
        bundle_dir: PathBuf,
    },
    /// Stubbed in M0; package builder lands at M5/M8.
    Build { pkg_dir: PathBuf },
    /// Stubbed in M0.
    Inspect { cfpkg: PathBuf },
    /// **M4A**: asset-ledger CLI. Append / list / verify / regenerate /
    /// summarize entries in `content/asset_ledger/ledger.jsonl`.
    Ledger {
        #[command(subcommand)]
        action: Box<LedgerAction>,
    },
    /// **M9A**: invoke the Tier-1 SVG asset pipeline. Wraps
    /// `tools/asset_gen/build_placeholders.py` so engine-side tooling can
    /// trigger a bake without shelling out manually.
    #[command(name = "asset-gen")]
    AssetGen {
        #[command(subcommand)]
        action: Box<AssetGenAction>,
    },
    /// **M12A**: invoke the Tier-1 SFX audio pipeline. Wraps
    /// `tools/audio_gen/generate_sfx.py` so engine-side tooling can
    /// trigger an audio bake without shelling out manually. Mirrors the
    /// `asset-gen` subcommand surface (run / check / report).
    #[command(name = "audio-gen")]
    AudioGen {
        #[command(subcommand)]
        action: Box<AudioGenAction>,
    },
    /// **M4B § "cf-mod save validate"** — full schema + migration +
    /// checksum validation pass over a single `.cfsave` file.
    Save {
        #[command(subcommand)]
        action: SaveAction,
    },
}

/// **M4B** save subcommands.
#[derive(Debug, Subcommand)]
pub(crate) enum SaveAction {
    /// Run full validation: schema_version parsable, migration registry
    /// reaches the current version, checksum (when sidecar exists)
    /// matches the canonical-JSON BLAKE3 of the payload.
    Validate { path: PathBuf },
}

/// **M12A**: audio-gen subcommands. Per spec § Files:
/// > `cf-mod` MODIFY — add `cf-mod audio-gen run` subcommand
#[derive(Debug, Subcommand)]
pub(crate) enum AudioGenAction {
    /// Run the full Tier-1 SFX bake. Equivalent to
    /// `tools/asset_gen/.venv/bin/python tools/audio_gen/generate_sfx.py --all`.
    Run {
        /// Optional category filter (e.g. `weapon` / `footstep` / `impact`).
        #[arg(long)]
        category: Option<String>,
        /// Skip the bake; only invoke the pipeline's `--check` dry-run.
        #[arg(long)]
        check: bool,
        /// Print on-disk + ledger SFX counts only.
        #[arg(long)]
        report: bool,
        /// Override the path to the asset pipeline venv's python binary.
        #[arg(long = "venv-python")]
        venv_python: Option<PathBuf>,
        /// Override the path to `generate_sfx.py`.
        #[arg(long = "generate-sfx")]
        generate_sfx: Option<PathBuf>,
        /// Optional mod-pack id for modder authoring (passed to
        /// `generate_sfx.py --mod <id>`).
        #[arg(long)]
        r#mod: Option<String>,
    },
}

/// **M9A**: asset-gen subcommands. Per spec § "Source / cf-mod Cargo.toml":
/// > add `cf-mod asset-gen run` subcommand invoking the Python pipeline
#[derive(Debug, Subcommand)]
pub(crate) enum AssetGenAction {
    /// Run the full Tier-1 bake. Equivalent to
    /// `tools/asset_gen/.venv/bin/python tools/asset_gen/build_placeholders.py --all`.
    Run {
        /// Optional category filter (e.g. `WeaponSprite`).
        #[arg(long)]
        category: Option<String>,
        /// Parallel worker count (0 = serial, 8 = default).
        #[arg(long, default_value_t = 8u32)]
        parallel: u32,
        /// Skip the bake; only invoke the pipeline's `--check` dry-run.
        #[arg(long)]
        check: bool,
        /// Print on-disk + ledger counts only.
        #[arg(long)]
        report: bool,
        /// Override the path to the asset pipeline venv's python binary.
        #[arg(long = "venv-python")]
        venv_python: Option<PathBuf>,
        /// Override the path to `build_placeholders.py`.
        #[arg(long = "build-placeholders")]
        build_placeholders: Option<PathBuf>,
    },
}

/// **M4A** asset-ledger subcommands.
#[derive(Debug, Subcommand)]
#[allow(clippy::large_enum_variant)]
pub(crate) enum LedgerAction {
    /// Append a new entry to the ledger. The output file must already
    /// exist; blake3 is computed at write-time. Re-running with the same
    /// `--canonical-name`+`--tier`+`--category` appends a new line AND
    /// marks the previous entry as `superseded_by` the new one.
    Add {
        #[arg(long)]
        category: String,
        #[arg(long)]
        kind: String,
        #[arg(long = "canonical-name")]
        canonical_name: String,
        #[arg(long)]
        tier: String,
        #[arg(long)]
        pipeline: String,
        #[arg(long)]
        prompt: String,
        #[arg(long = "negative-prompt")]
        negative_prompt: Option<String>,
        #[arg(long)]
        seed: u64,
        #[arg(long = "output-path")]
        output_path: PathBuf,
        #[arg(long = "generator-tool")]
        generator_tool: Option<String>,
        #[arg(long = "generator-model")]
        generator_model: Option<String>,
        #[arg(long = "generator-workflow")]
        generator_workflow: Option<String>,
        #[arg(long = "generator-model-version")]
        generator_model_version: Option<String>,
        /// Palette reference id used by the producing pipeline. Spec
        /// names this `--palette-ref`; both `--palette` (short) and
        /// `--palette-ref` (spec-literal) work.
        #[arg(long, alias = "palette-ref")]
        palette: Option<String>,
        #[arg(long = "style-lora")]
        style_lora: Option<String>,
        #[arg(long)]
        upstream: Vec<String>,
        #[arg(long = "package-source")]
        package_source: Option<String>,
        #[arg(long)]
        license: Option<String>,
        #[arg(long = "generated-by-human")]
        generated_by_human: bool,
        #[arg(long = "human-edit-notes")]
        human_edit_notes: Option<String>,
        #[arg(long = "regen-command")]
        regen_command: Option<String>,
        /// **M4A determinism**: pin the entry's `generated_at_iso` field
        /// instead of using wall-clock time. Combined with the `freeze`
        /// snapshot this makes the ledger byte-reproducible across CI.
        #[arg(long = "generated-at-iso")]
        generated_at_iso: Option<String>,
        /// **M4A determinism**: pin `generated_on_machine`. Defaults to
        /// `HOSTNAME` / `COMPUTERNAME` / `"unknown"` (or `"deterministic"`
        /// when `CF_DETERMINISTIC_LEDGER=1`).
        #[arg(long = "generated-on-machine")]
        generated_on_machine: Option<String>,
        /// Snapshot the canonical output bytes as `<output_path>.frozen`
        /// so future regens can reproduce byte-for-byte (default true so
        /// the deterministic contract holds for non-deterministic pipelines).
        #[arg(long, default_value_t = true)]
        freeze: bool,
        #[arg(long)]
        ledger_path: Option<PathBuf>,
    },
    /// List entries. Use `--category`, `--tier`, `--pipeline`, `--status`
    /// for filtering; `--include-superseded` to walk the full history.
    List {
        #[arg(long)]
        category: Option<String>,
        #[arg(long)]
        tier: Option<String>,
        #[arg(long)]
        pipeline: Option<String>,
        #[arg(long)]
        status: Option<String>,
        #[arg(long = "include-superseded")]
        include_superseded: bool,
        #[arg(long)]
        ledger_path: Option<PathBuf>,
    },
    /// Show a single entry by full hex id, by id prefix, or by canonical_name.
    Show {
        id: String,
        #[arg(long)]
        ledger_path: Option<PathBuf>,
    },
    /// Diff ledger metadata vs the actual disk state.
    Diff {
        /// Optional id; omit to diff every live entry.
        id: Option<String>,
        #[arg(long)]
        ledger_path: Option<PathBuf>,
        #[arg(long)]
        all: bool,
    },
    /// Verify integrity (re-hash and compare). With `--strict`, exits
    /// non-zero on any non-Fresh entry.
    Verify {
        id: Option<String>,
        #[arg(long)]
        all: bool,
        #[arg(long = "strict-status")]
        strict_status: bool,
        #[arg(long)]
        ledger_path: Option<PathBuf>,
        /// **M4B § "Ledger chain rejects tampered bundle"** — verify the
        /// per-event BLAKE3 chain in a run bundle (rather than the asset
        /// ledger). When set, ignores `id` / `all` and walks the bundle's
        /// `events.jsonl` against the manifest's `run_id` + `seed` +
        /// `ledger_chain_anchor`.
        #[arg(long)]
        bundle: Option<PathBuf>,
    },
    /// Re-bake one or more entries. Uses the freeze-then-store path by
    /// default; pipelines may register their own deterministic runner.
    Regenerate {
        id: Option<String>,
        #[arg(long)]
        cascade: bool,
        #[arg(long)]
        category: Option<String>,
        #[arg(long)]
        tier: Option<String>,
        #[arg(long)]
        all: bool,
        #[arg(long = "continue-on-error")]
        continue_on_error: bool,
        #[arg(long)]
        ledger_path: Option<PathBuf>,
    },
    /// Aggregate summary (counts by category / tier / status).
    Summary {
        #[arg(long)]
        ledger_path: Option<PathBuf>,
    },
    /// Compact the ledger: drop superseded history.
    Compact {
        #[arg(long, default_value_t = true)]
        keep_latest: bool,
        #[arg(long)]
        before: Option<String>,
        #[arg(long)]
        ledger_path: Option<PathBuf>,
    },
    /// **M4A § "Mod pack integration"**: walk a mod package directory
    /// and auto-register every asset as a new ledger entry with
    /// `category = Mod_Custom` and `package_source = mod:<mod_id>`.
    /// Optionally writes a sidecar mod manifest that references the
    /// generated ledger entry ids (not raw file paths) per the spec.
    RegisterPack {
        /// Mod package directory (must exist; contains `assets/...`).
        pkg_dir: PathBuf,
        /// Stable mod identifier; used for `package_source = mod:<id>`
        /// and as the canonical_name prefix.
        #[arg(long = "mod-id")]
        mod_id: String,
        /// Production tier for the mod's assets. Default
        /// `Mod_Supplied` per spec; pipelines that want stricter tiers
        /// can override (e.g. `--tier Tier1_SVG`).
        #[arg(long, default_value = "Mod_Supplied")]
        tier: String,
        /// Pipeline id recorded on every entry. Default
        /// `Mod_Supplied_v1`.
        #[arg(long, default_value = "Mod_Supplied_v1")]
        pipeline: String,
        /// Asset roots inside the mod package directory. Defaults to
        /// `assets`. Repeatable.
        #[arg(long = "asset-root")]
        asset_roots: Vec<PathBuf>,
        /// Per-asset license declaration. The author asserts; engine
        /// does NOT verify.
        #[arg(long)]
        license: Option<String>,
        /// Snapshot canonical bytes as `<path>.frozen` so freeze-then-
        /// store regens work for non-deterministic mod content.
        #[arg(long, default_value_t = true)]
        freeze: bool,
        /// Optional path to a sidecar mod manifest file (JSON). When
        /// set, the manifest is written referencing ledger entry ids
        /// per the M4A spec.
        #[arg(long = "manifest-out")]
        manifest_out: Option<PathBuf>,
        /// Override the global canonical ledger path (defaults to
        /// `<workspace>/content/asset_ledger/ledger.jsonl`).
        #[arg(long)]
        ledger_path: Option<PathBuf>,
    },
}
