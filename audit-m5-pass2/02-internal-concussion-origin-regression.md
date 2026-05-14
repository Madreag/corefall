# M5 Pass-2 Audit — `internal.*` + `concussion.*` + `internal_shock.*` + `origin.*`

**Pass-1 commit under audit:** `1784ad2 M5-A1: post-audit hardening pass`
**Schema source (pass-2):** `game/crates/cf-replay/schemas/event/`
**Validator source (pass-2):** `game/crates/cf-replay/src/schemas.rs`
**Spec:** `specs/done/M5.md` § `internal.* family`, `concussion.* + internal_shock.* family`, `origin.* family`
**M17 reference:** `specs/active/M17.md` (the producer milestone that will fill the matrix)

---

## Scope reminder (16 schemas + 1 snapshot)

| # | Family | Schema file | Pass-1 verdict (from `audit-m5/02-…audit.md`) |
|---|---|---|---|
| 1 | `internal.*` | `internal_organ_damaged.json` | PASS |
| 2 | `internal.*` | `internal_organ_destroyed.json` | PASS |
| 3 | `internal.*` | `internal_organ_failure_cascade.json` | PASS |
| 4 | `internal.*` | `internal_circuit_damaged.json` | PASS |
| 5 | `internal.*` | `internal_circuit_destroyed.json` | PASS |
| 6 | `internal.*` | `internal_circuit_failure_cascade.json` | PASS |
| 7 | `concussion.*` | `concussion_dose_changed.json` | PASS |
| 8 | `concussion.*` | `concussion_band_changed.json` | PASS |
| 9 | `concussion.*` | `concussion_ko_threshold_crossed.json` | PASS |
| 10 | `concussion.*` | `concussion_recovered.json` | PASS |
| 11 | `internal_shock.*` | `internal_shock_dose_changed.json` | PASS |
| 12 | `internal_shock.*` | `internal_shock_module_damaged.json` | PASS |
| 13 | `origin.*` | `origin_shot_force_feedback.json` | GAP (soft) — pass-1 fixed several constraints |
| 14 | `origin.*` | `origin_g_load_dose_changed.json` | PASS |
| 15 | `origin.*` | `origin_helmet_breach.json` | PASS |
| 16 | `origin.*` | `origin_oxygen_supply_changed.json` | PASS (soft) — open `source` string in pass-1 |
| 17 | snapshot | `snapshot_origin.json` | Cross-family drift flagged in pass-1 |

---

## Pass-1 deliveries — verified ON THIS COMMIT (`1784ad2`)

| Pass-1 fix | Status | Concrete proof |
|---|---|---|
| **schema_version canonical literal on all 16 schemas (+ snapshot)** | **VERIFIED PASS** | Every in-scope event schema reads `"schema_version": { "const": "prototype-recorder-event.v0.1" }`. `m5_schemas_declare_schema_version_v0_1` test in `schemas.rs:783-859` enforces it across the full 74-schema M5 set. `m5_event_schema_rejects_legacy_short_literal` (cf-mod) locks the validator side. |
| **Origin enum locked via `oneOf{integer ∣ string-enum}` on `concussion.dose_changed.origin_id`** | **VERIFIED PASS** | `concussion_dose_changed.json:21-26` has the literal `oneOf` shape with enum `["Human", "Android", "Robot", "PoweredOrganic", "HeavyBiomech"]`. Probe `[A]` REJECTs `"Construct"`; probe `[B]` ACCEPTs `"Human"`; probe `[C]` ACCEPTs integer 5. |
| **Origin enum locked on `origin.shot_force_feedback.origin_id`** | **VERIFIED PASS** | `origin_shot_force_feedback.json:28-33` has identical `oneOf` shape. Probe `[H]` REJECTs `"Construct"`; probe `[I]` ACCEPTs `"HeavyBiomech"`. |
| **`snapshot_origin.json` description sheds `Construct` + `HeavyBioMech`** | **VERIFIED PASS** | `snapshot_origin.json:16` reads `... origin_id: Human\|Android\|Robot\|PoweredOrganic\|HeavyBiomech ...`. `Construct` and `HeavyBioMech` strings are gone; line adds the clarifying note "5 values match concussion.dose_changed.origin_id + origin.shot_force_feedback.origin_id." |
| **`maximum: 100` on `concussion.dose_changed.from_dose`** | **VERIFIED PASS** | `concussion_dose_changed.json:19` — `"from_dose": { "type": "number", "minimum": 0.0, "maximum": 100.0 }`. |
| **`maximum: 100` on `concussion.dose_changed.to_dose`** | **VERIFIED PASS** | `concussion_dose_changed.json:20`. Probe `[D]` (to_dose=150) REJECTed; probe `[E]` (to_dose=100, boundary) ACCEPTed. |
| **`maximum: 100` on `concussion.band_changed.dose`** | **VERIFIED PASS** | `concussion_band_changed.json:21` — `"dose": { "type": "number", "minimum": 0.0, "maximum": 100.0 }`. Probe `[F]` (dose=100) ACCEPTed; probe `[G]` (dose=101) REJECTed. |
| **`origin.shot_force_feedback.chassis_layer` enum (surface_kind taxonomy)** | **VERIFIED PASS** | `origin_shot_force_feedback.json:34` has the 8-value enum `["armor_external", "armor_internal", "armor_core", "armor_chunked_breach", "flesh", "circuit", "unarmored", "terrain"]`. Probe `[J]` ACCEPTs `"armor_external"`; probe `[K]` REJECTs `"skin"`. |
| **`origin.oxygen_supply_changed.source` enum tightened** | **VERIFIED PASS** | `origin_oxygen_supply_changed.json:21` — `"source": { "type": "string", "enum": ["helmet_breach", "refilled", "exhaled", "atmosphere"] }`. Probe `[M]/[N]/[O]` all REJECT (`tank_swap`, `external_supply`, `death`). |
| **cf-replay validator: `oneOf` support** | **VERIFIED PASS** | `schemas.rs:171-176` declares `one_of: Option<Vec<Value>>` on `PropConstraint`; `schemas.rs:264-283` walks each branch and reports `"did not satisfy any oneOf branch — branch[…]"`. Helper `check_one_of_branch` at `schemas.rs:288-303` validates `type` + `enum` per branch. |
| **cf-replay validator: `maximum` support** | **VERIFIED PASS** | `schemas.rs:165` declares `maximum: Option<f64>`; `schemas.rs:255-261` enforces `n > max` → error. Probe `[D]/[G]` confirm. |
| **Validator accepts BOTH canonical + legacy short-form literal during migration** | **VERIFIED PASS** | `schemas.rs:218-225` — `matches!(sv, Some("prototype-recorder-event.v0.1") | Some("0.1"))`. cf-mod enforces canonical-only via `validate_event_schema_value`; cf-replay is lenient on the marker. |

