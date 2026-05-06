---
type: decision
id: DR-037
status: closed-direction
priority: P0
closed_at: 2026-05-06
revisit_trigger: "Atmospherics kernel cannot meet active-region perf budget at 60Hz/120Hz on Steam Deck floor; suit life-support math produces unfair invisible deaths the player cannot debug; combustion stoichiometry runs nondeterministic across replay; or Stationeers-grade depth meaningfully pulls focus from combat-base genre per DR-027 such that the project owner reverts to coarser room/atmosphere model."
---

← [[decisions/index|decision records]] · [[dashboards/decision-tracker|decision tracker]] · [[spec/atmospherics-and-chemistry-model|atmospherics/chemistry spec]] · [[spec/origin-reaction-and-resource-model|origin reaction/resource]] · [[spec/full-collision-physics-plan|full collision plan]] · [[spec/prototype-roadmap|native roadmap]] · [[decisions/dr-007-terrain-material-model|DR-007]] · [[decisions/dr-027-combat-base-scope|DR-027]] · [[decisions/dr-033-full-collision-physics-direction|DR-033]] · [[decisions/dr-036-systemic-material-simulation-direction|DR-036]]

# DR-037: Stationeers-Grade Atmospherics And Chemistry Direction

> [!success] Status: CLOSED-DIRECTION (project owner committed 2026-05-06)
> The game ships **Stationeers-grade atmospherics**: real ideal gas law (PV=nRT), per-gas molar quantities with locked specific heats / latent heats / autoignition temperatures, deterministic combustion stoichiometry, gradual phase change, first-class pipe networks, room atmospheres with doors as pressure barriers, suit/helmet/lung life-support, planetary atmospheres, vents/valves/regulators/filters/condensation chambers, breach detection, and wind from pressure differentials. Implementation extends the existing M7.5 (Base Atmospherics) and lands a new **M5.9 — Atmospherics-Grade Kernel** between M5.8 (origin resource) and M6 (AI core). DR-036 already commits to systemic materials; this DR raises atmospherics from "approximate Barotrauma-style" to "real Stationeers-grade chemistry/pressure" while keeping the curated-launch-set discipline of DR-036.

## Decision

**Atmospherics is a first-class simulated system, not a backdrop.** Every actor reads ambient atmosphere from one source. Every device that affects atmosphere is a node in the kernel. Every gas reaction has a deterministic stoichiometric output, replay-visible cause chain, and HUD-readable hazard overlay. The model mirrors Stationeers (the most authentic atmospherics in any game) but inherits the curated launch set of DR-036.

This DR ratifies what [[spec/atmospherics-and-chemistry-model]] specifies, elevates atmospherics from "future M7.5 stub" to "core direction with locked grammar", and threads it through the roadmap.

## What This Locks In

