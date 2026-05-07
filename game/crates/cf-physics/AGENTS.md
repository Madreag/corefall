# cf-physics — AGENTS.md

## Owns
- M1 stateless 2D physics helpers used by `cf-actor::sim`:
  - `step_kinematics`: gravity + ground collision + landed-impulse calc.
  - `apply_horizontal_motion`: ground/air acceleration + friction + region-bound clamp.
  - `apply_jump`: ground-only jump impulse.
  - `apply_recoil`: weapon recoil applied to the firer's velocity.
- M5.5 will replace the M1 floor-and-bounds model with the full collision matrix per DR-033.

## Public API Boundary
- Functions: `step_kinematics(StepInputs) -> StepOutputs`, `apply_horizontal_motion(HorizontalInputs) -> HorizontalOutputs`, `apply_jump(JumpInputs) -> (f32, bool)`, `apply_recoil(velocity_x, aim_x, recoil_impulse) -> f32`.
- Inputs/outputs structs are `Serialize + Deserialize` so they can ride in replay events later.

## Does NOT Own
- Material per-pixel sim → `cf-material`.
- Atmosphere networks → `cf-atmos`.
- Broadphase / narrowphase / CCD / contact manifolds / projectile-projectile rules / collision matrix → still M5.5 (DR-033 / T-PHYS).

## Test Surface
- Unit tests: `cargo test -p cf-physics` covers gravity, floor clamp, terminal velocity, jump-only-on-ground, region clamp, ground friction, recoil sign.
- M5.5 will add COLL-001..COLL-012 fixtures.

## Cross-Crate Contracts
- Pure functions; no shared mutable state. Callers (cf-actor::sim, future cf-physics) own the mutable fields.

## Common Pitfalls
- `step_kinematics` reports `landed_impulse` only on the tick the actor first contacts the ground; consumers must track the previous on-ground state if they need a continuous "is on ground" signal.
- `apply_recoil` falls back to `+x` when `aim_x` is exactly zero; this avoids NaN but may produce a surprise direction if callers feed un-normalized aims.

## Source Trail
- spec/prototype-roadmap §M1 — Actor Controller And Sim Core (M1-002, M1-003).
- spec/full-collision-physics-plan (M5.5 / T-PHYS).
- DR-033 (full collision physics direction; CLOSED).
- docs/implementation-log/2026-05-06-m1-actor-controller.md.
