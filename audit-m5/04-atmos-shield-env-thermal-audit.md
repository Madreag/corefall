# M5 Audit — atmos.* + shield.* + environment.* + thermal.*

Scope: 20 events across four families locked at M5 v0.1 against `specs/done/M5.md`.

- atmos.* (10 events): `pressure_changed`, `temperature_changed`, `gas_released`, `breach_detected`, `combustion_ignition`, `phase_transition`, `pipe_flow`, `pipe_freeze`, `pipe_rupture`, `electrolysis_started`
- shield.* (5 events): `hit`, `depleted`, `regen_started`, `regen_completed`, `disrupted`
- environment.* (2 events): `signal_delta`, `signal_aggregated`
- thermal.* (3 events): `signature_changed`, `heat_exchanged`, `material_phase_change`

All 20 schemas live under `game/crates/cf-replay/schemas/event/` and are wired into `event_schema_for` + the `schemas_load_for_every_registered_event_type` + `m5_schemas_declare_schema_version_v0_1` tests in `game/crates/cf-replay/src/schemas.rs`.

## Per-event verdict table

| Event | Schema file | Verdict | Notes / Gaps |
|---|---|---|---|
| atmos.pressure_changed | `atmos_pressure_changed.json` | PASS | payload requires `atm_id, from_pa, to_pa, source` — literal match to spec. `atm_id` accepts integer or string (M19 hasn't picked the ID flavor yet; spec is silent so permissive is correct). |
| atmos.temperature_changed | `atmos_temperature_changed.json` | PASS | payload requires `atm_id, from_k, to_k, source` — literal match. |
| atmos.gas_released | `atmos_gas_released.json` | PASS | `atm_id, gas, moles, source, ignition_risk` required; gas enum locked to all 10 values; ignition_risk constrained `[0.0, 1.0]` via `minimum`/`maximum`. |
| atmos.breach_detected | `atmos_breach_detected.json` | PASS | `atm_id, breach_size_m2, source, decompression_rate_pa_per_s` required; `breach_size_m2 >= 0` enforced. |
| atmos.combustion_ignition | `atmos_combustion_ignition.json` | PASS | `atm_id, reaction_id, energy_release_j, temperature_after_k` required. |
| atmos.phase_transition | `atmos_phase_transition.json` | PASS (with OPT-GAP) | `atm_id, gas, from_phase, to_phase, latent_heat_consumed_j` required; gas enum locked (all 10). `from_phase` / `to_phase` are typed `string` with **no enum lock** — spec is also silent on the literal phase enum (description text only references "gas / liquid / solid"). See "Locked taxonomy coverage" below for the recommendation. |
| atmos.pipe_flow | `atmos_pipe_flow.json` | PASS | `from_pipe_id, to_pipe_id, gas, moles_per_s` required; gas enum locked (all 10). |
| atmos.pipe_freeze | `atmos_pipe_freeze.json` | PASS | `pipe_id, temperature_k` required. |
| atmos.pipe_rupture | `atmos_pipe_rupture.json` | PASS | `pipe_id, breach_position, pressure_at_rupture_pa` required; `breach_position` enforced as a 2-item float array. |
| atmos.electrolysis_started | `atmos_electrolysis_started.json` | PASS | `electrolyzer_id, input_water_kg_per_s, output_o2_kg_per_s, output_h2_kg_per_s` required; all three rates constrained `>= 0`. Chemistry-locked field set matches spec verbatim. |
| shield.hit | `shield_hit.json` | PASS | payload requires `actor_id, hp_before, hp_after, cause` — literal match. ShieldState struct (hp/max_hp/regen_rate_per_s/downtime_after_break_s/status) is documented in the schema description as "(locked)" but only `hp` is materialized per event. See "ShieldState struct visibility" below. |
| shield.depleted | `shield_depleted.json` | PASS | `actor_id, cause` required — literal match. |
| shield.regen_started | `shield_regen_started.json` | PASS | minimal payload — only `actor_id` required — literal match. |
| shield.regen_completed | `shield_regen_completed.json` | PASS | minimal payload — only `actor_id` required — literal match. |
| shield.disrupted | `shield_disrupted.json` | PASS | `actor_id, duration_s, cause` required; `duration_s >= 0` enforced. |
| environment.signal_delta | `environment_signal_delta.json` | PASS | `actor_id, slice, from, to, tick` required; 11-slice enum locked. `from` / `to` are untyped (`{}`) — spec doesn't specify the type; intentionally polymorphic per-slice. |
| environment.signal_aggregated | `environment_signal_aggregated.json` | PASS (with OPT-GAP) | `actor_id, tick, signal` required; `signal` is just `{ "type": "object" }` — see "EnvironmentSignal sub-struct" below. `cosmetic` flag declared at envelope-level (`["boolean", "null"]`), per spec's "cosmetic-batchable" flag. |
| thermal.signature_changed | `thermal_signature_changed.json` | PASS | `actor_id, from_k, to_k` required — literal match. |
| thermal.heat_exchanged | `thermal_heat_exchanged.json` | PASS | `from_tile, to_tile, joules` required; both tiles enforced as 2-item **integer** arrays per the "ONI-style per-tile" semantics in spec. |
| thermal.material_phase_change | `thermal_material_phase_change.json` | PASS (with OPT-GAP) | `material_id, from_phase, to_phase, position, latent_heat_consumed_j` required; `position` enforced as 2-item float array. `from_phase` / `to_phase` are typed `string` with **no enum lock** — same issue as `atmos.phase_transition`. |

### Cross-check vs `event_schema_for` registration

All 20 (category, event_type) pairs are registered in `event_schema_for`:

```rust
("atmos", "pressure_changed") => Some(SCHEMA_ATMOS_PRESSURE_CHANGED),
("atmos", "temperature_changed") => Some(SCHEMA_ATMOS_TEMPERATURE_CHANGED),
("atmos", "gas_released") => Some(SCHEMA_ATMOS_GAS_RELEASED),
("atmos", "breach_detected") => Some(SCHEMA_ATMOS_BREACH_DETECTED),
("atmos", "combustion_ignition") => Some(SCHEMA_ATMOS_COMBUSTION_IGNITION),
("atmos", "phase_transition") => Some(SCHEMA_ATMOS_PHASE_TRANSITION),
("atmos", "pipe_flow") => Some(SCHEMA_ATMOS_PIPE_FLOW),
("atmos", "pipe_freeze") => Some(SCHEMA_ATMOS_PIPE_FREEZE),
("atmos", "pipe_rupture") => Some(SCHEMA_ATMOS_PIPE_RUPTURE),
("atmos", "electrolysis_started") => Some(SCHEMA_ATMOS_ELECTROLYSIS_STARTED),
("shield", "hit") => Some(SCHEMA_SHIELD_HIT),
("shield", "depleted") => Some(SCHEMA_SHIELD_DEPLETED),
("shield", "regen_started") => Some(SCHEMA_SHIELD_REGEN_STARTED),
("shield", "regen_completed") => Some(SCHEMA_SHIELD_REGEN_COMPLETED),
("shield", "disrupted") => Some(SCHEMA_SHIELD_DISRUPTED),
("environment", "signal_delta") => Some(SCHEMA_ENVIRONMENT_SIGNAL_DELTA),
("environment", "signal_aggregated") => Some(SCHEMA_ENVIRONMENT_SIGNAL_AGGREGATED),
("thermal", "signature_changed") => Some(SCHEMA_THERMAL_SIGNATURE_CHANGED),
("thermal", "heat_exchanged") => Some(SCHEMA_THERMAL_HEAT_EXCHANGED),
("thermal", "material_phase_change") => Some(SCHEMA_THERMAL_MATERIAL_PHASE_CHANGE),
```

All 20 also appear in `schemas_load_for_every_registered_event_type` and `m5_schemas_declare_schema_version_v0_1`. Verdict: registration is complete and consistent.

## Locked taxonomy coverage

### atmos.gas_released — 10-gas enum

Spec: `gas: 'O2'|'N2'|'CO2'|'volatiles'|'pollutant'|'H2'|'N2O'|'H2O'|'O3'|'He'`.

Schema (`atmos_gas_released.json`):

```json
"gas": { "type": "string", "enum": ["O2", "N2", "CO2", "volatiles", "pollutant", "H2", "N2O", "H2O", "O3", "He"] }
```

Count: 10. Order: matches spec literal. **Verdict: PASS.**

Bonus check — the same 10-value enum is mirrored verbatim in:

- `atmos_phase_transition.json` payload `gas` field ✓
- `atmos_pipe_flow.json` payload `gas` field ✓

All three gas-bearing atmos events share one canonical enum list — drift-free.

### atmos.phase_transition — phase enum (gas / liquid / solid?)

Spec literal payload signature: `atmos.phase_transition { atm_id, gas, from_phase, to_phase, latent_heat_consumed_j }`. The spec **does not literally enumerate** phase values in the bullet list; the prose example references "H2O gas → liquid at the dewpoint, or liquid → solid (ice)" — implying at least {gas, liquid, solid} and possibly supercritical.

Schema (`atmos_phase_transition.json`):

```json
"from_phase": { "type": "string" },
"to_phase": { "type": "string" }
```

Both are untyped `string` — no enum lock.

**Verdict: PASS (spec-conformant; spec is itself silent on the enum), with OPT-GAP recommendation.** The recommended tightening is to lock the enum to e.g. `["gas", "liquid", "solid", "supercritical"]` so M19 producers can't drift to `"gaseous"` / `"frozen"` / etc. This would be a non-breaking schema tightening (no current producer exists). Because the spec literally leaves it open, this is a recommendation rather than a strict gap.

### environment.signal_delta — 11-slice enum

Spec: `slice: 'atmospheric'|'gravitational'|'thermal'|'radiation'|'photic'|'em'|'weather'|'water'|'acoustic'|'day_night'|'comms'`.

Schema (`environment_signal_delta.json`):

```json
"slice": { "type": "string", "enum": ["atmospheric", "gravitational", "thermal", "radiation", "photic", "em", "weather", "water", "acoustic", "day_night", "comms"] }
```

Count: 11. Order: matches spec literal. **Verdict: PASS.**

### environment.signal_aggregated — EnvironmentSignal sub-struct

Spec: `environment.signal_aggregated { actor_id, tick, signal: EnvironmentSignal } (full bundle; cosmetic-batchable)`.

Schema (`environment_signal_aggregated.json`):

```json
"signal": { "type": "object" }
```

`signal` is an opaque object placeholder — no enforcement of `EnvironmentSignal` shape.

Cross-reference: `game/crates/cf-environment/src/lib.rs` ships the locked Rust type:

```rust
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EnvironmentSignal {
    pub schema_version: u32,
    pub active_hazards: Vec<HazardClass>,
}
```

The schema does NOT reference `schema_version` or `active_hazards` inside the `signal` object, nor does it pin the 11-slice band structure the M20 aggregator is expected to ship.

**Verdict: PASS (object placeholder), with OPT-GAP.** Strictly the spec's promise is "signal: EnvironmentSignal" — a sub-struct — and locking it to `{ "type": "object" }` is permissive. The full 11-slice + schema_version + active_hazards shape should ideally be locked at M5 since M20 has to ship a stable payload; today the schema accepts any object. Recommendation: extend the schema to require `signal.schema_version` (integer) + `signal.active_hazards` (array) + per-slice optional fields (currently undeclared) once M20 is locked. Marking this as opt-gap because the spec doesn't literally enumerate EnvironmentSignal's internal field set.

`cosmetic` flag — schema declares:

```json
"cosmetic": { "type": ["boolean", "null"] }
```

at envelope-level (top-level of the M5 envelope-shaped schema, not inside payload). Spec calls signal_aggregated "cosmetic-batchable" — so producer at M20 is expected to set `cosmetic: true`. The schema accepts any value (true/false/null) which is permissive. Recommendation: leave permissive (matches M4 envelope contract — every event can declare cosmetic), but document the M20 expectation. Verdict: PASS.

### thermal.material_phase_change — phase enum match with atmos

Spec literal: `thermal.material_phase_change { material_id, from_phase, to_phase, position, latent_heat_consumed_j }`. Spec's description text: "Material phase change (e.g. ice → water, water → vapor, metal → molten)" — implies {solid (ice), liquid (water), gas (vapor), molten}.

Schema (`thermal_material_phase_change.json`):

```json
"from_phase": { "type": "string" },
"to_phase": { "type": "string" }
```

Same shape as `atmos.phase_transition` — untyped string. No cross-link guarantee between the two phase enums.

**Verdict: PASS (spec-conformant), with OPT-GAP recommendation paired with atmos.phase_transition.** If the project locks `["gas", "liquid", "solid", "supercritical"]` at one event, the other should mirror it (or extend with a `"molten"` variant for material-only phase changes). Currently both are open-string and the spec doesn't constrain — so the schemas pass the literal spec check.

### ShieldState struct visibility (hp, max_hp, regen_rate_per_s, downtime_after_break_s, status)

Spec: `ShieldState struct (locked): { hp, max_hp, regen_rate_per_s, downtime_after_break_s, status: Up|Down|Regenerating|Disrupted }`.

Per-event audit of which `ShieldState` fields surface through each shield.* event payload:

| ShieldState field | shield.hit | shield.depleted | shield.regen_started | shield.regen_completed | shield.disrupted |
|---|---|---|---|---|---|
| `hp` | implicit via `hp_before`/`hp_after` (delta only) | NO | NO | NO | NO |
| `max_hp` | NO | NO | NO | NO | NO |
| `regen_rate_per_s` | NO | NO | NO | NO | NO |
| `downtime_after_break_s` | NO | NO | NO | NO | NO |
| `status` | NO (inferred from event type) | NO (implicit: Down) | NO (implicit: Regenerating) | NO (implicit: Up) | NO (explicit: Disrupted via event identity + `duration_s`) |

**Status transitions are reconstructible from event ordering** (depleted → regen_started → regen_completed forms the Down → Regenerating → Up cycle; disrupted overrides to Disrupted for `duration_s`). But **NO single shield.* event payload materializes the full ShieldState**. Specifically:

- `max_hp`, `regen_rate_per_s`, `downtime_after_break_s` are never written into any event — a consumer cannot recover them from the event stream without a separate `snapshot_shield` event (which **does not exist** — searched `schemas/event/snapshot_*.json`; no `snapshot_shield.json`).
- `status` is never serialized as a field; only inferable from event identity.

**Verdict: SPEC-CONFORMANT (spec literally writes the four-event payload signatures and they match), but OPT-GAP.** The schemas as written match the spec to the letter — the spec only requires the four minimal payloads. The opt-gap is:

1. No `snapshot_shield` schema exists. Compare against `snapshot_atmospherics.json` + `snapshot_environment_signal.json` which DO exist as M4 § M9 firehose placeholders. Recommendation: add `snapshot_shield.json` mirroring the ShieldState struct so M9-onward bundles can serialize shield state at scenario start / objective transitions.
2. M13+ shield producer may want to emit `max_hp` / `regen_rate_per_s` / `downtime_after_break_s` as additive payload extensions (the M4 envelope allows additive fields without bumping the envelope; `additionalProperties: true` is set on every payload). The schemas do NOT need changing — producers can extend at will.
3. The Rust `ShieldState` struct itself does not exist anywhere in `game/crates/` today (searched all crates; only the `shield_hit.json` description references it). This is fine for M5 (declarative milestone — no producer code expected) but worth flagging for M13.

### Shield status enum (Up | Down | Regenerating | Disrupted)

Spec: `status: Up|Down|Regenerating|Disrupted`.

No shield.* event payload schema carries a `status` field. The enum is locked **only in prose** (in the `shield_hit.json` description and in the M5 spec). No JSON schema enforces it.

**Verdict: SPEC-CONFORMANT (spec doesn't ask for a `status` field in any event payload), with OPT-GAP.** Same recommendation as above: add a `snapshot_shield.json` schema that locks `status` to the 4-value enum.

### Cosmetic flag on cosmetic events (environment.signal_aggregated)

Per-event check for the `cosmetic` envelope-level flag declaration:

| Event | Schema declares `cosmetic`? | Spec marks cosmetic? |
|---|---|---|
| environment.signal_aggregated | YES (`"cosmetic": { "type": ["boolean", "null"] }`) | YES ("cosmetic-batchable") |
| environment.signal_delta | NO | NO (sim-authoritative) |
| atmos.* (all 10) | NO | NO (sim-authoritative) |
| shield.* (all 5) | NO | NO (sim-authoritative) |
| thermal.* (all 3) | NO | NO (sim-authoritative) |

The one cosmetic event in this audit set (`environment.signal_aggregated`) correctly declares the `cosmetic` field. All other 19 events in this audit set are sim-authoritative per spec and do not declare a `cosmetic` field — consistent.

**Verdict: PASS.** Note: the schema declares `cosmetic` as **optional and nullable** rather than `const true`. Producer at M20 must remember to set `cosmetic: true` on every emit; the schema will not enforce it. This is the same permissive pattern used in `hazard_tick.json`, `affliction_tick.json`, and `fluid_ground_splatter_spawned.json` — consistent house style across all cosmetic events.

## Recommended fixes

The audit found NO strict spec violations across the 20 events; all schemas literally match the M5.md payload signatures. The following are **opt-gap recommendations** for tightening (non-breaking; all are schema-level additive constraints):

1. **Lock `from_phase` / `to_phase` enum in `atmos_phase_transition.json` and `thermal_material_phase_change.json`** to a shared list. Suggested literal: `["gas", "liquid", "solid", "supercritical"]` for atmos; `["gas", "liquid", "solid", "molten", "supercritical"]` for thermal. This prevents M19 / M16 producers from drifting to inconsistent string spellings. Trade-off: M5.md does not literally lock this enum, so leaving open is also defensible.

2. **Tighten `signal` shape in `environment_signal_aggregated.json`.** Today it's `{ "type": "object" }`; recommended to require `signal.schema_version` (integer, equal to `EnvironmentSignal::SCHEMA_VERSION = 1`) plus `signal.active_hazards` (array of strings from the `HazardClass` 15-class enum). This would lock the EnvironmentSignal sub-struct shape so M20 producers can't drift. The DR-040 `HazardClass` enum is already locked in `cf-environment::HazardClass` (Hypoxic / CombustibleAtmosphere / ToxicAtmosphere / BreachDecomp / Hyperthermic / Hypothermic / Radiation / LowVisibility / Glare / EmDisruption / WindForce / DrowningHazard / VacuumNoVoice / CommsBlackout / GravityShift).

3. **Add `snapshot_shield.json`** under `schemas/event/` mirroring the `ShieldState { hp, max_hp, regen_rate_per_s, downtime_after_break_s, status: Up|Down|Regenerating|Disrupted }` struct, paralleling the existing `snapshot_atmospherics.json` + `snapshot_environment_signal.json` placeholders. Wire it into `event_schema_for` as `("snapshot", "snapshot_shield")`. This unlocks M9 firehose visibility of shield state at scenario start / objective transitions without requiring per-event payload bloat.

4. **(Producer-side note only, no schema change needed.)** M13+ shield producer should consider emitting `max_hp`, `regen_rate_per_s`, `downtime_after_break_s` on `shield.regen_started` and `shield.disrupted` as additive payload extensions so consumers can reconstruct full ShieldState without needing a separate snapshot. The schemas already allow this via `additionalProperties: true`.

None of (1)–(4) require an envelope bump per M4 DR-002. (1) and (2) are schema-tightening (additive constraints). (3) adds a new schema. (4) is producer-side.

## Summary

- **Total events audited:** 20 (atmos 10 + shield 5 + environment 2 + thermal 3)
- **PASS (strict spec match):** 20 / 20
- **GAP (strict spec violation):** 0
- **OPT-GAP recommendations (non-violation tightening opportunities):** 4 (phase enums on `atmos_phase_transition` + `thermal_material_phase_change`; `signal` sub-struct on `environment_signal_aggregated`; no `snapshot_shield`; producer-side ShieldState surfacing on shield.regen_*)
- **Critical missing pieces:** none. The M5 event-surface lock for atmos + shield + environment + thermal is consistent with the spec to the letter.

Locked-taxonomy spot-checks:

- 10-gas enum: PASS (mirrored verbatim across `atmos_gas_released`, `atmos_phase_transition`, `atmos_pipe_flow`).
- 11-slice enum: PASS (`environment_signal_delta`).
- Phase enums (atmos + thermal): permissive (open string) — spec is also silent.
- Shield status enum: not encoded in any event payload — consistent with spec; status is reconstructible from event-type identity.
- ShieldState struct mirror: only `hp` (via hp_before/hp_after on shield.hit) is materialized; max_hp / regen_rate_per_s / downtime_after_break_s / status are doc'd-as-locked but not serialized per event.
- EnvironmentSignal sub-struct: opaque `{ "type": "object" }` placeholder.
- Cosmetic flag: correctly declared on `environment.signal_aggregated` only; matches spec.

Registration cross-check: all 20 (category, event_type) pairs present in `event_schema_for` and asserted in `schemas_load_for_every_registered_event_type` + `m5_schemas_declare_schema_version_v0_1` tests — drift-free.
