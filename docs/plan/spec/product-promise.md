---
type: spec
status: exploratory
ready_when: "DR-001 and DR-005 close; first-playable slice A is fun; DR-011 RET-A tests show return-loop value."
---

← [[spec/index|spec section]] · [[strategy/best-cortex-like-game-principles|principles]] · [[design/opportunities-for-our-fork|fork opportunities]] · [[decisions/dr-011-progression-retention-loop|DR-011]] · [[spec/progression-retention|progression]]

# Product Promise

> [!warning] Candidate promise, not final marketing copy
> Anchor pages: [[strategy/best-cortex-like-game-principles]] and [[design/opportunities-for-our-fork]]. Decisions gating: [[decisions/dr-001-engine-strategy]], [[decisions/dr-005-multiplayer-posture]], and [[decisions/dr-011-progression-retention-loop]].

## Candidate Product Promise

Build a modern Cortex-like tactical physics sandbox where you directly pilot fragile soldiers, command capable AI squads, tear through destructible terrain, recover from chaotic failures, and turn every battle into a replayable story. The game should be solo-first and mod-friendly, with strong replay/debug tools, readable UX, and progression built around mastery, creative loadouts, veteran actors, salvage, and shared challenge seeds.

## Target Player

| Player | Wants | Product Obligation |
|---|---|---|
| Cortex/Soldat/Liero veteran | Physical chaos, weapons, bodies, terrain, fast mastery. | Preserve direct-control friction but improve readability and controls. |
| Solo tactics player | AI companions that do not require human teammates. | Build AI trust harness and command UX before promising "great AI." |
| Builder/modder | Deep data, package validation, quick test loops. | Treat workbench UX as part of the product, not a side tool. |
| Replay/spectacle player | Weird deaths, heroic saves, shareable clips. | Recorder and replay cards need to exist early. |
| Long-tail campaign player | Persistent stakes without grind. | Progression should be horizontal, tactical, and story-rich. |

## Launch Boundary Candidates

These are not research bans. They are candidate launch promises to avoid overcommitting before prototypes exist.

| Area | Candidate Launch Boundary | Prototype Freedom |
|---|---|---|
| PvP | Do not promise launch PvP until terrain/entity authority and bandwidth are proven. | Prototype PvP/co-op/networking freely. |
| Gacha/monetization | Do not make monetized collection a launch pillar before fairness/modding/economy DR. | Prototype collection, gacha, cosmetics, and economy loops privately. |
| Noita-scale materials | Do not promise unlimited material chemistry for v1. | Prototype moonshot materials and labs freely. |
| Backend/live service | Local play, local replays, and local package validation should work before account/live-service dependency. | Prototype hubs, uploads, leaderboards, async strategy, and events freely. |

## Elevator Pitch Candidates

| Version | Pitch |
|---|---|
| Tactical | A solo-first tactical physics sandbox about commanding fragile AI squads through destructible battlefields. |
| Story | Every mission is a mess of bodies, tunnels, tools, rescues, and replayable mistakes you can actually understand. |
| Creator | A Cortex-like battlefield engine with a first-class modding workbench, replay debugger, and challenge-sharing loop. |

## Product Pillars

| Pillar | Must Be True |
|---|---|
| Physical battles create stories. | The sim must produce surprising outcomes without hiding causes. |
| AI is trustworthy enough for solo play. | Bots need visible intent, recovery behavior, and replayable failures. |
| UX makes chaos readable. | HUD, command, material, loadout, and replay surfaces must explain state fast. |
| Progression widens tactics. | New tools, veterans, salvage, and contracts should increase options, not raw grind. |
| Modding is core. | Packages, validation, provenance, and test launch belong in the core workflow. |

## Inputs

- [[strategy/best-cortex-like-game-principles]]
- [[design/design-decisions]]
- [[design/opportunities-for-our-fork]]
- [[game/what-is-cortex-command]]
- [[decisions/dr-011-progression-retention-loop]]
- [[spec/progression-retention]]
