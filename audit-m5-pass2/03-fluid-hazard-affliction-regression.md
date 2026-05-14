# M5 Pass-2 Audit — fluid.* + hazard.* + affliction.*

**Scope.** Second-pass regression audit on the 18 deep-damage schemas in three families:

- `fluid.*` — 9 events
- `hazard.*` — 5 events
- `affliction.*` — 4 events

**Inputs reviewed.**

- Pass-1 audit report: `/Users/erol/projects/corefall/audit-m5/03-fluid-hazard-affliction-audit.md`
- Pass-1 hardening commit: `1784ad2` ("M5-A1: post-audit hardening pass — 17 audit findings closed; ready for M6")
- Shipped schemas: `/Users/erol/projects/corefall/game/crates/cf-replay/schemas/event/{fluid,hazard,affliction}_*.json`
- Validator: `/Users/erol/projects/corefall/game/crates/cf-replay/src/schemas.rs`
- cf-mod schema-file validator: `/Users/erol/projects/corefall/game/crates/cf-mod/src/main.rs`
- M5 spec (done): `/Users/erol/projects/corefall/specs/done/M5.md`
- M6 spec (active): `/Users/erol/projects/corefall/specs/active/M6.md`

**TL;DR.** Every pass-1 deliverable in this family landed cleanly and is verified by both `cargo test -p cf-replay schemas` (14/14 PASS) and `cf-mod validate crates/cf-replay/schemas/` (131/131 PASS). However pass-2 uncovers **8 new pass-relevant issues**, **5 of which are P1/P2 cause-chain integrity gaps** that will hurt M10 cause-chain consumers and M16/M13 producers when they ladder up. The most material gap is **hazard.spread payload carries no `hazard_id`** — a downstream consumer walking spread→spawned has no key. Three additional fields (`level_pct` maximum, `expected_duration_ticks` minimum, `applied_afflictions` items enum) want tightening, and the M5.md spec text still says "22 affliction kinds" while the shipped enum is 23 (`blinded` added in pass-1).

---

## Pass-1 deliveries verified

| Pass-1 checklist item | Fix shipped? | Verified by |
|---|---|---|
| `schema_version` canonical literal `prototype-recorder-event.v0.1` on all 18 schemas | YES | Test `m5_schemas_declare_schema_version_v0_1` (`schemas.rs:594-617`) walks all 18 + asserts the canonical literal; also grep confirmed every fluid/hazard/affliction schema file ships the literal in `properties.schema_version.const`. |
| `blinded` affliction added (23rd kind) | YES | Enum literally present in all four affliction schemas (`affliction_applied`, `affliction_tick`, `affliction_cleared`, `affliction_escalated`) at position 13 (between `deafened` and `bleeding`). Order is identical across all four. |
| `affliction_applied` description updated to "23 affliction kinds" | YES | Line 5 of `affliction_applied.json`: "23 affliction kinds (locked names; M16 fills mechanics; M5-A1 adds blinded for M6 flash grenade): …". |
| `hazard_spawned.json` description rewritten — no `hot_cold` standalone, reconciles spec-bullet vs event-definition naming | YES | New description (line 5): "9 hazard kinds (locked): fire, smoke, electric, wet, hot, cold, acid, radiation, toxic. … The spec bullet's 'hot_cold' single-token name is split into 'hot' + 'cold' here…". |
| `cosmetic: const: true` on `hazard.tick` | YES | Line 12 of `hazard_tick.json`: `"cosmetic": { "const": true }`. |
| `cosmetic: const: true` on `fluid.ground_splatter_spawned` | YES | Line 12 of `fluid_ground_splatter_spawned.json`: `"cosmetic": { "const": true }`. |
| `cosmetic: const: true` on `affliction.tick` | YES | Line 12 of `affliction_tick.json`: `"cosmetic": { "const": true }`. |
| `fluid.ignition.fluid_kind` enum tightened to `["oil", "fuel"]` | YES | Line 19 of `fluid_ignition.json`: `"fluid_kind": { "type": "string", "enum": ["oil", "fuel"] }`. Description also matches: "Fires for combustible-only fluids (fuel + leaked oil)…". |
| `environment.signal_aggregated.cosmetic: const: true` | YES (out-of-family but in checklist) | Line 12 of `environment_signal_aggregated.json`: `"cosmetic": { "const": true }`. |