**Result: 11 / 11 pass-1 deliveries land correctly on commit `1784ad2`.** No regressions detected. The cf-replay test suite (`cargo test -p cf-replay --lib schemas::`) passes 14 / 14 schema tests including the new `m5_concussion_dose_changed_rejects_bad_origin` lock.

---

## New issues found (pass-2)

### NEW-A — `internal_shock.dose_changed` is missing `origin_id`  *(P2 — defensive)*

**Schema file:** `internal_shock_dose_changed.json`
**Current required set:** `["actor_id", "from_dose", "to_dose", "source_event_id"]`
**Spec (M5):** `internal_shock.dose_changed { actor_id, from_dose, to_dose, source_event_id }` — `origin_id` is NOT listed.

**Why this is still worth raising:** the cousin event `concussion.dose_changed` carries `origin_id` because per-origin decay rates differ (Human 5/s vs HeavyBiomech 4/s). The robot-side equivalent `internal_shock.dose_changed` ALSO has a decay rate that varies — Robot 2/s — but `internal_shock` is documented as **Robot-only** (Heavy biomech tracks the concussion-equivalent decay, not internal_shock). Per the spec table:

| Origin | Concussion decay | Internal-shock decay |
|---|---|---|
| Human | 5/s | n/a |
| Android | 5/s | n/a |
| Robot | 0/s (always 0) | 2/s |
| Powered organic | 5/s | n/a |
| Heavy biomech | 4/s | n/a |

So `internal_shock.dose_changed` is implicitly Robot-only and `origin_id` is unnecessary. **No drift from spec — recommend adding a description clause stating "Robot-only event; never fires for Human/Android/PoweredOrganic/HeavyBiomech actors"** so M17 producer code reads the gating contract from the schema.

**Recommended description amendment** (additive-only, no field change):

> "M5 § internal_shock.* family — declarative event schema locked at v0.1. **Robot-only event** (the Origin enum's other 4 origins use `concussion.dose_changed` for their accumulator; HeavyBiomech tracks the concussion-equivalent decay at 4/s). Decay rate (locked): Robot 2/s. Producer fills at M17. Additive-only per M4 DR-002."

---

### NEW-B — `internal_shock.dose_changed` missing dose maximum  *(P2)*

**Schema file:** `internal_shock_dose_changed.json:18-19`
**Current shape:** `"from_dose": { "type": "number", "minimum": 0.0 }`, same for `to_dose`. **No maximum.**

**Probe `[P]` evidence:** `to_dose = 500.0` is ACCEPTed by the validator.

**Spec:** M5 does not lock an explicit maximum for `internal_shock` dose. By analogy with the concussion accumulator (5-band 0..100 ceiling) and the parallel KO-equivalent for robot circuits, the internal_shock accumulator is presumably also 0..100. Pass-1 added `maximum: 100` to concussion dose; the symmetry argument is identical for `internal_shock`.

**Risk under M14/M17 producer load:** a robot taking sustained close-range RPG hits could plausibly emit `to_dose = 250.0` if the producer fails to clamp. Pass-1 already proved the validator catches it for `concussion`; it should be symmetric for `internal_shock`.

**Recommended fix (P2):**

```diff
-        "from_dose": { "type": "number", "minimum": 0.0 },
-        "to_dose": { "type": "number", "minimum": 0.0 },
+        "from_dose": { "type": "number", "minimum": 0.0, "maximum": 100.0 },
+        "to_dose": { "type": "number", "minimum": 0.0, "maximum": 100.0 },
```

If M17 owner decides internal_shock has a different ceiling, adjust to that value; but the schema should pin SOME maximum.

---

### NEW-C — `internal_shock.module_damaged.module_id` is open `["integer", "string"]` while spec implies it overlaps the 12-circuit enum  *(P2)*

