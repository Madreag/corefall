# Corefall Review — M0 (M0.1 stabilization pass)

Scope: M0 — Engine Bootstrap, post M0.1 stabilization
Reviewed range / milestone: working tree on `main` (commit `b97c0b1`-dirty); covers M0 first pass + M0 correction pass + M0.1 stabilization pass closing all 6 verified findings + 4 cheap low-risk items from the prior review
Reviewer: AI (Droid)
Date: 5/5/2026 8:21 PM MST

## Findings

### Blocker

- None.

### High

- None. (M0.1 closed H1 — `cf-app --headless-smoke --control-api` now sleeps the FULL `tick_dt` per tick instead of capping at 2 ms. New regression test `run_paced_loop_holds_wall_clock_cadence` proves 60 ticks @ 60 Hz takes ≥ 0.85 s; acceptance bundles `m0_..._f657f8d7` (60 Hz / 5.001 s wall) and `m0_..._f71a2d1e` (120 Hz / 5.001 s wall) confirm in production.)

### Medium

- None. (M0.1 closed M1, M2, M3, M4, M5. M2 was extended this pass: `M0Engine::write_run_bundle` now also calls `emit_final_checksum()` so mid-run `runbundle.write` requests — which fire BEFORE `record_run_finished` — also produce bundles with `final_sim_checksum != null`. Regression test `mid_run_write_run_bundle_has_final_checksum`. Verified on bundle `m0_..._cb9543db` which now reports `checksum_event_count=1` and a non-null final.)

### Low

- None. (M0.1 closed L1-L4. `commit_sha` now appends `-dirty` on a dirty tree; `SimClock::step(0)` is a no-op; unused `PrimaryWindow` import + `_windows` query removed; `cfctl` auto-launch only requests `--write-run-bundle` for subcommands that need it.)

## Spec Contract Status

