---
type: decision
id: DR-041
status: closed-direction
priority: P0
closed_at: 2026-05-06
revisit_trigger: "M8.6 cannot meet performance budget; AI miner doctrine cannot pass AI-MINE-A acceptance suite; server-authoritative resource ledger conflicts with MMO bandwidth; or extraction loop proves to overshadow combat-base focus per DR-027 such that the project owner reverts to a smaller post-launch slice."
---

← [[decisions/index|decision records]] · [[dashboards/decision-tracker|tracker]] · [[spec/mining-and-extraction-model|mining spec]] · [[decisions/dr-031-content-economy-and-monetization-posture|DR-031]] · [[decisions/dr-035-persistent-mmo-architecture|DR-035]] · [[decisions/dr-036-systemic-material-simulation-direction|DR-036]] · [[decisions/dr-039-celestial-bodies-and-worlds-direction|DR-039]]

# DR-041: Mining And Extraction Direction

> [!success] Status: CLOSED-DIRECTION (project owner committed 2026-05-06; chose **Full launch milestone (M8.6 right after M8.5)** option)

## Decision

Mining ships as a launch milestone (M8.6, sitting between M8.5 Material Lab and M9 Dedicated Server). The pipeline is sample → drill → extract → refine → smelt → use, with per-world ore deposits declared in the World catalog (DR-039), mining tools as equipment (DR-006), AI miner doctrine as part of M6.6 promoted to AI Environmental Competence, server-authoritative resource ledger per [[spec/persistent-mmo-architecture]], and extraction missions as mission director content (DR-017).

Launch ore set: 12 ores covering metal (iron, copper, nickel, cobalt, gold, platinum_group), nonmetal (silica, perchlorate), volatile (ice_volatiles, ice_oxite, ice_water), and radioactive (uranium). Modders extend.

Origin-gated mining tools: robots are vacuum-tolerant primary miners; humans need life-support overhead; androids are mid.

## What This Locks In

| Aspect | Commitment |
|---|---|
| Pipeline | sample → drill → extract → refine → smelt; per-step timer + replay events. |
| Ore registry | 12 launch ores; data-driven; modder-extensible. |
| Per-world deposits | World.ore_deposits feeds mining kernel; deterministic generation per scenario seed. |
| Equipment | Sampler, LightDigger, HeavyDrill, CoreDrill, RefiningStation, SmelterFurnace, EnrichmentReactor, OreCargoBay, ConveyorBelt — all data-driven role records. |
| Origin gating | Mining tools have `origin_compatibility`; AI bot picks emit `wrong_origin_for_mining_tool`. |
| AI miner doctrine | AI-MINE-A 8-test acceptance suite; integrated with M6.6 environmental competence. |
| Server-authoritative ledger | Per-shard ResourceLedger; audit-logged; anti-cheat enforced. |
| Mission integration | Mission manifest carries `MiningObjective` blocks; dynamic events tied to mining. |
| Replay | `mining` event category; sample → drill → extract → refine → smelt → trade event chain. |

## What This Explicitly REJECTS

- Mining as post-launch / DLC content (rejected because user committed launch milestone).
- Hand-coded ore tables outside `content/ores/`.
- Client-authoritative resource extraction (anti-cheat fails immediately).
- Mining that ignores the atmospheric / environmental hazard model (e.g., drilling volatiles in O2-rich room without combustion risk).

## Why Not The Alternatives

- **Lock direction now, implement post-launch as M13**: cuts a launch-day surface that the player + modder community will expect; puts the pipeline in unfunded territory.
- **Lock direction now, defer implementation completely**: weakens DR-031 economy story.

User chose **Full launch milestone (M8.6 right after M8.5)** explicitly.

## Cross-DR Anchors

- DR-006 modding data model — ores / tools / recipes are first-class moddable surfaces.
- DR-007 terrain/material model — material kernel handles in-pixel ore behavior.
- DR-017 mission generation strategy — mining missions extend manifest schema.
- DR-022 humanlike AI bar — AI miner doctrine.
- DR-027 combat-base scope — defenders' base may host RefiningStation + SmelterFurnace.
- DR-031 content economy — mining feeds the in-world economy without breaking premium-only monetization.
- DR-034, DR-035 — server-authoritative ledger.
- DR-036 systemic material simulation — ore-as-material entries.
- DR-039 celestial bodies — World.ore_deposits per world.
- DR-040 environmental conditions — AI miner reads EnvironmentSignal.

## Revisit Trigger

- M8.6 cannot meet performance budget on Steam Deck floor.
- AI miner doctrine cannot pass AI-MINE-A.
- Server-authoritative ledger conflicts with MMO bandwidth.
- Extraction loop overshadows combat-base focus per DR-027.

## Source Trail

- Project owner direction (2026-05-06).
- [[spec/mining-and-extraction-model]]
- [[research-log/2026-05-06-celestial-bodies-environments-mining-bunker-defence-design-intent]]
