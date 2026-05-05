---
type: spec
status: prototype-reqs
ready_when: "DR-003 closes; HUD-01..HUD-03 and BODY-A-01..BODY-A-12 pass against actor-feel, replay, equipment, AI, and UX prototypes."
feeds:
  - DR-002
  - DR-003
  - DR-004
  - DR-008
  - DR-009
  - DR-011
---

← [[spec/index|spec section]] · [[engine/body-damage-wound-gib-lifecycle|body damage lifecycle]] · [[engine/projectile-to-impact-lifecycle|projectile impact]] · [[systems/damage-equipment-and-items|damage/equipment primer]] · [[spec/equipment-loadout|equipment model]] · [[spec/replay-recorder-slice-a|replay recorder]] · [[spec/ux-wireframes-slice-a|UX wireframes]] · [[spec/ai-trust-harness-slice-a|AI harness]] · [[references/prototype-run-bundle-schema|run-bundle schema]] · [[decisions/dr-003-body-damage-readability|DR-003]]

# Body / Damage Model

> [!summary] Purpose
> Define the body, wound, gib, status, equipment fallout, AI, replay, and HUD contract for the future game. This page promotes the old stub into prototype requirements. It is not a final settled spec until DR-003 and BODY-A/HUD tests have real run evidence.

> [!important] Product stance
> Body damage should be brutal, physical, and story-rich without becoming a hidden medical spreadsheet. Players should understand "my arm is gone, my rifle dropped, my bot is unstable, my medic can still save this" in one glance. The model exists to create tactical consequences, readable rescues, death recaps, veteran memories, AI decisions, modding hooks, and replay/debug evidence.

## Slice A Question

Can a player, bot, replay viewer, and loadout/workbench UI all explain one actor's damage chain from hit -> wound -> limb/status consequence -> dropped equipment -> death/rescue outcome without needing to read engine logs?

## Evidence Stack

### Local Cortex / CCCP Evidence

| Evidence | Local Path | Design Lesson |
|---|---|---|
| Wounds, gibs, damage multipliers, mission-critical flag, impulse/wound limits, screen shake, alarm loudness, and gib settings are data fields on `MOSRotating`. | `Source/Entities/MOSRotating.cpp:301-330` | Body damage is authored content, not only code. Future wounds and gibs should stay data-driven for mods and tuning. |
| Wound counts and gib wound limits recurse through attachables depending on positive/negative/zero damage multipliers. | `Source/Entities/MOSRotating.cpp:378-408` | Limbs and attached parts are damage routers. The spec needs part-level state, not only actor HP. |
| Impulse can detach/gib the nearest suitable attachable before root destruction. | `Source/Entities/MOSRotating.cpp:410-445` | Knockback and blast should create visible part failure: arm loss, leg loss, device drop, or armor break. |
| `AddWoundExt` attaches wound emitters, checks gib limits, can detach a nearby attachable before gibbing, marks entry/exit wound sound behavior, and forces wound damage multiplier defaults. | `Source/Entities/MOSRotating.cpp:447-488` | Entry/exit wounds should remain eventful objects with parent offsets, audio/visual emission, replay labels, and treatment hooks. |
| `GibThis` refuses to gib mission-critical bodies, applies counterforce, spawns authored gibs, removes attachables, plays sound/effect, registers alarm, and marks object for deletion. | `Source/Entities/MOSRotating.cpp:883-913` | Mission-critical blocks are valid but must emit a warning/debug event so mods and designers understand why a part did not break. |
| Gib spawning clones authored particles with count, velocity, spread mode, life variation, inherited velocity/angular velocity, and team-hit ignore. | `Source/Entities/MOSRotating.cpp:915-1080` | Gibs are physical gameplay/debris/replay objects. They are not just gore sprites. |
| Actors collect wound damage and attachable damage into health, then run STABLE/UNSTABLE recovery and travel-impulse damage. | `Source/Entities/Actor.cpp:1170-1213` | Stability deserves first-class UI, AI, and replay state because falls/blasts can kill even after the projectile impact. |
| DYING/DEAD actors drop inventory and carried gold; entering DYING drops inventory and starts a death timer. | `Source/Entities/Actor.cpp:1215-1245` | Equipment fallout is part of the damage model. Death recaps and AI should include "weapon dropped" and salvage/pickup events. |
| Dropped inventory is physically ejected; passenger actors re-enter the scene as UNSTABLE. | `Source/Entities/Actor.cpp:760-827` | Body damage can create emergent rescue/salvage moments. The spec must preserve physical drops and passenger fallout. |
| Gold drops as `MOPixel` particles with material and randomized velocity. | `Source/Entities/Actor.cpp:829-848` | Economy fallout can share the replay/death event contract instead of being a separate post-mission accounting step. |
| AHuman becomes UNSTABLE when upside down, has distinct STABLE/UNSTABLE/DYING posture behavior, dies instantly without a head, bleeds if no arms and no legs, and detaches held devices while DYING. | `Source/Entities/AHuman.cpp:2541-2648` | Body state should be coupled to pose, control, equipment, and readability. |
| Medikit scripts remove wounds, heal health, and can revive a dead actor clone when dead actor health/wound state is fully restored. | `Data/Base.rte/Devices/Special/Medikit/Medikit.lua` | Treatment is already a scriptable item behavior. Future support tools need a formal treatment contract and AI target rules. |
| Boss/keycard content uses `MissionCritical`, high joint strength, and high gib wound limits to keep important objects intact until scripted release. | `Data/Browncoats.rte/Devices/Mission/RefineryKeycard/RefineryKeycard.ini`; `Data/Browncoats.rte/Actors/Infantry/BrowncoatBoss/AI/BrowncoatBossAI.lua` | The model needs authoring-time criticality controls, but the UI/workbench should flag them so designers do not hide invincible rules. |

