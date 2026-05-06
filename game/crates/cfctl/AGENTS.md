# cfctl — AGENTS.md

## Owns
- The `cfctl` CLI binary used by AI agents, dev scripts, and future bots.
- Two modes:
  - **Inline** for `run` and `observe --once`/`--settings` without `--connect`. Runs `M0Engine` directly.
  - **Server-driven** for `scenario`, `pause`, `step`, `act`, `observe --stream`, and `script run`. Connects to a running `cf-app --control-api` server, or auto-launches `cf-app --headless-smoke --control-api --control-port <auto-launch-port>` and shuts it down via `system.shutdown` on close. Use `--connect <addr>` to talk to an existing server, or `--no-auto-launch` to refuse implicit spawns.
- Output is structured JSON by default so AI agents can parse it. `--format pretty` switches to indented JSON.
- Every server-driven request injects `schema_version: 1` automatically.

## Public API Boundary
- A binary; no library API.
- Output schema: every command prints a JSON object with `schema_version`, `status` (when applicable), and the relevant payload.

## Does NOT Own
- The control server itself → `cf-control`.
- The sim loop → `cf-sim-core` + `cf-control::engine`.
- Mid-process orchestration (multiple servers / shard balancing) → later milestones.

## Test Surface
- Unit tests: `cargo test -p cfctl`.
- E2E:
  - `cargo run -p cfctl -- observe --once`
  - `cargo run -p cfctl -- run --scenario m0_blank --ticks 300 --tick-rate-hz 60 --write-run-bundle`
  - `cargo run -p cfctl -- run --scenario m0_blank --ticks 600 --tick-rate-hz 120 --paced --write-run-bundle`
  - `cargo run -p cfctl -- script run m0_settings_roundtrip --write-run-bundle`
  - `cargo run -p cfctl -- act settings-set --ui-scale 2.0 --high-contrast true`

## Cross-Crate Contracts
- Depends on: `cf-control` (engine + envelope + settings), `cf-replay` (diagnostics), `cf-sim-core` (types), `tokio-tungstenite`, `tokio` (with `process` feature).
- Auto-launch resolves the `cf-app` binary via `CF_APP_BIN` env, then `current_exe.parent`/`grandparent` lookup, then `cargo build -p cf-app --message-format=json` as a fallback.

## Common Pitfalls
- `--stream` requires a server; in M0 there is no inline-stream fallback. Without `--no-auto-launch` cfctl will auto-spawn `cf-app --headless-smoke --control-api`.
- The auto-launched server uses `--ticks 0` so it stays up until `system.shutdown`. cfctl always sends `system.shutdown` on `Session::close()`.
- `act settings-set` accepts each flag as `Option<f32|bool>`; only fields supplied on the CLI are sent in the patch.

## Source Trail
- spec/prototype-roadmap §CLI Reference.
- spec/ai-control-observability-layer.
- docs/implementation-log/2026-05-05-m0-engine-bootstrap.md.
