# M5 Pass-2 Audit — M6 Readiness Deep Dive

**Audit date:** 5/13/2026 (post-`1784ad2`)
**Auditor:** worker subagent invoked from parent session
**Pass:** 2 of N — second-pass deep dive on M6 readiness after M5-A1 hardening
**Goal:** verify pass-1 deliverables landed, enumerate every M6-implied event,
build an exhaustive M6 ↔ M5 coverage matrix, and surface gaps that would force
M6 to bump schemas mid-implementation.

**Inputs:**

- M5 spec (closed): `specs/done/M5.md`
- M6 spec (active): `specs/active/M6.md` (565 lines, read end-to-end)
- Pass-1 audit: `audit-m5/06-envelope-m6-readiness-audit.md`
- M5-A1 commit: `1784ad2` (full diff inspected)
- M4 envelope: `game/crates/cf-replay/schemas/v0_1/recorder_event.schema.json`
- All event schemas: `game/crates/cf-replay/schemas/event/*.json` (130 files)
- Validator: `game/crates/cf-replay/src/schemas.rs` (1166 lines)
- cf-mod validator: `game/crates/cf-mod/src/main.rs`
- cf-actor Stance enum: `game/crates/cf-actor/src/lib.rs:127-145`

---

## Pass-1 deliveries verified

| Item | Verified? | Evidence |
|---|---|---|
| `audio.event_requested` schema present | **YES** | `cf-replay/schemas/event/audio_event_requested.json` (2562 bytes). Locks `kind ∈ {material_state, internal_hit}`, 7-material + 5-impact-state taxonomy, 6-name internal-hit enum, `surface_kind` + `damage_kind` mirroring `combat.projectile_hit_mo`. Registered in validator at `schemas.rs:189-190` (const) + `:345-346` (lookup table) + `:691` + `:1140` (tests). |
| `audio.event_requested` round-trip test | **YES** | `m5_per_family_happy_path` (schemas.rs:992) validates a sample audio payload. |
| `blinded` affliction added (all 4 schemas) | **YES** | `affliction_applied.json:19`, `affliction_cleared.json:19`, `affliction_escalated.json:19`, `affliction_tick.json:20` all list `"blinded"` in their `kind` enum. `affliction_applied.json:5` description updated to "23 affliction kinds (locked names; M16 fills mechanics; M5-A1 adds blinded for M6 flash grenade)". Test `m5_per_family_happy_path` validates with `kind: "blinded"`. |
| `schema_version` canonical literal bulk rewrite | **YES** | `grep -l '"const": "0\.1"' cf-replay/schemas/event/*.json` returns zero matches. All 74 M5 schemas now declare `"const": "prototype-recorder-event.v0.1"`. Test `m5_schemas_declare_schema_version_v0_1` (schemas.rs:1078) asserts canonical literal on all 75 registered M5 schemas (74 from pass-1 + 1 new audio.event_requested). cf-mod validator's `validate_event_schema_value` updated to require canonical literal (commit log L34-37). |
| `combat.projectile_hit_mo` `parent_event_id` → `parent_hit_event_id` rename | **YES** | `combat_projectile_hit_mo.json:51` declares `parent_hit_event_id` in payload (line 53 lists it in required). Test `m5_combat_projectile_hit_mo_rejects_envelope_named_parent` (schemas.rs:1037) explicitly rejects the old payload-level `parent_event_id`. Commit log L23-26 records the symmetry rationale (matches `origin.shot_force_feedback.parent_hit_event_id` + `internal.organ_damaged.source_hit_event_id`). |
| `combat.projectile_hit_mo` duplicate `cosmetic` const removed from payload | **YES** | `combat_projectile_hit_mo.json:14` carries `"cosmetic": { "const": false }` at envelope level; the payload-level duplicate from the pre-pass-1 schema is gone. |

