# M5 Audit — internal.* + concussion.* + internal_shock.* + origin.*

Spec source: `/Users/erol/projects/corefall/specs/done/M5.md` § "internal.* family", "concussion.* + internal_shock.* family", "origin.* family"
Schema source: `/Users/erol/projects/corefall/game/crates/cf-replay/schemas/event/`
Registration source: `/Users/erol/projects/corefall/game/crates/cf-replay/src/schemas.rs`

---

## Per-event verdict table

| Event | Schema file | Verdict | Notes / Gaps |
|---|---|---|---|
| `internal.organ_damaged` | `internal_organ_damaged.json` | **PASS** | All 7 spec fields (`actor_id, organ_id, organ_kind, from_hp, to_hp, cause, source_hit_event_id`) in `properties` + `required`. 15-organ enum complete on `organ_id`. Registered at `("internal", "organ_damaged")`. |
| `internal.organ_destroyed` | `internal_organ_destroyed.json` | **PASS** | All 4 spec fields (`actor_id, organ_id, organ_kind, failure_cascade`) in `properties` + `required`. `failure_cascade` typed as `array of string` (M17 reaction-matrix fills the contents). 15-organ enum complete. Registered. |
| `internal.organ_failure_cascade` | `internal_organ_failure_cascade.json` | **PASS** | All 4 spec fields (`actor_id, organ_id, applied_afflictions, hp_drain_per_s`) in `properties` + `required`. 15-organ enum complete on `organ_id`. Registered. |
| `internal.circuit_damaged` | `internal_circuit_damaged.json` | **PASS** | All 5 spec fields (`actor_id, circuit_id, circuit_kind, from_hp, to_hp`) in `properties` + `required`. 12-circuit enum complete on `circuit_id`. Registered. |
| `internal.circuit_destroyed` | `internal_circuit_destroyed.json` | **PASS** | All 4 spec fields (`actor_id, circuit_id, circuit_kind, failure_cascade`) in `properties` + `required`. 12-circuit enum complete. Registered. |
| `internal.circuit_failure_cascade` | `internal_circuit_failure_cascade.json` | **PASS** | All 3 spec fields (`actor_id, circuit_id, applied_afflictions`) in `properties` + `required`. 12-circuit enum complete. Registered. |
| `concussion.dose_changed` | `concussion_dose_changed.json` | **PASS** | All 5 spec fields (`actor_id, from_dose, to_dose, source_event_id, origin_id`) in `properties` + `required`. **Note**: `origin_id` typed as `["integer", "string"]` — no Origin enum constraint (see Locked taxonomy section). Registered. |
| `concussion.band_changed` | `concussion_band_changed.json` | **PASS** | All 4 spec fields (`actor_id, from_band, to_band, dose`) in `properties` + `required`. Both `from_band` and `to_band` enums contain all 6 band names (`Clear, Mild, Moderate, Severe, KO_Imminent, KO`). Registered. |
| `concussion.ko_threshold_crossed` | `concussion_ko_threshold_crossed.json` | **PASS** | Both spec fields (`actor_id, ko_duration_s`) in `properties` + `required`. `ko_duration_s` has `minimum: 0.0`. Registered. |
| `concussion.recovered` | `concussion_recovered.json` | **PASS** | Both spec fields (`actor_id, recovery_reason`) in `properties` + `required`. `recovery_reason` enum contains all 3 values (`time, medikit, environment`). Registered. |
| `internal_shock.dose_changed` | `internal_shock_dose_changed.json` | **PASS** | All 4 spec fields (`actor_id, from_dose, to_dose, source_event_id`) in `properties` + `required`. Registered at `("internal_shock", "dose_changed")`. |
| `internal_shock.module_damaged` | `internal_shock_module_damaged.json` | **PASS** | All 5 spec fields (`actor_id, module_id, damage_amount, hit_zone, source_hit_event_id`) in `properties` + `required`. `hit_zone` enum carries the full 15-zone BodyZone taxonomy. Registered. |
| `origin.shot_force_feedback` | `origin_shot_force_feedback.json` | **GAP (soft)** | All 14 spec fields in `properties`; **only 10 in `required`**. The 4 nullable fields `internal_shock_module_id, internal_shock_damage, leak_channel, leak_rate` are typed as nullable (`["integer", "string", "null"]` / `["number", "null"]` / `["string", "null"]`) and excluded from `required`. Spec lists them as plain fields without an optional marker, so by strict reading every field MUST be in `required`. Designer-defensible (only populated for robot origins / fluid-leaks) but a drift from the literal spec. Feedback-kind enum (`pain_jolt, servo_jolt, frame_ring`) complete. `origin_id` typed as `["integer", "string"]` — no Origin enum constraint. Registered. |
| `origin.g_load_dose_changed` | `origin_g_load_dose_changed.json` | **PASS** | All 4 spec fields (`actor_id, from_dose, to_dose, source`) in `properties` + `required`. `source` enum complete (`fall, high_g_maneuver, rapid_impact`). Registered. |
| `origin.helmet_breach` | `origin_helmet_breach.json` | **PASS** | All 4 spec fields (`actor_id, helmet_item_id, breach_pos, oxygen_loss_rate`) in `properties` + `required`. `breach_pos` typed as 2-tuple of numbers. Registered. |
| `origin.oxygen_supply_changed` | `origin_oxygen_supply_changed.json` | **PASS (soft)** | All 4 spec fields (`actor_id, from_s, to_s, source`) in `properties` + `required`. `source` is `string` with no enum — spec also leaves `source` unconstrained (no `'a'\|'b'\|'c'` shape), so this matches the spec literally. Registered. |

