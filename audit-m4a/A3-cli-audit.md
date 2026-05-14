# A3 — CLI Surface Audit

**Auditor:** Worker subagent  
**Audit date:** 2026-05-13  
**Scope:** Every CLI command + flag listed in `specs/done/M4A.md` "CLI surface" section + Gherkin scenarios "Per-category + per-tier filtering" and "Audit reports missing + drifted + failed", validated against:
- `game/crates/cf-mod/src/main.rs` (clap `LedgerAction` enum + `run_ledger` dispatch)
- `game/crates/cf-asset-ledger/src/cli.rs` (`cmd_*` + `render_*` functions)
- `game/crates/cf-mod/tests/ledger_cli_integration.rs` (end-to-end binary tests)
- `game/crates/cfctl/src/main.rs` (`LedgerSummary` subcommand)
- `game/crates/cf-control/src/server.rs` (`observe.assets.ledger_summary` JSON-RPC dispatch)

Live tests were executed against `/Users/erol/projects/corefall/game/target/debug/cf-mod` in a `/tmp/m4a-cli-audit/` sandbox.

---

## Executive summary

8 of 9 spec commands work end-to-end. The single MAJOR drift is a verify exit-code bug:

> **`cf-mod ledger verify --strict` (spec syntax) does NOT exit non-zero on drift.** The dispatcher in `run_ledger` ignores the global `--strict` clap flag and only honors `--strict-status`, despite an inline comment claiming the global is wired through.

Beyond that:
- **MINOR:** Spec flag name `--palette-ref` is implemented as `--palette` (no alias). Pipelines copying the spec verbatim will fail.
- **MINOR:** `cf-mod ledger summary` reports `regen_status` recorded at add-time only; it never inspects disk, so the Gherkin "summary lists missing/drifted ids" only works if an upstream `verify` (or `mark_dependents_stale`) has already rewritten statuses. Both tests acknowledge this in a comment.
- **MINOR:** `cf-mod ledger compact --keep-latest` defaults to `true` and has no negate form, so the flag is effectively a no-op toggle.
- **MINOR:** `summary` adds two extra lines ("Live entries", "Superseded") and uses canonical tier strings (`Tier0_Placeholder`, `Tier2_Audio_Production`) where the spec example uses shortened forms (`Tier0`, `Tier2_Audio`). Both parse, but the prose-example mismatch is worth noting.

cfctl `ledger-summary` and `observe.assets.ledger_summary` JSON-RPC: shape conforms to spec.

---

## Per-Command Verdict

### Command 1: `cf-mod ledger add ...`

**Spec syntax:**
```
cf-mod ledger add \
    --category WeaponSprite \
    --kind weapon-side \
    --canonical-name "iron_rifle_m1_side_v1" \
    --tier Tier1_SVG \
    --pipeline M9A_svg_v1 \
    --prompt "industrial rifle, ..." \
    --seed 1234 \
    --output-path content/assets/placeholders/weapons/iron_rifle_m1_side.svg
```
Plus optional: `--negative-prompt, --palette-ref, --style-lora, --upstream, --package-source, --license, --generated-by-human, --human-edit-notes, --regen-command`.

**Rust enum variant:** `LedgerAction::Add` (cf-mod/src/main.rs:60-105).  
**Library impl:** `cmd_add` (cf-asset-ledger/src/cli.rs:79-160).

**Required flags (all spec-required are wired):** `--category`, `--kind`, `--canonical-name`, `--tier`, `--pipeline`, `--prompt`, `--seed`, `--output-path`.

**Optional flags wired:** `--negative-prompt`, `--style-lora`, `--upstream` (repeatable), `--package-source`, `--license`, `--generated-by-human`, `--human-edit-notes`, `--regen-command`.

**Optional flags MISSING / RENAMED:**
- `--palette-ref` (spec) ⇒ implemented as `--palette` (no `--palette-ref` alias). **MINOR drift.** Pipelines copying the spec literal will see clap reject `--palette-ref`.

**Extra (non-spec) flags wired:** `--generator-tool`, `--generator-model`, `--generator-workflow`, `--generator-model-version`, `--freeze` (default `true`), `--ledger-path` (test override). These are pipeline extensions; spec is silent on them.

**Default values:**
- `freeze`: `true` (snapshots `<output_path>.frozen` for byte-identical regen — implementation-side decision, undocumented in spec).
- `generated_by_human`: `false` (correct — spec says default is non-human).
- All `Option<String>` flags default to `None`.

**Output format (stdout, non-JSON mode):**
```
ledger add OK: id=<64-hex> canonical_name=<name> category=<cat> tier=<tier> pipeline=<pipeline>
```

**Exit codes:**
- 0 on success.
- 1 (anyhow propagated) on: unknown category/tier, output file missing, hash failure, ledger write failure.

