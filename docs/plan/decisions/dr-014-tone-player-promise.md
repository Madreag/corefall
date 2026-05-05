---
type: decision
id: DR-014
status: closed-direction
priority: P0
closed_at: 2026-05-04
revisit_trigger: "Playtests reveal the chosen tone is alienating the target player or fails to support a system the spec depends on (e.g. mech damage feels gimmicky instead of tactical)."
---

← [[decisions/index|decision records]] · [[dashboards/decision-tracker|decision tracker]] · [[spec/authoritative-game-spec-v0|game spec v0]] · [[spec/product-promise|product promise]] · [[spec/chassis-armor-mechs-and-origins|chassis/armor/mechs spec]]

# DR-014: Tone And Player Promise

> [!success] Status: CLOSED-DIRECTION (project owner committed)
> The tone and player promise are committed for the v0 planning anchor. Specific feature implementations remain open.

## Decision

**Tone is "Tactical pulp sci-fi disaster sandbox."**

Gritty tactical stakes, pulpy systemic consequences, surreal sci-fi accents, strong sandbox/workbench support. Players command fragile squads through destructible tactical disasters, save who they can, lose people and machines in memorable ways, learn from the replay, and return with a better plan.

**Excluded tones:**

- Pure comedy (Cortex's silliest mods).
- Pure X-COM grimness.
- Pure Noita / Rain World opacity.
- Pure Powder Toy sandbox.

The game blends these flavors; it does not commit to any single one.

## Required Fantasy Elements

These are first-class product elements, not flavor decoration:

| Element | Required Behavior |
|---|---|
| Mechs | Physical chassis with mass, mobility profile, weapon mounts, internal modules, pilot. Not a stat-boost suit. |
| Powered armor | Lighter chassis-on-actor system; same chassis grammar as mechs. |
| Armor layers | Multi-layer protection model (helmet, vest, plate, undersuit, etc.). Each layer can be damaged independently. |
| Robots / androids | First-class actor type with chassis grammar; behave differently under damage and EMP than organic actors. |
| Origins / races | Distinct actor families with different baseline chassis, weakness/resistance profile, and AI doctrines. Organic, synthetic, and hybrid. |
| Damageable equipment | Held weapons, tools, and modules can jam, overheat, lose components, or be destroyed. Not just "ammo runs out". |
| Staged machine/body damage | Damage progresses through readable stages (smoke, sparks, jams, disabled modules, system failure, gib/explosion, pilot wound, pilot eject, dead chassis). Each stage emits replay-grade events. |
| Pilot rescue / ejection | Pilots/operators can survive a chassis loss. Eject, crawl out, get carried. Their fate is its own story beat. |
| Repair / salvage | Damaged chassis and equipment can be repaired in field or salvaged from the battlefield. Not a magic "repair to full". |
| AI reason labels | Every chassis-related AI decision (eject, retreat, bail, repair, swap, suppress) emits a reason string for replay/debug/UX. |
| Replay/debrief cause chains | "Why did Lieutenant Hernandez die" must trace from final cause back through chassis stage transitions, equipment failures, and AI decisions. |

## What This Locks In

| Spec Area | Implication |
|---|---|
| Body damage model | Must support multi-layer armor + chassis modules + pilot/operator separation, not just wound emitters on a body. See [[spec/body-damage-model]] and [[spec/chassis-armor-mechs-and-origins]]. |
| Equipment | Items and tools must have damage state, jam/overheat/destroyed transitions, and field repair affordance. See [[spec/equipment-loadout]]. |
| Replay / event | Every chassis stage transition, equipment failure, AI bail decision, eject, and salvage event must be in the event taxonomy. See [[systems/replay-event-architecture]] and [[spec/replay-recorder-slice-a]]. |
| AI | Bots understand their chassis state, know when to eject/retreat, and explain it. See [[decisions/dr-008-ai-architecture]] and [[spec/ai-trust-harness-slice-a]]. |
| UX | Chassis HUD must show stage, modules, pilot state, repair affordance. See [[systems/ux-overlay-screen-brief]] and [[spec/ux-wireframes-slice-a]]. |
| Mission design | Missions reward repair/salvage/extract behaviors, not just kill/destroy. See [[spec/missions-and-objectives]]. |
| Modding | Origins/races and chassis classes are first-class mod surfaces. See [[spec/modding-model]]. |
| Visual / audio | Smoke, sparks, alarms, hydraulic whine, servo failure are part of the diegetic feedback layer. New audio-identity DR (TBD). |

## What This Explicitly Does Not Do

| Non-Goal | Why |
|---|---|
| Lock genre to "mech game". | Mechs are a fantasy element, not the only or even primary actor type. Foot infantry remains core. |
| Lock to a specific tech tier. | Origins span organic, synthetic, hybrid, primitive, advanced. The mix is part of the world. |
| Promise a specific number of origins/races at launch. | Quantity decided by content cost vs prototype-mission needs. |
| Promise mech customization depth. | Mod-friendly, but launch customization scope is open. |

## Evidence Trail

- Verbatim project-owner statement (2026-05-04 spec-questioning round): "Tactical pulp sci-fi disaster sandbox… mechs, powered armor, armor layers, robots/androids, different origins/races, damageable equipment, and staged machine/body damage are part of the fantasy. Armor and mechs should not be passive stat boosts."
- Captured in [[research-log/2026-05-04-spec-tone-and-scope-decisions]].
- Linked into [[spec/authoritative-game-spec-v0]], [[spec/product-promise]], [[spec/body-damage-model]], [[spec/equipment-loadout]], [[spec/chassis-armor-mechs-and-origins]], [[systems/ux-ui-and-retention]].

## Revisit Trigger

- A playtest shows the chosen tone is alienating the target player.
- Mech/armor/chassis depth proves to be content-cost-prohibitive at the chosen scope.
- A future origin/race/chassis design conflicts with the AI/UX/replay obligations above.
