# Corefall Review — M0 (M0.4 F7-only follow-up)

Scope: M0 — Engine Bootstrap, post M0.4 path-safety follow-up.
Reviewed range / milestone: working tree on `main` (`b97c0b1d14b2-dirty`); covers M0 first pass + M0 correction + M0.1 + M0.2 + M0.3 + M0.4.
Reviewer: AI (Droid)
Date: 5/5/2026 10:31 PM MST

Review source: `.claude/skills/corefall-review/SKILL.md`.

## Findings

### Blocker

- None.

### High

- None. (M0.3 closed F7-F9; M0.4 added the regression test + CI gate the reviewer requested.)

### Medium

- None.

### Low

- None.

## Spec Contract Status

| Contract | Source | Evidence | Status | Gap |
|---|---|---|---|---|
| **M0.4-F7** `cf-replay` unit test for bundle path resolution | independent-review recommendation | 5 new tests in `crates/cf-replay/src/bundle_paths.rs::tests`: `default_root_resolves_above_game_when_cwd_is_game` (cwd=game/ regression), `default_root_uses_cwd_when_cwd_is_repo_root`, `default_root_walks_up_from_nested_cwd`, `resolve_run_bundle_root_returns_explicit_unchanged`, `resolve_run_bundle_root_falls_back_to_default_when_none`. Resolver moved to `cf-replay::bundle_paths` (natural owner); `cf-control::runtime` re-exports for backwards compatibility. | PASS | None |
| **M0.4-F7** CI assertion: no `prototype_runs/` outside repo root | independent-review recommendation | `.github/workflows/ci.yml` step "enforce repo-root prototype_runs path (M0.4-F7)" runs `find` after the release build + acceptance bundles and fails CI on any stray directory. Verified locally: zero stray dirs after `cf-app --headless-smoke ... --write-run-bundle` (bundle landed at absolute `/Users/erol/projects/corefall/prototype_runs/native/m0_..._8466a407`). | PASS | None |
| **M3-006** `system.run_finished` checker tightening + `expected_outcome` enum | independent-review recommendation (capture only) | New task card added to `docs/plan/spec/native-implementation-backlog.md` §M3. Owns: `cf-replay`, `references/prototype-run-bundle-schema.md`, `tools/prototype_run_check.py`. Adds `expected_outcome` enum (`clean | panic | abort`) + tighter checker rules. **Intentionally NOT implemented in M0.4** — M3 closes DR-002 and is where the contract belongs. | DEFERRED to M3 (per reviewer guidance) | None — captured in roadmap-authoritative backlog |
| All M0.3 contracts (M0-001..008, M0-D01..D08, DR-002 v1, DR-012, eyes/ears/hands, no-compromise perf) | M0.3 review report | Unchanged from M0.3 verdict; M0.4 is additive (path-safety regression coverage, no behavioral changes to engine/server/cfctl). | PASS | None |

## Validation

| Command | Result | Notes |
|---|---|---|
| `cargo fmt --all -- --check` | PASS | Clean. |
| `cargo check --workspace --all-targets` | PASS | All crates compile. |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS | Zero warnings. |
| `cargo test --workspace` | PASS | **73 tests passing** (up from 68 in M0.3; +5 new `cf-replay::bundle_paths` tests). |
| `cargo build --release` | PASS | macOS aarch64 release build green. |
| `cargo run -p cf-mod -- validate content/` | PASS | `scanned=1 pass=1 warn=0 fail=0`. |
| `cargo run -p cf-control --example dump_schemas -- --check` | PASS | `schema check OK (18 schemas)`. |
| `cf-app --headless-smoke --tick-rate-hz 60 --ticks 300 --write-run-bundle` (no `--run-bundle-dir`) | PASS | 5.004 s wall, bundle landed at absolute `/Users/erol/projects/corefall/prototype_runs/native/m0_..._8466a407`. |
| `python3 game/tools/prototype_run_check.py prototype_runs/native/m0_..._8466a407` | PASS | `errors 0`. |
| F7 stray-dir scan: `find . -type d -name prototype_runs -not -path './prototype_runs' -not -path './prototype_runs/*' -not -path './target/*' -not -path './.git/*'` | PASS | Empty result. |

## Test Gaps And Missing Evidence

- **None for M0 scope.** M0.4 added 5 regression tests directly addressing the reviewer's recommendation. The M3-006 task card captures the deeper checker tightening for the milestone where DR-002 closes.

## Roadmap / Checklist / Changelog / Vault Updates

- `corefall/CHANGELOG.md` — `Unreleased > Fixed (M0.4 — F7 path-safety follow-up)` section landed with the bullet list, test-count delta, and acceptance bundle.
- `corefall/docs/implementation-log/2026-05-05-m0-engine-bootstrap.md` — M0.4 section appended with findings table, architectural diagram (cf-replay owns the resolver), test count delta (68 → 73), ID-by-ID matrix, Standard Validation table.
- `docs/plan/spec/native-implementation-backlog.md` — new `M3-006 run-finished outcome contract` task card under §M3.
- `corefall/docs/reviews/2026-05-06-m0-m0-4-review-report.md` — this report.
- `docs/plan/spec/feature-completion-checklist.md` — no row changes required; M0 rows already at `[x]` from M0.3, and the M0.4 regression test + CI gate strengthen evidence under existing rows without changing scope.

## Verdict

**Accept.**

Zero unresolved verified findings at any severity. M0.4 landed exactly the recommended F7 follow-up — a `cf-replay` unit test + a CI gate — without touching the M0.3 contracts. The deeper `system.run_finished` checker tightening was captured as M3-006 in the canonical backlog and is correctly deferred to M3 (which closes DR-002 and owns the run-bundle outcome contract).

Both repos are commit-ready. Awaiting the user's commit instruction per `corefall/AGENTS.md`.
