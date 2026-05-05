---
type: spec
status: planning-anchor-v0
authority: "World frame and faction grammar. Specific lore (named places, factions, antagonists) remains open and grows via play and mods."
ready_when: "First playable Breach Contract mission has at least one faction with a doctrine that is recognizably non-generic."
feeds:
  - DR-014
  - DR-015
  - DR-016
  - DR-017
---

← [[spec/index|spec section]] · [[spec/authoritative-game-spec-v0|game spec v0]] · [[spec/product-promise|product promise]] · [[spec/command-core-base-power|command core]] · [[decisions/dr-016-setting-and-world-frame|DR-016]] · [[decisions/dr-014-tone-player-promise|DR-014]] · [[decisions/dr-015-player-identity-control-posture|DR-015]]

# Setting And World Frame

> [!summary] What this page is
> The frontier disaster-contract sci-fi world frame. It is broad enough to support the full chassis/origin/faction grammar without copying Cortex Command's "brain in a jar" lore, and tight enough to give designers and mod authors a clear writing prompt.

> [!warning] Authority boundary
> v0 planning anchor. The frame is committed (DR-016). Specific factions, places, antagonists, and named characters are open and develop through play, prototype missions, and mods.

## The World In One Paragraph

Humanity (and friends, rivals, things) has spread across a fragile interstellar frontier. Old colony charters are failing, corporate empires are collapsing or feeding on the corpses, alien biomes are eating settlements, and abandoned megastructures hold riches and horrors. The player runs a small **merc / rescue / salvage outfit** that takes contracts in this mess. Some jobs are heroic. Some are corporate. Most go wrong. The player's identity is anchored in a **command core** that powers their base when rooted and becomes a dangerous mobile avatar when uprooted into a chassis.

## Recurring Mission Contexts

These are the world's writing prompts. A mission, anchor or generated, lives in one or more of these contexts:

| Context | Description | Example Job Types |
|---|---|---|
| Collapsing frontier colony | Dome cracking, life support failing, militia turning feral, corporate eviction looming. | Evacuate civilians, salvage cargo, defend a clinic, recover a brain core, secure water. |
| Corporate war zone | Two corps shooting each other for an asset; the player is hired by one (or neither). | Sabotage a refinery, raid a HQ, broker a ceasefire under fire, extract a deserter. |
| Alien biome | Hostile fauna, parasitic flora, weather systems, biological architecture. | Sample collection, rescue stranded survey team, clear an infestation, contain an outbreak. |
| Derelict megastructure | Generation ship, orbital ring, cracked Dyson element, void hulk. | Salvage rare tech, reactivate systems, rescue trapped crew, fight feral AIs/drones. |
| Disaster site | Recent catastrophe — crash, attack, breach, runaway nanite event. | First responders, triage, recover black-box data, contain spread. |
| Black-site / off-books | Illegal or unsanctioned operation. | Heist, prisoner break, evidence recovery, extracting a defector. |

The frame is generative without being heavy-handed; one mission can sit in two contexts (e.g. corporate war zone inside a derelict megastructure).

## Faction Grammar

The world supports many factions, but each faction reads through a small set of axes that shape both its missions and its loadouts:

| Axis | Range |
|---|---|
| Doctrine | Disposable swarm ↔ small elite ↔ siege/defensive ↔ scavenger ↔ corporate professional ↔ ideological |
| Tech tier | Salvage / improvised ↔ standard issue ↔ corporate top tier ↔ experimental ↔ alien |
| Origin mix | Pure organic ↔ organic + drones ↔ androids ↔ hybrids ↔ alien |
| Stance toward player | Always hostile ↔ contract-driven ↔ rival ↔ ally-of-convenience ↔ allied |
| Visual register | Gritty utilitarian ↔ corporate slick ↔ pulpy retro ↔ biological strange ↔ cobbled-together |

A faction is described by picking one or two values per axis. Mods add new factions by doing the same.

> One faction may be "disposable swarm + improvised + alien + always hostile + biological strange" — that's the alien biomass swarm. Another may be "small elite + corporate top tier + androids + contract-driven + corporate slick" — that's a corporate cleanup squad. Both fit.

## Suggested Launch Faction Set

Open. Working seed:

| Faction (working name) | Axes | Role |
|---|---|---|
| The Player's Outfit | Variable / contract-driven | The player. |
| Dominion Salvors | Scavenger + improvised + organic+drones + rival | Default rival; competes for jobs and salvage. |
| Halver Industries | Corporate professional + corporate top tier + androids + ally-of-convenience | A corp that hires the player and can betray. |
| The Seethe | Disposable swarm + alien + alien + always hostile | Biological threat in alien biomes. |
| Continuity Chapel | Ideological + experimental + hybrids + rival | Synthetic-religious faction with strong views about the command core. |

These are placeholders to prove the faction grammar. The actual launch roster is open and will grow via mods.

## Bodies, Origins, And Cheap Doctrines

DR-014 commits to first-class chassis/armor/mechs/origins. DR-016 adds the world rule that **bodies are not all disposable trash**. Specifically:

- The player's outfit treats named actors as valuable. Veterans, repair projects, salvage, and legacy assets are core retention objects per [[spec/progression-retention]].
- **One** common faction doctrine (`disposable_swarm`) uses cheap bodies/drones intentionally. This is a *faction choice*, not a world default.
- Other factions may use small elite squads, corporate cleanups, alien biomass, or hybrids.

## Command-Core Lore Hook

The "brain in a jar" mechanic from Cortex becomes the **command core**. Lore-side:

- Origin of the command core is open (technological, biological, hybrid, salvaged, gifted, stolen).
- Multiple in-world theories about what the command core is can coexist (mods can add their own).
- The continuity-commander identity (DR-015) is the player's relationship to the core.
- Mechanically, the core powers the base when rooted and becomes a mobile avatar when uprooted (see [[spec/command-core-base-power]]).

The lore is intentionally underdetermined so players, modders, and the design team can shape it.

## What This Frame Is Not

| Not | Why |
|---|---|
| A military realism game | Pulpy systemic consequences and surreal sci-fi accents (DR-014) are part of the frame. |
| A pure dystopia or pure utopia | The frontier is messy; some jobs are heroic, some venal. |
| A single-protagonist narrative | The player runs an outfit, not a single hero. Even the command core is a vessel/anchor, not a person. |
| Locked to one biome or culture | The frame supports many contexts, factions, origins, biomes. |
| A faithful Cortex Command universe | The mechanics inherit; the lore does not. |

## Open Questions

| Question | Status |
|---|---|
| What's the in-fiction term for "command core"? | Open. Working term: command core. Alternatives: continuity core, neural anchor, operator uplink, company command node. |
| FTL / scale of the frontier? | Open. Suggestion: deliberately vague. |
| Is there a recurring antagonist faction? | Open. Likely The Seethe + a corporate rival. |
| How does the player's outfit get hired? | Open. Likely a contract board / hub. |
| Persistent campaign world map? | Open. Tied to [[spec/progression-retention]] and [[decisions/dr-013-backend-service-scope]]. |

## Source Trail

- [[decisions/dr-016-setting-and-world-frame]]
- [[decisions/dr-014-tone-player-promise]]
- [[decisions/dr-015-player-identity-control-posture]]
- [[spec/authoritative-game-spec-v0]]
- [[spec/product-promise]]
- [[spec/chassis-armor-mechs-and-origins]]
- [[spec/command-core-base-power]]
- [[spec/progression-retention]]
- [[spec/missions-and-objectives]]
- [[spec/mission-director-slice-a]]
