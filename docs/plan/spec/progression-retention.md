---
type: spec
status: exploratory-reqs
ready_when: "RET-A tests have results from actor-feel, replay, loadout, and one repeatable mission prototype."
---

← [[spec/index|spec section]] · [[decisions/dr-011-progression-retention-loop|DR-011]] · [[decisions/dr-014-tone-player-promise|DR-014]] · [[systems/ux-ui-and-retention|UX/retention]] · [[spec/core-loop|core loop]] · [[spec/chassis-armor-mechs-and-origins|chassis/armor/mechs/origins]] · [[research-log/moonshot-register|moonshots]]

# Progression And Retention

> [!warning] Exploratory requirements
> This page is a design target and test plan, not a settled launch promise. It exists so prototypes can test the return loop with real gameplay evidence. Prototype daily seeds, roguelite structures, collection mechanics, async strategy, and copied/reused reference ideas freely; promote only what improves the game.

## North Star

Players should come back because battles create new readable stories and because they can improve their command, control, squad doctrine, terrain planning, and loadout craft.

Retention is a game-quality requirement here, not just a metric target. A good session should leave the player with at least one of these thoughts:

| Return Thought | Design Support |
|---|---|
| "I can beat that seed cleaner." | Same-seed retry, replay timeline, personal best, failure cause. |
| "This squad deserves another mission." | Named actors, scars, traits, rescue history, veteran UI. |
| "This machine deserves repair." | Damaged armor, recovered mech hulls, repaired modules, android shells, robot frames, and battle scars. |
| "This new tool changes the plan." | Horizontal equipment unlocks, lab tests, loadout templates. |
| "The enemy commander surprised me." | Visible enemy doctrine, adaptation, scouting clues. |
| "I want to show this moment." | Replay card, seed hash, mod/package list, short export. |
| "I can build a better bunker/challenge." | Workbench, package validation, modded contract browser. |

## Retention Stack

| Layer | Required Surface | Prototype Dependency |
|---|---|---|
| Core feel | Single actor must be fun for five minutes without meta rewards. | [[spec/actor-feel-sandbox-slice-a]] |
| Tactical command | Orders must show intent, path, blocked reason, and recovery behavior. | [[spec/ai-trust-harness-slice-a]], [[spec/ux-wireframes-slice-a]] |
| Loadout craft | Templates, role filters, AI competence, mass/cost/delivery warnings. | [[spec/equipment-loadout]], [[references/equipment-provenance-workbench-view]] |
| Destructible problems | Contracts must vary terrain/material/objective pressure. | [[spec/terrain-material-sandbox-slice-a]], [[systems/destruction-objective-mission-patterns]] |
| Persistence | Actors, salvage, base state, enemy commander, contract history. | Save/campaign stub; not built yet. |
| Learning | Replay/event recap explains deaths, breaches, AI failures, and losses. | [[spec/replay-recorder-slice-a]] |
| Sharing | Deterministic seed cards, replay exports, mod/package hashes. | [[spec/backend-service-hub-slice-a]], [[spec/package-builder-workbench-slice-a]] |
| Optional collection | Cosmetic/story/trophy layer first; power collection only after fairness DR. | Future monetization/economy DR. |

## Core Loop Integration

| Phase | Player Job | System Job | UX Output | Retention Function |
|---|---|---|---|---|
| Choose contract | Pick objective, seed, difficulty, constraints, faction pressure. | Validate material/path/AI feasibility. | Contract card with terrain/material/equipment warnings. | Variety + autonomy. |
| Build squad | Select actors, doctrine, tools, weapons, craft. | Check roles, mass, cost, bot competence, missing counters. | Loadout builder with saved templates and warnings. | Experimentation + mastery. |
| Deploy | Choose entry, delivery timing, fallback. | Simulate delivery risk and initial AI goals. | Delivery preview and abort/retry affordance. | Tactical agency. |
| Fight/command | Direct-control one actor; order others; adapt to terrain. | Run AI, physics, destruction, events. | HUD, squad panel, order overlay, material overlay. | Skill + story generation. |
| Rescue/recover | Extract brain, save actors, salvage gear, repair damage. | Persist deaths, wounds, inventory, terrain marks. | Recovery checklist and risk warnings. | Emotional stakes. |
| Recap/replay | Inspect cause of loss/win and key events. | Export stable event log, seed, package list. | Timeline, death causes, replay/share card. | Learning + sharing. |
| Improve | Update template, veteran, base, lab, commander dossier. | Save horizontal unlocks and lessons. | "Next best test" suggestions without mandatory chores. | Return reason. |