| Aspect | Commitment |
|---|---|
| Equation of state | `P · V = n · R · T` per Stationeers convention (R = 8314.46 L·Pa / mol·K). Per-atmosphere unit struct unifies room cells / pipe networks / suits / canisters / lungs / device internals. |
| Launch gas registry | Inherits Stationeers' base 7 gases (O2, N2, CO2, Volatiles=CH4, Pollutant=X, N2O, H2O) + elementals (H2, He, O3) + 6 launch liquid mixtures (Polluted Water, Alcohol, Silanol, Liquid NaCl, HCl, Hydrazine). Per-gas constants locked: specific heats, latent heats, autoignition temperatures, condensation/freeze pressures + temps, molar masses. See [[spec/atmospherics-and-chemistry-model#Gas Registry (Launch Set)]]. |
| Combustion engine | Deterministic stoichiometry per locked reaction table. Volatiles+O2, Volatiles+N2O, Volatiles+O3, H2+O2, H2+N2O, H2+O3 — six locked launch reactions with energy yields and autoignition temperatures. Reaction rate is a clamped function of temperature; combustor/gas-fuel-generator devices boost rate. 95% combustion efficiency per ignition cycle. |
| Phase change | Gases ↔ liquids ↔ solids gradually per phase diagram. Latent heat consumed on evaporation, released on condensation. Pipe damage thresholds locked: gas pipes rupture at frozen-content > 0.05 mol/L OR liquid stress > 100% (5000·L_liquid/V_network) OR ΔP > 60.795 MPa. Liquid pipes rupture at ΔP > 6.079 MPa or frozen > 0.05 mol/L. |
| Pipe networks | First-class atmospheres. Connected pipe segment graph = one atmosphere. Pumps / valves / regulators / filtration / condensation/evaporation chambers / purge/pressurant valves split networks. Per-tick flow proportional to ΔP for pressure-based devices and to dial setting for volume-based devices. |
| Room atmospheres | Connected sealed-volume graph = one atmosphere. Walls / floors / ceilings = sealed barriers; doors / windows / hatches / airlocks = stateful barriers. Adjacent sealed cells collapse into meta-atmospheres for kernel performance; partial-pressure HUD queries break apart on demand. |
| Suit life-support | EVA suit (10L, 6 slots) + Hardsuit (10L, 8 slots, IC10) per Stationeers; per-actor lung+helmet+suit nested atmospheres; canister-tank + waste-tank + filter slots; CO2 + N2 + per-gas filters. Breathing math: `inhaled_mol_per_tick = 0.0048 · BreathingRate · BreathingEfficiency`; humans exhale 50% of inhaled as CO2; min inhaled-gas partial pressure 16 kPa. Filter max waste-tank pressure 4052 kPa. Suit pressure tolerance 11-300 kPa survivable; 50-100 kPa comfortable. Suit temp tolerance -10 to 49 °C survivable; 18-21 °C comfortable. Origin gating per DR-036 + [[spec/origin-reaction-and-resource-model]]. |
| Planetary atmospheres | Per-world ambient: Earth (101 kPa, 0-40 °C, 75% N2 / 25% O2), Mars (2-3 kPa, -53 to 19 °C, 95% CO2), Moon/Mimas (vacuum), Europa (44-47 kPa, cold N2), Vulcan (24-56 kPa, hot oxidizing), Venus (239 kPa, 464 °C CO2). Each is an infinite reservoir with auto-correcting mole fractions. Modders add new worlds via data row. |
| Wind from ΔP | Pressure differentials drive gas flow at rate proportional to ΔP × interface area. Wind force on actors / items / debris at proportional impulse. Hooks into [[spec/full-collision-physics-plan]] M5.5-008 impulse-to-damage. |
| Door state machine | `closed_sealed` / `closed_unsealed` / `cycling_open` / `open` / `cycling_close` / `breached`. Airlocks = 2-door + 2-active-vent + logic-console assemblies. Emergency doors auto-close on detected ΔP. |
| Replay determinism | Kernel is CPU-deterministic; chunk/network update order pinned. Same seed + same actor inputs = byte-identical atmospherics event stream. Same authoritative server-replay model as DR-005 / DR-034. |
| Observation API | `cfctl observe --atmospheres`, `cfctl observe --pipe-networks`, `cfctl observe --rooms`, `cfctl observe --suits`. New `atmospherics` event category extending DR-002 schema. |
| Performance posture | Active-region scheduling per DR-036 model. Sleeping atmospheres are checksummed and skipped. Sealed-cell collapse for performance. Per-tick kernel budget per [[spec/prototype-roadmap#No-Compromise Performance Defaults]]. |
| Modding | New gas / new reaction / new device / new planet ambient are all data-driven schemas validated by `cargo run -p cf-mod -- validate content/`. Lua escape hatches for affliction logic. |

## What This Explicitly REJECTS

| Rejected Pattern | Why |
|---|---|
| Arcade-approximate atmosphere ("O2 timer ticks down") | Loses the systemic readability of Stationeers; breaks the suit life-support fantasy that DR-036 + DR-027 promise. |
| Different sim logic for client vs server | Replay/multiplayer determinism breaks; matches DR-034 same-binary policy. |
| Hidden chemistry without inspect/replay | Players can't learn what they can't see; AI can't trust what it can't query. |
| Real-life molar masses for Methane (16.04) when Stationeers uses gameplay-friendly 16 g/mol | Player intuition is built on Stationeers values; we keep gameplay-friendly molar masses at launch and reserve real-life values for a post-launch realism toggle. |
| Hardcoded `R = 8.314` (mol+m³ form) when Stationeers uses 8314.46 (kmol+L form) | We inherit Stationeers' convention to make modder math match the existing community. |
| Per-pixel atmosphere update (dilution toxin clouds at material-kernel resolution) | Performance + readability collapse. Atmospheres are room-cell / network / suit-scale; cloud visualization is presentation only. |
| Stationeers' double-welded-frame pipe-rupture immunity exploit | We keep the visual but charge the protection a small overhead so it doesn't trivialize design. |
| Subscription-funded gas/atmosphere content packs | Conflicts with DR-031. New gases / planets / reactions are free; expansions follow DR-031. |
| "Fudge gravity differently for gas stratification than for projectiles" | Gravity is universal per DR-038; atmospherics density layering reads `GravityField::sample(pos)` like everything else. |

## Why Not The Alternatives

| Alternative | Why Rejected |
|---|---|
| Approximate Barotrauma-style room atmosphere only | Misses the chemistry depth that the user explicitly committed to ("real chemistry and pressures and wind — just like in stationeers"). |
| Pure Stationeers (every device, every gas, every recipe) | Wrong genre; pulls focus from combat-base. We adopt Stationeers' grammar but the curated launch set of DR-036. |
| Simplified PV=nRT (no phase change, no combustion stoichiometry) | Half-measure; the user's promise covers fire spread, leak ignition, vacuum exposure — all of which require the full math. |
| Per-actor gas timer ("how many seconds of O2") with no real partial pressure | Breaks the suit-storage / canister / filter / waste-tank loop that gives Stationeers its replay-able life-support stories. |
| GPU-only kernel | Determinism is harder; cross-platform parity is harder; CI cost is higher. CPU deterministic kernel first; GPU stress test later (post-launch). |

## Evidence Trail

- Project owner direction (2026-05-06): "we want real chemistry and pressures and wind — just like in stationeers. make sure you fully understand stationeers how it works etc..."
- [[research-log/2026-05-06-atmospherics-and-chemistry-stationeers-research]] — 29+ source synthesis covering Atmosphere, PV=nRT, Volatiles, Oxygen, Carbon Dioxide, Pollutant, Hydrogen, Phase Change Guide, Furnace temperature/pressure math, EVA Suit, Hardsuit, Active Vent, Air, plus web-search context for room model + airlock guides + pipe device behavior.
- Cross-DR coherence:
  - DR-007 (terrain/material model) LEAN unchanged; the kernel-level coupling is in M5.6.
  - DR-036 (systemic material simulation) — atmospherics is the layer that talks to materials at the room/pipe boundary; both share the active-region kernel scheduling.
  - DR-027 (deep combat-base) — base modules (oxygen generators, scrubbers, vents, pumps, pipes, tanks, valves) are the core base-power layer.
  - DR-033 (full collision physics) — wind force is impulse routed through M5.5-008.
  - DR-022 (humanlike AI) — AI affordance tags now include hypoxia, combustible-atmosphere, breach proximity, suit-tank levels.
  - DR-002 (replay/event architecture) — new `atmospherics` event category; existing `material`/`reaction` categories from DR-036 keep their boundaries.
  - DR-005 / DR-013 / DR-034 / DR-035 — server-authoritative atmospherics state in multiplayer/MMO modes.
  - DR-038 (universal gravity and ballistics direction) — atmospherics density layering reads gravity from one source; gravity affects gas stratification per molar mass.
- [[spec/atmospherics-and-chemistry-model]] — 600-line canonical contract with locked equation of state, gas registry, combustion table, phase change rules, pipe network device list, room/door state machines, suit life-support math, planetary atmospheres, hazardous-composition detection, actor interactions, base modules, event family extensions, ATMOS-A acceptance tests.

## Risks And Mitigations

| Risk | Mitigation |
|---|---|
| Sim cost explodes in 50-100 player MMO shards | Active-region budgets; sleeping atmospheres; sealed-cell collapse; bounded per-shard concurrent atmospheres; perf gates at every milestone. T-PERF + T-MAT track. |
| Unfair invisible suffocation deaths | Hypoxia warnings (yellow at 16 kPa O2 / red at 12 kPa); hazard overlay UI; suit HUD readouts; replay cause chain (`atmospherics.breath_inhaled` events); grace windows. |
| Combustion runs nondeterministic across replay | CPU deterministic kernel; reaction-rate is pure function of temperature + present moles; chunk update order pinned; checksum events; first-divergence reports. |
| Modders break combustion balance | Schema validates min ratios + autoignition temperatures + energy yields are within sane ranges; balance review on M8.5 lab promotions. |
| Suit life-support is too punishing | Difficulty-tunable BreathingRate per Stationeers; configurable filter caps; helmet flush escape hatch; tutorial scenarios. |
| Pipe damage triggers cascading base failure | Insulated pipe class; double-welded frames stay (with overhead); player can hand-craft tank-grade reinforcement before pressurizing. |
| Gas registry inflation | Curated launch set; expansion gases go through M8.5 material lab; balance review required for any new ignition partner. |
| Stationeers-direct copy concerns | Math/equations are not copyrightable; gas list is industry-standard chemistry; specific values (specific heats, autoignition temps) are calibrated for game feel and don't replicate Stationeers source code. |
| Licensing contamination | Stationeers source is not public; we implement from wiki documentation only and from chemistry first principles; usage-ledger entries logged for any specific Stationeers wiki snippet quoted in spec/research notes. |
| Community-hosted MMO shards diverge on atmospherics | Server-authoritative; mod hash sync; atmospherics schema migration handlers. |

## Prototype / Validation Plan

| Test Pack | Milestone | What It Proves |
|---|---|---|
| ATMOS-A-01..ATMOS-A-04 | M5.9 (NEW) Atmospherics-Grade Kernel | PV=nRT correctness; mixing; pressure spike on heating; combustion stoichiometry. |
| ATMOS-A-05..ATMOS-A-06 | M5.9 + extended M7.5 | Pipe networks; regulators; filtration. |
| ATMOS-A-07..ATMOS-A-10 | M5.9 + M5.8 (origin resource) | Planetary ambient; suit life-support; filter mismatch failure mode; helmet flush. |
| ATMOS-A-11 | M5.9 + M5.6 | Phase change with cold pipe radiator; condensation/evaporation. |
| ATMOS-A-12 | M5.9 + M5.5 | Wind force on items at vacuum boundary. |
| ATMOS-A-13 | M5.9 + M7.5 + M8.5 | Photosynthesis (plant CO2 → O2 cycle). |
| ATMOS-A-14 | M5.9 | Furnace combustion math: 1 O2 + 2 H2 → exact temp/pressure spike per locked formula. |
| ATMOS-A-15 | M5.9 + M3 | Determinism replay across full atmospheric scenario for 10000+ ticks. |
| Atmospherics regression suite | T-MAT lifelong | All ATMOS-* slices keep passing as new gases / reactions / planets / devices land. |

## Cross-DR Anchors

| DR | Tie |
|---|---|
| DR-007 terrain/material model | Active-region kernel coupling at M5.6 boundary. |
| DR-027 combat-base scope | Base modules (generators, scrubbers, vents, pumps, pipes, tanks) are the core base-power layer. |
| DR-033 full collision physics | Wind force on entities = M5.5-008 impulse-to-damage routing. |
| DR-036 systemic material simulation | Material kernel and atmospherics kernel share active-region scheduling and run-bundle event categories. |
| DR-038 universal gravity and ballistics | Density-layering reads `GravityField::sample(pos)`; per-tick gas stratification proportional to local g × molar mass spread. |
| DR-022 humanlike AI bar | Hazard perception covers atmospheric hazards (hypoxia, combustible mix, breach, ambient extremes). |
| DR-002 replay/event architecture | New `atmospherics` event category. |
| DR-006 modding data model | Gas / reaction / device / planet schemas are first-class moddable surfaces. |
| DR-005 / DR-013 / DR-034 / DR-035 | Server-authoritative atmospherics state in multiplayer/MMO modes. |
| DR-031 content economy | Atmospheric content packs follow DR-031; community packs free; expansions per DR-031 monetization rules. |

## Revisit Trigger

- Atmospherics kernel cannot meet active-region perf budget at 60Hz/120Hz on Steam Deck floor after M5.9 evidence.
- Suit life-support math produces unfair invisible deaths the player cannot debug.
- Combustion stoichiometry runs nondeterministic across replay (any first-divergence is a hard halt).
- Stationeers-grade depth meaningfully pulls focus from combat-base genre per DR-027 such that the project owner reverts to coarser room/atmosphere model.
- A future "real-life molar masses" toggle for realism players that needs schema migration.

## Source Trail

- Project owner direction (2026-05-06).
- [[spec/atmospherics-and-chemistry-model]] — 600-line canonical contract.
- [[research-log/2026-05-06-atmospherics-and-chemistry-stationeers-research]] — 29+ source research synthesis.
- [[references/sources]] — Stationeers atmospherics research section.
- [[decisions/dr-002-replay-event-architecture]]
- [[decisions/dr-005-multiplayer-posture]]
- [[decisions/dr-006-modding-data-model]]
- [[decisions/dr-007-terrain-material-model]]
- [[decisions/dr-013-backend-service-scope]]
- [[decisions/dr-022-ai-humanlike-bar]]
- [[decisions/dr-027-combat-base-scope]]
- [[decisions/dr-031-content-economy-and-monetization-posture]]
- [[decisions/dr-033-full-collision-physics-direction]]
- [[decisions/dr-034-dedicated-server-application]]
- [[decisions/dr-035-persistent-mmo-architecture]]
- [[decisions/dr-036-systemic-material-simulation-direction]]
- [[decisions/dr-038-universal-gravity-and-ballistics-direction]]
- [[spec/origin-reaction-and-resource-model]]
- [[spec/full-collision-physics-plan]]
- [[spec/prototype-roadmap]]
- [[spec/native-implementation-backlog]]
