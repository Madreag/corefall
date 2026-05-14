# M5 Audit — M4 Envelope Conformance + M6 Readiness

**Audit date:** 5/13/2026
**Auditor:** worker subagent invoked from parent session
**Inputs:**

- M5 spec: `specs/done/M5.md`
- M4 envelope: `game/crates/cf-replay/schemas/v0_1/recorder_event.schema.json`
- M4 lib source: `game/crates/cf-replay/src/lib.rs` (`Event` struct + `Recorder::record*` family + `EVENT_SCHEMA_VERSION` const)
- M4 validator: `game/crates/cf-replay/src/schemas.rs` (`validate_event_payload`)
- cf-mod schema-file validator: `game/crates/cf-mod/src/main.rs` (`validate_event_schema_value`)
- Bundle checker: `game/tools/prototype_run_check.py` (`EVENT_VERSION`, `EVENT_ENVELOPE_ALLOWED`)
- M5 schemas: `game/crates/cf-replay/schemas/event/*.json` (74 deep-damage schemas at v0.1 + 23 pre-M5 schemas)
- M6 spec: `specs/active/M6.md`

---

## Part A: M4 envelope conformance

### A1. M4 envelope contract (required + optional fields)

Source: `game/crates/cf-replay/schemas/v0_1/recorder_event.schema.json` (LOCKED v0.1).

**Top-level `additionalProperties: false`** — envelope is closed. Unknown top-level fields are rejected by both the JSON Schema (in theory) and the bundle checker `prototype_run_check.py:312-322` (in practice, via `EVENT_ENVELOPE_ALLOWED`).

**Required envelope fields (8):**

| Field | Type | Source-of-truth string |
|---|---|---|
| `schema_version` | string | MUST equal `"prototype-recorder-event.v0.1"` |
| `run_id` | string | bundle dir basename |
| `tick` | integer ≥ 0 | monotonic non-decreasing |
| `sim_time_ms` | number ≥ 0 | sim-clock ms |
| `event_id` | string | `"<run_id>:<tick>:<seq>"` |
| `category` | string | one of the 38 categories |
| `event_type` | string | per category |
| `payload` | object | per-(category, event_type) shape |

**Optional envelope fields (9):**

| Field | Type | Purpose |
|---|---|---|
| `parent_event_id` | string \| null | cause-chain pointer |
| `actor_id` | integer \| string \| null | acting RecordId(u64) |
| `source_id` | integer \| string \| null | source RecordId (e.g. weapon) |
| `team` | string \| null | team affiliation tag |
| `pos` | array[2] \| null | 2D world pos `[x, y]` |
| `bbox` | object \| null | `{ min: [x,y], max: [x,y] }` |
| `dropped_count` | integer ≥ 0 \| null | recorder backpressure drops |
| `cosmetic` | boolean \| null | DR-052 cosmetic flag |
| `asset_ref` | string \| null | M4A asset-ledger AssetId |

**`schema_version` literal value (LOCKED):** `"prototype-recorder-event.v0.1"` (the canonical string declared in `cf-replay/src/lib.rs:EVENT_SCHEMA_VERSION` and enforced by `prototype_run_check.py:324-331`).

---

### A2. The `schema_version` discrepancy — M5 says `"0.1"`, M4 envelope says `"prototype-recorder-event.v0.1"`

**The collision (verbatim):**

- M4 envelope (`recorder_event.schema.json:14-17`):
  ```json
  "schema_version": {
    "type": "string",
    "description": "Envelope schema id. MUST be 'prototype-recorder-event.v0.1' for M4 bundles."
  }
  ```
- Each M5 per-event schema (e.g. `armor_layer_destroyed.json:9`):
  ```json
  "schema_version": { "const": "0.1" }
  ```
- Rust source-of-truth (`cf-replay/src/lib.rs:51`):
  ```rust
  pub const EVENT_SCHEMA_VERSION: &str = "prototype-recorder-event.v0.1";
  ```
- Recorder behaviour (`cf-replay/src/lib.rs::record_with_cosmetic`):
  ```rust
  let event = Event {
      schema_version: EVENT_SCHEMA_VERSION.to_string(),   // -> "prototype-recorder-event.v0.1"
      ...
  };
  ```

**Real events emitted by the recorder serialize `schema_version: "prototype-recorder-event.v0.1"`. The M5 per-event schemas constrain the field to the literal `"0.1"`. These are different strings. Under strict JSON Schema draft 2020-12 validation, every M5-schema-tagged event would fail.**

**Why the codebase hasn't caught this yet — three coordinated walk-arounds:**

