# cf-sim-core — AGENTS.md

## Owns
- Fixed-tick scheduler (60 Hz default; 120 Hz selectable).
- `Tick(u64)`, `WallClock`, `SimClock` types; `pause`/`resume`/`step(n)`/`run_for(n)` API.
- Deterministic `Rng` wrapper around `Xoshiro256StarStar`.
- `sim_state_v1` checksum scope helper (M0: `tick_counter || rng_state_bytes`; future milestones append fields).
- Run-id and event-id helpers used by every binary.

## Public API Boundary
- Types: `Tick`, `SimClock`, `SimMode`, `SimConfig`, `Rng`, `WallClock`.
- Modules: `checksum::{sim_state_v1, CHECKSUM_ALGORITHM, CHECKSUM_SCOPE}`, `ids::{make_run_id, make_event_id, iso_hyphen_safe}`.
- Everything else is internal.

## Does NOT Own
- Replay events or run-bundle writing → `cf-replay`.
- Control envelope / WebSocket transport → `cf-control`.
- Actors, terrain, physics, AI → `cf-actor`/`cf-terrain`/`cf-physics`/`cf-ai`.

## Test Surface
- Unit tests: `cargo test -p cf-sim-core`.
- Required event categories in run bundles: none (this crate emits no events; consumers do).

## Cross-Crate Contracts
- Depends on: nothing internal.
- Depended on by: `cf-replay`, `cf-control`, `cf-app`, `cfctl`.

## Common Pitfalls
- Do NOT call `rand::thread_rng` — clippy lint forbids it. Use `Rng::from_seed`.
- Do NOT call `std::time::SystemTime::now` — clippy lint forbids it. Use `WallClock::now_utc`/`now_instant`.
- Bumping the checksum scope's byte layout (vs append-only growth) requires moving to `sim_state_v2` and registering a migration in the run-bundle schema doc.

## Source Trail
- DR-002 (replay/event architecture, OPEN — closes at M3).
- spec/prototype-roadmap §Coordinate System And Units.
- docs/implementation-log/2026-05-05-m0-engine-bootstrap.md §DR-002 v1 schema lock.
