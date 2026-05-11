# cf-atmos — AGENTS.md

## Owns
- Stationeers-grade-or-better atmospherics + thermal kernel (real implementation pending M5.9 / DR-037).
- PV=nRT ideal gas law, per-gas moles + temperature, 10 launch gases + 6 liquid mixtures.
- Deterministic combustion reactions with autoignition temperatures.
- Gradual phase change with latent heat.
- Pipe networks with pumps, valves, regulators, filtration.
- Door/vent/bullet-hole/blast-breach/pipe-rupture apertures.
- Heat transfer through materials, coolant loops, heaters, radiators.
- Room atmospheres + airlock state machines + suit life-support.

## Public API Boundary
- (Stub until M5.9.)

## Does NOT Own
- Material properties → `cf-material`.
- Terrain chunks → `cf-terrain`.
- Room geometry → `cf-environment` (forward-compat crate).

## Test Surface
- (Stub.) Real coverage lands at M5.9.

## Cross-Crate Contracts
- Will depend on: `cf-sim-core`, `cf-material`, `cf-terrain`.
- Will be depended on by: `cf-physics`, `cf-actor`, `cf-ai`, `cf-render-2d`.

## Common Pitfalls
- All gas/liquid simulation must be deterministic for replay.
- Pipe networks must not block the fixed-tick sim.

## Source Trail
- DR-037 (Stationeers-grade atmospherics; CLOSED direction).
- comparables/stationeers-grade-atmospherics-and-chemistry-research.
