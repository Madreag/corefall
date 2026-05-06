# Changelog

Repo-only implementation changelog for Corefall.

The canonical roadmap, checklist, specs, and decision records remain in:

```text
/Users/erol/projects/cortex-command-repos-all/cortext_command_vault
```

Use this file to summarize what changed in the implementation repo. Do not copy the whole vault here.

## Unreleased

### Fixed (M0.4 — F7 path-safety follow-up)

After the M0.3 verdict, an independent reviewer recommended landing **F7 only**: a CI assertion that no `prototype_runs` directory exists outside `./prototype_runs` at the corefall repo root, plus a `cf-replay` unit test that the bundle writer resolves to the repo root rather than the cwd.

- **F7 path safety, ownership-correct.** Moved `default_run_bundle_root` and `resolve_run_bundle_root` from `cf-control::runtime` to `cf-replay::bundle_paths` (the natural owner — `cf-replay` writes the bundles). `cf-control::runtime` re-exports both for backwards compatibility, so `cf-app`, `cfctl`, and the live WebSocket integration tests are unaffected. Added 5 cf-replay unit tests proving the resolver:
  1. Returns `<repo>/prototype_runs/native` when cwd is `<repo>/game/` (the regression case for the M0.3-F9 root cause).
  2. Returns `<repo>/prototype_runs/native` when cwd is `<repo>/`.
  3. Walks up from a nested cwd (`<repo>/game/crates`) to find `game/Cargo.toml` and rejoins the repo root.
  4. Returns the explicit override unchanged when one is supplied.
  5. Falls back to the default when none is supplied.
- **CI gate against stray `prototype_runs/` directories.** `.github/workflows/ci.yml` adds an "enforce repo-root prototype_runs path (M0.4-F7)" step that runs `find . -type d -name prototype_runs -not -path './prototype_runs' -not -path './prototype_runs/*' -not -path './target/*' -not -path './.git/*'` and fails CI if any stray directory exists. This protects the repo-root contract from future regressions where a binary defaults to a relative path under `game/`.
- **Captured M3 follow-up.** `system.run_finished` checker tightening + `expected_outcome` manifest enum (`clean | panic | abort`) added as new task card `M3-006 run-finished outcome contract` in `cortext_command_vault/spec/native-implementation-backlog.md`. Owns: `cf-replay`, `references/prototype-run-bundle-schema.md`, `tools/prototype_run_check.py`. Not implemented in M0.4 — M3 closes DR-002 and is where the contract belongs.

**Test count**: 73 tests passing (up from 68 in M0.3; +5 = the 5 new `cf-replay::bundle_paths` regression tests).

**M0.4 acceptance bundle**: `m0_2026-05-06T05-30-36Z_8466a407` (cf-app headless 60 Hz / 300 ticks / 5.004 s wall) — written to absolute `/Users/erol/projects/corefall/prototype_runs/native/...`, post-run scan confirms zero stray `prototype_runs` directories anywhere else in the tree, canonical checker reports `errors 0`.

### Fixed (M0.3 — contract-integrity review loop)

Ran the project-local `corefall-review` loop for M0 again after M0.2 and fixed every verified finding found in scope. **Root causes fixed:**