**Counts: 15 PASS / 1 soft GAP (origin.shot_force_feedback nullable fields not in `required`).**

---

## Locked taxonomy coverage

### 15-organ humanoid graph

Spec (M5 § internal.* family):

> `brain` / `eyes_left` / `eyes_right` / `ears_left` / `ears_right` / `heart` / `lungs_left` / `lungs_right` / `liver` / `kidneys_left` / `kidneys_right` / `spine` / `stomach` / `intestines` / `pancreas`

**Verdict: PASS.** Identical enum present in all three internal organ schemas (`internal_organ_damaged.json`, `internal_organ_destroyed.json`, `internal_organ_failure_cascade.json`):

```json
"organ_id": { "type": "string", "enum": ["brain", "eyes_left", "eyes_right", "ears_left", "ears_right", "heart", "lungs_left", "lungs_right", "liver", "kidneys_left", "kidneys_right", "spine", "stomach", "intestines", "pancreas"] }
```

Order matches the spec verbatim. All 15 names present, none extra.

### 12-circuit robot graph

Spec (M5 § internal.* family):

> `power_core` / `cpu` / `sensor_array` / `motor_controller_left_arm` / `motor_controller_right_arm` / `motor_controller_left_leg` / `motor_controller_right_leg` / `hydraulic_pump` / `coolant_pump` / `oil_reservoir` / `fuel_tank` / `comm_relay`

**Verdict: PASS.** Identical enum present in all three internal circuit schemas (`internal_circuit_damaged.json`, `internal_circuit_destroyed.json`, `internal_circuit_failure_cascade.json`):

```json
"circuit_id": { "type": "string", "enum": ["power_core", "cpu", "sensor_array", "motor_controller_left_arm", "motor_controller_right_arm", "motor_controller_left_leg", "motor_controller_right_leg", "hydraulic_pump", "coolant_pump", "oil_reservoir", "fuel_tank", "comm_relay"] }
```

Order matches the spec verbatim. All 12 names present, none extra.

### 5-band concussion accumulator (Clear / Mild / Moderate / Severe / KO_Imminent / KO)

(Spec calls it "5-band" but the table has 6 rows including the absorbing KO state — both `from_band` and `to_band` must accept all 6.)

**Verdict: PASS.** `concussion_band_changed.json` from_band/to_band enums:

```json
"from_band": { "type": "string", "enum": ["Clear", "Mild", "Moderate", "Severe", "KO_Imminent", "KO"] },
"to_band":   { "type": "string", "enum": ["Clear", "Mild", "Moderate", "Severe", "KO_Imminent", "KO"] }
```

All 6 band names present, correct capitalization, no extras.

### Dose ranges per band (0-20, 20-40, 40-60, 60-80, 80-99, 100)

Spec table:

| Band | Dose threshold |
|---|---|
| Clear | 0-20 |
| Mild | 20-40 |
| Moderate | 40-60 |
| Severe | 60-80 |
| KO_Imminent | 80-99 |
| KO | 100 |

**Verdict: GAP (documentation-only).**

- `concussion_band_changed.json` carries the table in its `description` string:
  > "5-band accumulator (Clear 0-20, Mild 20-40, Moderate 40-60, Severe 60-80, KO_Imminent 80-99, KO 100)."