**Bonus deliveries (also in 1784ad2, beyond pass-1's explicit 4 items):**

| Bonus item | Notes |
|---|---|
| Origin enum locked via `oneOf` | `concussion.dose_changed.origin_id` + `origin.shot_force_feedback.origin_id` now accept either an integer OR one of `["Human","Android","Robot","PoweredOrganic","HeavyBiomech"]` strings (test `m5_concussion_dose_changed_rejects_bad_origin` rejects `"Construct"`). |
| `snapshot_origin.json` description realigned to canonical 5-value Origin enum | Pre-pass-1 had `HeavyBioMech` + `Construct` drift. |
| Concussion dose `maximum: 100` | spec locks 0..100. |
| Phase enums locked on `atmos.phase_transition` + `thermal.material_phase_change` | `gas/liquid/solid` + `supercritical` for atmos; `+molten` for thermal. |
| `environment.signal_aggregated.signal` sub-struct locks `EnvironmentSignal` shape | Including the 15-value `HazardClass` enum from cf-environment. |
| 4 cosmetic events tightened to `cosmetic: const true` | `hazard.tick`, `fluid.ground_splatter_spawned`, `affliction.tick`, `environment.signal_aggregated`. Producers cannot mis-emit gameplay as cosmetic. |
| `fluid.ignition.fluid_kind` tightened to `["oil","fuel"]` | Combustible-only per spec. |
| `origin.shot_force_feedback.chassis_layer` tightened to surface_kind enum | Previously open string. |
| `origin.oxygen_supply_changed.source` tightened to enum `[helmet_breach, refilled, exhaled, atmosphere]` | Previously open string. |
| New schema `snapshot.snapshot_shield` | M9 firehose mirror of `ShieldState { hp, max_hp, regen_rate_per_s, downtime_after_break_s, status: Up\|Down\|Regenerating\|Disrupted }`. |
| cf-mod validator hardening | Rejects `payload.additionalProperties: false`; envelope-dir regex widened from `v0_1`/`v1` literals to `^v[0-9]+(_[0-9]+)?$`. |
| cf-replay validator hardening | Added `oneOf` + `maximum` constraint support. |
| `hazard_spawned.json` description reconciled | Spec-bullet `hot_cold` vs event-definition `hot` + `cold` split. |

**Verdict: all 4 explicit pass-1 deliverables PASS. 13 bonus enhancements landed.**

---

## M6 event coverage matrix

The matrix below enumerates every event the M6 spec implies (gleaned by reading
all 565 lines of `specs/active/M6.md`, the Player-facing behavior section, the
Acceptance criteria Gherkin blocks, the Out-of-scope routing block, and the
Side-view facing + limb-loss subsection). For each event the matrix records:

- Category prefix.
- Whether M5 (or earlier milestones) already locked a schema.
- Schema filename if shipped.
- Status verdict: **PASS (in M5/legacy)** / **M6 must ship** / **M6 may want** /
  **NEEDS DECISION**.

### actor.* family

| M6 event | In M5? | Schema | Status | Notes |
|---|---|---|---|---|
| `actor.facing_changed { from, to, cause }` | NO | not shipped | M6 must ship | M6.md:502-503. Side-view facing flip. FacingDirection enum `{Left, Right}` is M6-owned. |
| `actor.action_rejected { action, reason }` | NO | not shipped | M6 must ship | M6.md:518-522. Limb-loss + NaN/Inf + stance restrictions. Reason enum locked in M6.md:514-525 (see NEW-E). |
| `actor.stance_changed { from, to, cause }` | NO | not shipped | M6 must ship | Implied by M6.md:31-32 ("Stance state machine") + M6.md:530-535 ("Stance forced transitions on limb loss"). 23-state enum (see NEW-K). |
| `actor.inventory_dropped` | YES (legacy) | `inventory_dropped.json` (M1 audit pass 6) | PASS | M1 ships this. M6 producers re-use. |
| `actor.actor_status_changed { actor_kind, new_status, cause }` | YES (legacy) | unregistered, but emitted by M0Engine for reactors at BP2 | PASS | M6 producers re-use for limb-loss "dead" transitions when head/torso destroyed (M6.md:524-525). |

### equipment.* family

| M6 event | In M5? | Schema | Status | Notes |
|---|---|---|---|---|
| `equipment.weapon_fired` | YES (legacy) | `weapon_fired.json` (M1) | PASS | Carries `loudness_radius` + `bloom_factor`. M6 SMG/shotgun/sniper/pistol/grenade-launcher all re-use; suppressor effect on `loudness_radius` lives in producer (multiplier 0.4 per M6.md:198). |
| `equipment.alarm_registered` | YES (legacy) | `alarm_registered.json` (M1) | PASS | M6.md:198 spec text confirms re-use: "equipment.alarm_registered.loudness × 0.4" under suppressor. |
| `equipment.tool_action_started` | YES (legacy) | `tool_action_started.json` (M3 pass 7) | PASS | M6 7-tool fleet (digger + repair + foam + concrete + welder + drill + multi-tool + beacon + sensor_pulse) re-uses. |
| `equipment.tool_action_completed` | YES (legacy) | `tool_action_completed.json` (M3 pass 7) | PASS | Same as above. |
| `equipment.tool_refused` | YES (legacy) | `tool_refused.json` (M3 pass 7) | PASS | re-use for cooling-jam, durability-zero, etc. |
| `equipment.weapon_swap_started { from_slot, to_slot, transition_ms }` | NO | not shipped | M6 must ship | M6.md:235-237 + M6.md:419. 300ms standard; 200ms for pistol. |
| `equipment.weapon_swap_completed { slot, weapon_id }` | NO | not shipped | M6 must ship | M6.md:419 explicitly. (See NEW-R below.) |
| `equipment.item_dropped { item_id, hand_position, toss_velocity }` | NO | not shipped | M6 must ship | M6.md:411-413. Different shape from `actor.inventory_dropped` (which is M1 legacy "shoot off backpack"); equipment.item_dropped is "Q-key drop" action. NEEDS DECISION — see NEW-W below. |
| `equipment.item_picked_up { actor_id, item_id, slot_index }` | NO | not shipped | M6 must ship | M6.md:407-410. E-key pickup. |
| `equipment.fire_mode_changed { weapon_id, from_mode, to_mode }` | NO | not shipped | M6 must ship | M6.md:130 + acceptance scenarios at M6.md:222-225 (SMG burst-3 / Shotgun pellets / Sniper charge). |
| `equipment.grenade_thrown { grenade_kind, throw_velocity, fuse_remaining_ms, cooked }` | NO | not shipped | M6 must ship | M6.md:302-303 acceptance scenario. |
| `equipment.grenade_detonated { grenade_id, position, radius, kind }` | NO | not shipped | M6 must ship | M6.md:303-304. (See NEW-A — does this overlap `armor.he_overpressure_wave`?) |
| `equipment.grenade_cooked { grenade_id, cook_duration_ms }` | NO | not shipped | M6 must ship | M6.md:306-310 acceptance scenario; lethal-in-hand path needs an event. |
| `equipment.shell_ejected { weapon_id, position, velocity }` | NO | not shipped | M6 may want (cosmetic) | M6.md:131 — "Shell casing ejection as cosmetic MovableObject per CCCP Round::Shell". Cosmetic = could be `cosmetic: true` batched event. |
| `equipment.tracer_round_spawned { weapon_id, round_idx, is_tracer }` | NO | not shipped | M6 may want | M6.md:230-234 says "tracer round deterministic pattern... replay reproduces the exact pattern". If determinism replay-verifies the pattern, an event must fire. Could fold into combat.projectile_spawned with `is_tracer: bool` field — see NEW-X. |
| `equipment.melee_swing { actor_id, melee_kind, swing_arc, range }` | NO | not shipped | M6 must ship | M6.md:314, 320, 343. 4 melee weapons + kick + shoulder check. |
| `equipment.bipod_deployed { actor_id, weapon_id, stance }` | NO | not shipped | M6 must ship | M6.md:140 + acceptance M6.md:268-273. Recoil × 0.3, bloom × 0.5. |
| `equipment.bipod_stowed { actor_id, weapon_id, cause }` | NO | not shipped | M6 must ship | Auto-stow on stand (M6.md:271). |
| `equipment.suppressor_attached { weapon_id }` | NO | not shipped | M6 may want | M6.md:140 — suppressor reduces loudness by 60%. Effect lives in producer; attach/detach event optional. |
| `equipment.tool_broken { tool_id, kind }` | NO | not shipped | M6 must ship | M6.md:378 explicit. |
| `equipment.tool_repair_applied { tool_id, target, restored_pct }` | NO | not shipped | M6 must ship | M6.md:362 acceptance scenario. |
| `equipment.tool_heat_jammed { tool_id, heat }` | NO | not shipped | M6 may want | M6.md:369 — "When heat > threshold: drill jams". Could fold into tool_refused. |
| `equipment.knife_throw { actor_id, knife_id, velocity }` | NO | not shipped | M6 must ship | M6.md:323-329 acceptance scenario; 50% damage projectile + retrievable. |
| `equipment.knife_retrieved { actor_id, knife_id }` | NO | not shipped | M6 must ship | M6.md:328-329 — "When player approaches + presses E: knife returned to inventory". |
| `equipment.weight_overload_changed { actor_id, total_kg, carry_limit_kg }` | NO | not shipped | M6 must ship | M6.md:401-405 acceptance scenario — forces Walk + rejects sprint. |

### combat.* family

| M6 event | In M5? | Schema | Status | Notes |
|---|---|---|---|---|
| `combat.projectile_spawned` | YES (legacy) | `projectile_spawned.json` (M1) | PASS | Shotgun emits 8 per shot (M6.md:227-229). |
| `combat.wound_added` | YES (legacy) | `wound_added.json` (M1) | PASS | M6 melee + projectile hits feed this. |
| `combat.projectile_hit_mo` | YES (M5) | `combat_projectile_hit_mo.json` | PASS | M6 projectile hits use this. |
| `combat.projectile_expired` | YES (legacy) | unregistered envelope-only | PASS | BP2 emitted `combat.projectile_expired { cause }`. |
| `combat.melee_hit_mo { attacker_id, melee_kind, target_id, hit_zone, impact_impulse, damage_kind, hp_before, hp_after }` | NO | not shipped | NEEDS DECISION (M5-A2 or M6) | Symmetric with projectile_hit_mo. Pass-1 flagged as MEDIUM nice-to-have; pass-2 elevates to **strongly recommended at M5-A2 or as M6's first commit** — see NEW-A below. |
| `combat.explosive_hit_mo { source_event_id, source_kind, target_id, hit_zone, impact_impulse, radius_m, damage_kind, hp_before, hp_after }` | NO | not shipped | NEEDS DECISION | Symmetric for grenade detonation per-actor hits. `armor.he_overpressure_wave` (M5) covers the area-effect wave but NOT the per-actor hit event. See NEW-A. |
| `combat.stealth_kill_executed { attacker_id, target_id, melee_kind, behind_threshold_deg }` | NO | not shipped | M6 must ship | M6.md:268-279. Only when `stealth_meter < 30%`; M6.md:281-287 acceptance. Could fold into combat.melee_hit_mo with `stealth: true` flag (see NEW-A). |
| `combat.shoulder_check { attacker_id, target_id, impulse, knockdown_chance }` | NO | not shipped | M6 must ship | M6.md:320 + acceptance M6.md:321 — "Shoulder check during sprint". Could fold into combat.melee_hit_mo with `melee_kind: shoulder_check`. |
| `combat.kick { attacker_id, target_id, impulse, knockdown_chance }` | NO | not shipped | M6 must ship | M6.md:316. Could fold into combat.melee_hit_mo with `melee_kind: kick`. |
| `combat.charge_fire_progress { weapon_id, charge_0_1 }` | NO | not shipped | M6 may want | M6.md:240-247 — sniper charge mode 0..100% with misfire/max-damage outcomes. Cosmetic; producer-internal HUD bar drives off observe state. |

### perception.* family (NEW CATEGORY at M6)

| M6 event | In M5? | Schema | Status | Notes |
|---|---|---|---|---|
| `perception.footstep_emitted { actor_id, position, surface_loudness, stance }` | NO | not shipped | M6 must ship | M6.md:208 + M6.md:434-438 acceptance. Per-surface modifier from `MaterialDef.loudness_modifier`. |
| `perception.occlusion_applied { source_event_id, occlusion_factor, intervening_material }` | NO | not shipped | M6 must ship | M6.md:209 + M6.md:441-444 acceptance. |
| `perception.stealth_meter_changed { actor_id, from, to, cause }` | NO | not shipped | M6 must ship | M6.md:210 + M6.md:446-451 acceptance. Range 0..1 (see NEW-G). |
| `perception.alarm_propagated { from_actor, to_actor, source_event_id, loudness }` | NO | not shipped | M6 may want | Implied by M6.md:212 + M6.md:453 ("Suppressor reduces alarm"). Could fold into equipment.alarm_registered cause-chain via `parent_event_id`. |
| `perception.sight_target_acquired { observer_id, target_id, range, line_of_sight }` | NO | not shipped | M6 may want | Implied by M6.md:204-211 perception kernel. |
| `perception.hearing_signal { actor_id, source_event_id, effective_loudness }` | NO | not shipped | M6 may want | Implied by M6.md:435 — "nearby enemies within range react". |

### squad.* family (NEW CATEGORY at M6)

| M6 event | In M5? | Schema | Status | Notes |
|---|---|---|---|---|
| `squad.member_added { squad_id, actor_id, role }` | NO | not shipped | M6 must ship | M6.md:303 + acceptance M6.md:458-461. |
| `squad.command_issued { squad_id, command_kind, target_actor, target_pos }` | NO | not shipped | M6 must ship | M6.md:286-298 — 4 commands: FollowLeader / HoldPosition / DefendPoint(pos) / PushToWaypoint(pos). |
| `squad.member_died { squad_id, actor_id }` | NO | not shipped | M6 may want | Implied by HUD squad strip at M6.md:299. |

### inventory.* family (NEW CATEGORY at M6)

| M6 event | In M5? | Schema | Status | Notes |
|---|---|---|---|---|
| `inventory.tank_slot_reserved { actor_id, slot_kind }` | NO | not shipped | M6 must ship | M6.md:170-175 + acceptance M6.md:425-430. 3 slot kinds: tank_primary, tank_secondary, tank_utility. |
| `inventory.slot_reject { actor_id, slot_kind, reason }` | NO | not shipped | M6 must ship | M6.md:432-433 — "tank_slot_locked_at_m2_2a" rejection event. |
| `inventory.hotbar_swap { actor_id, from_slot, to_slot }` | NO | not shipped | M6 may want | M6.md:393-398 — 1-8 key cycling. Could fold into equipment.weapon_swap_started. |
| `inventory.backpack_lost { actor_id }` | NO | not shipped | M6 may want | Implied by M6.md:520 — "Backpack lost → Jet disabled; battery disabled; inventory storage = 0". |

### audio.* family

| M6 event | In M5? | Schema | Status | Notes |
|---|---|---|---|---|
| `audio.event_requested { kind, material, impact_state, ... }` | YES (M5-A1) | `audio_event_requested.json` | PASS | Pass-1 added. M6 uses for armor-hit sounds AND footstep sounds (NEW-I — needs additional `kind` enum members; see below). |

---

## New issues found (pass-2)

The numbering follows the original task brief's NEW-A through NEW-T plus
3 additional issues surfaced during the deep-dive (NEW-U, NEW-V, NEW-W).

### NEW-A: M6 melee + grenade hit events have no M5 schema (CRITICAL pre-M6)

**Symptom:** M5 ships `combat.projectile_hit_mo` (the deep-damage hit event for
projectile impact). M6 introduces 4 melee weapons + kick + shoulder check +
4 grenade types — every one of which should route through the same deep-damage
envelope (so armor.* + internal.* + concussion.* + affliction.* cause-chain
correctly off the hit). There is no M5 `combat.melee_hit_mo` and no
`combat.explosive_hit_mo`.

**Pass-1 verdict:** flagged as "MEDIUM nice-to-have" but not acted on.

**Pass-2 elevation to STRONGLY-RECOMMENDED for M5-A2:**

1. The M5 spec promise is "no schema bump cascades when producers ladder up at
   M13/M14/M15/M16/M17/M19/M20". M6 is the first milestone with non-projectile
   hits that the M5 promise should arguably cover by symmetry.
2. Without M5-locked schemas, M6 has to design these schemas itself. If M6
   ships a `combat.melee_hit { attacker_id, target_id, damage }` micro-schema
   (e.g. without `hit_zone`, `impact_normal`, `surface_kind`), then M13 chassis
   damage will need to bump the schema OR live with `additionalProperties: true`
   permissive extension. The latter is allowed under M4 DR-002 but is
   stylistically inconsistent with `combat.projectile_hit_mo`.
3. Pass-1 noted: "It would be cleaner to fold into the M5 family at M6's
   invocation." Pass-2 agrees and elevates to **P1**.

**Recommended schemas:**

```text
combat.melee_hit_mo {
  attacker_id, weapon_id (melee item),
  target_id, hit_zone (BodyZone enum),
  impact_point, impact_normal, impact_impulse, impact_energy_j,
  melee_kind ∈ {rifle_bash, knife_stab, hatchet, baton, kick, shoulder_check, stealth_kill},
  damage_kind (DamageKind enum — kinetic/thermal/electric/chemical/radiation; M6's "blunt"+"piercing" fold into kinetic),
  knockdown_rolled (bool), knockdown_chance,
  stealth (bool — true for stealth_kill_executed; M6.md:268 says "instant kill"),
  parent_hit_event_id (optional cause-chain pointer),
  hp_before, hp_after, damage_amount,
  armor_absorbed_dmg, passthrough_dmg, pierced_armor (bool),
  surface_kind,
  organ_damaged_id?, circuit_damaged_id?
}

combat.explosive_hit_mo {
  source_event_id (grenade_detonated event id),
  source_kind ∈ {frag, smoke, flash, stick, grenade_launcher, demolition_charge},
  target_id, hit_zone, impact_point, impact_normal,
  impact_impulse, impact_energy_j,
  radius_m, distance_from_center_m, overpressure_pa,
  damage_kind (kinetic for frag/stick; thermal optional; flash-only events
    may fire affliction.applied directly without an explosive_hit_mo),
  parent_hit_event_id, hp_before, hp_after, damage_amount,
  armor_absorbed_dmg, passthrough_dmg, pierced_armor,
  surface_kind, organ_damaged_id?, circuit_damaged_id?
}
```

This makes `combat.stealth_kill_executed` a degenerate case of
`combat.melee_hit_mo` with `stealth: true` instead of a separate event family
(reduces taxonomy churn).

**Severity: P1.** Not a blocker; M6 CAN ship its own ad-hoc schemas (e.g.
`combat.stealth_kill_executed`, `combat.melee_swing`). But shipping these as
M5-A2 (event-surface lock) is cleaner project hygiene AND saves M13 from
revisiting the schemas.

### NEW-B: Stance enum NOT locked in any M5 schema (M5-clean; M6 will own)

**Audit:** grep for `Stance` / `stance` across `cf-replay/schemas/event/*.json`
returned zero matches. `snapshot_actor.json:16` declares `"status": { "type":
"string" }` with no enum constraint (currently used for `active|dead|downed|
inactive` per `cf-actor::Status::as_str`).

The cf-actor crate already ships a 10-value Stance enum
(`cf-actor/src/lib.rs:130-145`): Idle, Walking, Running, Airborne, Downed,
Dead + chassis-aware Crouching, Climbing, Jetting, Ejecting. **This is NOT
the M6 23-value enum.** M6.md:31-32 specifies:

> Stance state machine: Stand, Walk, Run, Sprint, Crouch, CrouchWalk, Prone,
> ProneWalk, Slide, Vault, Climb, Dive, Lean, KnockedDown, Downed, Dying,
> Dead + RopeClimb / LadderClimb / PipeClimb / StealthAttack / KnifeThrow /
> Swim (reserved M16+).

**Drift surfaced:** the existing cf-actor `Stance::{Idle, Walking, Running,
Crouching, Climbing, Jetting}` will need to be renamed/extended to
`Stance::{Stand, Walk, Run, Sprint, Crouch, CrouchWalk, Prone, ProneWalk,
Slide, Vault, Climb, Dive, Lean, KnockedDown, Downed, Dying, Dead}` (plus
6 reserved). This is a cf-actor refactor + breaking observe contract change.

**Severity: P1 for M6.** Either (a) M6 owns the enum and the M5 schemas remain
clean OR (b) M5-A2 ships an `actor.stance_changed` schema with the locked
23-value enum so M6 implementers can rely on the validator.

**Recommendation:** Option (a). M5 deliberately stayed clear of `actor.*` per
the pass-1 audit ("Stance is actor-controller scope, M5 stayed clear of it.").
M6 owns the schema + the enum. The `snapshot_actor.status` field stays open
(string, no enum) so observe consumers can read either the old or new values
without a schema bump.

### NEW-C: FacingDirection enum is M6-owned (no conflict)

**Audit:** grep for `FacingDirection` across the workspace returned zero
matches. M6.md:495-503 introduces `FacingDirection { Left, Right }` and
`Actor::facing: FacingDirection` with default Right at spawn.

**Verdict: PASS.** No M5 lock; M6 owns. The `actor.facing_changed { from, to,
cause }` event lives in M6. M5 schemas don't reference facing — no conflict.

### NEW-D: M6 grenade damage kinds don't fold cleanly into M5's DamageKind (5 kinds)

**Audit:** M5's locked `DamageKind` enum is `["kinetic", "thermal", "electric",
"chemical", "radiation"]` (5 values; `combat_projectile_hit_mo.json:51`).

M6 grenade types per M6.md:148-152:

- **Frag** — kinetic (shrapnel) + thermal (heat from detonation)? Or just
  kinetic?
- **Smoke** — non-damaging; spawns smoke hazard
- **Flash** — deafen + blind afflictions; non-damaging? Or thermal+electric
  (bright light)?
- **Stick** — same as Frag

**No "explosive" damage kind in M5.** Question: does the producer use
`kinetic` for frag damage? Pass-1 didn't flag this.

**Pass-2 verdict:** **NO BLOCKER.** The 5 M5 damage kinds cover M6 cleanly:

- Frag/Stick blast → `kinetic` (shrapnel impulse) + secondary `thermal` if
  proximity is close enough.
- Smoke → no damage; spawns `hazard.spawned { kind: smoke }`.
- Flash → no damage; emits `affliction.applied { kind: deafened }` +
  `affliction.applied { kind: blinded }` (the M5-A1 addition).

The "explosive" *event* (`combat.explosive_hit_mo`) is what M6 needs, NOT a
new damage kind. The 5-kind DamageKind enum is sufficient. **Recommendation:**
M6 producer routes frag damage as `damage_kind: kinetic`; the
`combat.explosive_hit_mo.source_kind = frag` distinguishes the event class.

### NEW-E: actor.action_rejected.reason enum locked enum

**Audit:** M6.md:514-525 enumerates 7 limb-loss reasons:

- `no_arms_for_weapon`
- `single_arm_two_hand_weapon_rejected`
- `no_legs_for_movement`
- `single_leg_reduced_mobility`
- `no_hands_for_grip`
- `backpack_lost_no_jet`
- `head_destroyed_instant_death`
- `torso_destroyed_instant_death`

Plus implicit non-limb reasons:

- `tank_slot_locked_at_m2_2a` (M6.md:432-433)
- NaN/Inf input rejection (M1 + preserved at M6.md:475 — "NaN/Inf guards on
  ALL floating-point inputs")
- Stance-blocked actions (implied: e.g. sprint rejected when prone)
- Stamina-low (implied: sprint disabled when stamina=0)
- Ammo-empty (implied)
- In-air (implied: e.g. dig rejected when Airborne)
- Knocked-down (implied: most actions rejected)

**Severity: P2 (recommend M6 lock the full enum on `actor.action_rejected`).**

Recommendation: M6 spec sketches the limb-loss reasons; M6 implementer should
expand to a single locked enum on the schema:

```json
"reason": {
  "type": "string",
  "enum": [
    "no_arms_for_weapon",
    "single_arm_two_hand_weapon_rejected",
    "no_legs_for_movement",
    "single_leg_reduced_mobility",
    "no_hands_for_grip",
    "backpack_lost_no_jet",
    "head_destroyed_instant_death",
    "torso_destroyed_instant_death",
    "tank_slot_locked_at_m2_2a",
    "stance_blocked",
    "stamina_low",
    "ammo_empty",
    "in_air",
    "knocked_down",
    "nan_inf_input"
  ]
}
```

### NEW-F: Tank slot taxonomy — M6 owns reservation; M17+M19 fills physics

**Audit:** M6.md:170-186 explicitly says:

> M6 placeholder; M17 + M19 fill. At M6 close, tank slots are EMPTY +
> non-functional (inventory grid shows them as locked icons). The slots emit
> `inventory.tank_slot_reserved` events for M4's snapshot path. M17 ships the
> GasTank struct + 5 tier ladder. M19 ticks gas physics (PV=nRT, leak rate,
> decompression). The M6 reservation prevents future schema bumps when tanks
> ship.

**Verdict: PASS.** M6 owns the reservation event family. Tank slot kinds are
locked at 3: `tank_primary`, `tank_secondary`, `tank_utility`.

**Recommendation for M6 schema:**

```json
"slot_kind": {
  "type": "string",
  "enum": ["tank_primary", "tank_secondary", "tank_utility"]
}
```

### NEW-G: Stealth meter range — 0..1 normalized

**Audit:** M6.md:209 + M6.md:268 ("stealth_meter < 30%") suggests stealth_meter
is a normalized 0..1 with a 0.30 threshold for stealth kill availability.
M6.md:281-287 acceptance says "When detection > 50%: caption 'Spotted'".

**Verdict:** stealth_meter is 0..1 normalized (NOT 0..100 percentage).
The "30%" / "50%" prose in M6.md are spec shorthand for 0.30 / 0.50.

**Recommendation for M6 schema:**

```json
"perception.stealth_meter_changed.payload": {
  "from": { "type": "number", "minimum": 0.0, "maximum": 1.0 },
  "to": { "type": "number", "minimum": 0.0, "maximum": 1.0 }
}
```

### NEW-H: M6 perception integrates with M5 alarm cause-chain

**Audit:** `equipment.weapon_fired.loudness_radius` (M1) + `equipment.alarm_
registered.loudness_radius` (M1) are the existing acoustic signal events. M6
perception kernel:

- Reads M5/M1 events as audio sources.
- Multiplies loudness by `MaterialDef.loudness_modifier` for footsteps.
- Applies occlusion factor based on intervening material.

**Verdict: PASS.** No new M5 schema needed; M6 perception consumes existing
events and emits new `perception.*` events with `source_event_id` cause-chain
pointers back to the weapon-fired / alarm event.

**Recommendation:** M6's perception events should declare `source_event_id`
field as required, pointing to the originating equipment.* event id.

### NEW-I: M6 audio events reuse audio.event_requested with extended `kind` enum

**Audit:** M5-A1's `audio.event_requested.payload.kind` enum is currently
`["material_state", "internal_hit"]`. M6 introduces:

- Footstep sounds (M6.md:208 + M6.md:434-438)
- Alarm propagation sounds (implied by suppressor effect)
- Throw / detonation sounds (implied by grenade events)
- Tool sounds (foam spray, drill spin, welder hiss)
- HUD UI sounds (weapon swap, pickup, etc.)

**Question:** Does M6 fold these into `audio.event_requested` with new `kind`
values (e.g. `"footstep"`, `"alarm"`, `"tool"`, `"ui"`, `"grenade"`) OR ship
separate event types?

**Pass-2 recommendation:** **Fold into `audio.event_requested`.** This keeps
the event surface unified and allows cf-audio (M13.x) to consume a single
event family. M6-A1 (or M6 closure) extends the `kind` enum additively (per
DR-002):

```json
"kind": {
  "type": "string",
  "enum": [
    "material_state",
    "internal_hit",
    "footstep",
    "alarm_propagation",
    "grenade_throw",
    "grenade_detonation",
    "tool",
    "ui",
    "ambient"
  ]
}
```

This is an **additive enum extension** — M4 DR-002 allows it without an
envelope bump.

**Alternative (rejected):** ship separate `audio.footstep_requested` /
`audio.tool_requested` / `audio.ui_requested` events. Rejected because it
fragments the consumer surface for no benefit.

### NEW-J: M6 squad commands + AI integration — squad.* family is NEW at M6

**Audit:** M5 didn't touch the `squad.*` event family. M6.md:286-298 introduces
4 commands via `act.squad.issue_command`:

- `FollowLeader`
- `HoldPosition`
- `DefendPoint(pos)`
- `PushToWaypoint(pos)`

**Verdict: M6 owns.** Recommended schemas:

```json
"squad.command_issued.payload": {
  "squad_id": { "type": "integer" },
  "issuer_actor_id": { "type": "integer" },
  "command_kind": {
    "type": "string",
    "enum": ["follow_leader", "hold_position", "defend_point", "push_to_waypoint"]
  },
  "target_actor_id": { "type": ["integer", "null"] },
  "target_position": { "type": ["array", "null"], "minItems": 2, "maxItems": 2 }
}
```

M7 extends with more commands (M6.md:298 — "Bot uses M2's ReactiveGuard
baseline at M6; M7 replaces with 5 archetypes"). M6 reserves the additive
extension headroom.

### NEW-K: Stance state machine + 23-value enum (M6 ships from scratch)

**Audit:** M6.md:31-32 specifies the full enum:

> Stand, Walk, Run, Sprint, Crouch, CrouchWalk, Prone, ProneWalk, Slide,
> Vault, Climb, Dive, Lean, KnockedDown, Downed, Dying, Dead +
> RopeClimb / LadderClimb / PipeClimb / StealthAttack / KnifeThrow / Swim
> (reserved M16+)

Total: 17 active + 6 reserved = 23 values. M6.md:530-535 adds 2 limb-loss-induced
stances: `Crawl` + `KneelStance`. Total enum size if M6 includes limb-loss:
**25 values**.

**Pre-existing M5 cf-actor::Stance has 10 values** (Idle/Walking/Running/
Airborne/Downed/Dead + Crouching/Climbing/Jetting/Ejecting), with serde
rename_all="snake_case".

**Compatibility analysis:**

| cf-actor (current) | M6 mapping | Notes |
|---|---|---|
| `Idle` | `Stand` | Rename |
| `Walking` | `Walk` | Rename |
| `Running` | `Run` | Rename |
| `Airborne` | (no direct mapping) | M6 doesn't have a generic Airborne; uses Vault / Climb / Dive instead. Could keep as `Airborne` reserved. |
| `Downed` | `Downed` | KEEP |
| `Dead` | `Dead` | KEEP |
| `Crouching` | `Crouch` | Rename |
| `Climbing` | `Climb` | Rename |
| `Jetting` | (no direct mapping at M6; preserved for M16 jet ladder) | Reserved |
| `Ejecting` | (no direct mapping at M6; preserved for M5 chassis salvage) | Reserved |

**Severity: P1 for M6 implementer.** This is a cf-actor enum refactor + a
`snapshot_actor.status` observe-string semantic shift. Either:

- (a) M6 renames cf-actor::Stance variants. Old observe strings (`"walking"`)
  become `"walk"` etc. — breaks any consumer hardcoded against the old strings.
- (b) M6 adds new variants and keeps the old as aliases. cf-actor exposes
  both `Stance::Walking` and `Stance::Walk` (both serialize to `"walk"`?
  Complicated).
- (c) M6 keeps the cf-actor enum at the legacy 10 values for backward compat
  AND ships a separate `Stance::Extended` enum for the full 25-value M6 set.

**Pass-2 recommendation:** Option (a). The cf-actor crate is internal; the
observe contract changes are tracked in the observe_frame.schema.json version
bump (the schema is at v1 today — bumping to v2 with the new stance vocab is
permitted by M4 DR-002 only at the observe surface, NOT the recorder envelope).

The new event `actor.stance_changed { from, to, cause }` lives in M6 with the
25-value locked enum.

### NEW-L: BodyZone limb-loss + Stance forced transitions cross-check

**Audit:** M5's `BodyZone` enum has 15 values (verified in
`armor_layer_destroyed.json:18`):

```
head, torso, arm_left, arm_right, forearm_left, forearm_right,
hand_left, hand_right, leg_left, leg_right, shin_left, shin_right,
foot_left, foot_right, backpack
```

M6 limb-loss table (M6.md:514-525) uses semantic categories:

- "both arms" — derived from `arm_left` + `arm_right` + `forearm_*` + `hand_*`
  states
- "single arm" — derived from XOR
- "both legs" — derived from `leg_left` + `leg_right` + `shin_*` + `foot_*`
- "single leg" — derived from XOR
- "both hands" — derived from `hand_left` + `hand_right`
- "backpack" — direct match
- "head" — direct match
- "torso" — direct match

**Verdict: PASS.** BodyZone covers M6's limb vocabulary fully. The "both arms"
predicate is a M6 derivation over the 15-zone state — no schema bump needed.

**Forced-stance transitions on limb loss** (M6.md:530-535):

| Limb loss | Forced stance |
|---|---|
| Both legs lost | `Crawl` (NEW M6 stance, 1500ms transition) |
| Single leg lost | `KneelStance` (NEW M6 stance) |
| Both arms lost | `Stand` (no stance change; weapons disabled) |

`Crawl` + `KneelStance` are M6 enum additions (per NEW-K). No M5 conflict.

### NEW-M: M5 spec moved to done/ but body not updated

**Audit:** `specs/done/M5.md:5` still says `Status: active`. The implementer
notes skeleton at the bottom of M5.md still shows `"const": "0.1"` (line
referenced in pass-1 audit § A2). The acceptance criterion at the top says
"each schema declares schema_version=\"0.1\" matching the M4 locked envelope".

**Pass-1 audit explicitly listed this as a recommended fix** (pass-1 § A2 step
4 + step 5). **Pass-2 verifies the fix did NOT land** — the M5 spec body is
unchanged from pre-pass-1 state.

**Project hygiene question:** does M5 need a spec amendment?

**Pass-2 recommendation: YES (P3, project hygiene).** Either:

- (a) Edit `specs/done/M5.md` in-place: flip `Status: active` → `Status: done`;
  rewrite the implementer-notes skeleton to use the canonical envelope literal
  + the new `audio.event_requested` schema + the `parent_hit_event_id` rename;
  update the acceptance criterion text.
- (b) Leave M5.md as historical (frozen at the moment it was moved to done/)
  and capture the M5-A1 amendment in a new `specs/done/M5-A1.md` file.

The repo doesn't establish a convention for amendment files (no
`specs/done/Mx-Ay.md` precedent — verified via `LS specs/done/`). My
recommendation: option (a) for minor in-spec text edits + option (b) for
substantive new event/enum additions like `blinded` + `audio.event_requested`
+ `parent_hit_event_id` rename, since these are "added in pass-1" rather
than "originally in M5".

**Either way, the schemas + validator are correct.** This is documentation
hygiene only; not a M6 blocker.

### NEW-N: CHANGELOG entry for M5-A1

**Audit:** `git log --oneline -5` shows `1784ad2 M5-A1: post-audit hardening
pass — 17 audit findings closed; ready for M6`. The CHANGELOG.md has no
entry for M5-A1 (verified via `grep -n M5-A1 CHANGELOG.md` — returns no
matches in the CHANGELOG sections; only matches in repo paths).

The CHANGELOG's "Unreleased" section is anchored on BP3; M5 itself was
labelled `LANDED` in the BP3 matrix with the M5-A1 fixes not yet documented.

**Pass-2 recommendation: NEEDED (P3, project hygiene).** Add an entry under
`### BP3 — Combat Readability Build` table OR as a sibling sub-section:

