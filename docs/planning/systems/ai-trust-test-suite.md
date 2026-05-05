← [[systems/ai-and-bots|AI and bots]] · [[spec/ai-trust-harness-slice-a|AI harness Slice A]] · [[engine/ai-order-lifecycle|AI order lifecycle]] · [[dashboards/research-readiness|readiness]]

# AI Trust Test Suite

> [!danger] Product-critical
> The future game should assume many players will play solo. Bots must be trustworthy enough that players want to command squads, not babysit them.

## Test Philosophy

| Principle | Meaning |
|---|---|
| Test visible behavior, not just internal success. | A bot that succeeds invisibly still feels dumb if the player cannot read intent. |
| Use destructible terrain in every serious test. | Flat-ground AI does not prove Cortex-like AI. |
| Include friendly and enemy AI. | Friendly trust and enemy challenge fail differently. |
| Record replays and reason labels. | Designers need to see why a bot chose a path, weapon, target, or retreat. |
| Measure recovery. | Bots will fail; the question is whether they recover gracefully. |

## Current Implementation Target

| Artifact | Status | What It Adds |
|---|---|---|
| [[spec/ai-trust-harness-slice-a]] | <span class="cc-flag cc-blue">READY TO BUILD</span> | Scenario manifest, event contract, local AI hook map, AI-H bootstrap scenarios, report format, overlay fields, acceptance tests, and first implementation tickets for turning AI-01..AI-12 into runnable checks. |
| [[references/equipment-ai-behavior-contract]] | <span class="cc-flag cc-blue">CONTRACT</span> | Bot item-choice/refusal fields, reason labels, scenario mapping, and consumer obligations for AI, UI, workbench/package, balance, replay, and backend. |
| [[references/equipment-ai-scenarios-slice-a]] | <span class="cc-flag cc-blue">SEED</span> | Nine equipment-specific AI-H-LOAD scenarios for weapon choice, breach tool choice, support use, explosive refusal, target-class reasoning, pickup value, and unsafe-loadout blocks. |

## Core Scenarios

| ID | Scenario | Setup | Pass Criteria | Debug Data |
|---|---|---|---|---|
| AI-01 | Defend brain room | Friendly squad, bunker, known entrances, enemy waves. | Bots hold useful positions, react to alarms, do not abandon brain without order. | Intent, target, alarm source, cover position. |
| AI-02 | Breach door | Enemy behind door, mixed tools/guns/grenades. | Bot selects breaching tool or penetrating weapon, avoids wasting ammo forever. | Door material, chosen tool, penetration estimate. |
| AI-03 | Dig to objective | Gold or waypoint behind terrain. | Bot equips digging tool, makes viable tunnel, avoids trapping self. | Path cost, dig strength, terrain edit boxes. |
| AI-04 | Recover from collapsed tunnel | Explosion blocks current path. | Bot detects stale/blocked path and replans/digs/retreats within deadline. | Path age, dirty regions, stuck timer. |
| AI-05 | Suppress alarm | Enemy fires offscreen. | Bot turns/investigates/suppresses only when plausible, does not chase every noise. | Alarm loudness, occlusion ray, confidence. |
| AI-06 | Rescue downed/veteran unit | Injured friendly with enemies nearby. | Bot either extracts, covers, or marks impossible instead of idling. | Role, threat score, rescue decision. |
| AI-07 | Avoid dropship crush | Delivery craft enters hot zone. | Bots avoid obvious landing path when warned. | Landing marker, local avoidance state. |
| AI-08 | Hold formation through bunker | 3-5 bots, doors, ladders/terrain, narrow passages. | Squad avoids permanent clumping and keeps useful spacing. | Slot assignment, blocker ID, yield decisions. |
| AI-09 | Use medikit | Hurt bot with medikit and no immediate target. | Bot heals when safe, interrupts if threatened. | Health threshold, current threat, equipped item. |
| AI-10 | Weapon pickup | Unarmed bot near weapon under fire. | Bot seeks nearby weapon if better than current behavior. | Weapon value, distance, risk score. |
| AI-11 | Retreat from unwinnable breach | Bot cannot damage door/terrain and no alternate path. | Bot reports impossible/requests tool instead of looping. | Failed action count, missing capability. |
| AI-12 | Enemy flanking through new hole | Terrain explosion opens route. | Enemy can use new route after path update deadline. | Path update latency, dirty area, chosen route. |

## Metrics

| Metric | Target Direction |
|---|---|
| Time to first useful action after order | Lower is better. |
| Time to recover after path invalidation | Lower is better, with bounded max. |
| Percentage of time in unexplained idle | Near zero. |
| Friendly obstruction duration | Lower is better. |
| Wasted shots at unbreakable obstacle | Near zero. |
| Deaths caused by delivery/terrain ignorance | Lower is better. |
| Player order overrides required per minute | Lower is better after learning curve. |
| Correct tool selection rate | Higher is better. |

## Required Debug Overlay

| Overlay | Why |
|---|---|
| Current order and current tactical intent | Explains what the bot thinks it is doing. |
| Target and last known target | Makes perception readable. |
| Path with stale/blocked segments | Shows terrain/pathfinding failures. |
| Tool/weapon score labels | Makes equipment decisions tunable. |
| Item choice/refusal reason | Shows selected item, rejected alternatives, and refusal labels from [[references/equipment-ai-behavior-contract]]. |
| Alarm source and confidence | Prevents invisible "why did they turn?" confusion. |
| Stuck timer and recovery action | Separates temporary navigation from broken AI. |

## Implementation Notes For Future Game

| System | Minimum Contract |
|---|---|
| Orders | Persistent high-level intent with cancellation and priority. |
| Tactics | Utility-scored behavior choices with reason labels. |
| Pathfinding | Incremental updates, stale-path detection, and dynamic obstacle semantics. |
| Inventory | Item capabilities exposed to AI as structured data. |
| Perception | Sight, hearing, memory, and confidence are inspectable. |
| Replay | Deterministic enough to reproduce failures with bot state logs. |

## Source Inspiration

- Cortex `NativeHumanAI.lua` behavior mode selection.
- Cortex `SharedBehaviors.lua` alarm and path behavior.
- Cortex `Controller::ShouldUpdateAIThisFrame` throttling.
- Cortex pathfinding dirty-area update behavior in `Scene::UpdatePathFinding`.
- Rain World ecosystem AI research in [[comparables/noita-powder-toy-teardown-rain-world]].
- [[references/equipment-ai-behavior-contract]] for bot equipment choice/refusal reason labels.
- [[references/equipment-ai-scenarios-slice-a]] for LOAD-A equipment scenario seeds.