- **F7 — strict JSON-RPC validation boundary was incomplete.** Several server param structs accepted unknown fields, some handlers defaulted unsupported values, and the engine could report `accepted` for work it did not or could not perform. `cf-control` now uses strict `#[serde(deny_unknown_fields)]` request structs, rejects unsupported/empty/zero-value inputs before dispatch, rejects `act.player.move` until M1 owns actors, rejects unsupported `runbundle.write.id_override`, and rejects unsupported `observe` filters/rates. Regression coverage: `unknown_params_reject_every_m0_method`, `unsupported_m0_params_reject_before_dispatch`, `step_zero_is_rejected_without_status_drift`, `run_for_zero_ticks_is_rejected_without_status_drift`, `act_player_move_rejects_until_m1_actor_exists`, `runbundle_id_override_is_rejected_until_supported`, plus 4 live WebSocket negative tests.
- **F8 — final run-bundle evidence used the same flag for mid-run snapshots and final exit evidence.** `cf-app --headless-smoke --control-api --write-run-bundle` now writes the final bundle on exit even if a mid-run `runbundle.write` already wrote the bundle, so successful control-script bundles include `system.run_finished`.
- **F9 — default run-bundle roots were cwd-relative.** Standard validation runs from `game/`, but the contract root is `/Users/erol/projects/corefall/prototype_runs/native`. `cf-app` and `cfctl` now share `cf_control::runtime::default_run_bundle_root()` / `resolve_run_bundle_root()`, and cfctl auto-launch passes that absolute default to spawned `cf-app`. Accidental `game/prototype_runs` output was removed.
- **Structural cleanup:** removed the stale duplicate mutable `EngineMutable.run_status`; observations now derive run state directly from `SimClock::mode()` plus shutdown state, eliminating the drift source rather than masking it.