```markdown
**M5-A1 — Post-audit hardening pass (LANDED 5/13/2026):**

Six parallel audit workers reviewed every M5 schema + the cf-replay payload
validator + the cf-mod schema-file validator + the M4 envelope contract +
M6 readiness. 17 audit findings closed in commit `1784ad2`.

CRITICAL fixes:
- All 74 M5 schemas' `schema_version.const` realigned from "0.1" to canonical
  "prototype-recorder-event.v0.1" — producers no longer have to lie on emit
  for strict JSON Schema validation.
- `combat.projectile_hit_mo.payload.parent_event_id` renamed to
  `parent_hit_event_id` (matches `origin.shot_force_feedback` +
  `internal.organ_damaged` naming; resolves envelope-level field collision).
- New `audio.event_requested` schema ships the 7-material × 5-impact-state
  taxonomy + 6-name internal-hit enum (M5 spec promise — "M5 just locks the
  request shape").
- `blinded` affliction kind added across all 4 affliction.* schemas (M6 flash
  grenade needs it).

MEDIUM hardening: Origin enum locked via oneOf on concussion + origin events;
fluid.ignition.fluid_kind tightened to oil/fuel combustible subset; 4 cosmetic
events tightened to `cosmetic: const true`; concussion dose 0..100 ceiling;
phase enums locked on atmos + thermal; new `snapshot.snapshot_shield` schema
for M9 firehose.

VALIDATOR hardening: cf-mod rejects payload.additionalProperties=false;
envelope-dir regex widened to ^v[0-9]+(_[0-9]+)?$; cf-replay payload validator
gained oneOf + maximum support.

Tests added: m5_per_family_happy_path (one round-trip per family);
m5_combat_projectile_hit_mo_rejects_envelope_named_parent; m5_concussion_dose_
changed_rejects_bad_origin; m5_event_schema_rejects_legacy_short_literal +
m5_event_schema_rejects_payload_additional_properties_false +
m5_envelope_version_dir_regex_accepts_canonical_forms.

M6 readiness verified: M5's enums (BodyZone, DamageKind, ArmorLayer, ammo
tiers, surface kinds, hazard kinds, fluid kinds, affliction kinds incl.
`blinded`) cover M6's actor controller + equipment + grenade + sound +
perception scope. M6's 3 new categories (perception/squad/inventory) are
orthogonal to M5's damage scope.
```