**Live-test result:**
```
$ cf-mod ledger add --category WeaponSprite --kind weapon-side --canonical-name test --tier Tier1_SVG \
    --pipeline M9A_svg_v1 --prompt p --seed 1 --output-path test.svg --ledger-path ./ledger.jsonl
ledger add OK: id=435ab3873a27ccae2b88bb9f2982870b1cbbb6b426015346c9e2a0c2802edfee canonical_name=test category=WeaponSprite tier=Tier1_SVG pipeline=M9A_svg_v1
EXIT: 0
```

**Verdict:** PARTIAL — one spec flag rename (`--palette-ref` → `--palette`) breaks literal spec command. Everything else PASS.

---

### Command 2: `cf-mod ledger list ...`

**Spec syntax:**
```
cf-mod ledger list --category WeaponSprite --tier Tier1_SVG
cf-mod ledger list --pipeline M32A_comfyui_v1 --status Fresh
```

**Rust enum variant:** `LedgerAction::List` (cf-mod/src/main.rs:107-120).  
**Library impl:** `cmd_list` (cf-asset-ledger/src/cli.rs:182-187).

**Required flags:** none (all optional filters).  
**Optional flags wired:** `--category`, `--tier`, `--pipeline`, `--status`, `--include-superseded` (extension), `--ledger-path` (extension).

**Missing flags:** none.

**Default values:** all `Option`; `--include-superseded` defaults `false` (correct — Gherkin "Per-category + per-tier filtering" implies superseded entries are hidden by default).

**Output format (stdout, non-JSON):**
```
<id>  <category>  <tier>  <canonical_name>  status=<status>
```
One line per match.

**Exit codes:** 0 always (no non-zero path on filter mismatch — empty list is success).

**Live-test result:**
```
$ cf-mod ledger list --category WeaponSprite --tier Tier1_SVG --ledger-path ./ledger.jsonl
435ab3873a27ccae2b88bb9f2982870b1cbbb6b426015346c9e2a0c2802edfee  WeaponSprite  Tier1_SVG  test  status=Fresh
EXIT: 0

$ cf-mod ledger list --pipeline M9A_svg_v1 --status Fresh --ledger-path ./ledger.jsonl
435ab3873a27ccae2b88bb9f2982870b1cbbb6b426015346c9e2a0c2802edfee  WeaponSprite  Tier1_SVG  test  status=Fresh
ad517f8e1a627ef00e1e85975ef1d245ce9192b1b787f58e3b91c47b479300e0  UiIcon  Tier1_SVG  test2  status=Fresh
EXIT: 0
```

**Verdict:** PASS.

---

### Command 3: `cf-mod ledger show <asset_id>`

**Spec syntax:** `cf-mod ledger show <asset_id>` — single positional arg.

**Rust enum variant:** `LedgerAction::Show` (cf-mod/src/main.rs:122-126).  
**Library impl:** `cmd_show` + `match_id` (cf-asset-ledger/src/cli.rs:189-214).

**Required args:** `<id>` (string positional). Matches: full 64-hex AssetId, OR any prefix of the hex, OR canonical_name (extension beyond spec — useful, no harm).

**Optional flags wired:** `--ledger-path`.

**Output format (stdout):** pretty-printed JSON of the entry (the inline `--json` global is documented as a no-op since "JSON is the only viable readable representation of an AssetEntry").

**Exit codes:**
- 0 on found.
- 1 on: not found (`no entry matches id-or-prefix <x>`), or ambiguous prefix.

**Live-test result:**
```
$ cf-mod ledger show 435a --ledger-path ./ledger.jsonl
{ "id": "435ab...", ... }   # pretty JSON, exit 0

$ cf-mod ledger show test --ledger-path ./ledger.jsonl
{ "id": "435ab...", "canonical_name": "test", ... }   # canonical_name match, exit 0

$ cf-mod ledger show nonexistent --ledger-path ./ledger.jsonl
Error: ledger show
Caused by: no entry matches id-or-prefix nonexistent
EXIT: 1
```

**Verdict:** PASS.

---

### Command 4: `cf-mod ledger diff --all` / `cf-mod ledger diff <asset_id>`

**Spec syntax:**
```
cf-mod ledger diff --all
cf-mod ledger diff <asset_id>
```

**Rust enum variant:** `LedgerAction::Diff` (cf-mod/src/main.rs:128-135).  
**Library impl:** `cmd_diff` (cf-asset-ledger/src/cli.rs:216-228).

**Required args/flags:** none (either id or `--all`; if both omitted, behaves like `--all` — every live entry).

**Optional flags wired:** `--all`, `--ledger-path`.

**Default values:** `--all` defaults `false` but if no id is given, target is `None` which exercises the all-entries branch anyway.

**Output format (stdout, non-JSON):**
```
<id>  status=<status>  observed_blake3=<hash>  observed_size=<bytes>
```
One line per entry; format not in spec but reasonable.

