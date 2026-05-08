# cf-terrain — AGENTS.md

## Owns
- M1.5 soft-breach barrier proxy: `BreachStrip`, `BreachWorld`, `try_dig`, `DigRequest`, `DigOutcome`, `BreachView`.
- Pair of M1.5 launch materials: `concrete_soft` (diggable) and `metal_nohook` (refuses dig with `material_metal_nohook`). M2's chunked terrain replaces the strip with real per-pixel terrain; the event names and payload shapes here intentionally match what M2 will emit so consumers (replay viewer, AI hooks, run-bundle checker) don't migrate.
- `BreachWorld::reset` for the engine to rewind on `scenario.reset`, plus `BreachWorld::checksum_bytes` so deterministic checksum extends through breach state.
- Anti-scope: no chunked terrain, no GPU carving, no material kernel, no full launch material set — those land at M2 (M2 chunked terrain) and M5.6 (material kernel).

## Public API Boundary
- Types: `BreachStrip`, `BreachWorld`, `DigRequest`, `DigOutcome`, `BreachView`.
- Functions: `try_dig(&mut BreachWorld, DigRequest) -> DigOutcome`, `BreachWorld::is_broken`, `BreachWorld::broken_map`, `BreachWorld::reset`, `BreachWorld::checksum_bytes`, `BreachWorld::iter`.

## Does NOT Own
- Recorder events / run-bundle writing → `cf-control` engine emits `terrain.*` events from `DigOutcome`.
- Material kernel + reactions → `cf-material` (DR-036 / T-MAT, lands at M5.6).
- Atmosphere networks → `cf-atmos` (DR-036 / M7.5).
- Physics/collision against terrain proxies → `cf-physics` (DR-033 / T-PHYS, lands at M5.5).

## Test Surface
- Unit tests: `cargo test -p cf-terrain` covers out-of-range refusal, metal-nohook material refusal, three-attempt breach, nearest-strip picker, explicit-target routing, unknown-target refusal, reset, broken-map consistency, and checksum-byte change after carve.

## Cross-Crate Contracts
- Depended on by: `cf-control` (engine + scenario + observe envelope + run-bundle event emission), `cf-render-2d` (render-side breach projection), `cf-app` (HUD bridge).
- Events emitted by the engine from a `DigOutcome`: `terrain.tool_action_started`, `terrain.terrain_carved`, `terrain.terrain_breach_stub`, `terrain.tool_refused`. M2 replaces only the `*_breach_stub` event; the rest stay live.

## Common Pitfalls
- Refusal reason names ship with a stable vocabulary: `out_of_range`, `already_broken`, `unknown_target`, `material_<material_name>`. Replay tooling parses these; do not change spelling without also bumping a schema fixture.
- The strip picker does NOT skip refusal-only strips. The player still gets a refusal event when they swing at a metal-nohook anchor — that's the documented teaching path.
- `BreachStrip` HP is a `f32` because hardness is a `f32`; the engine quantizes through `quantize` (×1024 → i32) for the checksum so per-pixel resolution is plenty.

## Source Trail
- spec/prototype-roadmap §M1.5 — Micro Breach Fun Slice (M1.5-003 temporary soft breach).
- spec/native-implementation-backlog M1.5-003.
- DR-007 (terrain/material model, OPEN — defers implementation specifics to DR-036).
- DR-036 (systemic material simulation direction, CLOSED — implementation specifics defer to M5.6).
