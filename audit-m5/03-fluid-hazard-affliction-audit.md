# M5 Audit — fluid.* + hazard.* + affliction.*

**Scope.** Audit the 18 shipped M5 event schemas across three families against `specs/done/M5.md`:

- `fluid.*` — 9 events
- `hazard.*` — 5 events
- `affliction.*` — 4 events

**Inputs reviewed.**

- Spec: `/Users/erol/projects/corefall/specs/done/M5.md` (sections "fluid.* family", "hazard.* family", "affliction.* family")
- Shipped schemas: `/Users/erol/projects/corefall/game/crates/cf-replay/schemas/event/{fluid,hazard,affliction}_*.json`
- Validator registration: `/Users/erol/projects/corefall/game/crates/cf-replay/src/schemas.rs`

**TL;DR.** Every shipped schema is *structurally* PASS. The only material drift is internal contradiction inside `hazard_spawned.json`'s `description` (it claims a 5-launch list "fire, smoke, electric, wet, hot_cold" while the enum uses `hot`+`cold` as two separate values and never the underscore form). Two minor opportunities to harden enums (`fluid.ignition` combustible gating, `fluid.leak_started` source_module ↔ fluid_kind binding) are noted as DEFERRED-TO-PRODUCER. All 22 affliction kinds, all 4 fluid kinds, all enum reason fields, and all cosmetic flags are present and correctly named.

---

## Per-event verdict table