### Comparable And External Evidence

| Source | Lesson For This Spec |
|---|---|
| Cataclysm: DDA Wounds docs | Wounds can be bodypart-specific data with damage type filters, damage thresholds, pain, healing time, body-part type allow/deny lists, and flags. |
| Cataclysm: DDA Effects docs | Long-running effects can target body parts, stack intensity, show part-specific descriptions, alter limb scores, and emit death events. |
| ACE3 Medical Framework | Raw damage can be intercepted into discrete wound events; damage types can be selection-specific; wound handlers can convert damage into bleeding, pain, limping, fractures, and serializable medical state. |
| /tg/ Station 13 bodypart codedocs | Limbs can be damaged, disabled, wounded, mangled, scarred, dismembered, treated, and used as interaction surfaces before being fully removed. |
| Space Station 14 Body docs | Body complexity can live in parts/organs/components while the core body stays simple and relays events. |
| Space Station 14 Combat design docs | Combat consequences should consume resources, support role teamwork, avoid no-win states, and keep equipment roles distinct. |
| [[spec/equipment-loadout]] and [[references/equipment-ai-behavior-contract]] | Item roles, bot use/refusal, equipment drops, support tools, and replay labels must agree with body state instead of inventing a separate "damage UI" language. |

## Body Model Layers

| Layer | What It Stores | Primary Consumers |
|---|---|---|
| `actor_health` | Core survival value, max health, prior health, death timer. | Simulation, HUD silhouette, death recap, AI triage. |
| `actor_status` | STABLE, UNSTABLE, DYING, DEAD, INACTIVE, plus optional prototype-only KNOCKED_OUT if tested. | Control feel, AI, replay, squad panel, accessibility text. |
| `body_parts` | Head, torso, arms, legs, hands, feet, backpack/jetpack, held-device attach points, optional faction/special parts. | Hit routing, UI silhouette, equipment fallback, modding schema. |
| `attachments` | Joint strength, gib impulse limit, gib wound limit, damage multiplier, mission-critical flag, part ownership. | Physics, limb detachment, equipment drop, workbench diagnostics. |
| `wounds` | Entry/exit emitter, source event, damage channel, part, severity, bleed/pain/stability modifiers, treatment tags. | Simulation, particles/SFX, med/support items, replay/death recap. |
| `stability` | Stable velocity thresholds, recovery timer, travel impulse damage, posture state. | Actor controller, AI rescue/retreat, HUD posture icon. |
| `inventory_fallout` | Dropped weapon/tool/gold/passenger objects, positions, velocities, owner, salvage state. | Replay, AI pickup, economy, loadout workbench, campaign recap. |
| `treatment_state` | Removed wounds, stabilized parts, revives, scars, prosthetics, repair/replace operations. | Medic tools, veteran persistence, progression/retention, replay. |

Design rule: HP remains useful, but it cannot be the only public truth. The readable truth is status + part consequence + item fallout + rescue/treatment possibility.

