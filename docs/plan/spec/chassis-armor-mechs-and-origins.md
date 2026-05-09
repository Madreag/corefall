---
type: spec
status: planning-anchor-v0
authority: "Direction and contract for chassis/armor/mechs/origins systems. Specific implementations remain open."
ready_when: "Slice A includes a single chassis-bearing actor whose damage stages drive HUD, AI, and replay events end-to-end."
feeds:
  - DR-003
  - DR-004
  - DR-008
  - DR-014
---

← [[spec/index|spec section]] · [[spec/authoritative-game-spec-v0|game spec v0]] · [[spec/body-damage-model|body damage model]] · [[spec/origin-reaction-and-resource-model|origin reaction/resource model]] · [[spec/equipment-loadout|equipment/loadout]] · [[decisions/dr-014-tone-player-promise|DR-014 tone]] · [[decisions/dr-003-body-damage-readability|DR-003 body damage]] · [[decisions/dr-008-ai-architecture|DR-008 AI]]

# Chassis, Armor, Mechs, And Origins

> [!summary] What this page is
> The unified spec for "things that wear damage": foot infantry armor, powered armor, mechs, robots/androids, and origin/race-specific bodies. They share one chassis grammar so AI, UX, replay, modding, and mission code can treat them through a single contract.

> [!warning] Authority boundary
> v0 planning anchor. Specific values, content scope, and visual direction stay open until prototype evidence backs them. The grammar here is a commitment; the catalog is illustrative.

## Why One Page

Foot infantry, powered armor, mechs, and androids all face the same problems:

- Local damage that should be readable.
- Equipment/modules that can fail before the actor dies.
- A pilot/operator that may survive the chassis.
- AI that needs to know when to bail vs fight on.
- A replay recap that explains what failed first.
- A mod author who wants to add a new chassis without re-implementing damage rules.

Spreading this across separate pages makes the contract drift. One page, one grammar.

## Chassis Grammar

A chassis is the layered physical thing an actor inhabits or wears.

| Layer | Description | Example: Infantry | Example: Powered Armor | Example: Light Mech | Example: Android |
|---|---|---|---|---|---|
| Operator/pilot | The "brain" the player or AI commands. May be the same as the actor body, separate, or an embedded command core/avatar state. | Soldier | Soldier in suit | Pilot in cockpit | Synthetic mind |
| Frame | Skeleton/frame that defines mass, mobility, weapon hardpoints. | Bone/muscle | Light exo | Bipedal frame | Endoskeleton |
| Armor layers | Independently damageable plates / shells / undersuits. | Helmet, vest, undersuit | Helmet, plate, undersuit, joints | Cockpit plate, leg plate, arm plate, joint shell | Outer hull, inner skin |
| Modules | Bolted-on subsystems with their own state (sensors, jet, shield, repair drone, ammo box). | Backpack, NVGs | Jet pack, HUD optic, shield emitter | Targeting computer, jet pods, shield, ammo silo | Sensor pod, comm relay |
| Held / mounted equipment | Weapons and tools in hands or on hardpoints. | Rifle, grenades | Rifle, grenades, breaching tool | Mounted cannon, missile rack, drill | Sidearm, beam emitter |

Every layer can independently take damage and emit events.

## Damage Stages

The damage model is **stage-based**, not a single HP bar. Stages are readable to the player, AI, replay, and mods.

