# A2 — Gherkin Scenarios Audit

Auditor: A2 (read-only audit subagent)
Date: 2026-05-13
Scope: every Gherkin scenario in `specs/done/M4A.md` § Acceptance criteria.
Source-of-truth crate: `game/crates/cf-asset-ledger/`.
Test-run evidence: `cargo test -p cf-asset-ledger` → 46 PASS / 0 FAIL.
                   `cargo test -p cf-mod`           → 18 PASS / 0 FAIL.
                   `cargo test -p cf-control --lib` → 88 PASS / 0 FAIL.
                   `cargo test -p cf-replay --lib`  → 31 PASS / 0 FAIL.

The 13 Gherkin scenarios are enumerated in spec order. Each contains:
- The verbatim Gherkin clauses.
- The test functions that exercise the scenario (with crate path and brief description).
- A per-clause verdict (PASS / PARTIAL / MISSING).
- Gaps the test surface doesn't verify.
- A final scenario verdict.

---

## Per-Scenario Verdict

### Scenario 1: cf-asset-ledger crate ships

**Gherkin (verbatim):**
```
Given M4A closure
Then `cf-asset-ledger` crate exists in game/crates/
And exports public API: `add_entry`, `list_entries`, `regenerate_entry`, `verify_entry`
And the AssetEntry schema is locked at v1.0.0
```

**Test functions that map:**
- `cf-asset-ledger::tests::public_api_round_trip` — exercises `add_entry` + `list_entries` via the public re-exports in `lib.rs`. (`game/crates/cf-asset-ledger/src/lib.rs:124`).
- `cf-asset-ledger::tests::schema_version_locked_at_v1` — asserts `ASSET_ENTRY_SCHEMA_VERSION == "1.0.0"` (`lib.rs:147`).
- `cf-asset-ledger::entry::tests::schema_version_default_locked_at_v1` — asserts the entry default (`entry.rs:540`).
- `cf-asset-ledger::tests::validate_entry_json_accepts_canonical_entry` — round-trips an entry through `validate_entry_json` (`lib.rs:152`).
- Workspace evidence: `game/Cargo.toml` lists `crates/cf-asset-ledger` as a member (line 24).
- Module evidence: `lib.rs:85` `pub fn add_entry`, `lib.rs:90` `pub fn list_entries`, `regenerator.rs:75` `pub fn regenerate_entry`, `integrity.rs:84` `pub fn verify_entry`. All four are re-exported via `pub use …` near `lib.rs:70-83`.

**Per-clause verdict:**
| Clause | Verdict |
|---|---|
| crate exists in game/crates/ | PASS |
| exports `add_entry` | PASS |
| exports `list_entries` | PASS |
| exports `regenerate_entry` | PASS |
| exports `verify_entry` | PASS |
| schema locked at v1.0.0 | PASS |

**Gaps:** None observed for this scenario.

**Verdict:** **PASS**.

---

### Scenario 2: Append-only JSONL ledger

**Gherkin (verbatim):**
```
Given a fresh ledger
When `cf-mod ledger add` is invoked 100 times
Then ledger.jsonl contains 100 lines (one entry per line)
And no line is ever modified post-write (append-only contract)
When the same asset is re-generated:
  Then a NEW entry is appended (NOT overwrite)
  And the old entry is marked `superseded_by = <new_entry_id>`
And the file can be tailed to see new entries (works with `tail -f`)
```

**Test functions that map:**
- `cf-asset-ledger::storage::tests::append_creates_jsonl_one_entry_per_line` — appends 100 entries and asserts the file has 100 lines (`storage.rs:381-393`).
- `cf-mod::ledger_cli_integration::append_only_one_line_per_entry` — drives the `cf-mod` binary five times and asserts JSON parsability per line (`tests/ledger_cli_integration.rs:75-91`). (5, not 100 — see Gaps.)
- `cf-asset-ledger::storage::tests::supersede_back_fills_old_entry` — confirms re-add of same canonical_name+tier+category produces a new entry with same `id` and supersedes the older line (`storage.rs:417-433`).
- `cf-asset-ledger::storage::tests::supersede_only_back_fills_unsuperseded_entries` — distinct canonical_names with explicit supersede (`storage.rs:436-477`).
- `cf-asset-ledger::cli::tests::cli_add_appends_a_fresh_entry_on_re_add` — drives the in-process CLI `cmd_add` twice and verifies the older entry is back-filled with `superseded_by` (`cli.rs:619-642`).
- `cf-mod::ledger_cli_integration::re_add_appends_new_entry_and_supersedes_old` — end-to-end via spawned `cf-mod` binary (`tests/ledger_cli_integration.rs:93-110`).

**Per-clause verdict:**
| Clause | Verdict |
|---|---|
| invoke `cf-mod ledger add` 100 times | PARTIAL — unit test does 100 via storage handle; CLI integration only does 5 |
| ledger.jsonl contains 100 lines | PASS via storage::tests |
| no line modified post-write | PARTIAL — supersede rewrites the *first* line in place when re-adding; tested but no test asserts that **OTHER** lines remain byte-identical after a supersede pass. The unit test only asserts `live.len() == 1`, not "non-superseded lines are byte-stable" |
| re-add ⇒ new entry appended | PASS |
| old entry marked `superseded_by` | PASS (both unit and integration) |
| `tail -f` can stream new entries | MISSING — no test exercises `tail -f` semantics. The append flow uses `OpenOptions::append` + `flush()` (storage.rs:91-101) so it's correct by construction, but there's no integration test that observes a partial-write or runs against a tail watcher |

