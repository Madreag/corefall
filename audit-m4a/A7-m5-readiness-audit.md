# A7 — Build Health + M5 Readiness Audit

**Scope:** M4A asset-ledger foundation; verifying readiness for M5.
**Date:** 5/13/2026
**Auditor:** A7 subagent (read-only audit; no source files modified).
**Working tree:** `main` @ `3a32c0a` with M4A closure changes uncommitted (`git status` already shows the M4A-introduced files as `??` / ` M`; user owns this state).

---

## Important framing correction

The parent task brief described M5 as the "chassis milestone." That is incorrect per the active spec on disk:

- `specs/active/M5.md` — title: **"Deep Damage Event Surface Lock."** M5 is a declarative event-schema-lock milestone (~60-80 JSON schemas under `cf-replay/schemas/event/`). It ships NO producer code.
- Chassis work lives under **M13** (and is referenced from M5 only as the future producer for `armor.*`, `fluid.*`, `shield.*` events).

This audit therefore evaluates "what does M5 (event surface lock) actually consume from M4A?" rather than the (mis-described) chassis dependency surface. The chassis-surface analysis is included at the end as a forward-looking note in case the parent intended that question.

---

## Build Health

| Check | Result | Notes |
|---|---|---|
| `cargo build --workspace` | **PASS** (exit 0) | `Finished dev profile in 0.46s` (incremental). Full rebuild was previously green per cargo cache. |
| `cargo test --workspace` | **PASS** (exit 0) | 27 binary crates with non-zero tests; tail of sweep shows `632 passed; 0 failed; 0 ignored` aggregated across `test result: ok` blocks; 0 FAILED reported anywhere. `cf-asset-ledger` itself: 46/46 pass. `cf-mod ledger_cli_integration` test binary: 9/9 pass. |
| `cargo clippy --workspace -- -D warnings` | **PASS** (exit 0) | Clean. No warnings promoted to errors. |
| `cargo fmt --check` | **FAIL drift** (20 diff sites) | Pre-existing drift in `cf-actor`, `cf-e2e`, `cf-headless`, `cf-mission`, `cf-physics`, `cf-render-2d`, `cf-ui`. **NONE in cf-asset-ledger / cf-mod / cf-replay / cf-control / cfctl** — i.e. all M4A-touched crates are clean. The drift is unrelated to M4A. |
| `cargo deny check` | **CONFIG REJECTED** | cargo-deny 0.19.4 refuses `unmaintained = "warn"` in `game/deny.toml` line 3 (it expects `["all", "workspace", "transitive", "none"]`). Pre-existing config drift, not introduced by M4A. License/advisory checks did not run. |

Bottom line: **M4A's own code is green on every relevant gate** (build / test / clippy / fmt-of-M4A-files). Workspace-wide fmt drift + cargo-deny config drift are pre-existing and unrelated.

---

## Determinism Risks