**Schema file:** `internal_shock_module_damaged.json:18`
**Current shape:** `"module_id": { "type": ["integer", "string"] }` — no enum.

**Probe `[Q]` evidence:** `module_id = "arbitrary_garbage_string"` is ACCEPTed.

**Cross-reference:**

- `internal.circuit_damaged.circuit_id` is **enum-locked** to the 12-circuit graph (`power_core, cpu, sensor_array, motor_controller_*4, hydraulic_pump, coolant_pump, oil_reservoir, fuel_tank, comm_relay`).
- `origin.shot_force_feedback.internal_shock_module_id` is `["integer", "string", "null"]` — same open shape as `internal_shock.module_damaged.module_id`. **Both robot-internal "module" references skip the enum**, but the canonical circuit names are already locked elsewhere.
- M5 spec for `internal_shock.module_damaged`: *"Fires when an internal robot module takes damage from an internal shock event (e.g. routed surge from circuit damage). hit_zone is the BodyZone the source ray entered."* — the wording "internal robot module" semantically aligns with "circuit" from the 12-circuit graph.

**The drift:** if `internal_shock.module_damaged.module_id` is supposed to identify the circuit (12-circuit enum), it should be enum-locked. If it's a different taxonomy (chassis module ≠ circuit), spec needs to enumerate it.

**Recommended action (P2):** clarify with M17 owner. Default conservative recommendation:

```diff
-        "module_id": { "type": ["integer", "string"] },
+        "module_id": {
+          "oneOf": [
+            { "type": "integer" },
+            { "type": "string", "enum": ["power_core", "cpu", "sensor_array", "motor_controller_left_arm", "motor_controller_right_arm", "motor_controller_left_leg", "motor_controller_right_leg", "hydraulic_pump", "coolant_pump", "oil_reservoir", "fuel_tank", "comm_relay"] }
+          ]
+        },
```

Same change applies symmetrically to `origin.shot_force_feedback.internal_shock_module_id` (extend the existing `["integer", "string", "null"]` to a `oneOf` carrying the same 12-circuit enum + `"null"`).

If M17 owner says "module" is a SEPARATE taxonomy (e.g. armor compartments, not circuits), then the spec must enumerate the module IDs as a parallel locked list. Without that, M17 producers will drift on their preferred string spelling.

---

### NEW-D — `organ_kind` and `circuit_kind` are open strings with no spec enumeration  *(P2)*

**Affected fields:**

- `internal.organ_damaged.organ_kind` — `internal_organ_damaged.json:21`
- `internal.organ_destroyed.organ_kind` — `internal_organ_destroyed.json:20`
- `internal.circuit_damaged.circuit_kind` — `internal_circuit_damaged.json:20`
- `internal.circuit_destroyed.circuit_kind` — `internal_circuit_destroyed.json:20`

**Current shape:** all four are `"type": "string"` with no enum.

**Spec:** M5 lists `organ_kind` and `circuit_kind` as bare fields with no taxonomy. The 15-organ / 12-circuit `id` lists are locked, but the "kind" classification (vital / sensory / digestive / etc.) is unspecified.

**Pass-1 test evidence (line 1058):** `m5_per_family_happy_path` uses `"organ_kind": "vital"` — a single ad-hoc value that no spec defines.

**Risk under M14/M17 producer load:** producer code will invent "vital" / "sensory" / "limb" / "musculoskeletal" / etc. inconsistently. If consumers (HUD, post-mortem recap, damage log) try to group damage by `organ_kind`, they'll see drift.

**Recommended action (P2):** either

1. Drop `organ_kind` / `circuit_kind` entirely (the 15/12-name enum already classifies each organ implicitly), **or**
2. Lock a kind enum. Suggested taxonomy for organs (5 kinds):
   - `vital_neural` (brain, spine)
   - `vital_circulatory` (heart)
   - `vital_respiratory` (lungs_left, lungs_right)
   - `vital_metabolic` (liver, kidneys_left, kidneys_right, stomach, intestines, pancreas)
   - `sensory` (eyes_left, eyes_right, ears_left, ears_right)

   Suggested taxonomy for circuits (4 kinds):
   - `power_distribution` (power_core)
   - `compute` (cpu, sensor_array, comm_relay)
   - `actuator` (motor_controller_*4, hydraulic_pump)
   - `support` (coolant_pump, oil_reservoir, fuel_tank)

This is M17-owner-territory. The schema today doesn't catch drift.

---

### NEW-E — `failure_cascade` and `applied_afflictions` arrays are open `array of string`  *(P1 — affects M17 producer drift)*

**Affected fields:**

- `internal.organ_destroyed.failure_cascade` — `internal_organ_destroyed.json:23-26`
- `internal.circuit_destroyed.failure_cascade` — `internal_circuit_destroyed.json:23-26`
- `internal.organ_failure_cascade.applied_afflictions` — `internal_organ_failure_cascade.json:22-25`
- `internal.circuit_failure_cascade.applied_afflictions` — `internal_circuit_failure_cascade.json:21-24`

**Current shape:**

```json
"failure_cascade": {
  "type": "array",
  "items": { "type": "string" }
}
```

— no `enum` on the item type. Same shape for `applied_afflictions`.

**Probe `[R]` evidence:** `failure_cascade: ["not_an_affliction", "neither_is_this"]` is ACCEPTed by the validator.