**Gaps:**
- CLI integration test only runs 5 add operations rather than 100. The unit test covers 100 but bypasses the spawned binary.
- No "byte-identical other lines after supersede" assertion. The `supersede_entry` code path rewrites the WHOLE file (`storage.rs:171-204`), which is the documented "only sanctioned mutation," but a regression that touched other lines during this rewrite would not be caught by any current test.
- No `tail -f` / partial-write test (e.g., assert that line-N+1 is flushed before subsequent appends).

**Verdict:** **PARTIAL** — core append-only + supersede semantics confirmed but the 100-line spec scale and `tail -f` clauses are unverified.

---

### Scenario 3: Integrity check detects drift

**Gherkin (verbatim):**
```
Given an asset whose output_path content is modified outside the pipeline (e.g. hand-edited)
When `cf-mod ledger verify <asset_id>` runs
Then status = Drifted
And the difference between ledger blake3 and current file blake3 is reported
Exit code is non-zero
```

**Test functions that map:**
- `cf-asset-ledger::integrity::tests::verify_entry_drift` — drift detection at the API level (`integrity.rs:258-279`).
- `cf-asset-ledger::cli::tests::cli_verify_detects_drift` — drives `cmd_verify` in-process and asserts `report.drifted == 1` + `!is_strict_ok()` (`cli.rs:580-595`).
- `cf-mod::ledger_cli_integration::verify_detects_drift_non_zero_exit` — drives the spawned `cf-mod` binary and asserts non-zero exit on `--strict-status` (`tests/ledger_cli_integration.rs:148-161`).

**Per-clause verdict:**
| Clause | Verdict |
|---|---|
| asset modified outside pipeline | PASS — tests overwrite the file with `b"hand-edited"` / `b"corrupted"` |
| status = Drifted | PASS |
| difference reported | PASS — `VerifyResult.note` carries `blake3 drift: expected ... observed ...B` (integrity.rs:135-141; `verify_entry_drift` test asserts the substring "drift" in the note) |
| Exit code non-zero | PASS — integration test asserts `code != 0`, in-process test asserts `!is_strict_ok()` |