**Exit codes:**
- 0 if every live entry is Fresh.
- **1 if any entry's status is Drifted or Missing** (`std::process::exit(1)` at cli.rs:228 → main.rs:264-272). Stale/Failed do NOT cause non-zero exit on diff (only verify does).

**Live-test result:**
```
$ cf-mod ledger diff --all --ledger-path ./ledger.jsonl
435ab... status=Fresh observed_blake3=ac5a... observed_size=7
ad517... status=Fresh observed_blake3=83a7... observed_size=12
EXIT: 0

$ echo 'drift!' > test2.svg && cf-mod ledger diff --all
... status=Drifted ...
EXIT: 1
```

**Verdict:** PASS.

---

### Command 5: `cf-mod ledger regenerate ...`

**Spec syntax:**
```
cf-mod ledger regenerate <asset_id>
cf-mod ledger regenerate --category WeaponSprite --tier Tier1_SVG
cf-mod ledger regenerate --all
cf-mod ledger regenerate --cascade <tier1_id>     # Gherkin "Upstream asset dependency graph"
```

**Rust enum variant:** `LedgerAction::Regenerate` (cf-mod/src/main.rs:139-155).  
**Library impl:** `cmd_regenerate` (cf-asset-ledger/src/cli.rs:295-365).

**Required args/flags:** at least ONE of: positional `<id>`, `--cascade <id>`, `--category` (with or without `--tier`), `--tier`, `--all`. If none provided, errors with "requires an id, --cascade <id>, --category/--tier filters, or --all" + exit 1.

**Optional flags wired:** `--cascade` (bool, requires positional id), `--category`, `--tier`, `--all`, `--continue-on-error` (extension), `--ledger-path` (extension).

**Missing flags:** none.

**Default values:** all bool flags default `false`; `--continue-on-error` defaults `false` (so first failure aborts unless flag set).

**Output format (stdout, non-JSON):**
```
regenerate total=<N> ok=<X> fail=<Y>
  OK   <id>
  FAIL <id>: <error>
```

**Exit codes:**
- 0 if every attempted regen succeeded.
- 1 if any attempt failed (`std::process::exit(1)` at main.rs:339-341).
- 1 if no id/filter/--all/--cascade supplied.

**Live-test results:**
```
$ cf-mod ledger regenerate --all                        # exits 0, all OK
$ cf-mod ledger regenerate 435ab...                     # exits 0, single OK
$ cf-mod ledger regenerate --category WeaponSprite --tier Tier1_SVG   # exits 0, single match
$ cf-mod ledger regenerate --cascade 435ab...           # exits 0, descendants (just root in scratch)
$ cf-mod ledger regenerate                              # exits 1, "requires an id, ..."
```

The cascade order is dependency-first via `topological_descendant_order` (regenerator.rs:318); the root IS included in the walk (Gherkin scenario "regenerates the entire downstream graph" is satisfied at unit-test level — `regenerate_with_cascade` unit test ships in regenerator.rs:493+).

**Verdict:** PASS.

---

### Command 6: `cf-mod ledger verify --all` / `cf-mod ledger verify --strict`

**Spec syntax:**
```
cf-mod ledger verify --all
cf-mod ledger verify --strict                                 # CI mode: exit non-zero on any drift
```

**Rust enum variant:** `LedgerAction::Verify` (cf-mod/src/main.rs:159-163).  
**Library impl:** `cmd_verify` (cf-asset-ledger/src/cli.rs:230-258).

**Required args/flags:** none (id positional optional; `--all` optional; defaults to "verify every live entry").

**Optional flags wired:** `--all`, `--strict-status`, `--ledger-path`.

**Missing flag (BLOCKER for spec literalism):**
- The spec calls the strict toggle `--strict`. The clap definition uses `--strict-status` (cf-mod/src/main.rs:166-169) and `cli.strict` is a global flag.
- The inline comment at main.rs:286-289 claims both `--strict-status` AND top-level `--strict` are honored:
  ```rust
  // Strict mode is forced on when:
  //   * `--strict-status` is explicitly set
  //   * top-level `--strict` is set
  let strict = *strict_status;
  ```
  …but the next line ONLY reads `*strict_status`. **The global `--strict` flag is silently ignored**, so `cf-mod ledger verify --strict` (spec syntax) does not engage strict mode.

**Default values:** all bools default `false`.

**Output format (stdout, non-JSON):**
```
verify total=<N> fresh=<F> stale=<S> drifted=<D> missing=<M> failed=<X>
  <id> status=<S> <note>
```
Only non-Fresh entries appear in the bulleted list. Output is informational only — exit code carries the verdict.

**Exit codes:**
- 0 normally.
- 1 **only when `--strict-status` is set AND (drifted + missing + failed) > 0** (cf-mod/src/main.rs:307-309 + cli.rs:255).
- `--strict` (global) is parsed by clap but **never read** by the verify dispatch path. **BUG.**