**Spec cross-reference:**

- M5 § `affliction.*` family locks 23 affliction kinds (post-pass-1 with `blinded` added for M6 flash grenade): `burning, wet, electrified, poisoned, hypoxic, combustible_atmosphere, breach_decomp, hyperthermic, hypothermic, radiation, concussed, deafened, blinded, bleeding, internal_shock, low_battery, coolant_leaking, oil_leaking, overheating, hunger, thirst, sleep_dep, sanity_low`.
- `internal.organ_failure_cascade` description literally says: *"applied_afflictions references the affliction.* family kinds"* — so this IS the affliction enum.
- `internal.circuit_failure_cascade` description says: *"applied_afflictions references affliction.* family kinds (e.g. low_battery, coolant_leaking)"* — same.

**Risk under M17 producer load:** the M17 origin-reaction-matrix owner will spell affliction names. If they typo `low_batter` or invent `motor_seized` (not in the 23-affliction list), the schema lets it through. Downstream consumers (HUD, run-bundle replay, recap renderer) silently drop unknown kinds.

**`failure_cascade` is more ambiguous** — the description says "afflictions / hp drains" so the array might mix affliction kinds with hp-drain identifiers. If the schema can't pin it down to a single taxonomy, at minimum `applied_afflictions` should be locked.

**Recommended fix (P1):** lock `applied_afflictions` to the 23-affliction enum (both organ + circuit variants):

```diff
-        "applied_afflictions": {
-          "type": "array",
-          "items": { "type": "string" }
-        },
+        "applied_afflictions": {
+          "type": "array",
+          "items": { "type": "string", "enum": ["burning", "wet", "electrified", "poisoned", "hypoxic", "combustible_atmosphere", "breach_decomp", "hyperthermic", "hypothermic", "radiation", "concussed", "deafened", "blinded", "bleeding", "internal_shock", "low_battery", "coolant_leaking", "oil_leaking", "overheating", "hunger", "thirst", "sleep_dep", "sanity_low"] }
+        },
```

For `failure_cascade`: clarify the taxonomy with M17 owner first. If it's pure affliction names, lock it; if it's mixed (affliction names + hp-drain ids), document the mixed semantics in the description and leave the type open (or add a `oneOf` per item).

**Validator note:** the current cf-replay validator does NOT walk `items` constraints inside arrays — `check_type` only verifies the outer `type: array`. Even if the schema adds the enum to `items`, the validator wouldn't enforce it without an additional pass. Pass-1 chose not to extend the validator for items-enum; pass-2 should flag that gap.

---

### NEW-F — `concussion.recovered` does NOT carry `origin_id`; cannot gate Robot misuse  *(P3 — defensive)*

**Schema file:** `concussion_recovered.json:18-23`
**Current required:** `["actor_id", "recovery_reason"]` — no `origin_id`.

**Spec gating:** Robot has 0 concussion dose by design (always 0). So `concussion.recovered` should never fire for a Robot actor. The schema does not enforce this.

**Probe `[T]/[U]` evidence:** validator ACCEPTs `concussion.recovered` for ANY actor without any origin check.

**Risk under M17 producer load:** a buggy producer that runs the concussion accumulator on robot actors would emit `concussion.recovered` for them — false-positive in damage analytics.

**Recommended action (P3):** at minimum, add a description clause: *"Never fires for Robot-origin actors (concussion dose is always 0 by design; see §5 of M5 spec)."* Optionally add `origin_id` to the payload to enable schema-level gating; this would be a SPEC EXTENSION (M5 does not list `origin_id` for `concussion.recovered`) and would need M17 owner sign-off.

---

### NEW-G — 4 nullable fields on `origin.shot_force_feedback`: description lacks explicit "populated when" clause  *(P3 — documentation)*

**Pass-1 audit recommendation #4 was:** *"add a description note: 'Populated only when origin is Robot (...) or actor has a fluid-bearing module (...)'."* Pass-1 closed the validator-side (kept nullable, kept out of `required`) but the description was NOT explicitly amended with the "populated only when" clause.

**Current description (`origin_shot_force_feedback.json:5`):**

> "M5 § origin.* family — declarative event schema locked at v0.1. Per-origin reaction matrix output for an incoming hit. feedback_kind: pain_jolt (organic), servo_jolt (android/robot), frame_ring (heavy biomech). impulse_vector is the 2D shock direction. internal_shock_module_id + internal_shock_damage route to robot circuits. leak_channel + leak_rate fold into fluid.leak_started. Producer fills at M17. Additive-only per M4 DR-002."

**Gap:** the description tells consumers what the fields do but does NOT say "these are null when origin != Robot" or "these are null when no fluid module is hit". A consumer reading the schema cold cannot tell from the description that a Human-origin event will carry `internal_shock_module_id: null`.

**Recommended description amendment (P3):**

> "...internal_shock_module_id + internal_shock_damage route to robot circuits **(non-null ONLY when origin_id is Robot)**. leak_channel + leak_rate fold into fluid.leak_started **(non-null ONLY when the hit punctured a fluid-bearing module: oil/coolant/fuel/electrolyte channel)**. Producer fills at M17."

---

### NEW-H — `origin.shot_force_feedback.leak_channel` is open string + null; should align with `fluid.leak_started.fluid_kind`  *(P2)*

