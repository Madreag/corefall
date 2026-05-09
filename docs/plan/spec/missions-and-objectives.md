---
type: spec
status: prototype-reqs
ready_when: "Mission Director Slice A proof mission passes MISSION-A-01..18 with recorder, commander, equipment-capability, and destruction-aware objective evidence."
---

← [[spec/index|spec section]] · [[spec/mission-director-slice-a|mission director Slice A]] · [[systems/destruction-objective-mission-patterns|destruction-objective patterns]] · [[engine/activity-scenario-lifecycle|activity lifecycle]] · [[spec/equipment-loadout|equipment/loadout]]

# Missions And Objectives

> [!summary] Purpose
> This page is the high-level missions/objectives spec index. The build-facing requirements are now in [[spec/mission-director-slice-a]], which defines the typed manifest, director pacing, commander AI, objective grammar, equipment capability contract, save/replay fields, and MISSION-A acceptance tests.

## Current Shape

| Layer | Canonical Note | Status | What It Provides |
|---|---|---|---|
| Activity/scenario code archaeology | [[engine/activity-scenario-lifecycle]] | DONE + MetaFight save finding | CCCP lifecycle, Lua hooks, save primitives, `MetaFight`/`BunkerBreach`/helper notes. |
| Destruction-aware design patterns | [[systems/destruction-objective-mission-patterns]] | DONE | Breach/defend/extract/sabotage/survival pattern catalog and anti-patterns. |
| Build-facing proof mission contract | [[spec/mission-director-slice-a]] | PROTOTYPE REQS | Typed manifest, director/commander model, objective grammar, capability requirements, save/replay, UI/workbench obligations, MISSION-A tests. |
| Equipment capability bridge | [[spec/equipment-loadout]] | READY TO BUILD | Mission capability tags, loadout slots, item role cards, AI item metadata, package diagnostics, and workbench traceability. |
| Recorder dependency | [[spec/replay-recorder-slice-a]] | READY TO BUILD | Event envelope and run-evidence contract for mission events. |
| AI dependency | [[spec/ai-trust-harness-slice-a]] | READY TO BUILD | Scenario harness, AI event contract, reports, and debug overlay fields. |

## Mission Principles

| Principle | Requirement |
|---|---|
| Missions are contracts. | Objectives, teams, terrain rules, director state, equipment needs, save fields, UI markers, and replay events live in a typed manifest. |
| Destruction is allowed by default. | Do not depend on ordinary walls as locks; use defended spaces, distance, elevation, water, alarms, timers, resource pressure, and critical-object policy. |
| Commander AI must explain itself. | Target, LZ, package, squad, breach, defend, and recovery decisions emit reason strings and structured event fields. |
| Equipment requirements are capabilities. | A mission asks for `breach`, `dig`, `heal`, `fight`, `carry`, or `repair` capability tags, not hard-coded item names. |
| UI and replay share truth. | Objective markers, phase text, debrief causes, and replay events use the same objective ids and event ids. |
| Private experiments stay unblocked. | Missing capability warnings can be overridden for private play, but the consequence must be visible. |

## Proof Mission Target

The first proof mission is a compact "Breach Contract" modeled after CCCP `BunkerBreach.lua` but with a typed manifest:

| Component | Requirement |
|---|---|
| Scene | `LZ Attacker`, `Main Bunker`, `Brain`, optional `Internal Reinforcements`, optional salvage cache, two breach paths. |
| Player | One commander/brain plus one or two actors. |
| Defender | Protected brain, guard squad, LZ or internal-door reinforcement response. |
| Objectives | Destroy/protect brain, open breach route, optional salvage, optional extraction/debrief. |
| Director | Setup, prep, launch, build-up, peak/fade/relax, objective push, emergency, debrief. |
| Equipment | Mission strip checks required/recommended/missing/dangerous/manual-only capabilities using [[references/equipment-device-loadout-field-atlas]] and [[references/equipment-role-cards-slice-a]]. |
| Evidence | MISSION-A-01..18 plus [[references/prototype-run-bundle-schema]] output. |

## Inputs

- [[spec/mission-director-slice-a]]
- [[systems/destruction-objective-mission-patterns]]
- [[engine/activity-scenario-lifecycle]]
- [[engine/ai-pathfinding-activities]]
- [[spec/equipment-loadout]]
- [[references/equipment-device-loadout-field-atlas]]
- [[spec/replay-recorder-slice-a]]
- [[spec/ai-trust-harness-slice-a]]
- [[decisions/dr-007-terrain-material-model]]
