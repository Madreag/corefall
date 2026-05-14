# M5 Pass-2 Audit — atmos.* + shield.* + environment.* + thermal.* + snapshot_shield

Scope: regression audit of the 20 events + 1 snapshot delivered or hardened by commit
`1784ad2 M5-A1: post-audit hardening pass`.

- atmos.* (10): pressure_changed, temperature_changed, gas_released, breach_detected,
  combustion_ignition, phase_transition, pipe_flow, pipe_freeze, pipe_rupture,
  electrolysis_started
- shield.* (5): hit, depleted, regen_started, regen_completed, disrupted
- environment.* (2): signal_delta, signal_aggregated
- thermal.* (3): signature_changed, heat_exchanged, material_phase_change
- snapshot.snapshot_shield (1, NEW)

Auxiliary cross-checked sources:
- `game/crates/cf-replay/src/schemas.rs` (validator + registry + tests)
- `game/crates/cf-mod/src/main.rs::validate_event_schema_value`
- `game/crates/cf-replay/src/lib.rs::EVENT_SCHEMA_VERSION`
- `game/crates/cf-environment/src/lib.rs::{HazardClass, EnvironmentSignal}`
- `game/crates/cf-replay/schemas/event/affliction_applied.json` (23-affliction enum)
- `game/crates/cf-replay/schemas/event/hazard_spawned.json` (9-kind enum)

End-to-end sanity verified before findings written:
- `cargo run -p cf-mod -- validate crates/cf-replay/schemas/` → `scanned=131 pass=131 warn=0 fail=0`
- `cargo test -p cf-replay` → 39 / 39 pass
- `cargo test -p cf-mod` → 20 unit + 11 integration / 31 pass

## Pass-1 deliveries verified