**Schema file:** `origin_shot_force_feedback.json:42`
**Current shape:** `"leak_channel": { "type": ["string", "null"] }` — open string.

**Cross-reference:** `fluid.leak_started.fluid_kind` is enum-locked to `["oil", "coolant", "fuel", "electrolyte"]` (`fluid_leak_started.json:21`). The `origin.shot_force_feedback.leak_channel` field is semantically a forward-pointer to `fluid.leak_started` (per the description: "leak_channel + leak_rate fold into fluid.leak_started"). The two enums MUST agree.

**Risk:** M17 producer might spell `leak_channel: "hydraulic"` when emitting `origin.shot_force_feedback`, then emit `fluid.leak_started{fluid_kind: "oil"}` on the same hit. Cross-event drift; consumers can't correlate.

**Recommended fix (P2):**

```diff
-        "leak_channel": { "type": ["string", "null"] },
+        "leak_channel": {
+          "oneOf": [
+            { "type": "string", "enum": ["oil", "coolant", "fuel", "electrolyte"] },
+            { "type": "null" }
+          ]
+        },
```

This forces leak_channel to use the SAME 4-fluid taxonomy as `fluid.leak_started`, removing the cross-event drift risk.

---

### NEW-I — `origin.helmet_breach.helmet_item_id` is `type: integer` — verified consistent with `item_id` everywhere else  *(VERIFIED — no action)*

**Cross-reference scan** (`grep "item_id|weapon_id|projectile_id"` over schemas/event/):

- All 15 `armor.*` schemas: `"item_id": { "type": "integer" }`. ✓
- `origin.helmet_breach.helmet_item_id`: `"type": "integer"`. ✓
- `combat.projectile_hit_mo.weapon_id`: `"type": "integer"`. ✓
- `combat.projectile_hit_mo.projectile_id`: `"type": "integer"`. ✓
- `terrain.terrain_penetration_threshold.projectile_id`: `"type": "integer", "minimum": 0`. ✓

The only outlier is `actor.inventory_dropped.item_id`: `"type": "string"` — but that's a M0 / cfctl-side schema, not in the M5 deep-damage scope. Not in pass-2 scope.

**Verdict: NO ACTION.** `helmet_item_id` is consistent with the dominant integer-typed pattern.

---

### NEW-J — `concussion.ko_threshold_crossed.ko_duration_s` lacks the 5..10 range constraint  *(P2)*

**Schema file:** `concussion_ko_threshold_crossed.json:19`
**Current shape:** `"ko_duration_s": { "type": "number", "minimum": 0.0 }` — no `maximum`, and `minimum` is `0.0` (not `5.0`).

**Spec lock:** M5 § concussion table — *"KO | 100 | full blackout 5-10s"*.

**Probe `[S]` evidence:** validator ACCEPTs `ko_duration_s = 3.0` (outside the 5..10 spec window). This is a drift the validator cannot catch.

**Recommended fix (P2):**

```diff
-        "ko_duration_s": { "type": "number", "minimum": 0.0 }
+        "ko_duration_s": { "type": "number", "minimum": 5.0, "maximum": 10.0 }
```

**Edge case:** M16 + M17 might want to extend KO duration for severe events (KO from explosive head shot = longer KO). If the M17 owner needs an extended range, they can lift the maximum at that point. Today's 5..10 is the spec lock; the schema should match.

---

### NEW-K — Cross-field constraint: `feedback_kind` MUST agree with `origin_id` — NOT enforced  *(P2 — cross-spec drift detected)*

**Schema file:** `origin_shot_force_feedback.json:34-35`
**Current shape:**

```json
"origin_id": { "oneOf": [ { "type": "integer" }, { "type": "string", "enum": [5 values] } ] },
"feedback_kind": { "type": "string", "enum": ["pain_jolt", "servo_jolt", "frame_ring"] }
```

**M5 spec lock** (`origin.* family` event list):

> `feedback_kind: 'pain_jolt'|'servo_jolt'|'frame_ring'`

**M5 description text** (`origin_shot_force_feedback.json:5`):

> "feedback_kind: pain_jolt (organic), servo_jolt (android/robot), frame_ring (heavy biomech)"

**Probe `[L]` evidence:** validator ACCEPTs `origin_id: "Robot", feedback_kind: "pain_jolt"` — a clearly invalid pair per the prose.

**Cross-spec drift** (M5 vs M17):

M5 schema description maps:
- pain_jolt → organic (Human / PoweredOrganic)
- servo_jolt → android / robot
- frame_ring → heavy biomech

M17 spec § "Origin reaction matrix" table (line 238 in M17):
- `robot` row: *"Servo jolt **+ frame ring**"* — implies robot can emit BOTH servo_jolt and frame_ring per hit (or per impact severity tier?). The M5 schema description claims frame_ring is HeavyBiomech-only, but M17 says robot also rings.

This is a **real cross-spec drift** between M5 and M17. Pass-1 didn't flag it because the M5-only audit didn't open M17.

**Recommended action (P2):**

1. **Reconcile the spec drift first.** Either:
   - Amend M5 description to reflect that frame_ring fires for BOTH Robot and HeavyBiomech (per M17), OR
   - Amend M17 § Origin reaction matrix row "robot" to read "Servo jolt" (drop "+ frame ring") if M5's stricter mapping is canonical.