## Status Vocabulary

| Status | Meaning | Player Feedback | AI Implication | Event |
|---|---|---|---|---|
| `STABLE` | Actor can normally aim, move, use tools, and follow orders. | Normal silhouette and control reticle. | Can fight, dig, carry, rescue, or execute current order. | `actor_status_changed` when entering. |
| `UNSTABLE` | Actor has lost control due to speed, fall, blast, pose, or impact. | Wobble/fall icon near reticle and squad card; short text reason in event tail. | Seek recovery, brace, stop firing explosives, medic may deprioritize unless wounded. | `actor_status_changed`, `body_stability_impulse`. |
| `DYING` | Actor is in death transition and drops gear. | Red silhouette pulse, "weapon dropped", rescue/finish window if design allows. | Squad can rescue/loot/retreat; enemies may finish or ignore depending doctrine. | `actor_status_changed`, `inventory_dropped`, `gold_dropped`. |
| `DEAD` | Actor is terminal debris/body. | Death marker and death recap entry. | No orders; may be salvage/revive target only if tool supports it. | `actor_status_changed`, `actor_death_finalized`. |
| `INACTIVE` | Actor is deliberately disabled by activity/script. | Muted squad card, scenario text if player-visible. | Excluded from combat decisions unless script says otherwise. | `actor_status_changed`. |
| `KNOCKED_OUT` | Prototype-only candidate for non-lethal rescue/arrest/medical play. | Blue/white prone marker and timer. | Medic/rescue behavior can test non-lethal stakes. | `actor_status_changed`; only promote after tests. |

## Damage Channels

| Channel | Local Current Equivalent | Prototype Requirement |
|---|---|---|
| `piercing` | Bullets/round particles create entry/exit wounds and impulse. | Track entry/exit, penetration, part hit, source item, body armor result. |
| `cut` | Not a distinct CCCP public channel, but comparable wound systems separate slash/cut. | Keep as schema channel for melee/blades/saw traps and modded content. |
| `blunt` | Travel impulse, body hit sounds, impact forces, falls. | Emit stability/impulse events and distinguish knockdown from wound count. |
| `explosive` | Blast impulse, gib limits, explosive rounds/devices. | Include danger radius, terrain carve, part detach/gib, dropped equipment, friendly-fire labels. |
| `thermal` | Fire/burn content is less central in CCCP, but needed for future hazards. | Add burn wounds/effects only when HUD can show persistent hazard and treatment state. |
| `electric_emp` | Useful for robotics/devices and future faction roles. | Start as device/bodypart disable channel; do not promise without equipment/AI tests. |
| `chemical_bio` | Future poison/acid/stim/bleed modifiers. | Treat as effect stack with bodypart or actor target; keep visible in advanced panel. |
| `terrain_crush` | Dropship/body collision, fall, unstable impact, terrain/object physics. | Emit causality from terrain/object/contact so death recap can say what killed the actor. |

## Body Part Contract

Every controllable actor body part should have enough data for simulation, HUD, AI, equipment, replay, modding, and balance to agree on what happened.

| Field | Required For | Notes |
|---|---|---|
| `part_id`, `display_name`, `side`, `parent_part_id` | UI, replay, localization, package diagnostics. | Stable ids must survive replay exports and save/load. |
| `attachable_kind` | Simulation and modding. | `root`, `limb`, `head`, `hand`, `foot`, `jetpack`, `held_device_socket`, `armor_plate`, `prosthetic`, `special`. |
| `damage_multiplier` | Damage routing and armor/prosthetic tuning. | Keep separate from raw health so helmets, shields, armor, and prosthetics can tune consequences. |
| `joint_strength` | Impulse detachment and blast readability. | Used for "why limb detached" and "why armor held" debug text. |
| `gib_wound_limit`, `gib_impulse_limit` | Dismember/gib thresholds. | Workbench should warn about extreme values, especially mission-critical parts. |
| `can_hold_item`, `held_slot_ids` | Equipment fallback and AI. | Arm/hand damage must explain weapon drops, one-arm fallback, and bot refusal. |
| `movement_contribution` | Leg/foot/mobility damage. | Supports limp, crawl, jump/jetpack failure, rescue carry choices. |
| `aim_contribution` | Arm/head/sensor damage. | Supports reticle spread reasons and bot accuracy confidence. |
| `criticality` | Death/mission behavior. | `lethal_if_missing`, `mission_critical`, `revivable`, `prosthetic_replaceable`. |
| `wound_slots` | Wound stacking and treatment. | Avoid unbounded invisible wounds; aggregate minor hits when needed for readability. |
| `treatment_hooks` | Medic tools, workbench, campaign. | Remove wound, stabilize bleed, splint, revive, replace limb, seal leak, repair prosthetic. |

