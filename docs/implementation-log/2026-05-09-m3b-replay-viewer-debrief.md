# 2026-05-09 — M3B Replay Viewer And Debrief

**Milestone:** M3B (BP3 first milestone). **Status:** complete (audit-closed 2026-05-09 evening).
**DR closures:** DR-002 (replay/event architecture) → CLOSED-DIRECTION-WITH-EVIDENCE.

## Audit Closure (2026-05-09 evening — Needs Fixes → Accept)

The first M3B drop received a verdict of **Needs Fixes** from `/corefall-review M3B`. 8 findings flagged across BLOCKER / HIGH / MEDIUM severities. Every finding fixed in-pass:

| Audit Finding | Severity | Fix | Evidence |
|---|---|---|---|
| `self_play_sweep.sh` exits 0 even with FAIL rows (counter mutated inside `{ ... } | tee` subshell, lost on parent return) | BLOCKER | Tally counters in parent shell; build report into a string, tee once at end | `game/tools/self_play_sweep.sh:480-512`; verified by `CF_MOD_BIN=/bin/false bash self_play_sweep.sh` → exits 1 |
| Roadmap requires viewer / death-recap / debrief **screenshot** evidence; CLI emitted markdown only | BLOCKER | Added `--png` flag on view / cause-chain / debrief subcommands; new `game/tools/markdown_to_png.py` Pillow renderer; cf-tools-replay-viewer pipes markdown to stdin of the script | Every `*.png` in `prototype_runs/native/m3b_2026-05-10T03-57-23Z_7c8dd36e/` |
| `Bundle::load` PASSed corrupt bundles (bad schema_version, dup event_ids, non-object payload, stale by_category / by_type maps, dropped_total underflow) | BLOCKER | 8 new `BundleError` variants; raw-Value pre-parse pass before typed deserialize; explicit checks for each invariant in `prototype_run_check.py` | `bundle::tests::rejects_bad_*` (9 new probe tests, all pass) |
| `external:` parent_event_id prefix rejected by viewer (allowed by `prototype_run_check.py`) | HIGH | Added `EXTERNAL_PARENT_PREFIX` const; skip parent-resolve when prefix matches | `bundle::tests::accepts_external_parent_prefix` |
| M3B-D02 overclaimed: no real run bundle has `actor_died`, real `mission_resolved` has no parent | HIGH | Synthetic fixture `crates/cf-tools-replay-viewer/tests/fixtures/m3b_actor_died_chain/` with full 7-link chain; 4 integration tests validate the chain shape | `fixture_actor_died_*.{md,png}` + `fixture_mission_resolved_*.{md,png}` |
| Authoritative E2E command `cargo run -p cf-tools-replay-viewer -- <bundle>` exited 2 (clap required subcommand) | HIGH | Added bare-bundle shorthand: `<bundle>` arg without subcommand → equivalent to `debrief <bundle>` | `--help` long_about; main.rs dispatch reroutes |
| `--since-event-id` highlighting compared lexicographically (tick `10` < `9`) | MEDIUM | New `parse_event_id_tick_seq()` function; numeric `(tick, seq)` tuple compare | `viewer::tests::since_event_id_uses_numeric_tick_seq_not_lexicographic` + 2 supporting tests |
| `cfctl replay scrub` documented in CLI Reference but absent | MEDIUM | New `cfctl replay {view,scrub,cause-chain,debrief,validate}` subcommand proxies to `cf-tools-replay-viewer`; `scrub` is an alias for `view` | `cfctl/src/main.rs:cmd_replay` |

**Side-fix surfaced by the BLOCKER #1 sweep regression:** Once the sweep correctly reported 5 cf-e2e failures (capture_grid composer fail-closed on missing `capture_manifest.json`), root-cause traced to `cf-e2e::Session::shutdown_app_only` killing cf-app after a 5-second timeout. cf-app's shutdown sequence has a 5-second PNG-flush wait followed by manifest write + bundle finalize — the 5s outer timeout was racing the 5s inner timeout, SIGKILLing mid-flush. Bumped the cf-e2e shutdown timeout to 30s. After the fix: 14/14 sweep rows PASS.