2. **After reconciling the spec, enforce in schema** via JSON Schema `if/then/else`:

   ```json
   "allOf": [
     {
       "if": { "properties": { "origin_id": { "type": "string", "const": "Human" } } },
       "then": { "properties": { "feedback_kind": { "const": "pain_jolt" } } }
     },
     {
       "if": { "properties": { "origin_id": { "type": "string", "const": "Robot" } } },
       "then": { "properties": { "feedback_kind": { "enum": ["servo_jolt", "frame_ring"] } } }
     },
     ...
   ]
   ```

3. **Validator gap:** cf-replay's minimal validator does NOT support `if/then/else`. Adding cross-field constraints to the schema would not be enforced by the validator (cf-mod's static schema-file checker can do it via a full draft-2020-12 implementation, but cf-replay's runtime validator would skip it).

   Two options:
   - (a) Encode in schema for documentation; live with no runtime enforcement at cf-replay layer; let M17's producer code own the gating.
   - (b) Extend cf-replay validator to support `if/then/else` (significant work; would require carrying a real JSON Schema crate or hand-rolled implementation).

**Recommendation:** spec-side reconciliation FIRST. Schema-side encoding is secondary.

---

### NEW-L — Dose-to-band consistency is not schema-checkable; document producer-side responsibility  *(P3)*

**Spec lock** (M5 § concussion):

| Band | Dose threshold |
|---|---|
| Clear | 0-20 |
| Mild | 20-40 |
| Moderate | 40-60 |
| Severe | 60-80 |
| KO_Imminent | 80-99 |
| KO | 100 |

**Schema status:** dose-to-band mapping is documentation-text-only on `concussion_band_changed.json:5`. Pass-1 did not encode it as constraint.

**Producer-side responsibility:** when `concussion.dose_changed.to_dose` crosses a band boundary (19.9 → 20.1), a paired `concussion.band_changed` event MUST fire same-tick. The schema cannot enforce cross-event firing.

**Risk under M17 producer load:** a producer that updates dose but forgets to emit the band transition will silently drift; downstream HUD logic that snapshots band state will miss transitions.

**Recommended action (P3):** ADD a description clause to BOTH `concussion_dose_changed.json` AND `concussion_band_changed.json`:

> "Producer contract: when `to_dose` crosses a band boundary (0-20-40-60-80-99-100), a paired `concussion.band_changed` event MUST fire on the same tick with consistent `dose` matching `to_dose`. Schema does not enforce; M17 producer is responsible."

This is purely defensive; M17 acceptance criteria should call out the pairing.

---

### NEW-M — HUD cue mapping is description-text-only; M11 HUD producer will need machine-readable form  *(P3 — deferred decision)*

**Pass-1 audit recommendation #6 flagged this and pass-1 chose to leave alone.**

**Current state (`concussion_band_changed.json:5`):** HUD cue table embedded in description string:

> "HUD cue per band: Clear=none, Mild=edge vignette 10%, Moderate=vignette 30% + bloom, Severe=vignette 60% + sway, KO_Imminent=vignette 85% + tunnel, KO=full blackout 5-10s."

**Pass-2 take:** M6 spec does NOT directly consume this (M6 ships actor controller + equipment, not HUD vignette tuning). M11 (HUD milestone, not yet read in scope) is the consumer. The mapping is locked in M5 spec prose; M11 implementer will read it from the spec, not from an event field.

**Recommendation:** **NO ACTION at pass-2.** The HUD cue is consumer-side (M11) tuning data, not event surface data. The event payload doesn't need to carry the cue; HUD will look it up from the band value. Pass-1's choice to leave it alone is correct.

**If future M11 audit demands machine-readable form,** the natural place is a sidecar JSON like `cf-replay/schemas/lookup/concussion_band_hud_cue.json`, not the event schema.

---

### NEW-N — `origin.oxygen_supply_changed.source` enum completeness  *(P3 — coverage call)*

**Pass-1 added enum:** `["helmet_breach", "refilled", "exhaled", "atmosphere"]`.

**Probe `[M]/[N]/[O]` evidence:** validator REJECTs `tank_swap`, `external_supply`, `death`.

**Candidate additions** the task brief raised:

- `tank_swap` — player swaps an empty tank for a fresh one. **Likely needed at M17** (oxygen tank management). Currently `refilled` covers refill-in-place; tank swap could plausibly fold under that, but is semantically different (different `from_s → to_s` step shape; refilled is incremental, swap is full reset).
- `external_supply` — hookup to base-level oxygen line. **Likely needed at M19** (base atmospherics ↔ actor tank). Could fold under `atmosphere` if the actor is unhelmeted in pressurized environment, but if helmeted-with-external-feed it's distinct.
- `death` — actor dies; oxygen reading goes meaningless. **Probably NOT a separate source** — death drops the actor from origin tracking; `origin.oxygen_supply_changed` shouldn't fire post-death.
- `suit_breach` — suit (not helmet) is punctured; oxygen leaks. Currently `helmet_breach` covers helmet only. M19 may extend.

**Recommended action (P3 — additive, await M17 / M19 review):**

```diff
-        "source": { "type": "string", "enum": ["helmet_breach", "refilled", "exhaled", "atmosphere"] }
+        "source": { "type": "string", "enum": ["helmet_breach", "suit_breach", "refilled", "tank_swap", "exhaled", "atmosphere", "external_supply"] }
```