## Wound Contract

| Field | Why It Exists |
|---|---|
| `wound_id`, `wound_type`, `source_event_id` | Replay/death recap can trace causality from projectile/device/terrain to body outcome. |
| `actor_id`, `part_id`, `entry_or_exit` | HUD and body silhouette know where to show the marker. |
| `damage_channel`, `severity`, `damage_amount` | Simulation and UI can separate tiny graze from limb-ending wound. |
| `impulse`, `penetration`, `armor_result` | Explains knockdown, pass-through, ricochet, armor block, and overpenetration. |
| `bleed_rate`, `pain_or_focus`, `stability_penalty` | AI triage, medic priority, and player state feedback. |
| `movement_penalty`, `aim_penalty`, `grip_penalty` | Equipment and control consequences. |
| `treatment_tags` | Medikit/support tools can decide what can be fixed. |
| `visibility_tier` | Default HUD can show severe state while advanced panel holds detail. |
| `ai_tags` | Bots can choose rescue, retreat, switch weapon, crawl, pickup, or refuse explosives. |
| `replay_tags` | Viewer and run-bundle checker can filter hit, wound, detach, gib, treatment, and death. |
| `package_source` | Modded wound types remain traceable to package/version. |

## Consequence Matrix

| Consequence | Simulation Result | UI Result | AI Result | Equipment Result |
|---|---|---|---|---|
| Arm wounded | Grip/aim/reload penalty. | Arm segment marked; reticle reason text. | Lower confidence with two-hand weapons. | May switch to sidearm or drop primary. |
| Arm detached/gibbed | Held device removed or unusable. | Clear missing-arm icon and dropped item marker. | Bot stops using blocked slot; may retreat/pickup. | `inventory_dropped`, slot invalidation, salvage marker. |
| Leg wounded | Stability and mobility penalty. | Limp/crawl icon; path overlay warning. | Bot slows, seeks safe route, calls rescue. | Heavy items become riskier. |
| Leg detached/gibbed | Movement/crawl/jet dependency change. | Missing-leg silhouette; extraction warning. | Rescue/carry priority if mission-critical actor. | Delivery/extraction UI must account for carry burden. |
| Head destroyed | Lethal except special actors. | Death cause: decapitation/head loss. | No rescue unless explicit revive/prosthetic rule. | Drops gear; veteran death/scar record. |
| Torso critical | High bleed/stability/death risk. | Central warning, not full medical chart. | Medic priority increases; retreat order likely. | Heavy equipment may worsen stability. |
| Jetpack/backpack damaged | Mobility/flight/support loss. | Mobility icon and "jetpack disabled" text. | Bot refuses vertical route. | Loadout/workbench sees mobility capability lost. |
| Held device damaged/dropped | Weapon/tool unavailable. | Dropped item marker and slot warning. | AI switches, retrieves, or asks for pickup. | Replay/export records item id and source. |
| Mission-critical gib blocked | Object stays intact despite threshold. | Debug/workbench warning; optional in-game spark/brace feedback. | AI should not assume invulnerability unless role says so. | Package diagnostics warn if overused. |

## UI / UX Rules

| Rule | Requirement |
|---|---|
| Default HUD stays compact. | Use the DR-003 silhouette default: part state, status, and severe consequences only. |
| Advanced view is opt-in. | Detailed wound list, source ids, treatment tags, and exact modifiers live in body panel/replay/workbench, not always-on combat HUD. |
| No color-only meaning. | Each body state needs shape/icon/text fallback per [[spec/ux-wireframes-slice-a]]. |
| Consequences use verbs. | Prefer "left arm dropped rifle" over raw "arm 0 HP". |
| Death recap is causal. | Show source item/terrain -> hit part -> wound/impulse -> status/drop/gib -> final state. |
| Rescue window is explicit. | If DYING or KNOCKED_OUT can be saved, show timer/condition; if not, say why in recap/debug. |
| Equipment state is co-located. | Body silhouette and slot cards share item ids, warnings, dropped state, and bot usability labels. |