**Live-test results:**
```
# Drift an asset, then:
$ cf-mod ledger verify --all --strict-status --ledger-path ./ledger.jsonl
verify total=2 fresh=1 stale=0 drifted=1 missing=0 failed=0
  ad51... status=Drifted blake3 drift: expected ... (12B), observed 8B
EXIT: 1                  # CORRECT

$ cf-mod ledger verify --all --strict --ledger-path ./ledger.jsonl
verify total=2 fresh=1 stale=0 drifted=1 missing=0 failed=0
  ad51... status=Drifted ...
EXIT: 0                  # WRONG — spec command exits 0 on drift

$ cf-mod --strict ledger verify --all --ledger-path ./ledger.jsonl
EXIT: 0                  # WRONG — global --strict ignored

$ cf-mod ledger --strict verify --all --ledger-path ./ledger.jsonl
EXIT: 0                  # WRONG — global --strict ignored
```

**Verdict:** FAIL (for the literal spec command `cf-mod ledger verify --strict`). PASS only via the `--strict-status` workaround.

---

### Command 7: `cf-mod ledger summary`

**Spec syntax:** `cf-mod ledger summary`. Spec example output:
```
Total entries: 4827
By category: UiIcon=84, WeaponSprite=210, ActorSprite=176, ...
By tier: Tier0=12, Tier1_SVG=2104, Tier1_LLM_Audio=412, Tier2_ComfyUI=1843, Tier2_Audio=298, Tier3_Polish=158
Status: Fresh=4801, Stale=18, Drifted=4, Missing=3, Failed=1
Missing: [list of asset_ids]
Drifted: [list of asset_ids]
```

**Rust enum variant:** `LedgerAction::Summary` (cf-mod/src/main.rs:184-186).  
**Library impl:** `cmd_summary` (cf-asset-ledger/src/cli.rs:367-371) + `render_summary` (cli.rs:391-419).

**Required flags:** none.  
**Optional flags wired:** `--ledger-path`.

**Output format (stdout, non-JSON, actual):**
```
Total entries: <N>
Live entries:  <L>
Superseded:    <S>
By category: <k>=<v>, ...
By tier:     <k>=<v>, ...
Status:      <k>=<v>, ...
<status>: [<id>, <id>, ...]      # one line per non-Fresh status, only when non-empty
```

**Differences from spec example:**
1. Adds `Live entries:` and `Superseded:` lines (additive, not breaking).
2. Tier strings use canonical `Tier0_Placeholder` / `Tier2_Audio_Production` rather than spec example's `Tier0` / `Tier2_Audio` shorthand (parse() accepts both; output uses long forms).
3. `Missing:` / `Drifted:` lines only emitted when the corresponding bucket has entries; spec example implies they should always appear (even if `[]`).
4. Order of non-Fresh lines is BTreeMap alphabetical (`Drifted`, `Failed`, `Missing`, `Stale`) not spec example's `Missing`, `Drifted`.

**Important behavioral note (MINOR):** `summary` aggregates the `regen_status` field **recorded at add-time** (always `Fresh`) — it never re-hashes disk. Consequently the Gherkin scenario "Audit reports missing + drifted + failed" only works if some upstream step has rewritten statuses (e.g., `mark_dependents_stale` flips dependents to `Stale`). The integration-test acknowledges this explicitly:
```rust
// by_status only reflects entry.regen_status (which is Fresh at write-time);
// the drift bucket is populated by `verify` not `summary`.
```
This is reasonable given the spec carve-up (summary == aggregation, verify == on-disk check), but it weakens the "Audit reports" Gherkin: a user who hand-edits an asset and runs `summary` will see `Fresh=N`, not `Drifted=1`. **MINOR drift between spec intent and implementation.**

**Exit codes:** 0 always.

**Live-test result:**
```
$ cf-mod ledger summary --ledger-path ./ledger.jsonl
Total entries: 2
Live entries:  2
Superseded:    0
By category: UiIcon=1, WeaponSprite=1
By tier:     Tier1_SVG=2
Status:      Fresh=2
EXIT: 0

$ cf-mod --json ledger summary --ledger-path ./ledger.jsonl
{
  "by_category": {"UiIcon": 1, "WeaponSprite": 1},
  "by_status": {"Fresh": 2},
  "by_tier": {"Tier1_SVG": 2},
  "drifted": [], "failed": [], "missing": [], "stale": [],
  "live_entries": 2, "superseded_entries": 0, "total_entries": 2,
  "schema_version": 1
}
EXIT: 0
```

**Verdict:** PARTIAL. Stdout deviates from the spec example layout (2 extra lines, missing/drifted lists conditional, alphabetical ordering). JSON shape (which cfctl reads) is clean and matches the `observe.assets.ledger_summary` spec table.

---

### Command 8: `cf-mod ledger compact --keep-latest --before <date>`

**Spec syntax:** `cf-mod ledger compact --keep-latest --before <date>` (Gherkin "Ledger size bounded under regen churn").