Or hold and let M17 / M19 add as needed (additive-only per M4 DR-002 — adding enum values to an existing enum is technically a SHAPE change and would require an envelope bump under strict reading). **Conservative pass-2 take:** keep pass-1's 4-value enum until M17 producer concretely needs more; flag the gap here so M17 audit picks it up.

---

## End-to-end validator probe — full transcript

Probe rig at `/tmp/m5_pass2_probe/main.rs`; depends on local `cf-replay` crate. Output:

```text
=== NEW-pass2 origin_id enforcement probes ===
[A] dose_changed origin=Construct -> Err("concussion.dose_changed::origin_id value \"Construct\" did not satisfy any oneOf branch — branch[0]: concussion.dose_changed::origin_id expected type [\"integer\"], got \"Construct\"; branch[1]: concussion.dose_changed::origin_id value \"Construct\" not in enum [String(\"Human\"), String(\"Android\"), String(\"Robot\"), String(\"PoweredOrganic\"), String(\"HeavyBiomech\")]")
[B] dose_changed origin=Human    -> Ok(())
[C] dose_changed origin=int 5    -> Ok(())
[D] dose_changed to_dose=150     -> Err("concussion.dose_changed::to_dose value 150 > maximum 100")
[E] dose_changed to_dose=100     -> Ok(())
[F] band_changed dose=100        -> Ok(())
[G] band_changed dose=101        -> Err("concussion.band_changed::dose value 101 > maximum 100")
[H] sff origin=Construct         -> Err("origin.shot_force_feedback::origin_id value \"Construct\" did not satisfy any oneOf branch — branch[0]: origin.shot_force_feedback::origin_id expected type [\"integer\"], got \"Construct\"; branch[1]: origin.shot_force_feedback::origin_id value \"Construct\" not in enum [String(\"Human\"), String(\"Android\"), String(\"Robot\"), String(\"PoweredOrganic\"), String(\"HeavyBiomech\")]")
[I] sff origin=HeavyBiomech      -> Ok(())
[J] sff chassis=armor_external   -> Ok(())
[K] sff chassis=skin             -> Err("origin.shot_force_feedback::chassis_layer value \"skin\" not in enum [String(\"armor_external\"), String(\"armor_internal\"), String(\"armor_core\"), String(\"armor_chunked_breach\"), String(\"flesh\"), String(\"circuit\"), String(\"unarmored\"), String(\"terrain\")]")
[L] sff Robot+pain_jolt (drift)  -> Ok(())                              ← NEW-K: cross-field UN-enforced
[M] oxygen source=tank_swap      -> Err("…not in enum…")
[N] oxygen source=external_supply-> Err("…not in enum…")
[O] oxygen source=death          -> Err("…not in enum…")
[P] internal_shock to_dose=500   -> Ok(())                              ← NEW-B: no maximum
[Q] shock module_id=garbage      -> Ok(())                              ← NEW-C: open module_id
[R] organ_destroyed cascade=garbage -> Ok(())                           ← NEW-E: open failure_cascade items
[S] ko duration=3 (out of 5-10)  -> Ok(())                              ← NEW-J: no 5..10 range
[T] recovered no origin_id        -> Ok(())                             ← NEW-F: no origin gating
[U] recovered (robot) accepted    -> Ok(())                             ← NEW-F: no origin gating
```

**Hand-test conclusions:**

- All pass-1 hardening (Origin enum REJECT of `Construct`, dose-100 ceiling, chassis_layer enum, oxygen_supply_changed.source enum) is enforced live by the validator. ✓
- 6 NEW gaps observed via probe (B, C, E, F×2, J, K), aligned with the issue list.

---

## cf-replay test suite — green on commit `1784ad2`

```text
$ cargo test -p cf-replay --lib schemas::

running 14 tests
test schemas::tests::m5_armor_layer_destroyed_accepts_additive_payload_extension ... ok
test schemas::tests::m5_armor_layer_destroyed_rejects_bad_zone_enum ... ok
test schemas::tests::m5_armor_layer_destroyed_rejects_missing_breach_kind ... ok
test schemas::tests::m5_armor_layer_destroyed_payload_validates ... ok
test schemas::tests::m5_concussion_dose_changed_rejects_bad_origin ... ok
test schemas::tests::terrain_carved_event_validates_minimum_payload ... ok
test schemas::tests::terrain_penetration_threshold_event_validates ... ok
test schemas::tests::m5_combat_projectile_hit_mo_rejects_envelope_named_parent ... ok
test schemas::tests::unknown_event_type_is_ok_by_default ... ok
test schemas::tests::validates_projectile_spawned_array_arity ... ok
test schemas::tests::validates_input_intent_received_required_fields ... ok
test schemas::tests::m5_per_family_happy_path ... ok
test schemas::tests::m5_schemas_declare_schema_version_v0_1 ... ok
test schemas::tests::schemas_load_for_every_registered_event_type ... ok

test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 25 filtered out
```

---

## Recommended fix priority list

