# cf-terrain — AGENTS.md

## Owns
- (M0 stub) Will own chunked pixel terrain (256×256 chunks), per-pixel material id, sparse storage, GPU-assisted carving, dirty-region tracking, and material-overlay state.

## Public API Boundary
- (M0 stub; nothing exposed yet.)

## Does NOT Own
- Material kernel + reactions → `cf-material` (DR-036 / T-MAT).
- Atmosphere networks → `cf-atmos` (DR-036 / M7.5).
- Physics/collision → `cf-physics` (DR-033 / T-PHYS).

## Test Surface
- (Stub.) Real coverage lands at M2 (chunk roundtrip, material set/get, carve bbox/count, replay parity).

## Cross-Crate Contracts
- Will be depended on by: `cf-render-2d`, `cf-physics`, `cf-material`, `cf-mission`, `cf-ai`.

## Common Pitfalls
- Do not introduce non-deterministic terrain mutation paths; everything must replay from a manifest+seed+input trace.

## Source Trail
- spec/prototype-roadmap §M2 — Pixel Terrain And Materials.
- DR-007 (terrain/material model, OPEN; defers implementation specifics to DR-036).
