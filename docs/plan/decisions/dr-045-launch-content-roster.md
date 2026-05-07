---
type: decision
id: DR-045
status: closed-direction
priority: P0
closed_at: 2026-05-06
revisit_trigger: "Roster scale proves unbalanceable in playtest cohort; or AI agents cannot author content at the volume required; or modders' bandwidth is competing with first-party content."
---

← [[decisions/index|decision records]] · [[dashboards/decision-tracker|decision tracker]] · [[spec/launch-content-roster|launch content roster]] · [[spec/equipment-loadout|equipment/loadout]] · [[spec/chassis-armor-mechs-and-origins|chassis/armor/mechs/origins]] · [[decisions/dr-044-audiovisual-production-pipeline|DR-044]]

# DR-045: Launch Content Roster — Scale, Authoring, Modding Parity

> [!success] Status: CLOSED-DIRECTION (project owner committed 2026-05-06)
> Launch content scale: **70+ weapons, 24+ actors, 12+ vehicles/dropcraft, 60+ base objects, 8 factions, 30+ missions, 12 worlds, 17 launch materials + expansion lab, 12 ores, 24+ tools/consumables**. Plus full AI-driven authoring pipeline so modders can hit parity. Inspired by Cortex Command's 5-faction × ~80-item baseline, scaled for modern player expectations.

## Decision

### Launch content target — minimum at v1.0

| Category | Count | Notes |
|---|---|---|
| **Weapons (firearms)** | **40+** | Pistols, SMGs, assault rifles, battle rifles, sniper rifles, shotguns, LMGs, HMGs, blaster pistols, laser rifles, plasma weapons, energy beams, anti-mat rifles, designated marksman rifles, micro-SMGs, dual-wield sidearms. |
| **Weapons (heavy/explosive)** | **15+** | Rocket launchers, grenade launchers, flak cannons, missile launchers, mortars, gauss cannons, railguns, particle accelerators. |
| **Throwables/explosives** | **15+** | Frag grenades, smoke, flash, EMP, incendiary, sticky, remote charge, tripwire mine, claymore, satchel charge, nano-grenade, plasma orb, breach charge, decoy beacon. |
| **Melee** | **8+** | Combat knife, machete, riot baton, vibroblade, plasma sword, monomolecular katana, stun rod, chainsaw. |
| **Tools** | **15+** | Light/medium/heavy diggers, breaching tool, repair tool, concrete sprayer, foam constructor, sample scanner, metal detector, oxygen analyzer, geological scanner, drill (light/medium/heavy/vacuum-rated), drill bits, ore cargo pack, refining unit, smelter unit. |
| **Mobility** | **6+** | Grapple hook, jetpack assist, rope/tether, deployable ladder, harpoon line, magnetic boots. |
| **Shields** | **5+** | Riot shield, combat shield, deployable barrier, energy shield, kinetic dampener. |
| **Sensors** | **6+** | Motion scanner, noise detector, material probe, EM scanner, heat-vision, command sensor. |
| **Medical** | **8+** | Medikit (basic/advanced), medical dart, stim, revive kit, defibrillator, suture stapler, pain blocker, decontamination spray. |
| **Repair/support** | **6+** | Repair tool (basic/advanced), spare module pack, oil/coolant refill canister, EMP-proof patch, weld torch, diagnostic sensor. |
| **Comms** | **10+** | Helmet voice pickup, throat mic, bone conductor, handheld VHF, backpack VHF, HF transceiver, satellite uplink, antennas (whip/dipole/yagi/dish/ground-plane), jammer, encryption module. |
| **Total weapons + tools + gear** | **140+** items | At launch. Plus modding-extensible. |
| | | |
| **Actors — humans** | **8** | Light infantry, heavy infantry, scout, sniper, assault, medic, engineer, demolitions. |
| **Actors — power armor (humans)** | **4** | Light PA, medium PA, heavy PA, jump PA. |
| **Actors — androids** | **4** | Civilian android (basic), military android (combat), engineer android, infiltrator android. |
| **Actors — robots** | **5** | Combat robot, scout drone, security drone, repair drone, anti-air drone. |
| **Actors — mechs** | **5** | Light mech, medium mech, heavy mech (4-leg crab), siege mech (artillery), aerial mech (gunship-style). |
| **Actors — civilians/NPCs** | **6** | Hostage, scientist, engineer-NPC, salvager-NPC, broker-NPC, prisoner-NPC. |
| **Actors — undead/anomaly** | **6** | Zombie thin, zombie medium, zombie fat, skeleton, mutant, alien-husk. |
| **Actors — turrets/static** | **6** | MG turret, AC turret, missile turret, laser turret, sentry drone, AA emplacement. |
| **Total actors** | **44+** at launch | Plus named NPCs (heroes/antagonists, see narrative bible). |
| | | |
| **Dropcraft (vehicles)** | **12+** | Light dropship, heavy dropship, attack dropship, rocket capsule, drop pod (1-actor), supply pod, troop transport, gunship, salvage rig, mining rig, scout drone-carrier, evac shuttle. |
| **Ground vehicles** | **6+** | APC, scout buggy, mining hauler, cargo flatbed, mobile command, recon walker. |
| | | |
| **Base objects** | **60+** | Command core (rooted/uprooted), shield generator, power core, power node, power cable, defense turret (4 variants), automated turret control, oxygen generator, atmosphere pump, vent, filter, gas tank, water tank, pipe segments (gas/liquid/coolant/power), valve, regulator, condenser, evaporator, sealed door (small/medium/large/airlock), reinforced wall section, sensor station, alarm, repair pad, cargo storage, ammo storage, weapon rack, medical bay, brain case mount, command console, scanner array, comm relay, satellite uplink station, jammer station, gravity generator, gravity well projector, magnetic plating, refinery, smelter, foundry, ore cargo bay, ladder, lift platform, elevator shaft, vending kiosk, deployable beacon, deployable bunker module, sandbag emplacement, blast door, decontamination chamber, autoclave, fuel canister rack, drone bay, dropship pad, hangar door. |
| | | |
| **Factions (launch)** | **8** | Trade Star (corporate mercenary), Coalition (military), Browncoats (heavy assault clones), Ronin (loner specialists), Tek-Mart (techbro frontier rats), Imperatus (oppressive empire), Free Hold (frontier independents), The Husks (post-anomaly biological-mechanical hybrids — antagonist faction). Plus 1 secret post-launch faction unlock. |
| | | |
| **Missions (launch)** | **30+** | 3 onboarding (DR-023 hybrid+) + 8 modular labs + 6 anchor campaign missions + 8 procedural contract templates + 4 Bunker Defence flagship maps + 3 PvP arena maps + 2 coop-vs-AI scenarios + 6 modder-template scenarios. Each with comic-panel briefing + debrief. |
| | | |
| **Worlds (launch)** | **12** | Per [[spec/celestial-bodies-and-worlds-model]]: Earth, Earth's Moon, Mars, Phobos, Deimos, Mimas, Europa, Vulcan, Venus, Sol, BeltAsteroid-A1, OrbitalStation-A1. |
| **Biomes per world** | **3-5 each** | ~50 biome variants total (urban ruin, desert, ice cave, vacuum surface, volcanic, jungle, sealed corridor, coral reef alien, etc.). |
| | | |
| **Materials (launch)** | **17 + lab expansion (10+)** | Per DR-036. |
| **Ores (launch)** | **12** | Per DR-041. |
| **Reactions (launch)** | **20+** | Per DR-036 reaction table. |
| | | |
| **Music tracks (launch)** | **30+** | Per [[spec/music-and-soundtrack]]: 1 main theme, 6 world themes, 6 combat layers, 4 base-tension layers, 4 menu/UI tracks, 8 mission-specific stings, 2-3 hero antagonist motifs. Adaptive layered. |
| **SFX library** | **400+** clips | Per `cf-audio` registry. Caption-bound per DR-020. |