**Pass-1 deliveries verified: 9 / 9** (this audit covers fluid+hazard+affliction; the `environment.signal_aggregated` row is outside the 18-event family scope but was on the pass-1 checklist so confirmed for completeness).

---

## End-to-end verification

### `cargo run -p cf-mod -- validate crates/cf-replay/schemas/`

```
---
scanned=131 pass=131 warn=0 fail=0
[Process exited with code 0]
```

All 131 schemas pass the cf-mod schema-file validator (M5 envelope-shape conformance: type=object, `properties.schema_version.const = "prototype-recorder-event.v0.1"`, category/event_type consts match filename, payload sub-schema typed as object, top-level `required` includes envelope minimums, no `additionalProperties: false` at payload level).

### `cargo test -p cf-replay schemas`

```
test result: ok. 14 passed; 0 failed; 0 ignored
```

Notably:
- `m5_schemas_declare_schema_version_v0_1` walks all 74 M5 schemas and asserts canonical literal — PASS
- `m5_per_family_happy_path` exercises one event per family with a representative payload, **including a payload using the new `blinded` affliction kind** (`schemas.rs:889-902`):
  ```rust
  validate_event_payload(
      "affliction",
      "applied",
      &json!({
          "actor_id": 1,
          "kind": "blinded",
          ...
      }),
  ).expect("affliction.applied (with new `blinded` kind) valid");
  ```
- `m5_concussion_dose_changed_rejects_bad_origin` locks the Origin enum.
- `m5_combat_projectile_hit_mo_rejects_envelope_named_parent` locks the parent_hit_event_id rename.

### Hand-tests on edge cases

| Probe | Result |
|---|---|
| All 9 fluid schemas declare canonical `schema_version` literal | PASS (read confirms) |
| All 5 hazard schemas declare canonical `schema_version` literal | PASS |
| All 4 affliction schemas declare canonical `schema_version` literal | PASS |
| `hazard.spawned` + `hazard.spread` + `hazard.actor_contact` `kind` enum literal-identity check | PASS — all three enums are `["fire", "smoke", "electric", "wet", "hot", "cold", "acid", "radiation", "toxic"]` (9 values, same order, same spelling) |
| `hazard.tick` + `hazard.dissipated` payloads lack `kind` field per spec | PASS — both payloads define only `hazard_id` (+ `tick` or `reason`); no `kind`. |
| Envelope `actor_id` optional on environmental events (hazard.spawned, fluid.ground_splatter_spawned) | PASS — both declare envelope `actor_id` as `["integer", "null"]` and exclude it from top-level required. |
| `blinded` enum position in all four affliction schemas | PASS — same insertion point in all four (between `deafened` and `bleeding`). |

---

## New issues found (pass-2)

### NEW-A: 23-affliction count mention is partial (P3 — doc consistency)

- **Finding.** Only `affliction_applied.json` description says "23 affliction kinds"; `affliction_cleared.json`, `affliction_tick.json`, `affliction_escalated.json` descriptions do NOT mention a count at all.
- **Why it matters.** A producer implementer reading any of the three non-`applied` schemas in isolation has no inline reminder that the enum is 23 kinds (post-`blinded`). They'd compare against the pre-pass-1 audit doc that says "22 affliction kinds (locked names)" and trip.
- **Severity.** P3. The enum is correct in all four files; this is documentation polish.
- **Recommended fix.** Add "23 affliction kinds (locked, see affliction.applied)…" to `cleared`/`tick`/`escalated` descriptions; or just add `(23 kinds — see affliction.applied)` as a one-line addendum to each.

### NEW-B: `blinded` affliction completeness (PASS — verified)