| # | Site | Risk | Verdict |
|---|---|---|---|
| 1 | `entry.rs:413` — `AssetEntryBuilder::build()` falls back to `chrono::Utc::now().to_rfc3339()` when `generated_at_iso` isn't set. | **HAZARD (low impact, real)**. `cmd_add` in `cli.rs:84-160` does NOT plumb a `--generated-at-iso` flag through `AddArgs`; every production-path `cf-mod ledger add` invocation gets wall-clock time. The ledger file's blake3 is therefore NOT byte-deterministic across CI runs even when output assets ARE byte-identical. Tests work around this via `with_generated_at_iso("2026-05-13T00:00:00Z")`. | HAZARD |
| 2 | `storage.rs:193` — `supersede_entry` writes `deprecated_at = Utc::now().to_rfc3339()`. | **HAZARD (same shape as #1)**. Re-adding an asset stamps wall-clock time onto the deprecated entry's rewrite. | HAZARD |
| 3 | `storage.rs:309-352` — `LedgerSummary.non_fresh: BTreeMap<String, Vec<String>>` ordering. | **DETERMINISTIC per input**. Keys are sorted (BTreeMap). The `Vec<String>` payload is appended in `&[AssetEntry]` walk order — i.e. file order, which is itself deterministic when callers pass `handle.read_all()` output (both the engine surface and cfctl inline summary do this). No sort jitter. | SAFE |
| 4 | `cf-replay/src/lib.rs:585-613` — `Recorder::record_with_asset_ref` takes `cosmetic: bool` from the caller. Capture-grid / audio-playback events are required to be cosmetic per M4A spec. The integration test at `lib.rs:1143` sets `cosmetic: true`. | **CALLSITE-DISCIPLINE HAZARD (not an M4A bug)**. There is no production callsite yet (M4A is foundation; M9A+/M32A+ will integrate). If a future producer omits `cosmetic: true`, the capture-grid screenshot will participate in `determinism.sim_checksum` and break replay. Recommend: introduce a typed wrapper or a `#[must_use]` builder enforcing cosmetic=true for capture surfaces. | HAZARD (forward) |

### Concrete impact of HAZARD #1 + #2

- The output asset bytes ARE deterministic (the freeze-then-store path enforces blake3 match in `regenerate_entry`, see `regenerator.rs:113-120`).
- The ledger.jsonl FILE bytes are NOT deterministic across CI runs (timestamps differ).
- This violates the literal reading of M4A's "byte-identical regen on fresh checkout" promise IF the promise is taken to include the ledger metadata file itself. The Acceptance Criteria say "regenerated output's blake3 matches the original ledger blake3" — only the output_blake3 field, not the ledger line. So the spec-letter is satisfied; the spec-spirit needs the ledger file to be reproducible too, which today it is not.

---

## Concurrent-Write Safety

| # | Site | Risk | Status |
|---|---|---|---|
| 1 | `storage.rs:79-99` — `append()` uses `OpenOptions::create(true).append(true)` plus a single `write_all`. | **POSIX append atomicity caveat**. Per POSIX, `O_APPEND` writes are atomic for byte counts ≤ `PIPE_BUF` (Linux: 4096; macOS regular files: undefined for files, only guaranteed for pipes per the standard — Apple has historically honored 4096 in practice but does not guarantee it). A serialized `AssetEntry` JSONL line for a vanilla entry is ~500-1500 B; with `extension_fields` or long prompts it can exceed 4096 B and lose atomicity. The doc comment at `storage.rs:80-83` mentions "POSIX append-mode is atomic for writes up to PIPE_BUF" but does NOT enforce a line-length cap. | HAZARD (low frequency; real for fat entries) |
| 2 | `storage.rs:170-209` — `supersede_entry` is a read-modify-rewrite: `read_all()` → mutate → `truncate(true)` → write every line. No file lock. | **RACE WINDOW (real)**. If two `cf-mod ledger add` processes target the same canonical_name at the same moment, both append a line, both call `supersede_entry`, and the LATER truncate-write clobbers the earlier one's superseded markings. Updates can be lost. | HAZARD |
| 3 | `storage.rs:4-7` doc comment claims "concurrent-write-safe via OS-level advisory locking when supported (best-effort)". | **DOC OVER-CLAIMS**. There is no `fcntl(F_SETLK)` / `flock(LOCK_EX)` call anywhere in the crate. The "best-effort" caveat is meaningless because nothing is attempted. The comment is misleading. | HAZARD (doc) |
| 4 | `regenerator.rs:270-304` — `rewrite_with_status` (called by `mark_dependents_stale`) is also a truncate-rewrite. Same race. | Same as #2. | HAZARD |
| 5 | `storage.rs:212-251` — `compact()` is truncate-rewrite; runs `std::fs::copy(...jsonl, ...jsonl.bak)` first. Same race window between read+backup and the truncating writer. | Same as #2. | HAZARD |

### Recommended fix

Two viable patterns; pick one before M5 starts:

1. **fcntl LOCK_EX advisory lock** wrapping every read-modify-write in `storage.rs` (append + supersede + compact) and every write in `regenerator.rs` (`rewrite_with_status`). Pure-Rust crate: `fs2 = "0.4"` (already wired in many embedded Rust ecosystems). Adds ~20 LOC and a workspace dependency.
2. **Per-mod sub-ledger pattern** as called out in M4A spec § "Pitfalls": each parallel writer creates its own `ledger_<mod_id>.jsonl` and a build-end pass merges them into the canonical file. Matches the spec's recommendation and avoids any locking primitive. Heavier surface change (writer API per-mod, merge tool).

Either works. The lock pattern is cheaper to ship; the sub-ledger pattern is more scalable for many parallel mod builds.

---

## M5 Dependencies (active M5 spec)

`grep -in -e asset -e ledger -e AssetId -e sprite -e asset_ref -e cf-asset -e cf_asset` on `specs/active/M5.md` returns **only one match**:

```
187: ... origin.shot_force_feedback { ... chassis_layer ... }
```

— and that match is for the noun "chassis" inside an event payload field name, NOT a cf-asset-ledger reference. There are **zero textual mentions** of `cf-asset-ledger`, `asset_ref`, `AssetId`, `ledger.jsonl`, or `observe.assets.*` anywhere in `M5.md`.

**Verdict:** M5 (event surface lock) has **zero direct surface dependencies on M4A**. M5's only contact with M4A is transitive: M5 schemas conform to M4's locked envelope, which already includes the optional `asset_ref` envelope field; the M4A integration into the envelope happened during M4 close and is already proven by `record_with_asset_ref_populates_envelope_field` (cf-replay/src/lib.rs:1143).

### Cross-checks (no blockers found)

| M5 acceptance criterion | M4A satisfies? |
|---|---|
| `cf-mod validate game/crates/cf-replay/schemas/` exits 0 | YES — `cf-mod validate` already understands ledger.jsonl (cf-mod/src/main.rs:469), schema-only paths, and is unaffected by M5 schema additions |
| Each schema declares `schema_version="0.1"` matching M4's envelope | YES — M5 work happens entirely in `cf-replay/schemas/event/`, no M4A surface required |
| `observe.assets.ledger_summary` query | NOT REQUIRED BY M5. Surface exists at `cf-control/src/server.rs:1408-1437` with empty-summary fallback. |

**No BLOCKER for M5 was found.** M5 can begin against the current M4A surface as-is.

### Forward-looking note: when chassis (M13) lands, what does it need?

The parent task brief asked about chassis-milestone dependency. M4A satisfies the chassis-prerequisite surface:

- `AssetCategory::ChassisSprite` exists at `category.rs:30` and is one of the 16 enum variants, serialized as `"ChassisSprite"`.
- `cf-chassis` crate exists (`game/crates/cf-chassis`) and does NOT yet depend on `cf-asset-ledger` — adding the dep when M13 ships is mechanical (no circular dep, since cf-asset-ledger depends on nothing in the cf-* tree).
- The cfctl scripts under `game/scripts/cfctl/m5_chassis_*.cfctl.json` are legacy artifacts from an older numbering scheme; they reference observe/inspect surfaces that already exist. No M4A change needed.

---

## Backward Compatibility

| # | Aspect | Status | Detail |
|---|---|---|---|
| 1 | New optional Rust struct fields use `serde(default)` | **PASS** | `entry.rs:118-156` — every optional field uses either `#[serde(default)]` or `#[serde(default, skip_serializing_if = ...)]`. `negative_prompt`, `palette_ref`, `style_lora`, `upstream_assets`, `additional_outputs`, `generated_by_human`, `human_edit_notes`, `package_source`, `license`, `regen_inputs`, `regen_validated_at`, `regen_status`, `superseded_by`, `deprecated_at`, `schema_version`, `extension_fields` are all defaulted. Required fields (id/category/kind/canonical_name/tier/pipeline/generator/prompt/seed/output_path/output_format/output_size_bytes/output_blake3/generated_at_iso/generated_on_machine/regen_command) cannot be added going forward without a schema bump — consistent with the spec. |
| 2 | Unknown category handling | **REJECT with clear error** | `category.rs:9-25` declares the enum without `#[serde(other)]` or `#[serde(deny_unknown_fields)]`. A JSON entry with `"category": "FutureCategory"` fails serde deserialization with a "unknown variant" error message. Then `validate_entry_json` in `lib.rs:96-119` surfaces it cleanly. **This is the spec-correct behavior** because adding a category SHOULD require a code bump — categories drive routing logic across cf-mod, cf-control, cfctl. A future M4A.1 that adds `ChassisSprite_v2` (hypothetical) must ship a code patch, not a content patch alone. |
| 3 | JSON schema `additionalProperties` | **STRICT (incompatible with serde-default growth)** | `schemas/v1/asset_entry.schema.json:7` sets `"additionalProperties": false` at the root, AND `schema_version` is an enum locked to `["1.0.0"]`. This means adding a new optional field via serde-default at the Rust layer cannot be persisted-then-validated under the v1 schema. Future optional fields **must** land in `extension_fields` (which permits arbitrary keys via `additionalProperties: true` on that sub-object). If a v1.1.0 wanted a true root-level field, the schema file would need to bump and re-allow `["1.0.0", "1.1.0"]`. | DOC GAP |
| 4 | M39 schema-registry contract | **NOT YET WIRED** | M4A spec § "Closure procedure" item 4 says "Register AssetEntry schema in M39's manifest at M39 close." M39 isn't merged yet (M39 lives at `specs/active/M39.md`). The reservation is documented at `lib.rs:14-17`. No M5 blocker; flag for M39 implementer. |

---

## Performance for 5000-Asset Roster

Assumptions: launch roster ~5000 entries; sprite assets avg ~50 KB; audio assets up to ~5 MB; ledger.jsonl avg line ~1 KB so file ~5 MB total. CI hardware: blake3 ~1 GB/s on a single core; SSD random write ~500 MB/s.

| Op | Implementation | Complexity (entries=N) | At N=5000 |
|---|---|---|---|
| `summarize` | one-pass scan, BTreeMap inserts | O(N log C) where C = categories+tiers+status buckets | ~1-5 ms (negligible) |
| `live_entries` | full read + HashMap dedup + Vec sort by canonical_name | O(N log N) | ~2-10 ms |
| `verify_entry` (single) | blake3 streaming hash of one file | O(file_size) | 50 KB sprite → <1 ms; 5 MB audio → ~5 ms |
| `verify_all` | walks live entries + verify each | O(N × avg_file_size) | sprite-heavy roster: ~5000 × 1 ms = ~5 s; audio-heavy: dominated by audio files, can be 30+ s |
| `append` | open + write_all + flush per call | O(line_size) | <1 ms |
| `supersede_entry` | **full file rewrite per call** | **O(N) read + O(N) write** | ~10-30 ms per call |
| `compact` (one-shot) | full rewrite once | O(N) | ~10-30 ms |
| `mark_dependents_stale` | full BFS + full file rewrite | O(N) + O(N) | ~10-30 ms per root |

### Concrete performance HAZARD: `supersede_entry` called per-entry

`cmd_add` in `cli.rs:148-159` always calls `supersede_entry` when a prior live entry with the same id exists. A full re-bake sweep where every entry has a prior version (i.e., a re-run of `cf-mod ledger regenerate --all` that also calls `cf-mod ledger add`) does **5000 supersede passes × O(5000) work each = O(N²) = 25 M file-line writes** for one sweep. At 5 MB ledger size, that's **~125 GB of disk I/O** for what could be a single rewrite.

In practice the sweep is usually `regenerate`, not `add`. `regenerate_entry` does NOT call `supersede_entry` — it only verifies blake3 and writes the file. So the worst case is rarer than the math suggests, **but** the door is open:

- Any pipeline tool that re-adds (rather than regenerates) en masse will hit O(N²).
- Mod-pack `cf-mod ledger add` loops on install/uninstall churn could see this.

### Recommended fix

Batch supersede:

```rust
impl LedgerHandle {
    pub fn supersede_many(&self, pairs: &[(AssetId, AssetId)]) -> Result<usize, StorageError> { ... }
}
```

— single read, one in-memory mutation pass that resolves every pair, single truncate-write. Drops O(N²) sweep cost to O(N). Same correctness; ~5000× speedup in the worst case.

Also: a `cmd_add_batch(paths, &[args])` thin wrapper for bulk pipeline tools.

### Other perf notes

- `verify_entry` re-hashes the file with a 64 KB read block (`integrity.rs:31` — `const READ_BLOCK: usize = 64 * 1024`). Fine for 5000 entries.
- `live_entries` builds a HashMap and then sorts the values — O(N log N). For N=5000 negligible.
- No parallelism. `verify_all` could be `par_iter` via `rayon` for a ~8× speedup on multi-core CI, but it's not required for the launch roster sizes — current cost is dominated by blake3 I/O which is already streaming.

---

## Closing Gaps

### BLOCKER for M5

**None.** M5 (event surface lock) has zero direct dependencies on M4A surfaces beyond the M4 envelope's `asset_ref` field, which is already wired (`cf-replay/src/lib.rs:103-110, 585-613`).

### MAJOR (recommend before any heavy ledger churn lands; not strict blockers for M5)

1. **Concurrent-write race in `supersede_entry` / `compact` / `rewrite_with_status`** — read-modify-rewrite without OS-level locking. Two parallel `cf-mod ledger add` calls on the same asset id can lose updates. Fix: `fcntl LOCK_EX` advisory locks OR per-mod sub-ledger pattern (M4A spec § "Pitfalls" calls this out).
2. **O(N²) sweep cost** in any caller that loops `cmd_add` over 5000 assets with prior live entries. Fix: `supersede_many` batched primitive + `cmd_add_batch` CLI wrapper.
3. **`generated_at_iso` defaults to wall-clock** in production paths — ledger.jsonl bytes are not reproducible across CI runs even with byte-identical outputs. Fix: surface `--generated-at-iso` (or `--from-source-mtime` / `--zero`) flag on `cf-mod ledger add` AND default to a deterministic stamp (e.g. `output_blake3`'s first 16 chars, or the source-content commit-time) in CI mode. Same for `deprecated_at` in `supersede_entry`.
4. **`storage.rs:4-7` doc comment claims advisory locking that doesn't exist.** Either ship the lock OR fix the doc.

### MINOR

1. **Schema `additionalProperties: false` + `schema_version: ["1.0.0"]`** at the root locks out new root-level optional fields under v1; future growth must land in `extension_fields`. Document this explicitly in the spec's "Forward-compat" notes.
2. **Pre-existing fmt drift** in 20 places under `cf-actor`, `cf-e2e`, `cf-headless`, `cf-mission`, `cf-physics`, `cf-render-2d`, `cf-ui` — NOT M4A's fault; flag for separate cleanup PR before M5 starts. Trivial `cargo fmt --workspace` fix.
3. **`game/deny.toml`** uses `unmaintained = "warn"` which cargo-deny 0.19.4 rejects. Pre-existing; needs migration to the new `unmaintained = "workspace"` (or `"all"`) form. Not M4A; flag for separate cleanup PR.
4. **Forward-looking integration risk:** when M9A / M32A / M37A pipeline producers add `record_with_asset_ref` callsites, they MUST pass `cosmetic: true` for capture-grid / audio-playback events; otherwise determinism breaks. Recommend a typed wrapper like `record_cosmetic_capture(asset_ref, ...)` that fixes `cosmetic=true` at compile time. (Not an M4A bug; producer-discipline forward-looking only.)
5. **M39 schema-registry registration** is reserved but not wired (M4A spec closure step #4). Track for M39.

---

## Recommended Fixes Before M5 Starts

Ordered by cost/benefit (cheapest first):

1. **`cargo fmt --workspace`** — clears the 20-site pre-existing drift. <1 minute; clears the gate.
2. **Fix the misleading `storage.rs:4-7` doc comment.** Either remove the "best-effort advisory locking" claim OR ship the lock. ~5 minutes for the doc fix; ~30 minutes if shipping `fs2`-backed locks (preferred — see #3).
3. **Add fcntl LOCK_EX advisory locks** around `append`, `supersede_entry`, `compact`, `rewrite_with_status`. Add `fs2 = "0.4"` to workspace deps. Wrap the writer paths with `file.lock_exclusive() ... file.unlock()`. ~1 hour with tests.
4. **Plumb `--generated-at-iso` through `AddArgs`** AND default to a deterministic value when `$CF_DETERMINISTIC_LEDGER=1`. ~30 minutes. Optional: also a `--generated-on-machine` flag (the `hostname` env fallback is already deterministic-ish in CI containers).
5. **Add `LedgerHandle::supersede_many(&[(AssetId, AssetId)])`** primitive + a `cmd_add_batch` thin CLI wrapper for bulk pipeline tools. ~1-2 hours with tests. Drops sweep cost from O(N²) to O(N).
6. **Defer:** fix `game/deny.toml` `unmaintained` key; M39 schema-registry registration; typed capture-cosmetic wrapper. These are not M5-blocking and have their own owning milestones.

After items 1–5 land, the ledger is **production-ready for a 5000-asset launch roster with parallel CI workers**, and M5 can ship without blocking on any further M4A work.

---

## Appendix: Evidence Summary

- M4A source: `/Users/erol/projects/corefall/game/crates/cf-asset-ledger/` (lib.rs 8.4 KB; entry.rs 16.5 KB; storage.rs 21.5 KB; integrity.rs 11.0 KB; regenerator.rs 20.3 KB; category.rs 14.5 KB; cli.rs 25.4 KB).
- Spec: `/Users/erol/projects/corefall/specs/done/M4A.md` (status `done`).
- M5 spec (active): `/Users/erol/projects/corefall/specs/active/M5.md` — Deep Damage Event Surface Lock; zero cf-asset-ledger references.
- Tests passing: 46 in cf-asset-ledger; 9 in cf-mod ledger_cli_integration; 632 workspace-wide (sweep aggregate; 0 failed).
- Lint: `cargo clippy --workspace -- -D warnings` clean.
- Schema: `/Users/erol/projects/corefall/game/crates/cf-asset-ledger/schemas/v1/asset_entry.schema.json` (locked v1.0.0).
- Cross-crate plumbing: cf-mod (CLI), cf-replay (`record_with_asset_ref` + `AssetRefRecordParams`), cf-control (`observe.assets.ledger_summary`), cfctl (`ledger-summary` subcommand). All exit-coded correctly per integration tests.
- Empty canonical ledger file: `/Users/erol/projects/corefall/game/content/asset_ledger/ledger.jsonl` (1 byte — just a newline; first entries will land with M9A SVG pipeline at BP6+).