**Final validation (all PASS):** `cargo fmt --all --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (68 tests + doctests), `cargo build --release`, `cargo run -p cf-control --example dump_schemas -- --check`, `cargo run -p cf-mod -- validate content/`, and `cargo run -p cfctl -- observe --once`.

**Final M0.3 evidence bundles (all PASS canonical checker; all under repo-root `prototype_runs/native/`):**

| run_id | mode | tick_rate_hz | ticks | wall_seconds | notes |
|---|---|---:|---:|---:|---|
| `m0_2026-05-06T04-46-04Z_1ad62cb4` | cfctl run --paced | 60 | 300 | 5.004 | repo-root default path; `system.run_finished` present |
| `m0_2026-05-06T04-46-14Z_2c7f5b05` | cfctl run --paced | 120 | 600 | 5.003 | repo-root default path; `system.run_finished` present |
| `m0_2026-05-06T04-46-27Z_a9675fc6` | cf-app headless-smoke | 60 | 300 | 5.006 | direct app default path; `system.run_finished` present |
| `m0_2026-05-06T04-46-37Z_56e26f4b` | cfctl script roundtrip | 60 | 6 | server | mid-run `runbundle.write` + final exit write; `system.run_finished` present |

### Fixed (M0.2 — independent-review stabilization pass; supersedes M0.1)

After the M0.1 verdict, an independent reviewer found six verified release-gating issues. M0.2 closed every one. **Root-cause rule applied:** no parallel test/tool paths that bypass production code — `cfctl`, `cf-app`, run bundles, scenario loading, metadata, schema validation, and control dispatch now share the same contract paths.

- **F1 — `cfctl run` and `cfctl observe --inline` now share the production config path with `cf-app`.** Both binaries route through the new `cf_control::runtime::build_engine_config(ConfigInputs { .. })` helper which loads the scenario manifest, calls `M0EngineConfig::for_loaded_scenario`, and stamps real `commit_sha` (with `-dirty` suffix), `rust_version`, `bevy_version`, `config_hash`, and the scenario manifest's `expected_tests` / `region` / `seed`. The legacy test-only constructor was renamed `M0EngineConfig::for_test_scenario_only` and marked `#[doc(hidden)]` so accidental production use stands out in code review and grep. Acceptance bundle `m0_2026-05-06T04-14-45Z_25e6cb16` (cfctl run) now ships identical metadata to `m0_2026-05-06T04-14-25Z_c6ed64df` (cf-app inline) — `commit_sha=b97c0b1d14b2-dirty`, `expected_tests=["M0-SMOKE-01"]`, real rustc + bevy versions, identical final checksum at the same seed.
- **F2 — `schema_version` is now MANDATORY on every JSON-RPC client→server request.** Pre-fix the server's `check_schema_version` returned `Ok(())` when the field was absent; several handlers (`act.settings.set`, `runbundle.write`, `system.shutdown`, `observe.once`, `observe.subscribe`/`unsubscribe`) used `unwrap_or` defaults so missing params silently succeeded. Now `check_schema_version` requires the field BEFORE any handler runs, and missing/non-numeric/mismatched values return `-32602 InvalidParams` with `data.reason = "schema_version_missing"` (or `schema_version_mismatch`). New tests: `missing_schema_version_rejects_every_m0_method` (server-level, covers all 14 M0 methods), `missing_params_object_rejects` (request without a `params` object), plus 2 live WebSocket round-trips (`live_ws_missing_schema_version_rejects_act_player_move`, `live_ws_missing_schema_version_rejects_runbundle_write`).
- **F3 — `scenario.load` with seed override is now REJECTED, not faked.** Pre-fix the engine destructured `ScenarioLoad { scenario, .. }` and silently discarded the `seed`. Now: same-scenario + matching-seed = accepted (no-op); same-scenario + mismatched-seed = rejected with `seed_override_not_supported_in_m0` and a fix-hint pointing to `scenario.reset` / `cf-app --seed <n>` relaunch; different-scenario = rejected with `scenario_swap_not_supported_in_m0`. The recorder logs `control.command_rejected` with `active_seed` + `requested_seed`. New tests: 3 engine-level (`scenario_load_with_mismatched_seed_is_rejected`, `scenario_load_with_matching_seed_is_accepted`, `scenario_load_unknown_scenario_is_rejected`) + 3 live WebSocket (`live_ws_scenario_load_with_mismatched_seed_rejected`, `live_ws_scenario_load_with_matching_seed_accepted`, `live_ws_scenario_load_unknown_scenario_rejected`). Direct cfctl wire trace: `cfctl scenario load m0_blank --seed 7` → `code:-32099, message:"command_rejected", data:{reason:"seed_override_not_supported_in_m0", tick:6}`.
- **F4 — `system.tick_sample` event implemented.** The M0-003 task card requires it as evidence of per-tick performance. The engine now emits a `system.tick_sample` every `cadence_ticks` (60) carrying `{tick_rate_hz, window_ticks, avg_tick_ms, max_tick_ms, p99_tick_ms, samples_observed}`. Visible in every M0.2 bundle: 60 Hz/300 ticks → 5 tick_sample events; 120 Hz/600 ticks → 10 events. New test: `tick_sample_event_emitted_at_cadence`.
- **F5 — M0-008 controlled panic test implemented end-to-end.** New unit test `panic_in_sub_thread_emits_system_panic_event_and_increments_severity`: spawns a real sub-thread that calls `panic!`, catches via `JoinHandle::join`, routes the captured payload through the same `report_panic_to_recorder` function the global panic hook drives, asserts `system.panic` event lands AND `event_counts.by_severity.error` increments. New `cf-app --debug-inject-panic-at-tick <n>` flag spawns a sub-thread that panics at the named tick — the global panic hook routes through the engine's reporter, and the engine's new lock-free `current_tick` atomic ensures the panic event records at the engine's actual tick (preserving events.jsonl monotonicity). Acceptance bundle `m0_2026-05-06T04-14-03Z_03164834` proves it: panic injected at tick 60, recorded at tick 61, `event_counts.by_severity.error: 1`, bundle PASSES the canonical checker.
- **F6 — checklist + log + CHANGELOG updated honestly.** `feature-completion-checklist.md` no longer claims `M0-002`/`M0-003`/`M0-008` complete with hidden caveats; the "deferred to M2" / "stub binary" / "deferred to a follow-up task" notes are gone. Implementation log appends an M0.2 section with the ID-by-ID acceptance matrix.

**Acceptance bundles (all PASS `python3 game/tools/prototype_run_check.py`)**:

| run_id | path | tick_rate_hz | ticks | wall_seconds | notes |
|---|---|---:|---:|---:|---|
| `m0_2026-05-06T04-14-25Z_c6ed64df` | cf-app inline | 60 | 300 | 5.001 | F4: 5 tick_sample events |
| `m0_2026-05-06T04-14-30Z_a988d3b3` | cf-app inline | 120 | 600 | 5.002 | F4: 10 tick_sample events |
| `m0_2026-05-06T04-14-35Z_63c979ed` | cf-app headless+control-api | 60 | 300 | 5.001 | Same checksum as inline → F1 contract parity |
| `m0_2026-05-06T04-14-40Z_c7ae8a2e` | cf-app headless+control-api | 120 | 600 | 5.001 | Same checksum as inline 120 Hz |
| `m0_2026-05-06T04-14-45Z_25e6cb16` | cfctl run --paced | 60 | 300 | 5.003 | F1: same metadata + checksum as cf-app inline |
| `m0_2026-05-06T04-14-50Z_08092095` | cfctl script roundtrip (live WS) | 60 | 6 | server | F1+F4 via auto-launched cf-app |
| `m0_2026-05-06T04-14-03Z_03164834` | cf-app --debug-inject-panic-at-tick 60 | 60 | 120 | 2.003 | F5: real `system.panic` event at tick 61, severity error=1 |

**Test count**: 55 (up from 47 in M0.1; +8 = 5 live WS acceptance tests + 3 scenario.load engine tests + tick_sample test + panic test).

### Fixed (M0.1 — stabilization pass on review findings)

- **H1 — wall-clock pacing in `cf-app --headless-smoke --control-api`**: the previous loop slept `min(remaining, 2 ms)` per outer iteration, which capped per-tick wait at ~2 ms and accelerated the sim to ~345 ticks/s at a configured 60 Hz. Replaced with a `poll_chunk = 5 ms` inner loop that sleeps the FULL remaining `tick_dt` while preserving shutdown responsiveness. New unit test `run_paced_loop_holds_wall_clock_cadence` proves 60 ticks at 60 Hz takes ≥ 0.85 s wall. Acceptance bundles `m0_2026-05-06T03-10-38Z_f657f8d7` (60 Hz, 5.001 s) and `m0_2026-05-06T03-10-50Z_f71a2d1e` (120 Hz, 5.001 s) confirm parity with the inline path.
- **M1 — scenario manifest now drives engine config**: `cf-app::build_config` calls `Scenario::load_from_file` and `M0EngineConfig::for_loaded_scenario(...)`, which pulls `seed`, `duration_ticks`, `expected_tests`, `region.{width,height}` straight from the RON file. CLI flags only override individual fields when explicitly provided. New test `for_loaded_scenario_pulls_seed_and_expected_tests_from_manifest`. Added `region_width` / `region_height` to `M0EngineConfig` and `config_hash_input`.
- **M2 — every bundle now ships a non-null final checksum**: `M0Engine::record_run_finished` calls a new `emit_final_checksum()` AND `M0Engine::write_run_bundle` calls `emit_final_checksum()` before snapshotting events. The first guarantees `record_run_finished()` paths emit a final; the second guarantees mid-run `runbundle.write` requests (which fire BEFORE the run is finalized) ALSO produce bundles with `final_sim_checksum != null` and `checksum_event_count >= 1`. Regression tests `very_short_run_still_has_final_checksum` (1-tick run) and `mid_run_write_run_bundle_has_final_checksum` (live `runbundle.write` path) confirm. All M0.1 acceptance bundles, including the live cfctl roundtrip bundle, report a non-null `final_sim_checksum`.
- **M3 — `runbundle.write` after target_ticks is now honored**: the Bevy app's `check_completion` and `finalize_engine` both call `drain_pending_bundle(...)` so a `runbundle.write` arriving after the tick budget hits is still written. Previously a late-arriving request was silently dropped on the natural-exit path.
- **M5 — `cfctl observe --inline --stream` is now an explicit error**: the combination is mutually exclusive (streaming requires a server, inline runs an in-process snapshot). Pre-fix the request silently fell through to the server path. Manual verification: `cfctl observe --inline --stream` exits 1 with a clear message.
- **L1 — `commit_sha` appends `-dirty` on a dirty tree**: `cf-app::git_commit_sha` now runs `git status --porcelain` after `rev-parse` and suffixes the SHA with `-dirty` when there are uncommitted changes. All M0.1 bundles correctly record `b97c0b1d14b2-dirty`.
- **L2 — `SimClock::step(0)` is a no-op**: pre-fix `step(0)` set `Stepping(0)` and the post-advance `remaining <= 1` check still permitted one tick. Now `step(0)` returns early. New test `step_zero_is_a_no_op`.
- **L3 — removed unused `PrimaryWindow` import + `_windows` query parameter** in `cf-app::esc_or_close_to_exit`.
- **L4 — `cfctl` auto-launch no longer pollutes `prototype_runs/native/` with stray bundles**: only `script run --write-run-bundle` (and other subcommands that need bundle evidence) ask the auto-launched `cf-app` to pass `--write-run-bundle`. `cfctl observe`, `pause`, `step`, etc. now keep the run-bundle root clean.
- **M4 — vault `references/prototype-run-bundle-schema.md` updated** with the canonical `summary.json.performance.tick_rate_hz`, `performance.p99_tick_ms`, `performance.avg_tick_ms`, `performance.wall_seconds`, and the `run_manifest.json.tick_rate_hz` rows under the DR-002 v1 Lock Extensions section. Tightened the wording on `final_sim_checksum` / `checksum_event_count` to record the M0.1 invariant that successful runs MUST have ≥ 1 final checksum.
- **Schema-drift check is now a real CI gate**: `dump_schemas` example accepts `--check` mode that fails if any on-disk schema diverges from the schemars derive output.