## AI Contract

| AI Need | Body Field/Event |
|---|---|
| Know when to stop firing a dangerous weapon. | `actor_status_changed: UNSTABLE`, `wound.stability_penalty`, equipment danger tags. |
| Decide whether to rescue. | `actor_status`, `criticality`, bleed/timer estimate, path safety, support tool availability. |
| Switch weapons after arm/item loss. | `body_part_detached`, `inventory_dropped`, `slot_invalidated`, item `bot_claim_state`. |
| Avoid finishing a downed ally with explosives. | `ai_danger_radius`, actor status, friendly body part/event positions. |
| Use med/support tools. | Wound `treatment_tags`, `severity`, target rules from [[references/equipment-ai-behavior-contract]]. |
| Explain failure. | `ai_item_refusal`, `ai_rescue_refusal`, `body_damage_reason_label`. |

Minimum reason labels:

| Label | Example Use |
|---|---|
| `no_usable_arm` | Bot cannot fire selected two-hand weapon. |
| `unstable_no_explosive` | Bot refuses launcher while tumbling/falling. |
| `leg_loss_route_invalid` | Path requires jump/climb/walk actor can no longer perform. |
| `wound_needs_support_tool` | Medic/support item required; none in squad. |
| `friendly_in_blast_radius` | Bot refuses explosive because downed ally is too close. |
| `mission_critical_cannot_gib` | Script/workbench warning for invulnerable body/key asset behavior. |

## Replay / Run Evidence Events

Every BODY-A prototype run should export these event families through [[spec/replay-recorder-slice-a]] and validate them through [[references/prototype-run-bundle-schema]].

| Event | Required Fields |
|---|---|
| `body_hit` | event id, frame/time, actor id, part id, source item/projectile/terrain id, hit position, impulse, damage channel. |
| `body_wound_added` | wound id, parent hit id, actor id, part id, entry/exit, severity, channel, visible tier. |
| `body_part_detached` | actor id, part id, cause event id, joint/gib threshold, resulting object id. |
| `body_gib_spawned` | actor/part id, cause event id, gib preset id/count, velocity class, team-hit policy. |
| `body_stability_impulse` | actor id, impulse, threshold, damage, previous/new status. |
| `actor_status_changed` | actor id, old/new status, cause event id, rescue window if any. |
| `inventory_dropped` | actor id, item id, slot, position, velocity, cause event id. |
| `gold_dropped` | actor id, amount/pixel count, position, cause event id. |
| `treatment_applied` | support item id, target actor/part, wound ids removed/changed, result, failure reason. |
| `death_recap_ready` | actor id, final cause chain, item drops, salvage/veteran consequence, replay marker. |
| `mission_critical_gib_blocked` | object/part id, attempted cause, authoring source, workbench warning id. |

## Acceptance Tests

| Test | Prototype Setup | Pass Condition |
|---|---|---|
| BODY-A-01 | Rifle shot into arm. | Wound marker, part id, source item, entry/exit, and reticle/slot consequence are visible in HUD/replay. |
| BODY-A-02 | Arm detach/gib while holding primary. | Weapon drops with item id; actor slot invalidates; bot switches/refuses with reason label. |
| BODY-A-03 | Leg wound + blast knockdown. | Actor enters UNSTABLE, mobility warning appears, replay records impulse and status change. |
| BODY-A-04 | No-head death. | Death recap says head loss/decapitation; actor status reaches DEAD; inventory/gold fallout is recorded. |
| BODY-A-05 | No-arms/no-legs bleed case. | Bleed timer/consequence is visible; replay names the limb-loss chain. |
| BODY-A-06 | Gib threshold hit. | Wound count/gib impulse threshold is recorded; gibs spawn with replay-visible preset/count class. |
| BODY-A-07 | Mission-critical gib blocked. | Object remains, but debug/workbench/replay event explains `mission_critical_gib_blocked`. |
| BODY-A-08 | Medikit/support treatment. | Treatment event removes/changes wound state; HUD, AI target list, and recap update. |
| BODY-A-09 | Downed ally near explosive. | Bot refuses dangerous use with `friendly_in_blast_radius` or equivalent label. |
| BODY-A-10 | Dropped equipment pickup. | Dropped weapon marker can be picked up by player/bot; event chain preserves original owner/cause. |
| BODY-A-11 | Death recap scrub. | Replay viewer can scrub body_hit -> wound -> detach/gib/drop -> death without missing parents. |
| BODY-A-12 | Accessibility pass. | Body state is readable without color, with compact labels and controller/keyboard focus in the advanced panel. |