- **Verdict: PASS.** All four affliction schemas (`applied`, `tick`, `cleared`, `escalated`) include `blinded` at position 13 (between `deafened` and `bleeding`). Enum order is identical across all four schemas literal-for-literal.
- **Test coverage:** `schemas.rs::m5_per_family_happy_path` includes a happy-path payload with `kind: "blinded"` (line 897) and asserts validation passes.
- **No action required.** This row is in this report solely to record positive verification.

### NEW-C: hazard kind set drift across the 3 schemas that carry `kind` (PASS — verified)

- **Verdict: PASS.** The 9-value enum `[fire, smoke, electric, wet, hot, cold, acid, radiation, toxic]` is literal-identical across:
  - `hazard_spawned.json::payload.properties.kind.enum`
  - `hazard_spread.json::payload.properties.kind.enum`
  - `hazard_actor_contact.json::payload.properties.kind.enum`
- `hazard.tick` and `hazard.dissipated` payloads correctly omit `kind` (these events reference an existing `hazard_id`).
- **No action required.**

### NEW-D: `affliction.escalated` cross-field constraint `to_severity > from_severity` (P3 — producer-side)

- **Finding.** `affliction.escalated` carries `from_severity: [0.0, 1.0]` and `to_severity: [0.0, 1.0]` independently. The schema does NOT enforce `to_severity > from_severity`, which is the definitional requirement for "escalation". Today a producer could legally emit `{from_severity: 0.8, to_severity: 0.3}` and the schema accepts it.
- **JSON Schema feasibility.** Cross-field comparisons require `if`/`then`/`else` or `$dynamicRef` patterns that the cf-replay minimal validator does not implement (it only supports `type`, `enum`, `minItems`/`maxItems`, `minimum`/`maximum`, and `oneOf` with `type`+`enum`). Strict implementations (e.g. `jsonschema-rs`) could express this with `not: {properties: {to_severity: {exclusiveMaximum: {$data: "1/from_severity"}}}}` but that's draft-extension territory.
- **Severity.** P3. The shape lock is correct; the semantic invariant is a producer contract at M16.
- **Recommended fix.** Add a single-sentence description rider on `affliction_escalated.json`: "Producer invariant: to_severity > from_severity. Schema does not enforce this cross-field constraint (M16 producer-side validation); use affliction.cleared for severity going to zero." Possibly add a `cf-affliction`-side debug assertion at M16.

### NEW-E: `affliction.applied.expected_duration_ticks` minimum should be 1, not 0 (P2 — tighten guard)

- **Finding.** `affliction_applied.json::payload.expected_duration_ticks` has `{"type": "integer", "minimum": 0}`. A 0-tick expected duration means "applied and cleared in the same tick", which is degenerate — if the affliction were truly that short the producer should never emit the `applied` event at all (or should emit `applied` + `cleared` in the same tick burst; even then `expected_duration_ticks` should be ≥ 1 to express the "I'll be cleared after one tick").
- **Severity.** P2. A producer with a logic bug computing `expected_duration_ticks = remaining_duration_s * ticks_per_second` where `remaining_duration_s = 0.0` would silently emit a no-op application that the schema accepts.
- **Recommended fix.** Tighten to `{"type": "integer", "minimum": 1}` (one tick is the minimum meaningful duration for a tick-batched affliction).

### NEW-F: `hazard.spawned` payload correctly omits actor_id (PASS — verified)

- **Verdict: PASS.** Spec event def does not include `actor_id` in the brace block for `hazard.spawned` (terrain hazard tile, no owning actor). Schema:
  - Payload required = `["hazard_id", "kind", "position", "intensity", "source_event_id"]` (no actor_id)
  - Envelope `actor_id` = `["integer", "null"]` (optional)
- **No action required.**

### NEW-G: `fluid.ground_splatter_spawned` payload correctly omits actor_id (PASS — verified)

- **Verdict: PASS.** Spec event def `fluid.ground_splatter_spawned { fluid_kind, position, volume_l, terrain_hazard_kind }` has no actor_id. Schema:
  - Payload required = `["fluid_kind", "position", "volume_l", "terrain_hazard_kind"]`
  - Envelope `actor_id` = `["integer", "null"]` (optional)