| Stage | What The Player Sees | Example Triggers | AI Behavior Hook |
|---|---|---|---|
| `nominal` | Default operational. | Spawn / repair to full. | Normal goals. |
| `degraded` | Cosmetic wear, smoke wisps, slight performance hit. | Minor hits to a layer. | No change. |
| `module-warning` | Specific module flashing in HUD; minor effect. | A module crosses its warn threshold. | Avoid escalating; suppress if non-critical. |
| `module-failed` | One module disabled (e.g. jet, shield, sensor). | Module crosses its failure threshold. | Reroute behavior; request repair if available. |
| `weapon-jammed` | Held/mounted weapon stops working until cleared. | Per-weapon jam roll on stress events. | Switch weapon, take cover, clear jam. |
| `armor-cracked` | A specific armor plate is torn open; underlying layers exposed. | Sustained damage to an armor zone. | Pivot away from the cracked side. |
| `disabled` | Chassis can no longer move or fight effectively. | Mobility module + leg armor failure, or mass damage. | Bail out / call rescue / surrender / self-destruct depending doctrine. |
| `pilot-injured` | Operator wounded inside a still-functional chassis. | Penetration past last armor layer. | Treat / extract / continue if AI doctrine allows. |
| `eject` | Pilot leaves chassis while it can still be saved. | Doctrine threshold hit. | Pilot becomes a separate actor. |
| `bail-too-late` | Pilot tries to leave but doesn't make it. | Eject after explosion threshold. | Mission failure event. |
| `wreck` | Chassis is destroyed but recoverable (salvageable, repairable). | Reached max damage but no critical-explosion roll. | Becomes a salvage object. |
| `gibbed/exploded` | Chassis is gone (with all the spectacle). | Critical-explosion roll on `wreck`, or massive single-event damage. | All onboard equipment becomes scattered debris/loot. |

Stages are not strictly linear. A jet module can fail without the chassis being disabled. A weapon can jam at `nominal`. The point is each transition has a reason and an event.

## Module System

Modules are first-class data, mounted in named slots on a chassis. Each module has:

- `module_id` (preset reference).
- `slot_id` (where on the chassis).
- `mass`, `power_draw`, `bandwidth_or_other_resource_use`.
- Health/damage stages (uses the chassis stage grammar).
- Function hooks (e.g. provides `jet_thrust`, `shield_pool`, `sensor_range`, `repair_rate`).
- Mod-author metadata (display name, icon, source provenance).

Examples:
- Jet pack module → provides thrust; can fail to `module-warning` (sputtering) → `module-failed` (no thrust).
- Shield emitter → provides shield_pool; can fail with overheat → cooldown.
- Targeting computer → provides aim assist; can fail with EMP.
- Repair drone → provides slow self-repair; can be destroyed independently.
- Sensor pod → provides AI/UX vision; degraded by smoke.

## Command Core Embedding

The command core from [[spec/command-core-base-power]] can be embedded into an eligible chassis as an avatar state. This is not a normal module slot and not a passive stat bonus.

| Requirement | Meaning |
|---|---|
| Explicit compatibility | A body/chassis declares whether it can accept the command core, whether it needs a cockpit/socket, and whether embedding is safe, risky, or impossible. |
| Base tradeoff | Embedding the core must report what base systems lose power or drop to reserve mode. |
| Avatar bonuses | Armor, health, mobility, energy, equipment power, abilities, and command/control aura are declared as readable boost records. |
| Damage risk | Core integrity can be damaged separately from the host chassis; losing the avatar can lose the run/campaign. |
| Extraction | Removing the core from the avatar is an explicit action with time, risk, and UI/replay events. |

This keeps "planting the core into a unit" from becoming invisible RPG math. The player should understand exactly what got stronger, what the base lost, and what failure would cost.

## Origins / Races

Origins are actor families with shared grammar but distinct defaults. They are not balance-skinned humans; they have different chassis baselines, damage profiles, AI doctrines, healing affordances, resource models, environment resistance, and HUD feedback.

> [!important] Per-origin reaction/resource contract lives in its own page
> The detailed branch matrix (force-feedback content, G-load susceptibility, concussion vs internal-shock, fall damage, limb wounds vs module failure, bleed vs coolant/oil leak, healing affordances, overclock vs downclock, resource model, environment resistance, affliction extensions, ORIGIN-A acceptance tests) is locked in [[spec/origin-reaction-and-resource-model]]. This page owns the grammar; that page owns the per-origin behavior table that M5 / M5.5 / M5.7 / proposed M5.8 implement.