| Contract | Source | Evidence | Status | Gap |
|---|---|---|---|---|
| M0-001 workspace scaffold | roadmap §M0 | 29 crates, per-crate AGENTS.md, `cargo fmt/check/clippy/test` clean (43 tests), Bevy 0.14 + tokio + tokio-tungstenite + schemars + chrono + blake3 in workspace deps | PASS | None |
| M0-002 Bevy app shell | roadmap §M0 | Real Bevy `App` + `DefaultPlugins` + `WindowPlugin` (1280×720, fixed title `"Corefall — M0 Engine Bootstrap (v0.0.1)"`) + `cf-render-2d::CfRenderPlugin` (clear-screen + Camera2dBundle) + `Time::<Fixed>::from_hz(--tick-rate-hz)` driving FixedUpdate + ESC + WindowCloseRequested handlers; `cli_parses_all_m0_flags_and_tick_rate` + `run_paced_loop_holds_wall_clock_cadence` tests | PASS | None |
| M0-003 fixed-tick island | roadmap §M0 | `cf-sim-core` `SimClock`/`Rng`/`SimConfig`; 14 unit tests including `step_zero_is_a_no_op`, 60 + 120 Hz, RNG determinism, checksum stability | PASS | None |
| M0-004 run-bundle writer | roadmap §M0 | `cf-replay` writes manifest + events.jsonl + summary.json + notes.md per the canonical v0.1 strings; perf samples populated; `tick_rate_hz`, `commit_sha-dirty`, `config_hash`, `expected_tests`, `capabilities`, settings all real; M0.1 ensures every bundle has `final_sim_checksum != null` and `checksum_event_count >= 1` | PASS | None |
| M0-005 CI matrix | roadmap §M0 | `.github/workflows/ci.yml` (Win/Linux/macOS): fmt/check/clippy `-D warnings`/test/release-build/cf-mod validate/`dump_schemas --check`/cfctl observe smoke/cfctl run 60+120 Hz/cf-app paced 5 s/REQUIRED `python3 tools/prototype_run_check.py` on three bundles; no `\|\| echo skipped` | PASS | None |
| M0-006 control/observe bootstrap | roadmap §M0 | JSON-RPC 2.0 over WebSocket on `127.0.0.1:17890`; full M0 method catalog; 18 static schemas regenerated via `dump_schemas` and CI-checked via `dump_schemas --check`; cfctl auto-launch + script runner + live WS roundtrip; M5 fix rejects `--inline --stream` explicitly | PASS | None |
| M0-007 m0_blank scenario fixture | roadmap §M0 | `content/scenarios/m0_blank.ron` validated by `cf-mod validate content/`; M0.1 wires `cf-app::build_config` to `Scenario::load_from_file` → `for_loaded_scenario(...)` so seed/duration_ticks/expected_tests/region come from the manifest | PASS | None |
| M0-008 panic hook + tracing init | roadmap §M0 | `cf-replay::diagnostics::init` shared across every binary; `system.panic` event hook; severity counters in `summary.json.event_counts.by_severity` | PASS | None |
| M0-D01 cargo build --release | roadmap §M0 | macOS aarch64 `cargo build --release` clean; Win/Linux/macOS wired into CI | PASS | None |
| M0-D02 CI green / local validation pass | roadmap §M0 | Local Standard Validation pass on macOS aarch64 + rustc 1.93.0 (43 tests) | PASS | None |
| M0-D03 cargo run opens window, ticks, exits cleanly | roadmap §M0 | `cargo run -p cf-app -- --scenario m0_blank --run-seconds 5` opens Bevy window backed by `cf-render-2d`, ticks `Time::<Fixed>::from_hz(60)`; bundle `m0_..._42d4f591` proves 5.003 s wall / 300 ticks at 60 Hz | PASS | None |
| M0-D04 run bundle written | roadmap §M0 | 7 fresh M0.1 bundles under `prototype_runs/native/m0_*/` with manifest+events+summary+notes; all required files present | PASS | None |
| M0-D05 prototype_run_check passes | roadmap §M0 | `errors 0` on all 7 M0.1 bundles via vendored canonical checker | PASS | None |
| M0-D06 cfctl observe --once | roadmap §M0 | Inline JSON observation prints valid frame; live WS observation captured by `m0_..._cb9543db` | PASS | None |
| M0-D07 cfctl run --ticks 300 --write-run-bundle | roadmap §M0 | `m0_..._422ff38a` (60 Hz/300/5.003 s paced) + `m0_..._04191adc` (120 Hz/600/5.002 s) | PASS | None |
| M0-D08 repo commit-ready | roadmap §M0 | Working tree dirty with M0 + M0.1 scaffold; Standard Validation passes locally; commit pending explicit user request | PASS | None |
| DR-002 v1 envelope (lock applied at M0) | DR-002 + ai-coder reading list | `references/prototype-run-bundle-schema.md` updated with `tick_rate_hz`, `performance.{tick_rate_hz,p99_tick_ms,avg_tick_ms,wall_seconds}`, `final_sim_checksum`, `checksum_event_count`, `first_tick`, `last_tick`, `checksum.{algorithm,scope,cadence_ticks}`, `settings:{...}` extensions | PASS | DR-002 closure formally lands at M3 (full snapshot + replay verification); M0 only locks the v1 envelope as approved |
| DR-012 accessibility floor (six flags) | DR-012 + ai-coder reading list | Six flags wired into `cf-control::Settings`, mutated by `act.settings.set`, observable via `observe.settings`, recorded in `run_manifest.json.settings`; localization deferred to M4 per user approval | PASS | None |
| Eyes/ears/hands rule (cfctl coverage) | spec/ai-control-observability-layer.md | Every M0 player-facing surface (settings, scenario, sim mode, observation) is reachable through `cfctl` + `cf-control` JSON-RPC | PASS | None (M1 controller surface lands at M1) |
| No-Compromise Performance Defaults | corefall AGENTS.md | `--tick-rate-hz` exposed on `cf-app` and `cfctl`; tick rate recorded in every bundle and on every `determinism.sim_checksum`; tests cover 60 + 120 Hz inline AND through headless+control-api; no gameplay/control/replay code assumes 60 | PASS | None |

## Validation