| Event | Schema file | Verdict | Notes / Gaps |
|---|---|---|---|
| fluid.leak_started | fluid_leak_started.json | PASS | All 5 spec fields (`actor_id`, `fluid_kind`, `source_module_id`, `leak_rate`, `position`) present and required. `fluid_kind` enum = the 4 locked kinds. `source_module_id` typed `["integer","string"]` — flexible, not bound to fluid_kind (see note below). |
| fluid.leak_rate_changed | fluid_leak_rate_changed.json | PASS | All 5 spec fields (`actor_id`, `fluid_kind`, `from_rate`, `to_rate`, `reason`) present and required. `reason` is a free-form string (no enum) — spec does not lock a reason set here. |
| fluid.reservoir_warning | fluid_reservoir_warning.json | PASS | All 3 spec fields (`actor_id`, `fluid_kind`, `level_pct`) present and required. 50% threshold documented in `description`. |
| fluid.reservoir_critical | fluid_reservoir_critical.json | PASS | All 3 spec fields present and required. 20% threshold documented in `description`. |
| fluid.reservoir_empty | fluid_reservoir_empty.json | PASS | All 3 spec fields (`actor_id`, `fluid_kind`, `cascade_effects`) present and required. All 4 locked cascade behaviours quoted in the description: oil → joint seizure + movement -50% + motor failure; coolant → heat buildup + overheating; fuel → mobility offline + chassis inert + ignition risk; electrolyte → action costs rise + eventually inert. |
| fluid.ignition | fluid_ignition.json | PASS (with note) | All 3 spec fields present and required. Combustible-only gating documented in `description` ("Fires for combustible-only fluids (fuel + leaked oil)") but **not enforced** at enum level — `fluid_kind` accepts all 4 values. Producer-side concern. |
| fluid.ground_splatter_spawned | fluid_ground_splatter_spawned.json | PASS | All 4 spec fields (`fluid_kind`, `position`, `volume_l`, `terrain_hazard_kind`) present and required; correctly omits `actor_id` per spec (cosmetic splatter event has no owning actor). Envelope-level `cosmetic` boolean field declared. |
| fluid.leak_stopped | fluid_leak_stopped.json | PASS | All 3 spec fields present and required. `reason` enum exactly matches spec: `["sealed", "repaired", "reservoir_empty"]`. |
| fluid.refilled | fluid_refilled.json | PASS | All 4 spec fields (`actor_id`, `fluid_kind`, `amount`, `source_actor_id`) present and required. `amount` is `{minimum: 0.0}` (no negative refills). |
| hazard.spawned | hazard_spawned.json | PASS (with documentation drift) | All 5 spec fields (`hazard_id`, `kind`, `position`, `intensity`, `source_event_id`) present and required. `kind` enum has 9 values matching the event-definition list `[fire, smoke, electric, wet, hot, cold, acid, radiation, toxic]`. **Drift:** the schema `description` still references the 5-launch label "fire, smoke, electric, wet, hot_cold" — the literal string `hot_cold` does NOT appear in the enum (the enum uses `hot` and `cold` separately). Same description also says "M16 extends to 9 with acid, radiation, toxic" — uses the shorter spec names, not "radiation_zone" / "toxic_atmosphere" from the bullet section. See ambiguity section below. |
| hazard.spread | hazard_spread.json | PASS | All 5 spec fields (`from_pos`, `to_pos`, `kind`, `intensity`, `rate`) present and required. `kind` enum = same 9 values as `hazard.spawned`. |
| hazard.actor_contact | hazard_actor_contact.json | PASS | All 4 spec fields (`actor_id`, `hazard_id`, `kind`, `intensity`) present and required. `kind` enum = same 9 values. |
| hazard.tick | hazard_tick.json | PASS | All 2 spec fields (`hazard_id`, `tick`) present and required. Envelope-level `cosmetic` boolean field declared. 10:1 batching documented in description. |
| hazard.dissipated | hazard_dissipated.json | PASS | All 2 spec fields (`hazard_id`, `reason`) present and required. `reason` enum exactly matches spec: `["time", "doused", "spread_out"]`. |
| affliction.applied | affliction_applied.json | PASS | All 5 spec fields (`actor_id`, `kind`, `source_event_id`, `expected_duration_ticks`, `severity_0_1`) present and required. `kind` enum has all 22 locked affliction names. `severity_0_1` constrained to `[0.0, 1.0]`. |
| affliction.tick | affliction_tick.json | PASS | All 4 spec fields (`actor_id`, `kind`, `hp_delta`, `tick`) present and required. Envelope-level `cosmetic` boolean field declared. `kind` enum = full 22 names. |
| affliction.cleared | affliction_cleared.json | PASS | All 3 spec fields (`actor_id`, `kind`, `reason`) present and required. `reason` enum exactly matches spec: `["time", "medikit", "environment", "death"]`. |
| affliction.escalated | affliction_escalated.json | PASS | All 4 spec fields (`actor_id`, `kind`, `from_severity`, `to_severity`) present and required. Severities both constrained to `[0.0, 1.0]`. |

**Registry cross-check.** All 18 events are registered in `event_schema_for(category, event_type)` (`schemas.rs:259-292`) and exercised by the `schemas_load_for_every_registered_event_type` test (`schemas.rs:411-466`). All 18 are also exercised by the `m5_schemas_declare_schema_version_v0_1` test (`schemas.rs:594-617`), which asserts each declares `properties.schema_version.const == "0.1"`, `properties.category.const == <family>`, `properties.event_type.const == <event>`.

---

## Locked taxonomy coverage

### 4 fluid kinds (oil / coolant / fuel / electrolyte)

**Verdict: PASS.**

The literal enum

```json
"fluid_kind": { "type": "string", "enum": ["oil", "coolant", "fuel", "electrolyte"] }
```

is present and identical across all 7 fluid events that include `fluid_kind` in their payload:

- `fluid_leak_started.json`
- `fluid_leak_rate_changed.json`
- `fluid_reservoir_warning.json`
- `fluid_reservoir_critical.json`
- `fluid_reservoir_empty.json`
- `fluid_ignition.json`
- `fluid_ground_splatter_spawned.json`
- `fluid_leak_stopped.json`
- `fluid_refilled.json`

All 9 share the same enum spelling. No drift.