### Added (M0 — Engine Bootstrap, correction pass)

- M0 Engine Bootstrap landed under `game/` with **zero deferrals**: 29-crate Cargo workspace, per-crate `AGENTS.md`, fixed-tick `cf-sim-core` (`Xoshiro256**` seeded RNG, `sim_state_v1` blake3 checksum at cadence 60), `cf-replay` run-bundle writer (`prototype-run-manifest.v0.1` / `prototype-recorder-event.v0.1` / `prototype-run-summary.v0.1`), `cf-control` JSON-RPC 2.0 envelope on loopback `127.0.0.1:17890` with the M0 method catalog (`scenario.{load,reset}`, `sim.{pause,resume,step,run_for_ticks}`, `observe.{once,subscribe,unsubscribe,settings}`, `observe.frame` notification, `act.{player.move,settings.set}`, `runbundle.write`, `system.shutdown`), `cfctl` CLI with real WebSocket sessions (auto-launches and shuts down `cf-app --headless-smoke --control-api`), `cf-app` Bevy app shell with title `"Corefall — M0 Engine Bootstrap (v0.0.1)"` + `cf-render-2d::CfRenderPlugin` clear-screen + ESC + `WindowCloseRequested` handlers + `Time::<Fixed>::from_hz` driving `FixedUpdate`, `m0_blank.ron` scenario fixture, GitHub Actions CI matrix (Win/Linux/macOS) with **required** run-bundle validation, `cf_replay::diagnostics::init` shared panic-hook + tracing init, and a real `cf-mod validate content/` walker with non-zero exit on any failure.
- `--tick-rate-hz <u32>` is exposed on both `cf-app` and `cfctl` and plumbed through `M0EngineConfig`. `run_manifest.json.tick_rate_hz`, `summary.json.performance.tick_rate_hz`, and `determinism.sim_checksum` payloads all record the configured tick rate. Tests cover 60 + 120 Hz; acceptance bundles exist for both. No gameplay/control/replay/render code assumes 60.
- `--run-seconds <f32>` is paced against wall-clock at the configured tick rate (`m0_2026-05-06T02-11-45Z_83ca1a85`: 300 ticks / 5.004 wall seconds at 60 Hz; `m0_2026-05-06T02-15-36Z_75e2a2db`: 600 ticks / 5.002 wall seconds at 120 Hz).
- DR-012 settings are LIVE engine state: `act.settings.set` patches `M0Engine`'s `Settings`; `observe.settings`, `run_manifest.json.settings`, and the `settings_observed`/`settings_changed` event payloads all read live state. Live roundtrip captured in `m0_2026-05-06T02-15-08Z_f22a8cea` via `cfctl script run m0_settings_roundtrip` over an auto-launched WebSocket session.
- Static JSON Schemas under `crates/cf-control/schemas/v1/` (18 files) generated by `cargo run -p cf-control --example dump_schemas` and guarded by a `static_schema_files_match_dump` test.
- Run-bundle metadata is real: `commit_sha` from `git rev-parse --short=12 HEAD` (or `CF_COMMIT_SHA`), `rust_version` from `$RUSTC --version`, `bevy_version` from the workspace pin, `config_hash` is blake3-16 over the engine config inputs, `expected_tests` flow from the scenario manifest, and `capabilities` reflect the CLI flags.
- CI now requires the run-bundle checker. The canonical `prototype_run_check.py` is vendored at `game/tools/prototype_run_check.py`. CI fails if no bundles are produced or any bundle fails the checker. The schema-drift check fails CI if `crates/cf-control/schemas/v1/` is out of sync with the schemars derives.
- `scripts/cfctl/m0_settings_roundtrip.cfctl.json` control script: `observe.settings → act.settings.set → observe.settings → act.player.move → sim.step → observe.once → runbundle.write` for the live JSON-RPC roundtrip evidence.
- Project-local Claude Code review skill at `.claude/skills/corefall-review/SKILL.md` for milestone bug hunts. The standing review rules live in that `SKILL.md` entrypoint.
- Repo-local changelog and completion discipline requiring implementation agents to update the canonical vault roadmap/checklist after feature or milestone work.

