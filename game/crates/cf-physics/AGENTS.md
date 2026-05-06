# cf-physics — AGENTS.md

## Owns
- (M0 stub) Will own broadphase, narrowphase, contact manifolds, CCD tiers, collision matrix loader, projectile-projectile contacts, impulse-to-damage routing.

## Public API Boundary
- (M0 stub.)

## Does NOT Own
- Material per-pixel sim → `cf-material`.
- Atmosphere networks → `cf-atmos`.

## Test Surface
- (Stub.) Real coverage lands at M5.5 (COLL-001..COLL-012).

## Cross-Crate Contracts
- Will be depended on by: `cf-actor`, `cf-chassis`, `cf-equipment`, `cf-ai`, `cf-mission`.

## Common Pitfalls
- Do NOT brute-force all-pairs. Every missing collision filter MUST carry a `collision_filter_reason` per DR-033.

## Source Trail
- spec/full-collision-physics-plan.
- DR-033 (full collision physics direction; CLOSED).
