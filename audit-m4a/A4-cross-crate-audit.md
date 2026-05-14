# A4 — Cross-Crate Integration Audit

Source spec: `specs/done/M4A.md`. Working tree at `3a32c0a M4: close all audit gaps (BLOCKERS + MAJORS) for M4A readiness` + uncommitted M4A code.

## cf-mod integration

### Subcommands present (`cf-mod ledger <verb>`)

| Verb (per spec) | Present in code? | Evidence |
|---|---|---|
| `add` | YES | `cf-mod/src/main.rs:57` (`LedgerAction::Add`) → `cmd_add` (cf-asset-ledger/cli.rs) |
| `list` | YES | `cf-mod/src/main.rs:110` (`LedgerAction::List`) → `cmd_list` |
| `show` | YES | `cf-mod/src/main.rs:125` (`LedgerAction::Show`) → `cmd_show` |
| `diff` | YES | `cf-mod/src/main.rs:131` (`LedgerAction::Diff`) → `cmd_diff` |
| `verify` | YES | `cf-mod/src/main.rs:141` (`LedgerAction::Verify`) → `cmd_verify` |
| `regenerate` | YES | `cf-mod/src/main.rs:152` (`LedgerAction::Regenerate`) → `cmd_regenerate` |
| `summary` | YES | `cf-mod/src/main.rs:168` (`LedgerAction::Summary`) → `cmd_summary` |
| `compact` | YES | `cf-mod/src/main.rs:173` (`LedgerAction::Compact`) → `cmd_compact` |
| `regenerate --cascade` | YES | `cf-mod/src/main.rs:155` (`cascade: bool` flag) |

All 8 spec subcommands plus `--cascade` flag are wired. Integration test
coverage is in `cf-mod/tests/ledger_cli_integration.rs` (covers all
Gherkin scenarios that map to the verbs).

### Mod-pack auto-registration

**Verdict: DEFERRED (mismatch with spec table).**

- Spec table (M4A.md:23): "cf-mod | MODIFY | … mod-pack publisher writes
  ledger entries automatically."
- Spec Gherkin "Mod pack integration" (M4A.md:235–243): "When the mod is
  packaged via `cf-mod package`: every asset in the mod is registered as a
  new ledger entry. category = Mod_Custom; package_source = mod_id."
- Reality (`cf-mod/src/main.rs:197–207`): `Cmd::Build { pkg_dir }` and
  `Cmd::Inspect { cfpkg }` both `anyhow::bail!` with "not implemented in
  M0; package builder lands at M5/M8". There is no `cf-mod package`
  subcommand at all.
- No code path in `cf-mod` enumerates mod assets and emits ledger entries
  with `category = Mod_Custom` / `package_source = mod_id`.

The hook surface exists in `cf-asset-ledger` (`AssetCategory::Mod_Custom`
in `category.rs`; `PackageRef` field in entries), but the cf-mod side that
the spec promises is unbuilt and cannot be exercised until M5/M8 ships
the package builder.

### `ledger.jsonl` validation

**Verdict: PASS.**

- `cf-mod/src/main.rs:736–778` (`validate_ledger_jsonl`) walks each JSONL
  line, parses it as JSON, calls `cf_asset_ledger::validate_entry_json`,
  reports `id_drift` / `schema_version_mismatch` / `id_not_blake3_hex`
  reasons per line.
- Wired into `validate_one` at `cf-mod/src/main.rs:628–631` (any path
  ending in `ledger.jsonl` routes through `validate_ledger_jsonl`).
- Test coverage: `cf-mod/src/main.rs:1037–1075` (two tests:
  `validate_ledger_jsonl_accepts_well_formed`,
  `validate_ledger_jsonl_rejects_id_drift`).
- The validator is reachable via `cf-mod validate content/` (the CI step
  already in `.github/workflows/ci.yml:53`).

## cf-replay integration

### `Event.asset_ref` envelope field

**Verdict: PASS.**

