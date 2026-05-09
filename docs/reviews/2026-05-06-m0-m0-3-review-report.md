# Corefall Review — M0 (M0.3 contract-integrity loop)

Scope: M0 — Engine Bootstrap, current working tree on `main` (`b97c0b1d14b2-dirty`).

Review source: `.claude/skills/corefall-review/SKILL.md`.

## Findings

None unresolved.

The M0.3 loop found and fixed three verified in-scope findings:

| ID | Severity | Root cause | Status |
|---|---|---|---|
| M0.3-F7 | High | No single strict JSON-RPC validation boundary; unknown/unsupported params could reach dispatch or be accepted as no-ops. | Fixed with strict serde params, pre-dispatch rejection, engine rejection, and live WS negative tests. |
| M0.3-F8 | High | `cf-app` conflated mid-run bundle snapshots with final `--write-run-bundle` exit evidence. | Fixed; final control-script bundle overwrites mid-run snapshot and includes `system.run_finished`. |
| M0.3-F9 | High | Default bundle roots were cwd-relative, so standard validation from `game/` could write to `game/prototype_runs`. | Fixed with shared repo-root resolver used by `cf-app`, `cfctl`, and cfctl auto-launch. |

## Spec Contract Status

| Contract | Source | Evidence | Status | Gap |
|---|---|---|---|---|
| M0 roadmap done criteria M0-D01..D08 | `prototype-roadmap.md` | final validation commands + bundles `m0_2026-05-06T04-46-04Z_1ad62cb4`, `m0_2026-05-06T04-46-14Z_2c7f5b05`, `m0_2026-05-06T04-46-27Z_a9675fc6`, `m0_2026-05-06T04-46-37Z_56e26f4b` | PASS | None |
| Backlog task cards M0-001..M0-008 | `native-implementation-backlog.md` | workspace, Bevy shell, fixed tick, run bundles, CI, control API, scenario fixture, panic/tracing tests | PASS | None |
| `cf-control` JSON-RPC mandatory schema and no fake success | M0-006 + contract gate | 36 `cf-control` unit tests + 9 live WS tests; unsupported fields/zero ticks/unsupported actor move/id override reject | PASS | None |
| Run-bundle source truth | M0-004 + M0 done criteria | all final M0.3 bundles contain `system.run_finished` and final checksum; checker errors 0 | PASS | None |
| Repo-root evidence path | Corefall AGENTS run-bundle contract | final bundles under `/Users/erol/projects/corefall/prototype_runs/native`; `game/prototype_runs` absent | PASS | None |

## Validation Status

| Command | Status |
|---|---|
| `cargo fmt --all --check` | PASS |
| `cargo check --workspace --all-targets` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `cargo test --workspace` | PASS: 68 tests + doctests |
| `cargo build --release` | PASS |
| `cargo run -p cf-control --example dump_schemas -- --check` | PASS |
| `cargo run -p cf-mod -- validate content/` | PASS |
| `cargo run -p cfctl -- observe --once` | PASS |
| Canonical checker on the four final M0.3 bundles | PASS: errors 0 |

## Contract Integrity Matrix

| Contract path | Shared source of truth | Positive proof | Negative/adversarial proof | Checklist truth |
|---|---|---|---|---|
| `cf-app` direct bundle | `cf_control::runtime::build_engine_config`, `M0Engine`, `cf_replay::write_run_bundle` | `m0_2026-05-06T04-46-27Z_a9675fc6` | final-write regression and checker | M0-D04/D05 updated |
| `cfctl run` bundle | shared runtime config + run-bundle-root resolver | `m0_2026-05-06T04-46-04Z_1ad62cb4`, `m0_2026-05-06T04-46-14Z_2c7f5b05` | repo-root default test, no `game/prototype_runs` | M0-D07 updated |
| JSON-RPC live control | strict server schemas + `M0Engine::dispatch` | `m0_2026-05-06T04-46-37Z_56e26f4b` | unknown field, missing schema, zero tick, unsupported movement, unsupported id override tests | M0-S06/M0-006 updated |
| Final bundle evidence after mid-run write | `record_run_finished` + `write_run_bundle` | script bundle includes `system.run_finished` | `final_write_replaces_mid_run_bundle_with_run_finished_evidence` | no hidden M0 deferrals |

## Test Gaps

None for M0 scope. Actor movement remains intentionally rejected until M1 owns actor/control-intent semantics.

## Updates Required

Completed in this pass:

- `CHANGELOG.md` M0.3 section.
- `docs/implementation-log/2026-05-05-m0-engine-bootstrap.md` M0.3 section.
- `docs/plan/spec/feature-completion-checklist.md` M0 rows refreshed to M0.3 evidence.
- `docs/plan/spec/prototype-roadmap.md` M0 done criteria checked with current evidence note.

## Verdict

**Accept.**

Zero unresolved verified M0 findings remain. The review loop reached halt criterion (a): Accept verdict with zero findings on the assigned M0 scope.