1. **`prototype_run_check.py`** (bundle checker) validates `event.schema_version == "prototype-recorder-event.v0.1"` at the **envelope** layer. It does NOT use the per-event JSON schemas under `schemas/event/`.

2. **`cf-replay::schemas::validate_event_payload`** (`schemas.rs:382-401`) detects the M5 envelope-shape via the *presence* of `properties.schema_version.const`, then walks into `properties.payload` and only validates the payload sub-schema. It NEVER compares the `schema_version.const` value against the real envelope value. The `"0.1"` const is being used as a **schema-shape marker**, not a value constraint.

3. **`cf-mod::validate_event_schema_value`** (`cf-mod/src/main.rs:931-934`) explicitly enforces `properties.schema_version.const == "0.1"` on every M5 schema file. It is INVERTED relative to the envelope — it requires `"0.1"` and would reject `"prototype-recorder-event.v0.1"`.

In other words: the M5 implementer followed the M5.md skeleton (which says `"const": "0.1"`) literally; the validators were then written so the discrepancy never surfaces because each validator either (a) only checks the envelope contract (bundle checker) OR (b) treats the per-event `schema_version.const` as a marker (`validate_event_payload`) OR (c) enforces the per-event `"0.1"` literally without comparing it to the envelope (cf-mod).

**Resolution recommended (single-fix, minimum churn):**

Change the M5 per-event schemas' `properties.schema_version.const` from `"0.1"` to `"prototype-recorder-event.v0.1"` so they match the M4 envelope contract under strict JSON Schema semantics. This requires:

1. Bulk rewrite of all 74 M5 schemas: `"const": "0.1"` → `"const": "prototype-recorder-event.v0.1"`.
2. Update `cf-mod/src/main.rs:931-934`: change the check from `if sv != "0.1"` to `if sv != "prototype-recorder-event.v0.1"`.
3. Update `cf-replay::schemas::validate_event_payload` detection check: instead of "schema_version.const exists", switch the marker to "schema_version.const exists AND equals the envelope literal" so a typo can't silently bypass the M5-shape branch.
4. Update the M5 spec example (`specs/done/M5.md` § Implementer notes skeleton) to use the canonical envelope literal.
5. Update the M5 acceptance scenario "each schema declares schema_version=\"0.1\" matching the M4 locked envelope" — the literal `"0.1"` is wrong; it should be `"prototype-recorder-event.v0.1"`.
6. Update tests in `schemas.rs` (specifically `m5_schemas_declare_schema_version_v0_1` at lines ~755) to assert the canonical envelope literal.

**Affected schemas:** all 74 M5 schemas (every file in `cf-replay/schemas/event/` whose `properties.schema_version.const` is currently `"0.1"`):

- armor.* (19): layer_hp_changed, layer_critical, layer_destroyed, all_layers_destroyed, chunked_off, debris_spawned, repaired, angle_deflection_calculated, ricochet, spalling, penetration_ray_traversed, he_overpressure_wave, heat_jet_penetrated, heat_jet_pre_detonated_by_era, apfsds_penetrated, era_panel_detonated, schurzen_pre_detonated, multi_hit_degradation, reactive_armor_consumed
- internal.* (6): organ_damaged, organ_destroyed, organ_failure_cascade, circuit_damaged, circuit_destroyed, circuit_failure_cascade
- concussion.* (4): dose_changed, band_changed, ko_threshold_crossed, recovered
- internal_shock.* (2): dose_changed, module_damaged
- fluid.* (9): leak_started, leak_rate_changed, reservoir_warning, reservoir_critical, reservoir_empty, ignition, ground_splatter_spawned, leak_stopped, refilled
- origin.* (4): shot_force_feedback, g_load_dose_changed, helmet_breach, oxygen_supply_changed
- hazard.* (5): spawned, spread, actor_contact, tick, dissipated
- affliction.* (4): applied, tick, cleared, escalated
- atmos.* (10): pressure_changed, temperature_changed, gas_released, breach_detected, combustion_ignition, phase_transition, pipe_flow, pipe_freeze, pipe_rupture, electrolysis_started
- shield.* (5): hit, depleted, regen_started, regen_completed, disrupted
- environment.* (2): signal_delta, signal_aggregated
- thermal.* (3): signature_changed, heat_exchanged, material_phase_change
- combat.projectile_hit_mo (1)

Total: 19+6+4+2+9+4+5+4+10+5+2+3+1 = **74 schemas**, matching M5's "74 deep-damage event schemas at v0.1 envelope" claim.