### Changed (M0 correction pass — supersedes the rejected first pass)

- DR-002 v1 envelope locked under user approval (Open Decision Gates Protocol). Manifest extensions (`checksum.{algorithm,scope,cadence_ticks}` + `settings:{...}`) and summary extensions (`final_sim_checksum`, `checksum_event_count`, `first_tick`, `last_tick`, `performance.tick_rate_hz`, `performance.p99_tick_ms`); `references/prototype-run-bundle-schema.md` updated in the same pass to enumerate them. M0 categories: `system`, `control`, `determinism`. `snapshot` opens at M3.
- DR-012 surface lock applied AND wired to live engine state. The previous "M0 surfaces flags but no behavior" claim was upgraded to actual mutation via `act.settings.set` over JSON-RPC.
- Bumped Rust toolchain pin from `1.84.0` to `1.93.0` (strict, no `stable`, no loose minor). Edited `cortext_command_vault/spec/prototype-roadmap.md` §Toolchain And Workspace Bootstrap in the same pass so the recipe matches the implementation.
- Bevy 0.14 is now a real workspace dependency, not a deferred placeholder. `cf-app` is a real Bevy app, not an inline-only sim runner.
- `jsonrpsee` is intentionally NOT a workspace dep; `tokio-tungstenite` + a minimal hand-rolled JSON-RPC envelope keep the dep tree small. Documented in `cf-control/AGENTS.md`.
- Tightened `AGENTS.md` to reduce repeated evidence/completion text while keeping the short-assignment, validation, vault-update, and handoff rules intact.
- Tightened review acceptance posture: every verified Low/Medium/High/Blocker finding must be fixed before milestone acceptance unless the user explicitly approves deferring that exact finding with recorded owner, reason, next checkpoint, and evidence path.
- Added Contract Integrity Gate to block green-but-wrong milestones: shared production paths, no fake success, mandatory required-field rejection, source-truthful run-bundle evidence, checklist truth, and regression proof for every reviewed bug.
- Moved standing review instructions into the project-local skill entrypoint at `.claude/skills/corefall-review/SKILL.md`. Moved the M0/M0.1 review report to `docs/reviews/2026-05-05-m0-m0-1-review-report.md`.
- Clarified that short milestone prompts such as "Implement M0 from the roadmap" are complete assignments; workers must expand them through `AGENTS.md`, the canonical roadmap, backlog, checklist, and linked DRs without requiring a giant pasted handoff prompt.
- Standardized the planned native workspace directory as `game/` so the corefall repo matches the canonical roadmap's Repository Layout name. No mapping table is needed; `game/` is the workspace root in both the canonical docs and this repo.
- Tightened `AGENTS.md` per a pre-implementation review: added Repository Layout (canonical = this repo), Per-Crate AGENTS.md mandate, Standard Validation block with exact commands, Run-Bundle Naming, Git Hygiene, Secrets Posture, and a Do-Not list. Added vault home to the mandatory read order. Pinned `cfctl` invocation path and run-bundle root.