## Progression Objects

| Object | Fields To Prototype | Consumer |
|---|---|---|
| `campaign_profile` | profile id, campaign seed, difficulty posture, unlocked labs, contract history, replay archive ids. | Save system, hub, replay browser. |
| `actor_veteran` | stable actor id, name, role, scars, injuries, traits, rescue count, mission count, favorite loadout. | Squad UI, AI doctrine, replay recap. |
| `chassis_record` | chassis id, owner/pilot history, armor/module condition, repairs, scars/paint, salvage state, mission count. | Loadout UI, mech bay, replay recap, progression, AI compatibility. |
| `origin_profile` | origin id, treatment/repair needs, vulnerabilities, personality/story tags, compatible armor/mechs. | Squad creation, AI, body damage, retention, mission constraints. |
| `loadout_template` | actor roles, item ids, role tags, mass, cost, delivery craft, AI warnings, package hashes. | Buy/loadout UI, package diagnostics, balancing. |
| `contract_seed` | seed id, objective, map/material profile, constraints, reward class, required capabilities, validation status. | Mission generator, replay, challenge browser. |
| `salvage_manifest` | recovered items, scrap/material types, enemy tech, damaged gear, base repair deltas. | Economy, loadout, workbench. |
| `enemy_commander` | commander id, doctrine, visible adaptations, grudges, recent defeats, scouting clues. | AI harness, campaign UI, mission briefing. |
| `replay_card` | seed, result, loadout, key events, actor fates, package versions, share hash. | Replay viewer, community browser, support/debug. |
| `collection_entry` | cosmetic/story/trophy id, source event, unlock path, release-readiness tag. | Optional cosmetics/identity; future economy DR. |

## Prototype Acceptance Tests

| ID | Name | Setup | Pass Criteria |
|---|---|---|---|
| RET-A-01 | Actor mastery retry | One five-minute bunker-breach obstacle course with same-seed retry. | Player can identify one skill mistake and wants to retry; replay shows cause. |
| RET-A-02 | Contract generator | Three deterministic contracts with distinct material/equipment constraints. | Each has a valid route, clear role requirement, and no impossible AI state. |
| RET-A-03 | Veteran value | One actor survives two missions with a visible scar/trait. | Player can explain what changed and why they care. |
| RET-A-04 | Salvage to loadout | Recovered item/tool changes next mission template. | Player sees a new tactical option without raw power creep. |
| RET-A-04B | Salvaged chassis repair | Damaged armor/mech module survives a mission and can be repaired or refit. | Player can explain whether repair, strip-for-parts, or fielding the damaged chassis is the better next move. |
| RET-A-05 | Enemy commander adaptation | Enemy changes one visible tactic after a player win. | Player can name the adaptation from briefing or battlefield evidence. |
| RET-A-06 | Loss recap | Player loses actor/objective to terrain, blast, or AI failure. | Recap names cause, timeline event, and retry option within 10 seconds. |
| RET-A-07 | Challenge sharing | Export a seed/replay card and re-open it locally. | Contract, loadout, package list, and replay hash round-trip. |
| RET-A-08 | Modded contract | Install a test package that adds one contract modifier. | Validator catches missing role/material metadata before launch. |
| RET-A-09 | No-obligation pacing | Player quits after one mission and returns later. | No lost mandatory reward; resume surface is clear. |
| RET-A-10 | Horizontal unlock | Unlock a new tool/variant. | It changes route or role strategy without obsoleting an earlier tool. |

## UI Requirements