## First Implementation Tickets

| Ticket | Output |
|---|---|
| BODY-001 | Add body damage event schema extension to recorder prototype. |
| BODY-002 | Implement silhouette fixture with head/torso/arms/legs/status + text fallback. |
| BODY-003 | Add dropped equipment/gold event rows to death recap. |
| BODY-004 | Connect item slot invalidation to [[spec/equipment-loadout]] and [[spec/equipment-loadout-workbench-slice-a]]. |
| BODY-005 | Add AI reason labels for unstable, no usable arm, leg route invalid, and friendly blast refusal. |
| BODY-006 | Add med/support treatment fixture using medikit-like remove-wound behavior. |
| BODY-007 | Add mission-critical warning fixture for package/workbench diagnostics. |
| BODY-008 | Add BODY-A fixture run to `prototype_run_check.py` once the prototype emits events. |
| BODY-009 | Add death recap wireframe rows to [[spec/ux-wireframes-slice-a]] if the prototype reveals missing UI states. |
| BODY-010 | Promote proven labels/fields into a future body-damage JSON schema after BODY-A evidence exists. |

## Open Questions

| Question | Cheapest Evidence |
|---|---|
| Should KNOCKED_OUT exist as a separate status or only as UNSTABLE/DYING variants? | Add one non-lethal fixture and test whether players understand it. |
| How much body detail is too much during direct control? | Run HUD-01..HUD-03 with compact silhouette vs advanced panel. |
| Should gibs remain physical gameplay objects forever? | Measure clutter/perf and replay readability in Slice A. |
| Which wounds persist into campaign/veteran progression? | Tie BODY-A treatment results to [[spec/progression-retention]] veteran/scar fields. |
| How much body part data should modders author manually? | Let package-builder prototype flag missing fields and observe which warnings matter. |
| Can bots safely use revive/med tools without feeling magical? | Add AI-H rescue/treatment scenario with replay-visible target reasoning. |

## Source Trail

### Local

- [[engine/body-damage-wound-gib-lifecycle]]
- [[engine/projectile-to-impact-lifecycle]]
- [[systems/damage-equipment-and-items]]
- [[spec/equipment-loadout]]
- [[spec/ux-wireframes-slice-a]]
- [[spec/replay-recorder-slice-a]]
- `../Cortex-Command-Community-Project/Source/Entities/MOSRotating.cpp:301-330`
- `../Cortex-Command-Community-Project/Source/Entities/MOSRotating.cpp:378-488`
- `../Cortex-Command-Community-Project/Source/Entities/MOSRotating.cpp:883-1080`
- `../Cortex-Command-Community-Project/Source/Entities/Actor.cpp:760-848`
- `../Cortex-Command-Community-Project/Source/Entities/Actor.cpp:1170-1245`
- `../Cortex-Command-Community-Project/Source/Entities/AHuman.cpp:2541-2648`
- `../Cortex-Command-Community-Project/Data/Base.rte/Devices/Special/Medikit/Medikit.lua`
- `../Cortex-Command-Community-Project/Data/Browncoats.rte/Devices/Mission/RefineryKeycard/RefineryKeycard.ini`
- `../Cortex-Command-Community-Project/Data/Browncoats.rte/Actors/Infantry/BrowncoatBoss/AI/BrowncoatBossAI.lua`

### Public Web

- Cataclysm: DDA Wounds docs: `https://docs.cataclysmdda.org/JSON/WOUNDS.html`
- Cataclysm: DDA Effects docs: `https://docs.cataclysmdda.org/JSON/EFFECTS_JSON.html`
- ACE3 Medical Framework: `https://ace3.acemod.org/wiki/framework/medical-framework.html`
- /tg/ Station 13 bodypart codedocs: `https://codedocs.tgstation13.org/obj/item/bodypart.html`
- Space Station 14 Body docs: `https://docs.spacestation14.com/en/space-station-14/core-tech/body.html`
- Space Station 14 Combat docs: `https://docs.spacestation14.com/en/space-station-14/combat.html`

## Change Log

- 2026-05-04: Promoted from stub to prototype requirements using CCCP body/gib/inventory code, durable body/wound references, and integration requirements for HUD, AI, replay, equipment, and treatment.