- No machine-checkable schema constraint pairs a band to its dose range. Per JSON Schema this would require conditional `if`/`then` per-band branches, which the schemas do not have.
- `dose` field carries `minimum: 0.0` only — **no `maximum: 100`** on `concussion_band_changed.dose`, `concussion_dose_changed.from_dose`, or `concussion_dose_changed.to_dose`. Per spec, dose is 0..100 (KO at 100), so a `maximum: 100` is missing.
- `concussion_dose_changed.json` description states "Concussion dose 0..100 accumulator" — also documentation-only.

**Acceptable for M5 (declarative envelope lock; producer enforces ranges)** but technically a literal-spec drift: the dose ceiling is locked in spec text, not in schema.

### HUD cues per band (locked details)

Spec table:

| Band | HUD cue |
|---|---|
| Clear | none |
| Mild | edge vignette 10% |
| Moderate | vignette 30% + bloom |
| Severe | vignette 60% + sway |
| KO_Imminent | vignette 85% + tunnel |
| KO | full blackout 5-10s |

**Verdict: DOCUMENTATION-ONLY.**

- `concussion_band_changed.json` description embeds the table verbatim:
  > "HUD cue per band: Clear=none, Mild=edge vignette 10%, Moderate=vignette 30% + bloom, Severe=vignette 60% + sway, KO_Imminent=vignette 85% + tunnel, KO=full blackout 5-10s."
- No event field carries the HUD cue (e.g. no `hud_cue` enum on `concussion.band_changed`).
- **Reasonable** for an event schema (HUD is a renderer concern downstream of the event surface), but the spec lists this as a **locked HUD detail**. If a future audit demands the HUD-cue mapping be machine-readable, a separate cue-spec doc or a `hud_cue_hint` field would be needed.

### Per-origin decay rates (Human / Android / Robot / Powered organic / Heavy biomech)

Spec table:

| Origin | Concussion decay | Internal-shock decay |
|---|---|---|
| Human | 5/s | n/a |
| Android | 5/s | n/a |
| Robot | 0/s (always 0 dose) | 2/s |
| Powered organic | 5/s | n/a |
| Heavy biomech | 4/s | n/a |

**Verdict: GAP — Origin enum NOT captured as a JSON enum constraint anywhere.**

Evidence:

- `concussion_dose_changed.json` payload: `"origin_id": { "type": ["integer", "string"] }` — free-form integer-or-string, no enum.
- `origin_shot_force_feedback.json` payload: `"origin_id": { "type": ["integer", "string"] }` — same.
- No `Origin` enum (`Human|Android|Robot|PoweredOrganic|HeavyBiomech`) is declared as a `string`+`enum` constraint in any of the 16 schemas in scope.
- Decay rate values appear only in description text:
  - `concussion_dose_changed.json` description: "Per-origin decay rates: Human 5/s, Android 5/s, Robot 0/s (always 0), Powered organic 5/s, Heavy biomech 4/s."
  - `internal_shock_dose_changed.json` description: "Decay rate (locked): Robot 2/s, Heavy biomech 4/s (concussion equivalent), others n/a."

**Cross-schema inconsistency on Origin spelling** (out of strict scope but found while auditing):

- `snapshot_origin.json` (M4 placeholder) carries an Origin enum spelling in its description: `Human|Android|Robot|PoweredOrganic|Construct|HeavyBioMech` — note `HeavyBioMech` (camel-case `M`) vs M5 spec's `Heavy biomech`, `PoweredOrganic` vs M5 spec's `Powered organic`, and **`Construct` is not in the M5 decay-rate table** (so either snapshot_origin.json includes a 6th origin not yet specified, or this is an unfixed drift).

Per spec the Origin enum is **locked**, so the lack of a schema-level enum is a real gap.

### origin.shot_force_feedback full payload

