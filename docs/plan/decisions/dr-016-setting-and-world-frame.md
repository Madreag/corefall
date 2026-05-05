---
type: decision
id: DR-016
status: closed-direction
priority: P0
closed_at: 2026-05-04
revisit_trigger: "Playtests show the world frame is too generic to support distinct factions/missions, or pulls focus from the chassis/command-core mechanics."
---

← [[decisions/index|decision records]] · [[dashboards/decision-tracker|decision tracker]] · [[spec/setting-and-world-frame|setting & world spec]] · [[spec/authoritative-game-spec-v0|game spec v0]] · [[decisions/dr-014-tone-player-promise|DR-014 tone]] · [[decisions/dr-015-player-identity-control-posture|DR-015 player identity]]

# DR-016: Setting And World Frame

> [!success] Status: CLOSED-DIRECTION (project owner committed 2026-05-04)
> The world is **frontier disaster-contract sci-fi with a vulnerable command-core anchor**. Specific lore (factions, biomes, named places, antagonists) remains open and can grow via play and mods.

## Decision

**Frontier disaster-contract sci-fi.**

The player runs a persistent merc / rescue / salvage outfit working dangerous contracts across:

- Collapsing frontier colonies.
- Corporate war zones.
- Alien biomes.
- Derelict megastructures.
- Disaster sites.

Missions feel like jobs gone wrong: breach, rescue, recover, salvage, sabotage, stabilize, extract, defend, investigate.

The Cortex "brain in a jar" mechanic is preserved as a **mechanic**, not as the lore. The vulnerable command/identity anchor becomes a **command core / neural anchor / continuity core / operator uplink / company command node** (precise term is open). See [[decisions/dr-015-player-identity-control-posture]] and [[spec/command-core-base-power]].

Bodies are **not** all disposable. Humans, androids, robots, mech pilots, armor sets, and damaged chassis can become named survivors, valuable machines, repair projects, salvage, or legacy assets. **One** faction doctrine may use cheap disposable bodies/drones, but that is not the world's central premise.

## What This Locks In

| Spec Area | Implication |
|---|---|
| Player identity | Continuity commander with optional direct possession; see DR-015 and [[spec/command-core-base-power]]. |
| Tone | Tactical pulp sci-fi disaster sandbox per DR-014 — confirmed compatible. |
| Factions | World supports corporate raiders, colony militias, rescue crews, scavengers, synthetic factions, alien/biological threats, rival commanders. Not just two armies. |
| Missions | Contract structure: jobs gone wrong. Anchor missions and procedural contracts both fit. See DR-017 and [[spec/mission-director-slice-a]]. |
| Body damage | Distinct death meanings per origin (organic, android/robot, mech, clone, command core) per DR-018 and [[spec/body-damage-model]]. |
| Modding | World/lore is open enough that mods can add factions, biomes, alien races, megastructures, and disaster types without breaking canon. |
| Visual / audio | Visual identity must support multiple biomes (colony, corporate facility, alien jungle, derelict, disaster zone) — not a single fixed look. |

## What This Does NOT Lock

| Non-Commitment | Why |
|---|---|
| Specific named factions / leaders / planets | Open. Develop through play and mods. |
| Number of factions at launch | Open. Suggested 2-4 to prove faction grammar. |
| Whether aliens are sentient / hostile / both | Open. Suggested mix. |
| Tech tier ceiling | Open. Mechs and androids exist; FTL/AI-godhood is a separate question. |
| Tone of individual missions | Open. Some can be grim, some pulpy, some weird. |
| Final term for the "brain" mechanic | Open. Working term: command core. |
| Whether the player's outfit is for-profit, idealist, criminal, or unclear | Open. Likely player-configurable. |

## Why Not The Alternatives

| Alternative | Why Rejected |
|---|---|
| Cortex straight clone (brain-in-jar literal) | Limits world breadth, feels derivative, locks one lore that not every player will love. |
| Post-collapse / scavenger only | Too narrow; prevents the chassis/mech/origin grammar from spanning all the contexts DR-014 wants. |
| Hard-fork your own world bible from scratch | Too much upfront writing; would delay actual game work. The "frontier disaster contract" frame is generative without being heavy. |

## Evidence Trail

- Project owner verbatim (2026-05-04 spec round 2): "Frontier disaster-contract sci-fi with a vulnerable command-core anchor… Missions feel like jobs gone wrong: breach, rescue, recover, salvage, sabotage, stabilize, extract, defend, investigate. The 'brain' should become a more flexible original concept: a command core, neural anchor, continuity core, operator uplink, or company command node."
- Captured in [[research-log/2026-05-04-spec-round-2-setting-mission-death]].
- Spec page: [[spec/setting-and-world-frame]].
- Linked into [[spec/authoritative-game-spec-v0]] product promise + launch commitments.

## Revisit Trigger

- Playtests show the world frame is too generic to support distinct factions/missions.
- The "command core" mechanic conflicts with the world frame in a way that requires choosing one.
- A killer setting idea emerges that justifies a hard-fork bible.
