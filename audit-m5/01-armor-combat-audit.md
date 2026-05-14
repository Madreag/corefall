# M5 Audit — armor.* + combat.projectile_hit_mo

Audit date: 5/13/2026
Scope: `armor.*` family (19 events) + `combat.projectile_hit_mo` expanded payload, audited literally against `/Users/erol/projects/corefall/specs/done/M5.md` §"armor.* family" and §"combat.projectile_hit_mo expanded payload".

Methodology:
- Read every shipped schema under `game/crates/cf-replay/schemas/event/armor_*.json` + `combat_projectile_hit_mo.json`.
- Cross-checked each event's `category.const` + `event_type.const` against the spec's `<family>.<type>` token.
- Cross-checked the schema's `payload.properties` keys + `payload.required` array against the spec's `{ field1, field2, ... }` brace block, field-by-field.
- Cross-checked each `enum` set against the M5 locked tables (round tiers, breach kinds, body zones, armor layers, surface kinds, damage kinds, organ ids, circuit ids).
- Verified registration in `cf-replay/src/schemas.rs::event_schema_for`.
- Searched the schema tree for the locked taxonomies M5 declares (ricochet thresholds, formulas, armor types, sound-clip variants, internal-hit sound names).

---

## Per-event verdict table

| # | Event | Schema file | Verdict | Notes / Gaps |
|---|---|---|---|---|
| 1 | `armor.layer_hp_changed` | `armor_layer_hp_changed.json` | PASS | All spec fields present (`actor_id, item_id, zone, layer, from, to, cause, ap_factor`); zone enum = 15 zones; layer enum = External/Internal/Core; all 8 fields are `required`. Note: `actor_id` is duplicated at envelope-level (typed `["integer","null"]`) and payload-level (typed `integer` — spec lists it in the brace block so this is correct). |
| 2 | `armor.layer_critical` | `armor_layer_critical.json` | PASS | Fields `item_id, zone, layer, hp` all present and required. |
| 3 | `armor.layer_destroyed` | `armor_layer_destroyed.json` | PASS | Fields `item_id, zone, layer, breach_kind` present and required; `breach_kind` enum = `["punctured","shattered","melted","chemically_corroded"]` — matches spec exactly. |
| 4 | `armor.all_layers_destroyed` | `armor_all_layers_destroyed.json` | PASS | `item_id, zone` present and required. |
| 5 | `armor.chunked_off` | `armor_chunked_off.json` | PASS | `item_id, zone, debris_record_id, impact_impulse, debris_kind, ground_position` all present and required. `ground_position` is a 2-element number array. Note: `debris_kind` is `type: string` with no enum — the spec doesn't enumerate kinds, so this is per-spec, but a future M13 fill might benefit from a locked enum. |
| 6 | `armor.debris_spawned` | `armor_debris_spawned.json` | PASS | `record_id, kind, material, position, velocity, can_pickup` all present and required. `material` accepts `["integer","string"]` (MaterialId can be either, per the envelope conventions). |
| 7 | `armor.repaired` | `armor_repaired.json` | PASS | `item_id, zone, layer, restored_hp, repaired_by_actor_id` all present and required. |
| 8 | `armor.angle_deflection_calculated` | `armor_angle_deflection_calculated.json` | PASS | `impact_angle, nominal_mm, effective_mm` present and required. Description carries the locked formula `effective_mm = nominal_mm / cos(angle)`. Minor: spec uses `impact_angle_rad` as the formula variable; schema field name is `impact_angle` (no unit suffix — radians implied by description). |
| 9 | `armor.ricochet` | `armor_ricochet.json` | PASS | `impact_angle, ricochet_probability, was_ricocheted, deflection_vector` present and required. **GAP-A**: payload lacks any `ap_round_tier` field, so the consumer cannot recover which tier's threshold was tested without joining on the parent `combat.projectile_hit_mo` event. Spec block doesn't list it, so this is per-spec, but worth a producer-side audit at M13 to confirm the cause-chain is sufficient. |
| 10 | `armor.spalling` | `armor_spalling.json` | PASS | `item_id, zone, layer, fragment_count, damage_per_fragment, cause_event_id` present and required. Description carries the locked formula `if damage_to_armor > armor.spalling_threshold, spawn 1-3 fragments at random angles within 30° forward cone, each carrying 0.2-0.5 of original damage` — verbatim from spec. |
| 11 | `armor.penetration_ray_traversed` | `armor_penetration_ray_traversed.json` | PASS | `ray_origin, ray_direction, modules_hit, final_resting_point` present and required. `modules_hit` is a tuple-array `[[module_id, damage], ...]` matching `Vec<(module_id, damage)>` — typed correctly (module_id `["integer","string"]`, damage `number`). |
| 12 | `armor.he_overpressure_wave` | `armor_he_overpressure_wave.json` | PASS | `center, radius, overpressure_pa, modules_affected` present and required. |
| 13 | `armor.heat_jet_penetrated` | `armor_heat_jet_penetrated.json` | PASS | `item_id, zone, layer, jet_depth_mm` present and required. |
| 14 | `armor.heat_jet_pre_detonated_by_era` | `armor_heat_jet_pre_detonated_by_era.json` | PASS | `item_id, zone, era_panel_id` present and required. |
| 15 | `armor.apfsds_penetrated` | `armor_apfsds_penetrated.json` | PASS | `item_id, zone, rod_length_remaining_mm` — exactly as spec. Spec brace block does NOT include `layer`; schema correctly omits it (a tactical note: when M13 ships chassis APFSDS, consumers may need to know which layer absorbed how much rod-length, but that's an M13 producer concern and the spec is unambiguous here). |
| 16 | `armor.era_panel_detonated` | `armor_era_panel_detonated.json` | PASS | `item_id, zone, era_panel_id, defeated_round_id` present and required. |
| 17 | `armor.schurzen_pre_detonated` | `armor_schurzen_pre_detonated.json` | PASS | `item_id, zone, schurzen_plate_id, defeated_round_id` present and required. |
| 18 | `armor.multi_hit_degradation` | `armor_multi_hit_degradation.json` | PASS | `item_id, zone, layer, hits_received, hardness_remaining` present and required. |
| 19 | `armor.reactive_armor_consumed` | `armor_reactive_armor_consumed.json` | PASS | `item_id, zone, panel_id` present and required. |
| 20 | `combat.projectile_hit_mo` | `combat_projectile_hit_mo.json` | PASS (with notes) | See "combat.projectile_hit_mo deep audit" below. |

Registration check: all 19 armor.* schemas + `combat.projectile_hit_mo` are wired into `event_schema_for` in `cf-replay/src/schemas.rs` (lines 244–263 + 314 in the current source). The `schemas_load_for_every_registered_event_type` test (`cf-replay/src/schemas.rs::tests`) and the M5-specific `m5_schemas_declare_schema_version_v0_1` test enumerate all 20 of these pairs and would fail-loudly if any pair were dropped. **PASS.**

Envelope-shape check: every armor.* schema declares the M4 v0.1 envelope shape — `properties.schema_version.const = "0.1"`, `properties.category.const = "armor"`, `properties.event_type.const = "<type>"`, with the actual payload nested under `properties.payload.properties` + `properties.payload.required`. The validator in `cf-replay/src/schemas.rs::validate_event_payload` walks into `properties.payload` when it detects this shape (line 343–354). **PASS.**

---

## combat.projectile_hit_mo deep audit

Spec block has 6 sections (who / physical / armor / damage / internal / attribution). Field-by-field:

### Section: who shot what at whom

| Spec field | Schema | Required | Type / Enum |
|---|---|---|---|
| `shooter_id: ActorId` | present | yes | `integer` ✓ |
| `weapon_id: ItemId` | present | yes | `integer` ✓ |
| `projectile_id: ProjectileId` | present | yes | `integer` ✓ |
| `target_id: ActorId` | present | yes | `integer` ✓ |
| `hit_zone: BodyZone` | present | yes | 15-zone enum ✓ |

### Section: physical truth

| Spec field | Schema | Required | Type / Enum |
|---|---|---|---|
| `impact_point: Vec2` | present | yes | 2-elem number array ✓ |
| `impact_normal: Vec2` | present | yes | 2-elem number array ✓ |
| `impact_impulse: f32` | present | yes | `number` ✓ |
| `impact_energy_j: f32` | present | yes | `number` ✓ |

### Section: armor characterization

| Spec field | Schema | Required | Type / Enum |
|---|---|---|---|
| `ap_factor: f32` | present | yes | `number` ✓ |
| `ap_round_tier: '...'` (8 tiers) | present | yes | enum: `["standard","armor_piercing","hardened_AP","discarding_sabot","explosive_warhead","kinetic_impact","HEAT","APFSDS"]` ✓ exact match with spec table |
| `material_at_impact: MaterialId` | present | yes | `["integer","string"]` ✓ |
| `surface_kind: '...'` (8 kinds) | present | yes | enum: `["armor_external","armor_internal","armor_core","armor_chunked_breach","flesh","circuit","unarmored","terrain"]` ✓ exact match |
| `armor_effective_hardness: f32` | present | yes | `number` ✓ |
| `armor_absorbed_dmg: f32` | present | yes | `number` ✓ |
| `passthrough_dmg: f32` | present | yes | `number` ✓ |

### Section: damage result

| Spec field | Schema | Required | Type / Enum |
|---|---|---|---|
| `damage_kind: '...'` (5 kinds) | present | yes | enum: `["kinetic","thermal","electric","chemical","radiation"]` ✓ exact match |
| `hp_before: f32` | present | yes | `number` ✓ |
| `hp_after: f32` | present | yes | `number` ✓ |
| `damage_amount: f32` | present | yes | `number` ✓ |
| `layer_struck: Option<ArmorLayer>` | present | no | enum: `["External","Internal","Core",null]`, type `["string","null"]` ✓ |
| `pierced_armor: bool` | present | yes | `boolean` ✓ |

### Section: internal damage routing

| Spec field | Schema | Required | Type / Enum |
|---|---|---|---|
| `organ_damaged_id: Option<OrganId>` | present | no | full 15-organ enum + `null` ✓ — names match `internal.*` spec block (`brain, eyes_left, eyes_right, ears_left, ears_right, heart, lungs_left, lungs_right, liver, kidneys_left, kidneys_right, spine, stomach, intestines, pancreas`) |
| `circuit_damaged_id: Option<CircuitId>` | present | no | full 12-circuit enum + `null` ✓ — names match (`power_core, cpu, sensor_array, motor_controller_left_arm, motor_controller_right_arm, motor_controller_left_leg, motor_controller_right_leg, hydraulic_pump, coolant_pump, oil_reservoir, fuel_tank, comm_relay`) |

### Section: attribution

| Spec field | Schema | Required | Type / Enum |
|---|---|---|---|
| `parent_event_id: EventId` | present | yes | `string` ✓ |
| `cosmetic: false` | **duplicated** | no | `const: false` ✓ |

**Note B (minor):** `cosmetic` is declared at **both** the envelope-level (`properties.cosmetic.const = false`) and the payload-level (`payload.properties.cosmetic.const = false`). The spec block lists `cosmetic: false` inside the payload braces; the M4 envelope schema also declares `cosmetic` as an optional envelope field. The duplication is benign (both `const: false`) but it's a minor schema-shape oddity worth noting. Recommend: drop the payload-level duplicate at the next additive revision OR keep both as a belt-and-suspenders contract that the canonical hit event is never cosmetic.

**Verdict: PASS** — every spec field is present, typed, and required where the spec implies it. Every enum exactly matches the spec's locked sets. The `cosmetic` duplication is the only drift and it's behavior-neutral.

---

## Locked taxonomy / formula coverage

### Per-ammo ricochet thresholds table

**Spec (LOCKED):**

| Round tier | Ricochet threshold (deg) |
|---|---|
| `standard` | 60 |
| `armor_piercing` | 65 |
| `hardened_AP` | 70 |
| `discarding_sabot` | 75 |
| `explosive_warhead` | 50 |
| `kinetic_impact` | 70 |
| `HEAT` | 75 |
| `APFSDS` | 80 |

**Shipped coverage:**

- 8 tier **names** captured as the `ap_round_tier` enum in `combat_projectile_hit_mo.json` (lines 27). ✓
- 8 tier **degree thresholds** documented in the `description` string of `combat_projectile_hit_mo.json` verbatim: `"Per-ammo ricochet thresholds (deg): standard 60, armor_piercing 65, hardened_AP 70, discarding_sabot 75, explosive_warhead 50, kinetic_impact 70, HEAT 75, APFSDS 80."`
- NOT encoded as a const lookup / referenceable enum/const table. A consumer or producer parsing the schemas programmatically would have to pull the values out of the description string with regex.

**Verdict:** **PARTIAL** — names are validatable, threshold degrees are prose-only. Spec says "LOCKED" which implies they should be referenceable from a machine-readable place.

**Suggested fix:** add a sibling lookup file at `cf-replay/schemas/event/_ricochet_thresholds.json` (or fold into the schema's description as a structured JSON snippet within the `description`, or embed as `"x-corefall-ricochet-thresholds": { "standard": 60, ... }` as an `x-` vendor extension on the schema root). The cleanest fix is a separate `cf-replay/schemas/tables/ricochet_thresholds.v0_1.json` data file plus a unit test that asserts the values match the spec.

### Effective thickness formula

**Spec (LOCKED):** `effective_thickness_mm = nominal_thickness_mm / cos(impact_angle_rad)`

**Shipped coverage:**

- `armor_angle_deflection_calculated.json` description: `"records the impact angle and the resulting effective_mm = nominal_mm / cos(angle)"` ✓ (variable names slightly compressed but formula correct)
- `combat_projectile_hit_mo.json` description: `"Effective thickness formula (locked): effective_thickness_mm = nominal_thickness_mm / cos(impact_angle_rad)"` ✓ verbatim

**Verdict:** **PASS** — formula is referenceable in two schemas.

**Minor note:** the schema field is `impact_angle` (no unit suffix), but the spec's formula uses `impact_angle_rad`. A future field rename to `impact_angle_rad` would clarify, but the current name is consistent across `armor.ricochet` (which uses **degrees** for threshold checks per the table above — `60..80°`) and `armor.angle_deflection_calculated` (which uses **radians** per the formula). This is a unit-ambiguity carried over from the spec itself, not a schema drift, but worth flagging for M13 producer authors so they don't conflate the two.

### Spalling formula

**Spec (LOCKED):** `if damage_to_armor > armor.spalling_threshold, spawn 1-3 fragments at random angles within 30° forward cone, each carrying 0.2-0.5 of original damage.`

**Shipped coverage:**

- `armor_spalling.json` description carries the formula **verbatim**. ✓
- Producer-side constants (`spalling_threshold`, the 1–3 fragment count, the 30° cone half-angle, the 0.2–0.5 damage fraction range) are **not** encoded as schema constraints — they're prose. `fragment_count` is constrained to `minimum: 0` only (the spec says 1–3, so technically a producer emitting `fragment_count = 0` would still pass schema validation even though that's semantically a non-event). `damage_per_fragment` has no constraint at all.

**Verdict:** **PARTIAL** — formula is referenceable in prose; the numeric ranges (1–3 fragments, 0.2–0.5 damage fraction, 30° cone) are not schema-validated.

**Suggested fix:** tighten `armor_spalling.json` to `"fragment_count": { "type": "integer", "minimum": 1, "maximum": 3 }` and document the 0.2–0.5 ratio + 30° cone in a vendor-extension key. Optional; not strictly required by spec.

### Armor types table (9 types)

**Spec:**

| Type | Hardness | Spalling | AP Resistance | HE Resistance | Special |
|---|---|---|---|---|---|
| Cast Iron | 0.6 | High (3 frag) | Low | Medium | Cracks easily |
| RHA | 0.8 | Medium (2 frag) | Medium | Medium | Standard |
| Composite | 0.85 | Low (1 frag) | High | Low | Layered |
| Ceramic | 0.95 | None | Very High vs AP | Low vs HE | Multi-hit degradation |
| Reactive (ERA) | 0.6 | None | Very High vs AP | Medium | One-shot per panel |
| Spaced | 0.7 | Low | Medium | Very High vs HEAT | Air gap |
| Schurzen | 0.4 | None | Low | Medium vs HEAT | Pre-detonates HEAT |
| Composite-ceramic | 0.92 | None | Very High | Medium | Endgame |
| Sloped Cast Steel | 0.7 | Medium | Medium (angled) | Medium | Natural 30-45° |

Spec annotation: "M13 fills producer."

**Shipped coverage:**

- The 9 type names are NOT referenced in any M5 schema. No enum. No const list. No description block.
- `combat.projectile_hit_mo.material_at_impact` uses an open-ended `["integer","string"]` MaterialId — the canonical material catalog is presumably resolved at M13.
- The spec text says "M13 fills producer" — but the spec also has an explicit closed list of 9 types that are LOCKED in M5 prose. The taxonomy IS locked at M5; the producer's data-driven content at M13 is what consumes the locked enum.

**Verdict:** **GAP** (deferred to producer fill, but the locked taxonomy is not discoverable from M5 schema outputs alone).

**Suggested fix:** add a `cf-replay/schemas/tables/armor_types.v0_1.json` reference file listing the 9 names + the spec's hardness / spalling fragment count / AP-res / HE-res / special metadata. This becomes the canonical machine-readable companion to the spec table and locks the names at M5 (before any M13 producer can drift them).

### Sound clip variants (audio.event_requested request shape)

**Spec:**

> "All sound events emit `audio.event_requested` with `kind: material_state` + `surface_kind` + `damage_kind`. M13.x+ cf-audio consumes; **M5 just locks the request shape**."

The spec is explicit: M5 is supposed to lock the request shape for `audio.event_requested`.

The spec also defines the LOCKED material × state taxonomy (7 materials × 5 states = 35 clip variants):

| Material | Pristine hit | Cracked hit | Destroyed hit | Chunked-off | Pierce |
|---|---|---|---|---|---|
| `metal` | … | … | … | … | … |
| `ceramic` | … | … | … | … | … |
| `composite` | … | … | … | … | … |
| `cloth` | … | … | … | … | … |
| `leather` | … | … | … | … | … |
| `hardened_plate` | … | … | … | … | … |
| `reactive_armor` | … | … | … | … | … |

**Shipped coverage:**

- No `audio.event_requested` schema exists. Searches for `audio` in `cf-replay/schemas/event/` return zero matches. ❌
- No registration in `event_schema_for` for `("audio", "event_requested")`. ❌
- Neither the 7-material enum (`metal, ceramic, composite, cloth, leather, hardened_plate, reactive_armor`) nor the 5-impact-state enum (`pristine_hit, cracked_hit, destroyed_hit, chunked_off, pierce`) is captured anywhere as a schema constraint. ❌

**Verdict:** **GAP — CRITICAL.** Spec explicitly says "M5 just locks the request shape" but the request shape is not locked anywhere. M13.x cf-audio implementer has no schema to conform to.

**Suggested fix (highest priority of any item in this audit):** ship a new schema `cf-replay/schemas/event/audio_event_requested.json` at v0.1 with payload:

```json
{
  "payload": {
    "type": "object",
    "properties": {
      "kind": { "const": "material_state" },
      "material": { "type": "string", "enum": ["metal","ceramic","composite","cloth","leather","hardened_plate","reactive_armor"] },
      "impact_state": { "type": "string", "enum": ["pristine_hit","cracked_hit","destroyed_hit","chunked_off","pierce"] },
      "surface_kind": { "type": "string", "enum": ["armor_external","armor_internal","armor_core","armor_chunked_breach","flesh","circuit","unarmored","terrain"] },
      "damage_kind": { "type": "string", "enum": ["kinetic","thermal","electric","chemical","radiation"] },
      "source_event_id": { "type": "string" }
    },
    "required": ["kind","material","impact_state","surface_kind","damage_kind","source_event_id"]
  }
}
```

Wire it into `event_schema_for` as `("audio", "event_requested") => Some(SCHEMA_AUDIO_EVENT_REQUESTED)`.

---

## Internal hit sound taxonomy (flesh_punctured / bone_cracked / organ_ruptured / circuit_sparked / circuit_destroyed / fluid_pierce)

**Spec:**

| Internal hit | Sound |
|---|---|
| `flesh_punctured` | Wet squelch |
| `bone_cracked` | Snap |
| `organ_ruptured` | Wet rupture |
| `circuit_sparked` | Electric zap |
| `circuit_destroyed` | Buzz + smoke |
| `fluid_pierce` | Liquid splash |

**Shipped coverage:**

- Grep for these 6 names across `cf-replay/schemas/event/`: zero matches. ❌
- No `audio.event_requested` schema captures them as an enum.
- The closest existing enum is `surface_kind` in `combat.projectile_hit_mo` (`armor_external, armor_internal, armor_core, armor_chunked_breach, flesh, circuit, unarmored, terrain`) — but this is the surface struck, not the internal-hit sound name.

**Verdict:** **GAP — HIGH.** Six locked names, none encoded.

**Suggested fix:** include an `internal_hit_kind` enum on the same proposed `audio.event_requested` schema:

```json
"internal_hit_kind": {
  "type": ["string","null"],
  "enum": ["flesh_punctured","bone_cracked","organ_ruptured","circuit_sparked","circuit_destroyed","fluid_pierce",null]
}
```

Optional + nullable because the request shape is `kind: material_state` for external-armor hits (where `internal_hit_kind` is null) and `kind: internal_hit` (or similar) when the hit is an internal-ray traversal sound.

Recommend two `kind` const variants on the schema: `material_state` and `internal_hit`. The `kind` field becomes a discriminator (`oneOf`-style branching on the schema, if validator supports it).

---

## Cross-cutting issues

### `cosmetic` field duplication on `combat.projectile_hit_mo`

The schema declares `cosmetic: { const: false }` at **two levels**:
1. `properties.cosmetic` (envelope-level — line 13 of the schema)
2. `payload.properties.cosmetic` (payload-level — line 49 of the schema)

Spec only shows `cosmetic: false` once, inside the payload braces. The envelope-level `cosmetic` field is already defined by the M4 envelope schema (`recorder_event.schema.json`) as a top-level optional boolean. Having it locked to `const: false` on the M5 hit-mo schema is correct (this canonical event is never cosmetic), but having it BOTH at envelope-level AND payload-level is mild over-specification. Behavior-neutral; both consts agree.

### `armor.ricochet` doesn't carry `ap_round_tier`

The spec brace block for `armor.ricochet` is `{ impact_angle, ricochet_probability, was_ricocheted: bool, deflection_vector }`. Consumers wanting to verify the M5 ricochet-threshold table at replay time would need to walk the cause-chain back to the parent `combat.projectile_hit_mo` event (which has `ap_round_tier` + `impact_angle`) to recover the (`ap_round_tier`, `threshold_deg`) pair. The schema follows the spec literally and is per-spec.

Not a bug; flagged for awareness of M13 producer + M10 cause-chain implementer.

### Unit ambiguity: `impact_angle` (degrees vs radians)

- `armor.ricochet`: threshold table is in **degrees** (50–80°).
- `armor.angle_deflection_calculated`: formula uses `cos(angle)`, which mathematically requires **radians**.
- Both events expose the same field name `impact_angle` with no unit suffix.

Spec is the same on this point; schemas follow spec. Producer authors at M13 should be alerted that the same field carries different units in different events, OR that there's a single canonical unit (radians) and the threshold table in degrees is a presentation/spec convention.

Not a schema gap, but worth surfacing in the M13 readiness note.

---

## Recommended fixes (prioritized)

1. **CRITICAL: ship `audio.event_requested` schema (M5 spec mandate).** Create `cf-replay/schemas/event/audio_event_requested.json` with the request-shape payload (kind, material, impact_state, surface_kind, damage_kind, source_event_id, optional internal_hit_kind). Wire into `event_schema_for` under `("audio", "event_requested")`. This is the only spec-mandated schema that is wholly missing. Without it, the M5 promise to "lock the request shape" is unmet and M13.x cf-audio has no contract to fulfill.

2. **HIGH: encode the 7-material × 5-impact-state sound clip taxonomy** as enums on the new `audio.event_requested` schema (per spec table). 7 materials = `metal, ceramic, composite, cloth, leather, hardened_plate, reactive_armor`. 5 impact states = `pristine_hit, cracked_hit, destroyed_hit, chunked_off, pierce`.

3. **HIGH: encode the 6-name internal-hit sound enum** (`flesh_punctured, bone_cracked, organ_ruptured, circuit_sparked, circuit_destroyed, fluid_pierce`) on the same `audio.event_requested` schema.

4. **MEDIUM: encode the 9-armor-type locked taxonomy** as a machine-readable companion at `cf-replay/schemas/tables/armor_types.v0_1.json` (or vendor-extension key on `snapshot_armor.json`). Names + hardness + spalling fragment count + AP-res + HE-res + special.

5. **MEDIUM: encode the per-ammo ricochet threshold table** at `cf-replay/schemas/tables/ricochet_thresholds.v0_1.json` so the values are referenceable from code without parsing the description prose. Add a unit test in `cf-replay/tests/` that asserts the values match the spec.

6. **LOW: tighten `armor_spalling.json` numeric ranges.** `fragment_count: { minimum: 1, maximum: 3 }` per spec. Document the 0.2–0.5 damage fraction + 30° cone in vendor-extension keys.

7. **LOW: drop the payload-level `cosmetic` duplicate** in `combat_projectile_hit_mo.json` (envelope-level `const: false` is sufficient). Or document why both are intentional.

8. **LOW: surface unit-ambiguity** of `impact_angle` (degrees in ricochet threshold context, radians in `cos()` formula context) in the schema descriptions of `armor_ricochet.json` and `armor_angle_deflection_calculated.json` so M13 producer authors don't conflate the two.

---

## Summary

- **Total events audited:** 20 (19 armor.* + 1 combat.projectile_hit_mo).
- **PASS:** 20 — every spec-listed event is shipped, registered, envelope-shaped at v0.1, with all spec fields present, typed correctly, required where applicable, and enums matching the locked tables.
- **GAP (per-event):** 0 events have payload drift; 0 events are missing.
- **GAP (cross-cutting locked taxonomy):**
  - 1 CRITICAL: `audio.event_requested` schema entirely missing despite spec mandate.
  - 1 HIGH: 7-material × 5-impact-state sound clip taxonomy not encoded.
  - 1 HIGH: 6-name internal-hit sound enum not encoded.
  - 1 MEDIUM: 9-armor-type locked taxonomy not encoded (deferred to M13 producer per spec, but the names are LOCKED at M5).
  - 1 MEDIUM: per-ammo ricochet threshold degrees prose-only, not machine-readable.
- **Critical missing pieces for M6 readiness:**
  - The `audio.event_requested` schema gap is the only spec-mandated missing artifact. Everything else is either spec-compliant or M13-producer-deferred.
  - M6 (Reactor Defense Scenario, sister milestone) does not consume `audio.event_requested` directly, but BP3 closure gate verifies M5 promise that "every damage event from M9 forward emits the structured event family" — and audio is part of the locked surface. Recommend closing this gap **before** moving M5.md from `active/` to `done/` (note: spec already shows `done/` location, so this is for an additive amendment commit, not a milestone reopen).

Net: the armor.* + combat.projectile_hit_mo core surface is **fully locked and conformant**. The locked taxonomies outside the 20 event schemas (sound clip variants, internal hit sounds, armor types, ricochet threshold degrees) are the residual gaps. The single CRITICAL item is the missing `audio.event_requested` schema.
