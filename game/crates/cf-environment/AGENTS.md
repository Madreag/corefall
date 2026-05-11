# cf-environment — AGENTS.md

## Owns
- **DR-040** `EnvironmentSignal` aggregator (per-actor per-tick environmental bundle).
- **DR-040** 15-class closed-enum `HazardClass` taxonomy (stable variant IDs, `#[repr(u8)]`).
- (Future M5.10) atmosphere / gravity / thermal / radiation / weather / comms / day_night slices folded into `EnvironmentSignal`.
- (Future M5.10) per-tick aggregation rules + cross-system reduction (`cf-atmos` + `cf-physics` + `cf-material` → single signal per actor).

## Public API Boundary
- Types: `EnvironmentSignal`, `HazardClass`.
- Constants: `EnvironmentSignal::SCHEMA_VERSION` (currently `1`).
- (Stub at BP4 scaffold; M5.10 fills in real aggregation API.)

## Does NOT Own
- Source hazard kernels (`cf-atmos` owns oxygen/pressure/fire; `cf-physics` owns gravity; `cf-material` owns thermal/radiation; weather owner is M5.10-scope).
- The `environment` event category (emitted by cf-control once `EnvironmentSignal` aggregation runs at M5.10).
- Affliction grammar (`cf-actor` owns at M5.7 + M5.8).
- AI consumption of the signal (`cf-ai` reads + reacts at M6.5+).

## Test Surface
- Unit tests: `cargo test -p cf-environment`.
- Current coverage: serde round-trip for every `HazardClass` variant + default `EnvironmentSignal` round-trip.
- Real coverage lands at M5.10 (per-tick aggregation + multi-hazard fixtures + replay determinism).

## Cross-Crate Contracts
- Depended on by: `cf-control`, `cf-actor`, `cf-ai` (M5.10+).
- Depends on: `serde`, `thiserror`, `tracing` only.

## Common Pitfalls
- **Closed-enum invariant**: `HazardClass` variants append at the end. Never renumber, never reorder. Adding a new variant in the middle breaks `#[repr(u8)]` discriminant order + every persisted signal.
- **Schema-version policy**: bump `EnvironmentSignal::SCHEMA_VERSION` only on a breaking change (field rename / type change / required-field removal). Additive fields with `#[serde(default)]` are schema-v1-compatible.
- Stub crate ships only the locked types — do NOT add aggregation logic outside M5.10 scope without an explicit roadmap update.

## Source Trail
- DR-040 (environmental conditions aggregation).
- spec/prototype-roadmap §M5.10.
- spec/milestone-enhancement-pass-m1-plus.md §M5.10 enhancement specifics.