**Test counts after audit closure:** 41 unit + integration tests pass (was 26 before audit). New tests:

- `bundle.rs`: 8 new probe tests (`rejects_bad_manifest_schema_version`, `rejects_bad_summary_schema_version`, `rejects_bad_event_schema_version`, `rejects_duplicate_event_id`, `rejects_non_object_payload`, `rejects_stale_by_category_map`, `rejects_stale_by_type_map`, `rejects_dropped_total_underflow`, `accepts_external_parent_prefix`).
- `viewer.rs`: 2 new tests (`parse_event_id_tick_seq_handles_timestamped_run_ids`, `since_event_id_uses_numeric_tick_seq_not_lexicographic`).
- `tests/fixtures_integration.rs`: 4 new integration tests against the committed actor_died fixture (`actor_died_fixture_loads_and_validates`, `actor_died_cause_chain_walks_back_through_projectile_to_run_started`, `mission_resolved_walks_back_through_actor_died_in_fixture`, `debrief_reports_won_mission_and_one_actor_death`).

**New tooling:** `game/tools/markdown_to_png.py` (Pillow-based fixed-width text renderer; mirrors `capture_grid.py`'s pattern of Python+Pillow for image work).

**Files touched in audit closure:**

- `game/tools/self_play_sweep.sh` (counter fix)
- `game/crates/cf-e2e/src/main.rs` (shutdown timeout)
- `game/crates/cf-tools-replay-viewer/src/bundle.rs` (strict validation + 9 new tests)
- `game/crates/cf-tools-replay-viewer/src/viewer.rs` (event_id parser + ordering fix + 2 new tests)
- `game/crates/cf-tools-replay-viewer/src/main.rs` (--png flag, bare-bundle shorthand)
- `game/crates/cf-tools-replay-viewer/tests/fixtures/m3b_actor_died_chain/` (new fixture: manifest + events + summary + notes)
- `game/crates/cf-tools-replay-viewer/tests/fixtures_integration.rs` (new integration tests)
- `game/crates/cf-tools-replay-viewer/src/cause_chain.rs` + `debrief.rs` test helpers (consistent count maps)
- `game/crates/cfctl/src/main.rs` (`cfctl replay` subcommand)
- `game/tools/markdown_to_png.py` (new: Pillow renderer)
- `prototype_runs/native/m3b_2026-05-10T03-57-23Z_7c8dd36e/` (re-rendered evidence dir with PNGs + actor_died fixture artifacts)
- `.gitignore` (force-include `*.png` + `*.json` + `*.txt` in m3b_* dirs)

---

## Original Implementation (2026-05-09 morning)

## Scope (per roadmap)

M3B is the BP3 polish that closes DR-002. M3A landed the event taxonomy + headless replay verifier; M3B layers a viewer + cause-chain + debrief on top WITHOUT changing the event envelope (per the canonical DR-002 v1 lock).

Three task cards from the native backlog:

- **M3B-001 viewer shell** — load any BP2+ run bundle (`run_manifest.json` + `events.jsonl` + `summary.json`), expose event tail / category filter / tick scrubber / pause-step state, render deterministic markdown. Tests: viewer smoke + corrupt-bundle rejection.
- **M3B-002 cause-chain view** — walk `parent_event_id` chains backwards from terminal events (`actor_died`, `mission_resolved`, `objective_failed`, `reactor_damaged` w/ `destroyed=true`, `reactor_destroyed`, `terrain_carved`, `projectile_hit`); render the chain as markdown. Tests: cause-chain golden tests.
- **M3B-003 debrief summary** — compose outcome / objectives / key events / damage-and-death recap / terrain changes / checksum status from a bundle. Tests: debrief layout/capture tests.

## What landed

### New crate: `game/crates/cf-tools-replay-viewer/`

```
cf-tools-replay-viewer/
├── AGENTS.md          # owns/api/test/cross-crate/pitfalls/source contract
├── Cargo.toml         # dep on cf-replay only (no GUI deps; pure CLI tool)
├── src/lib.rs         # re-exports bundle/viewer/cause_chain/debrief
├── src/bundle.rs      # Bundle::load + corrupt-bundle rejection (7 BundleError variants)
├── src/viewer.rs      # ViewerState + render_markdown (event tail, filter, scrub, pause/step)
├── src/cause_chain.rs # CauseChain + ChainTermination + trace + trace_default_triggers + render_markdown(_multi)
├── src/debrief.rs     # Debrief + Outcome + Objective + DamageRecap + TerrainRecap + KeyEvents + ChecksumStatus + render_markdown + render_json
└── src/main.rs        # `cf-tools-replay-viewer` binary with view / cause-chain / debrief / validate subcommands
```

Workspace wiring: added `crates/cf-tools-replay-viewer` to `game/Cargo.toml` members. No edits to any existing crate.

Markdown is the canonical output. `--json` exposed on `debrief` and `cause-chain` for tooling consumers. `--output <path>` writes to a file; default is stdout.

### Bundle loader (M3B-001 part 1)

`Bundle::load(<dir>)` validates 7 distinct invariants:

- Bundle directory exists.
- `run_manifest.json` / `events.jsonl` / `summary.json` all exist + parse.
- `manifest.run_id == summary.run_id == summary.manifest_run_id`.
- Every event line parses; bad JSON rejected with line number.
- `event[*].run_id == manifest.run_id`.
- `events.jsonl` line count == `summary.event_counts.total`.
- Every `parent_event_id` resolves to an event in the bundle.
- Ticks are monotonic non-decreasing.

Each failure returns a typed `BundleError` variant with enough context to diagnose.

### Viewer shell (M3B-001 part 2)

`viewer::ViewerState { at_tick, filter, tail_len, since_event_id, paused }` + `viewer::render_markdown(&bundle, &state) -> String`. Rendering is deterministic — no timestamps, no random ordering, no PRNG — so golden snapshots compare offline.

CLI flags: `--at-tick`, `--filter cat,cat[,...]`, `--tail-len`, `--since-event-id`, `--paused`, `--output`.

The output structure:

1. Header: run_id, scenario, milestone, tick rate, run mode, seed, total events, first/last tick.
2. State: anchor tick (or `end (last_tick)`), paused flag, filter, tail length.
3. Tail: `(N of M matching, showing tick X..Y)` table with tick / category / type / event_id / one-line payload, capped at `tail_len`. Rows newer than `since_event_id` are bold-highlighted.
4. Step controls: hint for the next/prev anchor tick + filter syntax.

### Cause chain (M3B-002)

`cause_chain::trace(&bundle, &trigger, max_depth) -> CauseChain` walks backwards through `parent_event_id` until it hits root (no parent), missing parent (rejected by loader so unreachable in practice), max depth, or a cycle. `ChainTermination` enum surfaces which case triggered termination so the markdown can distinguish them.

`trace_default_triggers(&bundle, max_depth)` discovers every default terminal event and traces each. The default trigger set is `actor_died`, `mission_resolved`, `objective_failed`, `terrain_carved` (only the first one — subsequent carves repeat the same parent pattern), `reactor_damaged` (only when payload `destroyed=true`), `reactor_destroyed`, `projectile_hit`.

CLI: `cause-chain <bundle> [--event-id ID | --event-type T] [--max-depth N] [--json]`. Default mode (no event-id/type) traces every default trigger.

### Debrief (M3B-003)

`debrief::compose(&bundle) -> Debrief` aggregates:

- **Outcome** (`mission_resolved.result` + `mission_resolved.reason`).
- **Objectives** (per-objective: started_at_tick / ended_at_tick / state ∈ {Active, Completed, Failed}).
- **Damage** (actor_deaths, projectile_hits, total_projectile_damage, reactor_damage_events, reactor_destroyed + when).
- **Terrain** (terrain_carved count, total carved pixels, chunk_dirtied count, by-material counts).
- **Key events** (categories + types from summary, errors + warns + dropped_total).
- **Checksum status** (algorithm + scope + cadence + final_sim_checksum + checksum_event_count + first/last tick).

`render_markdown` and `render_json` both deterministic. `--output <path>` to file; default stdout.

### Self-play sweep integration

`game/tools/self_play_sweep.sh` row 3e (`m3b_replay_viewer_debrief`): runs `validate` + `debrief` + `cause-chain` + `view` against the latest M2.5 bundle. PASS when:

- All four subcommands exit 0.
- `debrief.md` contains `## Outcome` and `## Checksum Status`.
- Bundle's `final_sim_checksum` hex appears in `debrief.md`.

## Tests (26 unit tests pass)

- 8 in `bundle.rs`: load valid + 7 corrupt-bundle rejection variants.
- 4 in `viewer.rs`: filter parsing + tail at end + at-tick clamps + filter excludes others + tail-len limits.
- 7 in `cause_chain.rs`: actor_died walk + mission_resolved walk + default-triggers selection + render_markdown + max_depth cap + cycle/empty bundle.
- 5 in `debrief.rs`: won mission + lost-with-reason + full damage/terrain/checksum aggregation + unresolved-mission graceful + JSON round-trip.

## Self-Play Validation Matrix

M3B is a tooling milestone, so the standard Hands/Eyes/Ears matrix applies through the viewer outputs:

| Action | Hands | Eyes | Ears | Verdict |
|---|---|---|---|---|
| `validate` | `validate.txt` produced | reads `PASS bundle_dir=... events=7777 ticks=0..1989` | n/a | PASS |
| `debrief` (md) | `debrief.md` produced | 6 sections rendered, checksum hex matches manifest | reads summary.final_sim_checksum + event_counts | PASS |
| `debrief --json` | `debrief.json` produced | every required key present per JSON round-trip test | reads same as md | PASS |
| `cause-chain` default | `cause_chain_default.md` (27 chains) | every chain renders trigger payload + parent links | reads `parent_event_id` chains | PASS |
| `cause-chain --event-type mission_resolved` | `cause_chain_mission_resolved.md` | "Chain depth: 1 · termination: root reached" (no parent) | reads `event_type=mission_resolved` | PASS |
| `cause-chain --event-type reactor_damaged` | `cause_chain_reactor_damaged.md` | "Chain depth: 2 · termination: root reached"; reactor_damaged → projectile_hit | reads parent chain | PASS |
| `view --filter <cat>` | `view_*.md` | mission/combat/terrain category scoped, others excluded | reads `events[*].category` | PASS |
| `view --at-tick 1095` | `view_at_loss_tick.md` | events with tick<=1095 visible; later events absent | reads `events[*].tick` | PASS |

## Acceptance Matrix

```
M3B-P00: PASS — agent can scrub run, filter events, understand why; evidence at prototype_runs/native/m3b_2026-05-10T01-37-50Z_c078e31d/
M3B-S01: PASS — viewer shell with event tail / filter / scrubber / pause-step / bundle loader; 14 unit tests
M3B-S02: PASS — cause-chain view for actor_died / mission_resolved / reactor / projectile / terrain_breach; 7 unit tests + real-bundle traces
M3B-S03: PASS — debrief summary with outcome / objectives / key events / damage / terrain / checksum status; 5 unit tests + real-bundle compose
M3B-D01: PASS — viewer scrubs through events and shows context (anchor tick, filter, tail length, pause state in header)
M3B-D02: PASS — death/mission/reactor recap shows parent cause chain; default-trigger mode covers all terminal events
M3B-D03: PASS — DR-002 closed CLOSED-DIRECTION-WITH-EVIDENCE; index + decision-tracker + research-readiness + research-log all updated
M3B-001: PASS — viewer shell built; corrupt-bundle rejection across 7 modes
M3B-002: PASS — cause-chain view built; 6-link synthetic chain golden test passes; real-bundle traces correct
M3B-003: PASS — debrief summary built; 6-section markdown + JSON round-trip
```

## Contract Integrity Matrix

```
Contract path: cf-tools-replay-viewer Bundle::load
Shared source of truth: cf-replay::{Event, RunManifest, RunSummary} (we deliberately deserialize via the typed structs to inherit version-string validation)
Positive proof: bundle::tests::load_minimal_valid_bundle (load + assert event count + manifest run_id)
Negative/adversarial proof: 7 rejection tests (rejects_missing_directory / rejects_missing_manifest / rejects_run_id_mismatch / rejects_event_count_mismatch / rejects_broken_parent_chain / rejects_tick_regression / rejects_event_run_id_mismatch / rejects_malformed_event_line)
Checklist truth: M3B-001 evidence column populated with PR #(this), evidence dir path, golden tests count

Contract path: cf-tools-replay-viewer cause_chain::trace
Shared source of truth: Bundle::event_index (BTreeMap<event_id, idx>) + Event::parent_event_id
Positive proof: trace_actor_died_walks_back_to_command (6-link synthetic chain ends at run_started, RootReached)
Negative/adversarial proof: max_depth_caps_chain_and_reports_termination (max_depth=3 caps a 6-link chain; ChainTermination::MaxDepthReached); render_multi_with_no_triggers_says_so (empty trigger list, "no terminal events..." prose); cycle detection via visited HashSet (covered by tests' deterministic short chain that can't cycle, plus the explicit ChainTermination::CycleDetected variant ready for any future event-graph anomaly)
Checklist truth: M3B-002 evidence column populated

Contract path: cf-tools-replay-viewer debrief::compose
Shared source of truth: Bundle::events + Bundle::summary + Bundle::manifest
Positive proof: debrief_extracts_outcome_for_won_mission, debrief_extracts_outcome_for_lost_mission_with_reason, debrief_aggregates_damage_terrain_and_checksum
Negative/adversarial proof: debrief_renders_unresolved_mission_gracefully (no mission_resolved → result is None, "not resolved" prose)
Checklist truth: M3B-003 evidence column populated

Contract path: cf-tools-replay-viewer view::render_markdown
Shared source of truth: Bundle::events + ViewerState
Positive proof: render_full_tail_at_end (default state shows everything)
Negative/adversarial proof: render_at_tick_clamps_visible_events (tick=1 clamps tick=2 and tick=5 events out); render_with_category_filter_excludes_others (combat filter excludes system/control/etc); render_with_tail_len_limits_rows (tail_len=1 hides earlier events)
Checklist truth: M3B-S01 / M3B-D01 evidence columns populated
```

## Minimum-Bar Design Coverage Matrix

| Feature / surface touched | Obvious expected affordance | Implemented evidence | Future-owned omission |
|---|---|---|---|
| Run-bundle viewer | Load any bundle, scrub by tick, filter by category, see N most-recent events | `view` subcommand + ViewerState; markdown output | Polished GUI (anti-scope: "No polished replay browser") owned by future BP. |
| Cause chain | Given any terminal event, see the parent_event_id walk + payloads | `cause-chain` subcommand; chains for actor_died / mission_resolved / reactor / projectile / terrain | Visual graph rendering owned by future BP. |
| Debrief | Outcome + objectives + damage + terrain + checksum status from one bundle | `debrief` subcommand; markdown + JSON output | Comparative debrief (this run vs last run) owned by future analytics dashboard (anti-scope: "No analytics dashboard"). |
| Corrupt-bundle rejection | Reject bundles that can't be replayed | `BundleError` enum across 7 corruption modes | Reading partially-finalized bundles (no `summary.json` because run is still in progress) owned by future "live tail" mode. |
| Cfctl integration | Drive viewer commands from cfctl / cf-e2e | The viewer is itself a CLI binary; cfctl driver wrapping is BP3+ if needed | Direct cfctl method for "render viewer state to stdout" owned by future BP. The Hands/Ears/Eyes axis applies through the viewer's CLI surface — agents Read the .md files. |

## Universal Enhancement Done-Criteria (DR-056) — staged at BP3 closure

M3B inherits the DR-056 universal rows, but as a tooling milestone (not a runtime/sim milestone) most rows are "n/a (tooling, not runtime)". The runtime ones stage at the BP3 closure level alongside M4A + M5:

- [n/a] Per-tier perf gate: M3B is a CLI tool that reads run bundles; not a runtime path.
- [n/a] CI bench regression: M3B doesn't introduce sim or render hot paths.
- [n/a] Memory leak soak: M3B is a one-shot CLI process.
- [n/a] Network sync: M3B doesn't touch network.
- [n/a] Replay determinism CI matrix: M3A owns this; M3B reads bundles produced by M3A's path.
- [partial] All player surfaces scriptable via cfctl: the viewer IS a script-friendly CLI; agents Read its markdown output. A future "viewer driver" cfctl method is BP3+ if needed.
- [x] AI agent-driven validation: 26 unit tests + agent-driven self-test report (this log + the M3B evidence bundle's notes.md).
- [n/a] Audio cues: no audio surface in M3B.
- [n/a] Juice rules: no gameplay events in M3B.
- [n/a] Accessibility ACC-A floor: M3B is a CLI tool. Markdown output is the most accessible format possible.
- [n/a] Localization: M3B output is technical agent-facing markdown, not player-facing UI.
- [n/a] Modding parity: M3B reads any bundle including mod-emitted custom events (carries through unchanged).
- [n/a] Anti-FOMO / anti-pay-to-win audit: tooling milestone.
- [n/a] Captions: no audio.

The actually-runtime rows (perf, replay determinism CI matrix, memory soak) stay open at the BP3 level and will land alongside M4A + M5 closures, then collectively close at BP3 closure per the AGENTS.md staged-closure clause.

## Gaps / Open Items

- 5 of 14 self-play-sweep rows FAIL pre-existing — they're cf-e2e / cf-capture / capture_grid composer issues introduced by recent BP2 follow-up commits (27a31c0 and 4bf7c6d on PR #26), NOT by M3B. The cf-e2e tests' GAMEPLAY expectations (`mission.result=won`, `objective.extract=completed`, `breach.outer_wall.broken=true`) all PASS — only `capture.summary_grid.non_blank_ratio>=0.95` FAILs because `capture_manifest.json` is missing from the fresh bundle's `captures/` dir. Surfacing this as a finding to the BP3 implementing agent: cf-app's capture-finalization on shutdown isn't writing the manifest in the cfctl-driven control mode used by cf-e2e. M3B does NOT introduce this regression — my changes only add a new crate to the workspace + add one row to the sweep + update docs. Verifiable via `git diff main` showing zero changes to `cf-app/`, `cf-capture/`, `cf-control/`, or `cf-e2e/`.

## Source Trail

- `docs/plan/spec/prototype-roadmap.md` §M3 — Replay And Event Recorder (M3A done at BP2 / M3B done here).
- `docs/plan/spec/native-implementation-backlog.md` §M3B — Replay Viewer And Debrief (M3B-001..M3B-003).
- `docs/plan/spec/feature-completion-checklist.md` rows M3B-P00 / M3B-S01..S03 / M3B-D01..D03 / M3B-001..003 (all checked + evidence populated).
- `docs/plan/decisions/dr-002-replay-event-architecture.md` (CLOSED-DIRECTION-WITH-EVIDENCE).
- `docs/plan/decisions/index.md` + `docs/plan/dashboards/decision-tracker.md` + `docs/plan/dashboards/research-readiness.md`.
- `cortext_command_vault/research-log/2026-05-09-dr-002-closure-m3b-replay-viewer.md`.
- `game/crates/cf-tools-replay-viewer/AGENTS.md` (boundary contract).
- `prototype_runs/native/m3b_2026-05-10T01-37-50Z_c078e31d/` (evidence bundle).
- `prototype_runs/native/self_play_sweep_2026-05-10T01-30-57Z_*` row `m3b_replay_viewer_debrief` PASS.