**Rust enum variant:** `LedgerAction::Compact` (cf-mod/src/main.rs:189-198).  
**Library impl:** `cmd_compact` (cf-asset-ledger/src/cli.rs:373-381) → `LedgerHandle::compact` (storage.rs:218-254).

**Required flags:** none.  
**Optional flags wired:** `--keep-latest` (bool, default `true`), `--before <DATE>` (string), `--ledger-path`.

**Default values:**
- `--keep-latest = true` (per clap `default_value_t = true`).
- `--before = None`.

**Defect (MINOR):** `--keep-latest` defaults to `true` and has no `--no-keep-latest` negate form via clap. Furthermore, the `else` branch of `keep_latest_only` in storage.rs:221-225 also filters `superseded_by.is_none()`, so it's functionally equivalent. The flag is a UI-only toggle with no observable behavior difference between true/false — **MINOR: spec lists `--keep-latest` as a meaningful filter but in practice it can never be exercised differently.**

**Semantics of `--before <date>`:** The storage code does `retained.retain(|e| e.generated_at_iso.as_str() >= keep_after)`. So `--before 2030-01-01T00:00:00Z` drops every entry whose `generated_at_iso` < `2030-01-01...`. Spec text says "reduces it to current-state-only" without specifying date semantics; the lexical RFC-3339 string compare works correctly for proper ISO timestamps but is **NOT robust** against malformed dates (no validation; `--before "not-a-date"` would silently drop nothing because no entry's ISO timestamp is `>= "not-a-date"` lexically… actually it would drop everything because `"2026..." < "not-a-date"`). **MINOR: no validation of the date string.**

**Output format (stdout, non-JSON):**
```
compact: before=<N> after=<M> backup=<path>
```
Where `backup` is `<ledger_path>.bak` written before truncation.

**Exit codes:** 0 normally; 1 on I/O failure.

**Live-test result:**
```
$ cf-mod ledger compact --ledger-path ./ledger.jsonl
compact: before=2 after=2 backup=./ledger.jsonl.bak
EXIT: 0

$ cf-mod ledger compact --keep-latest --before 2030-01-01T00:00:00Z --ledger-path ./ledger.jsonl
compact: before=2 after=0 backup=./ledger.jsonl.bak
EXIT: 0
```

**Verdict:** PASS (functionally) / PARTIAL (semantic — `--keep-latest` is a no-op toggle, `--before` accepts unvalidated strings).

---

### Command 9 (Gherkin coverage): `cf-mod ledger regenerate --cascade <tier1_id>`

Already covered under Command 5 — the `--cascade` flag is wired, accepts a positional id, and walks descendants via `topological_descendant_order`. Per unit tests in `regenerator.rs::tests::regenerate_with_cascade_includes_root_and_dependents`, the root + all transitive dependents are regenerated.

**Verdict:** PASS.

---

## Flag-by-Flag Compatibility Matrix

### `cf-mod ledger add`

| Spec Flag | Rust Flag | Type | Default | Notes |
|---|---|---|---|---|
| `--category` | `--category <CATEGORY>` | `String` (parsed → AssetCategory) | required | PASS |
| `--kind` | `--kind <KIND>` | `String` | required | PASS |
| `--canonical-name` | `--canonical-name <CANONICAL_NAME>` | `String` | required | PASS |
| `--tier` | `--tier <TIER>` | `String` (parsed → ProductionTier) | required | PASS |
| `--pipeline` | `--pipeline <PIPELINE>` | `String` | required | PASS |
| `--prompt` | `--prompt <PROMPT>` | `String` | required | PASS |
| `--seed` | `--seed <SEED>` | `u64` | required | PASS |
| `--output-path` | `--output-path <OUTPUT_PATH>` | `PathBuf` | required | PASS |
| `--negative-prompt` | `--negative-prompt <NEGATIVE_PROMPT>` | `Option<String>` | None | PASS |
| **`--palette-ref`** | **`--palette <PALETTE>`** | `Option<String>` | None | **MINOR drift** — flag renamed; no spec alias |
| `--style-lora` | `--style-lora <STYLE_LORA>` | `Option<String>` | None | PASS |
| `--upstream` | `--upstream <UPSTREAM>` | `Vec<String>` (repeatable) | empty | PASS |
| `--package-source` | `--package-source <PACKAGE_SOURCE>` | `Option<String>` | None | PASS — accepts `vanilla`, `mod:<id>`, `faction-pack:<id>` |
| `--license` | `--license <LICENSE>` | `Option<String>` | None | PASS — accepts `cc0`, `cc-by`, `cc-by-sa`, `proprietary`, `mod:<id>`, fallback `Custom(s)` |
| `--generated-by-human` | `--generated-by-human` | `bool` flag | false | PASS |
| `--human-edit-notes` | `--human-edit-notes <HUMAN_EDIT_NOTES>` | `Option<String>` | None | PASS |
| `--regen-command` | `--regen-command <REGEN_COMMAND>` | `Option<String>` | None | PASS |
| _(not in spec)_ | `--generator-tool <GENERATOR_TOOL>` | `Option<String>` | None | Extension |
| _(not in spec)_ | `--generator-model <GENERATOR_MODEL>` | `Option<String>` | None | Extension |
| _(not in spec)_ | `--generator-workflow <GENERATOR_WORKFLOW>` | `Option<String>` | None | Extension |
| _(not in spec)_ | `--generator-model-version <GENERATOR_MODEL_VERSION>` | `Option<String>` | None | Extension |
| _(not in spec)_ | `--freeze` | `bool` flag | **true** | Extension; freezes output as `<path>.frozen` |
| _(not in spec)_ | `--ledger-path <LEDGER_PATH>` | `Option<PathBuf>` | None | Test/extension override; falls back to `content/asset_ledger/ledger.jsonl` |

### `cf-mod ledger list`

| Spec Flag | Rust Flag | Type | Default | Notes |
|---|---|---|---|---|
| `--category` | `--category <CATEGORY>` | `Option<String>` | None | PASS |
| `--tier` | `--tier <TIER>` | `Option<String>` | None | PASS |
| `--pipeline` | `--pipeline <PIPELINE>` | `Option<String>` | None | PASS |
| `--status` | `--status <STATUS>` | `Option<String>` | None | PASS — accepts `Fresh`/`Stale`/`Drifted`/`Missing`/`Failed` |
| _(not in spec)_ | `--include-superseded` | `bool` flag | false | Extension; default hides superseded |
| _(not in spec)_ | `--ledger-path <LEDGER_PATH>` | `Option<PathBuf>` | None | Extension |

### `cf-mod ledger show`

| Spec | Rust | Type | Notes |
|---|---|---|---|
| `<asset_id>` | `<ID>` positional | `String` | PASS — also accepts prefix + canonical_name |
| _(not in spec)_ | `--ledger-path` | `Option<PathBuf>` | Extension |

### `cf-mod ledger diff`

| Spec Flag | Rust Flag | Type | Default | Notes |
|---|---|---|---|---|
| `<asset_id>` | `[ID]` positional | `Option<String>` | None | PASS |
| `--all` | `--all` | `bool` flag | false | PASS — when no id and no --all, target = None which is same as --all |
| _(not in spec)_ | `--ledger-path` | `Option<PathBuf>` | None | Extension |

### `cf-mod ledger verify`

| Spec Flag | Rust Flag | Type | Default | Notes |
|---|---|---|---|---|
| `<asset_id>` | `[ID]` positional | `Option<String>` | None | PASS |
| `--all` | `--all` | `bool` flag | false | PASS |
| **`--strict`** | **none — only `--strict-status`** | — | — | **MAJOR drift** — spec says `--strict`; binary parses global `--strict` but the verify dispatch IGNORES it and only honors `--strict-status` |
| _(global `--strict`)_ | `--strict` (global) | `bool` flag | false | Parsed but unused for verify; comment claims otherwise |
| _(not in spec)_ | `--ledger-path` | `Option<PathBuf>` | None | Extension |

### `cf-mod ledger regenerate`

| Spec | Rust | Type | Default | Notes |
|---|---|---|---|---|
| `<asset_id>` | `[ID]` positional | `Option<String>` | None | PASS |
| `--category` | `--category <CATEGORY>` | `Option<String>` | None | PASS |
| `--tier` | `--tier <TIER>` | `Option<String>` | None | PASS |
| `--all` | `--all` | `bool` flag | false | PASS |
| `--cascade <id>` | `--cascade` (paired with positional id) | `bool` flag | false | PASS — cascade requires `<id>` positional |
| _(not in spec)_ | `--continue-on-error` | `bool` flag | false | Extension |
| _(not in spec)_ | `--ledger-path` | `Option<PathBuf>` | None | Extension |

### `cf-mod ledger summary`

| Spec Flag | Rust Flag | Type | Default | Notes |
|---|---|---|---|---|
| _(none in spec)_ | _(none)_ | — | — | — |
| _(not in spec)_ | `--ledger-path` | `Option<PathBuf>` | None | Extension |

### `cf-mod ledger compact`

| Spec Flag | Rust Flag | Type | Default | Notes |
|---|---|---|---|---|
| `--keep-latest` | `--keep-latest` | `bool` | **true** | MINOR: default `true`, no negate form via clap; both code branches (`keep_latest_only=true` vs `false`) yield same result, so flag is effectively a no-op |
| `--before <date>` | `--before <BEFORE>` | `Option<String>` | None | MINOR: string compared lexically against `generated_at_iso`; no format validation |
| _(not in spec)_ | `--ledger-path` | `Option<PathBuf>` | None | Extension |

### Global cf-mod flags

| Spec Flag | Rust Flag | Type | Default | Notes |
|---|---|---|---|---|
| _(implied by spec for verify)_ | `--strict` (global) | `bool` flag | false | Parsed; **not threaded to ledger subcommands** |
| _(not in spec)_ | `--json` (global) | `bool` flag | false | Threaded to `run_ledger` second arg; flips every verb to JSON output |

---

## cfctl ledger-summary surface

- **Method:** `observe.assets.ledger_summary` (JSON-RPC over the cf-control endpoint).
- **CLI:** `cfctl ledger-summary [--format json|pretty] [--inline] [--connect ...] [--auto-launch-port ...] [--no-auto-launch]` (cfctl/src/main.rs:151-159).
- **Implementation:**
  - `cmd_ledger_summary` at cfctl/src/main.rs:514-552 — supports `--inline` (read `content/asset_ledger/ledger.jsonl` directly without a server) AND online (`session.send_request("observe.assets.ledger_summary", {})`).
  - Server dispatch at cf-control/src/server.rs:1408-1434.
  - `default_observe_assets_ledger_summary()` at cf-control/src/server.rs:523-549 walks the same three candidate paths cfctl uses (`content/asset_ledger/ledger.jsonl`, `../content/asset_ledger/ledger.jsonl`, `game/content/asset_ledger/ledger.jsonl`).
  - Empty-summary fallback at cf-control/src/server.rs:1414-1432 when no ledger exists.

### JSON shape vs spec (`observe.assets.ledger_summary`)

Spec asks for: "total count + per-category counts + per-pipeline-tier counts + missing-entry warnings".

Actual JSON shape (from `summary_to_observe_json` at cf-asset-ledger/src/cli.rs:434-461):

| Field | Spec ask | Actual key | Type |
|---|---|---|---|
| Schema version | — | `schema_version` | `1` |
| Total entries | "total count" | `total_entries` | u64 |
| Live entries | — | `live_entries` | u64 |
| Superseded entries | — | `superseded_entries` | u64 |
| Per-category counts | required | `by_category` | `{name: u64}` |
| Per-tier counts | required | `by_tier` | `{tier: u64}` |
| Per-status counts | — (in CLI but inferred for JSON) | `by_status` | `{status: u64}` |
| Missing ids | "missing-entry warnings" | `missing` | `[id]` |
| Drifted ids | — | `drifted` | `[id]` |
| Failed ids | — | `failed` | `[id]` |
| Stale ids | — | `stale` | `[id]` |

**Verdict:** PASS. Shape matches and exceeds the spec ask; all bucket arrays are always present (empty-array on no data) so consumers don't need to special-case missing keys.

**Live-test:**
```
$ cfctl ledger-summary --inline
{"by_category":{"UiIcon":1,"WeaponSprite":1},"by_status":{"Fresh":2},
 "by_tier":{"Tier1_SVG":2},"drifted":[],"failed":[],"live_entries":2,
 "missing":[],"schema_version":1,"stale":[],"superseded_entries":0,
 "total_entries":2}
```

The empty-ledger fallback (server-side, when no ledger.jsonl exists) is unit-tested at cf-control/src/server.rs:2147-2179 (`observe_assets_ledger_summary_falls_back_to_empty`).

---

## Gaps

### BLOCKER

_(none — all 9 spec commands exist with at least a working invocation)_

### MAJOR

1. **`cf-mod ledger verify --strict` does not exit non-zero on drift.** Spec explicitly calls this the CI gate ("`cf-mod ledger verify --strict` # CI mode: exit non-zero on any drift"). The implementation parses a global `--strict` clap flag but the verify dispatch only honors a separate `--strict-status` subcommand flag. The inline comment at cf-mod/src/main.rs:286-289 falsely claims both work. CI scripts copying the spec command verbatim will silently pass on drifted ledgers.

   - **Reproduction:**
     ```
     # Drift an asset, then:
     $ cf-mod ledger verify --all --strict
     verify total=N fresh=N-1 stale=0 drifted=1 missing=0 failed=0
     EXIT: 0      # WRONG — must be non-zero per spec
     ```
   - **Fix:** in the `LedgerAction::Verify` arm at main.rs:280-310, change:
     ```rust
     let strict = *strict_status;
     ```
     to:
     ```rust
     let strict = *strict_status || cli.strict;
     ```
     This requires threading `cli.strict` from `main()` through `run_ledger` (currently only `cli.json` is passed). Or, simpler: define `--strict` as a `verify`-local flag (alias of `--strict-status`) and drop the global-flag dead code.

### MINOR

2. **Spec flag `--palette-ref` is renamed to `--palette`.** Spec text (CLI surface section, optional flags list) writes `--palette-ref`. The Rust enum uses `#[arg(long)] palette` which exposes only `--palette`. Pipelines that copy the spec verbatim will get `error: unexpected argument '--palette-ref'`. Add an alias via `#[arg(long, alias = "palette-ref")]`.

3. **`cf-mod ledger summary` reports `regen_status` as recorded at add-time only.** The Gherkin "Audit reports missing + drifted + failed" implies summary detects on-disk drift, but the implementation aggregates only the entry-field status (always `Fresh` at add-time). To populate the `Missing`/`Drifted` buckets in summary output, an upstream step (verify, mark_dependents_stale) must first rewrite the entry's `regen_status`. The implementation test (`summary_groups_status`) acknowledges this: "by_status only reflects entry.regen_status … the drift bucket is populated by `verify` not `summary`". Either:
   - Document this carve-up explicitly in the spec ("`summary` reflects entry status; run `verify --all --strict` for on-disk truth"), OR
   - Make `summary` optionally re-hash via a `--verify` flag.

4. **`cf-mod ledger compact --keep-latest` is a no-op toggle.** Both branches of `LedgerHandle::compact` (`keep_latest_only=true` and `=false`) filter `superseded_by.is_none()`, yielding identical retained sets. The flag defaults to `true` and has no negate form via clap. Either delete the flag (it's always-on behavior) or make the `false` branch actually keep superseded history.

5. **`cf-mod ledger compact --before <date>` does not validate the date string.** `LedgerHandle::compact` does a lexical string compare against `generated_at_iso`. Malformed `--before` values silently skew the retention set without erroring. Add a `chrono::DateTime::parse_from_rfc3339` (or equivalent) guard at parse-time.

6. **`cf-mod ledger summary` stdout layout differs from spec example.**
   - Extra `Live entries:` and `Superseded:` lines (additive — not a regression).
   - Tier strings use canonical long forms (`Tier0_Placeholder`, `Tier2_Audio_Production`) where the spec example uses short forms (`Tier0`, `Tier2_Audio`). Parse accepts both; output renders the long form.
   - `Missing:` / `Drifted:` ID lists are only emitted for buckets that have entries; spec example shows both buckets always present. Recommend always emitting both lines (empty `[]` if no entries) so log-scrapers don't need a "present-or-absent" branch.
   - Non-Fresh bucket lines are emitted in BTreeMap-alphabetical order (`Drifted`, `Failed`, `Missing`, `Stale`) rather than spec example's `Missing`, `Drifted` order.

7. **Doc-comment drift in main.rs:286-289** falsely claims `cf-mod --strict ledger verify` works. Remove the line "`* top-level --strict is set`" until the bug is fixed.

---

## Recommended Fixes

Priority-ordered. Each is a small, self-contained patch.

1. **Fix `cf-mod ledger verify --strict` (MAJOR).** Thread the global `cli.strict` into the verify dispatch:
   ```rust
   // main.rs
   Cmd::Ledger { action } => run_ledger(action.as_ref(), cli.strict, cli.json),
   ```
   ```rust
   fn run_ledger(action: &LedgerAction, global_strict: bool, json_output: bool) -> Result<()> { ... }
   // ...
   LedgerAction::Verify { id, all, strict_status, ledger_path } => {
       let strict = *strict_status || global_strict;
       // ...
   }
   ```
   Add an integration test (`verify_global_strict_flag_exits_nonzero_on_drift`) to lock the contract.

2. **Add spec-name alias for `--palette-ref` (MINOR).** In `LedgerAction::Add`:
   ```rust
   #[arg(long, alias = "palette-ref")]
   palette: Option<String>,
   ```

3. **Always emit Missing/Drifted/Failed lines in summary stdout (MINOR).** In `render_summary` (cli.rs:391-419), iterate the canonical bucket list (`Missing`, `Drifted`, `Failed`, `Stale`) instead of `summary.non_fresh.iter()`, and emit `<bucket>: []` for empty buckets. Order them per the spec example: `Missing`, `Drifted`, then `Stale`, `Failed`.

4. **Validate `--before <date>` (MINOR).** In `cmd_compact` (cli.rs:373-381), parse the input string via `chrono::DateTime::parse_from_rfc3339` and bail with a clear error if malformed. Bonus: log the resolved cutoff for visibility.

5. **Remove or rebuild the `--keep-latest` toggle (MINOR).** Either:
   - Drop the flag entirely from `LedgerAction::Compact` (default-true behavior is the only useful one), OR
   - Implement the `keep_latest_only=false` branch to retain superseded history when `--no-keep-latest` is set (Note: clap requires explicit `ArgAction::SetFalse` / a dedicated `--keep-superseded` flag to expose this).

6. **Fix the misleading comment** at main.rs:286-289. Either remove the "top-level --strict is set" bullet (until fix #1 lands) or update the code to match the comment.

7. **Document the summary/verify carve-up in the spec** (MINOR, doc-only). Add a one-liner: "`summary` reflects the `regen_status` field recorded at add-time; for on-disk drift detection, use `verify --all --strict`."

---

## Appendix: Live-test sandbox transcript

All tests executed under `/tmp/m4a-cli-audit/` against `/Users/erol/projects/corefall/game/target/debug/cf-mod` (binary mtime 2026-05-13 20:05).

Cleanup happens at the end of this audit; no stray scratch files remain.
