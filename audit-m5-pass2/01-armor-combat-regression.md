# M5 Pass-2 Audit — `armor.*` + `combat.projectile_hit_mo` + `audio.event_requested`

Audit date: 5/14/2026
Scope: re-audit of the 19 `armor.*` schemas + `combat.projectile_hit_mo` + `audio.event_requested` after pass-1 fixes landed in commit `1784ad2 M5-A1: post-audit hardening pass`.

Methodology:
- Read every shipped schema under `game/crates/cf-replay/schemas/event/armor_*.json`, `combat_projectile_hit_mo.json`, `audio_event_requested.json`.
- Diffed pass-1 commit (`git show 1784ad2 -- <file>`) to verify each pass-1 finding actually landed correctly.
- Counted canonical-literal coverage (`grep -c "prototype-recorder-event.v0.1"` per file).
- Cross-checked the M4 envelope schema (`schemas/v0_1/recorder_event.schema.json`) for envelope-level cause-chain pointers, since the cause-chain walk surface is shared.
- Synthesized 6 adversarial event bundles and ran `cargo run -p cf-mod -- --json validate-bundle <bundle>` to verify the validator's tolerance to malformed / semantically incoherent producer payloads.
- Compared every M5.md armor.* event signature with the shipped schema's `payload.required` + `payload.properties`.
- Walked the cause-chain pattern: which armor.* events carry an upstream event-id pointer, and which don't.

---

## Pass-1 deliveries verified

| Pass-1 finding | Fix shipped? | Verified by |
|---|---|---|
| `schema_version` literal aligned to canonical `prototype-recorder-event.v0.1` on all 19 armor.* + combat.projectile_hit_mo + audio.event_requested | YES | `grep -c "prototype-recorder-event.v0.1" <file>` returns 1 for every in-scope file; `grep -c "\"0.1\""` returns 0 for every file (i.e. no legacy short literal anywhere in scope). `cargo test -p cf-replay schemas::tests::m5_schemas_declare_schema_version_v0_1` PASSES. |
| `combat.projectile_hit_mo.payload.parent_event_id` renamed to `parent_hit_event_id` | YES | `git show 1784ad2 -- combat_projectile_hit_mo.json` shows the literal field rename; `grep parent_event_id combat_projectile_hit_mo.json` returns 0 matches; `grep parent_hit_event_id combat_projectile_hit_mo.json` returns 2 matches (properties + required); `cf-mod --json validate-bundle <bundle with parent_event_id>` REJECTS with `required field 'parent_hit_event_id' missing`. |
| `combat.projectile_hit_mo` payload-level `cosmetic` duplicate dropped | YES | `grep cosmetic combat_projectile_hit_mo.json` returns exactly one line, at envelope level (`"cosmetic": { "const": false }` at line 14). Payload-level entry is gone. |
| `audio.event_requested` schema shipped + registered | YES | New file `schemas/event/audio_event_requested.json` (37 lines, 2562 bytes). Registered in `cf-replay/src/schemas.rs::event_schema_for` under `("audio", "event_requested")`. Listed in both `schemas_load_for_every_registered_event_type` test and `m5_schemas_declare_schema_version_v0_1` test. |
| 7-material taxonomy on `audio.event_requested.payload.material` | YES | Enum = `["metal", "ceramic", "composite", "cloth", "leather", "hardened_plate", "reactive_armor", null]` — 7 spec-locked names + null. |
| 5-impact-state taxonomy on `audio.event_requested.payload.impact_state` | YES | Enum = `["pristine_hit", "cracked_hit", "destroyed_hit", "chunked_off", "pierce", null]` — 5 spec-locked names + null. |
| 6-name internal-hit taxonomy on `audio.event_requested.payload.internal_hit_kind` | YES | Enum = `["flesh_punctured", "bone_cracked", "organ_ruptured", "circuit_sparked", "circuit_destroyed", "fluid_pierce", null]` — 6 spec-locked names + null. |
| `audio.event_requested.payload.surface_kind` + `damage_kind` mirror `combat.projectile_hit_mo` | YES | surface_kind enum is the same 8-value enum as `combat.projectile_hit_mo.payload.surface_kind`; damage_kind enum is the same 5-value enum. |
| `audio.event_requested.payload.kind` discriminator | PARTIAL | Two-value enum `["material_state", "internal_hit"]` shipped. But it is NOT wired into a `oneOf`/`if-then-else` constraint that would gate the other payload fields. See NEW-B for the gap this leaves. |

**Net pass-1 verification: 9/9 fixes landed with one (the discriminator) only structurally present — the `kind` enum exists but doesn't gate the dependent fields semantically.**

---

## New issues found (pass-2)

### NEW-A: Cause-chain integrity in `armor.*` family — uniformity gap

**Severity: HIGH**

**Finding.** M5 introduces a deep-damage cause-chain surface that the M10 cause-chain walker (and any downstream consumer that wants to reconstruct "this debris came from that hit") depends on. The pattern is implemented inconsistently across the armor.* family:

