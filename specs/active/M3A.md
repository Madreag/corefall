# M3A — Event Recorder Core

## Status

`active`

## Intent

The event taxonomy, snapshot writer, and headless replay verifier are complete enough that any prior milestone's run can be replayed off-line and produce identical state checksums per tick. Determinism is real, drift is detected, and the run bundle envelope is the source of truth for everything an offline reviewer needs.

## Player-facing behavior

- (M3A is infrastructure, not directly player-facing.) The player's actions emit events into a deterministic stream that can be re-played later to debug, audit, or grade a run.
- `cf-headless replay <bundle>` replays a run and prints `result=ok` (matching) or `result=divergence` with `first_divergence` (mismatch).
- `cfctl observe --once` returns a snapshot of current sim state.
- The run bundle directory contains everything needed to grade or debug the run: manifest, events, summary, snapshots, checksums, captures, expected_outcome.

## Crates / modules touched

| Crate | Status | What changes |
|---|---|---|
| `cf-replay` | MODIFY | Event taxonomy expanded to all 27 baseline categories: `input`, `control`, `mind`, `collision`, `server`, `anti_cheat`, `mmo`, `material`, `reaction`, `atmospherics`, `affliction`, `combat`, `body`, `terrain`, `ai`, `logistics`, `mission`, `system`, `snapshot`, `determinism`, `ux`, `accessibility`, `performance`, `equipment`, `chassis`, `actor`, `ability`. `Recorder::with_capacity`, `dropped_count()`, `event_count()`. Non-blocking recorder path with backpressure. |
| `cf-control` | MODIFY | `M0EngineConfig.checksum_cadence_ticks` field (configurable per-scenario, default 60). `system.run_started` event includes `protocol_version`. `system.category_baseline` event lists all 27 categories at run start with active/registered status. Snapshot writer fires actor + inventory + terrain snapshots at scene start AND on every objective state change (started/completed/failed). `runbundle.write` rejects path traversal. |
| `cf-headless` | MODIFY | Replay verifier emits structured `first_divergence` (tick, recorded, live) JSON output and `all_divergences` array. Non-zero exit on divergence. `--no-verify-checksums` flag. `--scenario-path` override. |
| `cf-app` | MODIFY | `--checksum-cadence-ticks <N>` CLI flag wired through ConfigInputs. `--tick-rate-hz <N>` already exists; verify M3A bundles record the configured tick rate. `expected_outcome` field on `runbundle.write` (clean/panic/abort). |
| Run-bundle checker (Python) | MODIFY | `prototype_run_check.py` validates `expected_outcome` against `system.run_finished.outcome`. Rejects bundles with malformed events, missing manifest, or replay checksum mismatch. |
| Documentation | NEW | `docs/plan/spec/determinism-island-contract.md` — names which subsystems are deterministic (sim core, terrain mutation, AI decisions, RNG) and which are not (audio, particles cosmetic, render). |

## Files

- `game/crates/cf-replay/src/lib.rs` (MODIFY)
- `game/crates/cf-replay/src/recorder.rs` (MODIFY)
- `game/crates/cf-replay/src/event.rs` (MODIFY: 27 categories)
- `game/crates/cf-control/src/engine.rs` (MODIFY: snapshot cadence, system.category_baseline, system.run_started.protocol_version)
- `game/crates/cf-control/src/server.rs` (MODIFY: runbundle.write path-traversal guard)
- `game/crates/cf-control/src/runtime.rs` (MODIFY: ConfigInputs.checksum_cadence_ticks)
- `game/crates/cf-headless/src/main.rs` (MODIFY: structured divergence output)
- `game/crates/cf-app/src/main.rs` (MODIFY: --checksum-cadence-ticks flag)
- `game/tools/prototype_run_check.py` (MODIFY: expected_outcome validation)
- `docs/plan/spec/determinism-island-contract.md` (NEW)
- `game/scripts/cfctl/m3a_replay_compare.cfctl.json` (EXISTS)

## Acceptance criteria