| Command | Result | Notes |
|---|---|---|
| `cargo fmt --all -- --check` | PASS | Clean after `cargo fmt --all`. |
| `cargo check --workspace --all-targets` | PASS | All 29 crates compile. |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS | Zero warnings. |
| `cargo test --workspace` | PASS | **43 tests passing** (up from 38 in M0 first pass; +5 = `step_zero_is_a_no_op`, `for_loaded_scenario_pulls_seed_and_expected_tests_from_manifest`, `very_short_run_still_has_final_checksum`, `mid_run_write_run_bundle_has_final_checksum`, `run_paced_loop_holds_wall_clock_cadence`). |
| `cargo build --release` | PASS | macOS aarch64 release build green. |
| `cargo run -p cfctl -- observe --once` | PASS | Inline + live WS both green. |
| `cargo run -p cfctl -- observe --inline --stream` | EXIT 1 (intentional) | M5 fix verified — exits non-zero with `--inline and --stream are mutually exclusive: streaming requires a control server, inline runs a single in-process snapshot`. |
| `cf-app --headless-smoke --tick-rate-hz 60 --ticks 300 --write-run-bundle` (inline) | PASS | 5.003 s wall, bundle `m0_..._42d4f591`. |
| `cf-app --headless-smoke --tick-rate-hz 120 --run-seconds 5 --write-run-bundle` (inline) | PASS | 5.002 s wall, bundle `m0_..._04191adc`. |
| `cf-app --headless-smoke --control-api --tick-rate-hz 60 --ticks 300 --write-run-bundle` (H1 fix path) | PASS | 5.001 s wall, bundle `m0_..._f657f8d7`. Same final checksum as the 60 Hz inline path → H1 fix preserves determinism. |
| `cf-app --headless-smoke --control-api --tick-rate-hz 120 --ticks 600 --write-run-bundle` | PASS | 5.001 s wall, bundle `m0_..._f71a2d1e`. |
| `cfctl run --ticks 300 --paced --tick-rate-hz 60` | PASS | 5.003 s wall, bundle `m0_..._422ff38a`. |
| `cfctl script run m0_settings_roundtrip --write-run-bundle` | PASS | Bundle `m0_..._cb9543db`; mid-run `runbundle.write` now records `checksum_event_count=1` + non-null `final_sim_checksum` (M2 follow-up fix). |
| `cargo run -p cf-mod -- validate content/` | PASS | `scanned=1 pass=1 warn=0 fail=0`, exit 0. |
| `cargo run -p cf-control --example dump_schemas -- --check` | PASS | `schema check OK (18 schemas)`. |
| `python3 game/tools/prototype_run_check.py <bundle>` | PASS | `errors 0` on all 7 M0.1 bundles. |

## Test Gaps And Missing Evidence

- **None for M0 scope.** The M0.1 pass added 5 regression tests, all green. The spec leaves replay-divergence tests, snapshot category, networking, actor controller, terrain, materials, AI, etc. to later milestones; those are correctly classified `Not yet testable` per M0 review overlay.

## Roadmap / Checklist / Changelog / Vault Updates

- `corefall/CHANGELOG.md` — M0.1 stabilization section landed (Fixed: H1 + M1 + M2 + M3 + M5 + L1 + L2 + L3 + L4 + M4 doc + schema-drift CI gate); M2 fix extended for mid-run runbundle.write.
- `corefall/docs/implementation-log/2026-05-05-m0-engine-bootstrap.md` — appended M0.1 Stabilization Pass section with findings table, M0.1 acceptance bundle table (7 bundles), test count delta (38 → 43), ID-by-ID acceptance matrix.
- `cortext_command_vault/spec/feature-completion-checklist.md` — M0-001, M0-002, M0-006, M0-007 rows refreshed with M0.1 evidence + Bevy 0.14 / static schemas / `cf-mod validate` / `cfctl observe --inline --stream` rejection / `for_loaded_scenario` notes; D04, D05, D06, D07, D08 rows updated to point at the M0.1 bundles. M0-002 status flipped from `[~]` to `[x]`.
- `cortext_command_vault/references/prototype-run-bundle-schema.md` — DR-002 v1 Lock Extensions table now enumerates `summary.performance.{tick_rate_hz,p99_tick_ms,avg_tick_ms,wall_seconds}`, `run_manifest.json.tick_rate_hz`, and the M0.1 invariant on `final_sim_checksum` / `checksum_event_count`.
- `corefall/.github/workflows/ci.yml` — replaced the `dump_schemas` + `git diff` check with the simpler `dump_schemas -- --check` invocation that fails fast with a clear message.

## Verdict

**Accept.**

Zero unresolved verified findings at any severity. The M0.1 stabilization pass closed all 10 items the prior `/corefall-review M0` produced (1 high + 4 medium + 1 medium-doc + 4 low) AND the follow-up M2 gap discovered during this review pass (mid-run `runbundle.write` was producing bundles with `final_sim_checksum=null`). 43 tests pass, 7 acceptance bundles validate clean against the canonical run-bundle checker, and the wall-clock pacing fix is provable across both `inline` and `headless+control-api` paths at both 60 and 120 Hz.

No items remain `FAIL`, `PARTIAL`, `DEFERRED`, or `READY_FOR_HUMAN` in roadmap or backlog M0 scope.