### 4 fluid source modules (oil_reservoir / coolant_pump / fuel_tank / power_core)

**Verdict: PASS (deferred-to-producer at enum level, locked in description).**

`fluid_leak_started.json` has

```json
"source_module_id": { "type": ["integer", "string"] }
```

— no enum constraint. The four locked module names appear in the schema's top-level `description`:

> "4 fluid reservoirs (locked): oil (oil_reservoir), coolant (coolant_pump), fuel (fuel_tank), electrolyte (power_core)."

This is acceptable for v0.1 because the producer ladder (M13 chassis fluid system) may use either string identifiers or internal integer module IDs. The validator does not bind `fluid_kind` to `source_module_id` (e.g. nothing prevents a producer emitting `{fluid_kind: "oil", source_module_id: "fuel_tank"}` — see "Recommended fixes" #1).

### 4 fluid empty cascades (locked behaviour descriptions)

**Verdict: PASS.**

The `description` field of `fluid_reservoir_empty.json` enumerates all four cascade behaviours verbatim:

```text
oil → joint seizure + movement -50% + eventual motor failure;
coolant → heat buildup + overheating affliction stacks;
fuel → mobility offline + chassis inert + ignition risk;
electrolyte → action costs rise + eventually inert.
```

These are documented (not enforced — `cascade_effects` is `array<string>` with no enum). Acceptable for v0.1 because the producer is responsible for emitting the right strings; the schema records intent.

### Hazard kind enum (5 launch / 8 / 9 — spec ambiguity)

**Spec literal text (two places, contradictory):**

- Event signature: `kind: fire|smoke|electric|wet|hot|cold|acid|radiation|toxic` (9 values)
- Bullet immediately below the events: "5 launch hazard kinds (locked): `fire`, `smoke`, `electric`, `wet`, `hot_cold` (M16 extends to 8 with `acid`, `radiation_zone`, `toxic_atmosphere`)."

Two distinct drifts in the spec itself:

1. **`hot_cold` (single value) vs. `hot`+`cold` (two values).**
2. **`radiation_zone`/`toxic_atmosphere` vs. `radiation`/`toxic` (long vs. short names).**

**Schema uses:**

```json
"kind": { "type": "string", "enum": ["fire", "smoke", "electric", "wet", "hot", "cold", "acid", "radiation", "toxic"] }
```

9 values, splitting hot/cold and using short names. The schema sides with the **event-definition** wording on both axes, which is the right call — the event definition is the machine-readable surface, the bullet is prose commentary.

**Internal schema drift (worth fixing):**

The `description` in `hazard_spawned.json` *still references the bullet's wording* even though the enum doesn't:

```text
"5 launch hazard kinds (locked): fire, smoke, electric, wet, hot_cold. M16 extends to 9 with acid, radiation, toxic."
```

`hot_cold` is mentioned by description but does **not appear in the enum**. A reader who trusts the description will write a producer that emits `kind: "hot_cold"`, which will fail validation. This is a documentation/enum mismatch inside one file.

**Recommendation.** Edit the description of `hazard_spawned.json` (and `hazard_spread.json` for symmetry — though `hazard_spread.json`'s description does not currently include this reference) to:

> "9 hazard kinds (locked): fire, smoke, electric, wet, hot, cold, acid, radiation, toxic. The 5-launch subset for M16-launch is {fire, smoke, electric, wet, hot|cold}; M16 extends to 9 by adding acid, radiation, toxic. The spec's older 'hot_cold' single-token name is split into 'hot' and 'cold' here; the spec's 'radiation_zone' / 'toxic_atmosphere' bullet names are short-form 'radiation' / 'toxic' here."

That removes the contradiction without changing any wire shape.

### 22 affliction kinds (locked names)

**Verdict: PASS. Exact match.**

Spec list:

```text
burning, wet, electrified, poisoned, hypoxic, combustible_atmosphere, breach_decomp,
hyperthermic, hypothermic, radiation, concussed, deafened, bleeding, internal_shock,
low_battery, coolant_leaking, oil_leaking, overheating, hunger, thirst, sleep_dep, sanity_low
```

Schema enum (identical in `affliction_applied.json`, `affliction_tick.json`, `affliction_cleared.json`, `affliction_escalated.json`):

```json
["burning", "wet", "electrified", "poisoned", "hypoxic", "combustible_atmosphere",
 "breach_decomp", "hyperthermic", "hypothermic", "radiation", "concussed", "deafened",
 "bleeding", "internal_shock", "low_battery", "coolant_leaking", "oil_leaking",
 "overheating", "hunger", "thirst", "sleep_dep", "sanity_low"]
```

Count: **22** ✓. Spelling: identical ✓. Order: identical across all 4 affliction schemas ✓.

### Cosmetic flag on cosmetic events (hazard.tick, fluid.ground_splatter_spawned, affliction.tick)

**Verdict: PASS (soft).**

All three schemas declare an **envelope-level** `cosmetic` field:

```json
"cosmetic": { "type": ["boolean", "null"] }
```

(Not in `payload`.) This is correct placement — the M4 envelope owns determinism gating, not the payload.

**Caveat — not strictly enforced as `true`.** The constraint is `["boolean", "null"]`, not `const: true`. A producer could legally emit `cosmetic: false` on these events without failing validation. For schemas the spec calls cosmetic, the strictest possible form would be `"cosmetic": { "const": true }` so the validator catches a producer that miscategorises one of these events. Today's schema is lenient.

No other event in the 18 declares `cosmetic` — that's correct; the spec only flags `hazard.tick`, `fluid.ground_splatter_spawned`, and `affliction.tick` as cosmetic.

### Threshold values (50 % warning, 20 % critical, 10:1 hazard tick batch ratio)

**Verdict: PASS (documented; not numerically enforced).**

| Threshold | Where documented | Enforced? |
|---|---|---|
| 50% reservoir warning | `fluid_reservoir_warning.json` description: "Fires once when fluid reservoir level crosses 50% on the way down (HUD warning trigger)." | No (`level_pct` only constrained `minimum: 0.0`). |
| 20% reservoir critical | `fluid_reservoir_critical.json` description: "Fires once when fluid reservoir level crosses 20% on the way down (HUD critical alert)." | No (`level_pct` only constrained `minimum: 0.0`). |
| 10:1 hazard tick batching | `hazard_tick.json` description: "Batched 10:1 ratio for determinism — one tick event per 10 sim ticks, NOT every tick. The cosmetic flag excludes this from determinism.sim_checksum per DR-052." | No (the batching is a producer concern; schema cannot enforce inter-event spacing). |

These are correctly documented as producer contracts (M13 fills the fluid producer, M16 fills the hazard producer). Numeric enforcement on a per-event basis is impossible here — the validator only sees individual events, not the producer's emission cadence. Acceptable for v0.1.

---

## Recommended fixes

Ranked by impact. Item 1 is a documentation contradiction that could mislead a producer implementer; items 2-4 are tightening opportunities, not bugs.

### 1. **(Documentation drift, P1)** Fix `hazard_spawned.json` description self-contradiction.

Current description references `hot_cold` (one of the 5 launch kinds per the spec bullet) but the enum uses `hot` + `cold` as two values, never `hot_cold`. A producer reading only the description will fail validation. Rewrite the description to use the same names the enum uses; explicitly call out the spec-bullet-vs-event-definition rename.

Proposed text:

```text
"9 hazard kinds (locked): fire, smoke, electric, wet, hot, cold, acid, radiation, toxic.
 The 5-launch subset for M16-launch is {fire, smoke, electric, wet, hot|cold}; M16 extends
 to 9 by adding acid, radiation, toxic. The spec bullet's older 'hot_cold' single-token
 name is split into 'hot' and 'cold' here; the spec bullet's 'radiation_zone' /
 'toxic_atmosphere' names appear as short-form 'radiation' / 'toxic' in this enum."
```

### 2. **(Tightening, P2)** Lock `cosmetic` to `const: true` on the three cosmetic event schemas.

Currently `cosmetic` is `{"type": ["boolean", "null"]}` on `hazard.tick`, `fluid.ground_splatter_spawned`, and `affliction.tick`. Switch to:

```json
"cosmetic": { "const": true }
```

so a producer that mistakenly emits `cosmetic: false` on one of these is caught at validation time. Aligns with DR-052 (cosmetic flag excludes from `determinism.sim_checksum`).

Risk: ZERO — additive-only constraint, only catches drift.

### 3. **(Tightening, P3)** Gate `fluid.ignition`'s `fluid_kind` enum to combustibles.

The spec says "combustible only" for `fluid.ignition`. The schema's prose acknowledges this ("Fires for combustible-only fluids (fuel + leaked oil)") but the enum still allows all 4 kinds. Either:

- Tighten the enum to `["oil", "fuel"]`, OR
- Add a JSON-Schema `oneOf`/`if`/`then` constraint binding `fluid_kind` to the combustible subset.

The first form is simpler and easier to validate with the M5 minimal validator. The risk is non-zero — if M16 later decides electrolyte is also combustible (likely false), the schema would need an additive enum extension (which is allowed under M5 additive-only rules at v0.1 minor-version bumps, but still friction).

Recommendation: tighten the enum to `["oil", "fuel"]`. Add a `description` note that an additive enum extension is permitted under DR-002.

### 4. **(Tightening, P3, optional)** Add `oneOf` binding `fluid_kind` ↔ `source_module_id` in `fluid_leak_started.json`.

Today, `{fluid_kind: "oil", source_module_id: "fuel_tank"}` validates. If the producer emits a mismatched pair, the schema misses it. The four locked pairs are:

| fluid_kind | source_module_id (string form) |
|---|---|
| oil | oil_reservoir |
| coolant | coolant_pump |
| fuel | fuel_tank |
| electrolyte | power_core |

A `oneOf` block enforces the binding *when* `source_module_id` is a string. Integer form remains unconstrained (producer's internal mapping). Skip if implementation cost outweighs value — this is genuinely a producer-side contract for M13, not v0.1 schema lock work.

### 5. **(Cosmetic, P4)** Confirm the same `cosmetic` envelope-level field is added on `hazard_spread.json` if M16's design treats hazard spreads as cosmetic in addition to ticks.

Spec text on `hazard.spread` does NOT mark it cosmetic, so the current state (no `cosmetic` field on spread) is correct per spec literal. Skip unless M16 reclassifies.

---

## Summary

- **Total events audited:** 18 (9 fluid + 5 hazard + 4 affliction).
- **PASS:** 18 (100% — every schema is structurally compliant with the spec).
- **GAP (true correctness gap):** 0.
- **Documentation contradiction (P1, single-file fix):** 1 — `hazard_spawned.json` description references `hot_cold` while its own enum doesn't.
- **Tightening opportunities (P2-P4, optional):** 3 — `cosmetic: const: true`, combustible enum gate on `fluid.ignition`, and `oneOf` binding `fluid_kind` ↔ `source_module_id`.
- **Critical missing pieces:** **None.** The 18-event surface is locked, the 22-affliction enum is exact, the 4-fluid enum is exact, the reason enums on `fluid.leak_stopped`, `hazard.dissipated`, and `affliction.cleared` are exact. All cosmetic events have the cosmetic envelope flag. All 18 events are registered in the validator and exercised by the M5 conformance test.

The M5 fluid / hazard / affliction event surface is ready for the M13 + M16 producer ladders to consume without further schema work. Apply fix #1 (`hazard_spawned.json` description) to remove the only contradiction a downstream implementer might trip over.