| Event | Cause-chain pointer? | Field name |
|---|---|---|
| `armor.spalling` | YES | `cause_event_id` (string) |
| `armor.layer_hp_changed` | NO | (has `cause` — string description like "kinetic_round", NOT an event id) |
| `armor.layer_critical` | NO | — |
| `armor.layer_destroyed` | NO | — |
| `armor.all_layers_destroyed` | NO | — |
| `armor.chunked_off` | NO | — |
| `armor.debris_spawned` | NO | — |
| `armor.repaired` | NO | (non-damage event; lower priority) |
| `armor.angle_deflection_calculated` | NO | — |
| `armor.ricochet` | NO | — |
| `armor.penetration_ray_traversed` | NO | — |
| `armor.he_overpressure_wave` | NO | — |
| `armor.heat_jet_penetrated` | NO | — |
| `armor.heat_jet_pre_detonated_by_era` | NO | — |
| `armor.apfsds_penetrated` | NO | — |
| `armor.era_panel_detonated` | PARTIAL | `defeated_round_id` is a **round id, NOT an event id** |
| `armor.schurzen_pre_detonated` | PARTIAL | `defeated_round_id` is a **round id, NOT an event id** |
| `armor.multi_hit_degradation` | NO | (aggregates multiple hits; reasonable that it lacks a single pointer) |
| `armor.reactive_armor_consumed` | NO | — |

Of 19 armor.* events, only **1 carries a payload-level cause-chain event-id pointer** (`armor.spalling.payload.cause_event_id`). 16 carry nothing at the payload level; 2 (`era_panel_detonated`, `schurzen_pre_detonated`) carry a `defeated_round_id` which is conceptually a projectile/round identifier (typed `["integer","string"]`) — NOT the upstream `combat.projectile_hit_mo` event_id.

The envelope schema (`schemas/v0_1/recorder_event.schema.json`) defines a single optional `parent_event_id` field that all events MAY use to chain back to an upstream event. So technically the cause-chain is *recoverable* via envelope-level walking — but:

1. M10's cause-chain walk semantics treat `parent_event_id` as a generic single-link parent pointer, not a typed "the hit event that caused this armor.* effect" pointer. A consumer reconstructing "everything caused by hit X" has to walk every armor.* event in the bundle and match by envelope `parent_event_id`, which doesn't disambiguate "armor.spalling caused by hit X" (cause_event_id) from "armor.debris_spawned caused by chunk-off Y" (envelope parent_event_id chains to the chunk-off, not the original hit).

2. The naming drift across the rest of the M5 surface is significant — there are **five different names** for what is conceptually the same cause-chain pointer:

   - `cause_event_id` (armor.spalling)
   - `source_event_id` (concussion.dose_changed, internal_shock.dose_changed, hazard.spawned, audio.event_requested, affliction.applied)
   - `source_hit_event_id` (internal.organ_damaged, internal_shock.module_damaged)
   - `parent_hit_event_id` (combat.projectile_hit_mo, origin.shot_force_feedback)
   - `ignition_source_event_id` (fluid.ignition)

   The pass-1 commit message claims the rename to `parent_hit_event_id` was made "consistent with origin.shot_force_feedback + internal.organ_damaged naming" — but `internal.organ_damaged` actually uses `source_hit_event_id`, not `parent_hit_event_id`. The pass-1 fix introduced new drift while fixing an envelope collision.

**Recommended fix (P1).** For the armor.* family, add an optional `source_hit_event_id: string` (or `cause_event_id`, pick one name across the entire M5 surface) to every armor.* event payload that is plausibly a downstream consequence of a `combat.projectile_hit_mo`:

- `armor.layer_hp_changed`
- `armor.layer_critical`
- `armor.layer_destroyed`
- `armor.all_layers_destroyed`
- `armor.chunked_off`
- `armor.ricochet`
- `armor.angle_deflection_calculated`
- `armor.penetration_ray_traversed`
- `armor.he_overpressure_wave`
- `armor.heat_jet_penetrated`
- `armor.heat_jet_pre_detonated_by_era`
- `armor.apfsds_penetrated`
- `armor.era_panel_detonated` (in addition to `defeated_round_id`, which stays as the round identifier)
- `armor.schurzen_pre_detonated` (same)
- `armor.multi_hit_degradation` (last-hit pointer)
- `armor.reactive_armor_consumed`

Make it optional (not required) so producers can fire armor.* events that weren't caused by a single hit (e.g. environmental melt + chemical corrosion downstream of `hazard.actor_contact`), but document the strong convention that for hit-caused events, the pointer SHOULD be populated.

**Recommended fix (P2).** Standardize the field name across the entire M5 surface. Two reasonable choices:

- `source_event_id` — already used by 5 event families (concussion, internal_shock, hazard, audio, affliction). Most popular existing name.
- `cause_event_id` — used by armor.spalling. Less drift to fix.

Pick one and bulk-rename the others (additive deprecation: producers emit BOTH names during a transition window, consumers accept either, then drop the old name at M6 or M7).

### NEW-B: `audio.event_requested.kind` discriminator does not gate dependent fields

**Severity: HIGH**