| Origin Class | Primary Differences | Notes |
|---|---|---|
| Organic / human | Standard wound model, can be revived/treated, fatigue, morale. Highest G-Force / concussion susceptibility, highest fall damage tolerance ceiling lowest, full bleed model, eats food, uses medkits + drugs, caloric energy resource. Needs sealed helmet + oxygen tank in vacuum. | Baseline. See [[spec/origin-reaction-and-resource-model#Origin Reaction Matrix]]. |
| Powered organic | Organic with cybernetic enhancements; mixed wound + module model. | Treat as bridge — inherits human reaction defaults plus per-cybernetic-module overclock / heat per [[spec/origin-reaction-and-resource-model#Robot-Specific: Internal Shock + Leaks + Overclock]]. |
| Synthetic / android | Wound model on organic side, module/circuit damage on synthetic side; vulnerable to EMP; reduced bleed; reduced G-load; per-installed-module overclock. Some variants ship with batteries; battery depletion → slowdown → ability lockout. Eats food, uses medkits + drugs (organic side). Needs sealed helmet + oxygen tank in vacuum (default; sealed-android variants are an open question). | First-class, not a re-skin. See [[spec/origin-reaction-and-resource-model#Origin Reaction Matrix]] and [[spec/origin-reaction-and-resource-model#Environment Resistance Matrix]]. |
| Synthetic / robot | NO organic wounds, NO bleed, NO concussion, NO G-load. Internal-shock damage to modules instead. Coolant + oil leak channels. Whole-processor overclock (voluntary boost) AND involuntary downclock under sustained heat. `power` resource gates every action. Vacuum-immune; heat-tolerant but downclocks under heat; cannot eat food / use medkits / take drugs. Repaired via repair tools, coolant/oil refills, module swaps. | First-class, not a re-skin. See [[spec/origin-reaction-and-resource-model#Origin Reaction Matrix]] and [[spec/origin-reaction-and-resource-model#Heat Tolerance — Robot Downclock vs Overclock]]. |
| Construct / drone | Pilot-less; controlled remotely; bandwidth-limited; can disconnect. | Different doctrine: sacrificeable. Inherits robot reaction defaults plus disconnect-on-bandwidth-loss behavior. |
| Heavy biomech / fused | Chassis grown rather than built; self-repair; weak to specific energy types. | Future pulp-sci-fi flavor. Inherits hybrid organic/synthetic posture per [[spec/origin-reaction-and-resource-model]] open questions. |

Number of origins at launch is open. The grammar is fixed.

## Required Events (For Replay/Debug/AI/UX)

Every chassis-related event uses the standard envelope from [[references/prototype-run-bundle-schema]] plus chassis-specific payload:

| Event Type | Payload Highlights | Consumers |
|---|---|---|
| `chassis_stage_changed` | actor_id, chassis_id, layer, old_stage, new_stage, cause_event_id, reason_label. | HUD, AI, replay, debrief. |
| `module_state_changed` | actor_id, module_id, slot_id, old_state, new_state, cause_event_id. | HUD, AI, replay. |
| `armor_layer_damaged` | actor_id, layer_id, zone, hit_event_id, integrity_remaining. | Replay, BODY-A tests. |
| `weapon_jammed` / `weapon_cleared` | actor_id, weapon_id, cause, ms_to_clear. | AI, HUD, replay. |
| `pilot_state_changed` | actor_id, pilot_id, old_state, new_state, chassis_state_at_event. | Replay, debrief. |
| `pilot_ejected` / `pilot_extracted` / `pilot_lost` | actor_id, pilot_id, success, cause. | Mission, debrief, retention. |
| `chassis_repaired` | actor_id, layer/module, repaired_by_actor_id, ms_to_repair. | Replay, AI. |
| `chassis_salvaged` | actor_id, recovered_modules, recovered_equipment, recovered_by. | Mission, debrief, retention. |

## AI Contract

Every AI doctrine that can pilot a chassis must implement (or explicitly opt out of):

| Behavior | Required Reason Label |
|---|---|
| `bail_chassis` | "armor critical, no repair available" / "module-failed: mobility" / "weapon disabled" / "pilot injured" |
| `request_repair` | "module-failed: shield, repair drone in range" |
| `swap_module` | "module-failed: sensor, swap to spare available" |
| `clear_jam` | "weapon jammed, distance > X" |
| `evade_layer` | "armor cracked on left side, rotate right" |
| `self_destruct` | doctrine-specific; only for some origins |

The point: when an actor bails or dies, the replay must explain the chain in human language.

## UX Contract

The HUD must surface chassis state without becoming noise.

| Surface | When |
|---|---|
| Body silhouette with armor zones | Always for selected actor; on hover for squad list. |
| Module strip (4-6 icons with stage colors) | Always for selected actor. |
| Pilot health pip | Distinct from chassis stage. |
| Stage banner ("ARMOR CRACKED LEFT", "JET FAILED", "EJECT NOW") | On stage transition; auto-fade. |
| Repair affordance icon | When a repairable module/layer is in range of a repair tool/drone. |
| Salvage marker on wrecks | Once chassis enters `wreck` stage. |

See [[systems/ux-overlay-screen-brief]] and [[spec/ux-wireframes-slice-a]].

## Modding Contract

Mod authors must be able to:

- Add a new origin/race with chassis defaults.
- Add a new chassis class (light mech, heavy mech, exo, drone, etc.).
- Add a new module with state grammar.
- Override damage stages for an existing chassis.
- Provide AI doctrine overrides for a new origin/race.
- Provide HUD assets for new chassis layers/zones.

Schema is data-first per [[decisions/dr-006-modding-data-model]] and [[spec/modding-model]]. Lua escape hatches for stage logic are allowed but the schema covers the common case.

## First Playable Implication

Slice A must include at least one chassis-bearing actor (powered armor or light mech) so that:

- The damage stage grammar is exercised end-to-end.
- The HUD silhouette + module strip wireframe is real.
- The replay recorder captures `chassis_stage_changed`, `module_state_changed`, `armor_layer_damaged`, `weapon_jammed`, and at least one `pilot_*` event.
- One AI doctrine demonstrates a `bail_chassis` decision with reason label.

Without this in Slice A, the body damage / replay / AI / equipment claims in [[spec/authoritative-game-spec-v0]] are paper.

## Open Questions Specific To Chassis

| Question | Status |
|---|---|
| How many origins/races at launch? | Open. Suggested 2-3 to prove grammar; expand via mods. |
| Mech scale range — exo to heavy mech? | Open. Suggested launch range: powered armor + light mech. Heavy mech as moonshot or post-launch. |
| Do mechs scale beyond actor-driven control to vehicle-driven? | Open. Affects camera and control feel. |
| Should chassis own its own inventory or share the pilot's? | Open. Affects UX and modding. |
| What's the salvage economy weight at launch? | Open. Tied to retention loop ([[decisions/dr-011-progression-retention-loop]]). |
| Self-destruct doctrine — universal, doctrine-specific, banned? | Open. Tone implication. |

## Source Trail

- [[decisions/dr-014-tone-player-promise]]
- [[decisions/dr-003-body-damage-readability]]
- [[decisions/dr-006-modding-data-model]]
- [[decisions/dr-008-ai-architecture]]
- [[spec/body-damage-model]]
- [[spec/origin-reaction-and-resource-model]]
- [[spec/equipment-loadout]]
- [[systems/ux-overlay-screen-brief]]
- [[spec/ux-wireframes-slice-a]]
- [[spec/replay-recorder-slice-a]]
- [[spec/missions-and-objectives]]
- [[spec/modding-model]]
- [[systems/replay-event-architecture]]
- [[systems/damage-equipment-and-items]]
- [[references/prototype-run-bundle-schema]]
- [[research-log/2026-05-06-origin-reaction-and-resource-design-intent]]