| ID | Title | Priority | Owner-milestone | Effort |
|---|---|---|---|---|
| NEW-E | Lock `applied_afflictions` items to 23-affliction enum on internal.organ_failure_cascade + internal.circuit_failure_cascade | **P1** | M5 (schema-only) | trivial (~10 LOC JSON × 2 files). Validator extension to walk `items.enum` is a separate ~30 LOC change. |
| NEW-K | Reconcile M5↔M17 drift on `feedback_kind` per origin; decide if frame_ring fires for Robot | **P2** | M5+M17 spec reconciliation | spec edit + optional schema if/then |
| NEW-C | Lock `internal_shock.module_damaged.module_id` to 12-circuit enum (or specify separate taxonomy) | **P2** | M5 spec + schema | trivial JSON if 12-circuit; M17 owner consult if separate |
| NEW-H | Lock `origin.shot_force_feedback.leak_channel` to 4-fluid-kind enum | **P2** | M5 schema | trivial JSON |
| NEW-B | Add `maximum: 100` to `internal_shock.dose_changed.{from,to}_dose` | **P2** | M5 schema | trivial JSON |
| NEW-D | Decide `organ_kind` / `circuit_kind` enum lock or remove field | **P2** | M5+M17 spec | spec edit |
| NEW-J | Tighten `concussion.ko_threshold_crossed.ko_duration_s` to 5..10 range | **P2** | M5 schema | trivial JSON |
| NEW-A | Add Robot-only description gate clause to `internal_shock.dose_changed` | **P3** | M5 schema description | one-line edit |
| NEW-F | Add "never fires for Robot" description clause to `concussion.recovered` | **P3** | M5 schema description | one-line edit |
| NEW-G | Add "populated only when…" clause to 4 nullable fields on `origin.shot_force_feedback` | **P3** | M5 schema description | description rewrite |
| NEW-L | Add producer-contract clause about dose↔band pairing | **P3** | M5 schema description | two-line edit on both schemas |
| NEW-N | Audit oxygen_supply_changed.source enum coverage at M17/M19 time | **P3** | M17/M19 owner | deferred |
| NEW-M | HUD cue mapping — keep as prose; defer machine-readable form to M11 | **N/A** | (no action) | — |
| NEW-I | helmet_item_id typed integer — verified consistent | **N/A** | (no action) | — |

---

## M6 readiness verdict

**M6 ships actor controller + equipment + inventory + sound + squad-of-two.** The deep-damage event surface (M5 scope) is NOT M6's primary domain. M6's relationship to the in-scope schemas:

- M6 doesn't emit any `concussion.*`, `internal.*`, `internal_shock.*`, or `origin.*` events (those producers ladder up at M14 + M17).
- M6 DOES emit `affliction.applied{kind: "blinded"}` for the flash grenade (the kind added in pass-1). Verified via `m5_per_family_happy_path` test that the 23-kind enum accepts `blinded`.
- M6 reserves 3 tank slots that snapshot via `inventory.tank_slot_reserved` (not in this audit's scope) — orthogonal to M5.

**M6 is unblocked by this audit's findings.** None of NEW-A through NEW-N would prevent M6 from shipping its in-scope behavior. Every gap surfaces only when M14 or M17 starts producing the events.

**The deferred fixes (P1 = NEW-E; P2 = the other 5) should land before M14 producer code starts.** Until M14 ships, the gaps are latent — no code emits these events, so no drift can occur. The schema-lock-now-fix-later trade-off is correct for M5's "additive-only declarative" scope; pass-2's recommended fixes are mostly enum-tightenings that are themselves additive (tightening an open string to an enum REJECTS a producer that previously emitted garbage, so technically this IS a shape change under strict reading, but only the validator side notices, not the consumer side).

---

## Summary

- **Pass-1 deliveries verified: 11 / 11.** All Origin-enum, maximum-100, chassis_layer-enum, oxygen-source-enum, and validator (oneOf + maximum) changes land correctly on `1784ad2`. cf-replay test suite green; runtime probe confirms enforcement behavior matches the schema declarations.
- **New issues found: 12 actionable (NEW-A through NEW-L, NEW-N) + 2 verified-no-action (NEW-I, NEW-M).**
  - **P1 (1):** NEW-E — `applied_afflictions` items missing 23-affliction enum lock.
  - **P2 (6):** NEW-B (`internal_shock` dose max), NEW-C (`module_id` open), NEW-D (`organ_kind`/`circuit_kind` open), NEW-H (`leak_channel` open), NEW-J (`ko_duration_s` 5..10), NEW-K (cross-spec M5↔M17 drift on `feedback_kind` for Robot).
  - **P3 (5):** NEW-A, NEW-F, NEW-G, NEW-L, NEW-N — mostly description tightening + one M17/M19 deferred coverage decision.
  - **No action (2):** NEW-I (verified consistent), NEW-M (description-only is correct for M11-owned mapping).
- **Critical (P0):** none.
- **M6 readiness verdict: UNBLOCKED.** M5 pass-2 findings are latent until M14 / M17 producer code lands. M6's only M5-side interaction (`affliction.applied{kind: "blinded"}`) is already locked by pass-1.
- **M14 readiness verdict (advisory):** address P1 (NEW-E) + the P2 enum lockings (NEW-B, NEW-C, NEW-H, NEW-J) before M14 starts emitting `internal.*` / `origin.*` / `internal_shock.*` events. NEW-K (cross-spec drift) needs spec-owner reconciliation between M5 and M17 before either milestone closes.