### NEW-O: M5-A1 spec amendment file

**Audit:** The repo has no `specs/done/M5-A1.md` file. Convention question:
should pass-1's hardening pass be captured in a dedicated amendment file or
remain in commit history only?

**Pass-2 recommendation: NOT NEEDED at the spec-file level; NEEDED at the
audit-archive level.**

Rationale:
- M5-A1 is purely a hardening pass (no new feature surface); doesn't warrant
  a new spec.
- The audit report at `audit-m5/06-envelope-m6-readiness-audit.md` captures
  the pre-pass-1 state. The commit message `1784ad2` captures the post-pass-1
  state.
- This pass-2 audit (the file you are reading) captures the post-pass-2 state.

**What IS missing:** there's no `audit-m5/00-pass1-summary.md` or
`audit-m5/SUMMARY.md` consolidating the 6 pass-1 auditors' findings + the
M5-A1 commit's resolutions. The pass-1 audits are scattered across 6 separate
files in `audit-m5/`. **Project hygiene recommendation (P3):** add a
`audit-m5/SUMMARY.md` listing the 6 pass-1 files + the M5-A1 commit + the
17 audit findings + their resolutions.

### NEW-P: M6 dependencies on M5 — exhaustive cross-reference

**Audit:** M6.md "Dependencies" block (M6.md:481-485) says:

> - **M1 closed**: extends 9-action surface to 36
> - **M2 closed**: extends 1 ReactiveGuard with squad surface
> - **M3 (must close)**: tools (foam, concrete, drill, welder) consume
>   `try_carve` + `try_fill_or_repair`
> - **M4 in flight**: M6 produces 30+ new event types; M4's locked envelope
>   must accept

M6 does NOT explicitly list M5 as a dependency. **But M5 enum surface is
implicitly required**: BodyZone (limb loss), AfflictionKind incl. `blinded`
(flash grenade), HazardKind incl. `smoke` (smoke grenade), DamageKind (melee
+ projectile + explosive).

**Pass-2 cross-validate:**

| M5 surface | M6 consumer | Lock status |
|---|---|---|
| `BodyZone` 15 values | limb-loss table (M6.md:514-525) | LOCKED in armor.* family |
| `DamageKind` 5 values | melee + projectile + explosive routing | LOCKED in `combat.projectile_hit_mo` |
| `AfflictionKind` 23 values (post-pass-1) | flash grenade deafen+blind; knife bleed; rifle bash concuss | LOCKED in affliction.* family |
| `HazardKind` 9 values | smoke grenade spawns `smoke` | LOCKED in `hazard.spawned` |
| `ArmorLayer` 3 values | bipod deploy effect on armor — wait, no, M6 doesn't touch armor; M13 does | NOT M6-relevant |
| `AmmoRoundTier` 8 values | M6 rifle/SMG/sniper/etc. ammo selection | LOCKED in `combat.projectile_hit_mo` |
| `SurfaceKind` 8 values | M6 hits on flesh / armor_* / unarmored / terrain | LOCKED in `combat.projectile_hit_mo` |
| `OrganId` 15 values | M6 deep-damage hits (M13 deeper integration) | LOCKED in `internal.organ_damaged` |
| `CircuitId` 12 values | M6 robot deep-damage (M13 integration) | LOCKED in `internal.circuit_damaged` |
| `ShieldStatus` 4 values | M6 shields (chassis-only at M6; full at M13) | LOCKED in shield.* family |
| Origin enum 5 values | M6 concussion dose attribution | LOCKED via `oneOf` |
| Phase enums | M6 doesn't tick atmos/thermal | NOT M6-relevant |
| HazardClass 15 values (env signal) | M6 doesn't read env signal directly | NOT M6-relevant |