**Finding.** The shipped schema declares `kind: enum["material_state", "internal_hit"]` as a payload field, plus `material`, `impact_state`, `internal_hit_kind` as nullable enums. There is NO `oneOf` / `if-then-else` constraint linking `kind` to the dependent fields. The validator therefore accepts every combination of:

| `kind` | `material` | `impact_state` | `internal_hit_kind` | Accepted? | Semantically coherent? |
|---|---|---|---|---|---|
| `material_state` | `metal` | `pristine_hit` | `null` | YES | YES |
| `material_state` | `metal` | `pristine_hit` | **`flesh_punctured`** | **YES** | **NO** (internal-hit on a material-surface event) |
| `material_state` | `null` | `null` | `null` | **YES** | **NO** (missing material + state on a material_state event) |
| `internal_hit` | `metal` | `pristine_hit` | `flesh_punctured` | YES | NO (material + impact_state should be null on internal_hit) |
| `internal_hit` | `null` | `null` | `null` | **YES** | **NO** (no internal_hit_kind on an internal_hit event) |

**Hard-test evidence (bundle 2):**
```json
{"kind":"material_state","material":"metal","impact_state":"pristine_hit",
 "internal_hit_kind":"flesh_punctured","surface_kind":"armor_external",
 "damage_kind":"kinetic","source_event_id":"run:42:5"}
```
→ `cf-mod --json validate-bundle` → `"failures": []` (ACCEPTED). This is semantically wrong: an armor-surface impact event should not carry an internal-hit kind.

**Hard-test evidence (bundle 5):**
```json
{"kind":"material_state","surface_kind":"armor_external",
 "damage_kind":"kinetic","source_event_id":"run:42:5"}
```
→ accepted with **no material and no impact_state**. The `material` + `impact_state` fields are nullable AND absent (not in `required`), so a producer can emit a `material_state` event with no material specified. The 7-material × 5-impact-state taxonomy is unenforced.

**Recommended fix (P0).** Use a JSON Schema `oneOf` discriminator on `kind`, with `if/then` style:

```json
{
  "payload": {
    "type": "object",
    "properties": {
      "kind":             { "type": "string", "enum": ["material_state","internal_hit"] },
      "material":         { "type": ["string","null"], "enum": [..., null] },
      "impact_state":     { "type": ["string","null"], "enum": [..., null] },
      "internal_hit_kind":{ "type": ["string","null"], "enum": [..., null] },
      "surface_kind":     { ... },
      "damage_kind":      { ... },
      "source_event_id":  { ... }
    },
    "required": ["kind","surface_kind","damage_kind","source_event_id"],
    "oneOf": [
      {
        "properties": {
          "kind":              { "const": "material_state" },
          "material":          { "type": "string", "enum": ["metal","ceramic","composite","cloth","leather","hardened_plate","reactive_armor"] },
          "impact_state":      { "type": "string", "enum": ["pristine_hit","cracked_hit","destroyed_hit","chunked_off","pierce"] },
          "internal_hit_kind": { "type": "null" }
        },
        "required": ["kind","material","impact_state"]
      },
      {
        "properties": {
          "kind":              { "const": "internal_hit" },
          "material":          { "type": "null" },
          "impact_state":      { "type": "null" },
          "internal_hit_kind": { "type": "string", "enum": ["flesh_punctured","bone_cracked","organ_ruptured","circuit_sparked","circuit_destroyed","fluid_pierce"] }
        },
        "required": ["kind","internal_hit_kind"]
      }
    ]
  }
}
```