**Producer impact when M13+ ships:** with the recommended fix, producers continue to set `schema_version = "prototype-recorder-event.v0.1"` (no change). Without the fix, producers would have to either lie (set `"0.1"`) OR the schema would have to keep the design quirk forever, in which case any external consumer who runs a strict JSON Schema validator against bundle events would see a 100% rejection rate.

---

### A3. Conflicts between M5 schemas and M4 envelope field types

Walked every required + optional envelope field, comparing the M4 envelope declaration to per-event M5 schema declarations:

| Envelope field | M4 envelope type | M5 schema declaration | Verdict |
|---|---|---|---|
| `schema_version` | string (literal `"prototype-recorder-event.v0.1"`) | `{ "const": "0.1" }` | **CONFLICT** — see § A2 |
| `category` | string | `{ "const": "<family>" }` | OK (narrowing const ⊂ string) |
| `event_type` | string | `{ "const": "<type>" }` | OK (narrowing const ⊂ string) |
| `actor_id` | `["integer","string","null"]` | `{ "type": ["integer","null"] }` | OK (subset; M5 producers only emit integers or null) |
| `tick` | integer ≥ 0 | `{ "type": "integer", "minimum": 0 }` | OK (matches) |
| `cosmetic` | `["boolean","null"]` | varies: `{ "type": ["boolean","null"] }` OR `{ "const": false }` (combat.projectile_hit_mo) | OK (`const false` is a narrowing of bool/null union) |
| `run_id`, `sim_time_ms`, `event_id`, `payload` | per envelope | not re-declared by M5 schemas | OK (envelope owns these) |
| `parent_event_id`, `source_id`, `team`, `pos`, `bbox`, `dropped_count`, `asset_ref` | per envelope | not re-declared at M5 envelope level | OK (envelope owns these; `additionalProperties: true` at M5 top-level admits them) |

**No type-level conflicts** other than the `schema_version` discrepancy in § A2.

---

### A4. Cosmetic flag coverage

Per M5.md, the cosmetic events are:

- `hazard.tick` — cosmetic; batched 10:1 ratio for determinism
- `fluid.ground_splatter_spawned` — cosmetic
- `affliction.tick` — consolidated cosmetic per-tick
- `environment.signal_aggregated` — cosmetic-batchable (heavy payload)

**M5 schemas that DECLARE `cosmetic` at envelope level:**

```
hazard_tick.json:14:                "cosmetic": { "type": ["boolean", "null"] }
fluid_ground_splatter_spawned.json:14:  "cosmetic": { "type": ["boolean", "null"] }
affliction_tick.json:14:                "cosmetic": { "type": ["boolean", "null"] }
environment_signal_aggregated.json:14:  "cosmetic": { "type": ["boolean", "null"] }
combat_projectile_hit_mo.json:14:       "cosmetic": { "const": false }
combat_projectile_hit_mo.json:54:       "cosmetic": { "const": false }   (also in payload — see § A6)
```

**Verdict: COMPLETE** — all 4 cosmetic events expose the envelope `cosmetic` field with `["boolean","null"]` type, allowing producers to set `cosmetic=true`. `combat.projectile_hit_mo` correctly pins `cosmetic: false` at envelope level (the deep-damage hit event is gameplay, never cosmetic).

**No drift** — every spec-declared cosmetic event has a schema declaration permitting the cosmetic field.

**Minor observation:** the remaining 69 M5 schemas (armor.*, internal.*, concussion.*, internal_shock.*, fluid.* except ground_splatter, origin.*, hazard.* except tick, affliction.* except tick, atmos.*, shield.*, environment.signal_delta, thermal.*) do NOT declare `cosmetic` at envelope level. Because their schemas have `additionalProperties: true` at envelope, the cosmetic field will be PERMITTED to pass through if a producer sets it. This is the "silent admission" behaviour the M4 contract implies. If the project wants to FORBID cosmetic on gameplay events, the M5 schemas should declare `"cosmetic": { "const": false }` at envelope level (mirroring combat.projectile_hit_mo). Currently nothing rejects a misbehaving producer that flags `armor.layer_destroyed` as cosmetic. Treat as nice-to-have (see § Summary).

---

### A5. Envelope optional fields visibility in M5 schemas

The M4 envelope optional fields (`parent_event_id`, `source_id`, `team`, `pos`, `bbox`, `dropped_count`, `cosmetic`, `asset_ref`) are NOT re-declared at envelope level in any M5 schema (other than the cosmetic field cases in § A4).

**Verdict: PASS** — every M5 schema declares `"additionalProperties": true` at envelope level (verified by grep across all 74 schemas; sample lines 7 in each file). This means producers can set any of the M4 envelope optional fields and the M5 schema will not reject the event for "extra properties at envelope level".