**Verdict: PASS.** M5 enum lock covers all M6-implied surfaces.

**Action item: M6 spec should add an explicit "M5 closed" line in its
Dependencies block.** Currently M6.md:481-485 lists M1, M2, M3, and M4 (in
flight), but NOT M5. M5 has subsequently closed (per BP3 status), so this is
purely cosmetic — but project hygiene suggests M6 implementer flips the M6.md
Dependencies block to include "M5 closed: enum surface available (BodyZone,
DamageKind, AfflictionKind incl. blinded, HazardKind, etc.)".

**Severity: P3 cosmetic.**

### NEW-Q: Per-weapon recoil patterns — actor controller internal, no event

**Audit:** M6.md:119-128 specifies locked recoil patterns:

| Weapon | Recoil pattern |
|---|---|
| Rifle | up-right zigzag |
| SMG | sharp up-right |
| Shotgun | strong upward |
| Sniper | minimal but slow recovery |
| Pistol | minimal up |
| Grenade Launcher | strong upward |

**Question:** Do these emit a `recoil_pattern_applied` event, or is it actor
controller internal?

**Pass-2 verdict: actor controller internal.** The `equipment.weapon_fired`
event carries `recoil_impulse` (M1 schema, line 11:
`"recoil_impulse": {"type": "number", "minimum": 0.0}`). The pattern is the
DIRECTION of the impulse vector applied to the actor's aim cone over time —
producer-internal state machine; no event emission needed.

**Severity: NONE.** No M5 or M6 schema gap.

### NEW-R: Hotbar swap timing — paired weapon_swap_started + weapon_swap_completed

**Audit:** M6.md:417-419 acceptance:

> When player presses 2
>   Then equipment.weapon_swap_started fires
>   And 300ms transition (HUD shows large icon)
>   Then equipment.weapon_swap_completed fires

**Verdict:** M6 ships PAIRED events (start + complete), NOT a progress event.
Confirmed in M6.md spec acceptance text. Pass-2 prefers this pattern over a
single `weapon_swap_progress` event — it gives replay-side reconstruction a
clean cause-chain (`weapon_swap_completed.parent_event_id = weapon_swap_
started.event_id`).

**Pass-2 schema sketch:**

```json
"equipment.weapon_swap_started.payload": {
  "actor_id": { "type": "integer" },
  "from_slot": { "type": "integer", "minimum": 0, "maximum": 7 },
  "to_slot": { "type": "integer", "minimum": 0, "maximum": 7 },
  "transition_ms": { "type": "integer", "enum": [200, 300] },
  "from_weapon_id": { "type": ["integer", "null"] },
  "to_weapon_id": { "type": "integer" }
}

"equipment.weapon_swap_completed.payload": {
  "actor_id": { "type": "integer" },
  "slot": { "type": "integer", "minimum": 0, "maximum": 7 },
  "weapon_id": { "type": "integer" },
  "parent_swap_event_id": { "type": "string" }
}
```

### NEW-S: Stamina events — actor.* category

**Audit:** M6.md:24-30 — stamina is 0..1 with -0.2/s drain when sprinting +
0.3/s recovery. M6.md:241-246 acceptance: "Sprint depletes stamina + auto-cancels
at 0".

**Question:** stamina event family — `stamina.*` or `actor.*`?

**Pass-2 verdict: fold into actor.* (no new category).** Stamina is a
per-actor pool state; consistent with M5's pattern of using `actor.*` for
per-actor state changes. Recommended events:

```text
actor.stamina_changed { actor_id, from, to, cause: 'sprint_drain' | 'recovery' | 'reset' }
actor.stamina_exhausted { actor_id }  (when stamina drops to 0)
actor.stamina_recovered { actor_id, threshold: 'low' | 'full' }  (cosmetic? recovery banner)
```

**Severity: P2.** M6 owns; M5 doesn't conflict. The high-cardinality nature
of stamina-changed (potentially every tick) suggests cosmetic-batching like
`affliction.tick` — see pattern from `affliction_tick.json` and
`hazard_tick.json`.

### NEW-T: Pass-1 conclusion vs current state — does READY still hold?

**Pass-1 verdict (verbatim):**

> **M6 readiness**: **READY** — M5 does NOT block M6 closure. M6's new
> categories (perception, squad, inventory) and new events
> (actor.facing_changed, actor.action_rejected, equipment.*,
> combat.stealth_kill_executed) are orthogonal to M5's damage scope. M5's
> BodyZone + DamageKind + ArmorLayer + ammo + surface enums all match M6's
> usage. The `audio.event_requested` schema gap (M5 promise) and `blinded`
> affliction gap (M6 surface) should be patched, but they are not blockers —
> M6 can ship the schemas itself if M5 isn't reopened.

**Pass-1 "should be patched" items + their pass-2 status:**