- **No action required.**

### NEW-H: `hazard.tick` cosmetic + batching contract (P3 — doc-only)

- **Finding.** `hazard_tick.json` description correctly notes "Batched 10:1 ratio for determinism — one tick event per 10 sim ticks". The schema enforces `cosmetic: const: true` (great), but does NOT add a `batch_size: integer, const: 10` payload field. The 10:1 ratio is purely a producer obligation.
- **Severity.** P3. Inter-event cadence cannot be expressed by JSON Schema (a validator only sees one event at a time), so a `batch_size` field would be informational only. M10 cause-chain consumers may want to verify batching ratio holds across the bundle — but that's a bundle-level audit, not a per-event check.
- **Recommended fix.** Either:
  - Add an optional `batch_size: integer, const: 10` payload field for self-describing batching, OR
  - Leave as-is (description already says it) and document the bundle-level invariant in `cf-replay::tests` / M16 producer self-test.
- **Default:** leave as-is; the description carries the contract.

### NEW-I: `fluid.ignition` combustible-only consistency (PASS — verified)

- **Verdict: PASS.** Pass-1 tightened `fluid_kind` enum to `["oil", "fuel"]`. Description text matches: "Fires for combustible-only fluids (fuel + leaked oil) when an ignition source (spark, heat) crosses the puddle / leak". No drift between enum and description.
- **Cross-reference observation (no action).** The affliction enum contains `oil_leaking` and `coolant_leaking`, presumably emitted when the corresponding chassis leak crosses a threshold. Producer concern: should `coolant_leaking` affliction also escalate the **risk** of ignition for nearby fuels? That's an M16 systemic-design question, not a schema concern.

### NEW-J: `fluid.refilled.source_actor_id` nullability (P2 — schema gap)