However, strict JSON Schema validation against the M4 envelope schema (`recorder_event.schema.json`) would STILL enforce `additionalProperties: false` at envelope level. So a producer adding e.g. `weather: "rain"` at envelope level would be rejected by `prototype_run_check.py` (via `EVENT_ENVELOPE_ALLOWED`) but accepted by the M5 schema. This is intentional: the M5 schemas describe the per-event payload contract, and the envelope contract is owned exclusively by `recorder_event.schema.json`.

---

### A6. The `combat.projectile_hit_mo` `parent_event_id` duplication

The M4 envelope owns the `parent_event_id` optional field at envelope level. The recorder (`cf-replay/src/lib.rs::record_with_cosmetic`) writes `parent_event_id` to the envelope, NOT to the payload.

**However**, `combat_projectile_hit_mo.json` declares `parent_event_id` at PAYLOAD level (line 53), AND lists it in the payload `required` array (line 56):

```json
"parent_event_id": { "type": "string" },
...
"required": ["shooter_id", ..., "parent_event_id"]
```

**Producer-time impact at M13/M14:** when the M13 chassis / M14 collision producer emits a `combat.projectile_hit_mo` event via the recorder API, the canonical call pattern is:

```rust
recorder.record(
    tick, sim_time_ms,
    "combat", "projectile_hit_mo",
    payload_json,
    Some(parent_hit_event_id),   // -> envelope.parent_event_id
);
```