(NOTE: cf-replay's minimal validator does not currently support nested `oneOf` inside `properties` and `if-then-else` is unsupported. The fix is twofold: (a) tighten the schema as above so external strict JSON-Schema validators reject incoherent payloads, AND (b) extend `validate_event_payload` in `schemas.rs` to walk a top-level `oneOf` constraint on the payload sub-schema. Pass-1 already added `oneOf` support for property-level branches; this is the same mechanism applied at object level.)

### NEW-C: `armor.ricochet` has no cause-chain pointer back to `combat.projectile_hit_mo`

**Severity: MEDIUM**

**Finding.** When a round ricochets, the canonical sequence is:
1. `combat.projectile_hit_mo` fires with `pierced_armor: false` + `surface_kind: "armor_external"` (or similar) + `ap_round_tier: <tier>` + `impact_angle` recorded in `impact_point`/`impact_normal`.
2. `armor.ricochet` fires with `impact_angle`, `ricochet_probability`, `was_ricocheted: true`, `deflection_vector`.

But `armor.ricochet` carries NO event-id pointer to step 1. A consumer wanting to verify the M5 locked ricochet-threshold table (`standard: 60°, armor_piercing: 65°, ..., APFSDS: 80°`) needs the `ap_round_tier` field from the parent hit event AND the `impact_angle` from the ricochet event. Right now the only way to recover the `ap_round_tier` is via envelope-level `parent_event_id` if the producer chains it (not guaranteed).

**Hard-test evidence (bundle 3):**
```json
{"category":"armor","event_type":"ricochet",
 "payload":{"impact_angle":1.2,"ricochet_probability":1.5,"was_ricocheted":true,"deflection_vector":[0.5,0.5]}}
```
→ Accepted. Note that there is NO field carrying back to a `combat.projectile_hit_mo` event id and NO `ap_round_tier`. The replay consumer cannot reconstruct "was this ricochet within the locked threshold for this ammo tier".

**Recommended fix (P1).** Add `source_hit_event_id: string` (or whatever name is chosen for NEW-A) to `armor.ricochet.payload.required`. Optionally add `ap_round_tier` as a redundant denormalized field so consumers don't need to walk the cause-chain just to bound-check the threshold table.

### NEW-D: `armor.debris_spawned.record_id` vs `armor.chunked_off.debris_record_id` — naming drift on the same physics record id

**Severity: LOW (naming) / MEDIUM (semantic clarity)**

**Finding.** The spec defines two events that share a single physics record identifier:

- `armor.chunked_off { item_id, zone, debris_record_id, impact_impulse, debris_kind, ground_position }` — fires first; the chunked-off layer becomes physics debris.
- `armor.debris_spawned { record_id, kind, material, position, velocity, can_pickup }` — fires when the physics debris becomes a pickup-eligible record.

The same identifier is named `debris_record_id` in the first event and `record_id` in the second. The schemas do not document that these are the same id, and they don't share field naming. A consumer reconstructing the chunk-off → spawn sequence has to know by convention that `chunked_off.debris_record_id == debris_spawned.record_id`.

Additionally, **neither event carries a pointer to the parent `combat.projectile_hit_mo` event** (which is what caused the chunking).

**Recommended fix (P2).** Rename `armor.debris_spawned.payload.record_id` → `armor.debris_spawned.payload.debris_record_id` for consistency with `armor.chunked_off`. Document in the schema description that the two events share the id. Add `source_hit_event_id` cause-chain pointer to both events (per NEW-A).

### NEW-E: `armor.he_overpressure_wave.modules_affected` taxonomy is unconstrained

**Severity: MEDIUM**

**Finding.** The schema declares:
```json
"modules_affected": {
  "type": "array",
  "items": { "type": ["integer", "string"] }
}
```

The M5 spec defines three module taxonomies that an HE overpressure wave could affect:
1. **15-organ humanoid graph** — `brain, eyes_left, eyes_right, ears_left, ears_right, heart, lungs_left, lungs_right, liver, kidneys_left, kidneys_right, spine, stomach, intestines, pancreas`
2. **12-circuit robot graph** — `power_core, cpu, sensor_array, motor_controller_left_arm, motor_controller_right_arm, motor_controller_left_leg, motor_controller_right_leg, hydraulic_pump, coolant_pump, oil_reservoir, fuel_tank, comm_relay`
3. **M13 chassis module ids** (not yet locked at M5; deferred to producer fill)

The current schema accepts any integer or any string in `modules_affected`. There is no constraint identifying which taxonomy the array elements come from, nor a cross-field discriminator on the target's origin to pick between organ_ids vs circuit_ids.

The same gap exists on `armor.penetration_ray_traversed.modules_hit` which is `Vec<(module_id, damage)>` — the module_id is open.

**Recommended fix (P2).** Add a `oneOf` constraint allowing each element of `modules_affected` to be one of: an organ_id enum, a circuit_id enum, or a string/integer chassis-module id (with the chassis enum to be filled at M13). At M5 the most we can lock is:

```json
"modules_affected": {
  "type": "array",
  "items": {
    "oneOf": [
      { "type": "string", "enum": [<15 organ_ids>] },
      { "type": "string", "enum": [<12 circuit_ids>] },
      { "type": ["integer","string"] }
    ]
  }
}
```

Same fix on `armor.penetration_ray_traversed.modules_hit[i][0]`.

Alternatively, document the taxonomy in the schema `description` field if cf-replay's minimal validator doesn't support nested `oneOf` inside `items`.

### NEW-F: `armor.ricochet.ricochet_probability` accepts values > 1.0

**Severity: MEDIUM**

**Finding.** The shipped schema declares:
```json
"ricochet_probability": { "type": "number", "minimum": 0.0 }
```

No `maximum`. A probability is by definition in `[0.0, 1.0]`. A producer at M13 emitting `ricochet_probability: 1.5` would pass validation despite being semantically a non-probability.

**Hard-test evidence (bundle 3):**
```json
{"impact_angle":1.2,"ricochet_probability":1.5,"was_ricocheted":true,"deflection_vector":[0.5,0.5]}
```
→ Accepted. No failure.

**Recommended fix (P1).** Tighten:
```json
"ricochet_probability": { "type": "number", "minimum": 0.0, "maximum": 1.0 }
```

The cf-replay validator added `maximum` support in pass-1 (used for concussion.dose ceiling) so this is a 1-line schema change with no validator work needed.

### NEW-G: `armor.apfsds_penetrated.rod_length_remaining_mm` accepts arbitrary large values

**Severity: LOW**

**Finding.** The shipped schema declares:
```json
"rod_length_remaining_mm": { "type": "number", "minimum": 0.0 }
```

No maximum. Real-world APFSDS rod lengths are ~400–800mm. Schema accepts `1e30`.

**Hard-test evidence (bundle 6):**
```json
{"item_id":12,"zone":"torso","rod_length_remaining_mm":1.0e30}
```
→ Accepted.

**Recommended fix (P3).** Tighten to a generous but finite ceiling. 2000mm covers any plausible long-rod penetrator: `"maximum": 2000.0`. Note: this is producer-error-prevention, not a spec-locked bound. Lowest priority.

### NEW-H: `armor.debris_spawned.material` is `["integer", "string"]` — no canonical material registry reference

**Severity: LOW**

**Finding.** The schema declares:
```json
"material": { "type": ["integer", "string"] }
```

The cf-material crate has a canonical 8-material registry (`air, dirt, concrete, metal_nohook, hazard, loose_fill, repair_fill, anchor`) baked into the engine. The M5 spec defines a SEPARATE 9-armor-type taxonomy (`Cast Iron, RHA, Composite, Ceramic, Reactive (ERA), Spaced, Schurzen, Composite-ceramic, Sloped Cast Steel`) and the audio.event_requested defines yet a THIRD 7-material taxonomy (`metal, ceramic, composite, cloth, leather, hardened_plate, reactive_armor`).

Three overlapping material taxonomies coexist:
1. cf-material's terrain registry (8 names)
2. M5 armor types (9 names)
3. audio.event_requested materials (7 names)

`armor.debris_spawned.material` is unconstrained — a producer could emit a value from any of the three taxonomies, or none of them.

**Recommended fix (P2).** Pick the canonical taxonomy for `armor.debris_spawned.material`. Most plausibly it's the cf-material registry (which is the actual material the layer was constructed from in cf-material terms). Constrain via enum. Alternatively, document in description that this is a cf-material MaterialId and reference the registry.

Note: pass-1 audit explicitly flagged this as MEDIUM ("encode the 9-armor-type locked taxonomy as a machine-readable companion at cf-replay/schemas/tables/armor_types.v0_1.json") but no `tables/` directory was created in pass-1.

### NEW-I: `era_panel_id`, `schurzen_plate_id`, `panel_id`, `defeated_round_id` field shapes are all `["integer", "string"]` — open polymorphism

**Severity: LOW**

**Finding.** Four armor.* events carry chassis-module/panel/round identifiers all typed as `["integer", "string"]`:

- `armor.era_panel_detonated.era_panel_id`
- `armor.era_panel_detonated.defeated_round_id`
- `armor.heat_jet_pre_detonated_by_era.era_panel_id`
- `armor.schurzen_pre_detonated.schurzen_plate_id`
- `armor.schurzen_pre_detonated.defeated_round_id`
- `armor.reactive_armor_consumed.panel_id`

M13 chassis is going to lock these as either `u64` integer record-ids OR string slugs but not both. Currently the schema is permissive in BOTH directions, which means a single producer could emit `era_panel_id: 42` in one event and `era_panel_id: "lt-43-era-3"` in another, and both validate.

**Recommended fix (P3).** Defer to M13 lock decision, but document an M13 readiness note that these fields will be tightened to a single type once cf-chassis decides. Alternatively, tighten NOW to `integer` (the natural record-id type) and force M13 producer authors to use the stable record_id layer rather than strings.

### NEW-J: `armor.spalling.fragment_count` still accepts 0 and arbitrary large values — spec locks 1–3

**Severity: MEDIUM (carryover from pass-1)**

**Finding.** Pass-1 audit recommended tightening `fragment_count` to `{"minimum": 1, "maximum": 3}` per the M5-locked spalling formula "spawn 1-3 fragments at random angles within 30° forward cone". The recommendation was **NOT applied** in pass-1. The shipped schema still declares:
```json
"fragment_count": { "type": "integer", "minimum": 0 }
```

minimum is 0 (should be 1); no maximum (should be 3).

**Hard-test evidence (bundle 4):**
```json
{"item_id":12,"zone":"torso","layer":"External","fragment_count":99,"damage_per_fragment":50.0,"cause_event_id":"run:42:5"}
```
→ Accepted. 99 fragments is way outside the spec-locked 1-3 range.

**Recommended fix (P1).** Apply the pass-1 recommendation:
```json
"fragment_count": { "type": "integer", "minimum": 1, "maximum": 3 }
```

Also lock `damage_per_fragment` to a range `[0.2, 0.5]` per spec, with the qualifier that this is "per-original-damage fraction" not absolute (so producer multiplies by the original damage_to_armor). Actually re-reading the spec, the 0.2-0.5 is "of original damage" which means the absolute value depends on the original damage; tightening damage_per_fragment to a fixed bound would over-constrain. Stick with just `fragment_count`.

### NEW-K: `armor.ricochet.deflection_vector` is locked at 2D — M13 chassis may emit 3D

**Severity: LOW**

**Finding.** The shipped schema declares:
```json
"deflection_vector": {
  "type": "array",
  "items": { "type": "number" },
  "minItems": 2,
  "maxItems": 2
}
```

This is hard-locked at 2D. The M5 spec is ambiguous — it just says "deflection_vector" with no dimensionality. cf-replay's other 2D-vector fields (impact_point, impact_normal, position, etc.) are also 2D-locked.

The whole prototype is 2D so this is consistent today. But if BP6+ ships 3D chassis collision, every 2D-locked field becomes a migration burden.

**Recommended fix (P3).** Document in the schema `description` that this is 2D-locked and that 3D producers (post-BP6) would need an envelope-bump migration. Or proactively widen `maxItems` to 3 with `minItems: 2` so producers can additively pass through 3D vectors at the cost of consumers ignoring the z-component. Most readable fix is to document the 2D lock and defer the question.

### NEW-L: Field-naming case consistency

**Severity: LOW**

**Finding.** Most armor.* / combat.projectile_hit_mo / audio.event_requested field names are snake_case (e.g. `era_panel_id`, `ap_round_tier`, `hit_zone`, `surface_kind`, `damage_kind`, `internal_hit_kind`). But some **enum values** mix conventions:

- `layer` enum: `["External", "Internal", "Core"]` — **PascalCase** (mirrors Rust enum variants `ArmorLayer::External`).
- `ap_round_tier` enum: `["standard", "armor_piercing", "hardened_AP", "discarding_sabot", "explosive_warhead", "kinetic_impact", "HEAT", "APFSDS"]` — **MIXED**:
  - `standard`, `discarding_sabot`, `explosive_warhead`, `kinetic_impact` — snake_case
  - `armor_piercing` — snake_case
  - `hardened_AP` — snake + UPPERCASE acronym suffix
  - `HEAT`, `APFSDS` — all-uppercase acronyms
- `damage_kind` enum: snake_case throughout.
- `surface_kind` enum: snake_case throughout.

The mixed-case in `ap_round_tier` came verbatim from the spec table. It's per-spec but ugly. A future tightening would unify to snake_case (e.g. `hardened_ap`, `heat`, `apfsds`).

`layer` PascalCase mirrors a Rust enum convention; the comparable `breach_kind` is snake_case. Consistency would say "all enum values are snake_case unless the spec literally locks PascalCase". The spec locks `ArmorLayer = External | Internal | Core` which IS PascalCase.

**Recommended fix (P3).** No-op at M5; flag for M6 + M13 producer authors that these conventions are inconsistent and the schema follows the spec verbatim. Document in schema descriptions.

**Side note:** I did NOT find any camelCase or PascalCase field NAMES in the audited schemas — only enum VALUES. Pass-1 audit fixed the Origin enum spelling drift (`HeavyBioMech` → `HeavyBiomech`); pass-2 confirms no similar drift in armor / combat / audio scopes.

### NEW-M: `audio.event_requested` Pierce variant semantic ambiguity

**Severity: LOW**

**Finding.** The shipped schema accepts `kind: "material_state"` + `impact_state: "pierce"` + any of the 8 `surface_kind` enum values. The M5 sound-clip table has "Pierce" only as a per-armor-material column (rows = 7 materials × column "Pierce" = 7 pierce variants). The 8-value `surface_kind` enum includes:

- `armor_external`, `armor_internal`, `armor_core`, `armor_chunked_breach` — armor surfaces; pierce sensible here
- `flesh`, `circuit` — internal targets; pierce here should arguably be a `kind: internal_hit` event with `internal_hit_kind: flesh_punctured / circuit_sparked`, not a `kind: material_state` + `impact_state: pierce`
- `unarmored` — unclear semantic
- `terrain` — pierce on terrain probably means terrain.terrain_carved or terrain.terrain_pixel_dislodged, not an audio.event_requested

So `kind: material_state + surface_kind: flesh + impact_state: pierce` is semantically odd — it would be more correctly emitted as `kind: internal_hit + internal_hit_kind: flesh_punctured`.

The lack of cross-field constraint (NEW-B) means the schema accepts the odd combination.

**Recommended fix (P2).** This is partly subsumed by NEW-B (oneOf discriminator). On top of NEW-B, add documentation in the schema description clarifying that `kind: material_state` should only be emitted when `surface_kind ∈ [armor_external, armor_internal, armor_core, armor_chunked_breach]`, and that `kind: internal_hit` should only be emitted when `surface_kind ∈ [flesh, circuit]`.

---

## End-to-end verification

### `cf-mod validate game/crates/cf-replay/schemas/`

```
scanned=131 pass=131 warn=0 fail=0
```

All 131 schemas under cf-replay/schemas/ pass cf-mod validation. Exit code 0.

The 21 in-scope schemas in this audit (19 armor.* + combat.projectile_hit_mo + audio.event_requested) all PASS.

### Hard test 1: combat.projectile_hit_mo with the legacy `parent_event_id` field

**Bundle:** `/tmp/m5-pass2-bundle-1/events.jsonl`
```json
{"category":"combat","event_type":"projectile_hit_mo","tick":100,
 "payload":{...,"parent_event_id":"run:42:4"}}
```

**Expected:** FAIL with "required field `parent_hit_event_id` missing".

**Actual:** FAIL ✓
```json
{
  "events_checked": 1,
  "failures": [{
    "category": "combat",
    "event_id": "run:42:5",
    "event_type": "projectile_hit_mo",
    "reason": "combat.projectile_hit_mo: required field `parent_hit_event_id` missing"
  }]
}
```

Pass-1 fix verified.

### Hard test 2: audio.event_requested with `kind: material_state` + non-null `internal_hit_kind` (semantic incoherence)

**Bundle:** `/tmp/m5-pass2-bundle-2/events.jsonl`
```json
{"category":"audio","event_type":"event_requested","tick":100,
 "payload":{"kind":"material_state","material":"metal","impact_state":"pristine_hit",
            "internal_hit_kind":"flesh_punctured","surface_kind":"armor_external",
            "damage_kind":"kinetic","source_event_id":"run:42:5"}}
```

**Expected:** FAIL (the kind discriminator should gate internal_hit_kind to null when kind=material_state).

**Actual:** PASS (incorrectly ACCEPTED). NEW-B documents the gap.
```json
{"events_checked": 1, "failures": []}
```

### Hard test 3: armor.ricochet with ricochet_probability = 1.5

**Bundle:** `/tmp/m5-pass2-bundle-3/events.jsonl`
```json
{"category":"armor","event_type":"ricochet","tick":100,
 "payload":{"impact_angle":1.2,"ricochet_probability":1.5,"was_ricocheted":true,
            "deflection_vector":[0.5,0.5]}}
```

**Expected:** FAIL (probability > 1.0 is non-physical).

**Actual:** PASS (incorrectly ACCEPTED). NEW-F documents the gap.

### Hard test 4: armor.spalling with fragment_count = 99 (spec locks 1-3)

**Bundle:** `/tmp/m5-pass2-bundle-4/events.jsonl`
```json
{"category":"armor","event_type":"spalling","tick":100,
 "payload":{"item_id":12,"zone":"torso","layer":"External","fragment_count":99,
            "damage_per_fragment":50.0,"cause_event_id":"run:42:5"}}
```

**Expected:** FAIL (spec locks 1-3 fragments).

**Actual:** PASS (incorrectly ACCEPTED). NEW-J documents the gap (pass-1 audit recommendation was not applied).

### Hard test 5: audio.event_requested with `kind: material_state` + no material + no impact_state

**Bundle:** `/tmp/m5-pass2-bundle-5/events.jsonl`
```json
{"category":"audio","event_type":"event_requested","tick":100,
 "payload":{"kind":"material_state","surface_kind":"armor_external",
            "damage_kind":"kinetic","source_event_id":"run:42:5"}}
```

**Expected:** FAIL (a material_state event should require both material and impact_state).

**Actual:** PASS (incorrectly ACCEPTED). Subsumed by NEW-B.

### Hard test 6: armor.apfsds_penetrated with rod_length_remaining_mm = 1e30

**Bundle:** `/tmp/m5-pass2-bundle-6/events.jsonl`
```json
{"category":"armor","event_type":"apfsds_penetrated","tick":100,
 "payload":{"item_id":12,"zone":"torso","rod_length_remaining_mm":1.0e30}}
```

**Expected:** FAIL or WARN (1e30 mm is non-physical).

**Actual:** PASS (ACCEPTED). NEW-G documents the gap.

### Pass-1 test suite

- `cargo test -p cf-replay` → 39 passed; 0 failed.
- `cargo test -p cf-mod` → 20 unit + 11 integration → 31 passed; 0 failed.

All pass-1 introduced tests (`m5_combat_projectile_hit_mo_rejects_envelope_named_parent`, `m5_concussion_dose_changed_rejects_bad_origin`, `m5_per_family_happy_path`, `m5_schemas_declare_schema_version_v0_1`) PASS.

---

## Recommended fixes (prioritized)

**P0 (blockers for M6 if M6 introduces a cf-audio consumer or a cause-chain walker):**

1. **NEW-B**: Add a `oneOf` discriminator on `audio.event_requested.payload` so `kind: material_state` REQUIRES `material` + `impact_state` (both non-null) and FORBIDS `internal_hit_kind` (must be null/absent); `kind: internal_hit` REQUIRES `internal_hit_kind` and FORBIDS `material` + `impact_state`. Extend `cf-replay::schemas::validate_event_payload` to support a top-level `oneOf` on the payload sub-schema. Lock with new tests `m5_audio_rejects_material_state_with_internal_hit_kind` + `m5_audio_rejects_internal_hit_without_internal_hit_kind`.

**P1 (semantic integrity; should land before M13 producer work begins):**

2. **NEW-A**: Add optional `source_hit_event_id: string` to every armor.* event payload that is plausibly downstream of a combat.projectile_hit_mo (16 events listed in NEW-A). Stable naming convention (`source_event_id` is the most popular existing name; pick it and document).
3. **NEW-C**: Add `source_hit_event_id` (required) to `armor.ricochet.payload.required`. Optional: add denormalized `ap_round_tier` so consumers can bound-check the ricochet threshold without walking the cause-chain.
4. **NEW-F**: Tighten `armor.ricochet.ricochet_probability` with `"maximum": 1.0` (1-line schema change).
5. **NEW-J**: Tighten `armor.spalling.fragment_count` with `"minimum": 1, "maximum": 3` (pass-1 audit recommendation that was not applied; 1-line schema change). Add test `m5_armor_spalling_rejects_fragment_count_outside_1_to_3`.

**P2 (locked-taxonomy reference; M13 producer aid):**

6. **NEW-D**: Rename `armor.debris_spawned.payload.record_id` → `debris_record_id` for consistency with `armor.chunked_off.payload.debris_record_id`. Document the cross-event id relationship in both schema descriptions. Add `source_hit_event_id` to both (per NEW-A).
7. **NEW-E**: Add a documented `oneOf{organ_id_enum | circuit_id_enum | open_module_id}` constraint on `armor.he_overpressure_wave.modules_affected[i]` and `armor.penetration_ray_traversed.modules_hit[i][0]`. Or document in description if validator unsupported.
8. **NEW-H**: Constrain `armor.debris_spawned.material` to a canonical material registry (probably cf-material MaterialId). Ship `cf-replay/schemas/tables/armor_types.v0_1.json` as the machine-readable companion to the spec's 9-armor-type table (pass-1 recommendation that was not applied).
9. **NEW-M**: Add documentation cross-referencing `audio.event_requested.kind` with `surface_kind` semantics (mostly subsumed by NEW-B).

**P3 (tightening; nice-to-have):**

10. **NEW-G**: Add `"maximum": 2000.0` to `armor.apfsds_penetrated.rod_length_remaining_mm`. Producer-error prevention.
11. **NEW-I**: Either tighten the `["integer", "string"]` polymorphism on `era_panel_id` / `schurzen_plate_id` / `panel_id` / `defeated_round_id` to a single canonical type now, OR defer to M13 lock decision with a documented readiness note in each schema's description.
12. **NEW-K**: Document `armor.ricochet.deflection_vector` 2D lock in the schema description; flag for BP6+ 3D migration.
13. **NEW-L**: Document the mixed-case enum convention (`ap_round_tier` mixing snake_case + UPPERCASE acronyms; `layer` using PascalCase) in the affected schemas' descriptions. Per-spec; no functional change.

---

## Summary

- **Pass-1 deliveries verified: 9/9 fully shipped + 1 partial (the `audio.event_requested.kind` discriminator is structurally present but doesn't gate dependent fields — NEW-B).**
- **New issues found (pass-2): 13** (NEW-A through NEW-M).
  - Critical (P0): **1** (NEW-B).
  - High (P1): **4** (NEW-A, NEW-C, NEW-F, NEW-J).
  - Medium (P2): **4** (NEW-D, NEW-E, NEW-H, NEW-M).
  - Low (P3): **4** (NEW-G, NEW-I, NEW-K, NEW-L).
- **M6 readiness blockers:**
  - NEW-B (audio kind discriminator): only a blocker if M6 ships any consumer of `audio.event_requested`. M6 is the "Reactor Defense Scenario" / sister-of-M9 milestone; per the pass-1 commit message "M6 introduces 3 new categories (perception/squad/inventory)" — none of those consume audio. So NEW-B is not a strict M6 blocker but it IS a blocker before M13.x cf-audio implementer can write against a contract.
  - NEW-A (cause-chain uniformity): not an M6 blocker (M6 doesn't ship cause-chain consumers per the spec); becomes a blocker at M10 (cause-chain walk surface) and at M14 (full collision producer needs to populate the pointers).
  - NEW-F + NEW-J (probability + fragment-count bounds): not a strict M6 blocker but quality gates that pass-1 recommended and pass-1 missed; should land before M13 to prevent producer-side drift.

**Net assessment.** Pass-1 closed the 17 audit findings as claimed at the structural level. The pass-1 implementation work was real — schemas were rewritten, the audio.event_requested schema was shipped, the cosmetic duplicate was dropped, the parent_event_id rename was applied with a regression test. But pass-1 also:

1. **Missed applying its own recommendation on `armor.spalling.fragment_count`** (NEW-J — the audit report said "Suggested fix: tighten `armor_spalling.json` to `fragment_count: { minimum: 1, maximum: 3 }`" but the shipped schema still has `minimum: 0` and no maximum).
2. **Shipped the audio discriminator structurally without the cross-field constraint** (NEW-B — the `kind` enum exists but doesn't gate the dependent fields, so semantic incoherence passes validation).
3. **Did not address the cause-chain naming drift** that the pass-1 fix itself introduced (NEW-A — renaming `parent_event_id` → `parent_hit_event_id` consolidated with origin.shot_force_feedback's `parent_hit_event_id` but did NOT consolidate with the wider M5 surface's `source_event_id` / `source_hit_event_id` / `cause_event_id` / `ignition_source_event_id` family — five names for one concept).

These 3 gaps are real but bounded; they don't reopen the M5 milestone close because M5 spec acceptance criteria are met (every schema exists at v0.1; cf-mod validate exits 0; each schema declares the canonical schema_version; schemas accept producer events from later milestones additively). The gaps are quality / semantic-tightening items that should land as a pass-3 hardening pass before M13 / M14 producers begin populating armor.* events.