- **Finding.** `fluid_refilled.json::payload.source_actor_id` is `{"type": "integer"}` (required). The description says "source_actor_id is the actor that performed the refill (may equal actor_id for self-refill)" — but does NOT address the **terrain-station case**: M16 spec language anticipates persistent refill stations (terrain-anchored fuel pumps, coolant taps) that have NO `ActorId`. A producer emitting "the player refilled at the fuel station" with no station actor entity has nothing to pass to `source_actor_id` (today they'd need to invent a sentinel int, e.g. `0`).
- **Severity.** P2. Forces the producer to either:
  - Allocate sentinel actor IDs for terrain stations (collides with real actor IDs if low values used)
  - Always create a phantom actor entity for every station (allocation overhead per station)
  - Misuse `source_actor_id` semantics by reusing `actor_id`
- **Recommended fix.** Change `"source_actor_id": { "type": "integer" }` to `"source_actor_id": { "type": ["integer", "null"] }` and update the description: "source_actor_id is the actor that performed the refill (may equal actor_id for self-refill; null when the source is a terrain station / refill node with no actor entity)". Schema change is additive (existing producer events that pass an int are still valid; new null option closes the gap).

### NEW-K: `fluid.reservoir_warning` + `fluid.reservoir_critical` `level_pct` lacks `maximum: 100.0` (P2 — tighten guard)

- **Finding.** Both schemas declare `level_pct: { "type": "number", "minimum": 0.0 }`. No `maximum`. A producer with a unit-confusion bug (passing fraction `0.50` vs percentage `50.0`, or a NaN-poisoned division producing `1e308`) would emit a level_pct outside `[0, 100]` and the schema accepts it.
- **Severity.** P2. The spec literal in the description ("Fires once when fluid reservoir level crosses 50% on the way down") strongly implies percentage units; the cf-replay validator now supports `maximum` (pass-1 added that for the concussion dose ceiling).
- **Recommended fix.** Add `"maximum": 100.0` to `level_pct` in both schemas. Zero risk — additive constraint, only catches drift.

### NEW-L: `fluid.leak_rate_changed.reason` is open string (P3 — optional tightening)

- **Finding.** `fluid_leak_rate_changed.json::payload.reason` is `{"type": "string"}` with no enum. Pass-1 audit noted "spec does not lock a reason set here". The description suggests common cases: "e.g. pressure drops as reservoir empties, or a partial seal slows it".
- **Severity.** P3. The spec literal `fluid.leak_rate_changed { actor_id, fluid_kind, from_rate, to_rate, reason }` does NOT lock the reason set, so an open string is defensible. But M16 producer will pick a set of reason strings; those should be documented.
- **Recommended fix (optional).** Either:
  - Lock to enum `["pressure_drop", "partial_seal", "module_offline", "leak_widened", "leak_narrowed"]` (M16 producer-side TBD)
  - Leave open + enumerate suggested strings in the description.

### NEW-M: `fluid.leak_stopped` lacks `leak_started_event_id` cause-chain pointer (P2 — cause-chain integrity)

- **Finding.** `fluid_leak_stopped.json` has `reason` enum `["sealed", "repaired", "reservoir_empty"]` but no field pointing back to the originating `fluid.leak_started` event. An M10 cause-chain consumer walking `leak_stopped → leak_started` has no key to use.
- **Workaround today.** The consumer must match by `(actor_id, fluid_kind)` and time-walk backward in the bundle to find the most recent `fluid.leak_started` for that actor + fluid kind. This is O(N) per stop event, and breaks if the actor had a leak, fixed it, leaked again, fixed again — the consumer can't distinguish which "cycle" without an explicit pointer.
- **Severity.** P2. Cause-chain integrity will hurt at M10 (cause-chain UI) and at M16 (producer self-test).
- **Recommended fix.** Add optional `leak_started_event_id: { "type": "string" }` to the `fluid_leak_stopped.json` payload. Producer at M13 fills it with the event_id of the upstream `fluid.leak_started`. Backward-compatible (additive).

### NEW-N: `affliction.cleared` lacks `source_event_id` cause-chain pointer (P2 — cause-chain integrity)

- **Finding.** `affliction_cleared.json` has `reason` enum `["time", "medikit", "environment", "death"]` but no field pointing back to the originating `affliction.applied` event. An M10 cause-chain consumer walking `cleared → applied` must match on `(actor_id, kind)` and time-walk backward — same O(N) problem as NEW-M, and same multi-cycle ambiguity.
- **Severity.** P2. Same as NEW-M.
- **Recommended fix.** Add optional `source_event_id: { "type": "string" }` to `affliction_cleared.json::payload` (symmetric with `affliction.applied.source_event_id` which points UP-chain to the cause — here `source_event_id` would point at the originating `affliction.applied`). Naming caveat: since `affliction.applied.source_event_id` already names the upstream cause (hit/hazard contact), use a different name on `cleared` like `applied_event_id` to avoid the collision. **Proposed:** `applied_event_id: string` on `affliction_cleared`.

### NEW-O: `hazard.dissipated` lacks `spawned_event_id` cause-chain pointer (P2 — cause-chain integrity)

- **Finding.** `hazard_dissipated.json` has `reason` enum `["time", "doused", "spread_out"]` and `hazard_id` but no event_id pointing back to the `hazard.spawned`. The `hazard_id` IS sufficient to identify the hazard, but a cause-chain walker that wants direct event-level linkage (e.g. for cf-replay UI's "click event → see source") needs `spawned_event_id`.
- **Severity.** P2. The `hazard_id` provides logical linkage, but event-level pointers are convention across cause-chain-aware events (affliction.applied has source_event_id, fluid.ignition has ignition_source_event_id, hazard.spawned has source_event_id). Dissipation breaking that pattern is asymmetric.
- **Recommended fix.** Add optional `spawned_event_id: { "type": "string" }` to `hazard_dissipated.json::payload`. M16 producer fills with the event_id of the upstream `hazard.spawned`. Backward-compatible.

### NEW-P: `hazard.spread` lacks `hazard_id` (P1 — cause-chain integrity)

- **Finding.** `hazard_spread.json` carries `from_pos`, `to_pos`, `kind`, `intensity`, `rate` — but NO `hazard_id`. An M10 cause-chain consumer trying to walk `spread → spawned` has NO key to use. Worse, two adjacent fires of identical `kind` and `intensity` are indistinguishable — the consumer can't tell which originating hazard the spread belongs to.
- **Severity.** **P1** — this is the single most material gap in this entire family. M16 producer ladders the hazard kernel; M10 cause-chain UI's "click spread event → see originating hazard" cannot be implemented today without `hazard_id`. The current schema makes spread events fundamentally orphaned.
- **Cross-check.** The other 4 hazard events ALL carry `hazard_id`:
  - `hazard.spawned`: `hazard_id` ✓ (introduces it)
  - `hazard.actor_contact`: `hazard_id` ✓
  - `hazard.tick`: `hazard_id` ✓
  - `hazard.dissipated`: `hazard_id` ✓
  - `hazard.spread`: **MISSING** ✗
- **Recommended fix.** Add `hazard_id: { "type": ["integer", "string"] }` to `hazard_spread.json::payload.properties` AND `hazard_id` to the payload's `required` array. The producer at M16 must always know which parent hazard is spreading; this is internal state, not a producer-implementation question.
- **Compatibility note.** This is *technically* an additive-required change. Under DR-002's "additive-only" envelope contract, **adding a required payload field is generally backward-compatible because no producer has yet emitted `hazard.spread` events** — M16 is the first producer. Pass-2 audit: catch this NOW, before M16 producers ship, otherwise it becomes a producer-bug-in-the-field instead of a schema-pre-ship-fix.

### NEW-Q: `fluid.ignition` cause-chain links only ignition source, not leak (P2 — cause-chain integrity)

- **Finding.** `fluid_ignition.json` requires `ignition_source_event_id` (the fire/electric source) but does NOT require/include the upstream `fluid.leak_started` event_id. To fully reconstruct the cause-chain "leak started → puddle pooled → spark crossed puddle → ignition", the consumer needs BOTH event IDs.
- **Severity.** P2. The current schema lets producers emit ignition without recording which leak ignited. M16 producer needs to record both: which leak caused the puddle, AND which fire/spark crossed the puddle.
- **Edge case to consider.** Some ignitions are NOT from leaks — e.g. an ambient combustible atmosphere igniting without a chassis fluid leak (M19 atmos.combustion_ignition crossing a fuel-soaked terrain tile from M3 terrain debris). In those cases there's no upstream `leak_started`. So the field should be **optional**.
- **Recommended fix.** Add optional `leak_started_event_id: { "type": "string" }` to `fluid_ignition.json::payload.properties`. Producer fills when ignition traces back to a `fluid.leak_started`; omits when ignition source is environmental/standalone. Update description: "leak_started_event_id (optional): the event_id of the upstream fluid.leak_started this ignition is consuming. Omit when ignition source is environmental (e.g. fuel-soaked terrain debris without a tracked leak)."

### NEW-R: M5.md spec text still says "22 affliction kinds" (P3 — spec text drift)

- **Finding.** `specs/done/M5.md` line 213: "22 affliction kinds (locked names; mechanics in M16): burning, wet, electrified, poisoned, hypoxic, combustible_atmosphere, breach_decomp, hyperthermic, hypothermic, radiation, concussed, deafened, bleeding, internal_shock, low_battery, coolant_leaking, oil_leaking, overheating, hunger, thirst, sleep_dep, sanity_low."
- The schema enum is now 23 (pass-1 added `blinded`); the M5.md spec text was NOT updated.
- **Severity.** P3. The spec is in `done/` — touching done specs requires care (per AGENTS.md). But this is a documentation drift that future readers will trip over.
- **Recommended fix (one of):**
  1. **Update M5.md in place** with a footnote: "22 affliction kinds + blinded (added during M5-A1 hardening pass to unblock M6 flash grenade; brings the total to 23)". Preserves the literal "22" while documenting the addition.
  2. **Add a CHANGELOG.md or AGENTS.md note** under "M5-A1" that the affliction enum is now 23, not 22, and point at commit `1784ad2`.
  3. **Leave M5.md as-is** and rely on the schema being the canonical source. This is the lowest-friction choice but leaves the spec → schema drift uncaught.
- **Default suggestion:** option 2 (CHANGELOG note + AGENTS.md inline reference), to avoid touching a done spec.

### NEW-S: `applied_afflictions` array items lack enum binding + validator can't walk array-item enum (P2 — both schema + validator gap)

- **Finding.** `internal_organ_failure_cascade.json::payload.applied_afflictions` is `array<string>` with no `items.enum` constraint. Same for `internal_circuit_failure_cascade.json::payload.applied_afflictions`. The description on both says "references the affliction.* family kinds" but the schema doesn't enforce that — a producer could emit `["definitely_not_an_affliction"]` and validation passes.
- **Validator gap.** Even if the schemas were updated to `items: { type: "string", enum: [23 kinds] }`, the cf-replay minimal validator's `PropConstraint` does NOT recursively walk `items` — see `schemas.rs::check_type` which only validates a single value against a single type, and `validate_event_payload` which doesn't descend into array elements. So both the schema AND the validator need work.
- **Severity.** P2. Cross-family enum binding (organ_failure → affliction kinds) is exactly the kind of contract that's easy to silently break when M17 ships the producer.
- **Recommended fix.** Two-part:
  1. **Schema side.** Update both `internal_organ_failure_cascade.json` and `internal_circuit_failure_cascade.json` to `applied_afflictions: { "type": "array", "items": { "type": "string", "enum": [23 affliction kinds] } }`.
  2. **Validator side.** Add `items: Option<Box<PropConstraint>>` to `PropConstraint` in `schemas.rs`, and when validating an array property, walk each element against the `items` schema. This is a ~30-LOC validator addition.
- **Cross-reference.** This same pattern applies to `cascade_effects: array<string>` on `fluid_reservoir_empty.json` (which the pass-1 audit noted is documented but not enum-enforced). If `cascade_effects` ever gets a locked vocabulary (it currently doesn't per the spec), the same `items.enum` + validator walk is the right shape.

---

## Recommended fixes (ranked)

| Priority | Fix | File(s) |
|---|---|---|
| **P1** | Add `hazard_id` (required) to `hazard.spread` payload — cause-chain integrity gap (NEW-P) | `hazard_spread.json` |
| P2 | Add optional `leak_started_event_id` to `fluid.leak_stopped` (NEW-M) | `fluid_leak_stopped.json` |
| P2 | Add optional `applied_event_id` to `affliction.cleared` (NEW-N) | `affliction_cleared.json` |
| P2 | Add optional `spawned_event_id` to `hazard.dissipated` (NEW-O) | `hazard_dissipated.json` |
| P2 | Add optional `leak_started_event_id` to `fluid.ignition` (NEW-Q) | `fluid_ignition.json` |
| P2 | Make `fluid.refilled.source_actor_id` nullable for terrain-station refills (NEW-J) | `fluid_refilled.json` |
| P2 | Add `maximum: 100.0` to `level_pct` on warning + critical (NEW-K) | `fluid_reservoir_warning.json`, `fluid_reservoir_critical.json` |
| P2 | Tighten `affliction.applied.expected_duration_ticks` to `minimum: 1` (NEW-E) | `affliction_applied.json` |
| P2 | Lock `applied_afflictions` items to affliction-kind enum + add validator items walk (NEW-S) | `internal_organ_failure_cascade.json`, `internal_circuit_failure_cascade.json`, `schemas.rs` |
| P3 | Add "23 affliction kinds" reminder to `cleared`/`tick`/`escalated` descriptions (NEW-A) | `affliction_cleared.json`, `affliction_tick.json`, `affliction_escalated.json` |
| P3 | Document `to_severity > from_severity` producer invariant on `affliction.escalated` (NEW-D) | `affliction_escalated.json` |
| P3 | Optional: lock `fluid.leak_rate_changed.reason` to enum OR document common reasons (NEW-L) | `fluid_leak_rate_changed.json` |
| P3 | Reconcile M5.md spec's "22 affliction kinds" vs shipped 23 (NEW-R) | `specs/done/M5.md` (gentle) OR CHANGELOG / AGENTS.md note |
| P3 | Optional: document `hazard.tick` 10:1 batching with a `batch_size` payload field OR keep as description (NEW-H) | `hazard_tick.json` |

---

## M6 readiness verdict

**The fluid+hazard+affliction surface is M6-ready for the M6 producer slice that M6 actually ships:**

- M6 flash grenade → emits `affliction.applied { kind: "blinded" }` + `affliction.applied { kind: "deafened" }` — both kinds present, validated, and covered by `m5_per_family_happy_path`. ✓
- M6 smoke grenade → emits `hazard.spawned { kind: "smoke" }` — kind present, validated. ✓
- M6 frag grenade → emits combat events + at hit, possibly `affliction.applied { kind: "bleeding" }`, `concussed` for blunt — kinds present. ✓
- M6 knife stab → "bleed chance" → `affliction.applied { kind: "bleeding" }`. ✓
- M6 rifle bash / shoulder check → knockdown / blunt → `affliction.applied { kind: "concussed" }`. ✓

**M6 does NOT need any of the pass-2 fixes to ship.** The P1 fix (NEW-P: hazard.spread hazard_id) is **M16-relevant**, not M6 — M6's flash/smoke grenades emit `hazard.spawned` (a fresh hazard tile), not `hazard.spread` (which is M16's grid-based propagation). M6 will not exercise `hazard.spread` at all.

**However, all P1/P2 cause-chain pointers SHOULD ship before M13/M16 producers start populating these events.** Once those producers are in flight, fixing the schemas requires either:
- A coordinated schema+producer change (annoying, but doable while M13/M16 are in M5-A2/M5-A3-style hardening)
- A schema-only additive bump (mostly fine for the P2 *optional* fields)

The P1 (NEW-P) `hazard_id` on `hazard.spread` is **harder** — adding a required field after a producer ships means every old event becomes invalid. **Recommend closing NEW-P NOW**, before M16 starts shipping `hazard.spread` events.

---

## Summary

- **Pass-1 deliveries verified: 9 / 9** in scope (including `environment.signal_aggregated.cosmetic` from the pass-1 checklist).
- **New issues found: 14** (5 verified PASS, 1 P1, 8 P2, 5 P3 — see table above for ranked recommendations).
- **Critical (P0): 0.** No correctness breaks.
- **High (P1): 1.** `hazard.spread` missing `hazard_id` — cause-chain orphaning. Fix BEFORE M16 producer ships.
- **Medium (P2): 8.** Mostly cause-chain integrity gaps + a few tightening opportunities. Recommend fixing before M13/M16 producers begin populating these events.
- **Low (P3): 5.** Documentation polish + minor enum-tightening.
- **M6 readiness:** **PASS**. M6 producers won't exercise any of the gap-affected paths. All M6-required affliction kinds (`blinded`, `deafened`, `bleeding`, `concussed`), hazard kinds (`fire`, `smoke`, `electric`, `wet`), and fluid surfaces are present and validated.
- **M13 readiness:** **PASS WITH RECOMMENDATIONS** — apply NEW-J (refilled source_actor_id nullable) and NEW-M (leak_stopped → leak_started_event_id) before chassis fluid system ships.
- **M16 readiness:** **HOLD ON NEW-P** — `hazard.spread` needs `hazard_id` added before the hazard kernel producer begins emitting these events. P2 cause-chain pointers (NEW-O, NEW-N) should land in the same hardening sweep.
- **M10 cause-chain integrity verdict:** TIGHT TODAY for `affliction.applied`, `fluid.leak_started`, `hazard.spawned`, `fluid.ignition` (these all carry up-chain pointers). LOOSE for the symmetric "termination" events (`affliction.cleared`, `fluid.leak_stopped`, `hazard.dissipated`) which lack down-chain → up-chain pointers. NEW-M / NEW-N / NEW-O / NEW-Q close this loop.