The payload would NOT contain `parent_event_id` unless the producer explicitly duplicates it. Under the current M5 schema, this would FAIL the schema validation (the payload requires `parent_event_id` but it's missing).

**Concrete failure mode:** at M13 closure, the M13 producer would either (a) discover the schema requires `parent_event_id` in payload and start duplicating it (adding noise), or (b) emit non-conforming payloads and fail `cf-mod validate-bundle`.

**Comparison with origin.shot_force_feedback (the only other event with a parent reference):** `origin_shot_force_feedback.json` uses `parent_hit_event_id` — a DIFFERENT name from envelope `parent_event_id`, deliberately avoiding the collision. Same for `internal.organ_damaged` (uses `source_hit_event_id`).

**Resolution recommended:**

Pick one:
1. (Preferred) Rename `payload.parent_event_id` → `payload.parent_hit_event_id` in `combat_projectile_hit_mo.json`. This matches the pattern used elsewhere and reserves the envelope `parent_event_id` for the recorder API. Update the M5.md spec block accordingly.
2. (Alternative) Remove `parent_event_id` from `combat.projectile_hit_mo` payload required list entirely. Document that producers must set it at envelope level.

Option 1 is cleaner because it preserves the cause-chain pointer inside the payload (useful for offline analysis without re-reading the envelope) and is consistent with origin / internal naming.

---

### A7. Envelope-shape conformance verdict per-schema

Each of the 74 M5 schemas was checked for:
- `type: object` at top level — PASS (all)
- `additionalProperties: true` at top level — PASS (all 74)
- `properties.schema_version.const` present — PASS (all 74; value is `"0.1"`; see § A2 for fix)
- `properties.category.const` matches filename family prefix — PASS (verified in `cf-mod` validator test `m5_schemas_declare_schema_version_v0_1`)
- `properties.event_type.const` matches filename event type — PASS (same test)
- `properties.payload` is an object sub-schema with `type: object` — PASS
- top-level `required` includes `[schema_version, category, event_type, tick, payload]` — PASS (verified by grep — every schema lists all 5 in its required array)
- payload `additionalProperties: true` — PASS (all 74)

**Overall verdict: PASS modulo § A2 (schema_version literal) and § A6 (combat.projectile_hit_mo parent_event_id collision).**

These two findings are the only real blockers on the "M4 envelope at v0.1 acceptance" front, and both have well-defined fixes.

---

## Part B: M6 readiness

### B1. M6 scope summary (from `specs/active/M6.md`)

M6 = the first bridge slice between M3 and M9. It closes the actor-controller + equipment + inventory + sound + squad gap. Player-facing promise: **"a single actor in a single scenario already feels like a modern tactical shooter."**

Scope buckets:

- **Actor controller depth** (M1 9 → M6 36 actions): sprint, crouch, prone, slide, vault, climb, dive, lean, stealth kill, knife throw, drop/pickup/signal/mark, swap 1-8, plus expanded Stance state machine (Stand, Walk, Run, Sprint, Crouch, CrouchWalk, Prone, ProneWalk, Slide, Vault, Climb, Dive, Lean, KnockedDown, Downed, Dying, Dead, RopeClimb, LadderClimb, PipeClimb, StealthAttack, KnifeThrow, Swim reserved).
- **Equipment depth** (M1 1 weapon → M6 6 weapons + 4 grenades + 4 melee + 7 tools): rifle/SMG/shotgun/sniper/pistol/grenade launcher with per-weapon magazine + tracers + recoil + fire modes; suppressor + bipod attachments; tool degradation.
- **Inventory:** 8 active slots + 3 reserved tank slots (locked at M6; M17 + M19 fill); weight system; drop/pickup; weapon swap 300ms transition.
- **Centralized sound propagation + perception kernel** (NEW `cf-perception` crate): per-surface footstep loudness + distance attenuation + occlusion + echo + stealth meter + suppressor effect on alarm propagation.
- **Stealth kill / takedown:** sneak melee from behind when `stealth_meter < 30%`.
- **1 friendly bot + 4 squad commands** (FollowLeader / HoldPosition / DefendPoint / PushToWaypoint).
- **Side-view facing + limb-loss action restrictions** (M13 forward-compat): `Actor::facing: FacingDirection { Left | Right }`; limb loss blocks corresponding actions with structured rejection reasons.

M6 produces 30+ new event types per its dependencies block. M4's locked envelope must accept them additively.

---

### B2. M6 events that overlap M5 families

Enumerating every event mentioned in M6.md and mapping to whether M5 already locks the schema:

| M6 event | Category | In M5? | Notes |
|---|---|---|---|
| `equipment.tool_broken` | equipment | NO | M6-owned new event |
| `equipment.item_dropped` | equipment | NO (M5 has only legacy `equipment.weapon_fired` + `equipment.alarm_registered` + `equipment.tool_action_completed`) | M6-owned |
| `equipment.weapon_swap_started` | equipment | NO | M6-owned |
| `equipment.weapon_swap_completed` | equipment | NO | M6-owned |
| `equipment.grenade_thrown` | equipment | NO | M6-owned |
| `equipment.grenade_detonated` | equipment | NO | M6-owned (consider routing to existing `armor.he_overpressure_wave` for radius damage) |
| `equipment.melee_swing` | equipment | NO | M6-owned |
| `equipment.tool_repair_applied` | equipment | NO | M6-owned |
| `equipment.alarm_registered` | equipment | YES (legacy, pre-M5) | M6 producers can emit; suppressor multiplies `loudness × 0.4` |
| `combat.stealth_kill_executed` | combat | NO | M6-owned; orthogonal to `combat.projectile_hit_mo` |
| `combat.projectile_spawned` | combat | YES (legacy) | shotgun emits 8 per shot |
| `combat.wound_added` | combat | YES (legacy) | melee + projectile hits |
| `combat.projectile_hit_mo` | combat | YES (M5) | M6 producers can begin filling this for the deep-damage path (M13 will ladder up the full payload) |
| `actor.facing_changed { from, to, cause }` | actor | NO | M6-owned (side-view facing flip) |
| `actor.action_rejected { action, reason }` | actor | NO | M6-owned (limb-loss action restrictions) |
| `actor.inventory_dropped` | actor | YES (legacy) | already exists |
| `perception.footstep_emitted` | perception | NO (M5 didn't declare this category) | M6-owned; NEW category |
| `perception.occlusion_applied` | perception | NO | M6-owned; NEW category |
| `perception.stealth_meter_changed` | perception | NO | M6-owned; NEW category |
| `squad.member_added` | squad | NO (M5 didn't declare this category) | M6-owned; NEW category |
| `squad.command_issued` (implied) | squad | NO | M6-owned; NEW category |
| `inventory.tank_slot_reserved` | inventory | NO (M5 didn't declare this category) | M6-owned; NEW category |

**Three NEW categories M6 introduces that M5 doesn't touch:** `perception.*`, `squad.*`, `inventory.*`.

**Overlap with M5 damage families — none of M6's new events overlap a locked M5 schema.** M6 deliberately scopes itself to actor controller + equipment + inventory + sound + squad (none of which are M5's damage scope). When M6 producers emit deep-damage events (e.g. via `act.player.melee_bash` → 15 blunt dmg → triggers `combat.projectile_hit_mo`? OR a new `combat.melee_hit_mo`?), the M5 schemas apply.

**Implication:** M5's "no schema bump cascades when producers ladder up at M13/M14/M15/M16/M17/M19/M20" promise still holds. M6 is NOT in that ladder list; M6 ships its OWN event schemas (perception, squad, inventory, equipment new sub-events, actor.facing_changed, actor.action_rejected, combat.stealth_kill_executed). M6 will need to:

1. Add per-event JSON schemas under `cf-replay/schemas/event/` for ~22 new event types.
2. Register them in `cf-replay/src/schemas.rs::event_schema_for`.
3. Run `cf-mod validate cf-replay/schemas/` to verify conformance.

These are pure additive moves on top of the M4 envelope — no envelope bump required.

---

### B3. M5 enums M6 will consume

| Enum | Locked at M5 | Used by M6 | Verdict |
|---|---|---|---|
| **BodyZone (15 values)** — head, torso, arm_left, arm_right, forearm_left, forearm_right, hand_left, hand_right, leg_left, leg_right, shin_left, shin_right, foot_left, foot_right, backpack | YES (`armor_layer_destroyed.json:18`, `combat_projectile_hit_mo.json::hit_zone`, all armor + internal schemas) | M6 hit-zone selection (limb-loss table at M6.md:514-525 enumerates "both arms / single arm / both legs / single leg / both hands / backpack lost / head destroyed / torso destroyed") | **MATCHES** — M6 limb-loss vocabulary maps onto M5's BodyZone enum cleanly. "head" + "torso" + arm_left/right + hand_left/right + leg_left/right + backpack are all in M5's 15-zone list. The "both arms" / "single arm" predicate is a M6 derivation over the 15-zone state. |
| **DamageKind (5 values)** — kinetic, thermal, electric, chemical, radiation | YES (`combat_projectile_hit_mo.json::damage_kind`) | M6 melee (blunt, piercing) → maps to `kinetic`; M6 flash grenade → `thermal`? Or new `concussive`? | **MOSTLY MATCHES**. "blunt" / "piercing" both fold into `kinetic`. M6 doesn't introduce a new DamageKind; it leans on M5's enum. |
| **ArmorLayer (3 values)** — External, Internal, Core | YES (multiple armor schemas) | M6 doesn't directly emit armor.* events; M13 will when chassis ladders up | **MATCHES** — no M6 conflict |
| **AmmoRoundTier (8 values)** — standard, armor_piercing, hardened_AP, discarding_sabot, explosive_warhead, kinetic_impact, HEAT, APFSDS | YES (`combat_projectile_hit_mo.json::ap_round_tier`) | M6 ammo types: regular + tracer; shotgun pellets; grenade launcher explosive rounds. The 6 weapons all map to `standard` or `explosive_warhead` at M6; M13 fills the full ladder | **MATCHES** — M6 doesn't add new round tiers |
| **SurfaceKind (8 values)** — armor_external, armor_internal, armor_core, armor_chunked_breach, flesh, circuit, unarmored, terrain | YES (`combat_projectile_hit_mo.json::surface_kind`) | M6 uses `unarmored` for unarmored hits at this stage; M13 ladders up full chassis | **MATCHES** |
| **Stance enum (~20 values)** — Stand, Walk, Run, Sprint, Crouch, CrouchWalk, Prone, ProneWalk, Slide, Vault, Climb, Dive, Lean, KnockedDown, Downed, Dying, Dead, RopeClimb, LadderClimb, PipeClimb, StealthAttack, KnifeThrow, Swim, plus M6's limb-loss additions Crawl + KneelStance | **NOT in M5** | M6 ships this enum from scratch | **NO CONFLICT** — Stance is actor-controller scope, M5 stayed clear of it. M6 will own the Stance enum + the associated `actor.stance_changed` event schema (if M6 chooses to emit one). |
| **22 affliction kinds** — burning, wet, electrified, poisoned, hypoxic, combustible_atmosphere, breach_decomp, hyperthermic, hypothermic, radiation, concussed, deafened, bleeding, internal_shock, low_battery, coolant_leaking, oil_leaking, overheating, hunger, thirst, sleep_dep, sanity_low | YES (`affliction_applied.json::kind`) | M6 flash grenade emits "deafen + **blind** afflictions"; knife stab emits "bleed chance" → `bleeding`; rifle bash → `concussed` is reachable | **GAP: `blinded` is missing from M5's 22-kind enum.** M6 flash grenade explicitly requires it. Either M6 adds `blinded` to the affliction enum (additive enum extension; allowed per M5 contract) or M6 emits a different non-affliction event for "blind" (e.g. `actor.vision_disrupted`). |
| **Fluid kinds (4 values)** — oil, coolant, fuel, electrolyte | YES (`fluid_leak_started.json::fluid_kind`) | Not used at M6 (chassis fluid system is M13 scope) | **MATCHES** |
| **Hazard kinds (8 values)** — fire, smoke, electric, wet, hot, cold, acid, radiation, toxic | YES (`hazard_spawned.json::kind`) | M6 smoke grenade spawns `smoke` hazard tile; flash grenade may spawn `electric`? (spec ambiguous) | **MATCHES** — M6 grenades fold into M5 hazard kinds |
| **5 affliction-band thresholds (concussion)** — Clear / Mild / Moderate / Severe / KO_Imminent / KO | YES (`concussion_band_changed.json::from_band`) | M6 doesn't ship the concussion accumulator (M17 owns it). But rifle-bash + shoulder-check producing knockdown could feed into concussion accumulator if M16/M17 producers are wired. | **MATCHES** for forward-compat |

---

### B4. M5 gaps that would force M6 to add new schemas

These are M5 implicit or explicit promises that lack a concrete schema in `cf-replay/schemas/event/`. M6 producers will surface these gaps when they ladder up.

| Gap | M5 spec reference | M6 surface | Severity |
|---|---|---|---|
| **`audio.event_requested` schema** | M5.md § "Sound clip variants per armor material + impact state": "All sound events emit `audio.event_requested` with `kind: material_state` + `surface_kind` + `damage_kind`. ... M5 just locks the request shape." | M6 cf-perception emits footstep + alarm-propagation events; M13.x cf-audio consumes. If M5 promised to lock the shape, the schema should exist before M6 ships. | **HIGH** — direct M5 spec promise, schema missing |
| **`blinded` affliction kind** | M5.md § "22 affliction kinds (locked names)" enumerates 22 — `deafened` is in, `blinded` is NOT | M6 flash grenade "1.5s fuse + deafen + blind afflictions" → needs `blinded` | **MEDIUM** — M6 spec explicitly mentions blind; M5 enum is closed without it |
| **`combat.melee_hit_mo` (or equivalent)** | M5 ships `combat.projectile_hit_mo` for deep-damage projectile hits; nothing equivalent for melee | M6 has 4 melee weapons + kick + shoulder-check + stealth_kill; produces hit events that should route through the same deep-damage event family | **MEDIUM** — M5 spec doesn't promise this, but design symmetry suggests M5 should have shipped a unified `combat.hit_mo` or split projectile/melee/explosive variants. M6 can ship `combat.melee_hit` itself but it would be cleaner to fold into the M5 family at M6's invocation. |
| **`combat.explosive_hit_mo` (or equivalent)** | Same as above — grenade detonation hits should route through deep-damage envelope | M6 frag/flash/smoke/stick grenades produce radius damage that should route through M5 armor + internal schemas | **MEDIUM** — `armor.he_overpressure_wave` is in M5 but covers only the area-effect wave, not the per-actor hit event |
| **`actor.stance_changed` schema** | M5 didn't claim to lock actor.* | M6 introduces 17-stance state machine + transitions on knockdown / dying / dead / limb-loss-induced crawl + kneel | **LOW** — strictly M6 scope (M5 stayed clear of actor.*); M6 ships the schema |
| **`actor.facing_changed` schema** | M5 didn't claim | M6 spec lists it as a replay event | **LOW** — M6 scope |
| **`actor.action_rejected` schema** | M5 didn't claim | M6 spec lists it for limb-loss + NaN/Inf guards | **LOW** — M6 scope |
| **Cosmetic flag enforcement on gameplay events** | M5 only declares `cosmetic` field on the 4 cosmetic events + combat.projectile_hit_mo (const false). Other gameplay events allow cosmetic via `additionalProperties: true`. | M6 producers could mis-tag a gameplay event as cosmetic and the M5 schema wouldn't reject it (the bundle checker would still allow it because envelope allows the field). | **LOW** — defensive hardening, not blocking |

---

### B5. Recommended M5 enhancements before (or at) M6 close

These low-effort additions to the M5 schema set prevent M6-driven schema bumps and align the event surface with M6 expectations. None of them require envelope changes — all are additive at the per-event schema level.

1. **Add `audio.event_requested` schema** in `cf-replay/schemas/event/audio_event_requested.json` with the locked shape (`kind: material_state`, `surface_kind: enum [metal, ceramic, composite, cloth, leather, hardened_plate, reactive_armor, flesh, bone, organ, circuit, fluid]`, `damage_kind` enum from M5). Register in `cf-replay/src/schemas.rs::event_schema_for` under `("audio", "event_requested")`. This is the explicit M5 promise per spec.

2. **Extend `affliction.applied::kind` enum to include `blinded`** in `affliction_applied.json`, `affliction_tick.json`, `affliction_cleared.json`, `affliction_escalated.json`. Update the description string to "23 affliction kinds" and add `blinded` to the canonical list. Update M5.md spec text to match.

3. **Rename `combat.projectile_hit_mo` payload `parent_event_id` → `parent_hit_event_id`** in `combat_projectile_hit_mo.json` to match the `origin.shot_force_feedback::parent_hit_event_id` + `internal.organ_damaged::source_hit_event_id` convention and avoid the envelope-field name collision.

4. **Fix `schema_version` const** — change all 74 M5 schemas from `"const": "0.1"` to `"const": "prototype-recorder-event.v0.1"`. Update the cf-mod validator + cf-replay validator + M5 spec example accordingly. See § A2 for the full step list.

5. **(Optional, defensive)** Declare `"cosmetic": { "const": false }` at envelope level on all 69 non-cosmetic M5 schemas. This forbids producers from mis-flagging gameplay events as cosmetic. Currently the field passes through silently via `additionalProperties: true`.

6. **(Optional, low-effort)** Add `combat.melee_hit_mo` schema under `cf-replay/schemas/event/combat_melee_hit_mo.json` with the same payload shape as `combat.projectile_hit_mo` but with melee-specific fields (`melee_weapon_id`, `swing_arc`, drop `projectile_id` + `ap_round_tier`). Producers at M6 emit it for melee hits, M13 ladders up the full deep-damage routing. Symmetric with projectile.

7. **(Optional, low-effort)** Add `combat.explosive_hit_mo` schema for radius damage hits (frag grenade per-actor hit events; grenade-launcher impact). Symmetric with projectile/melee.

---

## Summary

### Verdicts

- **M4 envelope conformance**: **FIX-REQUIRED** — two concrete issues (§ A2 schema_version literal mismatch on 74 schemas; § A6 `combat.projectile_hit_mo.payload.parent_event_id` collision). Neither blocks M5's "no schema bump cascades" promise so long as the schemas continue to be validated only by the cf-mod + cf-replay validators (which coordinate around the discrepancy). Strict third-party JSON Schema validation against bundle events would fail today.
- **M6 readiness**: **READY** — M5 does NOT block M6 closure. M6's new categories (perception, squad, inventory) and new events (actor.facing_changed, actor.action_rejected, equipment.*, combat.stealth_kill_executed) are orthogonal to M5's damage scope. M5's BodyZone + DamageKind + ArmorLayer + ammo + surface enums all match M6's usage. The `audio.event_requested` schema gap (M5 promise) and `blinded` affliction gap (M6 surface) should be patched, but they are not blockers — M6 can ship the schemas itself if M5 isn't reopened.

### Top 3 must-fix items

1. **Fix `schema_version` literal mismatch (§ A2).** Change 74 schemas' `"const": "0.1"` → `"const": "prototype-recorder-event.v0.1"`; update cf-mod and cf-replay validators + M5 spec skeleton + tests. Single-PR bulk rewrite.
2. **Resolve `combat.projectile_hit_mo.parent_event_id` collision (§ A6).** Rename payload field to `parent_hit_event_id` (consistent with origin.* + internal.* naming); update M5.md spec block.
3. **Ship `audio.event_requested` schema (§ B4 row 1).** M5 spec explicitly promised "M5 just locks the request shape" but the schema is missing from `cf-replay/schemas/event/`.

### Top 3 nice-to-have enhancements

1. **Add `blinded` to affliction enum** so M6 flash grenade can fold into the M5 affliction family without ad-hoc handling.
2. **Add `combat.melee_hit_mo` + `combat.explosive_hit_mo` schemas** symmetric with `combat.projectile_hit_mo` so M6 melee + grenade producers route through the same deep-damage envelope.
3. **Defensive `cosmetic: const false`** on the 69 non-cosmetic M5 schemas so producers can't mis-flag gameplay events as cosmetic.

---

## Cross-references

- M5 spec (closed): `specs/done/M5.md`
- M4 envelope: `game/crates/cf-replay/schemas/v0_1/recorder_event.schema.json`
- M4 lib: `game/crates/cf-replay/src/lib.rs:51` (`EVENT_SCHEMA_VERSION`)
- M4 validator: `game/crates/cf-replay/src/schemas.rs:382-408` (envelope-shape detection)
- cf-mod schema-file validator: `game/crates/cf-mod/src/main.rs:931-934` (`if sv != "0.1"`)
- Bundle checker: `game/tools/prototype_run_check.py:31` + `:324-331`
- M6 spec: `specs/active/M6.md`
- M6 new event taxonomy: `specs/active/M6.md:77, 89, 109, 237, 302-303, 314, 320, 343, 378, 398, 411-413, 419, 436, 446, 452, 466, 503, 522`
