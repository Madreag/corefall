# cf-material — AGENTS.md

## Owns
- Systemic material kernel (real implementation pending M5.6 / DR-036).
- Material registry with 8 launch materials at BP2 (air, dirt, concrete, metal-nohook, hazard, loose fill, repair-fill, anchor).
- Per-pixel CA kernel, reaction table, density layering, phase change, electricity (all M5.6+).
- Replay-deterministic per-chunk checksums.

## Public API Boundary
- (Stub until M5.6. cf-terrain currently owns the 8-material id enum.)

## Does NOT Own
- Chunked terrain storage → `cf-terrain`.
- Atmospheric simulation → `cf-atmos`.
- Material overlay rendering → `cf-render-2d`.

## Test Surface
- (Stub.) Real coverage lands at M5.6 (MAT-01..MAT-03, MAT-06, MAT-13).

## Cross-Crate Contracts
- Will depend on: `cf-sim-core` (RNG, tick).
- Will be depended on by: `cf-terrain`, `cf-atmos`, `cf-physics`, `cf-render-2d`.

## Common Pitfalls
- Material ids must be deterministic across platforms for replay checksums.
- The reaction table must be data-driven (RON/JSON) for modding parity.

## Source Trail
- DR-036 (systemic material simulation direction; CLOSED).
- comparables/noita-grade-material-simulation-research.
