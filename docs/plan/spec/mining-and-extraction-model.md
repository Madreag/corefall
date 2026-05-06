---
type: spec
status: design-intent-post-m1
authority: "Canonical contract for resource extraction: ore types, per-world deposit distribution, mining tools, sample → drill → extract → refine → smelt pipeline, mining missions, AI miner doctrine, server-authoritative resource ledger. Ships as launch milestone M8.6 (after M8.5 Material Lab so modders can author mining content)."
ready_when: "OreId registry exists; per-world ore_deposits resolve at scenario load; mining tool roles ship in equipment; sample → drill → extract → refine pipeline runs deterministically; mining mission objectives fire; AI miner doctrine passes AI-MINE-A; server-authoritative ledger replicates per [[spec/persistent-mmo-architecture]]."
feeds:
  - DR-002
  - DR-005
  - DR-006
  - DR-007
  - DR-011
  - DR-013
  - DR-014
  - DR-016
  - DR-017
  - DR-022
  - DR-027
  - DR-029
  - DR-031
  - DR-033
  - DR-034
  - DR-035
  - DR-036
  - DR-039
  - DR-040
  - DR-041
---

← [[index|vault home]] · [[spec/index|spec section]] · [[spec/celestial-bodies-and-worlds-model|worlds catalog]] · [[spec/environmental-conditions-model|environmental conditions]] · [[spec/equipment-loadout|equipment/loadout]] · [[spec/origin-reaction-and-resource-model|origin reaction/resource]] · [[spec/full-collision-physics-plan|full collision plan]] · [[spec/mission-director-slice-a|mission director]] · [[decisions/dr-041-mining-and-extraction-direction|DR-041]]

# Mining And Extraction Model

