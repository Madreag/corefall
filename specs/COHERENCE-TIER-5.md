# Coherence Tier 5 — Grand-Strategy Economy + AWAW-Inspired Strategic Layer (Opt-In Per Server)

**Status:** `active` — strategic-depth pillar; can run AFTER Tier 1-4 OR in parallel with Tier 3 / 4 (no cross-dependency on Tier 2's M7 split is required as long as M7 family is treated as one closure unit)
**Prerequisite:** Tier 1 + Tier 2 merged (Tier 5 references M7.1, M7.2, M11.6, M11.7 which Tier 2 creates; Tier 5 also references the M9.10 settings hierarchy that Tier 1 doesn't touch but assumes is stable). If Tier 2 has NOT merged, swap M11.7 → M11.5 and M7.1 → M7 in all edits.
**Estimated effort:** AI-scale 2-3 hours (single large PR, 8 commits) — would be 4-6 person-weeks for a human writing by hand
**Output:** 1 PR titled `specs: tier-5 grand-strategy economy + AWAW-inspired strategic layer (opt-in per server)`

---

## Why this tier exists

Corefall's current economy is **one-time-cost**: build a thing, it exists forever. No grand-strategy game works that way — every canonical title (A World at War / Hearts of Iron / Stellaris / Crusader Kings / Civ / RimWorld / ONI) has a **periodic upkeep cycle** that creates the hardest strategic tension: a unit you can't afford to keep is worse than one you never built.

Tier 5 fills this gap. Per the user's confirmed design knobs:

1. **Cycle period** — per in-game day (upkeep tick) AND per scenario completion (reconciliation) — hybrid
2. **Unit of upkeep** — BP currency shell + multi-resource consumption (power + parts + food + fuel) — hybrid
3. **Pool scope** — faction-wide HQ + base-local with supply line transfer — hybrid
4. **AI parity** — AI factions follow the same upkeep + choice + goal rules as players — full parity

Plus the user's explicit design requirement: **"players will be able to choose what kind of features they want enabled on their servers, AWAW rulesets are among the choices."**

This means every grand-strategy + AWAW-inspired mechanic in Tier 5 is **opt-in via server config** (per M9.10's 7-tier settings hierarchy). Server admins can ship vanilla Corefall (everything off), classic-Corefall-with-upkeep (M7.3 on, AWAW off), or full-AWAW-grade-grand-strategy (all toggles on).

---

## Goals — 8 edits across 3 NEW milestones + 5 existing extensions

| # | Edit | Type | Scope | Default |
|---|------|------|-------|---------|
| 5.1 | **NEW M7.3 — Upkeep Economy** | new milestone | Per-cycle costs (BP + power + parts + food + fuel); bankruptcy cascade | Opt-in (default OFF; PvE Survival recommends ON) |
| 5.2 | **NEW M7.4 — Strategy Phase + Goals** | new milestone | Per-cycle stance / focus / priority choice; 1-3 goals with stake | Opt-in (default OFF) |
| 5.3 | **NEW M7.1.5 — Inter-Faction Intelligence** | new milestone | 4 subsystems: codebreaking / spy rings / covert ops / counter-intelligence (AWAW Rule 44-48) | Opt-in (default OFF) |
| 5.4 | Add Code-Name Research Secrecy to M7.8 | modify | AWAW Rule 41.5 — public dice, hidden project intent | Opt-in (default OFF) |
| 5.5 | Add Faction Resistance Levels to M11.7 | modify | AWAW Rule 60 — graduated collapse (-1 / -2 / -3) | Opt-in (default OFF) |
| 5.6 | Add 9-phase Strategic Campaign Turn to M7 | modify | AWAW Rule 8 — Research → Diplomacy → DOW → Movement → Combat → Post-combat → Construction → Redeployment | Opt-in (default OFF) |
| 5.7 | Add Industrial Center Evacuation to M11.5 | modify | AWAW Rule 37 — lose territory but keep IC via strategic redeployment | Opt-in (default OFF) |
| 5.8 | Add AWAW Ruleset Toggle Tree to M9.10 | modify | Comprehensive opt-in server config: per-feature flags + 4 presets (vanilla / classic_upkeep / awaw_lite / awaw_full) | (M9.10 config surface itself; not opt-in) |

**Active spec count: 43 → 46** (Tier 1-4 baseline + M7.3 + M7.4 + M7.1.5)

---

## Order of operations

```
┌─────────────────────────────────────────────────────────────┐
│ Edit 5.1 — NEW M7.3 (Upkeep Economy)                        │
│ Edit 5.2 — NEW M7.4 (Strategy Phase + Goals; depends on 5.1)│
│ Edit 5.3 — NEW M7.1.5 (Inter-Faction Intelligence)           │
└─────────────────────────────────────────────────────────────┘
                            │
        ┌───────────────────┼───────────────────┐
        ▼                   ▼                   ▼
┌────────────────┐ ┌────────────────┐ ┌────────────────┐
│ Edit 5.4 — M7.8 │ │ Edit 5.5 — M11.7│ │ Edit 5.6 — M7   │
│ code names     │ │ resistance lvls │ │ turn sequence  │
└────────────────┘ └────────────────┘ └────────────────┘
        │                   │                   │
        └───────────┬───────┴───────────────────┘
                    ▼
            ┌────────────────┐
            │ Edit 5.7 — M11.5│
            │ IC evacuation  │
            └────────────────┘
                    │
                    ▼
            ┌────────────────┐
            │ Edit 5.8 — M9.10│
            │ ruleset toggles│
            └────────────────┘
```

Edits 5.1-5.3 are independent; 5.4-5.7 are independent modifications; 5.8 wires everything as opt-in server config.

---

## Working agreement

Every Tier 5 edit ships as **opt-in**. The default server config (per `server.ron.template-vanilla`) has all Tier 5 toggles set to `false`. Players who want grand-strategy depth flip the toggles in their server config; players who want classic Corefall ignore them entirely.

Per M9.10's 7-tier settings hierarchy:
- Engine defaults: all Tier 5 features OFF
- Server admins flip them per-server via `server.ron`
- Scenario configs can lock specific toggles
- Players cannot override server-locked Tier 5 settings

---

## Edit 5.1 — Create M7.3 — Upkeep Economy

### Problem

Corefall's current economy is single-payment. Build a turret → it exists forever (no maintenance). Build a base → it never costs anything ongoing. Hire 4 squad members → no upkeep. This creates a one-shot game design — there's no resource-pressure decision space.

Real grand-strategy games (A World at War, HoI4, Stellaris, Civ, ONI, RimWorld) all have **periodic upkeep cycles** that force ongoing strategic decisions:

- Standing army vs. raw resources — keep N units = N resources/cycle drained
- Build-vs-maintain tradeoff — every new factory adds future cost
- Logistics matters — sprawling supply lines cost more than compact ones
- Death spiral — running out of resources triggers forced demobilization → vulnerability cascade

Without this layer, Corefall's M7 campaign + M11.5 PvE Survival + M12 MMO are all economically static. Players never face the hardest strategic decision in any grand-strategy game: which assets to retire so others can survive.

### Fix

Create **M7.3 — Upkeep Economy** as a new milestone that ships the periodic upkeep tick + bankruptcy cascade. Defaults are OFF; server admins enable via `server.ron`.

### Files to modify

| File | Action |
|---|---|
| `specs/active/M7.3.md` | **CREATE** |
| `README.md` | **MODIFY** (add to BP7; update active spec count 43 → 44) |

### Step 1: Create `specs/active/M7.3.md`

```markdown
# M7.3 — Upkeep Economy (Grand-Strategy Foundation; Opt-In)

## Status

`active`

## Intent

**M7.3 is the periodic-upkeep economy foundation milestone** — the per-cycle drain that turns Corefall's one-shot economy into a grand-strategy resource-pressure system. After M7.3, every actor, vehicle, building, supply route, research project, and diplomatic asset costs resources per cycle to maintain. Resources flow from faction-wide HQ pool through supply lines to per-base local pools. Running out triggers a bankruptcy cascade — forced demobilization → building shutdown → faction collapse.

**M7.3 is opt-in per server.** Default `server.ron` has `grand_strategy.enable_upkeep_economy=false`; admins who want grand-strategy depth flip it on. PvE Survival default templates set it ON; PvP arena defaults set it OFF.

M7.3 promise: **"every unit and building costs resources per cycle; running out forces hard choices — which assets to keep, which to retire, which die in bankruptcy."**

## Player-facing behavior

### Cycle period — hybrid (per-day upkeep + per-scenario reconciliation)

Per the user's confirmed design knob: **upkeep ticks per in-game day (continuous pressure); reconciliation happens per scenario completion (formal accounting)**.

**Per in-game day (upkeep tick):**
- Fires at 24:00 in-game time (configurable via M7.7 day/night cycle)
- All faction-owned assets drain their upkeep from the HQ pool (after supply-line transfer)
- Bankruptcy cascade triggered if HQ pool insufficient
- Replay event: `upkeep.cycle_started`, `upkeep.cycle_completed`

**Per scenario completion (reconciliation tick):**
- Fires when scenario ends (mission resolved + outcome captured)
- Faction-wide accounting: total in/out per resource type across scenario
- Carries deficit forward to next scenario (per M11.5 persistence)
- Replay event: `upkeep.scenario_reconciliation_completed`

### Multi-resource upkeep (BP + power + parts + food + fuel) — hybrid

Per the user's confirmed design knob: **single BP currency shell + multi-resource consumption**.

Each asset declares a `UpkeepProfile`:

```rust
pub struct UpkeepProfile {
    pub bp_per_cycle: f32,           // build points (canonical Corefall currency)
    pub power_kw_continuous: f32,    // electrical draw (per M7.6 power kernel)
    pub parts_per_cycle: f32,        // mechanical parts (M7.8 fabrication; iron + electronics)
    pub food_per_cycle: f32,         // caloric for biological actors (per M5.7 affliction.hunger)
    pub fuel_per_cycle: f32,         // liquid fuel (vehicles only; per M5.9 fuel types)
    pub oxygen_per_cycle: f32,       // breathing tanks (per M5.9 gas tanks; biological actors in vacuum)
    pub coolant_per_cycle: f32,      // robot/chassis coolant (M5.8 robots only)
}
```

Per-resource shortage triggers DIFFERENT cascade effects (NOT uniform "all stop"):

| Resource short | Effect |
|---|---|
| `bp` | All non-essential construction paused; mission rewards reduced 50% |
| `power` | M7.6 brown-out cascade — modules auto-shed per priority band |
| `parts` | Repairs paused; tool durability degrades faster |
| `food` | M5.7 affliction.hunger applied to biological actors; speed × 0.8 |
| `fuel` | Vehicles immobilized; aircraft grounded |
| `oxygen` | M5.7 affliction.hypoxic applied to biological actors in vacuum |
| `coolant` | M2.5 fluid.reservoir_empty + affliction.overheating to robots |

### Per-cycle upkeep table (locked baseline)

| Asset type | BP/day | Power kW | Parts | Food | Fuel | O2 | Coolant |
|---|---|---|---|---|---|---|---|
| **Actor — Human soldier** | 1 | 0 | 0.05 | 1 ration | 0 | 1 tank/30 days vacuum | 0 |
| **Actor — Android soldier** | 1.5 | 0.1 | 0.05 | 0.5 ration | 0 | 0.5 tank/30 days | 0 |
| **Actor — Robot soldier** | 2 | 0.2 | 0.1 | 0 | 0 | 0 | 1 unit/10 days |
| **Vehicle — Jeep** | 3 | 0.05 | 0.2 | 0 | 1 fuel unit | 0 | 0 |
| **Vehicle — Tank/Mech** | 5 | 0.5 | 0.5 | 0 | 3 fuel units | 0 | 1 unit/10 days |
| **Vehicle — Dropship** | 8 | 0 | 0.5 | 0 | 5 fuel units | 0 | 0 |
| **Vehicle — Rocket** | 15 | 0 | 1.0 | 0 | 10 fuel units | 0 | 0 |
| **Building — Factory** | 2 | 5 | 0.1 | 0 | 0 | 0 | 0 |
| **Building — Smelter** | 3 | 10 | 0.2 | 0 | 0 | 0 | 0 |
| **Building — Power generator** | 1 | 0 | 0.5 | 0 | 0-5 fuel | 0 | 0 |
| **Building — Turret (active)** | 1 | 0.5 | 0.1 | 0 | 0 | 0 | 0 |
| **Building — Turret (passive)** | 0.5 | 0.05 | 0.05 | 0 | 0 | 0 | 0 |
| **Building — Bunker wall** | 0.1 | 0 | 0.01 | 0 | 0 | 0 | 0 |
| **Building — Workbench** | 0.5 | 0.5 | 0.02 | 0 | 0 | 0 | 0 |
| **Building — Reactor (M7.6)** | 5 | 0 (produces) | 0.5 | 0 | 0 | 0 | 5 units/day |
| **Supply route (per hex per day)** | 1 | 0 | 0.05 | 0 | 0.5 fuel | 0 | 0 |
| **Research project (active)** | 3 | 1 | 0 | 0 | 0 | 0 | 0 |
| **Diplomatic asset (spy ring; M7.1.5)** | 2 | 0 | 0 | 0 | 0 | 0 | 0 |
| **Diplomatic asset (ambassador)** | 1 | 0 | 0 | 0.5 ration | 0 | 0 | 0 |

**Per-race upkeep modifiers** (per M5.10 race-env matrix from Tier 1):

| Race | BP modifier | Power modifier | Food modifier | Notes |
|---|---|---|---|---|
| Human | 1.0× | 1.0× | 1.0× | Baseline |
| Robot | 1.2× | 1.5× | 0× | Higher BP/power; no food |
| Android | 1.1× | 1.2× | 0.5× | Hybrid; less food |
| Crystalline | 0.8× | 0.7× | 0× | Solar-self-sufficient; cheap |
| Insectoid | 0.7× | 0.5× | 0.8× | Hive-efficient |
| Photosynthetic | 0.6× | 0.3× | 0× sunlight | Self-feeding via photosynthesis |
| Aqueous | 1.0× | 1.0× | 0.5× water | Needs flooded habitat |
| Methane breather | 1.1× | 1.0× | 0.5× methane | Cryogenic native; needs CH4 |
| Heavy biomech | 1.5× | 1.2× | 1.5× bio-fluid | Expensive but tough |
| Powered organic | 1.3× | 1.5× | 1.0× | Hybrid + cybernetics |

### Pool scope — hybrid (faction-wide HQ + base-local with supply line transfer)

Per the user's confirmed design knob: **per-faction shared HQ pool + per-base local pool + supply line transfers**.

```rust
pub struct FactionEconomy {
    pub hq_pool: ResourcePool,                       // central faction reserve
    pub bases: BTreeMap<BaseId, BaseEconomy>,        // per-base local pools
    pub supply_routes: Vec<SupplyRoute>,             // connections (per M11.6 transport)
}

pub struct BaseEconomy {
    pub local_pool: ResourcePool,                    // base's own stockpile
    pub connected_to_hq: bool,                       // supply line intact?
    pub max_local_capacity: ResourcePool,            // storage limit
    pub upkeep_demand: ResourcePool,                 // sum of all asset upkeep at this base
}

pub struct SupplyRoute {
    pub from: BaseId,
    pub to: BaseId,
    pub bp_per_cycle: f32,                           // supply route upkeep itself
    pub throughput_per_cycle: f32,                   // max units transferred per day
    pub disrupted: bool,                             // enemy interdiction (M7.1.5 intel; M12 PvP)
}
```

**Per-cycle resource flow:**

```text
1. HQ pool accumulates faction-wide income (mining + research + diplomacy + scenario rewards)
2. Each base computes its upkeep_demand (sum of asset UpkeepProfiles at that base)
3. For each base:
   a. If local_pool >= upkeep_demand → drain local_pool; no transfer needed
   b. If local_pool < upkeep_demand AND supply line intact → request transfer from HQ
   c. Transfer = min(supply_route.throughput, hq_pool.available, deficit)
   d. If deficit not covered → base enters local bankruptcy state
4. Faction-wide bankruptcy check: hq_pool < 0 OR all bases bankrupt → faction enters bankruptcy cascade
```

**Supply line disruption:**

- Enemy can interdict supply routes (M12 PvP / M7.1.5 covert ops / M11.5 raids)
- Disrupted route = 0 throughput until repaired
- Player can re-route via alternate paths (per M11.6 transport network)
- Per-route HUD pip shows disrupted state

### Bankruptcy cascade — graduated faction collapse

Per AWAW Rule 35.53 (BRP deficits) + AWAW Rule 60 (Russian Resistance Level cascade). Cumulative deficit triggers escalating consequences:

| Deficit duration | Effect |
|---|---|
| **Day 1 deficit** | All bankrupt-base assets flagged: "low morale" (-20% combat); buildings flagged "low maintenance" (-10% output) |
| **Day 3 deficit** | Forced demobilization: lowest-priority assets convert to "reserve" state (no upkeep, can't act); player chooses priority via M7.4 OR auto via storyteller |
| **Day 7 deficit** | Buildings start auto-shutting: factories pause, smelters stop; per-asset reserve duration timer |
| **Day 14 deficit** | Faction enters Resistance Level -1 (pairs with Edit 5.5: M11.7 graduated collapse) |
| **Day 30 deficit** | Faction collapses or surrenders per M11.7 endgame storyteller |

**Rescue mechanisms** (player can break the cascade):

- **Emergency mobilization** — one-time BP injection at cost of -2 morale + 1 cycle commitment
- **Allied BP grant** — friendly faction transfers BP (per AWAW Rule 40 BRP grants; M11+ co-op)
- **Faction-wide austerity** — auto-applies Defensive stance + Survival focus (per M7.4); -50% combat readiness BUT -50% upkeep
- **Sell strategic asset** — convert building/vehicle to BP (75% recovery; one-cycle delay)
- **Trade with neutral** — sell ore/material/research to NPC traders for BP (per M11.5 trader events)

### Income sources

Per cycle, factions accumulate resources from:

| Source | BP yield | Other |
|---|---|---|
| **Mining (M8.6)** | per ore extracted; T1 = 1 BP/unit; T4 = 50 BP/unit | + raw ore for fabrication |
| **Research breakthrough (M7.8)** | one-time bonus 50-500 BP per tier unlock | + recipe unlocked |
| **Diplomatic alliance (M7.1)** | recurring BP grant per cycle from allied faction | + DP relationship boost |
| **Scenario rewards (M2.5/M7)** | 100-1000 BP per scenario won | + loot + reputation |
| **Trade (M11.5 trader event)** | variable; price differential per world | |
| **Faction territory (M12)** | per-territory BP/cycle (passive income) | + faction reputation |
| **Production buildings (M7.6 + M7.8)** | factory output sold to faction OR consumed | |

### Configurable upkeep multipliers (per M9.10 server config)

```ron
GrandStrategyUpkeepConfig (
    enable_upkeep_economy: false,                   // master switch
    cycle_period: "in_game_day",                    // "in_game_day" | "scenario_completion" | "campaign_turn"
    upkeep_multiplier: 1.0,                         // scales ALL upkeep
    bp_upkeep_multiplier: 1.0,
    power_upkeep_multiplier: 1.0,
    parts_upkeep_multiplier: 1.0,
    food_upkeep_multiplier: 1.0,
    fuel_upkeep_multiplier: 1.0,
    enable_bankruptcy_cascade: true,
    bankruptcy_day_1_threshold: 1,                  // days deficit before Day 1 effects fire
    bankruptcy_day_3_threshold: 3,
    bankruptcy_day_7_threshold: 7,
    bankruptcy_day_14_threshold: 14,
    bankruptcy_day_30_threshold: 30,
    enable_emergency_mobilization: true,
    enable_allied_bp_grants: true,                  // requires M11+ co-op
    enable_austerity_auto_apply: true,
    enable_supply_line_disruption: true,            // M12 PvP / M7.1.5 intel
    default_starting_hq_pool_bp: 1000,
    default_starting_hq_pool_parts: 100,
    default_starting_hq_pool_food: 100,
)
```

### Per-cycle replay events (locked v0.1)

- `upkeep.cycle_started { tick, cycle_period }`
- `upkeep.asset_drained { asset_id, resources_drained, source_pool: base|hq }`
- `upkeep.base_transfer_requested { base_id, deficit, transfer_amount }`
- `upkeep.supply_line_disrupted { route_id, cause: enemy_action|terrain_damage|repair_needed }`
- `upkeep.base_bankruptcy { base_id, cause: insufficient_local_pool|supply_line_disrupted }`
- `upkeep.faction_bankruptcy_day_n { faction_id, deficit_days, effects_applied }`
- `upkeep.forced_demobilization { faction_id, asset_id, reason }`
- `upkeep.emergency_mobilization { faction_id, bp_injected, morale_cost }`
- `upkeep.allied_grant_sent { from_faction, to_faction, bp_amount }`
- `upkeep.austerity_applied { faction_id, stance, duration }`
- `upkeep.cycle_completed { tick, hq_pool_state, bases_state }`
- `upkeep.scenario_reconciliation_completed { faction_id, deficit_carry_forward }`

## Crates / modules touched

| Crate | Status | What |
|---|---|---|
| `cf-upkeep` | NEW (deep) | Per-cycle upkeep ticker + bankruptcy cascade + rescue mechanisms |
| `cf-upkeep::profile` | NEW | UpkeepProfile registry per asset type |
| `cf-upkeep::economy` | NEW | FactionEconomy + BaseEconomy + ResourcePool + SupplyRoute |
| `cf-upkeep::cascade` | NEW | Bankruptcy day-1/3/7/14/30 state machine |
| `cf-upkeep::rescue` | NEW | Emergency mobilization + allied grants + austerity |
| `cf-faction` | MODIFY | Add `economy: FactionEconomy` to Faction state (per Tier 2 M7.1) |
| `cf-replay` | MODIFY | `upkeep.*` event category (NEW) added to M3A taxonomy |
| `cf-config` | MODIFY | `GrandStrategyUpkeepConfig` integrated into 7-tier hierarchy (per M9.10) |
| `cf-storyteller` | MODIFY | Storyteller reacts to bankruptcy events (per M7) |

## Files

- `game/crates/cf-upkeep/src/lib.rs` (NEW)
- `game/crates/cf-upkeep/src/profile.rs` (NEW: UpkeepProfile registry)
- `game/crates/cf-upkeep/src/economy.rs` (NEW: FactionEconomy + BaseEconomy + ResourcePool)
- `game/crates/cf-upkeep/src/cycle.rs` (NEW: per-tick + per-day + per-scenario ticker)
- `game/crates/cf-upkeep/src/cascade.rs` (NEW: bankruptcy day-1/3/7/14/30)
- `game/crates/cf-upkeep/src/rescue.rs` (NEW: emergency mob + grants + austerity)
- `game/crates/cf-upkeep/src/supply_route.rs` (NEW: route disruption + throughput)
- `game/crates/cf-replay/src/event.rs` (MODIFY: upkeep.* category)
- `game/content/upkeep/asset_upkeep.ron` (NEW: per-asset-type UpkeepProfile registry)
- `game/content/templates/server.ron.template-vanilla` (NEW: upkeep=OFF)
- `game/content/templates/server.ron.template-classic-upkeep` (NEW: upkeep=ON, AWAW=OFF)
- `game/content/templates/server.ron.template-pve-survival` (MODIFY: upkeep=ON default)
- `game/content/templates/server.ron.template-pvp-arena` (MODIFY: upkeep=OFF default)

## Acceptance criteria

```gherkin
Scenario: Upkeep economy disabled by default
  Given server.ron with no grand_strategy block (vanilla default)
  When scenario starts
  Then no upkeep.cycle_started events fire
  And asset HP / capability is unaffected by resource state
  And economy works as classic Corefall

Scenario: Upkeep cycle fires per in-game day
  Given server config: enable_upkeep_economy=true + cycle_period=in_game_day
  When in-game day advances (24:00 → 00:00 next day)
  Then upkeep.cycle_started fires with cycle_period="in_game_day"
  And every asset's UpkeepProfile.bp_per_cycle drained from base/HQ pool
  And upkeep.cycle_completed fires after drains

Scenario: Per-resource shortage triggers correct cascade
  Given a base with 4 actors but 0 food in local pool + supply line to HQ disrupted
  When daily upkeep tick fires
  Then upkeep.base_transfer_requested fires; FAILS (disrupted)
  And upkeep.base_bankruptcy fires for that base
  And affliction.hunger applied to all 4 actors per M5.7
  And actor speed × 0.8

Scenario: Bankruptcy Day 1 → Day 3 → Day 7 cascade
  Given faction with 14-day BP deficit
  When day 1: assets flagged low-morale (-20% combat)
  When day 3: upkeep.forced_demobilization fires for lowest-priority assets
  When day 7: factory buildings auto-shut
  When day 14: faction enters Resistance Level -1 (per Edit 5.5)
  
Scenario: Emergency mobilization rescues faction
  Given faction in bankruptcy Day 5 deficit
  When player invokes act.faction.emergency_mobilization
  Then upkeep.emergency_mobilization fires with bp_injected=500
  And faction.morale -= 2
  And deficit cleared for 1 cycle

Scenario: Supply route disruption triggers transfer failure
  Given base A connected to HQ via route R
  And enemy strike disrupted route R (M12 PvP or M11.5 raid)
  Then upkeep.supply_line_disrupted fires
  When daily tick: base A cannot transfer from HQ
  And local pool drains faster → bankruptcy

Scenario: AI faction follows same upkeep rules
  Given AI faction with same UpkeepProfile assets
  When daily tick fires
  Then AI faction drains pool identically
  And AI faction can be bankrupt (storyteller fires events)
  And player can EXPLOIT AI bankruptcy (cut supply lines, force collapse)

Scenario: Multi-resource hybrid — all 7 resources independently tracked
  Given asset with UpkeepProfile { bp: 1, power: 0.1, parts: 0.05, food: 1, fuel: 0, o2: 0, coolant: 0 }
  When daily tick fires
  Then all 7 resources independently drained from respective pools
  And only resources actually consumed (zero-value resources skipped)

Scenario: Per-race upkeep multipliers apply
  Given robot soldier (race=robot; per-asset profile)
  Then bp drained × 1.2 (robot modifier)
  And power drained × 1.5
  And food drained × 0× (robots don't eat)

Scenario: Replay determinism with upkeep enabled
  Given same seed + upkeep enabled
  When replayed via cf-headless
  Then per-tick checksums match (resource pools + bankruptcy state included in sim_state_v1)

Scenario: Configurable upkeep multipliers
  Given server config: bp_upkeep_multiplier=2.0
  Then BP drained = base × 2.0 per cycle
  Easier mode: 0.5; Hardcore: 4.0

Scenario: Allied BP grant in co-op
  Given player faction in deficit + ally with surplus
  When ally invokes act.faction.grant_bp { target: player, amount: 200 }
  Then upkeep.allied_grant_sent fires
  And player's HQ pool += 200
  And ally's HQ pool -= 200

Scenario: Reserve state for demobilized assets
  Given asset demobilized in Day 3 deficit
  Then asset.state="reserve"
  And asset's UpkeepProfile drain = 0
  And asset cannot act (no movement, no fire)
  When deficit cleared:
    Then player can re-mobilize via act.faction.mobilize { asset_id } for one-cycle BP cost

Scenario: Per-cycle event family complete
  Given a full bankruptcy cascade
  Then all 12 upkeep.* event types fire at appropriate moments
  And M3B viewer renders them in plain language per template templates

Scenario: M11.5 PvE Survival defaults to upkeep=ON
  Given server.ron.template-pve-survival
  Then grand_strategy.enable_upkeep_economy=true
  And upkeep_multiplier=1.0 baseline

Scenario: M12 PvP arena defaults to upkeep=OFF
  Given server.ron.template-pvp-arena
  Then grand_strategy.enable_upkeep_economy=false
  And PvP plays as classic Corefall (no upkeep)
```

## Dependencies

- M7 (campaign + faction state) — must close (or M7.1 if Tier 2 done)
- M7.6 (power kernel — power upkeep ties to grid) — must close
- M7.8 (crafting — parts upkeep ties to fabrication chain) — must close
- M5.7 (afflictions — hunger/thirst applied per resource shortage) — must close
- M5.9 (atmospherics — oxygen/coolant upkeep) — must close
- M9 (server config baseline) — must close
- M9.10 (settings hierarchy — opt-in toggle) — must close
- M11.5 (PvE Survival — primary use case) — should be concurrent or close after

## Closure procedure

Reference bundle: `prototype_runs/native/m7.3_<UTC>_<hash>/`.
Self-play sweep rows:
- `m7.3_default_off_classic_corefall_unchanged`
- `m7.3_upkeep_cycle_daily_drain`
- `m7.3_multi_resource_hybrid_7_resources`
- `m7.3_bankruptcy_cascade_day_1_to_30`
- `m7.3_supply_line_disruption`
- `m7.3_emergency_mobilization_rescue`
- `m7.3_allied_bp_grant`
- `m7.3_austerity_auto_apply`
- `m7.3_ai_faction_bankruptcy_exploitable`
- `m7.3_per_race_upkeep_multipliers`
- `m7.3_replay_determinism`
- `m7.3_pve_survival_default_on`
- `m7.3_pvp_arena_default_off`
- `m7.3_universal_done_criteria`

All PASS. Move `specs/active/M7.3.md` → `specs/done/M7.3.md`.

## Cross-DR

DR-002 (replay), DR-005 (multiplayer), DR-006 (mod parity), DR-013 (backend), DR-024, DR-027 (combat-base), DR-029 (save model — resource pools), DR-031 (economy), DR-034 (server admin), DR-035 (MMO), DR-042 (game modes), DR-048 (endgame retention), DR-052 (determinism), DR-056.
```

### Step 2: Modify `README.md`

Find the active spec count badge (43 after Tier 4):

**BEFORE:**
```markdown
[![Specs](https://img.shields.io/badge/active%20specs-43%20%28M0.5..M12%29-blueviolet?style=flat-square)](specs/active/)
```

**AFTER (Edit 5.1 alone bumps to 44; Edit 5.2 + 5.3 add 2 more for total 46):**
```markdown
[![Specs](https://img.shields.io/badge/active%20specs-44%20%28M0.5..M12%29-blueviolet?style=flat-square)](specs/active/)
```

Find the BP7 row in the BP table and add new row immediately after M7.2 + before M7.5 (M7.6.5 row from Tier 1):

```markdown
| **BP7** | **M7.3 — Upkeep Economy (Grand-Strategy Foundation; Opt-In)** | Planned | Per-cycle upkeep drain (BP + power + parts + food + fuel + O2 + coolant) per asset type; faction-wide HQ pool + per-base local pool + supply-line transfers; bankruptcy cascade Day 1 → 3 → 7 → 14 → 30 with graduated effects (low morale → forced demobilization → building auto-shut → Resistance Level → collapse); rescue mechanisms (emergency mobilization / allied BP grants / austerity / sell asset). AI factions follow same rules (exploitable). Opt-in per server (`grand_strategy.enable_upkeep_economy=true` in server.ron). PvE Survival defaults ON; PvP defaults OFF. |
```

### Acceptance criteria for Edit 5.1

```bash
test -f specs/active/M7.3.md && echo "PASS: M7.3.md exists" || echo "FAIL"
grep -q "Upkeep Economy" specs/active/M7.3.md && echo "PASS: M7.3 intent specified" || echo "FAIL"
grep -q "UpkeepProfile" specs/active/M7.3.md && echo "PASS: UpkeepProfile schema" || echo "FAIL"
grep -q "FactionEconomy" specs/active/M7.3.md && echo "PASS: FactionEconomy schema" || echo "FAIL"
grep -q "bankruptcy_day_30" specs/active/M7.3.md && echo "PASS: bankruptcy cascade specified" || echo "FAIL"
grep -q "active%20specs-44" README.md && echo "PASS: README badge 44" || echo "FAIL"
grep -q "M7.3 — Upkeep Economy" README.md && echo "PASS: README BP7 lists M7.3" || echo "FAIL"
```

### Commit message for Edit 5.1

```
specs: Edit 5.1 — add M7.3 — Upkeep Economy (grand-strategy foundation; opt-in)

Corefall's current economy is single-payment; no per-cycle resource
pressure. M7.3 adds the canonical periodic-upkeep cycle that drives
every grand-strategy game (AWAW / HoI4 / Stellaris / Civ / ONI / RimWorld).

Per the user's confirmed design knobs:
- Cycle period: per in-game day (drain) + per scenario (reconciliation)
- Unit of upkeep: BP shell + multi-resource (BP + power + parts + food + fuel + O2 + coolant)
- Pool scope: hybrid (faction HQ + per-base local + supply line transfers)
- AI parity: AI factions follow same rules; exploitable via supply line cuts

Bankruptcy cascade: Day 1 (low morale) → Day 3 (forced demobilization) →
Day 7 (buildings shut) → Day 14 (Resistance Level -1; pairs with Edit 5.5)
→ Day 30 (collapse).

Rescue mechanisms: emergency mobilization / allied grants / austerity /
sell asset / trade with neutral.

Opt-in per server via M9.10 settings. PvE Survival defaults ON; PvP
arena defaults OFF.

- specs/active/M7.3.md created
- README.md updated (badge 43 → 44; BP7 row added; planning spine reference)
```

---

## Edit 5.2 — Create M7.4 — Strategy Phase + Goals

### Problem

M7.3 ships the mechanical layer (upkeep + bankruptcy). But there's no PLAYER DECISION LAYER on top — no "this turn I'm going aggressive vs. defensive", no "this campaign goal will earn me a major reward but a major penalty if I fail." Without a strategy phase, players experience upkeep as pure-mechanic resource drain (boring) rather than as a strategic choice space (engaging).

Grand-strategy games solve this with **per-cycle decision phases**:
- HoI4: focus tree (per-decision week)
- Stellaris: per-empire policy slider per decade
- Civ: per-era policy choices
- AWAW: per-year DP allocation (Rule 49) + research allocation (Rule 41)
- RimWorld: per-quadrum priorities + ideology

Plus **goal systems** that create stakes:
- HoI4: focus tree branching paths with rewards
- Civ: government legacy bonuses + losses
- AWAW: surrender thresholds (Rule 60) per major power

Corefall needs both: per-cycle strategy choices + per-cycle goal stakes.

### Fix

Create **M7.4 — Strategy Phase + Goals** as a new milestone that ships the player-decision layer on top of M7.3's upkeep mechanics.

### Files to modify

| File | Action |
|---|---|
| `specs/active/M7.4.md` | **CREATE** |
| `README.md` | **MODIFY** (add to BP7; update active spec count 44 → 45) |

### Step 1: Create `specs/active/M7.4.md`

```markdown
# M7.4 — Strategy Phase + Goals (Per-Cycle Decision Layer; Opt-In)

## Status

`active`

## Intent

**M7.4 is the per-cycle strategy decision + goal stake milestone** — the player-decision layer that turns M7.3's upkeep mechanics into a strategic choice space. After M7.4, each campaign turn the player picks a Stance + Production Focus + Logistics Priority (3 axes) plus 1-3 Goals with explicit stake levels (low / medium / high). Choices modify upkeep + production + combat + diplomacy outputs for the next cycle. Goals create reward/penalty stakes that drive long-term planning.

**M7.4 is opt-in per server.** Default `server.ron` has `grand_strategy.enable_strategy_phase=false`; PvE Survival recommends ON; PvP defaults OFF.

M7.4 promise: **"every campaign turn you choose what kind of faction you are this cycle — and what you'll commit to. Goals make commitment matter."**

## Player-facing behavior

### Strategy Phase — 3 axes per cycle

At the start of each campaign turn (or scenario), the player picks ONE option per axis.

#### Stance axis (5 options)

Per AWAW Rule 8 phase structure + HoI4 stance system:

| Stance | Combat dmg | Fortification | Economy growth | Upkeep cost | Notes |
|---|---|---|---|---|---|
| **Offensive** | +20% | -10% | -20% | +15% | Aggressive expansion |
| **Defensive** | -10% | +30% | 0% | -10% | Hold + fortify |
| **Economic** | -20% | -5% | +25% | 0% | Resource accumulation |
| **Research** | -10% | -5% | -5% | +5% | +30% RP/cycle (per M7.8) |
| **Survival** | -50% | +20% | +10% (food) | -25% | Winterize / hunker down (per M11.5 hunger/thirst) |

#### Production Focus axis (5 options)

| Focus | Effect | Tradeoff |
|---|---|---|
| **Units** | +30% unit build rate | -20% building speed |
| **Buildings** | +30% building build rate | -20% unit speed |
| **Consumables** | +50% food/fuel/parts production | -30% military output |
| **Vehicles** | +40% vehicle build | -30% infantry/drone build |
| **Research** | +30% RP/cycle (stacks with Research stance) | -20% all production |

#### Logistics Priority axis (3 options)

| Priority | Effect |
|---|---|
| **Local** | -30% supply route upkeep; supply range -50% |
| **Regional** | Baseline (no modifier) |
| **Inter-planet** | +50% supply route upkeep; supply range +200% (per M11.6 inter-planet transport) |

### Strategy phase UX

```text
┌─────────────────────────────────────────────────────────────┐
│ STRATEGY PHASE — Year 1942, Turn 3                          │
├─────────────────────────────────────────────────────────────┤
│ STANCE                                                      │
│   ( ) Offensive  : +20% dmg, -20% econ, +15% upkeep         │
│   (•) Defensive  : +30% fort, -10% dmg, -10% upkeep         │
│   ( ) Economic   : +25% growth, -20% military               │
│   ( ) Research   : +30% RP, -10% all else                   │
│   ( ) Survival   : winterize; +20% fort, -50% combat        │
│                                                             │
│ PRODUCTION FOCUS                                            │
│   ( ) Units                                                 │
│   ( ) Buildings                                             │
│   (•) Consumables                                           │
│   ( ) Vehicles                                              │
│   ( ) Research                                              │
│                                                             │
│ LOGISTICS PRIORITY                                          │
│   (•) Local      ( ) Regional      ( ) Inter-planet         │
├─────────────────────────────────────────────────────────────┤
│ FORECAST FOR NEXT CYCLE:                                    │
│   - Combat dmg:       1.0× × 0.9 (Defensive) = 0.9× ↓       │
│   - Fortification:    1.0× × 1.3 = 1.3× ↑                   │
│   - Economy growth:   1.0× (Defensive cancels out)          │
│   - Upkeep:           1.0× × 0.9 (Defensive) × 0.7 (Local)  │
│                       × 1.5 (Consumables food) = 0.945×     │
│   - RP/cycle:         1.0× (no Research bonus)              │
│   - Supply range:     1.0× × 0.5 = 0.5× (Local) ↓           │
├─────────────────────────────────────────────────────────────┤
│ [LOCK CHOICES] [REVIEW PREVIOUS] [HELP]                     │
└─────────────────────────────────────────────────────────────┘
```

### Goal System — 1-3 goals per cycle with stake

Per HoI4 focus tree + AWAW campaign goals + Civ era policies. Each campaign turn, player picks 1-3 goals from category templates.

#### Goal categories (6 launch types)

```rust
pub enum GoalCategory {
    Territorial,   // capture N hexes / hold capital region
    Economic,      // accumulate N BP / N parts / etc.
    Military,      // eliminate enemy / capture key boss
    Tech,          // research X by deadline
    Survival,      // last N cycles without losing capital
    Diplomatic,    // form alliance / negotiate peace
}
```

#### Goal stake levels (3 tiers)

Each goal has 3 stake levels with proportional reward/penalty:

| Stake | Reward on completion | Penalty on failure |
|---|---|---|
| **Low** | +5% small bonus (per category) | -2% small penalty |
| **Medium** | +15% baseline (per category) | -10% baseline penalty |
| **High** | +50% major bonus | -30% major penalty |

#### Goal templates (8 launch)

| Goal template | Low stake reward | Medium stake reward | High stake reward |
|---|---|---|---|
| **Hold Capital N turns** | +10 BP/turn / -5 BP on fail | +50 BP/turn / -50 BP on fail | +200 BP/turn / -300 BP on fail |
| **Accumulate N BP by deadline** | +5% growth / -2% | +20% growth / -15% | +50% growth / -40% |
| **Eliminate enemy faction** | unlock tier-3 tech / -1 morale | unlock tier-4 tech / -3 morale | unlock T4 + artifact / -10 morale |
| **Complete tech tree X** | +1 RP/cycle / -1 RP | +3 RP/cycle / -3 RP | +10 RP/cycle / -10 RP |
| **Last N cycles without losing HQ** | +1 Resistance Level / -1 | +3 Resistance Levels / -3 | +5 Resistance Levels (immunity) / -5 (death spiral) |
| **Form alliance with faction Y** | +1 ally / +1 enemy on fail | +2 allies / +2 enemies | +3 allies / faction war on fail |
| **Capture N strategic hexes** | +5 hexes / -3 hexes loss | +20 hexes / -15 hexes | +50 hexes / +faction reputation |
| **Survive boss raid** | +50 BP + recipe / -100 BP | +200 BP + 3 recipes / -500 BP | +1000 BP + artifact / faction collapse |

#### Goal stacking

- Player can pick 1-3 goals per cycle
- Rewards STACK if multiple completed
- Penalties STACK if multiple failed
- Overcommitment is intentional risk (high-stake goals can compound penalties)

### Strategy phase UX

```text
┌─────────────────────────────────────────────────────────────┐
│ GOALS — Year 1942, Turn 3                                   │
├─────────────────────────────────────────────────────────────┤
│ GOAL 1: Hold Capital Region for 3 turns                     │
│   ( ) Low stake (+10 BP/turn / -5 BP on fail)               │
│   (•) Medium stake (+50 BP/turn / -50 BP on fail)           │
│   ( ) High stake (+200 BP/turn / -300 BP on fail)           │
│                                                             │
│ GOAL 2: Complete "Plasma Engineering" tech                  │
│   ( ) Low stake  ( ) Medium  (•) High stake                 │
│                                                             │
│ GOAL 3: (none — overcommitment risk avoided)                │
├─────────────────────────────────────────────────────────────┤
│ CUMULATIVE STAKE FORECAST                                   │
│   Best case (both completed): +50 BP/turn + +10 RP/cycle    │
│   Worst case (both failed):   -50 BP/turn + -10 RP/cycle    │
│   Expected value (50/50):     +0 BP/turn / -0 RP/cycle      │
├─────────────────────────────────────────────────────────────┤
│ [LOCK GOALS] [SKIP GOALS THIS CYCLE]                        │
└─────────────────────────────────────────────────────────────┘
```

### Goal lifecycle events

- `goal.proposed { goal_id, category, stake_level }`
- `goal.locked_in { goal_id, deadline_tick }`
- `goal.progress_updated { goal_id, current_value, target_value }`
- `goal.completed { goal_id, reward_applied }`
- `goal.failed { goal_id, penalty_applied, reason }`
- `goal.expired { goal_id, deadline_reached }`

### AI parity — AI factions also pick stance + goals

Per user's confirmed design knob: AI factions follow same rules. AI factions:

- Pick stance per turn based on storyteller (Cassandra = balanced; Randy Random = chaotic)
- Pick 1-3 goals per turn based on faction personality + game state
- AI choices visible to player via M7.1.5 intelligence (codebreaking can reveal AI's stance/goals)
- AI goals can be EXPLOITED (knowing AI committed to high-stake "eliminate player faction" lets you outlast them)

### Configurable strategy phase (per M9.10)

```ron
GrandStrategyPhaseConfig (
    enable_strategy_phase: false,                   // master switch
    enable_goal_system: false,                      // can have phase WITHOUT goals
    stance_count: 5,                                // can disable Research/Survival options
    focus_count: 5,
    logistics_count: 3,
    enable_low_stake_goals: true,
    enable_medium_stake_goals: true,
    enable_high_stake_goals: true,
    max_concurrent_goals_per_cycle: 3,
    goal_stake_reward_multiplier: 1.0,              // scale all rewards
    goal_stake_penalty_multiplier: 1.0,             // scale all penalties
    goal_categories_enabled: ["Territorial", "Economic", "Military", "Tech", "Survival", "Diplomatic"],
    ai_strategy_phase_enabled: true,
    ai_goal_system_enabled: true,
    ai_intelligence_reveals_choices: false,         // M7.1.5 dependency
)
```

## Crates / modules touched

| Crate | Status | What |
|---|---|---|
| `cf-strategy` | NEW (deep) | Per-cycle strategy phase + goal system |
| `cf-strategy::stance` | NEW | 5 stances + modifiers |
| `cf-strategy::focus` | NEW | 5 production focuses |
| `cf-strategy::logistics` | NEW | 3 logistics priorities |
| `cf-strategy::goal` | NEW | Goal registry + stake levels + lifecycle |
| `cf-strategy::ai_doctrine` | NEW | AI faction strategy + goal selection |
| `cf-replay` | MODIFY | `strategy.*` + `goal.*` event categories (NEW) added to M3A taxonomy |
| `cf-ui::strategy_phase` | NEW | Strategy phase decision UI (auto-generated from schema) |

## Files

- `game/crates/cf-strategy/src/lib.rs` (NEW)
- `game/crates/cf-strategy/src/stance.rs` (NEW: 5 stances + modifier registry)
- `game/crates/cf-strategy/src/focus.rs` (NEW: 5 focuses)
- `game/crates/cf-strategy/src/logistics.rs` (NEW: 3 logistics priorities)
- `game/crates/cf-strategy/src/goal.rs` (NEW: 8 goal templates × 3 stake tiers = 24 launch goals)
- `game/crates/cf-strategy/src/lifecycle.rs` (NEW: goal proposal → lock → progress → completion/failure)
- `game/crates/cf-strategy/src/ai_doctrine.rs` (NEW: AI strategy + goal selection)
- `game/crates/cf-replay/src/event.rs` (MODIFY: strategy.* + goal.* categories)
- `game/content/strategy/stances.ron` (NEW: 5 launch stance modifiers)
- `game/content/strategy/focuses.ron` (NEW: 5 launch focus modifiers)
- `game/content/strategy/logistics.ron` (NEW: 3 launch logistics modifiers)
- `game/content/strategy/goals.ron` (NEW: 8 launch goal templates × 3 stake tiers)

## Acceptance criteria

```gherkin
Scenario: Strategy phase disabled by default
  Given server.ron with no grand_strategy block
  When campaign turn advances
  Then no strategy.phase_started events fire
  And no choice prompt presented
  And faction operates with baseline modifiers

Scenario: Player picks stance + focus + logistics
  Given enable_strategy_phase=true + campaign turn start
  When strategy phase UI presented
  And player picks Defensive + Consumables + Local
  Then strategy.locked_in fires with all 3 choices
  And next cycle's modifiers reflect choices:
    - Combat dmg × 0.9 (Defensive)
    - Fortification × 1.3 (Defensive)
    - Food production × 1.5 (Consumables)
    - Supply range × 0.5 (Local)
    - Upkeep × 0.9 (Defensive) × 0.7 (Local) = 0.63×

Scenario: Player picks 1-3 goals with stake
  Given enable_goal_system=true + strategy phase
  When player picks 2 goals:
    Goal 1: Hold Capital 3 turns (Medium stake; +50 BP/turn / -50 BP fail)
    Goal 2: Plasma Engineering tech (High stake; +10 RP/cycle / -10 RP fail)
  Then goal.locked_in fires for both
  And HUD shows active goals + progress bars
  When deadline reached + both completed:
    Then goal.completed fires for both
    And rewards STACK (+50 BP/turn + +10 RP/cycle)

Scenario: Goal failure applies penalty
  Given Medium stake Tech goal: complete in 3 cycles
  When 3 cycles elapse without completion:
    Then goal.failed fires with reason="deadline_reached"
    And faction RP/cycle reduced by 3 for 1 cycle (penalty)

Scenario: Overcommitment risk
  Given player picks 3 high-stake goals
  When 2 of 3 fail:
    Then 2× high-stake penalty applies (-60% growth + -300 BP + ...)
    And player learns lesson on overcommitment

Scenario: AI faction picks stance + goals
  Given AI faction with cassandra_classic storyteller
  When AI's turn starts
  Then AI internally picks stance based on faction state (low HP → Defensive)
  And AI picks 1-2 goals (per faction personality)
  And ai.strategy_chosen fires (visible to M3B viewer for grading)

Scenario: AI intelligence reveals choices (with M7.1.5 codebreaking)
  Given player has 5 RP/cycle invested in codebreaking
  When AI's strategy phase fires
  Then M7.1.5 codebreaking attempt fires
  And on success: intelligence.ai_strategy_revealed fires
  And player sees AI's stance + goals via HUD intel panel

Scenario: Stance + focus stack correctly
  Given player picks Research stance + Research focus
  Then RP/cycle modifier = base × 1.3 (stance) × 1.3 (focus) = 1.69×
  And combat modifier = 0.9 (stance) × 0.8 (focus) = 0.72×

Scenario: Configurable stake multipliers
  Given server config: goal_stake_reward_multiplier=2.0 + penalty_multiplier=1.5
  Then Medium goal reward = base × 2.0; penalty = base × 1.5
  Hardcore mode: 0.5 reward + 2.0 penalty (high risk, low reward)

Scenario: Goal categories filterable
  Given server config: goal_categories_enabled=["Territorial", "Military"]
  Then only those 2 categories appear in goal selection UI
  And Tech / Economic / Survival / Diplomatic goals unavailable

Scenario: Replay determinism with strategy phase
  Given same seed + strategy enabled
  When replayed
  Then strategy + goal events byte-identical
  And per-tick checksums include strategy state in sim_state_v1

Scenario: Skip goals this cycle option
  Given player doesn't want to commit
  When player clicks "Skip Goals This Cycle"
  Then goal.skipped event fires
  And no goals active for this cycle
  And no reward/penalty risk (neutral cycle)
```

## Dependencies

- M7.3 (upkeep economy) — must close (M7.4 multiplies M7.3 upkeep values)
- M7 (campaign turn architecture) — must close (or M7.1 per Tier 2)
- M7.8 (tech tree for Tech goals) — must close
- M9.10 (settings hierarchy — opt-in toggle) — must close
- M7.1.5 (intelligence — optional; goal-reveal feature) — should close concurrent

## Closure procedure

Reference bundle: `prototype_runs/native/m7.4_<UTC>_<hash>/`.
Self-play sweep rows:
- `m7.4_default_off_strategy_phase`
- `m7.4_player_picks_stance_focus_logistics`
- `m7.4_modifier_stacking`
- `m7.4_goal_low_medium_high_stake`
- `m7.4_goal_completion_reward_stacking`
- `m7.4_goal_failure_penalty_stacking`
- `m7.4_overcommitment_risk`
- `m7.4_ai_faction_strategy_picks`
- `m7.4_intelligence_reveals_ai_strategy`
- `m7.4_skip_goals_neutral_cycle`
- `m7.4_replay_determinism`
- `m7.4_universal_done_criteria`

All PASS. Move `specs/active/M7.4.md` → `specs/done/M7.4.md`.

## Cross-DR

DR-002, DR-005, DR-013, DR-024, DR-027, DR-029, DR-031, DR-042, DR-048, DR-052, DR-056.
```

### Step 2: Modify `README.md`

Update badge 44 → 45:

```markdown
[![Specs](https://img.shields.io/badge/active%20specs-45%20%28M0.5..M12%29-blueviolet?style=flat-square)](specs/active/)
```

Add M7.4 row to BP7 (after M7.3):

```markdown
| **BP7** | **M7.4 — Strategy Phase + Goals (Per-Cycle Decision Layer; Opt-In)** | Planned | Per-cycle decision phase (5 stances × 5 production focuses × 3 logistics priorities = 75 combos) + goal system (8 launch goal templates × 3 stake tiers = 24 launch goals; pick 1-3 per cycle; stake creates reward/penalty risk). AI factions follow same rules; choices revealable via M7.1.5 intelligence. Opt-in per server. |
```

### Acceptance criteria for Edit 5.2

```bash
test -f specs/active/M7.4.md && echo "PASS: M7.4.md exists" || echo "FAIL"
grep -q "Strategy Phase + Goals" specs/active/M7.4.md && echo "PASS: M7.4 intent" || echo "FAIL"
grep -q "5 stances" specs/active/M7.4.md && echo "PASS: 5 stances" || echo "FAIL"
grep -q "8 launch goal templates" specs/active/M7.4.md && echo "PASS: 8 goal templates" || echo "FAIL"
grep -q "3 stake tiers" specs/active/M7.4.md && echo "PASS: 3 stake tiers" || echo "FAIL"
grep -q "active%20specs-45" README.md && echo "PASS: README badge 45" || echo "FAIL"
grep -q "M7.4 — Strategy Phase" README.md && echo "PASS: README BP7 lists M7.4" || echo "FAIL"
```

### Commit message for Edit 5.2

```
specs: Edit 5.2 — add M7.4 — Strategy Phase + Goals (opt-in)

M7.3 ships the mechanical upkeep layer; M7.4 adds the player-decision
layer on top. Per-cycle the player picks Stance (5 options) + Focus
(5 options) + Logistics (3 options) = 75 combinations; modifiers apply
to next cycle's upkeep / production / combat.

Plus goal system: 8 launch goal templates × 3 stake tiers; pick 1-3
goals per cycle; rewards stack; penalties stack; overcommitment risk
intentional.

AI factions follow same rules; choices revealable via M7.1.5 codebreaking.

Opt-in per server. Default OFF.

- specs/active/M7.4.md created
- README.md updated (badge 44 → 45; BP7 row added)
```

---

## Edit 5.3 — Create M7.1.5 — Inter-Faction Intelligence

### Problem

Corefall currently has no inter-faction espionage / intelligence model. AI doctrine reads `EnvironmentSignal` (M6.6) but nothing models inter-faction information warfare:

- Player can't spy on enemy faction's research progress
- Player can't intercept enemy supply lines via covert ops
- Player can't decode enemy battle plans
- Player can't deceive enemy intelligence

AWAW Rules 44-48 model this beautifully with 4 distinct subsystems:

- **Rule 44 — Intelligence**: catch-all framework
- **Rule 46 — Spy Rings**: placed in enemy minor countries; +/-1 to diplomatic die rolls
- **Rule 47 — Covert Operations**: single-use; negate one enemy action
- **Rule 48 — Codebreaking**: passive; modifies combat resolution, naval interception, ground combat per success level

This adds a deep strategic axis Corefall currently lacks: **information warfare as gameplay**.

### Fix

Create **M7.1.5 — Inter-Faction Intelligence** as a sub-milestone of M7.1 (factions; Tier 2) that ships 4 intelligence subsystems per AWAW model.

### Files to modify

| File | Action |
|---|---|
| `specs/active/M7.1.5.md` | **CREATE** |
| `README.md` | **MODIFY** (add to BP7; update active spec count 45 → 46) |

### Step 1: Create `specs/active/M7.1.5.md`

```markdown
# M7.1.5 — Inter-Faction Intelligence (4 Subsystems; AWAW-Inspired; Opt-In)

## Status

`active`

## Intent

**M7.1.5 is the inter-faction information warfare milestone** — 4 distinct intelligence subsystems per AWAW Rules 44-48: Codebreaking (passive combat modifier) + Spy Rings (placed in target factions) + Covert Operations (single-use action negation) + Counter-Intelligence (defensive). After M7.1.5, factions can gather intelligence on each other, modifying combat / diplomacy / research outcomes per the depth of their intelligence investment.

**M7.1.5 is opt-in per server.** Default `server.ron` has `grand_strategy.awaw_rulesets.enable_faction_intelligence=false`. Hardcore campaign + competitive PvP defaults ON; PvE Survival typically OFF.

M7.1.5 promise: **"the war isn't just on the battlefield — it's in the radio rooms, in the embassies, in the back-alley dead drops. Information is a weapon."**

## Player-facing behavior

### 4 Intelligence Subsystems

#### 1. Codebreaking (passive; combat modifier)

Per AWAW Rule 48. Factions allocate RP/cycle to codebreaking projects. Higher investment → wider reveal of enemy actions.

```rust
pub struct CodebreakingState {
    pub faction_id: FactionId,
    pub rp_invested_per_cycle: f32,
    pub current_level: CodebreakingLevel,           // 0-5
    pub coverage_categories: BTreeSet<CodebreakingCategory>,
}

pub enum CodebreakingCategory {
    SubmarineWarfare,         // M2.5 sub combat / M9 naval
    AsW,                      // anti-submarine (M9 naval)
    Tactical,                 // ground combat reveals (M2.5 / M5 / M7)
    Strategic,                // strategic bomber operations (M2.5)
    WildCard,                 // generic; +/-1 on any roll
    BlankSpace,               // shows no useful info; wasted RP
}

pub enum CodebreakingLevel {
    Level0,                   // no codebreaking active
    Level1,                   // 1 category active; +/-1 modifier
    Level2,                   // 2 categories; +/-2 modifier
    Level3,                   // 3 categories; +/-3 modifier  
    Level4,                   // 4 categories; +/-4 modifier
    Level5,                   // 5 categories + reveal AI strategy choices (per M7.4)
}
```

**Effect (per AWAW Rule 24.622):**
- +/-1 to +/-5 modifier on combat resolution dice rolls (Submarine Warfare, ASW, etc.)
- +1 (player favorable) at level 1; +5 at level 5
- Modifier applies SYMMETRICALLY — if codebreaking helps player by +3, enemy is at -3
- At level 5: M7.4 AI strategy phase reveals — player can see AI stance/goals before next cycle

**Investment cost:** 1-5 RP/cycle per category. Multiple categories invested separately. Total RP/cycle = sum of categories.

**Per AWAW Rule 48.5 (code names):** if Edit 5.4 enabled (code-name research secrecy), codebreaking can REVEAL the underlying project a code name refers to.

#### 2. Spy Rings (placed in target factions)

Per AWAW Rule 46. Each spy ring lives in a target faction's territory.

```rust
pub struct SpyRing {
    pub ring_id: SpyRingId,
    pub home_faction: FactionId,                    // who controls
    pub target_faction: FactionId,                  // where placed
    pub installed_at_cycle: Cycle,
    pub effectiveness_pct: f32,                     // 0.0-1.0; degrades on counter-intel hits
    pub revealed: bool,                             // exposed to target?
    pub upkeep_cost_bp: f32,                        // per M7.3 upkeep (2 BP/cycle default)
}
```

**Effects when active:**

- **+/-1 to diplomatic die rolls** for target faction (per AWAW Rule 49.4262)
- **Reveal target's BP pool** (real-time visible to home faction)
- **Reveal target's army composition** (per M5.5 chassis + M5 actor count)
- **Reveal target's research investment** per category
- **Reveal target's stance/goals** (per M7.4 if enabled)

**Per AWAW Rule 46.411A:** if target faction's counter-intelligence detects the spy ring (per Subsystem 4), spy ring becomes **revealed**. Revealed rings:
- Effectiveness × 0.5
- Can be eliminated by target's covert ops (Subsystem 3)
- Home faction reputation -10 (diplomatic penalty)

#### 3. Covert Operations (single-use; negate enemy action)

Per AWAW Rule 47. Player invokes 1-2 covert ops per cycle (depending on RP investment).

```rust
pub struct CovertOperation {
    pub op_id: CovertOpId,
    pub home_faction: FactionId,
    pub target_faction: FactionId,
    pub target_action: CovertTarget,                // which enemy action to negate
    pub cost_bp: f32,                               // 50 BP per op default
    pub cost_rp: f32,                               // 10 RP per op default
    pub success_probability_pct: f32,               // 0.0-1.0 (modified by codebreaking + counter-intel)
}

pub enum CovertTarget {
    NegateEnemyResearchRoll,                        // cancels enemy research breakthrough
    NegateEnemyDiplomaticResult,                    // cancels favorable diplomatic outcome
    NegateEnemyConstruction,                        // cancels target's unit/building completion
    DisruptSupplyLine,                              // disable M7.3 supply route for N cycles
    StealFactionIntel,                              // copy target's stance/goals to player
    SabotageFactory,                                // destroy target factory + 30% materials lost
    AssassinateNPCCommander,                        // eliminate named AI commander (per M7)
    LeakFalseStrategy,                              // deceive enemy AI doctrine (per M6.6)
}
```

**Effect:** if covert op succeeds, target action is NEGATED (does not happen). If fails:
- Spy ring effectiveness × 0.5
- Target faction enters retaliation (counter-covert-op against home faction)
- Diplomatic penalty -10 if exposed

#### 4. Counter-Intelligence (defensive; protect from above 3)

Per AWAW Rule 45. Faction invests RP in counter-intel to:

- **Detect spy rings** (per cycle reveal chance)
- **Reduce enemy codebreaking effectiveness** (per category)
- **Block covert ops** (negate enemy attempts)

```rust
pub struct CounterIntelligence {
    pub faction_id: FactionId,
    pub rp_invested_per_cycle: f32,
    pub spy_ring_detection_pct_per_cycle: f32,      // 0.0-1.0
    pub codebreaking_block_pct: f32,                // reduces enemy codebreaking effectiveness
    pub covert_op_block_pct: f32,                   // chance to negate enemy covert ops
}
```

**Investment scaling:**
- 0 RP/cycle: no counter-intel; vulnerable
- 1 RP/cycle: detect 10% of spy rings; block 10% covert ops
- 3 RP/cycle: detect 30% spy rings; block 30% covert ops; -2 to enemy codebreaking
- 5 RP/cycle: detect 60% spy rings; block 60% covert ops; -5 to enemy codebreaking

### Intelligence HUD

```text
┌────────────────────────────────────────────────────┐
│ INTELLIGENCE — Faction Overview                    │
├────────────────────────────────────────────────────┤
│ CODEBREAKING                                       │
│   Submarine Warfare:    ████████░░ Level 4 (8 RP) │
│   ASW:                  ████░░░░░░ Level 2 (4 RP) │
│   Tactical:             ██████░░░░ Level 3 (6 RP) │
│   Strategic:            ░░░░░░░░░░ Level 0 (0 RP) │
│   Wild Card:            ██░░░░░░░░ Level 1 (2 RP) │
│                                                    │
│ SPY RINGS                                          │
│   In Hostile Corp:      ▲ Active (effectiveness   │
│                            87%; not revealed)      │
│   In Drone Collective:  ⚠ Revealed (eff 45%;      │
│                            target covert ops imminent)
│   In Pirates:           — None placed             │
│                                                    │
│ COVERT OPS THIS CYCLE                              │
│   [PROPOSE OP] 1/2 used                            │
│                                                    │
│ COUNTER-INTELLIGENCE                               │
│   Spy Ring Detection:   60%/cycle (5 RP invested) │
│   Codebreaking Block:   -3 to enemy efforts       │
│   Covert Op Block:      60%/cycle                 │
└────────────────────────────────────────────────────┘
```

### Configurable intelligence (per M9.10)

```ron
GrandStrategyIntelligenceConfig (
    enable_faction_intelligence: false,                 // master switch
    enable_codebreaking: true,                          // can selectively disable
    enable_spy_rings: true,
    enable_covert_operations: true,
    enable_counter_intelligence: true,
    
    // Codebreaking
    codebreaking_level_5_reveals_ai_strategy: true,     // tie-in with M7.4
    codebreaking_category_count: 5,                     // can ship with fewer
    
    // Spy rings
    spy_ring_install_cost_bp: 100,
    spy_ring_upkeep_per_cycle_bp: 2,
    spy_ring_max_per_target: 1,                         // can place multiple per faction
    
    // Covert ops
    covert_op_base_cost_bp: 50,
    covert_op_base_cost_rp: 10,
    covert_op_targets_enabled: ["NegateEnemyResearchRoll", "NegateEnemyDiplomaticResult", "NegateEnemyConstruction", "DisruptSupplyLine", "StealFactionIntel", "SabotageFactory", "AssassinateNPCCommander", "LeakFalseStrategy"],
    
    // Counter-intel
    counter_intel_detection_rate_per_rp: 0.12,         // 12% per RP/cycle
)
```

## Crates / modules touched

| Crate | Status | What |
|---|---|---|
| `cf-intelligence` | NEW (deep) | 4 subsystems |
| `cf-intelligence::codebreaking` | NEW | Codebreaking categories + level/RP system |
| `cf-intelligence::spy_ring` | NEW | Spy ring placement + effectiveness + reveal |
| `cf-intelligence::covert_op` | NEW | Covert operation registry + execution + success calc |
| `cf-intelligence::counter_intel` | NEW | Counter-intelligence per-faction state |
| `cf-faction` | MODIFY | Faction state extended with intelligence references |
| `cf-replay` | MODIFY | `intelligence.*` event category (NEW) added to M3A taxonomy |
| `cf-ui::intelligence_panel` | NEW | Intelligence HUD per faction |

## Acceptance criteria

```gherkin
Scenario: Intelligence subsystems disabled by default
  Given server.ron with no awaw_rulesets block
  Then no intelligence.* events fire
  And factions don't have intel state

Scenario: Codebreaking level 4 gives +/-4 modifier
  Given enable_codebreaking=true + 8 RP invested in Submarine Warfare
  Then codebreaking_level for Sub Warfare = 4
  When submarine combat resolved between factions:
    Then home faction +4 modifier; enemy -4 modifier symmetric

Scenario: Spy ring reveals target's stance + goals
  Given spy ring placed in Hostile Corp; effectiveness 90%
  When player invokes inspect.intel target=Hostile Corp:
    Then intel returns: BP pool, army composition, stance, goals
  And HUD shows real-time enemy info

Scenario: Counter-intel detects spy ring
  Given target faction has 3 RP/cycle counter-intel
  When daily tick fires:
    Then 30% chance to detect home's spy ring
    On detection: spy_ring.revealed event fires
    Effectiveness × 0.5; covert ops imminent against home

Scenario: Covert op negates enemy action
  Given enemy completes "Plasma Engineering" research; player counter-ops it
  When CovertOp NegateEnemyResearchRoll succeeds:
    Then enemy.research_breakthrough event NOT fired
    Player gets satisfaction; -50 BP + -10 RP cost

Scenario: Failed covert op exposes spy ring
  Given covert op fails (counter-intel blocks):
    Then home's spy ring effectiveness × 0.5
    And diplomatic penalty -10 if exposed

Scenario: AI factions use intelligence too
  Given AI faction with intelligence investment
  When daily tick fires:
    Then AI may invoke covert ops against player
    And AI checks codebreaking against player's actions

Scenario: Level 5 codebreaking reveals AI M7.4 strategy
  Given player invests 10 RP/cycle in Wild Card category → Level 5
  When AI faction picks stance + goals (per M7.4):
    Then intelligence.ai_strategy_revealed fires
    And player sees AI choices via HUD

Scenario: Replay determinism with intelligence
  Given same seed + intelligence enabled
  When replayed
  Then all intelligence events byte-identical
  And per-tick checksums include intelligence state

Scenario: Configurable cost multipliers
  Given server config: covert_op_base_cost_bp=200 (hardcore)
  Then covert op cost = 200 BP each (vs default 50)
  Easier mode: 25 BP
```

## Dependencies

- M7 (campaign + faction state) — must close (or M7.1 per Tier 2)
- M7.8 (research + RP system) — must close
- M7.6 (RP generated per scenario completion) — must close
- M9.10 (settings hierarchy — opt-in toggles) — must close
- M11 (online co-op — multi-faction setting required for inter-faction intel) — should be concurrent

## Closure procedure

Reference bundle: `prototype_runs/native/m7.1.5_<UTC>_<hash>/`.
Self-play sweep rows:
- `m7.1.5_default_off_no_intel_events`
- `m7.1.5_codebreaking_level_progression`
- `m7.1.5_spy_ring_install_and_reveal`
- `m7.1.5_counter_intel_detection`
- `m7.1.5_covert_op_negate_research`
- `m7.1.5_covert_op_negate_diplomatic`
- `m7.1.5_failed_covert_op_exposes_ring`
- `m7.1.5_level_5_reveals_ai_strategy`
- `m7.1.5_ai_intelligence_parity`
- `m7.1.5_replay_determinism`
- `m7.1.5_universal_done_criteria`

All PASS. Move `specs/active/M7.1.5.md` → `specs/done/M7.1.5.md`.

## Cross-DR

DR-002, DR-005, DR-013, DR-022 (AI humanlike — intelligence as AI doctrine), DR-024, DR-027, DR-029, DR-031, DR-042, DR-048, DR-052, DR-056.
```

### Step 2: Modify `README.md`

Update badge 45 → 46:

```markdown
[![Specs](https://img.shields.io/badge/active%20specs-46%20%28M0.5..M12%29-blueviolet?style=flat-square)](specs/active/)
```

Add M7.1.5 row to BP7 (after M7.1):

```markdown
| **BP7** | **M7.1.5 — Inter-Faction Intelligence (4 Subsystems; AWAW-Inspired; Opt-In)** | Planned | 4 intelligence subsystems per AWAW Rules 44-48: Codebreaking (level 0-5 across 5 categories; passive combat modifier ±1 to ±5) + Spy Rings (placed in target factions; reveal BP/army/stance/goals; +/-1 diplomatic die roll modifier) + Covert Operations (8 launch action types: negate research/diplomacy/construction/supply/intel-steal/sabotage/assassinate/false-strategy) + Counter-Intelligence (detect spies, block covert ops, reduce enemy codebreaking). AI factions follow same rules. Opt-in per server. |
```

### Acceptance criteria for Edit 5.3

```bash
test -f specs/active/M7.1.5.md && echo "PASS: M7.1.5.md exists" || echo "FAIL"
grep -q "Inter-Faction Intelligence" specs/active/M7.1.5.md && echo "PASS: M7.1.5 intent" || echo "FAIL"
grep -q "Codebreaking" specs/active/M7.1.5.md && echo "PASS: codebreaking" || echo "FAIL"
grep -q "Spy Rings" specs/active/M7.1.5.md && echo "PASS: spy rings" || echo "FAIL"
grep -q "Covert Operations" specs/active/M7.1.5.md && echo "PASS: covert ops" || echo "FAIL"
grep -q "Counter-Intelligence" specs/active/M7.1.5.md && echo "PASS: counter-intel" || echo "FAIL"
grep -q "AWAW Rule 4" specs/active/M7.1.5.md && echo "PASS: AWAW references" || echo "FAIL"
grep -q "active%20specs-46" README.md && echo "PASS: README badge 46" || echo "FAIL"
grep -q "M7.1.5 — Inter-Faction" README.md && echo "PASS: README BP7 lists M7.1.5" || echo "FAIL"
```

### Commit message for Edit 5.3

```
specs: Edit 5.3 — add M7.1.5 — Inter-Faction Intelligence (AWAW-inspired; opt-in)

Corefall had no inter-faction information warfare model. Adopted AWAW's
4-subsystem framework per Rules 44-48:

- Codebreaking (passive combat modifier per category; levels 0-5)
- Spy Rings (placed in target factions; reveal intel + DP modifier)
- Covert Operations (8 launch action types; single-use action negation)
- Counter-Intelligence (defensive; protect from above 3)

AI factions follow same rules. Level 5 codebreaking reveals AI's M7.4
strategy/goals.

Opt-in per server. Default OFF.

- specs/active/M7.1.5.md created
- README.md updated (badge 45 → 46; BP7 row added)
```

---

## Edit 5.4 — Add Code-Name Research Secrecy to M7.8

### Problem

M7.8 ships a 5-tier research tree with 30 launch nodes. Every player can see every other faction's research progress (public). This is OK for solo PvE but boring for PvP / MMO where information warfare matters.

AWAW Rule 41.5 is the most novel mechanic in the entire game: **public die rolls + hidden project intent**. Players assign secret code names to research projects. Die rolls are announced publicly ("project Rattlesnake rolled a 5") but the actual project the code name refers to is SECRET. Opponents can try to DEDUCE from patterns; players can MISLEAD with decoy code names.

This adds a deep bluffing layer that's currently completely absent from Corefall.

### Fix

Extend M7.8 with code-name research secrecy as an opt-in feature. Default OFF; competitive PvP / MMO recommends ON.

### Files to modify

| File | Action |
|---|---|
| `specs/active/M7.8.md` | **MODIFY** (add code-name secrecy section + acceptance criteria) |

### Step 1: Modify `specs/active/M7.8.md`

Find the **Recipe + research system** section. Immediately AFTER it, add:

```markdown
### Code-Name Research Secrecy (AWAW Rule 41.5 — Opt-In)

**Opt-in per server.** Default `server.ron` has `grand_strategy.awaw_rulesets.enable_code_name_research_secrecy=false`. Competitive PvP / MMO recommends ON.

When enabled, factions assign **code names** to research projects. Other factions see allocations + die rolls but NOT what project each code name refers to.

#### Code-name mechanics

When a faction starts a research project:

```rust
pub struct ResearchProject {
    pub project_id: ResearchProjectId,
    pub home_faction: FactionId,
    pub actual_tech_id: TechTreeNodeId,             // PRIVATE — only home faction sees
    pub code_name: String,                          // PUBLIC — visible to all factions
    pub rp_allocated_per_cycle: f32,
    pub die_rolls_this_cycle: Vec<DiceRoll>,        // public; results visible
    pub current_progress_pct: f32,
}

pub struct DiceRoll {
    pub tick: Cycle,
    pub roll_value: u32,                            // 1-20 default; visible to all
    pub modifier_total: i32,                        // visible to all
    pub success: bool,                              // public outcome
}
```

#### Public information

- Code name (player-chosen; e.g. "Project Rattlesnake")
- RP allocated per cycle
- Die roll results per cycle
- Success/failure of each roll
- When project completes (which code name finished)

#### Private information (only home faction sees)

- The actual `TechTreeNodeId` the code name refers to
- Internal modifier breakdown (per-category bonuses)

#### Deduction mechanics

Other factions can try to DEDUCE what code name refers to:
- Pattern: if home faction always rolls +3 modifier on "Rattlesnake", probably a category that has +3 inherent bonus for that race
- Time pattern: T4 atomic research takes 1+ year; if "Rattlesnake" has been ongoing for 2 years, probably T4
- Resource pattern: high RP investment per cycle suggests T3-T4 tier
- Combined: deduction is uncertain but tactical

#### Decoy code names (advanced)

Players can DELIBERATELY use misleading code names:
- Name "Project Cookbook" for atomic weapons → bluff
- Name "Project Hephaestus" for plant cultivation → distract
- Counter-deduction: AI factions get smarter at decoy detection at higher difficulty

#### Codebreaking interaction (per M7.1.5)

If M7.1.5 codebreaking enabled + faction has Level 5 codebreaking:
- Player can REVEAL one enemy code name per cycle
- Cost: 1 covert op + 200 BP
- Success probability based on codebreaking level + target counter-intel

#### Replay events

- `research.code_name_assigned { project_id, faction_id, code_name, actual_tech: PRIVATE }`
- `research.die_roll_made { project_id, code_name (public), roll_value, modifier, success: PUBLIC }`
- `research.code_name_revealed { code_name, actual_tech, revealed_by: codebreaking|completion }`

#### Configuration

```ron
GrandStrategyCodeNameConfig (
    enable_code_name_research_secrecy: false,        // master switch
    enable_decoy_code_names: true,
    require_code_name_at_project_start: true,
    code_name_max_length: 32,
    enable_codebreaking_reveal: true,                // M7.1.5 dependency
    default_codebreaking_reveal_cost_bp: 200,
)
```
```

Add acceptance criterion at the end of M7.8's acceptance section:

```gherkin
Scenario: Code-name secrecy disabled by default
  Given server.ron with no awaw_rulesets block
  When research project starts
  Then research.code_name_assigned does NOT fire
  And all factions see actual tech being researched (public)

Scenario: Code-name secrecy reveals projects only on completion
  Given enable_code_name_research_secrecy=true
  When faction starts "Project Rattlesnake" researching Plasma Engineering
  Then research.code_name_assigned fires with actual_tech=PRIVATE
  And other factions see "Rattlesnake at 3 RP/cycle, 2 rolls completed"
  And other factions don't see "Plasma Engineering" until project completes
  When project completes:
    Then research.code_name_revealed fires
    And all factions now know "Rattlesnake" = Plasma Engineering

Scenario: Codebreaking can reveal code name mid-project
  Given enable_code_name_research_secrecy=true + enable_codebreaking_reveal=true
  And player has Level 5 codebreaking in Strategic category
  When player invokes act.faction.reveal_code_name { code_name: "Rattlesnake" }
  Then 200 BP drained + 1 covert op used
  And research.code_name_revealed fires with revealed_by="codebreaking"
  And player sees "Rattlesnake" = Plasma Engineering

Scenario: Decoy code name confuses AI
  Given enable_decoy_code_names=true
  And player names "Project Cookbook" for atomic research
  Then AI factions see "Cookbook" + RP allocation
  And AI must deduce; not always correct
```

### Acceptance criteria for Edit 5.4

```bash
grep -q "Code-Name Research Secrecy" specs/active/M7.8.md && echo "PASS: code-name section" || echo "FAIL"
grep -q "AWAW Rule 41.5" specs/active/M7.8.md && echo "PASS: AWAW Rule 41.5 cited" || echo "FAIL"
grep -q "actual_tech_id" specs/active/M7.8.md && echo "PASS: schema specified" || echo "FAIL"
grep -q "Scenario: Code-name secrecy disabled by default" specs/active/M7.8.md && echo "PASS: default OFF scenario" || echo "FAIL"
grep -q "Scenario: Codebreaking can reveal code name" specs/active/M7.8.md && echo "PASS: codebreaking integration" || echo "FAIL"
```

### Commit message for Edit 5.4

```
specs: Edit 5.4 — add code-name research secrecy to M7.8 (AWAW Rule 41.5; opt-in)

The most novel mechanic in AWAW: public die rolls + hidden project
intent via code names. Players assign secret code names; opponents see
"Project Rattlesnake rolled a 5" but not what Rattlesnake actually is.

Deduction + decoys + codebreaking integration (per M7.1.5 Level 5
reveals).

Opt-in per server. Default OFF; competitive PvP / MMO recommends ON.

- specs/active/M7.8.md modified (added Code-Name Research Secrecy section)
```

---

## Edit 5.5 — Add Faction Resistance Levels to M11.7

### Problem

M11.7 (PvE endgame / storyteller) currently has binary faction states: alive OR collapsed. No graduated collapse model. This makes the endgame feel too abrupt — factions go from "doing fine" to "destroyed" with no narrative arc.

AWAW Rule 60 (Russian Resistance Level) handles this with **graduated collapse**: -1 (20 BRPs of units lost), -2 (40 BRPs), -3 (60 BRPs), etc. Each level adds defensive bonus + reduced offensive capability. Cumulative levels eventually trigger surrender.

This adds rich narrative arc to faction decline.

### Fix

Extend M11.7 with Faction Resistance Levels as an opt-in mechanic.

### Files to modify

| File | Action |
|---|---|
| `specs/active/M11.7.md` | **MODIFY** (add Resistance Levels section + acceptance criteria) |

(Note: M11.7 is created in Tier 2 Edit 2.2. If Tier 2 has not merged, apply to M11.5 instead.)

### Step 1: Modify `specs/active/M11.7.md`

Find the **5 PvE endgame bosses** section. Immediately AFTER, add:

```markdown
### Faction Resistance Levels (AWAW Rule 60 — Opt-In Graduated Collapse)

**Opt-in per server.** Default `server.ron` has `grand_strategy.awaw_rulesets.enable_faction_resistance_levels=false`. Hardcore PvE Survival / persistent MMO recommends ON.

Instead of binary "alive/collapsed", factions cascade through **5 resistance levels** based on cumulative losses + bankruptcy.

#### Resistance Level system

```rust
pub struct FactionResistance {
    pub faction_id: FactionId,
    pub current_level: ResistanceLevel,             // 0 / -1 / -2 / -3 / -4 / surrender
    pub cumulative_losses_bp: f32,                  // running total
    pub last_level_change_tick: Tick,
    pub recovery_possible: bool,                    // can the faction climb back?
}

pub enum ResistanceLevel {
    Level0,         // healthy faction; no penalties
    LevelMinus1,    // 20 BP of units lost OR 14-day deficit
    LevelMinus2,    // 40 BP of units lost OR 21-day deficit
    LevelMinus3,    // 60 BP of units lost OR 28-day deficit
    LevelMinus4,    // 80 BP of units lost OR 35-day deficit
    Surrender,      // 100+ BP of units lost OR 42-day deficit
}
```

#### Per-level effects

| Level | Combat dmg | Defensive bonus | Production | Diplomatic | Effects |
|---|---|---|---|---|---|
| **0** | 1.0× | 1.0× | 1.0× | normal | healthy faction |
| **-1** | 0.95× | 1.10× | 1.0× | -1 DP/cycle | minor decline; defensive bonus |
| **-2** | 0.85× | 1.20× | 0.9× | -2 DP/cycle | visible weakness; allies hesitate |
| **-3** | 0.70× | 1.30× | 0.8× | -3 DP/cycle | desperate measures; refugee crises |
| **-4** | 0.50× | 1.40× | 0.6× | -5 DP/cycle | near-collapse; rebellion risk |
| **Surrender** | 0× | 0× | 0× | banished | faction defeated |

**Recovery:** factions CAN climb back UP a level by:
- Winning major battles (per scenario reward)
- Receiving allied BP grants (per M7.3 Edit 5.1)
- Diplomatic alliances (per AWAW Rule 49 — diplomatic results)
- Tech breakthroughs (per M7.8 research tree)

Once at Surrender, recovery is impossible. Surrender is final per AWAW Rule 54.

#### Storyteller integration

M7 storyteller (Cassandra/Phoebe/Randy/Ironman/Sandbox) reads resistance level + escalates events:

- **Level -1**: occasional refugee events (player can help; +diplomatic)
- **Level -2**: rebellion in occupied territory (player can crush or pacify)
- **Level -3**: rival faction smells blood; aggressive raids
- **Level -4**: cascading defections; named NPCs leave; loyal followers desperate
- **Surrender**: faction-wide funeral event; allied factions mourn

#### Per-faction surrender variants

Per AWAW Rules 54-62 (faction-specific surrender):

- Hostile Corp surrender → all assets sold at firesale; player gets bonus equipment
- Allied Resistance surrender → player loses major ally; reputation -50
- Pirates surrender → fleet disbands; loot scattered
- Drone Collective surrender → drone hive shuts; mining operations stop
- ...etc per faction

#### Configuration

```ron
GrandStrategyResistanceConfig (
    enable_faction_resistance_levels: false,                 // master switch
    level_minus_1_threshold_bp_lost: 20,
    level_minus_2_threshold_bp_lost: 40,
    level_minus_3_threshold_bp_lost: 60,
    level_minus_4_threshold_bp_lost: 80,
    level_surrender_threshold_bp_lost: 100,
    bankruptcy_to_resistance_level_link: true,          // 14-day deficit → -1; 21 → -2; etc.
    enable_recovery: true,                              // factions can climb back
    storyteller_resistance_event_intensity: 1.0,        // scale event frequency
)
```

#### Replay events

- `resistance.level_changed { faction_id, from_level, to_level, cause: cumulative_losses|bankruptcy }`
- `resistance.recovery_action { faction_id, action: allied_grant|alliance_formed|tech_breakthrough }`
- `resistance.storyteller_event_triggered { event_kind, severity }`
- `resistance.surrender_imminent { faction_id, days_remaining }`
- `resistance.surrender_completed { faction_id, surrender_variant }`
```

Add acceptance criteria at the end of M11.7's acceptance section:

```gherkin
Scenario: Resistance levels disabled by default
  Given server.ron with no awaw_rulesets block
  When faction takes losses
  Then no resistance.* events fire
  And faction stays at level 0 or transitions to collapsed (binary)

Scenario: Faction descends through resistance levels
  Given enable_faction_resistance_levels=true + faction loses 30 BP of units
  Then resistance.level_changed fires with to_level=-1 (>20 BP threshold)
  When losses reach 50 BP:
    Then to_level=-2 (>40 BP threshold)
  Per level: combat × 0.95 → × 0.85 → × 0.70 → × 0.50

Scenario: Bankruptcy triggers resistance level via Edit 5.1
  Given upkeep economy active + faction at 14-day deficit (per Edit 5.1)
  Then resistance.level_changed fires from level 0 to -1
  And M7.3 bankruptcy cascade synchronizes with resistance level

Scenario: Faction recovers via allied grant
  Given faction at level -2
  When allied faction grants 1000 BP (per M7.3 Edit 5.1):
    Then resistance.recovery_action fires
    And level moves -2 → -1 → 0 over 2 cycles

Scenario: Surrender is final
  Given faction reaches Surrender level (100+ BP lost)
  Then resistance.surrender_completed fires
  And faction cannot recover
  And per-faction surrender variant applies

Scenario: Storyteller escalates with resistance levels
  Given faction at -3 + Randy Random storyteller
  Then aggressive raid events fire (high frequency)
  And rival factions smell blood
```

### Acceptance criteria for Edit 5.5

```bash
grep -q "Faction Resistance Levels" specs/active/M11.7.md && echo "PASS: resistance levels section" || echo "FAIL"
grep -q "AWAW Rule 60" specs/active/M11.7.md && echo "PASS: AWAW Rule 60 cited" || echo "FAIL"
grep -q "ResistanceLevel" specs/active/M11.7.md && echo "PASS: schema specified" || echo "FAIL"
grep -q "Scenario: Faction descends through resistance levels" specs/active/M11.7.md && echo "PASS: cascade scenario" || echo "FAIL"
grep -q "Scenario: Bankruptcy triggers resistance level" specs/active/M11.7.md && echo "PASS: M7.3 integration" || echo "FAIL"
```

### Commit message for Edit 5.5

```
specs: Edit 5.5 — add faction resistance levels to M11.7 (AWAW Rule 60; opt-in)

Replaces binary "alive/collapsed" with 5-level graduated cascade per
AWAW Rule 60 (Russian Resistance Level model).

Level 0 → -1 → -2 → -3 → -4 → Surrender. Per-level effects on combat,
production, diplomacy. Recovery possible via allied grants, alliances,
tech breakthroughs.

Integrates with M7.3 bankruptcy cascade: 14-day deficit → -1; 21 → -2;
etc. M7 storyteller escalates events per level.

Opt-in per server. Default OFF.

- specs/active/M11.7.md modified
```

---

## Edit 5.6 — Add 9-Phase Strategic Campaign Turn Sequence to M7

### Problem

M7 (campaign + base + commander) currently runs as continuous real-time gameplay. Players can do anything in any order. This is fine for tactical scenarios but missing for strategic-layer gameplay where each campaign turn should have a structured rhythm.

AWAW Rule 8 specifies a 9-phase turn sequence that every faction follows:

```
Research → Diplomacy → DOW → Movement → Combat → Post-combat → Construction → Redeployment
```

Each phase has a specific purpose. The discipline forces players to think across all axes every turn. Combined with M7.4 (Strategy Phase) + M7.1.5 (Intelligence), this creates a rich strategic-layer rhythm.

### Fix

Add an optional 9-phase Strategic Campaign Turn structure to M7. Default off (continuous gameplay); opt-in for strategic-layer servers.

### Files to modify

| File | Action |
|---|---|
| `specs/active/M7.md` | **MODIFY** (add Strategic Campaign Turn section + acceptance criteria) |

### Step 1: Modify `specs/active/M7.md`

Find the **Storyteller / incident director** section. Immediately AFTER it, add:

```markdown
### Strategic Campaign Turn Sequence (AWAW Rule 8 — Opt-In)

**Opt-in per server.** Default `server.ron` has `grand_strategy.awaw_rulesets.enable_campaign_turn_sequence=false`. Server admins who want structured strategic-layer gameplay flip ON. Vanilla Corefall has continuous real-time campaign; AWAW-grade Corefall has 9-phase turn rhythm.

#### Per-faction 9-phase turn

Every faction's campaign turn follows this sequence:

```
1. RESEARCH PHASE
   - Allocate RP/cycle to research projects (per M7.8)
   - Make research die rolls (per M7.4 if Strategy Phase enabled)
   - Reveal completed projects (M7.8 + Edit 5.4)
   
2. DIPLOMACY PHASE  
   - Allocate Diplomatic Points (per AWAW Rule 49)
   - Trigger diplomatic die rolls (per M7.1 factions)
   - Receive diplomatic results (per AWAW Rule 49.5)
   - M7.1.5 covert ops can negate (if enabled)
   
3. DECLARATIONS OF WAR PHASE
   - Announce DOWs (per M7.1)
   - Pay BP cost for each DOW (per AWAW Rule 8.23)
   - Set up minor country forces (M7.1)
   
4. MOVEMENT PHASE
   - Move units (per M5 actor + M5.5 collision)
   - Supply determination (per M7.3 upkeep + AWAW Rule 8.24)
   - Oil shipments + BRP grants (per AWAW Rule 40 + M7.3)
   - BRP expenditures for offensive operations (per AWAW Rule 9)
   
5. COMBAT PHASE
   - Resolve offensive combat (per M5 / M5.5 / M2.5)
   - Limited Offensive (≤14 BP/front) vs. Full Offensive (≥15 BP/front)
   - Apply combat modifiers (codebreaking per M7.1.5; weather per M7.7)
   - Resolve attrition combat at end
   
6. POST-COMBAT ADJUSTMENTS PHASE
   - BRP base adjustments due to conquests/losses (per M7.3)
   - Post-combat supply determination
   - Oil shipments + BRP grants (final)
   - Unsupplied units eliminated
   
7. UNIT CONSTRUCTION PHASE
   - Construct unbuilt units (per AWAW Rule 27)
   - Apply M7.4 Strategy Phase production modifiers (if enabled)
   
8. REDEPLOYMENT PHASE
   - Strategic redeployment across long distances
   - Reduced by SW losses inflicted (per AWAW Rule 24)
   - Transport units between SW boxes (per AWAW Rule 24.52)
   
9. END-OF-TURN PHASE
   - Upkeep tick fires (per M7.3 if enabled; daily or per turn)
   - Goal progress updated (per M7.4 if enabled)
   - Storyteller fires per-phase events
```

Phase transitions are explicit; HUD shows current phase prominently.

#### Phase HUD

```text
┌────────────────────────────────────────────────────┐
│ STRATEGIC TURN — Year 1942, Spring Turn            │
├────────────────────────────────────────────────────┤
│ CURRENT PHASE: COMBAT                              │
│ ████████░░░░░░░░░░ Phase 5/9                       │
│                                                    │
│   [✓] 1. Research                                  │
│   [✓] 2. Diplomacy                                 │
│   [✓] 3. Declarations of War                       │
│   [✓] 4. Movement                                  │
│   [▶] 5. Combat (active)                           │
│   [ ] 6. Post-Combat                               │
│   [ ] 7. Unit Construction                         │
│   [ ] 8. Redeployment                              │
│   [ ] 9. End-of-Turn                               │
│                                                    │
│ Faction status: 7 of 8 factions in this phase     │
└────────────────────────────────────────────────────┘
```

#### Phase events

- `campaign_turn.phase_started { faction_id, phase, tick }`
- `campaign_turn.phase_completed { faction_id, phase, duration_ticks }`
- `campaign_turn.synchronization_required { faction_id, blocking_factions: Vec<FactionId> }`
- `campaign_turn.turn_completed { tick, year, season }`

#### Async vs. sync phases

By default, phases are **async** — each faction progresses through its turn independently. Optional sync mode:
- **Hybrid sync**: factions advance independently EXCEPT combat phase requires all factions complete movement first
- **Full sync**: all factions advance through phases in lockstep (AWAW-strict; slower but more predictable)

#### Configuration

```ron
GrandStrategyTurnSequenceConfig (
    enable_campaign_turn_sequence: false,           // master switch
    sync_mode: "async",                             // "async" | "hybrid_sync" | "full_sync"
    enable_all_9_phases: true,                      // can skip some phases
    phase_timeout_minutes: 5,                       // per-phase real-time limit (optional)
    auto_advance_inactive_factions: true,           // AI factions auto-progress
    enable_phase_callouts: true,                    // banner per phase transition
)
```

#### M7.4 Strategy Phase interaction

If M7.4 (Strategy Phase + Goals) enabled:
- Strategy phase fires at start of turn (before Phase 1)
- Stance/focus/logistics modifiers apply throughout all 9 phases
- Goal progress evaluated at end-of-turn (Phase 9)
```

Add acceptance criteria:

```gherkin
Scenario: Campaign turn sequence disabled by default
  Given server.ron with no awaw_rulesets block
  Then no campaign_turn.* events fire
  And gameplay is continuous real-time

Scenario: 9-phase turn fires in order
  Given enable_campaign_turn_sequence=true + new turn starts
  Then campaign_turn.phase_started fires for phase=research (Phase 1)
  After research complete: phase=diplomacy (Phase 2)
  ... through Phase 9 (end_of_turn)
  When all phases complete:
    Then campaign_turn.turn_completed fires
    And new turn starts (loop)

Scenario: AWAW phase requires BRP cost per offensive
  Given Phase 5 (Combat) + player declares offensive on faction A
  Then BRP cost per AWAW Rule 9.5 applies
  And per-unit cost deducted from HQ pool (per M7.3)
  And combat resolves with modifiers (per M7.1.5 codebreaking if enabled)

Scenario: Strategic redeployment in Phase 8
  Given Phase 8 (Redeployment) + player moves units to back-line
  Then movement is faster than normal (strategic redeployment bonus)
  When SW losses inflicted in Phase 5 (per AWAW Rule 24):
    Then redeployment capacity reduced proportionally

Scenario: Sync modes work correctly
  Given sync_mode="hybrid_sync"
  When 5 factions reach Phase 4 (Movement) at different times:
    Then they pause at end of Phase 4 (waiting)
  When all factions complete Phase 4:
    Then all advance to Phase 5 (Combat) together

Scenario: M7.4 Strategy Phase fires before Phase 1
  Given enable_strategy_phase=true + new turn starts
  Then strategy.phase_started fires FIRST (before Phase 1 Research)
  And player picks stance/focus/logistics
  Then Phase 1 Research begins with modifiers applied
```

### Acceptance criteria for Edit 5.6

```bash
grep -q "Strategic Campaign Turn Sequence" specs/active/M7.md && echo "PASS: turn sequence section" || echo "FAIL"
grep -q "AWAW Rule 8" specs/active/M7.md && echo "PASS: AWAW Rule 8 cited" || echo "FAIL"
grep -q "RESEARCH PHASE" specs/active/M7.md && echo "PASS: Phase 1 specified" || echo "FAIL"
grep -q "REDEPLOYMENT PHASE" specs/active/M7.md && echo "PASS: Phase 8 specified" || echo "FAIL"
grep -q "Scenario: 9-phase turn fires in order" specs/active/M7.md && echo "PASS: 9-phase scenario" || echo "FAIL"
```

### Commit message for Edit 5.6

```
specs: Edit 5.6 — add 9-phase strategic campaign turn to M7 (AWAW Rule 8; opt-in)

Vanilla Corefall has continuous real-time campaign. AWAW-grade Corefall
has structured 9-phase turn rhythm per Rule 8:

Research → Diplomacy → DOW → Movement → Combat → Post-combat →
Construction → Redeployment → End-of-Turn

Each phase has specific purpose. Combined with M7.4 Strategy Phase +
M7.1.5 Intelligence, creates rich strategic-layer rhythm.

Sync modes: async (default) / hybrid_sync (combat sync) / full_sync (lockstep).

Opt-in per server. Default OFF.

- specs/active/M7.md modified
```

---

## Edit 5.7 — Add Industrial Center Evacuation to M11.5

### Problem

M11.5 (PvE Survival + Procgen) currently has binary territory: faction owns it or doesn't. Losing territory = losing all the economic value. This is the "all-or-nothing" feeling that frustrates strategic play.

AWAW Rule 37 (Soviet Industrial Centers) models this beautifully: ICs are large named resource hubs that can be CAPTURED (lost forever) OR EVACUATED (moved to safer region; partial value retained). The Soviet 1941-42 industrial evacuation is a canonical historical example of this mechanic.

For Corefall, ICs are large production buildings (factories, smelters, reactors) that anchor faction economy. Losing all factories simultaneously is catastrophic; saving 30% via evacuation is grim but viable.

### Fix

Extend M11.5 procgen with Industrial Centers as named, evacuable economic hubs.

### Files to modify

| File | Action |
|---|---|
| `specs/active/M11.5.md` | **MODIFY** (add IC evacuation mechanic + acceptance) |

### Step 1: Modify `specs/active/M11.5.md`

Find the **Procedural world generation algorithm** section. Immediately after the AI raider settlement seeding pass (pass 6), add:

```markdown
### Industrial Center Evacuation (AWAW Rule 37 — Opt-In)

**Opt-in per server.** Default `server.ron` has `grand_strategy.awaw_rulesets.enable_industrial_center_evacuation=false`. Hardcore PvE Survival recommends ON.

#### Industrial Center placement

Per procgen pass: each world has 3-8 **Industrial Centers** (depending on world size + faction count). ICs are large named production hubs:

```rust
pub struct IndustrialCenter {
    pub ic_id: IndustrialCenterId,
    pub name: String,                               // e.g. "Magnitogorsk Steelworks"
    pub world_id: WorldId,
    pub position: Vec2,
    pub kind: ICKind,                               // factory / smelter / reactor / etc.
    pub bp_value: f32,                              // economic value (50-500 BP)
    pub current_owner: FactionId,
    pub evacuation_state: EvacuationState,
}

pub enum ICKind {
    Factory,            // produces parts / units
    Smelter,            // produces steel / refined metal
    Reactor,            // produces power (per M7.6)
    ResearchLab,        // produces RP/cycle
    Refinery,           // produces fuel
    Workshop,           // produces tools
}

pub enum EvacuationState {
    Operational,
    Evacuating { progress_pct: f32, target_region: BaseId },
    Evacuated { value_retained_pct: f32 },         // typically 30-70%
    Captured { by_faction: FactionId },             // lost forever
    Destroyed,                                      // scorched-earth
}
```

#### Per-IC defensive characteristics

Each IC has:
- **BP value**: economic worth (50-500 BP)
- **Defensive HP**: 100-1000 (lots of fortifications around it)
- **Evacuation time**: 3-7 in-game days (per IC value)
- **Evacuation BP cost**: 20-50% of IC value (cost to move materials + workers)
- **Materials carried**: 30-70% of original value (varies)

#### Evacuation mechanic

When enemy threatens IC + player decides to save value:

1. Player invokes `act.faction.evacuate_ic { ic_id, target_region }`
2. IC enters Evacuating state
3. Per-day progress accumulates (depending on player labor + transport)
4. During evacuation: IC produces 0 (workers transferring)
5. When complete: IC at target_region produces 30-70% of original value
6. Original IC location becomes empty (no value left)

#### Evacuation failure

If enemy captures IC during evacuation:
- Evacuation halts
- Materials in transit go to enemy (per AWAW Rule 37.6)
- Player loses BP + reputation

#### Scorched-earth option

Alternative: player destroys IC instead of letting enemy capture it.
- IC kind=Destroyed; produces 0
- BP value to player = 25% of original (salvaged scrap)
- Reputation -5 with player's faction
- Enemy gets 0
- Per AWAW Rule 37.5

#### Storyteller events

- "Enemy 3 hexes from Magnitogorsk Steelworks — evacuate or defend?"
- "Magnitogorsk Steelworks evacuation 60% complete — defend the convoy!"
- "Magnitogorsk Steelworks captured — economic crisis"
- "Scorched-earth at Magnitogorsk — refugees flee"

#### Configuration

```ron
GrandStrategyICConfig (
    enable_industrial_center_evacuation: false,         // master switch
    default_ics_per_world: 5,
    default_ic_bp_value_range: (50, 500),
    default_evacuation_time_days_range: (3, 7),
    default_evacuation_cost_pct: 0.3,                  // 30% of IC value
    default_evacuation_retention_pct: 0.5,             // 50% of value retained
    enable_scorched_earth: true,
    scorched_earth_salvage_pct: 0.25,
)
```

#### Replay events

- `ic.placed { ic_id, name, world_id, position, kind, bp_value, current_owner }`
- `ic.evacuation_started { ic_id, target_region, cost }`
- `ic.evacuation_progress_updated { ic_id, progress_pct }`
- `ic.evacuated { ic_id, original_value, retained_value }`
- `ic.captured { ic_id, by_faction, original_value }`
- `ic.scorched_earth { ic_id, salvage_value }`
- `ic.destroyed_in_evacuation { ic_id, materials_lost }`
```

Add acceptance criteria:

```gherkin
Scenario: IC evacuation disabled by default
  Given server.ron with no awaw_rulesets block
  Then ICs don't have evacuation state (they just exist as production buildings)

Scenario: Player evacuates threatened IC
  Given enable_industrial_center_evacuation=true + IC at risk of capture
  When player invokes act.faction.evacuate_ic
  Then ic.evacuation_started fires
  After 3 days: ic.evacuated fires with retained_value=50% of original
  And IC at target_region produces 50% of original value

Scenario: Enemy captures IC during evacuation
  Given IC mid-evacuation + enemy reaches IC location
  When evacuation interrupted:
    Then ic.captured fires
    And materials in transit go to enemy

Scenario: Scorched-earth option
  Given IC about to fall to enemy
  When player chooses scorched-earth:
    Then ic.scorched_earth fires
    And player BP += IC value × 0.25
    And enemy gets 0 from IC
    And reputation -5

Scenario: AI faction also evacuates ICs
  Given AI faction with IC at risk
  Then AI auto-decides per storyteller (Cassandra=evacuate, Randy=random)
  And AI evacuates per same rules as player

Scenario: Storyteller events on IC actions
  Given IC under threat
  When storyteller fires:
    Then events surface: "Evacuate Magnitogorsk?" / "Defense ongoing" / "Captured!"
  Plain-language captions via M3B viewer
```

### Acceptance criteria for Edit 5.7

```bash
grep -q "Industrial Center Evacuation" specs/active/M11.5.md && echo "PASS: IC evacuation section" || echo "FAIL"
grep -q "AWAW Rule 37" specs/active/M11.5.md && echo "PASS: AWAW Rule 37 cited" || echo "FAIL"
grep -q "EvacuationState" specs/active/M11.5.md && echo "PASS: schema specified" || echo "FAIL"
grep -q "Scenario: Player evacuates threatened IC" specs/active/M11.5.md && echo "PASS: evacuation scenario" || echo "FAIL"
grep -q "Scenario: Scorched-earth option" specs/active/M11.5.md && echo "PASS: scorched-earth scenario" || echo "FAIL"
```

### Commit message for Edit 5.7

```
specs: Edit 5.7 — add industrial center evacuation to M11.5 (AWAW Rule 37; opt-in)

Replaces all-or-nothing territory model. ICs are large named production
hubs (factories / smelters / reactors / labs / refineries / workshops)
that can be:
- Captured (lost forever)
- Evacuated (move materials to safer region; 30-70% value retained)
- Scorched-earth (destroyed; 25% salvage; enemy gets 0)

Per Soviet 1941-42 historical model. Storyteller surfaces evacuation
crises as narrative events.

Opt-in per server. Default OFF.

- specs/active/M11.5.md modified
```

---

## Edit 5.8 — Add Comprehensive AWAW Ruleset Toggle Tree to M9.10

### Problem

All Tier 5 edits add new mechanics that must be opt-in per server. But M9.10 doesn't have a structured place for these toggles. Without a comprehensive toggle tree, server admins won't know what's available + 4 preset configurations (vanilla / classic_upkeep / awaw_lite / awaw_full) aren't easily discoverable.

### Fix

Extend M9.10 with a comprehensive `grand_strategy.*` config block + 4 preset server.ron templates.

### Files to modify

| File | Action |
|---|---|
| `specs/active/M9.10.md` | **MODIFY** (add Grand Strategy config block + 4 preset templates) |

### Step 1: Modify `specs/active/M9.10.md`

Find the **`server.ron` — complete server admin config** section. Add a new sub-section at the end:

```markdown
### Grand Strategy Config Block (Edit 5.8; references Tier 5 features)

Comprehensive grand-strategy + AWAW-inspired feature toggles. All Tier 5 features are opt-in via this block.

```ron
ServerConfig (
    ...
    grand_strategy: (
        // M7.3 — Upkeep Economy (foundation; opt-in)
        enable_upkeep_economy: false,
        upkeep_cycle_period: "in_game_day",
        upkeep_multiplier: 1.0,
        bp_upkeep_multiplier: 1.0,
        power_upkeep_multiplier: 1.0,
        parts_upkeep_multiplier: 1.0,
        food_upkeep_multiplier: 1.0,
        fuel_upkeep_multiplier: 1.0,
        oxygen_upkeep_multiplier: 1.0,
        coolant_upkeep_multiplier: 1.0,
        enable_bankruptcy_cascade: true,
        bankruptcy_day_thresholds: (1, 3, 7, 14, 30),
        enable_emergency_mobilization: true,
        enable_allied_bp_grants: true,
        enable_austerity_auto_apply: true,
        enable_supply_line_disruption: true,
        default_starting_hq_pool_bp: 1000,
        
        // M7.4 — Strategy Phase + Goals (decision layer; opt-in)
        enable_strategy_phase: false,
        enable_goal_system: false,
        max_concurrent_goals_per_cycle: 3,
        goal_stake_reward_multiplier: 1.0,
        goal_stake_penalty_multiplier: 1.0,
        ai_strategy_phase_enabled: true,            // when global enabled
        ai_goal_system_enabled: true,
        
        // M7.1.5 — Inter-Faction Intelligence (4 subsystems; opt-in)
        awaw_rulesets: (
            enable_faction_intelligence: false,
            enable_codebreaking: true,
            enable_spy_rings: true,
            enable_covert_operations: true,
            enable_counter_intelligence: true,
            codebreaking_level_5_reveals_ai_strategy: true,
            
            // Edit 5.4 — Code-Name Research Secrecy (AWAW Rule 41.5)
            enable_code_name_research_secrecy: false,
            enable_decoy_code_names: true,
            
            // Edit 5.5 — Faction Resistance Levels (AWAW Rule 60)
            enable_faction_resistance_levels: false,
            level_minus_1_threshold_bp_lost: 20,
            level_minus_2_threshold_bp_lost: 40,
            level_minus_3_threshold_bp_lost: 60,
            level_minus_4_threshold_bp_lost: 80,
            level_surrender_threshold_bp_lost: 100,
            enable_resistance_recovery: true,
            
            // Edit 5.6 — Strategic Campaign Turn Sequence (AWAW Rule 8)
            enable_campaign_turn_sequence: false,
            sync_mode: "async",                     // "async" | "hybrid_sync" | "full_sync"
            
            // Edit 5.7 — Industrial Center Evacuation (AWAW Rule 37)
            enable_industrial_center_evacuation: false,
            default_ics_per_world: 5,
            enable_scorched_earth: true,
            
            // AWAW Rule 36 — Mobilization (Phase Transitions)
            enable_mobilization_phases: false,      // civilian → military shift per year
            
            // AWAW Rule 35.31 — BRP-Oil Coupling
            enable_brp_growth_oil_coupling: true,   // oil shortage cuts growth rate 5% per missing
            
            // AWAW Rule 49.31 — One-Third DP Limit
            enable_one_third_dp_limit: false,
            
            // AWAW Rule 49.5 — Lesser Diplomatic Result Downgrade
            enable_lesser_diplomatic_result_downgrade: false,
            
            // AWAW Rule 49.21 — Secret Diplomatic Allocation
            enable_secret_diplomatic_allocation: false,
        ),
    ),
)
```

### 4 Preset Server.ron Templates

Per M11.4 (self-hosted deployment) — 4 launch presets covering common server configurations:

#### 1. `server.ron.template-vanilla` (default)

```ron
ServerConfig (
    server_name: "Vanilla Corefall Server",
    server_motd: "Classic Corefall experience",
    grand_strategy: (
        // ALL grand strategy features OFF
        enable_upkeep_economy: false,
        enable_strategy_phase: false,
        enable_goal_system: false,
        awaw_rulesets: (
            enable_faction_intelligence: false,
            enable_code_name_research_secrecy: false,
            enable_faction_resistance_levels: false,
            enable_campaign_turn_sequence: false,
            enable_industrial_center_evacuation: false,
            enable_mobilization_phases: false,
            enable_brp_growth_oil_coupling: false,
            enable_one_third_dp_limit: false,
            enable_lesser_diplomatic_result_downgrade: false,
            enable_secret_diplomatic_allocation: false,
        ),
    ),
)
```

#### 2. `server.ron.template-classic-upkeep`

```ron
ServerConfig (
    server_name: "Classic Upkeep Server",
    server_motd: "PvE survival with economic upkeep — vanilla feel + resource management",
    grand_strategy: (
        // Only M7.3 + M7.4 enabled
        enable_upkeep_economy: true,
        upkeep_cycle_period: "in_game_day",
        enable_bankruptcy_cascade: true,
        enable_strategy_phase: true,
        enable_goal_system: true,
        // AWAW rulesets all OFF
        awaw_rulesets: (
            enable_faction_intelligence: false,
            enable_code_name_research_secrecy: false,
            enable_faction_resistance_levels: false,
            enable_campaign_turn_sequence: false,
            enable_industrial_center_evacuation: false,
            enable_brp_growth_oil_coupling: true,    // single AWAW rule: oil-economy coupling
        ),
    ),
)
```

#### 3. `server.ron.template-awaw-lite`

```ron
ServerConfig (
    server_name: "AWAW-Lite Corefall Server",
    server_motd: "Grand strategy depth without full AWAW complexity — focus on intelligence + resistance + IC evacuation",
    grand_strategy: (
        enable_upkeep_economy: true,
        enable_strategy_phase: true,
        enable_goal_system: true,
        awaw_rulesets: (
            enable_faction_intelligence: true,           // codebreaking + spy rings + covert ops + counter-intel
            enable_code_name_research_secrecy: true,     // public dice, hidden intent
            enable_faction_resistance_levels: true,      // graduated collapse
            enable_industrial_center_evacuation: true,   // evacuate ICs
            enable_brp_growth_oil_coupling: true,
            // Heavier AWAW rules OFF
            enable_campaign_turn_sequence: false,        // continuous gameplay
            enable_mobilization_phases: false,
            enable_one_third_dp_limit: false,
            enable_lesser_diplomatic_result_downgrade: false,
            enable_secret_diplomatic_allocation: false,
        ),
    ),
)
```

#### 4. `server.ron.template-awaw-full`

```ron
ServerConfig (
    server_name: "Full AWAW Corefall Server",
    server_motd: "Maximum grand strategy depth — every AWAW rule active",
    grand_strategy: (
        enable_upkeep_economy: true,
        enable_strategy_phase: true,
        enable_goal_system: true,
        awaw_rulesets: (
            enable_faction_intelligence: true,
            enable_code_name_research_secrecy: true,
            enable_faction_resistance_levels: true,
            enable_campaign_turn_sequence: true,         // 9-phase turn structure
            sync_mode: "hybrid_sync",
            enable_industrial_center_evacuation: true,
            enable_mobilization_phases: true,            // civilian → military shifts
            enable_brp_growth_oil_coupling: true,
            enable_one_third_dp_limit: true,             // forces diversification
            enable_lesser_diplomatic_result_downgrade: true,
            enable_secret_diplomatic_allocation: true,    // hidden DP allocation
        ),
    ),
)
```

### Configurable lock semantics

Server admins can LOCK Tier 5 toggles against player override using M9.10's existing tier-locking:

```ron
grand_strategy: (
    enable_upkeep_economy: (value: true, locked: true, reason: "PvE Survival baseline"),
    enable_faction_intelligence: (value: true, locked: true, reason: "Competitive PvP requires intel parity"),
)
```

Per M9.10's 7-tier hierarchy: server-locked Tier 5 toggles cannot be overridden by player profile or session config.
```

Add acceptance criteria at the end of M9.10:

```gherkin
Scenario: 4 preset templates ship + validate
  Given content/templates/server.ron.template-{vanilla, classic-upkeep, awaw-lite, awaw-full}
  When `cargo run -p cf-mod -- validate content/templates/` runs
  Then all 4 templates validate against M9.10's settings schema
  And each represents a distinct grand-strategy configuration

Scenario: server.ron template inheritance + override
  Given player starts with server.ron.template-awaw-lite
  When admin edits server.ron to disable enable_industrial_center_evacuation:
    Then resolved config has enable_industrial_center_evacuation=false
    And other AWAW-lite settings remain enabled

Scenario: All grand_strategy settings have schema entries
  Given M9.10 closure
  Then all grand_strategy.* + grand_strategy.awaw_rulesets.* keys exist in settings-schema.ron
  And each has typed schema + default + range + description + owner + lockable_by tiers

Scenario: Tier 5 toggles inherit M9.10's lock semantics
  Given server.ron: grand_strategy.enable_upkeep_economy=(value: true, locked: true)
  When player attempts to override via runtime cfctl:
    Then settings.override_rejected fires with reason="server-locked"
    And upkeep economy stays enabled
```

### Acceptance criteria for Edit 5.8

```bash
grep -q "Grand Strategy Config Block" specs/active/M9.10.md && echo "PASS: grand strategy section" || echo "FAIL"
grep -q "server.ron.template-vanilla" specs/active/M9.10.md && echo "PASS: vanilla template" || echo "FAIL"
grep -q "server.ron.template-classic-upkeep" specs/active/M9.10.md && echo "PASS: classic-upkeep template" || echo "FAIL"
grep -q "server.ron.template-awaw-lite" specs/active/M9.10.md && echo "PASS: awaw-lite template" || echo "FAIL"
grep -q "server.ron.template-awaw-full" specs/active/M9.10.md && echo "PASS: awaw-full template" || echo "FAIL"
grep -q "enable_upkeep_economy" specs/active/M9.10.md && echo "PASS: upkeep toggle" || echo "FAIL"
grep -q "enable_faction_intelligence" specs/active/M9.10.md && echo "PASS: intelligence toggle" || echo "FAIL"
grep -q "Scenario: 4 preset templates ship" specs/active/M9.10.md && echo "PASS: preset validation scenario" || echo "FAIL"
```

### Commit message for Edit 5.8

```
specs: Edit 5.8 — add grand strategy toggle tree to M9.10 (opt-in wiring)

All Tier 5 features (M7.3 upkeep, M7.4 strategy phase, M7.1.5 intel,
plus 5 AWAW rule extensions) need server config toggles. Added
comprehensive grand_strategy.* + grand_strategy.awaw_rulesets.* block
to M9.10's server.ron schema.

4 preset templates:
- vanilla: all OFF (classic Corefall)
- classic-upkeep: upkeep + strategy ON; AWAW rules OFF
- awaw-lite: + intelligence + code names + resistance + IC evacuation
- awaw-full: + 9-phase turns + mobilization + 1/3 DP limit + downgrades + secret diplomacy

Per M9.10's 7-tier locking: admins can lock Tier 5 toggles against player
override.

- specs/active/M9.10.md modified (Grand Strategy Config Block added)
- 4 acceptance scenarios added (preset validation, inheritance, schema completeness, lock semantics)
```

---

## Tier 5 — Full acceptance criteria

Run from `/Users/erol/projects/corefall/`:

```bash
cd /Users/erol/projects/corefall

# Edit 5.1 (M7.3 — Upkeep Economy)
test -f specs/active/M7.3.md
grep -q "Upkeep Economy" specs/active/M7.3.md
grep -q "UpkeepProfile" specs/active/M7.3.md
grep -q "FactionEconomy" specs/active/M7.3.md
grep -q "bankruptcy_day_30" specs/active/M7.3.md

# Edit 5.2 (M7.4 — Strategy Phase + Goals)
test -f specs/active/M7.4.md
grep -q "Strategy Phase + Goals" specs/active/M7.4.md
grep -q "5 stances" specs/active/M7.4.md
grep -q "8 launch goal templates" specs/active/M7.4.md
grep -q "3 stake tiers" specs/active/M7.4.md

# Edit 5.3 (M7.1.5 — Inter-Faction Intelligence)
test -f specs/active/M7.1.5.md
grep -q "Inter-Faction Intelligence" specs/active/M7.1.5.md
grep -q "Codebreaking" specs/active/M7.1.5.md
grep -q "Spy Rings" specs/active/M7.1.5.md
grep -q "Covert Operations" specs/active/M7.1.5.md
grep -q "Counter-Intelligence" specs/active/M7.1.5.md

# Edit 5.4 (M7.8 — Code-Name Research Secrecy)
grep -q "Code-Name Research Secrecy" specs/active/M7.8.md
grep -q "AWAW Rule 41.5" specs/active/M7.8.md

# Edit 5.5 (M11.7 — Faction Resistance Levels)
grep -q "Faction Resistance Levels" specs/active/M11.7.md
grep -q "AWAW Rule 60" specs/active/M11.7.md

# Edit 5.6 (M7 — Strategic Campaign Turn Sequence)
grep -q "Strategic Campaign Turn Sequence" specs/active/M7.md
grep -q "AWAW Rule 8" specs/active/M7.md

# Edit 5.7 (M11.5 — Industrial Center Evacuation)
grep -q "Industrial Center Evacuation" specs/active/M11.5.md
grep -q "AWAW Rule 37" specs/active/M11.5.md

# Edit 5.8 (M9.10 — Grand Strategy Toggle Tree)
grep -q "Grand Strategy Config Block" specs/active/M9.10.md
grep -q "server.ron.template-vanilla" specs/active/M9.10.md
grep -q "server.ron.template-classic-upkeep" specs/active/M9.10.md
grep -q "server.ron.template-awaw-lite" specs/active/M9.10.md
grep -q "server.ron.template-awaw-full" specs/active/M9.10.md

# Active spec count
test "$(ls specs/active/M*.md | wc -l | tr -d ' ')" = "46"
grep -q "active%20specs-46" README.md

# Workspace still builds
cd game && cargo build && cargo clippy --all-targets -- -D warnings
cd ..

echo "TIER 5 — ALL CHECKS PASS"
```

All checks must complete without errors.

---

## Tier 5 PR template

**Title:** `specs: tier-5 grand-strategy economy + AWAW-inspired strategic layer (opt-in per server)`

**Body:**

```markdown
## Summary

Tier 5 of the spec coherence pass per `specs/COHERENCE-PLAN.md`. Adds the **grand-strategy economy + AWAW-inspired strategic layer**:

1. **Edit 5.1 — NEW M7.3 — Upkeep Economy** — per-cycle drain (BP + power + parts + food + fuel); faction-wide HQ + per-base local pools; bankruptcy cascade Day 1 → 3 → 7 → 14 → 30; rescue mechanisms
2. **Edit 5.2 — NEW M7.4 — Strategy Phase + Goals** — per-cycle stance/focus/logistics choices + 1-3 goals with 3-tier stake
3. **Edit 5.3 — NEW M7.1.5 — Inter-Faction Intelligence** — 4 subsystems per AWAW Rules 44-48: codebreaking + spy rings + covert ops + counter-intelligence
4. **Edit 5.4** — Code-Name Research Secrecy to M7.8 (AWAW Rule 41.5) — public dice, hidden project intent
5. **Edit 5.5** — Faction Resistance Levels to M11.7 (AWAW Rule 60) — 5-level graduated collapse
6. **Edit 5.6** — 9-Phase Strategic Campaign Turn Sequence to M7 (AWAW Rule 8) — Research → Diplomacy → DOW → Movement → Combat → Post-Combat → Construction → Redeployment → End-of-Turn
7. **Edit 5.7** — Industrial Center Evacuation to M11.5 (AWAW Rule 37) — evacuate threatened ICs; scorched-earth option
8. **Edit 5.8** — Comprehensive Grand Strategy Toggle Tree + 4 preset server.ron templates to M9.10 (opt-in wiring)

## Active spec count

- Before: 43 (after Tier 4)
- After: 46 (added M7.3 + M7.4 + M7.1.5)

## Default behavior (CRITICAL)

**All Tier 5 features default to OFF in vanilla server.ron.** Server admins flip toggles in `server.ron` per their gameplay style.

Per M9.10's 7-tier settings hierarchy:
- Engine defaults: ALL Tier 5 features OFF
- Server admin flips via `server.ron`
- Scenario configs can lock toggles
- Players cannot override server-locked Tier 5 settings

### 4 launch preset templates

- `vanilla` — all OFF; classic Corefall
- `classic-upkeep` — upkeep + strategy; no AWAW
- `awaw-lite` — + intelligence + code names + resistance + IC evacuation
- `awaw-full` — + 9-phase turns + mobilization + 1/3 DP limit + downgrades + secret diplomacy

## AI parity

Per the user's confirmed design knob: AI factions follow same rules as players. AI factions:
- Drain upkeep from HQ pool (exploitable via supply line cuts)
- Pick stance + goals per turn (revealable via M7.1.5 intel)
- Use intelligence subsystems against player
- Can bankrupt + go through resistance level cascade
- Evacuate threatened ICs

## Verification

Ran all acceptance checks from `COHERENCE-TIER-5.md` § Tier 5 — Full acceptance criteria. All PASS.

`cargo build` + `cargo clippy --all-targets -- -D warnings` both green.

## Master checklist update

Updated `COHERENCE-PLAN.md` master checklist to add 5 new boxes (19 → 24 total):

- Box 19: M7.3 — Upkeep Economy milestone exists
- Box 20: M7.4 — Strategy Phase + Goals milestone exists
- Box 21: M7.1.5 — Inter-Faction Intelligence milestone exists
- Box 22: 4 preset server.ron templates ship + validate
- Box 23: All AWAW rulesets opt-in via M9.10 settings hierarchy
- Box 24: Default vanilla server.ron has all Tier 5 features OFF (regression test)

## Next

After this PR merges, M2.2A implementation can proceed with optional Tier 5 features available per-server. The coherence pass is fully complete; the canonical roadmap covers **57 closed/directional decision records**, **46 sequenced milestones** (3 closed + 43 active in `specs/active/M0.5..M12.md`), and 4 launch server preset templates.
```

---

## Done with Tier 5

Once the PR merges:
- ✅ M7.3 (Upkeep Economy) ships per-cycle drain + bankruptcy cascade
- ✅ M7.4 (Strategy Phase + Goals) ships per-cycle decision layer
- ✅ M7.1.5 (Inter-Faction Intelligence) ships 4 subsystems per AWAW
- ✅ M7 has 9-phase Strategic Campaign Turn (AWAW Rule 8)
- ✅ M7.8 has code-name research secrecy (AWAW Rule 41.5)
- ✅ M11.5 has IC evacuation (AWAW Rule 37)
- ✅ M11.7 has Faction Resistance Levels (AWAW Rule 60)
- ✅ M9.10 ships comprehensive grand_strategy.* config block + 4 preset templates
- ✅ All features opt-in per server (default OFF in vanilla)
- ✅ AI factions follow same rules (full parity)
- ✅ 46 active specs

---

## Master coherence pass complete

When Tier 1 + Tier 2 + Tier 3 + Tier 4 + Tier 5 PRs all merge:

1. ✅ M7.8 has no hard dependency on M8.6 (Tier 1)
2. ✅ SmelterFurnace appears in exactly one spec (Tier 1)
3. ✅ M2.2A inventory has 3 reserved tank slots (Tier 1)
4. ✅ M5.8 references M7.6 / M5.9 / M5.10 for battery / tank / race-env data (Tier 1)
5. ✅ M7 + M7.1 + M7.2 each cover one coherent scope (Tier 2)
6. ✅ M11.5 + M11.6 + M11.7 each cover one coherent scope (Tier 2)
7. ✅ Boss schema defined once in M7, referenced elsewhere (Tier 2)
8. ✅ M5.7 has 22 afflictions (was 18; added hunger/thirst/sleep_dep/sanity_low) (Tier 2)
9. ✅ M2.5 + M2.5-SCHEMA split cleanly (Tier 3)
10. ✅ Storyteller API documented in M7 (Tier 3)
11. ✅ Damage-model specs have cross-reference headers (Tier 3)
12. ✅ M11.5 procgen acceptance covers all 12 worlds (Tier 3)
13. ✅ M0.5 — Schema Locks milestone exists (Tier 4)
14. ✅ M11.4 — Self-Hosted Server Deployment milestone exists (Tier 4)
15. ✅ README badge shows 46 active specs
16. ✅ README BP table reflects all new + split milestones
17. ✅ `cargo build` + `cargo test` + `cargo clippy` all green
18. ✅ `cargo run -p cf-mod -- validate content/` exits 0 (no spec/content drift)
19. ✅ M7.3 — Upkeep Economy milestone exists (Tier 5)
20. ✅ M7.4 — Strategy Phase + Goals milestone exists (Tier 5)
21. ✅ M7.1.5 — Inter-Faction Intelligence milestone exists (Tier 5)
22. ✅ 4 preset server.ron templates ship + validate (Tier 5)
23. ✅ All AWAW rulesets opt-in via M9.10 settings hierarchy (Tier 5)
24. ✅ Default vanilla server.ron has all Tier 5 features OFF (Tier 5 regression test)

All 24 boxes checked → spec coherence pass complete → ready for M2.2A implementation + optional Tier 5 features per server preference.