| Surface | Must Show |
|---|---|
| Contract card | Objective, expected session length, material profile, required roles, seed, constraints, reward type, validation badge. |
| Campaign map | Current pressure, available contracts, base damage, enemy commander clues, saved challenge seeds. |
| Squad/veteran panel | Name, role, health, scars, traits, current doctrine, rescue risk, recent event. |
| Loadout builder | Role filters, item tags, AI competence, provenance/warnings, delivery risk, missing capability summary. |
| Mech/chassis bay | Origin compatibility, armor slots, module condition, repair cost, route/delivery warnings, pilot/rescue state. |
| Mission HUD | Current contract goal, high-risk actor warnings, salvage/recovery prompts only when relevant. |
| Recap screen | Win/loss cause, key events, actor fates, salvage, retry same seed, save replay, edit loadout. |
| Replay card | Mission title, seed, duration, result, mods/packages, notable events, share/export actions. |
| Lab/workbench | Test weapon/material interactions, compare role metadata, validate package fields, create contract fixtures. |

## Data And Telemetry

| Metric | Why It Matters |
|---|---|
| Same-seed retry rate | Measures mastery pull without external rewards. |
| Time to first meaningful event | Confirms the loop gets to fun quickly. |
| Loadout edits after recap | Shows whether replay teaches useful changes. |
| Veteran preservation behavior | Tests emotional stakes without hard punishment. |
| Chassis repair/reuse rate | Tests whether armor/mechs create attachment and tactics rather than maintenance chores. |
| Salvage usage in next mission | Checks whether economy creates decisions. |
| Contract abandonment cause | Finds impossible/boring/generated bad seeds. |
| Replay saved/shared/opened | Measures spectacle and learning loop. |
| Mod challenge install errors | Measures creator/community friction. |
| Session return after no reward claim | Checks whether retention survives without obligation. |
| Power-obsolescence incidents | Flags power creep when new tools erase old roles. |

## Guardrails

| Guardrail | Rule |
|---|---|
| No core-power opacity | Core counters, terrain tools, and role coverage need transparent access in any settled spec. |
| No missed-reward punishment | Shared daily seeds are fine; missed daily chores are not a retention foundation. |
| No UI dark patterns | No fake urgency, hidden costs, confusing currency, obstructed cancellation, or disguised purchases in any release-facing plan. |
| Modding remains first-class | Official progression must not make mods feel second-class or invalid by default. |
| AI must understand progression | Veteran traits, item roles, and contract constraints need AI metadata and harness cases. |
| Replays must explain progression losses | If an actor dies, a contract fails, or salvage is lost, the recap must explain why. |

## Open Questions

| Question | Next Evidence |
|---|---|
| How much persistence is enough before the game feels like a campaign? | RET-A-03 and RET-A-04 prototype feedback. |
| Should contracts be roguelite runs, campaign ops, or both? | RET-A-02 and first proof mission. |
| How expressive should veteran traits be? | AI harness and HUD readability tests. |
| Should chassis/mechs have veteran-like history? | CHASSIS-A plus RET-A-04B. |
| Can enemy commanders adapt without feeling unfair? | RET-A-05 plus AI-H scenario failures. |
| Does salvage improve tactics or just create chores? | Loadout edits and salvage usage metrics. |
| Which sharing layer comes first: local replay cards, backend upload, or mod browser? | Backend/hub Slice A tests. |
| Can collection mechanics add identity without harming fairness? | RET-A-08 and future monetization/economy DR. |

## Source Trail

- [[decisions/dr-011-progression-retention-loop]]
- [[systems/ux-ui-and-retention]]
- [[spec/core-loop]]
- [[spec/product-promise]]
- [[spec/chassis-armor-mechs-and-origins]]
- [[spec/equipment-loadout]]
- [[references/equipment-provenance-workbench-view]]
- [[comparables/the-powder-toy-local-audit]]
- [[comparables/openlierox-local-audit]]
- [[comparables/opensoldat-local-audit]]
- GameDeveloper, GDC Online retention: https://www.gamedeveloper.com/game-platforms/gdc-online-player-retention-requires-real-motivation-engagement
- GameDeveloper, replayability mechanics: https://www.gamedeveloper.com/design/replayability-part-2-game-mechanics
- GameDeveloper, power progression: https://www.gamedeveloper.com/design/power-progression-in-games-crafting-rewarding-player-experiences
- Springer, SDT/video games: https://link.springer.com/article/10.1007/s11031-006-9051-8
- FTC, dark patterns report: https://www.ftc.gov/reports/bringing-dark-patterns-light
- HBS faculty research, personalized game design: https://www.hbs.edu/faculty/Pages/item.aspx?num=67771