> [!summary] What this page is
> The contract for "what's in the dirt and how do you get it out". Inherits the worlds catalog (DR-039) for per-world ore deposits, the materials kernel (DR-036) for in-pixel ore behavior, the equipment system for mining tools, the chassis system for mining-spec mechs, the mission director for extraction contracts, the server architecture for authoritative resource ledger, and the origin model for mining-friendly origins (robots don't need oxygen on a vacuum belt; humans need full life support).
>
> Mining ships as **launch milestone M8.6** between M8.5 (Material Lab) and M9 (Dedicated Server). User-confirmed direction: full launch milestone, not deferred.

> [!warning] Authority boundary
> Captured 2026-05-06 as **design intent**. The pipeline shape (sample → drill → extract → refine → smelt) and the per-world deposit grammar are committed. Specific ore values, abundance tuning, and tool stats stay open until M8.6 prototype evidence backs them.

> [!important] Out of scope right now
> M0..M5.9 stay mining-config-only. M5.6 material kernel may pre-declare ore-as-material entries but the extraction pipeline lands at M8.6. Earlier milestones (M5 equipment, M5.6 materials, M7 mission director) carry placeholder hooks per [[spec/prototype-roadmap]].

## Why This Page Exists

Mining is intrinsically cross-cutting:

- **Materials** owns the in-pixel CA behavior of ore (does it fall, conduct heat, react with water).
- **Worlds** owns where ore lives (Mars has iron + perchlorate; belt asteroids have ice).
- **Equipment** owns the tools (drill heads, samplers, refining stations).
- **Chassis** owns mining-spec mechs (heavy drill arm, ore cargo bay).
- **Origin** owns who can mine where (robot in vacuum vs human in pressure suit).
- **Mission director** owns extraction contracts.
- **Server / MMO** owns the authoritative resource ledger.
- **Economy** owns ore→credit value.

If we don't lock the contract NOW, each subsystem will fork its own ore concept. This page is the joint surface.

## Principles (locked)

1. **Ore is data.** OreId is a stable string id. Each ore declares mass per unit, refining recipe, smelting recipe, hazard tags, market value baseline.
2. **Deposits live on worlds.** Per [[spec/celestial-bodies-and-worlds-model]] each world declares an `ore_deposits` table.
3. **Mining is a verb chain.** Sample → drill → extract → refine → smelt → use. Each step is a real player/AI action with replay events.
4. **Server-authoritative in MMO.** Per [[spec/persistent-mmo-architecture]] the resource ledger is server-owned; clients receive ledger deltas; anti-cheat enforces extraction limits.
5. **AI miners are first-class.** AI bots (commander-controlled or independent) prospect, drill, extract, return cargo. AI-MINE-A acceptance suite proves they don't dig in unsafe gas, don't carry cargo through breaches, don't strand themselves in low-power states.
6. **Mining missions are mission-director content.** Per [[spec/mission-director-slice-a]] missions can author "extract 1000 mol of ice volatiles from Phobos within 30 minutes".
7. **Origin gating.** Robots are good vacuum miners; androids are mid; humans need life-support overhead. Mining tools have origin compatibility per [[spec/origin-reaction-and-resource-model]].
8. **Modder-extensible.** New ores / new tools / new recipes are data rows. Schema validates.

## Ore Registry

```text
struct Ore {
    id: OreId,                                      // "iron", "silica", "ice_volatiles", "perchlorate", "platinum_group", ...
    display_name: String,
    classification: OreClass,                       // Metal | NonMetal | Volatile | Radioactive | Composite | Special
    mass_per_unit_kg: f32,                          // mining-pile unit mass
    bulk_density_kg_m3: f32,
    market_value_baseline: f32,                     // credits per kg at canonical market

    // Hazards / handling
    hazard_tags: Vec<HazardTag>,                    // FlammableDust | RadioactiveBeta | RadioactiveGamma | Toxic | Cryogenic | Explosive | StaticDischarge
    storage_constraints: StorageConstraints,        // Standard | InsulatedTank | LeadShielded | InertGasFilled | Pressurized

    // Pipeline
    refining_recipe: RefiningRecipeId,              // input ore → refined material per [[decisions/dr-036-systemic-material-simulation-direction]] launch material set
    smelting_recipe: Option<SmeltingRecipeId>,     // for metal ores; output = ingot

    // Provenance
    canonical: bool,
    package_source: PackageRef,
}
```

Launch ore set (12 ores; expansion via M8.5 material lab):

| OreId | Class | Mass kg/unit | Hazards | Pipeline |
|---|---|---:|---|---|
| `iron` | Metal | 1.0 | — | refine → iron_ore_concentrate → smelt → iron_ingot |
| `copper` | Metal | 1.0 | — | refine → copper_ore_concentrate → smelt → copper_ingot |
| `silica` | NonMetal | 0.8 | — | refine → silica_dust → smelt → glass / silicon |
| `ice_volatiles` | Volatile | 0.9 | Cryogenic | melt → liquid volatiles → electrolyze → H2 + Volatiles |
| `ice_oxite` | Volatile | 0.9 | Cryogenic | melt → liquid oxite → electrolyze → O2 |
| `ice_water` | Volatile | 1.0 | Cryogenic | melt → liquid water → distill → water + minerals |
| `nickel` | Metal | 1.1 | — | refine → nickel_concentrate → smelt → nickel_ingot |
| `cobalt` | Metal | 1.1 | — | refine → cobalt_concentrate → smelt → cobalt_ingot |
| `gold` | Metal | 1.9 | — | refine → gold_concentrate → smelt → gold_ingot |
| `uranium` | Radioactive | 1.4 | RadioactiveGamma | refine → uranium_concentrate → enrich (special) → fuel rod |
| `perchlorate` | NonMetal | 0.9 | Toxic | refine → perchlorate_salt → cleaner / oxidizer |
| `platinum_group` | Metal | 2.0 | — | refine → PG_concentrate → smelt → catalyst metals |

Modders can add ores via `content/ores/<id>.ore.ron`. Schema validates.

## Mining Tools And Equipment

Per [[spec/equipment-loadout]] role records, mining-related tools:

| Tool Class | Role | Notes |
|---|---|---|
| `Sampler` | Surface scan to detect ore presence + abundance | Hand-held; outputs scan event with ore mix + depth band. |
| `LightDigger` | Surface drill for soft material | Existing CCCP-style digger. |
| `HeavyDrill` | Sub-surface drill for hard rock | Tier-2; takes longer; needs power. |
| `CoreDrill` | DeepCrust extraction | Tier-3; mech-mounted; needs cooling. |
| `RefiningStation` | Stationary; converts ore → refined material | Base module; consumes power; emits waste gas (handled by atmospherics). |
| `SmelterFurnace` | Stationary; converts refined → ingot | Base module; couples with [[spec/atmospherics-and-chemistry-model]] combustion math (uses fuel + ore). |
| `EnrichmentReactor` | Stationary; uranium → fuel rod (special) | Tier-3 base module; radiation hazard; AI affordance avoid-unless-shielded. |
| `OreCargoBay` | Mech / vehicle / dropship slot | Holds extracted ore; mass affects mobility. |
| `ConveyorBelt` | Base module | Routes raw ore from drill → refining → smelter. |

All tools have `origin_compatibility` per [[spec/origin-reaction-and-resource-model]]. Robots can operate every tool; humans need cooled / pressurized variants for vacuum/hot environments; androids are mid.

## Pipeline (Sample → Drill → Extract → Refine → Smelt → Use)

```
[1] Sample
    Actor uses Sampler at position P. Reads world.ore_deposits at cell(P).
    Output: ScanResult { surface_visible: [(ore, abundance)], subsurface_likely: [(ore, depth_band, abundance)] }
    Replay event: mining.sampled

[2] Drill / Extract
    Actor uses Drill on cell(P) at depth_band D.
    Material kernel removes ore-as-material from chunk; spawns ore-as-rigid-body or pickup-able pile.
    Cargo capacity check: actor + chassis + cargo bay must hold mass.
    Replay event: mining.drilled, mining.extracted

[3] Refine
    Stationary RefiningStation takes raw ore as input.
    Per-ore RefiningRecipe: input ore (mol/kg) + power + time → output refined_material + waste gas (via atmospherics).
    Replay event: mining.refined

[4] Smelt
    Stationary SmelterFurnace takes refined_material as input.
    Couples with [[spec/atmospherics-and-chemistry-model]] combustion math: fuel + O2 + ore → ingot + CO2 / Pollutant byproducts at locked temperature/pressure.
    Replay event: mining.smelted

[5] Use
    Ingot enters equipment crafting (M8 scenario editor + post-launch crafting bench).
    Replay event: economy.material_consumed
```

Each step has a per-tick or per-action timer; cancellation is safe (returns input to inventory). Sleeping when no actor/AI is present.

## Per-World Deposit Generation

At scenario load, per-world `ore_deposits` table seeds the material kernel chunks. Generation rule:

- For each ore entry with `(abundance, depth_band, distribution)`:
  - Distribute ore-as-material into chunks at the named depth band per the distribution shape:
    - `Uniform`: even spread across all chunks at depth_band
    - `Veined`: connected linear streaks; favored for metal ores
    - `Pocketed`: clustered pockets; favored for ice/volatile ores
    - `Streak`: surface streaks across the world (favored for `perchlorate` on Mars)
- Total ore mass per scenario is bounded; server-authoritative cap per shard per [[spec/persistent-mmo-architecture]].

Deposit determinism: same world id + same scenario seed = byte-identical deposit map across replay.

## Resource Ledger (Server-Authoritative)

Per [[spec/persistent-mmo-architecture]]:

```text
struct ResourceLedger {
    shard_id: ShardId,
    per_actor: HashMap<ActorId, Inventory>,
    per_team: HashMap<TeamId, TeamStockpile>,
    per_world: HashMap<WorldId, WorldDepositState>,
    market: MarketState,
    audit_log: Vec<LedgerEvent>,           // all extractions, refinements, trades for anti-cheat replay
}
```

- All extraction events are server-validated against world deposit caps.
- Trade is server-mediated; clients propose, server commits.
- Audit log replicates as part of run-bundle for replay scrub + dispute resolution.

## Mining Missions

Per [[spec/mission-director-slice-a]] mission manifest:

```text
mission.objectives.add MiningObjective {
    target_world: WorldRef,
    target_ore: OreId,
    target_amount_kg: f32,
    delivery_zone: ZoneRef,
    deadline_ticks: Option<u64>,
    bonus_objectives: [...],               // "no civilian casualties", "extract while under fire"
}
```

Mission types:

- **Survey**: scan a world; report findings; no extraction. Tutorial-friendly.
- **Quick Strike**: drop in, drill X, extract Y, dropship out. PvE-friendly.
- **Holdout Extraction**: hold a position on a hostile world while a slow drill runs; common bunker-defence inverse. PvE/PvP-friendly.
- **Salvage**: extract from a wrecked station / mech graveyard; bonus rare ores.
- **Black Market Dump**: extract uranium-class ore; ship to a contested drop zone.

Mission director can author dynamic events tied to mining: "you've drilled 500 kg; defenders just spawned"; "the storm is collapsing the mine in 5 minutes; extract everything".

## AI Miner Doctrine (AI-MINE-A acceptance suite)

Per DR-022 + M6.6 promoted to AI Environmental Competence:

| AI Need | Reasoning |
|---|---|
| Survey before drilling | Prevent wasted time on empty cells; prefer high-abundance pockets. |
| Avoid hazardous deposits without protection | Don't drill uranium without lead shielding; don't tap volatiles in O2-rich room without atmospheric venting. |
| Cargo capacity awareness | Don't extract beyond actor + bay capacity; route to refinery before overloading. |
| Power awareness | Don't start a slow drill if `power` resource won't last per [[spec/origin-reaction-and-resource-model]]. |
| Environmental awareness | Don't mine on Vulcan surface unprotected; don't dig in active dust storm. |
| Squad coordination | Sampler + driller pairing; haul-back drone routing. |
| Defense awareness | If hostile presence detected, retreat with cargo; don't get pinned at drill site. |

AI-MINE-A acceptance:

| Test | Pass condition |
|---|---|
| AI-MINE-A-01 | AI samples → drills → extracts → returns 100 kg iron from Mars under 5 minutes. |
| AI-MINE-A-02 | AI refuses to drill volatiles in atmosphere with O2 > 5% AND temp > 280 K. Reason: `combustible_atmosphere`. |
| AI-MINE-A-03 | AI miner runs out of power → returns to refinery before continuing. |
| AI-MINE-A-04 | AI miner under fire → drops cargo and retreats per doctrine. |
| AI-MINE-A-05 | AI prioritizes high-abundance veins over uniform spread when sampler reports both. |
| AI-MINE-A-06 | Robot AI mines on vacuum belt without life-support warning; human AI refuses without sealed suit + tank. |
| AI-MINE-A-07 | Mining mission objective triggers AI commander to allocate squads. |
| AI-MINE-A-08 | Determinism replay across full mining mission. |

## Origin Gating

Per [[spec/origin-reaction-and-resource-model]]:

| Origin | Mining strengths | Mining weaknesses |
|---|---|---|
| Human | Versatile; can use any tool with proper life-support overhead | Needs O2 + thermal + radiation suit on hostile worlds; bleed/concuss risk; food/medical resource cost |
| Android | Mid-tier; organic side handles delicate ops; synthetic side handles harsh tools | Battery depletion limits long shifts; module overheat in foundry-adjacent mining |
| Robot | Vacuum-tolerant; no oxygen overhead; sustained heat under thermal envelope | Power resource is gating; cannot eat / refill via medkit; coolant/oil leaks if frame breached |

Mining tools' `origin_compatibility` field gates assignment; AI bot picks emit `wrong_origin_for_mining_tool` reason label when mismatched.

## Run-Bundle Event Family Extensions

`mining` event category. Locked per DR-002 + DR-041.

| Event Type | Required Fields |
|---|---|
| `mining.sampled` | actor_id, pos, scan_result, parent_event_id |
| `mining.drilled` | actor_id, pos, depth_band, ore_id, mass_extracted_kg, duration_ticks, parent_event_id |
| `mining.extracted` | actor_id, ore_id, mass_kg, container_id, parent_event_id |
| `mining.refined` | refining_station_id, ore_id, input_kg, output_material_id, output_kg, waste_gas, energy_cost, parent_event_id |
| `mining.smelted` | smelter_id, refined_material_id, input_kg, output_ingot_id, output_kg, fuel_consumed, parent_event_id |
| `mining.cargo_overflow` | actor_id, container_id, attempted_kg, available_kg | (rejection event) |
| `mining.deposit_exhausted` | world_id, cell, ore_id, parent_event_id |
| `mining.market_trade` | seller_id, buyer_id, ore_or_material_id, kg, credits, parent_event_id | (server-authoritative) |
| `mining.theft_detected` | shard_id, suspect_actor_id, evidence_hash, parent_event_id | (anti-cheat) |

## Modding Contract

- Add a new ore: data row in `content/ores/<id>.ore.ron` with full schema. Validates via `cf-mod`.
- Add a new mining tool: data row in `content/equipment/` with role record + origin_compatibility.
- Add a new refining/smelting recipe: data row in `content/recipes/`.
- Add a new mining mission objective type: extends mission director manifest schema.

## Performance Posture

- Deposit map is computed once at scenario load; held immutable per scenario.
- Per-tick mining state is sleeping unless an actor/AI is actively at a drill/refining/smelting station.
- Server-authoritative ledger updates are event-driven, not per-tick.

## Out Of Scope (during M0..M8.5)

- M0..M5.9: scenario manifest may carry `mining_enabled` flag (placeholder); behavior is no-op.
- M5.6 (Material Kernel): may pre-register ore-as-material entries (silica, iron, nickel, cobalt, gold, perchlorate, platinum_group) so M8.6 can extract them without engine churn. Pre-registered entries are inert until M8.6 promotes them.
- M5.10 (Environmental Conditions Aggregation): EnvironmentSignal includes `world.ore_deposits` summary so AI prospectors can plan; full mining pipeline waits for M8.6.
- M7 (Mission Director): mission manifest schema may pre-declare MiningObjective shape; runtime fires no-op until M8.6.
- M8 (Scenario Editor): editor may pre-stub a mining objectives panel; full editing waits for M8.6.
- M8.5 (Material Lab): material lab can pre-promote ore materials via the same gate. Mining recipes (refining/smelting) are added in M8.6.
- M8.6 lands the full pipeline + AI miner + mission integration + ledger.

## Source Trail

- [[spec/celestial-bodies-and-worlds-model]]
- [[spec/environmental-conditions-model]]
- [[spec/atmospherics-and-chemistry-model]]
- [[spec/equipment-loadout]]
- [[spec/origin-reaction-and-resource-model]]
- [[spec/mission-director-slice-a]]
- [[spec/persistent-mmo-architecture]]
- [[spec/server-app-architecture]]
- [[references/prototype-run-bundle-schema]]
- [[decisions/dr-031-content-economy-and-monetization-posture]]
- [[decisions/dr-035-persistent-mmo-architecture]]
- [[decisions/dr-036-systemic-material-simulation-direction]]
- [[decisions/dr-037-stationeers-grade-atmospherics-direction]]
- [[decisions/dr-039-celestial-bodies-and-worlds-direction]]
- [[decisions/dr-041-mining-and-extraction-direction]]
- [[research-log/2026-05-06-celestial-bodies-environments-mining-bunker-defence-design-intent]]

## Change Log

- 2026-05-06: Captured during M1 from user-supplied design intent ("the engine to also support mining resources, etc... something we will add later on in the roadmap"). User chose **full launch milestone (M8.6)** over deferred / post-launch alternatives. Status: `design-intent-post-m1`. Lands at M8.6 between M8.5 (Material Lab) and M9 (Dedicated Server). Pre-registers hooks at M5.6 + M5.10 + M7 + M8 + M8.5 so M8.6 lands cleanly.