### Authoring requirements

Every roster item MUST be:

1. **Generatable by an AI agent** end-to-end (Tier 1 → Tier 2 → Tier 3 per DR-044). Prompt + seed + ControlNet inputs committed.
2. **Authored as data** in `content/<category>/<id>.ron` per DR-006 schema-first.
3. **Functional in-game** — every weapon fires; every chassis walks/jumps/dies; every base object powers/repairs/breaks; every consumable is consumable. NO "asset exists but stat-only" entries.
4. **AI-readable** — has a role-card, AI metadata, refusal reasons, capability tags per [[spec/equipment-loadout]].
5. **Replay-recorded** — emits typed events; cause-chain visible.
6. **Caption-bound** — every audible cue has a caption per T-AUDIO + T-ACCESSIBILITY.
7. **Mod-validated** — `cf-mod validate --strict` passes.
8. **Localizable** — every player-visible string is keyed.
9. **Balance-fixtured** — has a BALANCE-A row in `content/balance/fixtures/`.
10. **Hot-reloadable** — `cf-mod reload <id>` works at runtime in dev builds.

### Modding parity requirements

Modders MUST be able to:

- Run the same Tier 1/2/3 generation pipeline.
- Use the same ComfyUI workflow `.json` files committed to repo.
- Use the same ControlNet inputs (chassis silhouette templates, palette JSON, action poses).
- Use the same Aseprite headless cleanup pipeline.
- Use the same `cf-asset-pipeline` CLI.
- Use the same balance-fixture format.
- Submit content via Steam Workshop (per DR-050 + DR-047) with one-button publish.
- Hot-reload mod content in-game during development.
- Author a complete chassis (sprite + animation + role-card + balance fixture + scenario test) **in under 1 hour** with AI agent assistance (target: 15-30 min average).

