# cf-control — AGENTS.md

## Owns
- JSON-RPC 2.0 envelope types (`JsonRpcRequest`/`JsonRpcResponse`/`JsonRpcNotification`/`JsonRpcError`).
- Method catalog for M0: `scenario.load`, `scenario.reset`, `sim.pause`, `sim.resume`, `sim.step`, `sim.run_for_ticks`, `observe.once`, `observe.subscribe`, `observe.unsubscribe`, `observe.frame` (notification), `observe.settings`, `act.player.move`, `runbundle.write`, `system.shutdown`.
- Local control server (`ControlServer`) on `127.0.0.1:17890` (loopback only).
- DR-012 lock: `Settings` resource (six accessibility flags) + observability via `observe.settings`.
- Inline `M0Engine` + `run_m0_inline` driver shared by `cf-app` and `cfctl`.
- Scenario loader for the M0 minimal manifest shape.

## Public API Boundary
- Types: `Settings`, `ObserveFrame`, `ObserveSettings`, `EngineState`, `ControlEnvelopeStatus`, `RunStatus`, `Scenario`, `ScenarioLoadError`, `M0Engine`, `M0EngineConfig`, `M0EngineOutcome`.
- Functions: `run_m0_inline`.
- Server: `ControlServer`, `ControlServerConfig`, `EngineHandle`, `ControlCommand`, `CommandResult`.
- Constant: `SCHEMA_VERSION = 1`.

## Does NOT Own
- Render/UI → `cf-render-2d`/`cf-ui`.
- Sim core / RNG → `cf-sim-core`.
- Run-bundle envelope / event taxonomy → `cf-replay`.
- Network transport for multiplayer → `cf-net` (decision deferred to M9).

## Test Surface
- Unit tests: `cargo test -p cf-control`.
- Schema mismatch returns `-32602` with fix-hint.
- Unknown method returns `-32601`.
- `observe.once` returns a frame matching `SCHEMA_VERSION`.

## Cross-Crate Contracts
- Depends on: `cf-sim-core`, `cf-replay`.
- Depended on by: `cf-app`, `cfctl` (and later `cf-server`, `cf-e2e`, `cf-tools-editor`).
- Events emitted (via injected `Recorder`): `control.command_accepted`, `control.command_rejected`, `control.observation_sent`, `control.settings_observed`.
- Events consumed: none directly; the engine reads sim/scenario state.

## Common Pitfalls
- Every request param object MUST carry `schema_version: 1`. Mismatches return `-32602`.
- The control server binds to loopback by default. Remote bind must require an auth token (see DR-005 / DR-013) — NOT in M0.
- The `EngineHandle` trait is async. Adding sync calls inside it will deadlock the WebSocket runtime.

## Source Trail
- spec/ai-control-observability-layer.
- spec/prototype-roadmap §Control Transport And Envelope.
- DR-012 (accessibility floor; OPEN — closes at M4).
- docs/implementation-log/2026-05-05-m0-engine-bootstrap.md §DR-012 floor lock.
