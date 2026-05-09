---
type: decision
id: DR-021
status: closed-direction
priority: P0
closed_at: 2026-05-04
revisit_trigger: "v1 chassis content cost overruns; or heavy mechs turn the game into a mech-only experience and crowd out infantry/tactical play."
---

← [[decisions/index|decision records]] · [[dashboards/decision-tracker|decision tracker]] · [[spec/chassis-armor-mechs-and-origins|chassis spec]] · [[spec/authoritative-game-spec-v0|game spec v0]] · [[decisions/dr-014-tone-player-promise|DR-014]]

# DR-021: Mech Scale Ladder And Archetypes

> [!success] Status: CLOSED-DIRECTION (project owner committed 2026-05-04)
> Full chassis ladder at launch (powered armor → light → medium → heavy mech), with a constrained v1 roster but every tier proven and playable. Every tier includes distinct archetypes with shared chassis grammar — not just bigger stat versions.

## Decision

**Deep full ladder, not shallow stat ladder.**

### Required tiers at launch

| Tier | Scale | Role | Mission-Fit Notes |
|---|---|---|---|
| Powered armor | 1-1.5× human | Infantry-scale; fits tunnels; boosts armor/modules/mobility. | Always usable. |
| Light mech | 2-3× human | Fast tactical mech; pilot/eject/salvage loop. | Most missions. |
| Medium mech | ~4-5× human | Mission-shaping platform; heavier weapons; route-fit tradeoffs; needs larger spaces or prepared paths. | Some missions; cannot enter tight bunkers. |
| Heavy mech | ~6-10× human | Rare headline asset; slow, terrifying, terrain-altering, expensive to deploy/repair. | Set-piece missions; not usable in every scenario. |

### Required archetypes (each tier may include several)

| Archetype | Identity |
|---|---|
| Armored | Heavy plates, local armor stages, slow but durable, strong against ballistic/explosive. |
| Shielded | Electric/force-field defenses, shield emitters, recharge/overheat windows, EMP weak, visible shield arcs. |
| Assault | Mounted cannons, missile pods, breaching weapons, high recoil and ammo/heat pressure. |
| Engineer / Siege | Drills, cutters, concrete/foam sprayers, repair cranes, terrain-shaping tools. |
| Recon / Sensor | Radar, scanner, stealth/ECM, target painting, weaker direct combat. |
| Support / Repair | Repair beams, rescue winches, shield projectors, ammo/energy resupply. |
| Command | Stronger AI relay, squad buffs, sensor sharing, order range, command-core compatibility. |
| Experimental / Biomech | Self-repair, weird biological weapons, unusual vulnerabilities, high-risk abilities. |

Not every tier needs every archetype at launch. The ladder × archetype matrix is the long-term content surface; v1 picks a small but representative set.

### Module system (interchangeable equipment)

Mechs ship with module slots:

- Arm weapons.
- Shoulder weapons.
- Shield projectors.
- Reactors.
- Batteries.
- Sensors.
- Jump jets.
- Repair drones.
- Cockpit upgrades.
- Melee tools.
- Cargo clamps.
- Command relays.
- Deployable turrets.
- Special abilities.

Modules can be **damaged, jammed, overheated, disabled, destroyed, repaired, salvaged, or swapped**. This uses the chassis grammar from [[spec/chassis-armor-mechs-and-origins]].

## Anti-Goal

> Heavy mechs must NOT turn the game into only a mech game.

Foot infantry, androids, robots, powered armor, and light squads still matter. Large mechs are strategic assets with map-fit, power, transport, repair, sensor, pilot, and terrain consequences.

## What This Locks In

| Spec Area | Implication |
|---|---|
| Chassis spec | [[spec/chassis-armor-mechs-and-origins]] gets the archetype list + module list as first-class. |
| First playable | Slice A must include at least one chassis-bearing actor (powered armor) so the grammar is exercised end-to-end. Heavier mechs follow Slice B/C. |
| Mission design | Mission manifests include a "max chassis tier" parameter; some missions exclude heavy mechs. |
| Modding | Mod authors can add new tiers, archetypes, modules using the same grammar. |
| AI | Each archetype needs doctrine logic (the assault mech behaves differently from the shielded mech). See [[decisions/dr-008-ai-architecture]] and [[decisions/dr-022-ai-humanlike-bar]]. |
| Replay | Module damage/jam/destroy/repair/salvage events are mandatory. |
| UX | Chassis HUD must work for powered armor through heavy mech without falling apart. |
| Economy | Heavy mechs cost a lot to deploy and repair; this is part of the economy. See [[spec/progression-retention]]. |

## v1 Roster Constraint

Open exact count, but the principle: **every tier present, none of them lavish.** Suggested seed:

- Powered armor: 2-3 archetypes (e.g. armored, assault, engineer).
- Light mech: 2-3 archetypes (e.g. assault, recon, support).
- Medium mech: 1-2 archetypes (e.g. shielded, engineer/siege).
- Heavy mech: 1 archetype (e.g. armored or experimental).

Total v1 mechs ~6-9. Extension via mods.

## What This Does NOT Lock

| Non-Commitment | Why |
|---|---|
| Specific named mech models | Open. Each archetype gets at least one named entry; names tied to faction/setting. |
| Exact module count per chassis | Open. Tied to playtest balance. |
| Whether heavy mechs are pilotable directly or commander-only | Open. Pilotable likely; commander-only is a fallback if direct control feels bad. |
| Whether non-mech vehicles exist (tanks, dropships, hovers) | Open. Dropships already in lore for delivery. Tanks/hovers TBD. |
| Cosmetic / paint customization depth | Open. Modding-friendly. |

## Why Not The Alternatives

| Alternative | Why Rejected |
|---|---|
| Powered armor only | Loses headline mech fantasy DR-014 commits to. |
| Powered armor + light mech only | Same — under-delivers on the "tactical pulp sci-fi disaster sandbox" promise. |
| Powered armor + light + medium | Closer, but loses the "rare terrifying heavy" set-piece moments. |
| Full ladder lavish (10+ mechs/tier) | Too much content cost for v1. |

## Evidence Trail

- Project owner verbatim (2026-05-04 spec round 3): "Full ladder, but with a constrained v1 roster and distinct mech archetypes… every scale tier should exist, be playable, be damageable, and prove the shared chassis grammar."
- Captured in [[research-log/2026-05-04-spec-round-3-visuals-audio-tutorial-mechs-ai]].
- Builds on [[spec/chassis-armor-mechs-and-origins]] grammar.

## Revisit Trigger

- v1 chassis content cost overruns.
- Heavy mechs crowd out infantry/tactical play.
- Module system proves too complex for AI or modders.