## What This Locks In

| Spec Area | Implication |
|---|---|
| `content/` directory | Master roster. Every category gets its own subdirectory: `weapons/`, `actors/`, `vehicles/`, `base_objects/`, `factions/`, `missions/`, `materials/`, `ores/`, `reactions/`, `music/`, `sfx/`, `worlds/`, `biomes/`. |
| Schema-first authoring | RON manifests per DR-006; validated by `cf-mod validate`. |
| Balance fixtures | `content/balance/fixtures/<category>/<id>.ron` — TTK matrix, faction asymmetry, economy curves, AI difficulty. |
| Modding workshop | Steam Workshop integration + community-hostable mod repository per DR-050 + DR-047. |
| AI-driven authoring | Every roster item is auto-generated by AI agent from a prompt template + ControlNet inputs. |

## What This Does NOT Lock

| Non-Commitment | Why |
|---|---|
| Specific weapon stats | Open. Will be tuned in M-BALANCE pass with playtest data. |
| Final faction lore | Open per DR-016 + future narrative bible. |
| Post-launch DLC content | Open. v1.0 roster is committed; expansions are post-launch decisions. |
| Whether all named NPCs are voiced | Open. Default text-only with subtitle-style presentation. |
| Final balance numbers | Open until BALANCE-A acceptance suite passes. |

## Why This Scale

| Driver | Detail |
|---|---|
| Cortex Command precedent | CCCP base game has ~5 factions × ~80 items × ~10 actors each. Modern player expectation is 1.5-2× that for a "complete" indie release. |
| AI-augmented authoring | Per DR-044 + DR-026, AI agent can author 5-10 items/day at Tier 2 quality. 6-month content pass = ~700 items. Conservative target = 280+ unique authored items. |
| Modding ecosystem | Modders won't bother if first-party content is too thin (signals "abandoned"). 100+ first-party items + clear authoring pipeline signals "alive ecosystem." |
| Match-grammar variety | Per DR-042, 6 match modes × 4 team configs × 12 worlds × 8 factions = thousands of unique match setups. Roster needs to support that combinatorial space. |
| Replayability | Per DR-048, retention requires per-mission variety. 30+ launch missions + procedural contracts + Bunker Defence + creator tools + match grammar = hundreds of unique sessions. |

## Why Not The Alternatives

| Alternative | Why Rejected |
|---|---|
| Smaller launch roster (~20 items) | Cortex precedent + modder-signal floor + match-grammar combinatorics demand 100+ items minimum. |
| Larger launch roster (~500 items) | Authoring cost (even with AI) + balance complexity + playtest cohort bandwidth + first impression "noise" risk. |
| Defer most content to post-launch DLC | Launch must feel complete. DR-031 forbids gating core mechanic content behind DLC. |
| Skip vehicle/dropcraft | Dropships are core to Cortex player loop (delivery + LZ risk + extraction); cannot skip. |
| Generic enemy roster only (no factions) | Loses tactical pulp tone (DR-014). Faction visual register + doctrine + signature gear is product promise. |

## Evidence Trail

- Project owner verbatim (2026-05-06): "full roaster of equipments, actors, terrain, base objects, etc... I want everything you listed above and more!!!! a lot more!"; "iteams equipment etc should be plentifull"; "use cortex command repos asset as inspiration"; "I want the equipment to work, I want all this stuff to be moddable by others when the game releases".
- Cortex Command Wiki List of Weapons: https://datarealmscortexcommand.fandom.com/wiki/List_of_Weapons (~30 weapons in vanilla CC).
- Cortex Command Wiki List of Actors: https://datarealmscortexcommand.fandom.com/wiki/List_of_Actors (~20 actors across 5 factions in vanilla).
- CCCP unified repo: `/Users/erol/projects/cortex-command-repos-all/Cortex-Command-Community-Project/Data/` (CCCP-extended roster ~80+ weapons, ~30 actors, ~5 factions).
- Captured in [[research-log/2026-05-06-ai-driven-asset-pipeline-research]].

## Revisit Trigger

- Roster scale proves unbalanceable in playtest cohort.
- AI agents cannot author content at the volume required.
- Modders' bandwidth competes with first-party content (signal: modder count drops).
- Specific category becomes bloated without gameplay value (e.g., 40+ weapons but only 3 are used in matches).
