# cf-headless — AGENTS.md

## Owns
- M3A replay verifier binary `cf-headless replay <bundle> [--scenario-path <path>] [--no-verify-checksums]`.
- Walks `events.jsonl`, parses every `control.command_accepted` payload back into a `ControlCommand` (15 method types: `scenario.reset`, `scenario.load`, `sim.{pause,resume,step,run_for_ticks}`, `act.player.{move,aim,fire,reload,jump,dig,select_item,reset}`, `act.settings.set`), dispatches at the engine's current tick, drives forward, and compares cadence checksums tick-for-tick. Final-kind checksums (emitted by `record_run_finished` / `write_run_bundle`) are intentionally skipped because they fire outside the replay loop.
- Replay-verifier safety: `MAX_NO_ADVANCE_RETRIES=3` iteration guard so a permanently stalled engine (e.g., `shutdown_requested` set + `RunForTicks` ineffective) cannot infinite-loop. Hard-codes `write_run_bundle: false` on every replayed `RunForTicks` so the verifier never produces a side-effect bundle.
- `--scenario-path` override + relative-path fallback (resolves common Corefall layouts so the verifier works from any cwd).
- JSON-on-stdout output: `{result, replayed_ticks, checksums_verified, commands_replayed, final_run_id}` on success, or `{result: divergence, first_divergence: {tick, recorded, live}, total_divergences}` on first mismatch with non-zero exit.
- 3 unit tests cover `parse_command` strict semantics (`write_run_bundle` always false, default branch, BP2 method catalog round-trip).

## Public API Boundary
- A binary (`cargo run -p cf-headless -- replay <bundle> ...`); no library API exposed.
- CLI flags: `replay <bundle_dir>`, `--no-verify-checksums`, `--scenario-path <path>`.

## Does NOT Own
- The sim core / engine — drives `cf-control::M0Engine` through its public dispatch API; never re-implements sim logic.
- Run-bundle envelope or schema → `cf-replay` (the verifier reads, does not write).
- The viewer + cause-chain UI — that lands at M3B (`cf-tools-replay-viewer`).
- Server-authoritative networking — separate concern at M9 (`cf-server`).

## Test Surface
- `cargo test -p cf-headless` covers the `parse_command` write_run_bundle override + default behavior + the BP2 method catalog round-trip.
- End-to-end smoke from `self_play_sweep.sh` row `m3a_headless_replay_m2_5_win`: replays the M2.5 win bundle and asserts `result: ok` + checksum match.

## Cross-Crate Contracts
- Depends on: `cf-control` (engine + scenario + ControlCommand surface), `cf-replay` (manifest + bundle paths), `cf-actor` (`IntentSource` for replayed commands), `tokio` (current-thread runtime for the async dispatch path).
- Reads run bundles produced by `cf-app` / `cfctl` / `cf-e2e`; never writes.
- Replays through the same `EngineHandle::dispatch` path the live engine uses (per AGENTS.md "no parallel production paths").

## Common Pitfalls
- The verifier dispatches `RunForTicks` to recover from a paused engine; that command lands in the recorder as `control.command_accepted`, so the replay's event log diverges from the original by those resume entries. This does NOT affect checksum verification (`sim_state_v1` hashes tick + RNG + actor + breach + chunked-terrain + reactor bytes, not the event log).
- Settings patches are replayed as `SettingsPatch::default()` (no-op) because the recorded `command_accepted` payload deliberately doesn't carry the patch contents (avoids leaking accessibility flags into the audit log). Settings don't affect the checksum so this is safe.
- The `verify_checksums` boolean is exposed as `--no-verify-checksums` (default false → verify) per the clap v4 idiomatic negation pattern; `--verify-checksums true` is NOT a valid invocation.
- Never mutate a run bundle in place — the verifier's job is to prove the bundle is replayable, not to amend it.

## Source Trail
- spec/prototype-roadmap §M3A — Event Recorder Core (M3A-003 headless replay verifier).
- spec/native-implementation-backlog M3A-001..M3A-006.
- DR-002 (replay/event architecture, OPEN — M3B closes); DR-005 (server-authoritative sim path).
- corefall/docs/implementation-log/2026-05-08-bp2-terrain-replay-build.md.