### Fixed (correction pass)

- The first M0 pass was rejected because nine items were either deferred, faked, or evidence-incorrect. Every one is now PASS:
  1. Real Bevy app shell (M0-002): window + title/version + ESC + `cf-render-2d` clear-screen + `FixedUpdate`-driven sim. `--headless-smoke` is a flag on top of the same engine, not the only path.
  2. `--run-seconds` pacing: 300 ticks at 60 Hz now genuinely takes ~5 wall seconds; perf samples ship real `avg_tick_ms` / `p99_tick_ms` / `wall_seconds`.
  3. DR-012 propagation: settings are mutated by `act.settings.set` and re-observed in the next `observe.settings` and `run_manifest.json.settings`.
  4. cfctl is server-driven for `scenario`/`pause`/`step`/`act`/`script`/`observe --stream` with auto-launch + structured failure on connect-refused. No fake-success stubs remain.
  5. Static schemas are checked into `crates/cf-control/schemas/v1/` and CI fails on drift.
  6. `cf-mod validate content/` is a real RON scenario validator; verified to fail on broken RON.
  7. Run-bundle evidence is factual: corrected the false same-checksum claim from the first pass; the 60 Hz/300-tick same-seed runs DO share `e500280653...0e6e`, but the live `--control-api` and the 120 Hz/600-tick paths produce different checksums (now in the bundle table).
  8. Run-bundle metadata: `commit_sha`, `rust_version`, `bevy_version`, `platform`, `config_hash`, `expected_tests`, and `capabilities` are populated from real sources, not stubs.
  9. CI is a hard gate: required run-bundle checker on three bundles, required schema-drift check, required release build, required `cf-mod validate content/`. No `|| echo skipped`.
- Architectural fix per user direction (no compromised defaults): `--tick-rate-hz` is exposed on every binary, recorded in every bundle, asserted in tests at 60 + 120 Hz, and the M0 engine has no hardcoded `60` outside named defaults.
- Fixed real bug found by acceptance testing: `cf-app --headless-smoke --control-api` previously fell into `run_headless` which never started the JSON-RPC server, so `cfctl` auto-launch got `Connection refused`. New `run_headless_server` path starts the server, ticks the sim until shutdown OR target_ticks, and writes the run bundle on exit.

## 2026-05-05

### Added

- Created the private `Madreag/corefall` implementation repository.
- Added root implementation instructions, a future native workspace under `game/`, and `docs/implementation-log/` for milestone evidence.

### Changed

- Slimmed the repo to use the canonical vault directly instead of maintaining a duplicated planning snapshot.