```gherkin
Scenario: 5-minute M2/M2.5 run replays headlessly with matching checksums
  Given a 5-minute m2_material_lane or micro_reactor_defense run bundle
  When `cf-headless replay <bundle>` runs
  Then result=ok
  And replayed_ticks matches the original tick count
  And every recorded checksum matches the live re-run
  And no first_divergence event fires

Scenario: Drift between replay and live run reported per-tick
  Given a tampered run bundle (one event mutated)
  When cf-headless replay runs
  Then result=divergence
  And first_divergence carries tick + recorded checksum + live checksum
  And all_divergences array lists every per-tick divergence
  And exit code is non-zero

Scenario: Run bundle includes manifest + events + summary + snapshots + checksums + captures + expected_outcome
  Given any cf-app run with --write-run-bundle
  Then the bundle dir contains run_manifest.json, events.jsonl, summary.json, captures/ (if --capture-grid), snapshots in events stream
  And summary.json.expected_outcome is present (one of: clean, panic, abort)

Scenario: Canonical checker rejects bundles violating expected_outcome
  Given a run bundle with expected_outcome=clean but system.run_finished.outcome=panic
  When prototype_run_check.py runs against the bundle
  Then exit code is non-zero with a structured error pointing at the mismatch

Scenario: Per-scenario checksum cadence
  Given cf-app --checksum-cadence-ticks 30 micro_breach
  Then checksum events fire every 30 ticks
  And run_manifest.json.checksum_cadence_ticks=30
  And cf-headless replay verifies at the same cadence

Scenario: Snapshot cadence covers every objective transition
  Given a scenario with 3 objectives
  When the player completes objective 1, fails objective 2, starts objective 3
  Then snapshot_actor + snapshot_inventory + snapshot_terrain_summary fire at scene start AND at each objective transition (started/completed/failed)

Scenario: Recorder backpressure does not drop silently
  Given a recorder with capacity=100 events/tick
  When sim emits 150 events in one tick
  Then 50 events are dropped
  And recorder.dropped_count() returns 50
  And summary.json.recorder.dropped_count=50

Scenario: 27-category baseline declared at run start
  Given any cf-app run
  When the run starts
  Then system.category_baseline event fires once
  And the payload lists all 27 categories with status (active|registered)

Scenario: protocol_version in system.run_started
  Given any cf-app run
  When the run starts
  Then system.run_started event includes protocol_version equal to cf-control SCHEMA_VERSION

Scenario: runbundle.write rejects path traversal
  Given an active engine session
  When cfctl invokes runbundle.write with id_override="../../../etc/passwd"
  Then the engine rejects with reason="path_traversal_rejected"
  And no file is written outside prototype_runs/native/

Scenario: Determinism island contract document exists
  Given the project tree
  Then docs/plan/spec/determinism-island-contract.md exists
  And lists deterministic subsystems (sim, terrain, AI, RNG)
  And lists non-deterministic subsystems (audio, particles cosmetic, render)
  And documents the implications for replay verification
```

## Out of scope

- Replay viewer GUI / scrubbing / cause-chain — M3B
- Replay branching (multiple paths from same checkpoint) — DR-002 future / M3B+
- Replay editing tools — DR-002 future / M3B+
- Per-platform CI checksum matrix — depends on CI infra; out of M3A code scope
- Atmospherics / material kernel events — M5.6 (the categories are registered now; producers land later)

## Dependencies

- M1 + M1.5 + M2 + M2.5 (must be done): event sources for the verifier to chew on.

## Notes for the implementer

- The category baseline event is purely a declaration — categories with no producers yet still appear with status="registered". Producers move them to status="active" later when they emit their first event.
- `protocol_version` in `system.run_started` is the cf-control JSON-RPC schema version; bump it ONLY when an act/observe/inspect surface changes shape (additive method adds don't bump per current policy — see cf-control AGENTS.md if it still exists).
- 5-minute literal is required, not 60-second equivalent. The run-bundle checker's tick-count + cadence math is real.
- `expected_outcome` is set by the run originator (the cfctl script or cf-app caller). The checker validates the actual outcome matches.