| Pass-1 finding | Pass-2 status |
|---|---|
| `audio.event_requested` schema gap | **CLOSED in M5-A1.** |
| `blinded` affliction missing | **CLOSED in M5-A1.** |
| `combat.melee_hit_mo` symmetry suggestion | **STILL OPEN.** Recommended for M5-A2 (NEW-A). |
| `combat.explosive_hit_mo` symmetry suggestion | **STILL OPEN.** Recommended for M5-A2 (NEW-A). |
| Defensive `cosmetic: const false` on 69 gameplay schemas | **PARTIALLY CLOSED.** M5-A1 added `cosmetic: const true` on the 4 cosmetic events (preventing mis-emit as cosmetic the OTHER way). The 69 gameplay schemas still allow `cosmetic` via `additionalProperties: true`. Pass-2 verdict: low-priority defensive hardening; M6 doesn't need it. |

**Pass-2 verdict: M6 readiness REMAINS READY.** The two M5-A1 fixes have
landed; the remaining open items (combat.melee_hit_mo + explosive_hit_mo)
are P1 quality-of-life recommendations, not blockers.

### NEW-U: M6 carries 30+ new event types; M4 validator must register each

**Audit:** M6.md:73 says "M6 produces 30+ new event types". Pass-2's matrix
above enumerates ~35 new events. Each new event needs:

1. JSON schema file under `cf-replay/schemas/event/<family>_<type>.json`
2. `include_str!` const + `event_schema_for` arm in
   `cf-replay/src/schemas.rs`
3. Unit test in the `schemas_load_for_every_registered_event_type` list
4. Optional happy-path test in `m5_per_family_happy_path` style

**Severity: NONE — purely M6 implementer's work.** No M5-side change needed.

### NEW-V: M6 weight system threshold (>30kg forces Walk)

**Audit:** M6.md:401-405 acceptance:

> Given total inventory weight = 35kg + carry_limit = 30kg
> Then actor stance forced to Walk (sprint rejected)

The 30kg threshold is locked at M6. M5 didn't touch this surface. **Pass-2
recommendation:** lock the threshold in `equipment.weight_overload_changed`
payload as documentation:

```json
"description": "...30kg carry_limit forces Stance::Walk; sprint rejected via actor.action_rejected with reason='weight_overload'..."
```

**Severity: NONE.** No M5 schema gap.

### NEW-W: equipment.item_dropped (Q-key) vs actor.inventory_dropped (legacy "shot off backpack")

**Audit:** The existing `actor.inventory_dropped` schema (M1 audit pass 6)
has fields `[actor, item_id, hand_position, toss_velocity]`. M6.md:411-413
implies the same shape for the Q-key drop:

> Given slot 1 with rifle
> When act.player.drop_item slot=1
> Then equipment.item_dropped fires
> And rifle spawns at hand position with toss velocity

**Question:** Does M6 reuse `actor.inventory_dropped` (M1 legacy schema with
the right shape) OR ship a new `equipment.item_dropped`?

**Pass-2 recommendation:** **REUSE `actor.inventory_dropped`** (M1 legacy).
The schema's already-correct shape covers both legacy "shot off backpack"
AND M6 Q-key drop semantics. **Add a `cause` field** in M6 to disambiguate:

```json
"actor.inventory_dropped.payload": {
  ... existing fields ...
  "cause": {
    "type": "string",
    "enum": ["backpack_shot_off", "player_drop", "death_drop", "limb_loss"]
  }
}
```

(`cause` is an additive extension; allowed under M4 DR-002.)

**Severity: P3 (M6 implementer decision).** Either approach is allowed; the
reuse path is cleaner.

### NEW-X: Tracer round determinism — fold into combat.projectile_spawned

**Audit:** M6.md:230-234 acceptance:

> Given Rifle with RTTRatio=3
> When fires 9 rounds
> Then 3 tracers + 6 regular spawn in deterministic order
> And replay reproduces the exact pattern

**Question:** Does this need a new `equipment.tracer_round_spawned` event, or
fold into the existing `combat.projectile_spawned`?

**Pass-2 recommendation:** **FOLD into `combat.projectile_spawned`** with an
additive `is_tracer: bool` field. M6 update to projectile_spawned.json:

```json
"combat.projectile_spawned.payload": {
  ... existing fields ...
  "is_tracer": { "type": "boolean", "description": "M6: deterministic 1:N tracer ratio per weapon's RTTRatio. M5+ producer fills." }
}
```

**Severity: NONE.** Additive field; no schema bump needed; replay determinism
verified via the existing event_id ordering.

---

## M5 enum coverage for M6 surfaces

Comprehensive table proving M5's locked enum vocabulary is sufficient for M6:

| M5 enum | Locked at | Used by M6 | Status |
|---|---|---|---|
| **BodyZone (15)** — head, torso, arm_left, arm_right, forearm_left, forearm_right, hand_left, hand_right, leg_left, leg_right, shin_left, shin_right, foot_left, foot_right, backpack | armor.* family (M5) | hit zone selection + limb-loss state | **PASS** — M6 limb-loss table maps onto BodyZone cleanly |
| **DamageKind (5)** — kinetic, thermal, electric, chemical, radiation | combat.projectile_hit_mo (M5) | All M6 damage routing (frag → kinetic; flash → no damage; melee blunt+piercing → kinetic) | **PASS** — 5 covers M6's grenade/melee/projectile |
| **AfflictionKind (23 after M5-A1)** — burning, wet, electrified, poisoned, hypoxic, combustible_atmosphere, breach_decomp, hyperthermic, hypothermic, radiation, concussed, deafened, **blinded**, bleeding, internal_shock, low_battery, coolant_leaking, oil_leaking, overheating, hunger, thirst, sleep_dep, sanity_low | affliction.* family (M5-A1) | flash grenade deafen+blind, knife bleed, rifle bash concuss | **PASS** — `blinded` added in M5-A1 |
| **ArmorLayer (3)** — External, Internal, Core | armor.* family (M5) | M6 doesn't touch armor.* directly (M13 chassis ladders up) | **PASS (irrelevant at M6)** |
| **AmmoRoundTier (8)** — standard, armor_piercing, hardened_AP, discarding_sabot, explosive_warhead, kinetic_impact, HEAT, APFSDS | combat.projectile_hit_mo (M5) | M6 6 weapons: rifle/SMG/pistol/shotgun → standard; sniper → standard|hardened_AP; grenade launcher → explosive_warhead | **PASS** |
| **SurfaceKind (8)** — armor_external, armor_internal, armor_core, armor_chunked_breach, flesh, circuit, unarmored, terrain | combat.projectile_hit_mo + audio.event_requested (M5/M5-A1) | M6 hit routing | **PASS** |
| **OrganId (15)** — brain, eyes_left, eyes_right, ears_left, ears_right, heart, lungs_left, lungs_right, liver, kidneys_left, kidneys_right, spine, stomach, intestines, pancreas | internal.organ_damaged + combat.projectile_hit_mo (M5) | M6 deep-damage routes via M13 chassis (orthogonal at M6 close) | **PASS** |
| **CircuitId (12)** — power_core, cpu, sensor_array, motor_controller_left_arm, motor_controller_right_arm, motor_controller_left_leg, motor_controller_right_leg, hydraulic_pump, coolant_pump, oil_reservoir, fuel_tank, comm_relay | internal.circuit_damaged + combat.projectile_hit_mo (M5) | Same as OrganId | **PASS** |
| **FluidKind (4)** — oil, coolant, fuel, electrolyte | fluid.* family (M5) | M6 doesn't tick fluids (M13 chassis fluid system) | **PASS (irrelevant at M6)** |
| **HazardKind (9)** — fire, smoke, electric, wet, hot, cold, acid, radiation, toxic | hazard.* family (M5) | M6 smoke grenade emits hazard.spawned{kind:smoke}; flash may spawn electric? (spec ambiguous, see NEW-D) | **PASS** — `smoke` covers M6's smoke grenade |
| **Origin (5)** — Human, Android, Robot, PoweredOrganic, HeavyBiomech | concussion.dose_changed + origin.shot_force_feedback (M5-A1 via oneOf) | M6 friendly bot is M2 ReactiveGuard origin (single-archetype at M6) | **PASS** |
| **Origin decay rates** — 5/s human + 2/s robot internal_shock | concussion + internal_shock (M5) | M6 doesn't tick origin model directly (M17 producer fills) | **PASS (irrelevant at M6)** |
| **AfflictionBand (6)** — Clear, Mild, Moderate, Severe, KO_Imminent, KO | concussion.band_changed (M5) | M6 rifle bash + shoulder check could feed concussion accumulator at producer level | **PASS** — additive |
| **ShieldStatus (4)** — Up, Down, Regenerating, Disrupted | shield.* family (M5) | M6 doesn't ship a chassis shield system (M13 chassis owns) | **PASS (irrelevant at M6)** |
| **HazardClass (15)** — env-signal hazard tags | environment.signal_aggregated (M5-A1) | M6 doesn't read env signal directly | **PASS (irrelevant at M6)** |
| **GasKind (10)** — O2, N2, CO2, volatiles, pollutant, H2, N2O, H2O, O3, He | atmos.* family (M5) | M6 doesn't tick atmos | **PASS (irrelevant at M6)** |
| **PhaseKind (4-5)** — gas, liquid, solid, supercritical (atmos); +molten (thermal) | atmos + thermal (M5-A1) | M6 doesn't tick thermal | **PASS (irrelevant at M6)** |
| **EnvironmentSlice (11)** — atmospheric, gravitational, thermal, radiation, photic, em, weather, water, acoustic, day_night, comms | environment.signal_delta (M5) | M6 perception kernel may consume `acoustic` slice in the long term | **PASS** |
| **EnvironmentSignal schema-version 1** — schema_version + active_hazards | environment.signal_aggregated.signal (M5-A1) | M6 doesn't ship env signal producer | **PASS (irrelevant at M6)** |

