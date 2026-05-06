# cf-app — AGENTS.md

## Owns
- The `cf-app` binary entry point.
- Three run modes: Bevy app shell (default), `--headless-smoke` (sim only), `--headless-smoke --control-api` (sim + JSON-RPC server, no window).
- All M0 CLI flags: `--scenario`, `--seed`, `--run-seconds`, `--ticks`, `--tick-rate-hz`, `--write-run-bundle`, `--run-bundle-dir`, `--control-api`, `--control-port`, `--control-uds`, `--headless-smoke`, `--debug-capabilities`, `--ui-scale`, `--high-contrast`, `--captions on|off`, `--reduced-motion`, `--reduced-shake`, `--reduced-flash`.
- Wiring `cf-control::M0Engine` to either `run_m0_inline`, the headless+server loop, or the Bevy `FixedUpdate` schedule at the configured `--tick-rate-hz`.
- `cf-render-2d::CfRenderPlugin` integration (clear-screen + 2D camera).
- ESC + `WindowCloseRequested` → `AppExit::Success`.
- Real metadata for run bundles: `git rev-parse --short=12 HEAD` (or `CF_COMMIT_SHA`), `$RUSTC --version`, the Bevy workspace version pin, blake3-16 over the engine config inputs.
- Scenario file location resolution.
- `tracing` + panic-hook initialization via `cf-replay::diagnostics::init("cf::app")`.

## Public API Boundary
- A binary; no library API.

## Does NOT Own
- The fixed-tick scheduler → `cf-sim-core`.
- Run-bundle writer → `cf-replay`.
- Control server / envelope → `cf-control`.
- Render pipelines → `cf-render-2d` (M2 chunked terrain).
- HUD / UI → `cf-ui` (M1/M4).

## Test Surface
- `cli_parses_all_m0_flags_and_tick_rate` covers every M0 flag including `--tick-rate-hz 120`.
- `duration_is_ticks_first_then_seconds` covers `compute_duration` for ticks/run-seconds at 60 + 120 Hz.
- E2E:
  - `cargo run -p cf-app -- --scenario m0_blank --headless-smoke --run-seconds 5 --write-run-bundle`
  - `cargo run -p cf-app -- --scenario m0_blank --headless-smoke --control-api --ticks 0` (server stays up until shutdown)
  - `cargo run -p cf-app -- --scenario m0_blank --tick-rate-hz 120 --run-seconds 2 --write-run-bundle` (Bevy + 120 Hz)

## Cross-Crate Contracts
- Depends on: `cf-sim-core`, `cf-replay`, `cf-control`, `cf-render-2d`, `bevy`.
- Emits run bundles under `--run-bundle-dir` (default `prototype_runs/native/`).
- Boots the `cf-control` JSON-RPC server on `127.0.0.1:<--control-port>` when `--control-api` is set.

## Common Pitfalls
- The combination `--headless-smoke --control-api` runs `run_headless_server`, which DOES start the JSON-RPC server and ticks until either `system.shutdown` or `--ticks`/`--run-seconds` budget. `--ticks 0` with `--control-api` means "run until shutdown".
- `--control-uds` is reserved (POSIX UDS); not implemented in M0.
- `--debug-capabilities` only records the requested flags into `run_manifest.json.capabilities.debug_capabilities`; debug actions themselves are gated to later milestones.
- Do not hardcode 60 Hz anywhere; use `config.tick_rate_hz`. M0's tests cover 60 + 120.

## Source Trail
- spec/prototype-roadmap §M0 — Engine Bootstrap, §CLI Reference.
- spec/native-implementation-backlog M0-002, M0-006.
- docs/implementation-log/2026-05-05-m0-engine-bootstrap.md.