| # | Pass-1 finding | Fix shipped? | Verified by |
|---|---|---|---|
| 1 | All 74 M5 schemas declare canonical envelope literal `prototype-recorder-event.v0.1` (was `0.1`). | **YES** | `grep -l '"const": "prototype-recorder-event\.v0\.1"' atmos_*.json shield_*.json environment_*.json thermal_*.json` returns all 20 files; cf-mod validator enforces the literal (`validate_event_schema_value`, line 965-967); test `m5_schemas_declare_schema_version_v0_1` asserts the canonical literal on all 75 envelope-shaped event schemas. |
| 2 | `atmos.phase_transition.{from_phase, to_phase}` locked to `["gas", "liquid", "solid", "supercritical"]`. | **YES** | Confirmed in `atmos_phase_transition.json` lines 19-20; happy-path test `m5_per_family_happy_path` round-trips `material_phase_change` with valid phase values; pass-2 grep diff against pass-0 confirms enum was added by 1784ad2. |
| 3 | `thermal.material_phase_change.{from_phase, to_phase}` locked to `["gas", "liquid", "solid", "molten", "supercritical"]` (5 values — atmos's 4 + `molten`). | **YES** | Confirmed in `thermal_material_phase_change.json` lines 19-20. |
| 4 | `environment.signal_aggregated.payload.signal` sub-struct locked: `schema_version` (integer minimum 1) + `active_hazards` (array of 15-value HazardClass enum). | **YES** | Confirmed in `environment_signal_aggregated.json` lines 17-31. All 15 HazardClass variants present in spec literal order (Hypoxic, CombustibleAtmosphere, ToxicAtmosphere, BreachDecomp, Hyperthermic, Hypothermic, Radiation, LowVisibility, Glare, EmDisruption, WindForce, DrowningHazard, VacuumNoVoice, CommsBlackout, GravityShift) — byte-for-byte match with `cf-environment::HazardClass`. `signal.required` = `["schema_version", "active_hazards"]`. |
| 5 | `environment.signal_aggregated.cosmetic` tightened to `const: true` (was `["boolean", "null"]`). | **YES** | Confirmed in `environment_signal_aggregated.json` line 14: `"cosmetic": { "const": true }`. Same pattern mirrored on the 3 sibling cosmetic events (hazard.tick, fluid.ground_splatter_spawned, affliction.tick). |
| 6 | `snapshot.snapshot_shield` shipped. | **YES** | File `snapshot_shield.json` exists (1050 bytes); const + registry + load-test wired into `cf-replay/src/schemas.rs` lines 92, 254, 603. Description enumerates the ShieldState struct: `{ hp, max_hp, regen_rate_per_s, downtime_after_break_s, status: Up|Down|Regenerating|Disrupted }`. cf-mod validator accepts it (`scanned=131 pass=131`). |

**Verdict: 6 / 6 pass-1 deliveries landed correctly.**

### Spot-check of full schema-by-schema (regression check)

| Event | required fields verified | enums verified | numeric bounds verified |
|---|---|---|---|
| atmos.pressure_changed | atm_id, from_pa, to_pa, source ✓ | n/a | none (see NEW-J) |
| atmos.temperature_changed | atm_id, from_k, to_k, source ✓ | n/a | none (see NEW-I) |
| atmos.gas_released | atm_id, gas, moles, source, ignition_risk ✓ | gas 10-value ✓ | ignition_risk ∈ [0, 1] ✓ |
| atmos.breach_detected | atm_id, breach_size_m2, source, decompression_rate_pa_per_s ✓ | n/a | breach_size_m2 ≥ 0 ✓ (see NEW-N for decompression_rate_pa_per_s) |
| atmos.combustion_ignition | atm_id, reaction_id, energy_release_j, temperature_after_k ✓ | n/a (see NEW-E) | none |
| atmos.phase_transition | atm_id, gas, from_phase, to_phase, latent_heat_consumed_j ✓ | gas 10-value ✓; phase **4-value** locked ✓ | none |
| atmos.pipe_flow | from_pipe_id, to_pipe_id, gas, moles_per_s ✓ | gas 10-value ✓ | none |
| atmos.pipe_freeze | pipe_id, temperature_k ✓ | n/a | none (Kelvin not bounded, see NEW-I/H pattern) |
| atmos.pipe_rupture | pipe_id, breach_position, pressure_at_rupture_pa ✓ | n/a | breach_position 2-tuple float ✓ |
| atmos.electrolysis_started | electrolyzer_id, input_water_kg_per_s, output_o2_kg_per_s, output_h2_kg_per_s ✓ | n/a | all 3 rates ≥ 0 ✓ |
| shield.hit | actor_id, hp_before, hp_after, cause ✓ | n/a | none |
| shield.depleted | actor_id, cause ✓ | n/a | none |
| shield.regen_started | actor_id ✓ | n/a | none |
| shield.regen_completed | actor_id ✓ | n/a | none |
| shield.disrupted | actor_id, duration_s, cause ✓ | n/a | duration_s ≥ 0 ✓ (no max — see NEW-P) |
| environment.signal_delta | actor_id, slice, from, to, tick ✓ | slice 11-value ✓ | tick ≥ 0 ✓ |
| environment.signal_aggregated | actor_id, tick, signal ✓; signal req schema_version+active_hazards ✓ | active_hazards 15-value ✓ | signal.schema_version ≥ 1 ✓; envelope `cosmetic: const true` ✓ |
| thermal.signature_changed | actor_id, from_k, to_k ✓ | n/a | none (Kelvin not bounded, see NEW-H) |
| thermal.heat_exchanged | from_tile, to_tile, joules ✓ | n/a | tiles 2-tuple int ✓; joules unbounded (see NEW-G) |
| thermal.material_phase_change | material_id, from_phase, to_phase, position, latent_heat_consumed_j ✓ | phase **5-value** locked ✓ | position 2-tuple float ✓ |

All 20 events match the spec literal payload signatures and the pass-1 fixes are persistent.

## New issues found (pass-2)

### NEW-A: HazardClass enum vs hazard.kind enum — two different taxonomies, no cross-link doc

`environment.signal_aggregated.payload.signal.active_hazards` (DR-040 HazardClass — 15 values) is **deliberately a different taxonomy** from `hazard.spawned.payload.kind` (M16 hazard tile — 9 values). The cross-table:

| HazardClass (M20 environmental signal) | hazard.kind (M16 terrain tile) | affliction.kind (M16 actor state) |
|---|---|---|
| Hypoxic | — | hypoxic ✓ (1:1 name match) |
| CombustibleAtmosphere | (becomes `fire` once ignited) | combustible_atmosphere ✓ (1:1) |
| ToxicAtmosphere | toxic (approximate; gas vs volume) | poisoned (closest match; different name) |
| BreachDecomp | — | breach_decomp ✓ (1:1) |
| Hyperthermic | hot (approximate; the heat hazard tile) | hyperthermic ✓ (1:1) |
| Hypothermic | cold (approximate; the cold hazard tile) | hypothermic ✓ (1:1) |
| Radiation | radiation ✓ (1:1) | radiation ✓ (1:1) |
| LowVisibility | smoke (approximate; smoke causes LowVisibility) | blinded? (could route here; smoke-blinded) |
| Glare | — | blinded? (could route here; bright-light-blinded) |
| EmDisruption | electric (approximate; electric tile disrupts EM) | (no direct map — possibly electrified) |
| WindForce | — | — |
| DrowningHazard | wet (approximate; wet → drowning when severe) | (no direct map) |
| VacuumNoVoice | — | (no direct map; deafened is acoustic-perception, not signal-source) |
| CommsBlackout | — | (no direct map; radio-mute ≠ deafened) |
| GravityShift | — | (no direct map) |

**Observations:**
1. The 3 enums (15 HazardClass + 9 hazard.kind + 23 affliction.kind) form a **producer-side N:M mapping graph**, not a 1:1 cross-walk. This is correct: HazardClass is a *signal-aggregator* (cause), hazard.kind is a *world-instance* (medium), affliction.kind is an *actor-state effect*.
2. Several HazardClass variants have NO downstream hazard tile or affliction representation: WindForce, VacuumNoVoice, CommsBlackout, GravityShift. These are pure-signal slices (the AI sees them; no actor-affliction propagates).
3. M20 producer authors will need a documented routing table (which HazardClass triggers which affliction.applied event under what threshold). This routing is currently in nobody's spec or code.
4. **No description text on any of the 3 schemas cross-references the other two enums.** The taxonomies are silently parallel.

**Verdict: NOT a strict spec violation; the taxonomies are deliberately separate per the M5 spec's "environment.* is a separate concern from hazard.*". OPT-GAP: add cross-reference notes in the schema descriptions.**

### NEW-B: ShieldState struct surface in snapshot_shield.by_actor

`snapshot_shield.json` ships with `by_actor` as **open `type: array` + description-only enumeration** of the 5 per-actor fields:

```json
"by_actor": {
  "type": "array",
  "description": "Per-actor shield state: { actor_id, hp, max_hp, regen_rate_per_s, downtime_after_break_s, status: Up|Down|Regenerating|Disrupted }. status is a string enum mirroring the ShieldState struct."
}
```

This is identical to the pattern used by `snapshot_atmospherics`, `snapshot_environment_signal`, `snapshot_origin`: legacy draft-07 placeholder schemas, untyped arrays, description-only field enumeration. The pattern is intentional — at M4/M5 these are placeholders (`placeholder: true`) and M13+ producers fill the real types.

Compare with `snapshot_actor.json`, which IS typed (top-level keys actor, kind, team, position, etc.). The difference: `snapshot_actor` is one-per-event (per-actor record), so it gets a typed root. The `by_actor`-style snapshots aggregate multiple actors and emit ONE event with N records; the per-actor record sub-schema is untyped.

**Verdict: CONSISTENT with the established M4 firehose-placeholder pattern.** Status enum mirror is present in description text but not in JSON schema enum constraint — which is the same posture as `snapshot_atmospherics.atm_ids` and `snapshot_environment_signal.by_actor` (both also description-only). Recommendation in NEW-S below.

### NEW-C: shield.hit + max_hp visibility (M13 producer awareness)

`shield.hit` payload requires only `actor_id, hp_before, hp_after, cause`. The schema declares `additionalProperties: true` so M13 producer CAN emit `max_hp`, `regen_rate_per_s`, `downtime_after_break_s` additively. But the schema description text doesn't surface this expectation:

```
"description": "M5 § shield.* family ... ShieldState struct (locked): { hp, max_hp, regen_rate_per_s, downtime_after_break_s, status: Up|Down|Regenerating|Disrupted }. Producer fills at M13+ ..."
```

The description enumerates the ShieldState struct as "(locked)" but doesn't say "M13 producer is encouraged to emit max_hp/regen_rate/downtime_after_break_s as additive payload extensions so consumers can reconstruct shield state without depending on snapshot_shield."

**Verdict: NOT a strict gap — `additionalProperties: true` allows the extension. OPT-GAP: add a note in the descriptions of `shield.hit`, `shield.depleted`, `shield.regen_started`, `shield.regen_completed`, `shield.disrupted` clarifying the M13-extension expectation.**

### NEW-D: atmos.gas_released ignition_risk vs gas (no cross-validation)

`ignition_risk` is bounded `[0.0, 1.0]`, but the schema does not cross-validate with `gas`. H2 / volatiles intrinsically have higher ignition risk than O2 / N2 / He, but the schema would accept `{ gas: "He", ignition_risk: 1.0 }` (semantically nonsense — He is a noble gas).

**Verdict: CORRECT decision at v0.1 — cross-field validation is producer-side. JSON Schema's `dependencies` / `if-then-else` can express this but adds complexity; M5 chose simplicity. M19 producer enforces stoichiometric sanity.**

### NEW-E: atmos.combustion_ignition.reaction_id (open string)

`reaction_id: { "type": "string" }` — no enum. Spec doesn't lock a starter set.

Candidate starter set for M19 producer (not in schema; documentation only):
- `hydrogen_oxygen` (2H2 + O2 → 2H2O)
- `methane_oxygen` (CH4 + 2O2 → CO2 + 2H2O)
- `volatiles_oxygen` (generic fuel + O2 → CO2 + H2O)
- `wood_oxygen` (cellulose combustion)

**Verdict: NOT a gap — spec deliberately leaves `reaction_id` open. M19 will lock the M19 reaction recipe registry; M5 schema accepts any string. OPT-DOC: list a starter set in the schema description for M19 implementer awareness.**

### NEW-F: atmos.electrolysis_started rates (mass-balance unenforced)

`input_water_kg_per_s, output_o2_kg_per_s, output_h2_kg_per_s` are all bounded ≥ 0 but the schema does not enforce stoichiometry. Theoretical: 2H2O → 2H2 + O2 gives mass ratios input_water : output_h2 : output_o2 = 18 : 2 : 16 (per mole of water). So output_h2_kg + output_o2_kg = input_water_kg ideally; the schema doesn't enforce.

**Verdict: CORRECT decision — stoichiometric correctness is producer-side. v0.1 schema is permissive (e.g. partial electrolysis efficiency, intermediate state).**

### NEW-G: thermal.heat_exchanged + joules sign

`joules: { "type": "number" }` — no minimum. A negative joule value would mean reverse flow (from `to_tile` to `from_tile`). Reverse direction is encoded structurally in the from→to ordering of the tile-id pair, so producer SHOULD emit `joules >= 0` (positive scalar magnitude of energy moving in the from→to direction).

But: a producer might emit `joules: -5.0` to mean "actually it went the other way this tick" rather than re-ordering the tile pair. The spec is silent on convention.

**Verdict: AMBIGUOUS at v0.1. OPT-GAP: either (a) tighten to `minimum: 0.0` and require producer to use canonical from→to ordering, or (b) document the sign-convention in the description. Recommended (a) — explicit constraint catches producer bugs.**

### NEW-H: thermal.signature_changed K range (no Kelvin floor)

`from_k, to_k: { "type": "number" }` — Kelvin is physically bounded below at 0 K (absolute zero) and practically bounded above by combustion temperatures, but neither bound is in the schema.

**Verdict: GAP. P2: tighten to `minimum: 0.0` on both fields. Non-breaking; producers cannot emit negative Kelvin without violating physics.**

### NEW-I: atmos.temperature_changed K range (no Kelvin floor)

Same as NEW-H but for atmos. `atmos.temperature_changed.{from_k, to_k}` and `atmos.pipe_freeze.temperature_k` and `atmos.combustion_ignition.temperature_after_k` are ALL unbounded numbers.

**Verdict: GAP. P2: tighten to `minimum: 0.0` on all four fields:**
- `atmos.temperature_changed.from_k`
- `atmos.temperature_changed.to_k`
- `atmos.pipe_freeze.temperature_k`
- `atmos.combustion_ignition.temperature_after_k`

### NEW-J: atmos.pressure_changed Pa range (no vacuum floor)

`from_pa, to_pa: { "type": "number" }` — vacuum is 0 Pa; negative absolute pressure is unphysical. Same issue on `atmos.pipe_rupture.pressure_at_rupture_pa`.

**Verdict: GAP. P2: tighten to `minimum: 0.0` on:**
- `atmos.pressure_changed.from_pa`
- `atmos.pressure_changed.to_pa`
- `atmos.pipe_rupture.pressure_at_rupture_pa`

(Note: `decompression_rate_pa_per_s` on `atmos.breach_detected` is a different beast — it's a rate (Δpressure / Δtime), so it can be negative meaning "pressure RISING after breach", e.g. external atmosphere flooding in to a previously-evacuated zone. See NEW-N.)

### NEW-K: environment.signal_delta + signal value types (polymorphism intentional but undocumented)

`environment.signal_delta.payload.{from, to}: {}` — fully untyped. Per the 11-slice enum, the actual type varies:

| Slice | Expected `from`/`to` type | Example |
|---|---|---|
| atmospheric | object (gas-mix struct) | `{ "O2": 0.21, "N2": 0.78, "pressure_pa": 101325 }` |
| gravitational | number (g) | `1.0` |
| thermal | number (K) | `295.5` |
| radiation | number (Sv/h or dose) | `0.01` |
| photic | number (lux) | `500.0` |
| em | enum (EmBand) or number | `"normal"` or `0` |
| weather | enum (WeatherKind) | `"rain"` |
| water | number (liters? m³?) | `0.0` |
| acoustic | number (dB) | `40.0` |
| day_night | enum (DayPhase) | `"daylight"` |
| comms | boolean (online?) | `true` |

The polymorphism is correct (you can't lock a single sub-type), but the schema description doesn't enumerate the per-slice type contract. Producer at M20 will need a documentation reference.

**Verdict: OPT-GAP. P3: extend the description text to enumerate per-slice type expectations. No schema constraint change.**

### NEW-L: environment.signal_aggregated.signal completeness (per-slice fields missing)

`environment.signal_aggregated.payload.signal` (pass-1 locked) requires `schema_version` + `active_hazards`. Cross-reference `cf-environment::EnvironmentSignal`:

```rust
pub struct EnvironmentSignal {
    pub schema_version: u32,
    pub active_hazards: Vec<HazardClass>,
}
```

The Rust struct currently has ONLY those 2 fields. The doc comment says:

> "**Stub at M5**; M5.10 fills in atmosphere / gravity / thermal / radiation / weather / comms / day_night slices."

**Verdict: PASS — the schema sub-struct is byte-equivalent to the current Rust struct.** Per-slice fields are deliberately future work; they will be added via additive payload extension at M5.10. The `additionalProperties: true` on `signal` (line 21 of `environment_signal_aggregated.json`) permits this without an envelope bump.

**OPT-DOC**: extend the schema description to call out the planned M5.10 expansion (atmosphere / gravity / thermal / radiation / weather / comms / day_night sub-fields). Currently the description just says "Full per-actor 11-slice bundle" but doesn't note that the 11 slices are NOT yet materialized in the Rust struct.

### NEW-M: thermal.heat_exchanged tile coordinates (no coord system doc)

`from_tile, to_tile: [int, int]` — 2-tuples of integers. Description: "Per-tile ONI-style heat flow event."

"ONI-style" implies grid tiles, but the schema doesn't document:
- Origin (top-left? bottom-left?)
- Axis direction (y-up? y-down?)
- Tile size (1m? variable?)

ONI uses bottom-left origin + y-up + 1m tiles. cf-terrain probably follows the same convention (Bevy 2D world space convention is y-up). Producer at M16/M19 needs this committed.

**Verdict: OPT-GAP. P3: add a coordinate-system note to the description.**

### NEW-N: atmos.breach_detected decompression_rate (sign semantics)

`decompression_rate_pa_per_s: { "type": "number" }` — no constraint. "Decompression" semantically means pressure falling (positive rate of pressure drop = positive number = decompression occurring). If pressure RISES after breach (e.g. high-pressure external atmosphere flooding into a previously-evacuated zone), the rate is negative.

Spec is silent on the convention. Producer at M19 could go either way:
- Convention A: `decompression_rate_pa_per_s = -(d_pressure/dt)`. Positive number = pressure dropping = decompression. Negative number = pressure rising.
- Convention B: `decompression_rate_pa_per_s = d_pressure/dt`. Negative number = pressure dropping = decompression. Positive number = pressure rising.

These conventions are equally defensible. The schema doesn't pick. Producer has to pick.

**Verdict: AMBIGUOUS. OPT-GAP. P3: document the sign convention in the description. Recommended Convention A (positive = decompression) since the event name says "breach_detected" which is overwhelmingly the d_pressure/dt < 0 case.**

### NEW-O: atmos.pipe_freeze + pipe_id consistency

`pipe_id: { "type": ["integer", "string"] }` — accepts either int or string. Same dual-type pattern as `atm_id`. Spec is silent on pipe-ID taxonomy.

**Verdict: NOT a gap. M19 picks the flavor; v0.1 schema is correctly permissive.**

### NEW-P: shield.disrupted duration range (no max)

`duration_s: { "type": "number", "minimum": 0.0 }` — no maximum. A producer could emit `duration_s: 1e18` (a million years). Practically the producer will clamp to chassis-design-time limits (60s? 600s?).

**Verdict: NOT a gap at v0.1 — the maximum belongs to M13's chassis-design schema, not the event envelope. The event records what the chassis emits; the chassis's own balance constraints are separate.**

### NEW-Q: shield.regen_started + shield.regen_completed empty payload (state reconstruction)

Both events carry only `actor_id`. To reconstruct shield state at scenario replay time, a consumer must:

1. Read `snapshot.snapshot_shield` at scenario-start or objective-transition → get baseline `{ hp, max_hp, regen_rate_per_s, downtime_after_break_s, status }`.
2. Walk subsequent shield.* events in tick order:
   - `shield.hit { hp_before, hp_after }` → update hp.
   - `shield.depleted` → set hp=0, status=Down, start countdown timer for downtime_after_break_s.
   - `shield.regen_started` → set status=Regenerating.
   - `shield.regen_completed` → set hp=max_hp, status=Up.
   - `shield.disrupted { duration_s }` → set status=Disrupted, set timer.

This works for the M9 firehose model (snapshot at scenario boundaries + event chain between snapshots). The minimal payloads on regen_* events are sufficient given the M9 contract.

**Verdict: NOT a gap. State reconstruction is well-defined given snapshot + event-chain.** OPT-GAP: the schema descriptions don't make the snapshot-anchored reconstruction model explicit. M9 / M13 implementer awareness improves if descriptions note "consumers reconstruct full ShieldState from snapshot_shield + the shield.* event chain".

### NEW-R: HazardClass enum vs M16 hazard.kind cross-table (alignment risk)

Detailed cross-table moved to NEW-A. Re-verifying enum surface area:

- **affliction.applied/tick/cleared/escalated** lock the 23-name list (pass-1 added `blinded` to make 23): burning, wet, electrified, poisoned, hypoxic, combustible_atmosphere, breach_decomp, hyperthermic, hypothermic, radiation, concussed, deafened, blinded, bleeding, internal_shock, low_battery, coolant_leaking, oil_leaking, overheating, hunger, thirst, sleep_dep, sanity_low. **Verified in `affliction_applied.json` line 21 — 23 values exact, `blinded` is at index 12.**
- **hazard.spawned** locks the 9-name list: fire, smoke, electric, wet, hot, cold, acid, radiation, toxic. **Verified in `hazard_spawned.json` line 17.**
- **HazardClass** locks the 15-name list (`environment.signal_aggregated.payload.signal.active_hazards`). **Verified.**

**Cross-table for the M20 producer (this should become a documented routing table):**

| HazardClass (signal cause) | Likely hazard.spawned.kind (when materialized) | Likely affliction.applied.kind (when affecting actor) |
|---|---|---|
| Hypoxic | (no hazard tile; pure atmosphere O2 deficit) | `hypoxic` |
| CombustibleAtmosphere | `fire` (when ignited via atmos.combustion_ignition) | `combustible_atmosphere` (proximity) → `burning` (ignited) |
| ToxicAtmosphere | `toxic` (gas concentration > threshold) | `poisoned` |
| BreachDecomp | (no hazard tile; pure atmosphere ΔP) | `breach_decomp` |
| Hyperthermic | `hot` (heat-source proximity) | `hyperthermic` → `burning` (severe) |
| Hypothermic | `cold` (cold-source proximity) | `hypothermic` |
| Radiation | `radiation` (radiation field) | `radiation` |
| LowVisibility | `smoke` (smoke field) | `blinded` (severe smoke) |
| Glare | (no hazard tile; pure photic flash) | `blinded` (severe glare) |
| EmDisruption | `electric` (electric field) | `electrified` (contact) |
| WindForce | (no hazard tile; pure weather force) | (no affliction; pure perception) |
| DrowningHazard | `wet` (water tile) | (no affliction at v0.1; could route to drowning if added) |
| VacuumNoVoice | (no hazard tile; pure acoustic blanking) | (no affliction; pure perception) |
| CommsBlackout | (no hazard tile; pure EM signal blanking) | (no affliction; pure AI/HUD effect) |
| GravityShift | (no hazard tile; pure gravitational ΔG) | (no affliction at v0.1; could route to g_load_dose accumulation in cf-origin) |

**Observations:**
- 8 of 15 HazardClass variants have a clean hazard.spawned.kind mapping.
- 11 of 15 HazardClass variants have a clean affliction.applied.kind mapping.
- 4 HazardClass variants have NO downstream representation in M5's event taxonomy: WindForce, VacuumNoVoice, CommsBlackout, GravityShift. These are "pure environmental signals" that affect AI perception / HUD / movement but don't ladder up into an affliction or hazard tile. This is FINE — they're aggregator-only.
- `affliction.deafened` (one of 23 affliction.kind) has NO clean HazardClass mapping. It exists for ear-damage scenarios (post-explosion tinnitus), not as an environmental-aggregator output. So the affliction list is a superset of HazardClass-routable afflictions.

**Verdict: OPT-GAP. P3: ship a routing-table doc (M20 spec or cross-reference comment) that maps HazardClass → hazard.kind → affliction.kind. The schemas themselves are correctly locked; the gap is producer-side documentation.**

### NEW-S: snapshot_shield by_actor description completeness

Verified `snapshot_shield.json` line 18: `"description": "Per-actor shield state: { actor_id, hp, max_hp, regen_rate_per_s, downtime_after_break_s, status: Up|Down|Regenerating|Disrupted }. status is a string enum mirroring the ShieldState struct."`

All 5 ShieldState fields enumerated (hp, max_hp, regen_rate_per_s, downtime_after_break_s, status) + actor_id key. Status enum values listed (Up|Down|Regenerating|Disrupted). **Verdict: PASS.**

The description is description-only, not JSON schema enum constraint, but that matches the pattern of the 4 other firehose placeholders (snapshot_atmospherics, snapshot_environment_signal, snapshot_origin, etc.).

### NEW-T: cosmetic flag enforcement on environment.signal_aggregated

Verified `environment_signal_aggregated.json` line 14: `"cosmetic": { "const": true },`.

This is at envelope-level, not payload-level — correct placement. Mirrors the 3 sibling cosmetic events (hazard.tick line 14, fluid.ground_splatter_spawned line 14, affliction.tick line 14). **Verdict: PASS.**

The cf-replay validator (`validate_event_payload`) walks INTO `properties.payload` and does NOT check envelope-level `cosmetic`, so this const lock is enforced by strict JSON Schema validators (cf-mod, draft-2020-12 external validators) only. M20 producer must remember to set `cosmetic: true` at emit time, or external schema validation will fail. The cf-mod validator does NOT explicitly enforce this either (it doesn't walk envelope-level cosmetic), so a producer bug that mis-emits `cosmetic: false` on `signal_aggregated` would only be caught at run-bundle-validation time against the JSON Schema, not at schema-author time.

**OPT-OBSERVATION (not a P0/P1/P2/P3 issue, just an awareness note):** the `cosmetic: const true` lock is in the schema but neither cf-replay's validator nor cf-mod's schema-file validator enforces it on the emit side. External strict JSON Schema validation (e.g. third-party tools reading the schema) will reject events that mis-emit it. This is the desired contract.

## End-to-end verification

| Check | Result |
|---|---|
| `cargo run -p cf-mod -- validate game/crates/cf-replay/schemas/` | `scanned=131 pass=131 warn=0 fail=0` |
| `cargo test -p cf-replay --quiet` | `39 passed; 0 failed` |
| `cargo test -p cf-mod --quiet` | unit `20 passed`; integration `11 passed` |
| `m5_per_family_happy_path` (atmos/shield/env/thermal round trips) | ✓ |
| `m5_schemas_declare_schema_version_v0_1` (canonical literal lock) | ✓ |
| Phase enum lock atmos = `[gas, liquid, solid, supercritical]` (4) | ✓ |
| Phase enum lock thermal = `[gas, liquid, solid, molten, supercritical]` (5; superset of atmos by `molten`) | ✓ |
| 15-value HazardClass enum byte-match cf-environment::HazardClass | ✓ |
| `EnvironmentSignal { schema_version, active_hazards }` Rust struct byte-match `payload.signal` schema | ✓ |
| snapshot_shield wired into event_schema_for + load test | ✓ (lines 92, 254, 603 of cf-replay/src/schemas.rs) |
| envelope-level `cosmetic: const true` on environment.signal_aggregated | ✓ (line 14) |
| envelope-level `cosmetic: const true` on hazard.tick (sibling) | ✓ |
| envelope-level `cosmetic: const true` on fluid.ground_splatter_spawned (sibling) | ✓ |
| envelope-level `cosmetic: const true` on affliction.tick (sibling) | ✓ |

## Recommended fixes

Categorized by priority (P0 = blocker, P1 = should-fix, P2 = nice-to-have, P3 = documentation-only):

1. **P2 — Tighten Kelvin to `minimum: 0.0` (NEW-H, NEW-I).** Add `"minimum": 0.0` to:
   - `thermal_signature_changed.json` payload `from_k` + `to_k`
   - `atmos_temperature_changed.json` payload `from_k` + `to_k`
   - `atmos_pipe_freeze.json` payload `temperature_k`
   - `atmos_combustion_ignition.json` payload `temperature_after_k`
   
   Non-breaking schema tightening. Catches producer bug where Kelvin is mis-emitted as Celsius (negative number).

2. **P2 — Tighten Pa to `minimum: 0.0` (NEW-J).** Add `"minimum": 0.0` to:
   - `atmos_pressure_changed.json` payload `from_pa` + `to_pa`
   - `atmos_pipe_rupture.json` payload `pressure_at_rupture_pa`
   
   Non-breaking. Catches negative-pressure producer bugs.

3. **P2 — Tighten thermal.heat_exchanged.joules to `minimum: 0.0` and document direction convention (NEW-G).** Add `"minimum": 0.0` to `thermal_heat_exchanged.json` payload `joules`. Update description: "joules is the magnitude (always ≥ 0) of energy moving in the from_tile → to_tile direction; reverse flow swaps the tile pair rather than negating joules."

4. **P3 — Document atmos.breach_detected.decompression_rate_pa_per_s sign convention (NEW-N).** Update description in `atmos_breach_detected.json`: "decompression_rate_pa_per_s is positive when atmosphere is leaving the zone (typical breach); negative when external atmosphere is flooding INTO a previously-evacuated zone."

5. **P3 — Cross-link HazardClass ↔ hazard.kind ↔ affliction.kind taxonomies (NEW-A, NEW-R).** Add a short table in the descriptions of:
   - `environment_signal_aggregated.json` — point at hazard.spawned + affliction.applied for the routing graph.
   - `hazard_spawned.json` — point at environment.signal_aggregated.signal.active_hazards (15-class signal cause) + affliction.applied (effect on actor).
   - `affliction_applied.json` — point at HazardClass (signal cause) + hazard.kind (medium).
   
   Or alternatively ship a separate doc file. Either way the routing graph is currently undocumented and M20 implementer will need it.

6. **P3 — Document per-slice from/to type expectations on environment.signal_delta (NEW-K).** Update description to list the 11 expected types: atmospheric→gas-mix-struct, gravitational→number(g), thermal→number(K), radiation→number(Sv/h), photic→number(lux), em→string-or-number, weather→string-enum, water→number(volume), acoustic→number(dB), day_night→string-enum, comms→boolean.

7. **P3 — Document tile coordinate system on thermal.heat_exchanged (NEW-M).** Add a coordinate-system note: "Tile coordinates use cf-terrain's grid convention: bottom-left origin, y-up, 1m tiles (subject to cf-terrain's tile-size constant)."

8. **P3 — Document M5.10 expansion plan on environment.signal_aggregated.signal (NEW-L).** Update description to call out that schema_version=1 stub will expand at M5.10 to add atmosphere/gravity/thermal/radiation/weather/comms/day_night slices via `additionalProperties: true` (no envelope bump).

9. **P3 — Document M13 ShieldState extension expectation on shield.* (NEW-C).** Update descriptions of shield.hit/depleted/regen_started/regen_completed/disrupted: "M13 producer is encouraged to emit max_hp + regen_rate_per_s + downtime_after_break_s additively (allowed by `additionalProperties: true`) so consumers can reconstruct ShieldState without depending on snapshot_shield boundaries."

10. **P3 — Document state-reconstruction model on shield.regen_started/regen_completed (NEW-Q).** Update descriptions: "ShieldState is reconstructed by consumers from `snapshot_shield` at scenario-start + objective-transitions, then walked forward via the shield.* event chain. Minimal payload on regen_* events is sufficient given this anchored-chain contract."

11. **P3 — Document M19 reaction starter set on atmos.combustion_ignition (NEW-E).** Update description: "M19 reaction_id starter set (subject to extension): hydrogen_oxygen (2H2+O2→2H2O), methane_oxygen (CH4+2O2→CO2+2H2O), volatiles_oxygen (generic fuel), wood_oxygen (cellulose combustion)."

**None of (1)–(11) are P0 / P1 blockers for M6 readiness.** All are non-breaking, additive constraint tightening + documentation clarification.

## Summary

- **Pass-1 deliveries verified:** 6 / 6 (canonical schema_version literal on 20 events; phase enum lock on atmos.phase_transition + thermal.material_phase_change; environment.signal_aggregated.signal sub-struct lock; environment.signal_aggregated cosmetic const true; snapshot_shield shipped + registered).
- **New issues found (P0):** 0
- **New issues found (P1):** 0
- **New issues found (P2):** 3 (Kelvin bounds, Pa bounds, joules sign+bound on thermal.heat_exchanged)
- **New issues found (P3 docs):** 8 (cross-taxonomy routing, sign conventions, slice-type docs, tile-coord docs, M5.10 expansion plan, ShieldState extension expectation, state-reconstruction model, M19 reaction starter set)
- **Critical (P0):** **0**
- **M6 readiness verdict:** **GO.** The 20 atmos/shield/env/thermal events + snapshot_shield are M5-spec-conformant and M6-unblocking. M6 introduces 3 new categories (perception/squad/inventory) orthogonal to M5's damage scope; no rework needed.

The P2 items (numeric bounds on Kelvin/Pa/joules) are catch-the-producer-bug-at-runtime tighten-ups; they're non-breaking and would defensively cover producer mistakes at M13/M16/M19/M20. They can be batched into a single post-M5 hardening pass (M5-A2) without touching producer code.

The P3 docs items are all description-text-only improvements; they raise implementer awareness for M19/M20 but don't change schema constraints.

**Pass-1 closed all blocking gaps. M6 can start.**