Spec lists 14 fields (parent agent's task summary said "15 fields" — recount of the spec text: `actor_id, parent_hit_event_id, impulse_vector, impulse_magnitude, origin_id, chassis_layer, feedback_kind, g_load_delta, concussion_dose_delta, internal_shock_module_id, internal_shock_damage, leak_channel, leak_rate, screen_kick_intensity` = **14**).

| Field | In `properties`? | In `required`? | Notes |
|---|---|---|---|
| `actor_id` | YES | YES | |
| `parent_hit_event_id` | YES | YES | |
| `impulse_vector` | YES | YES | array, minItems=2, maxItems=2 |
| `impulse_magnitude` | YES | YES | |
| `origin_id` | YES | YES | `["integer", "string"]` — no Origin enum |
| `chassis_layer` | YES | YES | string, no enum (`armor_external`/`armor_internal`/`armor_core` not enumerated here) |
| `feedback_kind` | YES | YES | enum `pain_jolt, servo_jolt, frame_ring` ✓ |
| `g_load_delta` | YES | YES | |
| `concussion_dose_delta` | YES | YES | |
| `internal_shock_module_id` | YES | **NO** | `["integer", "string", "null"]` |
| `internal_shock_damage` | YES | **NO** | `["number", "null"]` |
| `leak_channel` | YES | **NO** | `["string", "null"]` |
| `leak_rate` | YES | **NO** | `["number", "null"]` |
| `screen_kick_intensity` | YES | YES | |

**Verdict: PASS on properties (14/14) / GAP on required (10/14).**

The 4 excluded fields are designer-marked optional via nullable types because they only populate for robot origins (`internal_shock_*`) or actors that bleed fluids (`leak_*`). This is defensible but disagrees with the audit rubric's "every field listed must appear in `payload.required`". Spec text gives no optional marker.

### Feedback kind enum (`pain_jolt | servo_jolt | frame_ring`)

**Verdict: PASS.** `origin_shot_force_feedback.json` carries:

```json
"feedback_kind": { "type": "string", "enum": ["pain_jolt", "servo_jolt", "frame_ring"] }
```

All 3 spec values present, no extras.

### Recovery reason enum (`time | medikit | environment`)

**Verdict: PASS.** `concussion_recovered.json` carries:

```json
"recovery_reason": { "type": "string", "enum": ["time", "medikit", "environment"] }
```

All 3 spec values present, no extras.

### G-load source enum (`fall | high_g_maneuver | rapid_impact`)

**Verdict: PASS.** `origin_g_load_dose_changed.json` carries:

```json
"source": { "type": "string", "enum": ["fall", "high_g_maneuver", "rapid_impact"] }
```

All 3 spec values present, no extras.

---

## Registration cross-check (`event_schema_for`)

All 16 events have a `(category, event_type) -> Some(SCHEMA_*)` match arm in `/Users/erol/projects/corefall/game/crates/cf-replay/src/schemas.rs` and a corresponding `include_str!` constant. Verified:

```text
("internal", "organ_damaged")            -> SCHEMA_INTERNAL_ORGAN_DAMAGED              ✓
("internal", "organ_destroyed")          -> SCHEMA_INTERNAL_ORGAN_DESTROYED            ✓
("internal", "organ_failure_cascade")    -> SCHEMA_INTERNAL_ORGAN_FAILURE_CASCADE      ✓
("internal", "circuit_damaged")          -> SCHEMA_INTERNAL_CIRCUIT_DAMAGED            ✓
("internal", "circuit_destroyed")        -> SCHEMA_INTERNAL_CIRCUIT_DESTROYED          ✓
("internal", "circuit_failure_cascade")  -> SCHEMA_INTERNAL_CIRCUIT_FAILURE_CASCADE    ✓
("concussion", "dose_changed")           -> SCHEMA_CONCUSSION_DOSE_CHANGED             ✓
("concussion", "band_changed")           -> SCHEMA_CONCUSSION_BAND_CHANGED             ✓
("concussion", "ko_threshold_crossed")   -> SCHEMA_CONCUSSION_KO_THRESHOLD_CROSSED     ✓
("concussion", "recovered")              -> SCHEMA_CONCUSSION_RECOVERED                ✓
("internal_shock", "dose_changed")       -> SCHEMA_INTERNAL_SHOCK_DOSE_CHANGED         ✓
("internal_shock", "module_damaged")     -> SCHEMA_INTERNAL_SHOCK_MODULE_DAMAGED       ✓
("origin", "shot_force_feedback")        -> SCHEMA_ORIGIN_SHOT_FORCE_FEEDBACK          ✓
("origin", "g_load_dose_changed")        -> SCHEMA_ORIGIN_G_LOAD_DOSE_CHANGED          ✓
("origin", "helmet_breach")              -> SCHEMA_ORIGIN_HELMET_BREACH                ✓
("origin", "oxygen_supply_changed")      -> SCHEMA_ORIGIN_OXYGEN_SUPPLY_CHANGED        ✓
```

All 16 also covered by the `schemas_load_for_every_registered_event_type` and `m5_schemas_declare_schema_version_v0_1` tests in `cf-replay/src/schemas.rs`.

---

## Recommended fixes

1. **(P1) Add Origin enum constraint where `origin_id` is a string.**
   - In `concussion_dose_changed.json` and `origin_shot_force_feedback.json`, replace
     ```json
     "origin_id": { "type": ["integer", "string"] }
     ```
     with a string variant carrying a locked enum:
     ```json
     "origin_id": {
       "oneOf": [
         { "type": "integer" },
         { "type": "string", "enum": ["Human", "Android", "Robot", "PoweredOrganic", "HeavyBiomech"] }
       ]
     }
     ```
     (Matching the M5 spec's 5 origin classes literally — drop `Construct` until specced.)

2. **(P1) Reconcile `snapshot_origin.json` Origin spelling with M5.**
   - `snapshot_origin.json` description has `Human|Android|Robot|PoweredOrganic|Construct|HeavyBioMech` (note `HeavyBioMech` ≠ spec's `Heavy biomech`/`HeavyBiomech`, and `Construct` is not in the M5 table).
   - Pick one canonical Pascal-case spelling and use it everywhere. Suggested: `Human, Android, Robot, PoweredOrganic, HeavyBiomech` (5 values, drop `Construct` until M5 adds it).

3. **(P2) Add `maximum: 100` to concussion dose fields.**
   - `concussion_band_changed.json` `dose`, `concussion_dose_changed.json` `from_dose` + `to_dose` — add `"maximum": 100` per the locked 0..100 dose range.

4. **(P2) Decide on the 4 origin.shot_force_feedback nullable fields.**
   - Either (a) make them required and let producers send `null` (current shape allows nullable types), or (b) document in the schema description that they're optional-by-origin. Today's shape is properties-include + required-exclude with nullable type — workable, but the spec lists them as plain fields with no optional marker, so a literal-spec audit flags it.
   - Recommended: leave optional, but add a description note: `"description": "Populated only when origin is Robot (...) or actor has a fluid-bearing module (...)"`.

5. **(P3) Encode dose-to-band mapping if a future audit demands it.**
   - JSON-schema `if`/`then` per band would let `band == 'Mild'` imply `dose in [20, 40)`. Skipping this is fine for M5 (declarative envelope lock), but if M17 producer compliance is later audited against schema, this would catch mismatches at validation time. Today the mapping is description-text-only.

6. **(P3) Encode HUD cue mapping out-of-band if needed.**
   - HUD cues are documented in `concussion_band_changed.json` description but not field-encoded. Acceptable; flag only if a renderer-side audit needs machine-readable cues.

7. **(P3) Consider an enum on `origin.oxygen_supply_changed.source`.**
   - Spec says: `origin.oxygen_supply_changed { actor_id, from_s, to_s, source }` — `source` is described literally as "describes the change (helmet_breach, refilled, exhaled, atmosphere)" in the schema description. Tightening to a string enum (`helmet_breach, refilled, exhaled, atmosphere`) would lock it.

---

## Summary

- **Total events audited: 16**
- **PASS: 15** (all `internal.*` (6) + all `concussion.*` (4) + all `internal_shock.*` (2) + 3 of 4 `origin.*`)
- **Soft GAP: 1** — `origin.shot_force_feedback` has all 14 properties but only 10 in `required` (4 nullable robot/fluid fields excluded). Designer-defensible but a literal-spec drift.
- **Cross-cutting GAPs (taxonomy / locked tables not encoded in schema):**
  - **Origin enum NOT encoded as a JSON enum** anywhere in the 16 schemas (P1 — locked per M5 spec, currently free-form `["integer", "string"]`).
  - **Cross-schema Origin-spelling drift** in `snapshot_origin.json` (`Construct` extra, `HeavyBioMech` capitalization vs `Heavy biomech` / `HeavyBiomech`).
  - **Dose maximum 100** not encoded on `concussion_dose_changed.{from,to}_dose` or `concussion_band_changed.dose` (P2).
  - **Dose-to-band mapping** and **per-band HUD cues** are description-text-only, not field-encoded (P3 — acceptable for an event surface, flagged only).
  - **Per-origin decay rates** are description-text-only (acceptable; rates are producer state, not event payload, and live in `cf-actor`/`cf-sim-core` per M17).
- **Registration: 16 of 16** events present in `event_schema_for` and covered by the schema-load + schema-version conformance tests.
- **Critical missing pieces:**
  - **NONE BLOCKING M5 closure.** All 16 event schemas exist, all fields enumerated in M5 prose are present in `properties`, all locked enums (15-organ, 12-circuit, 6-band concussion, 3-value feedback_kind, 3-value recovery_reason, 3-value g_load source) carry every spec name correctly, and all 16 are registered in `event_schema_for`.
  - The Origin-enum gap (P1) is the highest-impact drift — every M17 producer will reach for a typed `Origin` value, and right now the only locked spelling is in a `snapshot_origin.json` description with a 6-value extension (`Construct`) not in M5's table. Fix before M17 producer ships.