- `cf-replay/src/lib.rs:110` — `pub asset_ref: Option<String>` on the
  `Event` envelope struct.
- Serde rules at `cf-replay/src/lib.rs:108–110`:
  ```rust
  #[serde(skip_serializing_if = "Option::is_none", default)]
  pub asset_ref: Option<String>,
  ```
  Skips emission when `None` (so legacy bundles round-trip unchanged) and
  defaults to `None` when reading legacy bundles. The envelope is locked
  at `prototype-recorder-event.v0.1` per `EVENT_SCHEMA_VERSION` at
  `cf-replay/src/lib.rs:41`.

### `Recorder::record_with_asset_ref` + `AssetRefRecordParams`

**Verdict: PASS.**

- API signature at `cf-replay/src/lib.rs:585–610`:
  ```rust
  pub fn record_with_asset_ref(&self, params: AssetRefRecordParams<'_>) -> String
  ```
  Records the event via `record_with_cosmetic`, then back-fills
  `last.asset_ref = Some(asset_ref)` on the just-pushed event.
- Params bundle at `cf-replay/src/lib.rs:826–835`: `AssetRefRecordParams<'a>`
  fields = `tick`, `sim_time_ms`, `category`, `event_type`, `payload`,
  `parent_event_id`, `asset_ref: String`, `cosmetic: bool`.
- Test coverage: `cf-replay/src/lib.rs:1138–1167`
  (`record_with_asset_ref_populates_envelope_field`) — asserts the JSON
  round-trip preserves `asset_ref` and `cosmetic`.

### `cf-headless replay` cross-check of `asset_ref` vs ledger

**Verdict: MISSING — spec Gherkin gap.**

Spec Gherkin (M4A.md:230–234):
> Scenario: Run bundle references ledger entries
>   Given a run bundle with capture grid screenshots
>   Then each screenshot in the bundle has an `asset_ref` field linking
>   to a ledger entry
>   And `cf-headless replay` validates that referenced ledger entries
>   exist + are Fresh

Evidence of absence:
- `cf-headless/src/main.rs:1–755` contains no reference to `asset_ref`,
  `cf-asset-ledger`, `ledger`, or `Fresh` (grep for those terms returns
  zero hits).
- `cf-headless/Cargo.toml:13–22` does not list `cf-asset-ledger` as a
  dependency — only `cf-control`, `cf-replay`, `cf-actor`. The crate
  literally cannot call into the ledger.
- No test asserts that a bundle event with `asset_ref` is checked against
  any ledger.