**Gaps:** Minor — no test asserts the verify output reports BOTH the original blake3 AND the current file blake3 explicitly (the `note` carries `expected <hash> ... observed <size>B` but elides the observed hash in the human-readable note; the observed hash is on `VerifyResult.observed_blake3` and the spawned CLI's text/JSON output does include it via `render_verify_report` / serialized JSON). No assertion in any test that the OBSERVED blake3 appears in stdout.

**Verdict:** **PASS** — drift detection + non-zero exit fully covered; the "difference reported" clause is satisfied via `VerifyResult.observed_blake3` but a test verifying the rendered stdout contains BOTH hashes would close the loop.

---

### Scenario 4: Regenerate produces byte-identical output

**Gherkin (verbatim):**
```
Given a Tier 1 SVG asset with pinned pipeline + seed + model_version
When `cf-mod ledger regenerate <asset_id>` runs
Then the regenerated output's blake3 matches the original ledger blake3
And the file is byte-identical
(Determinism contract: same prompt + same seed + same model_version = same bytes)
```

**Test functions that map:**
- `cf-asset-ledger::regenerator::tests::freeze_then_store_round_trip` — freeze-then-store path: create, freeze, corrupt, regen, assert blake3 + raw bytes restored (`regenerator.rs:457-479`).
- `cf-asset-ledger::cli::tests::cli_regenerate_byte_identical` — in-process CLI regen of one entry; asserts file content restored (`cli.rs:622-639`).
- `cf-mod::ledger_cli_integration::regenerate_byte_identical` — spawned-binary regen of all entries; asserts file content + verify pass (`tests/ledger_cli_integration.rs:163-180`).

**Per-clause verdict:**
| Clause | Verdict |
|---|---|
| Tier 1 SVG asset | PASS — tests use `ProductionTier::Tier1Svg` |
| pinned pipeline + seed + model_version | PARTIAL — seed pinned; pipeline pinned; `model_version` pinning happens via `GeneratorRef.model_version` field but no test sets / asserts this field is reproduced |
| regen → blake3 matches original | PASS |
| file is byte-identical | PASS (raw bytes compared) |
| Determinism contract clause | PASS by construction — the freeze-then-store path makes regen byte-identical regardless of pipeline determinism |

**Gaps:**
- No test sets `generator.model_version` and asserts post-regen the entry's serialized form still pins that value. The schema supports it (`entry.rs:84`), but its preservation isn't asserted.
- The "pinned pipeline + seed + model_version" clause is verified indirectly via the freeze-then-store contract — the regenerator does NOT actually re-run a pipeline; it copies the frozen canonical bytes. This is documented in `regenerator.rs:21-35` and is the spec-sanctioned approach for non-deterministic pipelines. Acceptable.

**Verdict:** **PASS** — byte-identical regen confirmed end-to-end; model_version pinning assertion is the only minor gap.

---

### Scenario 5: Full re-bake from scratch

**Gherkin (verbatim):**
```
Given a fresh checkout (no content/assets/ directory)
When `cf-mod ledger regenerate --all` runs
Then every entry in ledger.jsonl is regenerated
And every output_path file exists with correct blake3
And exit code is 0 if no failures
And the operation is idempotent (running twice yields no change on second pass)
```

**Test functions that map:**
- `cf-asset-ledger::regenerator::tests::regen_all_walks_every_live_entry` — appends 3 entries with freezes, runs `regenerate_all`, asserts every result `ok==true` (`regenerator.rs:481-507`).
- `cf-mod::ledger_cli_integration::full_re_bake_from_scratch_is_idempotent` — deletes the output file, runs `cf-mod ledger regenerate --all`, asserts file restored + blake3 stable on a second regen pass (`tests/ledger_cli_integration.rs:219-241`).

**Per-clause verdict:**
| Clause | Verdict |
|---|---|
| fresh checkout, no content/assets/ | PARTIAL — test deletes the single output file but does NOT delete the parent directory `content/assets/`. The freeze-then-store path creates parent dirs (regenerator.rs:115-121) so this would work; no test asserts it |
| every entry in ledger regenerated | PASS — unit test asserts 3-of-3 walked |
| every output_path exists with correct blake3 | PASS |
| exit code 0 if no failures | PASS — CLI integration test asserts `code == 0` |
| idempotent on second pass | PASS — `full_re_bake_from_scratch_is_idempotent` compares blake3 before/after second regen |

**Gaps:**
- No test fully simulates "fresh checkout (no content/assets/ directory)" — i.e., delete the entire content/assets directory and re-create from ledger+freezes. The integration test only deletes one file. The freeze copy lives at `<output_path>.frozen` which is adjacent to the output_path, so this would still pass; but the spec literal "no content/assets/ directory" is not asserted.
- No test asserts that `cf-mod ledger regenerate --all` returns exit code 1 (not 0) when there's at least one failure. (`regenerator.rs:178-188` returns the partial results; the CLI dispatcher in `cf-mod/src/main.rs:402-405` does exit with code 1 when any attempt has `ok=false`, but this path is untested.)

**Verdict:** **PASS** — core re-bake + idempotence confirmed; the "fresh checkout literally has no content/assets dir" wording is a MINOR gap.

---

### Scenario 6: Per-category + per-tier filtering

**Gherkin (verbatim):**
```
Given a ledger with 5 categories and 4 tiers
When `cf-mod ledger list --category WeaponSprite --tier Tier2_ComfyUI` runs
Then output is only entries matching both filters
When `--status Drifted` is passed:
  Then output is filtered to drifted entries
```

**Test functions that map:**
- `cf-asset-ledger::storage::tests::list_filter_by_category_and_tier` — appends 3 entries spread across categories/tiers, filters by `(WeaponSprite, Tier1Svg)`, asserts the single match (`storage.rs:395-415`).
- `cf-asset-ledger::cli::tests::cli_list_filter_category_and_tier` — same drill via the in-process CLI (`cli.rs:644-664`).
- `cf-mod::ledger_cli_integration::list_filters_category_tier` — spawned-binary equivalent (`tests/ledger_cli_integration.rs:112-145`).

**Per-clause verdict:**
| Clause | Verdict |
|---|---|
| 5 categories × 4 tiers test set-up | PARTIAL — tests use 2 categories × 2 tiers; the matrix isn't actually 5×4 but the filter logic is generic |
| `--category WeaponSprite --tier Tier2_ComfyUI` matches both | PASS — confirmed via `list_filters_category_tier` (matches `WeaponSprite + Tier1_SVG` actually, but the filter mechanism is identical for any tier choice) |
| `--status Drifted` filters | MISSING — `ListFilter.status` is implemented (storage.rs:281, 287-289), parsed in the CLI (`cf-mod/src/main.rs:343`), but NO test exercises `--status` filtering with a Drifted entry. Coverage is at the `matches()` impl level only |

**Gaps:**
- Critical-but-fixable: there is no test for the `--status` filter end-to-end. The `ListFilter.matches()` function handles `Some(s)` (storage.rs:287-289) but no test populates `filter.status` and asserts drifted-only output.
- The "5 categories and 4 tiers" wording is not literally honored in any test (it's a description of the scenario shape, but no test loops over 5×4 = 20 combinations).

**Verdict:** **PARTIAL** — category+tier filter is well-tested; status filter is unverified.

---

### Scenario 7: Mod pack integration

**Gherkin (verbatim):**
```
Given a mod author's `.cfmod` package
When the mod is packaged via `cf-mod package`:
  Then every asset in the mod is registered as a new ledger entry
  And category = Mod_Custom; package_source = mod_id
  And the mod's manifest references ledger entry ids (NOT raw file paths)
When the mod is installed by another player:
  Then ledger entries are copied to local ledger
  And blake3 integrity verified on install
```

**Test functions that map:**
- **NONE** that exercise the mod-pack integration flow end-to-end.
- Tangential evidence:
  - `cf-asset-ledger::category::tests::parse_categories_case_insensitive` asserts `AssetCategory::parse("Mod_Custom")` resolves to `ModCustom` (`category.rs:353`).
  - `parse_package_source` in `cli.rs:178` parses `"mod:<id>"` to `PackageRef::Mod(id)`.
  - The `AddArgs.package_source` field supports `--package-source mod:my_mod_id` (cli.rs:64-66) and is wired through `cf-mod/src/main.rs:283`.
  - `package_source_label` filter exists (`storage.rs:271`) but no test exercises it.

**Per-clause verdict:**
| Clause | Verdict |
|---|---|
| `cf-mod package` packages a `.cfmod` | MISSING — `cf-mod build` is explicitly stubbed: `"cf-mod build is not implemented in M0; package builder lands at M5/M8"` (`cf-mod/src/main.rs:203`); same for `inspect` (line 209). |
| Auto-register every asset on package | MISSING — there is no auto-registration code path. A mod author must manually call `cf-mod ledger add --package-source mod:<id>` for each asset. |
| category = Mod_Custom | MISSING (no test) — the category enum supports it but no test asserts a package flow sets it |
| package_source = mod_id | MISSING (no test) — parsing works, no test asserts it round-trips |
| manifest references ledger entry ids, not raw paths | MISSING — no mod-manifest writer exists |
| Install copies entries to local ledger | MISSING — no install flow exists |
| blake3 integrity verified on install | MISSING — no install flow exists |

**Gaps (all):** The entire `.cfmod` packaging/install integration is unimplemented. `cf-mod package` (renamed from `build` in the spec) is intentionally stubbed at M0 with a panic message pointing forward to M5/M8.

**Verdict:** **FAIL (BLOCKER)** — the spec's mod-pack integration scenario is not covered by any implementation or test. This is the most significant gap; it falls into the "Out of scope" for M4A only if the implementer intentionally deferred to M5/M8 packaging, but the spec lists this as an Acceptance Criterion of M4A itself.

---

### Scenario 8: Upstream asset dependency graph

**Gherkin (verbatim):**
```
Given a Tier 2 ComfyUI sprite that uses a Tier 1 SVG as ControlNet input
Then the Tier 2 entry's `upstream_assets` field includes the Tier 1 entry's id
When the Tier 1 entry is regenerated:
  Then dependents (Tier 2, Tier 3) are marked Stale
  And `cf-mod ledger regenerate --cascade <tier1_id>` regenerates the entire downstream graph
```

**Test functions that map:**
- `cf-asset-ledger::regenerator::tests::cascade_walks_dependents` — appends a Tier1 SVG + Tier2 ComfyUI w/ `upstream=[tier1_id]`, calls `regenerate_with_cascade(tier1_id)`, asserts the result is `[tier1, tier2]` in topological order (`regenerator.rs:509-558`).
- `cf-asset-ledger::regenerator::tests::mark_dependents_stale_flips_descendants` — appends Tier1 + Tier2, calls `mark_dependents_stale(tier1_id)`, asserts Tier2 is now `Stale` (`regenerator.rs:560-600`).
- `cf-asset-ledger::entry::tests::*` — the `AssetEntryBuilder::with_upstream` API is exercised by the builder tests.

**Per-clause verdict:**
| Clause | Verdict |
|---|---|
| Tier 2 entry has upstream_assets including Tier 1 | PASS |
| Tier 1 regen → dependents marked Stale | PASS via `mark_dependents_stale_flips_descendants` — BUT note: this is a SEPARATE function from `regenerate_entry`. Regenerating a Tier 1 entry directly does NOT auto-mark dependents Stale; the caller must run `mark_dependents_stale` first. No test confirms a "regenerate Tier 1, then verify Tier 2 is automatically Stale" workflow end-to-end |
| `--cascade <id>` regenerates downstream graph | PASS — `cascade_walks_dependents` confirms; CLI wired in `cf-mod/src/main.rs:166-167` and `cli.rs:325-339` |
| Tier 3 mark Stale | PARTIAL — `mark_dependents_stale` uses BFS over reverse-dep map, which handles arbitrary depth; the test only covers Tier1→Tier2, not Tier1→Tier2→Tier3. No test exercises the 3-deep chain |

**Gaps:**
- No end-to-end test of `cf-mod ledger regenerate --cascade <id>` via the spawned binary (the CLI flag is wired but its integration-test surface is empty).
- No test exercises a 3-deep cascade (Tier1 → Tier2 → Tier3).
- The implicit promise "Tier 1 regen automatically marks Tier 2/Tier 3 Stale" is NOT in the implementation — `regenerate_entry` does not call `mark_dependents_stale`. A caller could regen Tier 1 and leave Tier 2 marked Fresh in the ledger. The spec's literal "When the Tier 1 entry is regenerated, then dependents... are marked Stale" implies automatic marking; this is MISSING in code.

**Verdict:** **PARTIAL** — cascade regen and the standalone `mark_dependents_stale` work; the spec's expectation of automatic stale-marking after Tier 1 regen is unimplemented.

---

### Scenario 9: Schema version locked at v1

**Gherkin (verbatim):**
```
Given M4A closes with AssetEntry schema v1.0.0
Then the schema is registered in M39's manifest of locked schemas
Future schema bumps require a migration handler per M39 policy
Additive field extensions (serde-default new fields) do NOT require a bump
```

**Test functions that map:**
- `cf-asset-ledger::tests::schema_version_locked_at_v1` (lib.rs:147-150).
- `cf-asset-ledger::entry::tests::schema_version_default_locked_at_v1` (entry.rs:540-542).
- `cf-mod::ledger_cli_integration::schema_version_locked_v1` (tests/ledger_cli_integration.rs:213-221).
- `cf-asset-ledger::tests::validate_entry_json_rejects_schema_drift` (lib.rs:174-188).
- `cf-asset-ledger::tests::validate_entry_json_accepts_canonical_entry` (lib.rs:152-167).
- `cf-mod::tests::validate_ledger_jsonl_accepts_well_formed` (cf-mod/src/main.rs:835-862).
- `cf-mod::tests::validate_ledger_jsonl_rejects_id_drift` (cf-mod/src/main.rs:864-895).

**Per-clause verdict:**
| Clause | Verdict |
|---|---|
| schema_version is "1.0.0" | PASS |
| schema is registered in M39's manifest | MISSING — `docs/plan/m39/` registration is out of scope per AGENTS.md (no doc-read), and there is no in-repo test asserting registration. The spec § Closure procedure says "Register AssetEntry schema in M39's manifest at M39 close" so this is deferred to M39 — acceptable per the spec text |
| Future schema bumps require migration | PARTIAL — `validate_entry_json` rejects `schema_version != "1.0.0"` ; no test asserts an actual migration handler. Migration shims are deferred to M39 |
| Additive (serde-default) fields don't bump | PASS implicitly — `entry.rs` annotates every optional field with `#[serde(default)]`. Tested implicitly via `entry_roundtrip_through_jsonl` (a default-built entry serializes + deserializes cleanly). No test specifically adds a NEW additive field at runtime to prove it round-trips without a bump |

**Gaps:**
- No test demonstrates additive forward-compat by reading a v1 line with an EXTRA unknown field and confirming it's preserved (or harmlessly dropped). The `extension_fields: BTreeMap<String, Value>` (entry.rs:170-172) is the documented extension surface but its forward-compat round-trip isn't tested.
- M39 manifest registration is a documentation concern, deferred per the spec's Closure procedure.

**Verdict:** **PASS** — schema lock is enforced; additive forward-compat is asserted by construction (serde defaults) but not by an explicit "unknown field is preserved" test.

---

### Scenario 10: Run bundle references ledger entries

**Gherkin (verbatim):**
```
Given a run bundle with capture grid screenshots
Then each screenshot in the bundle has an `asset_ref` field linking to a ledger entry
And `cf-headless replay` validates that referenced ledger entries exist + are Fresh
```

**Test functions that map:**
- `cf-replay::tests::record_with_asset_ref_populates_envelope_field` — populates `Event.asset_ref` via `record_with_asset_ref`, serializes, round-trips, asserts the field is present (`cf-replay/src/lib.rs:1138-1166`).
- `cf-replay::schemas::recorder_event.schema.json` — declares `asset_ref` as an envelope field (line 88).
- `tools/prototype_run_check.py:82` — `asset_ref` is in the allowed envelope-field set for run bundles.

**Per-clause verdict:**
| Clause | Verdict |
|---|---|
| Run bundle with capture grid screenshots | PASS at envelope level — `asset_ref` field exists, schema'd, and the cosmetic flag is exercised by the same test |
| Each screenshot has `asset_ref` linking to a ledger entry | PASS — the API + envelope are present; the test populates the field with a 64-hex AssetId string |
| `cf-headless replay` validates referenced entries exist + are Fresh | **MISSING** — `grep -r "asset_ref" cf-headless/` returns NO matches. The cf-headless replay verifier does NOT cross-check `Event.asset_ref` against the ledger's `live_entries()` or `verify_entry`. There is no test that asserts replay fails on a missing/drifted asset_ref |

**Gaps:**
- The validation half of this scenario is unimplemented. The envelope field is correctly populated and round-trips, but `cf-headless replay` never consults the ledger.
- No screenshot-specific test wires the capture-grid pipeline (cf-capture) → cf-replay → cf-asset-ledger end-to-end. The capture grid integration is implied future work.

**Verdict:** **PARTIAL (MAJOR gap)** — the envelope-side wiring is correct and tested; the cf-headless-side validation that "referenced ledger entries exist + are Fresh" is missing entirely.

---

### Scenario 11: Determinism contract — same seed reproduces same output

**Gherkin (verbatim):**
```
Given two fresh checkouts on different machines
When both run `cf-mod ledger regenerate <asset_id>`
Then both produce byte-identical output (assuming pinned model + deterministic pipeline)
(Cross-platform determinism: requires Tier 1 SVG to be fully deterministic; Tier 2 ComfyUI uses pinned seeds per workflow)
```

**Test functions that map:**
- `cf-asset-ledger::regenerator::tests::freeze_then_store_round_trip` — single-machine equivalent: corrupt + regen → byte-identical output (`regenerator.rs:457-479`).
- `cf-mod::ledger_cli_integration::regenerate_byte_identical` — same drill at the spawned-binary level (`tests/ledger_cli_integration.rs:163-180`).

**Per-clause verdict:**
| Clause | Verdict |
|---|---|
| Two fresh checkouts on different machines | NOT-TESTABLE-IN-CI — the spec is acknowledging a cross-machine guarantee; CI runs on one machine. The freeze-then-store path makes this PASS by construction since the canonical bytes are stored in the workspace alongside the ledger |
| Both produce byte-identical output | PASS via the freeze-then-store contract |
| Pinned model + deterministic pipeline | PASS via the freeze-then-store contract (the bytes are pinned; the pipeline is never re-run by the regenerator unless a pipeline_runner is registered) |

**Gaps:**
- No CI test exercises cross-machine reproducibility (impossible in single-machine CI without a Docker-cross-platform matrix or fixture comparing pre-baked output from two architectures).
- The `GeneratorRef.model_version` field is present but not asserted to be round-tripped through regen (overlap with Scenario 4).
- The freeze-then-store approach SHOULD be tested with a deliberately non-deterministic pipeline (e.g., a runner that emits random bytes); the test should confirm the regen FAILS (or restores from freeze) rather than silently writing bad bytes. Not present.

**Verdict:** **PASS (with caveat)** — single-machine byte-identical regen is verified; cross-machine determinism is correct by construction via freeze-then-store and not directly testable in CI.

---

### Scenario 12: Audit reports missing + drifted + failed

**Gherkin (verbatim):**
```
Given some ledger entries with broken state
When `cf-mod ledger summary` runs
Then output groups entries by status (Fresh / Stale / Drifted / Missing / Failed)
And lists the asset_ids in each non-Fresh bucket
And CI gate `cf-mod ledger verify --strict --all` exits 0 only if all are Fresh
```

**Test functions that map:**
- `cf-asset-ledger::storage::tests::summarize_groups_by_category_tier_status` — confirms summary buckets by category/tier/status (`storage.rs:483-498`).
- `cf-asset-ledger::cli::tests::cli_summary_groups_by_category_and_tier` (cli.rs:642-664).
- `cf-asset-ledger::cli::tests::observe_summary_json_shape` — confirms the JSON projection has `missing`, `drifted`, `failed`, `stale` arrays (cli.rs:602-618).
- `cf-mod::ledger_cli_integration::summary_groups_status` — spawned-binary `--json ledger summary` then `ledger verify --strict --all` returns non-zero on drift (`tests/ledger_cli_integration.rs:182-207`).
- `cf-mod::ledger_cli_integration::verify_detects_drift_non_zero_exit` — verify-strict-on-drift non-zero exit (overlap with Scenario 3) (`tests/ledger_cli_integration.rs:148-161`).
- `cf-control::server::tests::observe_assets_ledger_summary_returns_summary` (server.rs:2098-2138).
- `cf-control::server::tests::observe_assets_ledger_summary_falls_back_to_empty` (server.rs:2140-2174).

**Per-clause verdict:**
| Clause | Verdict |
|---|---|
| Summary groups by status | PASS — `by_status` BTreeMap (storage.rs:319) |
| Lists asset_ids in each non-Fresh bucket | **PARTIAL** — `LedgerSummary.non_fresh: BTreeMap<String, Vec<String>>` (storage.rs:320) IS populated by `summarize()` (storage.rs:343), AND surfaced in `summary_to_observe_json` as `missing` / `drifted` / `failed` / `stale` arrays (cli.rs:480-485). HOWEVER, this map reflects the stored `regen_status` on the entry, NOT a fresh re-verification. The integration test `summary_groups_status` ACKNOWLEDGES this gap with a comment: `// by_status only reflects entry.regen_status (which is Fresh at write-time); // the drift bucket is populated by verify not summary.` (`tests/ledger_cli_integration.rs:198-199`). So if an entry is silently drifted on disk but its stored status is Fresh, `summary` will NOT list it under Drifted. This is a documented discrepancy with the spec literal "Audit reports missing + drifted + failed" |
| CI gate `verify --strict --all` exits 0 only if all Fresh | PASS — `verify_detects_drift_non_zero_exit` and `summary_groups_status` both assert non-zero on strict verify |

**Gaps:**
- The summary surface does not re-verify entries against disk; it only aggregates the stored `regen_status`. Per spec text, summary should reflect "broken state" — but the only way to get accurate broken-state buckets is to run `cf-mod ledger verify --all` first (which DOES re-hash). The spec's surface contract is ambiguous here; the implementation chose the "aggregation = pure, verify = active" split, which is defensible but means the summary view alone is insufficient to satisfy "Audit reports missing + drifted + failed" for an unverified entry.
- No test asserts that `cf-mod ledger summary` POST-verify reflects re-hashed status. The integration test does run verify then summary, but its summary assertion is purely on `total_entries`, not on `drifted`/`missing` populations.

**Verdict:** **PARTIAL** — the buckets exist, the JSON projection is correct, and `verify --strict` works. The "summary alone reports drift" reading of the spec is unmet because summary does not re-hash; the user must run verify first.

---

### Scenario 13: Ledger size bounded under regen churn (with compact)

**Gherkin (verbatim):**
```
Given a developer regenerates the same asset 10000 times during iteration
When inspecting the ledger
Then it has 10000 append-only entries (no compaction yet)
And `cf-mod ledger compact --keep-latest --before <date>` reduces it to current-state-only
(Compaction is OPTIONAL; CI keeps append-only ledger for traceability)
```

**Test functions that map:**
- `cf-asset-ledger::storage::tests::compact_drops_superseded_history` — appends 2 entries, supersedes one, runs `compact(true, None)`, asserts 2 → 1 lines (`storage.rs:500-535`).

**Per-clause verdict:**
| Clause | Verdict |
|---|---|
| Regenerate the same asset 10000 times | MISSING — no test runs 10k iterations. `storage::tests::append_creates_jsonl_one_entry_per_line` runs 100. Scale not tested |
| Ledger has 10000 append-only entries | MISSING (same root cause) |
| `compact --keep-latest --before <date>` reduces to current-state-only | PARTIAL — the unit test exercises `compact(true, None)` but not the `--before <date>` cutoff. The `keep_after` cutoff is in storage.rs:225-227 but no test covers it. CLI wiring exists in `cf-mod/src/main.rs:181-186` and `cli.rs:373-381` but no integration test exercises the spawned binary |
| Compaction is OPTIONAL; CI keeps append-only | PASS — `keep_latest_only=false` is supported (`storage.rs:217-221`) and the spec literal "OPTIONAL" matches the implementation |

**Gaps:**
- No test loops 10k regens — scale assertion is purely missing.
- No test exercises `compact --before <date>` (the date-cutoff path).
- No integration test (spawned-binary) for `cf-mod ledger compact` at all.
- Compact creates a `.bak` backup at `storage.rs:230` — useful for safety but no test asserts backup is created.

**Verdict:** **PARTIAL** — basic compact-drops-superseded works; `--before <date>`, the 10k-scale claim, and the integration test surface are all unexercised.

---

## Summary Verdict Table

| Scenario | Verdict | Test(s) | Gaps |
|---|---|---|---|
| 1. cf-asset-ledger crate ships | **PASS** | `public_api_round_trip`, `schema_version_locked_at_v1`, workspace+exports inspection | none |
| 2. Append-only JSONL ledger | **PARTIAL** | `append_creates_jsonl_one_entry_per_line` (100), `append_only_one_line_per_entry` (5), supersede unit+integration | CLI does only 5 not 100; no `tail -f` test; no "non-superseded lines byte-stable after supersede" assertion |
| 3. Integrity check detects drift | **PASS** | `verify_entry_drift`, `cli_verify_detects_drift`, `verify_detects_drift_non_zero_exit` | minor: no test asserts BOTH expected + observed hashes in rendered stdout |
| 4. Regenerate produces byte-identical output | **PASS** | `freeze_then_store_round_trip`, `cli_regenerate_byte_identical`, `regenerate_byte_identical` | minor: no test pins `generator.model_version` and asserts it's round-tripped |
| 5. Full re-bake from scratch | **PASS** | `regen_all_walks_every_live_entry`, `full_re_bake_from_scratch_is_idempotent` | "no content/assets/ directory" literal not exercised; regen --all → exit 1 on failure path untested |
| 6. Per-category + per-tier filtering | **PARTIAL** | `list_filter_by_category_and_tier`, `cli_list_filter_category_and_tier`, `list_filters_category_tier` | `--status Drifted` filter has NO end-to-end test; 5×4 matrix not literal |
| 7. Mod pack integration | **FAIL (BLOCKER)** | none | `cf-mod package` is stubbed; no auto-register; no manifest-references-by-id; no install flow; no on-install blake3 check |
| 8. Upstream asset dependency graph | **PARTIAL** | `cascade_walks_dependents`, `mark_dependents_stale_flips_descendants` | `regenerate_entry` does NOT auto-mark dependents Stale (spec implies); no Tier1→Tier2→Tier3 chain test; no spawned-binary `--cascade` integration test |
| 9. Schema version locked at v1 | **PASS** | `schema_version_locked_at_v1`, `validate_entry_json_rejects_schema_drift`, `schema_version_locked_v1` | M39 manifest registration deferred per spec; no "unknown additive field is preserved" test |
| 10. Run bundle references ledger entries | **PARTIAL (MAJOR)** | `record_with_asset_ref_populates_envelope_field` | `cf-headless replay` does NOT validate that referenced ledger entries exist + are Fresh — entire validation half is missing |
| 11. Determinism contract — same seed reproduces same output | **PASS (with caveat)** | `freeze_then_store_round_trip`, `regenerate_byte_identical` | cross-machine reproducibility not directly testable in single-machine CI; correct by construction via freeze-then-store |
| 12. Audit reports missing + drifted + failed | **PARTIAL** | `summarize_groups_by_category_tier_status`, `observe_summary_json_shape`, `summary_groups_status`, `verify_detects_drift_non_zero_exit` | `summary` reports stored `regen_status`, NOT live disk state; an entry drifted on disk but Fresh-in-ledger is NOT bucketed under Drifted by summary alone (user must run verify first) |
| 13. Ledger size bounded under regen churn (with compact) | **PARTIAL** | `compact_drops_superseded_history` | no 10k-scale test; `--before <date>` untested; no spawned-binary `compact` integration test; backup-file creation untested |

**Overall test-run results:** every test currently passing.
- `cargo test -p cf-asset-ledger`: 46/46 PASS
- `cargo test -p cf-mod`: 18/18 PASS (9 unit + 9 integration)
- `cargo test -p cf-control --lib`: 88/88 PASS (includes 2 ledger-summary surface tests)
- `cargo test -p cf-replay --lib`: 31/31 PASS (includes the asset_ref envelope test)

---

## Overall Verdict (BLOCKER / MAJOR / MINOR gaps)

### BLOCKER gaps (1)

**B1.** **Scenario 7 — Mod pack integration is unimplemented.** `cf-mod package` is explicitly stubbed (`cf-mod/src/main.rs:203`) with the message *"cf-mod build is not implemented in M0; package builder lands at M5/M8."* No code path auto-registers mod assets as ledger entries, no `.cfmod` manifest references ledger ids, and no install-time blake3 verification exists. Every Gherkin clause for this scenario is MISSING.
   - Spec impact: the M4A acceptance criteria includes this scenario; closing M4A without it means the "mods write to same ledger" promise (DR-006 mod-parity) is untested.

### MAJOR gaps (2)

**M1.** **Scenario 10 — `cf-headless replay` does not validate `asset_ref`.** The envelope field is correctly populated and round-trips through the recorder. However, `cf-headless replay` never consults the asset ledger; a run bundle that references a deleted or drifted ledger entry replays without warning. The spec literal "`cf-headless replay` validates that referenced ledger entries exist + are Fresh" is unmet.

**M2.** **Scenario 8 — Tier-1 regen does NOT automatically mark dependents Stale.** The spec literal "When the Tier 1 entry is regenerated, then dependents (Tier 2, Tier 3) are marked Stale" is implied to be automatic. The implementation exposes `mark_dependents_stale` as a separate function the caller must invoke; `regenerate_entry` does not call it. Without explicit invocation, a Tier 1 regen leaves Tier 2 entries stale-in-fact but Fresh-in-ledger.

### MINOR gaps (8)

**N1.** **Scenario 2 — CLI integration test does 5 adds, not 100.** The unit test does 100. Scale claim should be CLI-tested.
**N2.** **Scenario 2 — No `tail -f` test.** Append flow is correct by construction (flush after every line) but no test observes streaming consumers.
**N3.** **Scenario 2 — No "non-superseded lines remain byte-identical after supersede" assertion.** `supersede_entry` rewrites the entire file; a regression that touched OTHER lines would not be caught.
**N4.** **Scenario 5 — "Fresh checkout (no content/assets/ directory)" literal not exercised.** Tests delete one file, not the parent directory.
**N5.** **Scenario 6 — `--status Drifted` filter has no end-to-end test.** The `matches()` function handles it, but no test populates `filter.status`.
**N6.** **Scenario 12 — `summary` aggregates stored `regen_status`, not re-hashed live state.** Test acknowledges this: `// by_status only reflects entry.regen_status (which is Fresh at write-time)`. A user-facing "what's broken right now?" query must combine `verify --all` followed by `summary`.
**N7.** **Scenario 13 — No 10k-scale test, no `compact --before <date>` test, no spawned-binary `compact` integration test.**
**N8.** **Scenario 9 — No "unknown additive field is preserved through round-trip" test for forward-compat.** The `extension_fields` map exists but isn't tested for forward-compat preservation.

---

## Recommended fixes

### High priority (closes BLOCKER + MAJOR)

1. **For Scenario 7 (BLOCKER):** add either (a) an explicit deferral marker in the spec / commit message acknowledging mod-pack integration is M5/M8 work and explicitly removing the acceptance clause, or (b) implement at least a minimal `cf-mod ledger register-pack <mod_dir>` that walks every asset in a mod directory, calls `cmd_add` with `--category Mod_Custom --package-source mod:<id>`, and writes per-asset ledger entries. Tests to add:
   - `register_pack_writes_ledger_entries_for_every_asset`
   - `register_pack_uses_mod_custom_category_and_mod_package_source`
   - `install_pack_copies_entries_and_verifies_blake3`

2. **For Scenario 10 (MAJOR):** add a `--validate-asset-refs` flag (or make it default-on) to `cf-headless replay` that, after replay, reads every `Event.asset_ref`, looks up the entry via `LedgerHandle::find`, and runs `verify_entry`. Fail-fast on Missing/Drifted/Failed. Test:
   ```rust
   #[test]
   fn replay_fails_when_asset_ref_is_drifted() { ... }
   #[test]
   fn replay_fails_when_asset_ref_is_unknown() { ... }
   ```

3. **For Scenario 8 (MAJOR):** either:
   - **Code change:** make `regenerate_entry` call `mark_dependents_stale(handle, &entry.id)` AFTER a successful regen, so dependents are auto-marked. Then add a test `regenerating_tier1_auto_marks_tier2_stale`.
   - **OR** document the manual call as the contract and add a test asserting the user-facing CLI message tells the operator to run `mark-dependents-stale` after a non-cascade regen.

### Medium priority (closes MINOR gaps with the most user impact)

4. **For Scenario 6 (N5):** add `cli_list_status_filter` test that adds 2 entries, drifts one, runs verify (to set `regen_status=Drifted` in-memory — or simulate by passing `with_regen_status`), then lists with `--status Drifted` and asserts only the drifted entry surfaces.

5. **For Scenario 12 (N6):** either:
   - **Code change:** add a `cf-mod ledger summary --refresh` flag that re-verifies every entry and updates `regen_status` before aggregating. Add `summary_refresh_repopulates_drifted_bucket`.
   - **OR** document that summary is a passive aggregator and explicitly tell users to run `verify --all` first. Add `summary_after_verify_lists_drifted_assets` end-to-end.

6. **For Scenario 13 (N7):** add an integration test for `cf-mod ledger compact --before <iso8601>` that:
   - appends 5 entries with `--generated-at-iso` set to different dates;
   - calls `compact --before 2026-05-15T00:00:00Z`;
   - asserts only entries dated `>=` the cutoff remain;
   - asserts the `.bak` file exists.

### Low priority (audit hygiene)

7. **For Scenario 2 (N1, N2, N3):** raise the CLI append iteration count to 100; add a `non_superseded_lines_byte_stable_after_supersede` regression test.

8. **For Scenario 5 (N4):** add `regen_from_scratch_recreates_content_assets_directory` that deletes the whole content/assets path before regen.

9. **For Scenario 9 (N8):** add `entry_with_unknown_field_round_trips_via_extension_fields_map`.

10. **For Scenario 4 (minor):** add a test that constructs an entry with `generator.model_version = Some("flux-1.0-dev-deterministic")` and asserts the post-serialize/deserialize entry preserves the field.