**Verdict: 100% M5 enum coverage for M6's M5-dependent surfaces.** All M6
events that route through M5-locked enum surfaces have a valid value.

---

## M5 spec hygiene

| Item | Status | Recommendation |
|---|---|---|
| M5 spec at `specs/done/M5.md` body | Still says `Status: active`; skeleton uses old `"const": "0.1"`; acceptance text says `schema_version="0.1"` | **Edit in-place** (P3 hygiene). Flip status; update skeleton; rewrite acceptance text. |
| CHANGELOG entry for M5-A1 | **MISSING.** No entry under "Unreleased" or BP3. | **Add** (P3 hygiene). See NEW-N for the suggested entry. |
| `specs/done/M5-A1.md` amendment file | Not present | **NOT NEEDED** at spec-file level. The audit + commit message capture the change. |
| `audit-m5/SUMMARY.md` consolidation | Not present | **Recommended** (P3 hygiene). Consolidates the 6 pass-1 audits + the 17 M5-A1 fixes. |
| M6 spec Dependencies block lists M5? | **NO.** M6.md:481-485 lists M1, M2, M3, M4 — but not M5. | **Edit M6.md** (P3 cosmetic). Add: "M5 closed: enum surface available (BodyZone, DamageKind, AfflictionKind incl. blinded, HazardKind, etc.)". |

---

## Recommended fixes BEFORE M6 starts

### P1 — strongly recommended

1. **(P1, NEW-A) Ship `combat.melee_hit_mo` + `combat.explosive_hit_mo`
   schemas at M5-A2.** Closes the deep-damage symmetry gap. ~2-3 schemas + 2-3
   validator entries + 2-3 round-trip tests. Folds `combat.stealth_kill_
   executed` into `combat.melee_hit_mo { stealth: true }`. M13 chassis +
   M14 collision can immediately ladder up without revisiting.
2. **(P1, NEW-K) Decide cf-actor Stance enum strategy before M6 starts.**
   Either (a) rename cf-actor::Stance variants AND bump observe_frame.schema
   to v2 OR (b) keep cf-actor at the legacy 10 values + ship a parallel M6
   25-value enum in the new `actor.stance_changed` schema. Pass-2 recommends
   (a) for long-term consistency.

### P2 — recommended

3. **(P2, NEW-E) Lock the `actor.action_rejected.reason` enum at M6.**
   Expand to 15 values covering limb-loss + tank-slot-locked + stance-blocked
   + stamina-low + ammo-empty + in-air + knocked-down + nan_inf_input.
4. **(P2, NEW-I) Extend `audio.event_requested.kind` enum additively at M6
   start.** Add `footstep`, `alarm_propagation`, `grenade_throw`,
   `grenade_detonation`, `tool`, `ui`, `ambient` to the existing
   `[material_state, internal_hit]`. Allowed under DR-002.
5. **(P2, NEW-S) Pick a stamina event family.** Recommend `actor.stamina_*`
   over `stamina.*` to fold into the actor.* category. Lock at M6 close.

### P3 — project hygiene (non-blocking for M6)

6. **(P3, NEW-M) Edit `specs/done/M5.md` in-place** to remove the `"0.1"`
   skeleton + acceptance text + status flip.
7. **(P3, NEW-N) Add CHANGELOG entry** documenting M5-A1.
8. **(P3, NEW-O) Add `audit-m5/SUMMARY.md`** consolidating the 6 pass-1
   audits + M5-A1 fixes.
9. **(P3, NEW-P) Edit M6.md Dependencies block** to add M5 as a closed
   dependency.
10. **(P3, NEW-W) Decide reuse vs new on `actor.inventory_dropped` vs
    `equipment.item_dropped`** — recommend reuse with additive `cause` field.

---

## Summary

| Metric | Value |
|---|---|
| Pass-1 deliveries verified | **4/4 PASS** (audio.event_requested + blinded + schema_version canonical + parent_hit_event_id rename) |
| Pass-1 bonus deliveries verified | **13/13 PASS** (Origin enum lock, fluid.ignition tightening, cosmetic const true on 4 events, concussion dose ceiling, phase enum locks, snapshot.snapshot_shield, validator hardening, etc.) |
| New issues found (pass-2) | **24** (NEW-A through NEW-X; lettered A-T + U+V+W+X) |
| Critical M6 blockers from M5 surface | **0** |
| P1 recommended M5-A2 fixes | **2** (combat.melee_hit_mo + combat.explosive_hit_mo schemas; Stance enum strategy decision) |
| P2 recommended M6-opening fixes | **3** (action_rejected.reason enum; audio.event_requested kind extension; stamina family decision) |
| P3 project-hygiene items | **5** (M5 spec body update; CHANGELOG; audit summary; M6 deps cross-ref; inventory_dropped reuse) |
| M5 enum coverage for M6 surfaces | **18/18 PASS** (no M5 enum gap for any M6 surface) |
| M6 event coverage matrix rows | **40+** (across actor, equipment, combat, perception, squad, inventory, audio families) |

**Pass-2 M6 readiness verdict: READY (REMAINS READY POST PASS-1).**

Pass-1's READY verdict still holds after M5-A1 closed `audio.event_requested`
+ `blinded` affliction. The remaining 24 pass-2 findings are quality-of-life
improvements (combat.melee_hit_mo + explosive_hit_mo symmetry), enum hygiene
(action_rejected.reason, audio.event_requested.kind extension), and project
hygiene (CHANGELOG, M5 spec body, audit summary). **None of these are M6
blockers.** M6 can ship without any of them; M6 ships better with the P1+P2
items addressed first.

**Critical M6 blockers:** NONE.

**Recommended pre-M6 enhancements:** P1 items (NEW-A combat hit-mo
symmetry + NEW-K Stance enum strategy) should be discussed with the user
before M6 starts. Both can be addressed in a single ~15-20 min M5-A2 commit
(2-3 new schemas + validator entries + tests; 1 cf-actor enum rename + observe
schema bump). The remaining P2/P3 items can land during M6 itself.

---

## Cross-references

- M5 spec (closed): `specs/done/M5.md`
- M5-A1 commit: `1784ad2` (post-audit hardening; 17 findings closed)
- M6 spec (active): `specs/active/M6.md`
- Pass-1 audit: `audit-m5/06-envelope-m6-readiness-audit.md`
- M4 envelope: `game/crates/cf-replay/schemas/v0_1/recorder_event.schema.json`
- M4 lib: `game/crates/cf-replay/src/lib.rs:51` (`EVENT_SCHEMA_VERSION`)
- cf-replay validator: `game/crates/cf-replay/src/schemas.rs` (lines 1-1166)
- cf-replay validator tests: `schemas.rs:710-1166` (incl. `m5_per_family_
  happy_path` + `m5_combat_projectile_hit_mo_rejects_envelope_named_parent`
  + `m5_concussion_dose_changed_rejects_bad_origin`)
- cf-mod validator: `game/crates/cf-mod/src/main.rs`
- cf-actor::Stance: `game/crates/cf-actor/src/lib.rs:127-145` (legacy 10-value
  enum; NEW-K refactor candidate)
- Pass-1 audit conclusion (verbatim): "M5 does NOT block M6 closure. ... The
  audio.event_requested schema gap (M5 promise) and blinded affliction gap (M6
  surface) should be patched, but they are not blockers." — PASS-2: both
  patched in M5-A1; READY verdict preserved.