The first half of the Gherkin scenario ("each screenshot has an
`asset_ref` field") is enforced at the producer side via
`record_with_asset_ref` (PASS). The second half ("`cf-headless replay`
validates that referenced ledger entries exist + are Fresh") is **not
wired** — there is no consumer of `asset_ref` in the replay verifier.

### v0.1 recorder schema

**Verdict: PASS.**

- `cf-replay/schemas/v0_1/recorder_event.schema.json:88–91` declares the
  `asset_ref` property as `["string", "null"]` with the M4A description.
- Listed in `properties` (not `required`), so legacy bundles validate.

## cf-control integration

### `observe.assets.ledger_summary` JSON-RPC method

**Verdict: PASS.**

- Dispatcher at `cf-control/src/server.rs:1408–1433` matches
  `"observe.assets.ledger_summary"` → calls
  `engine.observe_assets_ledger_summary().await`; falls back to an
  empty-but-well-formed projection when the engine returns `None`.
- Projection shape (`cf-asset-ledger::summarize` + `summary_to_observe_json`,
  referenced at `cf-control/src/server.rs:547–548` and used by both the
  default path and the dispatcher fallback at line 1419–1432):
  ```json
  {
    "schema_version": 1,
    "total_entries": <u64>,
    "live_entries": <u64>,
    "superseded_entries": <u64>,
    "by_category":   { "<AssetCategory>": <count>, ... },
    "by_tier":       { "<ProductionTier>": <count>, ... },
    "by_status":     { "Fresh": <n>, "Stale": <n>, "Drifted": <n>, "Missing": <n>, "Failed": <n> },
    "missing":   ["<asset_id>", ...],
    "drifted":   ["<asset_id>", ...],
    "failed":    ["<asset_id>", ...],
    "stale":     ["<asset_id>", ...]
  }
  ```
- The spec's "per-category counts + per-pipeline-tier counts +
  missing-entry warnings" are covered by `by_category`, `by_tier`, and
  the four non-Fresh arrays. **NOTE**: the spec says "per-pipeline-tier";
  the implementation provides `by_tier` (`ProductionTier` enum) AND the
  audit's missing/drifted/failed/stale ID lists. There is **no separate
  per-pipeline projection** (e.g. `by_pipeline: { "M9A_svg_v1": n }`); a
  per-pipeline breakdown would require an extra projection over the
  entries. See "Gaps" below.
- Test coverage: `cf-control/src/server.rs:2087–2185` (two tests).

### `EngineHandle::observe_assets_ledger_summary` trait method + default

**Verdict: PASS.**

- Trait method at `cf-control/src/server.rs:513–515` with default impl
  delegating to `default_observe_assets_ledger_summary()`.
- Default at `cf-control/src/server.rs:523–549` searches three candidate
  paths (`content/asset_ledger/ledger.jsonl`,
  `../content/asset_ledger/ledger.jsonl`,
  `game/content/asset_ledger/ledger.jsonl`), reads via
  `cf_asset_ledger::LedgerHandle`, summarizes via
  `cf_asset_ledger::summarize`, and projects via
  `summary_to_observe_json`. Returns `None` when no ledger file exists at
  any candidate path.

### `M0Engine` override

**Verdict: USES_DEFAULT (acceptable per spec).**

- `cf-control/src/engine.rs:6518` opens `impl EngineHandle for M0Engine`,
  but a `grep -n observe_assets_ledger_summary cf-control/src/engine.rs`
  returns zero matches.
- M0Engine therefore inherits the trait default. The default reads the
  canonical on-disk ledger (which lives at
  `game/content/asset_ledger/ledger.jsonl`), so callers from cfctl or
  test scripts running from `game/` get the right answer without an
  override. Acceptable per spec "engines that ship a non-default ledger
  path can override" (cf-control/src/server.rs:519–522).

## cfctl integration

### `cfctl ledger-summary` subcommand

**Verdict: PASS.**

- Subcommand at `cfctl/src/main.rs:154–159` (`LedgerSummary { format,
  inline }`).
- Dispatcher at `cfctl/src/main.rs:507–509` routes to
  `cmd_ledger_summary`.
- `cmd_ledger_summary` at `cfctl/src/main.rs:514–552` supports two modes:
  - `--inline` (no server): reads the local
    `content/asset_ledger/ledger.jsonl` candidates and projects via
    `cf_asset_ledger::{LedgerHandle, summarize, summary_to_observe_json}`.
  - Over-WS (default): opens a `Session`, sends
    `observe.assets.ledger_summary` over JSON-RPC, prints the response.
- Output format honored via the global `OutputFormat` (`Json`
  default) → `print_value`.

### cfctl `m4a_*.cfctl.json` scripts

| Script | Exercises |
|---|---|
| `m4a_ledger_summary.cfctl.json` (NEW, this milestone) | `observe.assets.ledger_summary` twice — pre + post `sim.run_for_ticks` |
| `m4a_acc_a_floor.cfctl.json` | M4A accessibility floor (UI scale, captions, etc.) — not the ledger |
| `m4a_focus_traversal.cfctl.json` | M4A focus traversal — not the ledger |
| `m4a_hold_remap_settings.cfctl.json` | M4A hold-to-confirm + key remap — not the ledger |
| `m4a_micro_breach_readability.cfctl.json` | M4A readability test — not the ledger |

Only `m4a_ledger_summary.cfctl.json` directly exercises the ledger
JSON-RPC surface, which is sufficient for the per-pipeline `observe`
contract.

### cfctl `version` reports `SCHEMA_VERSION`

**Verdict: PASS.** — `cfctl/src/main.rs:1184–1191` prints
`{ "schema_version": SCHEMA_VERSION, "cfctl_version": <pkg>, "milestone": "m0" }`.

## Per-pipeline integration

Spec (M4A.md:325, "Per-pipeline integration"): "Each downstream pipeline
milestone (M9A, M12A, M18A, M24A, M25A, M32A, M37A, M38A, M45A, M48A,
M48B) writes a ledger entry per generated asset."

`regen_manifest.ron` coverage (`game/content/asset_ledger/regen_manifest.ron`):

| Pipeline | Present? | pipeline_id |
|---|---|---|
| M9A SVG | YES | `M9A_svg_v1` |
| M12A audio | YES | `M12A_llm_audio_v1` |
| M18A animation | YES | `M18A_animation_v1` |
| M24A VFX | YES | `M24A_particle_v1` |
| M25A narrative | YES | `M25A_narrative_v1` |
| M32A ComfyUI | YES | `M32A_comfyui_v1` |
| M37A voice | YES | `M37A_voice_v1` |
| M37A music | YES | `M37A_music_v1` |
| M38A localization | YES | `M38A_localization_v1` |
| M45A cosmetic | YES | `M45A_cosmetic_v1` |
| M48A polish | YES | `M48A_polish_v1` |
| **M48B marketing** | **NO** | (not in manifest) |
| Mod_Supplied_v1 | YES (bonus) | `Mod_Supplied_v1` |

**Gap: `M48B_*_v1` pipeline missing from `regen_manifest.ron`.** Spec
explicitly enumerates M48B alongside the others.

## CI integration

### `game/scripts/ledger_audit.sh`

**Verdict: EXISTS but NOT WIRED + has a flag bug.**

- File present: `game/scripts/ledger_audit.sh`, mode `-rwxr-xr-x@` (1040
  bytes; chmod is executable).
- Behavior: changes into `game/`, runs `cargo run --release -q -p cf-mod
  -- ledger verify --strict --all` (or `--json` variant).
- Bug at line 38 of the script: it passes `--strict` (the global cf-mod
  flag declared at `cf-mod/src/main.rs:18`), **NOT** `--strict-status`
  (the verify-subcommand flag declared at `cf-mod/src/main.rs:145–146`).
  The verify handler at `cf-mod/src/main.rs:382–391` only honors
  `strict_status`:
  ```rust
  // Strict mode is forced on when:
  //   * `--strict-status` is explicitly set
  //   * top-level `--strict` is set
  let strict = *strict_status;
  ...
  if strict && !report.is_strict_ok() {
      std::process::exit(1);
  }
  ```
  The comment claims top-level `--strict` should activate strict mode,
  but the code does not consult `cli.strict`. As written, `ledger_audit.sh`
  will print a non-strict verify report and exit 0 even when entries are
  Drifted / Missing / Failed.
- Wiring into CI: `grep -r ledger_audit .github` returns zero matches.
  Neither `.github/workflows/ci.yml` nor `.github/workflows/release.yml`
  invokes `ledger_audit.sh`. The spec calls for nightly CI:
  > Nightly CI runs `cf-mod ledger verify --strict --all` → must pass

  No nightly workflow file exists; no cron schedule references the
  script.

### Pre-commit hook

**Verdict: NOT IMPLEMENTED.**

- Spec (M4A.md:335): "Pre-commit hook runs `cf-mod ledger verify --strict
  <changed files>` → catches local drift before push"
- No `.git/hooks/pre-commit` is installed (`ls .git/hooks` shows only the
  default `.sample` files).
- No `.pre-commit-config.yaml` / `husky` / Lefthook / other hook manager
  config exists in the repo (`grep -r pre-commit /Users/erol/projects/corefall`
  returns zero matches).

### Release CI re-bake from clean checkout

**Verdict: NOT IMPLEMENTED.**

- Spec (M4A.md:336): "Release CI runs `cf-mod ledger regenerate --all`
  from clean checkout → validates full reproducibility"
- `grep -n "ledger\|regenerate" .github/workflows/release.yml` returns
  zero matches.
- No job in `release.yml` runs `cf-mod ledger regenerate --all`.

## Gaps (severity ranked)

### BLOCKER

1. **`ledger_audit.sh` strict-mode flag bug.** Script passes `--strict`,
   but cf-mod only respects `--strict-status` for `ledger verify`. Result:
   the audit script exits 0 even on drift / missing / failed entries,
   silently defeating the audit. Fix: change the script to
   `cf-mod ledger verify --strict-status --all` (and `--json` variant
   same fix) **OR** wire `cli.strict` into `cf-mod/src/main.rs:384`
   so the global flag works as documented.
   - `game/scripts/ledger_audit.sh:38`
   - `game/crates/cf-mod/src/main.rs:382–391` (the doc-comment lies about
     "top-level `--strict` is set" being honored)

### MAJOR

2. **No `cf-headless replay` cross-check of `asset_ref` against the
   ledger.** Spec Gherkin "Run bundle references ledger entries" requires
   `cf-headless replay` to validate that referenced ledger entries exist
   and are `Fresh`. The verifier does not depend on `cf-asset-ledger`
   and does not consume `asset_ref` at all.
   - `game/crates/cf-headless/src/main.rs:1–755` (no reference)
   - `game/crates/cf-headless/Cargo.toml:13–22` (missing
     `cf-asset-ledger` dep)

3. **No nightly CI invocation of `ledger_audit.sh`.** Spec mandates
   nightly. No GitHub Actions workflow runs the script on a schedule or
   on PRs. Even after fixing #1, the gate has no enforcement surface.
   - `.github/workflows/ci.yml` + `release.yml`

4. **Mod-pack auto-registration scenario unverifiable.** Spec table
   promises "mod-pack publisher writes ledger entries automatically" and
   Gherkin tests it via `cf-mod package`. `cf-mod package` does not
   exist; `cf-mod build` / `cf-mod inspect` bail "M0 only; package
   builder lands at M5/M8". The acceptance criterion cannot be
   exercised at M4A close. Either the spec must document this as
   deferred (currently `Out of scope` doesn't list it) or a shim
   `cf-mod package` must enumerate `.cfmod` contents and call
   `cmd_add` per asset.
   - `cf-mod/src/main.rs:197–207`

5. **M48B pipeline missing from `regen_manifest.ron`.** Spec enumerates
   M48B alongside M9A…M48A as a downstream pipeline that writes ledger
   entries. The manifest does not contain `M48B_*_v1`.
   - `game/content/asset_ledger/regen_manifest.ron:7–104`

### MINOR

6. **No pre-commit hook installed.** Spec promises a pre-commit gate.
   Repo ships zero hook scaffolding (`.git/hooks/` is bare; no
   `.pre-commit-config.yaml`; no Husky / Lefthook / etc.). This is a
   developer-UX gate; the nightly CI gate (#3) is the safety net.

7. **No release-CI `cf-mod ledger regenerate --all` step.** Spec asks
   release CI to validate full reproducibility from clean checkout. Not
   wired into `release.yml`.

8. **`observe.assets.ledger_summary` has no per-pipeline-tier
   projection.** Spec says "per-category counts + per-pipeline-tier
   counts." The current shape has `by_tier` (ProductionTier) and
   `by_category` but no `by_pipeline` (the per-pipeline-id breakdown
   that would let cfctl filter "all M9A_svg_v1 entries by status").
   The summary already lists missing/drifted/failed/stale `AssetId`s, so
   callers can pivot in user-space; ranking this MINOR since the data
   is reachable from `list` filtered by `--pipeline`.

## Recommended Fixes

Concrete, minimal edits:

1. **Fix the audit-script strict flag (BLOCKER #1).** Replace `--strict`
   with `--strict-status` in `game/scripts/ledger_audit.sh:38` and the
   `--json` branch above it. Alternatively (less surgical): in
   `cf-mod/src/main.rs:384`, change
   ```rust
   let strict = *strict_status;
   ```
   to read the global as well:
   ```rust
   let strict = *strict_status || cli.strict;
   ```
   Requires passing `cli.strict` into `run_ledger` (currently it takes
   only `json_output`).

2. **Wire `asset_ref` cross-check into `cf-headless replay` (MAJOR
   #2).** Add `cf-asset-ledger = { path = "../cf-asset-ledger" }` to
   `cf-headless/Cargo.toml`. In the replay loop (after parsing
   `events.jsonl`), for every event whose `asset_ref.is_some()`, open the
   canonical ledger (same candidate-path discovery used by
   `default_observe_assets_ledger_summary`), look up the `AssetId`,
   assert `RegenStatus == Fresh`, and emit a structured failure
   (similar to `determinism.first_divergence`) when the entry is
   `Missing` / `Drifted` / `Stale` / `Failed`. Add a flag
   `--no-verify-asset-refs` to opt out (matches the existing
   `--no-verify-checksums` precedent at line 41).

3. **Add a nightly CI job (MAJOR #3).** Append to
   `.github/workflows/ci.yml` a `schedule:` trigger (`cron: '0 7 * * *'`)
   on `main` and a job `ledger_audit:` that runs
   `bash game/scripts/ledger_audit.sh` (after the strict-flag fix from
   #1) and uploads the `--json` report as an artifact.

4. **Document `cf-mod package` as deferred OR ship a shim (MAJOR #4).**
   Either:
   - Add to `specs/done/M4A.md` § "Out of scope" a bullet stating "Mod
     package builder shipping at M5/M8; `cf-mod ledger add` is the
     interim contract — pipelines must call it explicitly."
   - OR add a `Cmd::Package { pkg_dir }` subcommand to
     `cf-mod/src/main.rs` that walks `<pkg_dir>/assets/`, computes
     canonical names, and calls `cmd_add` per asset with
     `category = Mod_Custom` + `package_source = mod_id`.

5. **Add M48B to `regen_manifest.ron` (MAJOR #5).** Append a new entry:
   ```ron
   (
       pipeline_id: "M48B_marketing_v1",
       owner_milestone: "M48B",
       regen_command: "cf-tools-marketing --asset-id $ASSET_ID --seed $SEED --out $OUTPUT_PATH",
       model_version: "marketing-v1",
       deterministic: true,
       freeze_path_suffix: ".frozen",
       notes: "Marketing assets (key art, store badges); freeze-then-store.",
   ),
   ```
   to `game/content/asset_ledger/regen_manifest.ron`.

6. **(MINOR #6) Install a sample pre-commit hook.** Ship
   `game/scripts/install-hooks.sh` that copies
   `game/scripts/hooks/pre-commit` into `.git/hooks/`; the hook runs
   `cf-mod ledger verify --strict-status --all` (scoped to changed
   ledger entries via `git diff --name-only HEAD`).

7. **(MINOR #7) Add a release-CI re-bake step.** In
   `.github/workflows/release.yml`, add a job that on tag-push checks
   out fresh, deletes `game/content/assets/`, runs
   `cargo run --release -p cf-mod -- ledger regenerate --all`, and
   confirms exit 0 + clean `git diff` against
   `content/asset_ledger/ledger.jsonl`.

8. **(MINOR #8) Add `by_pipeline` to summary projection.** Extend
   `cf_asset_ledger::LedgerSummary` with a `by_pipeline:
   BTreeMap<String, u64>` field; update `summary_to_observe_json` so the
   `observe.assets.ledger_summary` response carries it. Additive change
   — schema_version remains 1.
