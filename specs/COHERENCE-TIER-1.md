# Coherence Tier 1 — Critical Fixes

**Status:** `active` — must complete BEFORE M2.2A implementation starts
**Prerequisite:** None
**Estimated effort:** AI-scale 30-60 minutes (single PR, 4 commits)
**Output:** 1 PR titled `specs: tier-1 coherence fixes (dependency inversion + data ownership + tank slots)`

---

## Goals

Fix 4 hard issues that will block M2.2A implementation if left in place:

1. **Edit 1.1** — Fix the M7.8 ↔ M8.6 dependency inversion by splitting M8.6 into M7.6.5 + M8.6
2. **Edit 1.2** — Unify SmelterFurnace/EnrichmentReactor in M7.8 (remove from M8.6)
3. **Edit 1.3** — Add 3 tank slots to M2.2A inventory spec (placeholder reservation)
4. **Edit 1.4** — Tighten M5.8 by moving battery / tank / race-env data tables to canonical owners

After Tier 1 PR merges:
- M7.8 closes without needing M8.6
- One source of truth per data table (battery tiers, tank tiers, race-env matrix)
- M2.2A inventory has reserved slots for tanks (M5.8/M5.9 fill at their milestones)
- 35 → 36 active specs

---

## Edit 1.1 — Fix M7.8 ↔ M8.6 dependency inversion

### Problem

`specs/active/M7.8.md` line ~150 (in `## Dependencies` section) lists `M8.6 (mining)` as `must close`. M7.8 ships in BP7. M8.6 ships in BP8. The player can't fabricate steel at M7.8 close because there's no source of iron ore.

This is a hard dependency inversion — implementer at M7.8 can't satisfy the dep.

### Fix — Split M8.6 into M7.6.5 + M8.6

Create a new milestone **M7.6.5 — Basic Mining + Smelting** that ships with M7.8 in BP7. It contains the minimum mining + refining surface needed for M7.8 crafting. M8.6 keeps the advanced mining tools, server-ledger replication, and AI-MINE-A acceptance suite for BP8.

### Files to modify

| File | Action |
|---|---|
| `specs/active/M7.6.5.md` | **CREATE** |
| `specs/active/M8.6.md` | **MODIFY** (trim to advanced-only) |
| `specs/active/M7.8.md` | **MODIFY** (update dependency list) |
| `README.md` | **MODIFY** (add to BP7; update active spec count) |

### Step 1: Create `specs/active/M7.6.5.md`

Create a new file with this exact structure:

```markdown
# M7.6.5 — Basic Mining + Smelting (Foundation Tier)

## Status

`active`

## Intent

**M7.6.5 is the basic mining + smelting foundation milestone** — the minimal extraction + refining surface needed for M7.8's crafting tier ladder to work. After M7.6.5, players can survey ore deposits, drill basic ores, refine them at a RefiningStation, and feed M7.8's fabrication chain.

**This milestone exists to break the M7.8 ↔ M8.6 dependency inversion** — the advanced mining tools, server-ledger replication, and AI-MINE-A acceptance suite remain at M8.6 (BP8) but the bare-minimum survey + drill + refine surface ships at M7.6.5 (BP7) alongside M7.8.

M7.6.5 promise: **"you can mine iron, smelt steel, and craft your first rifle — without waiting for advanced mining tech to ship."**

## Player-facing behavior

### 4 launch mining tools (subset of M8.6's 9)

| Tool | Function | Origin compatibility | Power source |
|---|---|---|---|
| `Sampler` | Survey ore deposits (reveals composition + yield) | Human / Android / Robot | Hand-held / batteries |
| `LightDigger` | Existing M1 tool; mines soft ores (dirt-grade) | All origins | (M1 baseline) |
| `HeavyDrill` | Drills harder ores (concrete/metal-grade); slow | All origins; recommends Heavy chassis | Battery / power |
| `RefiningStation` | Process raw ore → refined material | Base station (not portable) | Power grid |

**Advanced tools deferred to M8.6:** `CoreDrill` (deep drilling for rare ores) + `EnrichmentReactor` (now moved to M7.8) + `OreCargoBay` (storage bin) + `ConveyorBelt` (automation).

### 7 launch ores (subset of M8.6's 12)

| Ore | Distribution | Yield | Use |
|---|---|---|---|
| `iron` | Veined (long deposits) | Common | Steel, base hardware, ammo |
| `copper` | Streak (thin lines) | Common | Wiring, electronics |
| `coal` | Streak | Common | Smelting, alloys |
| `lead` | Pocketed | Common | Bullets, shielding |
| `nickel` | Streak | Common | Alloys |
| `tin` | Pocketed | Common | Alloys, plating |
| `ice` | Uniform (water-bearing rock; cold worlds) | Variable | Water source (Mimas / Europa) |

**Advanced ores deferred to M8.6:** `gold` (pocketed; rare currency) + `uranium` (deep; reactor fuel) + `sulfur` (volcanic) + `lithium` (battery) + `titanium` (deep armor).

### Per-world ore deposit generator (subset)

M7.6.5 ships ore distribution for **3 launch worlds** (Earth + Mars + Mimas) — the same worlds M11.5 PvE Survival ships at launch. M8.6 extends to all 12 worlds.

Each world's deposit pattern is data-driven via `content/worlds/<world_id>.world.ron`:

- **Earth** → all 7 ores (mostly surface)
- **Mars** → iron + copper + coal + ice (deep)
- **Mimas** → ice + nickel + tin (cryogenic)

### AI miner doctrine (subset)

M7.6.5 ships **AI-MINE-A-01..05** (the first 5 of 8 AI-MINE-A acceptance tests). The remaining 3 tests (AI-MINE-A-06 retreat, AI-MINE-A-07 EnvironmentSignal, AI-MINE-A-08 reason labels) ship at M8.6.

| Test | What it proves | Owner |
|---|---|---|
| `AI-MINE-A-01` | AI surveys ore deposit (Sampler tool) | **M7.6.5** |
| `AI-MINE-A-02` | AI drills with appropriate tool per ore type | **M7.6.5** |
| `AI-MINE-A-03` | AI refines raw ore at RefiningStation | **M7.6.5** |
| `AI-MINE-A-04` | AI smelts refined at SmelterFurnace | **M7.6.5** (smelter is owned by M7.8) |
| `AI-MINE-A-05` | AI hauls smelted output to OreCargoBay | **M7.6.5** (manual haul; auto via ConveyorBelt is M8.6) |
| `AI-MINE-A-06` | AI retreats when threatened during mining | M8.6 |
| `AI-MINE-A-07` | AI reads EnvironmentSignal (vacuum suit check) | M8.6 |
| `AI-MINE-A-08` | AI emits reason labels for mining decisions | M8.6 |

### Server-authoritative resource ledger (basic)

M7.6.5 ships a **single-server ledger** (per-shard mining counts). The cross-shard replication + audit-log for anti-cheat is deferred to M8.6 (which is the milestone that owns server-grade extraction).

### Content roster at M7.6.5

| Content | Roster |
|---|---|
| **Mining tools** | 4 launch tools (Sampler / LightDigger / HeavyDrill / RefiningStation) |
| **Ores** (toward 12) | 7 launch ores cumulative |
| **Worlds with ore distribution** | 3 launch worlds (Earth / Mars / Mimas) |

## Crates / modules touched

| Crate | Status | What |
|---|---|---|
| `cf-equipment::mining` | NEW | 4 mining tools + role data |
| `cf-material::ores` | MODIFY | 7 ores promoted from stub; per-ore properties |
| `cf-ore-generator` | NEW (basic) | Per-world deposit generator for 3 launch worlds |
| `cf-ai::mining_doctrine` | NEW (basic) | AI miner doctrine + AI-MINE-A-01..05 suite |
| `cf-replay` | MODIFY | `mining.*` event category (basic — extracted, refined, smelted, hauled) |
| `cf-server-persistence::ledger` | NEW (basic) | Single-shard mining ledger |

## Acceptance criteria (AI-MINE-A-01..05 subset)

```gherkin
Scenario: AI-MINE-A-01 — AI surveys ore deposit
  Given AI bot with Sampler tool + ore deposit
  When bot uses Sampler at deposit:
    Then mining.deposit_surveyed fires with composition + yield
  And bot has data for next decision

Scenario: AI-MINE-A-02 — AI drills with appropriate tool
  Given AI bot + iron deposit (concrete-grade)
  When bot chooses tool:
    Then bot selects HeavyDrill (per ore hardness)
  When drilling:
    Then mining.drill_started fires
    And ore extracted over time

Scenario: AI-MINE-A-03 — AI refines raw ore
  Given AI bot with raw iron + RefiningStation
  When bot interacts with station:
    Then mining.refining_started fires
    After cooldown: mining.refining_completed fires
    And refined_iron in output bin

Scenario: AI-MINE-A-04 — AI smelts refined (via M7.8 SmelterFurnace)
  Given refined iron + coal + M7.8's SmelterFurnace
  When bot loads furnace + activates:
    Then mining.smelt_started fires
    After cooldown: steel produced
  Cross-DR: integrates with M5.9 atmospherics (smelter requires heat per Stationeers)

Scenario: AI-MINE-A-05 — AI hauls output (manual)
  Given smelted output + storage location
  When bot hauls:
    Then mining.hauled fires (bot moves output manually)
  (Auto-haul via ConveyorBelt deferred to M8.6)

Scenario: 7 launch ores load + smelt
  Given 7 ore types in registry (iron / copper / coal / lead / nickel / tin / ice)
  When each ore mined → refined → smelted:
    Then all 7 produce expected output material

Scenario: 3 launch worlds have ore distribution
  Given content/worlds/{earth,mars,mimas}.world.ron
  Then each lists 4-7 ore deposit zones per world
  And cf-mod validates ore_distribution schema

Scenario: M7.8 crafting works with M7.6.5 output
  Given M7.6.5's refined iron + M7.8's iron_rifle_m1 recipe
  When player crafts at workbench:
    Then craft completes; rifle in inventory
  (Validates the M7.6.5 → M7.8 chain end-to-end)
```

## Dependencies

- M2 (terrain — extraction carves terrain) must close
- M5 (chassis — drill tools can be chassis-mounted) must close
- M5.6 (material kernel — ore-as-material) must close OR concurrent
- M5.9 (atmospherics — smelter requires heat per Stationeers) must close
- M7 (campaign + base building hosts RefiningStation placement) must close

## Closure procedure

Reference bundle + AI-MINE-A-01..05 tests + 7 ores + 4 tools verified + 3 launch worlds + determinism replay. PASS.

Move `specs/active/M7.6.5.md` → `specs/done/M7.6.5.md`.

## Cross-DR

DR-006 (mining content moddable), DR-007 (ore-as-material), DR-024, **DR-041 (extends the mining scope; closes at M8.6)**.
```

### Step 2: Modify `specs/active/M8.6.md`

Find the `## Intent` section and update it to start with:

```markdown
## Intent

**M8.6 is the advanced mining + extraction milestone** (per DR-041). Builds on M7.6.5's basic mining foundation to ship the full extraction pipeline: deep drilling, advanced refining, server-grade ledger replication, conveyor-belt automation, and the full AI-MINE-A acceptance suite. After M8.6, mining is server-authoritative, mod-extensible, and accessible across all 12 launch worlds.

**M7.6.5 must close first** — M8.6 extends M7.6.5's surface; it does not replace it.

M8.6 promise: **"mining is a first-class server-authoritative gameplay loop — survey ore deposits, drill, refine, smelt, build with the output, defend supply lines from raiders, automate via conveyor belts."**
```

(Delete the old "M8.6 promise" line.)

Find the **9 launch mining tools** table and change it to **5 launch mining tools** (deferred from M7.6.5):

```markdown
### 5 launch mining tools (extending M7.6.5's 4)

Per `spec/mining-and-extraction-model`:

| Tool | Function | Origin compatibility | Power source |
|---|---|---|---|
| `CoreDrill` | Deep drilling for rare ores (uranium / titanium / lithium); chassis-mounted | Light mech / Heavy mech | Reactor power |
| `OreCargoBay` | Storage bin for raw + processed ore | Base station | (passive) |
| `ConveyorBelt` | Moves ore between stations | Base station | Power |
| `AutoExtractor` | Deploys on belt asteroids; auto-mines | Drone | Solar / battery |
| `SeismicSurveyor` | Wide-area ore detection (chunk-scale; per Stationeers parity) | All origins | Battery |

**Tools from M7.6.5 (basic foundation):** `Sampler`, `LightDigger`, `HeavyDrill`, `RefiningStation` — see `specs/active/M7.6.5.md` for the basic mining surface.

**Tools moved to M7.8 (fabrication chain):** `SmelterFurnace`, `EnrichmentReactor` — these are fabrication stations, not mining tools; see `specs/active/M7.8.md` § fabrication stations.
```

Find the **12 launch ores** table and change it to **5 launch ores** (additional; the other 7 are at M7.6.5):

```markdown
### 5 launch ores (extending M7.6.5's 7)

| Ore | Distribution | Yield | Use |
|---|---|---|---|
| `gold` | Pocketed (sparse clusters) | Rare | Currency, electronics |
| `uranium` | Pocketed (deep) | Rare | Reactor fuel, weapons |
| `sulfur` | Veined (volcanic regions) | Variable | Chemicals, gunpowder |
| `lithium` | Pocketed (battery-bearing) | Rare | Battery cells |
| `titanium` | Veined (deep) | Rare | Hardened armor |

**Ores from M7.6.5 (basic):** `iron`, `copper`, `coal`, `lead`, `nickel`, `tin`, `ice`.

**Total at M8.6 close:** 12 ores (cumulative across M7.6.5 + M8.6).
```

Find the **AI-MINE-A 8-test acceptance suite** table and update to mark M7.6.5 ownership:

```markdown
### AI miner doctrine (AI-MINE-A 8-test acceptance suite — extending M7.6.5)

| Test | What it proves | Owner |
|---|---|---|
| `AI-MINE-A-01` | AI surveys ore deposit | M7.6.5 (basic) |
| `AI-MINE-A-02` | AI drills with appropriate tool | M7.6.5 (basic) |
| `AI-MINE-A-03` | AI refines raw ore | M7.6.5 (basic) |
| `AI-MINE-A-04` | AI smelts refined | M7.6.5 (basic; smelter owned by M7.8) |
| `AI-MINE-A-05` | AI hauls smelted output manually | M7.6.5 (basic) |
| `AI-MINE-A-06` | **AI retreats when threatened during mining** | **M8.6** |
| `AI-MINE-A-07` | **AI reads EnvironmentSignal (won't mine in vacuum without seal)** | **M8.6** |
| `AI-MINE-A-08` | **AI emits reason labels for mining decisions** | **M8.6** |
```

Find the `## Dependencies` section and update:

```markdown
## Dependencies

- **M7.6.5 (basic mining foundation; must close first)**
- M8 (mod workbench — mining mission authoring) must close
- M5.6 (material kernel for ore-as-material) must close
- M5.9 (atmospherics — smelter requires heat) must close
- M6.6 (AI environmental competence — for AI-MINE-A-07) must close
- M7 (mission director hosts mining missions) must close
```

### Step 3: Modify `specs/active/M7.8.md`

Find the `## Dependencies` section and update:

**BEFORE:**
```markdown
- M8 (mod workbench — recipe authoring) must close (or concurrent)
- M8.6 (mining — raw ore source) must close
```

**AFTER:**
```markdown
- M8 (mod workbench — recipe authoring) must close (or concurrent)
- **M7.6.5 (basic mining + smelting) must close** — provides ore + refining surface that M7.8 recipes consume; replaces previous hard dep on M8.6
- M8.6 (advanced mining + server ledger) — NOT required at M7.8 close; M8.6 extends M7.6.5 but M7.8 only needs the basic surface
```

### Step 4: Modify `README.md`

Find the active spec count badge:

**BEFORE:**
```markdown
[![Specs](https://img.shields.io/badge/active%20specs-35%20%28M2.2A..M12%29-blueviolet?style=flat-square)](specs/active/)
```

**AFTER:**
```markdown
[![Specs](https://img.shields.io/badge/active%20specs-36%20%28M2.2A..M12%29-blueviolet?style=flat-square)](specs/active/)
```

Find the "38 sequenced milestones" line and update:

**BEFORE:**
```markdown
The canonical roadmap now covers **57 closed/directional decision records**, **38 sequenced milestones** (3 closed + 35 active in `specs/active/M2.2A..M12.md`),
```

**AFTER:**
```markdown
The canonical roadmap now covers **57 closed/directional decision records**, **39 sequenced milestones** (3 closed + 36 active in `specs/active/M2.2A..M12.md`),
```

Find the BP7 row in the Build Points table. Look for the line containing `| BP7 | M7.5 — Base Atmospherics`. Add a new row immediately AFTER M7.5 + BEFORE M7.7:

```markdown
| BP7 | **M7.6.5 — Basic Mining + Smelting (Foundation)** | Planned | Foundation tier for M7.8 crafting chain: 4 launch mining tools (Sampler / LightDigger / HeavyDrill / RefiningStation) + 7 launch ores (iron / copper / coal / lead / nickel / tin / ice) + 3 launch worlds (Earth / Mars / Mimas) ore distribution + AI-MINE-A-01..05 acceptance subset. M8.6 extends with advanced tools + 5 more ores + 9 more worlds + AI-MINE-A-06..08 + server-grade ledger replication. |
```

Find the planning spine reference and update:

**BEFORE:**
```markdown
- **35 active milestone specs** in [`specs/active/M2.2A..M12.md`](specs/active/) — the executable implementation contracts for the gameplay spine. Each is read-by-implementing-agent-only per AGENTS.md.
```

**AFTER:**
```markdown
- **36 active milestone specs** in [`specs/active/M2.2A..M12.md`](specs/active/) — the executable implementation contracts for the gameplay spine. Each is read-by-implementing-agent-only per AGENTS.md.
```

### Acceptance criteria for Edit 1.1

Run from `/Users/erol/projects/corefall/`:

```bash
# Verify M7.6.5.md exists and parses
test -f specs/active/M7.6.5.md && echo "PASS: M7.6.5.md exists" || echo "FAIL"

# Verify M7.8 no longer hard-deps M8.6
! grep -q "^- M8.6 (mining" specs/active/M7.8.md && echo "PASS: M7.8 no longer hard-deps M8.6" || echo "FAIL"

# Verify M8.6 reference M7.6.5 as dependency
grep -q "M7.6.5" specs/active/M8.6.md && echo "PASS: M8.6 references M7.6.5" || echo "FAIL"

# Verify README badge
grep -q "active%20specs-36" README.md && echo "PASS: README badge updated" || echo "FAIL"

# Verify README BP7 includes M7.6.5
grep -q "M7.6.5 — Basic Mining" README.md && echo "PASS: README BP7 lists M7.6.5" || echo "FAIL"

# Verify total active spec count on disk
test "$(ls specs/active/M*.md | wc -l | tr -d ' ')" = "36" && echo "PASS: 36 active spec files" || echo "FAIL: $(ls specs/active/M*.md | wc -l) files found"
```

All 6 checks must print `PASS`.

### Commit message for Edit 1.1

```
specs: Edit 1.1 — split M8.6 into M7.6.5 + M8.6 (fix M7.8 dependency inversion)

M7.8 (crafting) was listing M8.6 (mining) as a hard dependency, but M7.8
ships in BP7 and M8.6 in BP8. Split M8.6 into M7.6.5 (basic mining +
smelting foundation; ships with M7.8 in BP7) and M8.6 (advanced tools +
server-grade ledger; stays in BP8).

- Create specs/active/M7.6.5.md
- Trim specs/active/M8.6.md (remove 4 basic tools, 7 basic ores, 5 basic
  AI-MINE-A tests; reference M7.6.5 as dependency)
- Update specs/active/M7.8.md (depends on M7.6.5, not M8.6)
- Update README.md (badge 35 → 36, BP7 table adds M7.6.5, planning
  spine reference)

Acceptance criteria from COHERENCE-TIER-1.md § Edit 1.1 — all pass.
```

---

## Edit 1.2 — Unify SmelterFurnace + EnrichmentReactor in M7.8

### Problem

`SmelterFurnace` is defined in **both** `specs/active/M7.8.md` (as 1 of 8 fabrication stations) **and** `specs/active/M8.6.md` (as 1 of the 9 mining tools — now 5 after Edit 1.1). Same equipment, two homes. `EnrichmentReactor` has the same problem.

This was partially fixed by Edit 1.1's M8.6 trim, which already moved them out. Edit 1.2 finalizes the unification by ensuring the canonical home is M7.8 and adding explicit cross-references.

### Fix

M7.8 owns the smelter + enrichment reactor (they are fabrication stations, not mining tools — fabrication is the post-extraction step).

M8.6 references M7.8 for smelter access.

### Files to modify

| File | Action |
|---|---|
| `specs/active/M7.8.md` | **MODIFY** (confirm smelter section + add cross-reference note) |
| `specs/active/M8.6.md` | **MODIFY** (verify Edit 1.1's trim is complete + add cross-reference) |

### Step 1: Modify `specs/active/M7.8.md`

Find the **8 launch stations** content. Verify it lists exactly these 8:
1. Workbench (T1)
2. Fabricator (T2)
3. Assembly Line (T2)
4. **SmelterFurnace (T1)** ← canonical home
5. Plasma Forge (T3)
6. **EnrichmentReactor (T3)** ← canonical home
7. Material Lab (M8.5 unlock; T4)
8. Exotic-Matter Forge (T4)

If `SmelterFurnace` and `EnrichmentReactor` are not present in M7.8's station list, add them. Find the section header `**8 launch stations**` and ensure the list above is complete.

Add a new note immediately after the 8-station list:

```markdown
**Cross-reference:** `SmelterFurnace` and `EnrichmentReactor` are owned by M7.8 (fabrication chain). M8.6 (mining) does NOT define these; M8.6 only ships extraction tools (Sampler / LightDigger / HeavyDrill / CoreDrill / RefiningStation / etc.). Smelting + enrichment are post-extraction refining steps and belong to M7.8's fabrication ladder.
```

### Step 2: Verify `specs/active/M8.6.md`

After Edit 1.1, M8.6's 5-tool list should NOT include `SmelterFurnace` or `EnrichmentReactor`. Verify by searching:

```bash
# These should return zero matches:
grep -c "SmelterFurnace" specs/active/M8.6.md   # → expect 0 OR 1 (only as cross-reference)
grep -c "EnrichmentReactor" specs/active/M8.6.md   # → expect 0 OR 1 (only as cross-reference)
```

Add a cross-reference note in M8.6's tools section:

```markdown
**Cross-reference:** `SmelterFurnace` and `EnrichmentReactor` are owned by M7.8 (fabrication stations), NOT by M8.6 (mining tools). Mining produces raw ore; M7.8 handles refining + smelting + enrichment. See `specs/active/M7.8.md` § fabrication stations.
```

### Acceptance criteria for Edit 1.2

```bash
# Verify SmelterFurnace appears only in M7.8 (not M8.6) as a station definition
grep -l "SmelterFurnace.*fabrication station" specs/active/*.md
# Expected output: ONLY specs/active/M7.8.md

# Verify EnrichmentReactor appears only in M7.8
grep -l "EnrichmentReactor.*fabrication station" specs/active/*.md
# Expected output: ONLY specs/active/M7.8.md

# Cross-reference notes added
grep -q "owned by M7.8" specs/active/M8.6.md && echo "PASS: M8.6 cross-ref" || echo "FAIL"
grep -q "M8.6.*does NOT define" specs/active/M7.8.md && echo "PASS: M7.8 cross-ref" || echo "FAIL"
```

### Commit message for Edit 1.2

```
specs: Edit 1.2 — unify SmelterFurnace + EnrichmentReactor in M7.8

These fabrication stations were defined in both M7.8 and M8.6. M7.8 is
the canonical owner (fabrication chain); M8.6 only ships extraction
tools. Added cross-reference notes in both files.
```

---

## Edit 1.3 — Add tank slots to M2.2A inventory

### Problem

`specs/active/M2.2A.md` defines an 8-slot inventory: `primary / secondary / sidearm / tool1 / tool2 / grenade / medical / special`.

M5.8 (BP4) defines gas tank tiers (T0 emergency / T1 compressed / T2 pressurized / T3 cryogenic / T4 closed-loop cycler) that go in **inventory slots**. M11.5 also references vehicle tank slots.

But M2.2A's 8-slot inventory has NO reservation for tanks. Implementer at M2.2A will ship a fully-baked 8-slot system; M5.8 implementer then has to retrofit tank slots.

### Fix

Add **3 tank slots** to M2.2A's inventory as placeholder reservations:
- `tank_primary` — primary breathing tank
- `tank_secondary` — backup breathing tank
- `tank_utility` — utility gas tank (CO2 / volatiles / specialty mix)

At M2.2A close: slots exist but are empty + non-functional. M5.8 defines fillable tanks. M5.9 ticks gas physics.

### Files to modify

| File | Action |
|---|---|
| `specs/active/M2.2A.md` | **MODIFY** (extend inventory to 11 slots + add tank acceptance criterion) |

### Step 1: Modify `specs/active/M2.2A.md`

Find the **8-slot inventory** section (it's in the "### Inventory: 8 slots + weight + drop/pickup" subsection).

**BEFORE:**
```markdown
### Inventory: 8 slots + weight + drop/pickup

**8-slot inventory** (`primary / secondary / sidearm / tool1 / tool2 / grenade / medical / special`) with:
```

**AFTER:**
```markdown
### Inventory: 8 active slots + 3 reserved tank slots + weight + drop/pickup

**8 active slots** (`primary / secondary / sidearm / tool1 / tool2 / grenade / medical / special`) + **3 reserved tank slots** (`tank_primary / tank_secondary / tank_utility`) with:

**Tank slot reservation (M2.2A placeholder; M5.8 + M5.9 fill):**

- `tank_primary` — primary breathing tank (humans/androids organic side)
- `tank_secondary` — backup breathing tank (long-duration missions)
- `tank_utility` — utility gas tank (CO2 / volatiles / specialty mix for non-O2-breathers)

At M2.2A close, tank slots are EMPTY + non-functional (inventory grid shows them as locked icons). The slots emit `inventory.tank_slot_reserved` events for M3A's snapshot path. M5.8 ships the GasTank struct + 5 tier ladder. M5.9 ticks gas physics (PV=nRT, leak rate, decompression). The M2.2A reservation prevents future schema bumps when tanks ship.

**Why 3 tank slots?** Per M5.8: humans need 1 breathing tank + optional backup; androids need 1 organic-side tank (synthetic side immune); robots need 0 tanks (no respiratory system, but may carry utility gas tanks for chemical generators). 3 slots covers the worst case (human/android with backup + utility).
```

Find the **Acceptance criteria** section (`### Inventory: 8 slots + weight + drop/pickup`) and add this scenario at the end:

```gherkin
Scenario: Tank slots reserved (M5.8 + M5.9 forward-compat)
  Given a fresh actor inventory
  Then 3 tank slots exist (tank_primary, tank_secondary, tank_utility)
  And each tank slot has slot_state="locked" (empty placeholder at M2.2A)
  And inventory.tank_slot_reserved fires for each at actor spawn
  And cfctl observe.actor.inventory returns all 11 slots (8 active + 3 reserved tank)
  And the HUD inventory widget shows tank slots as locked icons with tooltip "Reserved — see M5.8 for tank ladder"

Scenario: Tank slots cannot be filled at M2.2A
  Given M2.2A close (no M5.8 producer yet)
  When cfctl tries to insert a tank into tank_primary slot
  Then the engine rejects with reason="tank_slot_locked_at_m2_2a"
  And the tank slot remains empty
  (M5.8 unlocks tank slots when GasTank struct ships)
```

### Acceptance criteria for Edit 1.3

```bash
# Verify M2.2A inventory section mentions 11 slots
grep -q "8 active slots" specs/active/M2.2A.md && echo "PASS: 8 active slots wording" || echo "FAIL"
grep -q "3 reserved tank slots" specs/active/M2.2A.md && echo "PASS: 3 reserved tank slots wording" || echo "FAIL"

# Verify tank slot kinds defined
grep -q "tank_primary" specs/active/M2.2A.md && echo "PASS: tank_primary defined" || echo "FAIL"
grep -q "tank_secondary" specs/active/M2.2A.md && echo "PASS: tank_secondary defined" || echo "FAIL"
grep -q "tank_utility" specs/active/M2.2A.md && echo "PASS: tank_utility defined" || echo "FAIL"

# Verify new acceptance scenarios exist
grep -q "Scenario: Tank slots reserved" specs/active/M2.2A.md && echo "PASS: tank slot reservation scenario" || echo "FAIL"
grep -q "Scenario: Tank slots cannot be filled at M2.2A" specs/active/M2.2A.md && echo "PASS: tank slot locked scenario" || echo "FAIL"
```

### Commit message for Edit 1.3

```
specs: Edit 1.3 — add 3 reserved tank slots to M2.2A inventory

M2.2A defines an 8-slot inventory but M5.8 (BP4) needs gas tank slots
for the GasTank tier ladder. Adding 3 reserved tank slots
(tank_primary, tank_secondary, tank_utility) at M2.2A prevents a
future schema bump when M5.8 ships the GasTank struct + tier ladder.

At M2.2A close: tank slots exist but are locked (non-functional).
M5.8 unlocks them when GasTank ships. M5.9 ticks gas physics.

Added 2 new acceptance scenarios:
- Tank slots reserved (forward-compat for M5.8 + M5.9)
- Tank slots cannot be filled at M2.2A (placeholder discipline)
```

---

## Edit 1.4 — Tighten M5.8 (move battery / tank / race-env data to canonical owners)

### Problem

`specs/active/M5.8.md` (894 lines) contains:
- Per-origin reaction matrix (canonical here) ✓ keep
- 4 battery pack tiers + charging mechanics (M7.6 also defines this) ✗ move
- 5 gas tank tiers + tank physics (M5.9 should define this) ✗ move
- 10-race × per-environment resistance matrix (M5.10 EnvironmentSignal aggregator should own) ✗ move

The duplications create:
1. Drift risk (M5.8 tier table can diverge from M7.6 / M5.9 tier tables)
2. Implementer confusion (which spec is canonical?)
3. Spec sprawl (M5.8 is 894 lines; could be ~600 after tightening)

### Fix

Move data tables to canonical owners. M5.8 keeps **only** the per-origin contract (humans need O2; robots need power; etc.) and references the canonical specs for the data.

**Canonical owners after Edit 1.4:**

| Data table | Lives in | M5.8 says |
|---|---|---|
| 4 battery pack tiers | **M7.6** § Personal battery packs | "Defined canonically in M7.6 § Personal battery packs" |
| 5 gas tank tiers | **M5.9** § Gas tank inventory system | "Defined canonically in M5.9 § Gas tank inventory" |
| 10-race × env-factor resistance matrix (120 cells) | **M5.10** § Per-race environmental resistance matrix | "Defined canonically in M5.10 § Per-race environmental resistance matrix" |

M5.8 retains:
- 10 race definitions (canonical here — it's the origin spec)
- Per-origin reaction contract (canonical)
- Resource model (no-HP-bar; canonical)
- Origin force feedback events (canonical)
- Origin-specific death recap templates (canonical)
- Concussion vs internal_shock per origin (canonical)
- Helmet breach mechanics (canonical reference; M5.9 ticks the physics)

### Files to modify

| File | Action |
|---|---|
| `specs/active/M5.8.md` | **MODIFY** (cut battery tier table; cut tank tier table; cut race-env matrix; replace with cross-references) |
| `specs/active/M7.6.md` | **MODIFY** (add "Personal battery packs" section with 4-tier table) |
| `specs/active/M5.9.md` | **MODIFY** (add "Gas tank inventory system" section with 5-tier table) |
| `specs/active/M5.10.md` | **MODIFY** (add "Per-race environmental resistance matrix" section with 10×10 matrix) |

### Step 1: Extract content from M5.8

Read `specs/active/M5.8.md` and locate these three sections (search by header):

1. **"### 4 battery pack tiers (per M7.8 crafting ladder)"** — copy the entire table + surrounding paragraphs
2. **"### 5 launch gas tank tiers"** — copy the entire table + surrounding paragraphs (this section starts around the gas tank inventory system)
3. **"### Per-race environmental resistance matrix"** — copy the entire 120-cell matrix (temperature + pressure + radiation + gas exposure)

Save these to scratch (e.g., temp files) — you'll paste them into M7.6 / M5.9 / M5.10 next.

### Step 2: Modify `specs/active/M7.6.md`

Find the existing **"### Personal-power layer — per-actor power grid"** section. Immediately AFTER that section, add a new section:

```markdown
### Personal battery packs (per-actor; canonical owner)

The 4-tier personal battery pack ladder. Referenced by M5.8 (origin contract) + M5.10 (env signal) + M11.5 (PvE survival vehicles).

[PASTE THE 4-tier table extracted from M5.8 here, formatted exactly as it was]

[PASTE the surrounding paragraphs that describe charging modes, hot-swap UX, battery aging from M5.8 here]

**Cross-references:**
- M5.8 § Per-origin equipment power consumption examples — shows how each origin uses these batteries
- M2.2A § Inventory 8 active slots + 3 reserved tank slots — battery pack lives in an existing inventory slot
- M11.5 § Vehicle tank progression — vehicles use these battery tiers
```

### Step 3: Modify `specs/active/M5.9.md`

Find the existing **"### 10 launch gases + 6 liquid mixtures"** section. Immediately AFTER it, add:

```markdown
### Gas tank inventory system (canonical owner)

The 5-tier gas tank ladder. Referenced by M5.8 (origin contract; helmet breach) + M2.2A (inventory slot reservation) + M11.5 (vehicle tank requirements) + M7.5 (base atmospheric scrubbers + filling stations).

[PASTE the 5-tier gas tank tier table from M5.8 here]

[PASTE the surrounding paragraphs describing tank gas content, filter system, tank physics via PV=nRT, cryogenic Joule-Thomson cooling, overpressure rupture from M5.8 here]

**Cross-references:**
- M5.8 § Per-origin breathing requirements — defines which origins need which tanks
- M2.2A § Inventory 3 reserved tank slots — slot reservation (placeholder at M2.2A)
- M11.5 § Vehicles require gas tanks — vehicle tank requirements
- M7.5 § Atmospheric mixer station — base infrastructure to fill tanks
```

### Step 4: Modify `specs/active/M5.10.md`

Find the existing **"### EnvironmentSignal struct (locked v0.1)"** section. Immediately AFTER it, add:

```markdown
### Per-race environmental resistance matrix (canonical owner)

The 10-race × per-environmental-factor resistance matrix. Referenced by M5.8 (origin definitions), M6.6 (AI environmental competence), M11.5 (PvE survival difficulty), M7.6 (per-world power generation modifiers).

#### Temperature resistance

[PASTE the temperature resistance table from M5.8 here — 10 races × comfort range × cold/hot baselines × suit mitigation tiers]

#### Pressure resistance

[PASTE the pressure resistance table from M5.8 here — 10 races × native pressure × low/high tolerance × mitigation]

#### Radiation resistance

[PASTE the radiation resistance table from M5.8 here — 10 races × background tolerance × acute exposure × mitigation]

#### Gas exposure (per M5.9 10 launch gases)

[PASTE the gas exposure table from M5.8 here — 10 races × 10 gases full matrix]

**Cross-references:**
- M5.8 § 10 launch races / origins — race definitions + per-origin reaction
- M6.6 § AI doctrine per race — bot doctrine reads this matrix
- M11.5 § Stationeers-grade environmental difficulty — PvE difficulty rating per world × race
- M7.6 § Per-world generation modifiers — solar / wind per world
```

### Step 5: Modify `specs/active/M5.8.md` — replace moved sections with references

In `specs/active/M5.8.md`:

#### Replace the 4 battery pack tiers section

Find **"### 4 battery pack tiers (per M7.8 crafting ladder)"** and **delete the entire section** (table + surrounding paragraphs). Replace with:

```markdown
### Personal battery packs (defined canonically in M7.6)

The 4-tier personal battery pack ladder lives in `specs/active/M7.6.md` § Personal battery packs (canonical owner). M7.6 owns the data table; M5.8 declares per-origin needs:

- **Humans**: battery powers equipment only (helmet HUD, suit servo, energy weapons). Body unaffected by battery state.
- **Androids**: synthetic side draws power from battery; organic side independent. Battery empty = synthetic abilities offline.
- **Robots**: full body power-dependent. Battery empty = INERT state (recoverable via repair tool + new battery).
- **Powered organic**: per-cybernetic-module power needs.
- **Heavy biomech**: bio-energy primary; equipment-only external power needs.

See M7.6 for the 4-tier table (T1 Small Lithium-Ion / T2 Standard Lithium-Ion / T3 Heavy-Duty Reactor Battery / T4 Superconductor Capacitor Pack).
```

#### Replace the 5 gas tank tiers section

Find **"### 5 launch gas tank tiers"** and **delete the entire section** (table + surrounding paragraphs about tank physics, filter system, etc.). Replace with:

```markdown
### Gas tank inventory system (defined canonically in M5.9)

The 5-tier gas tank ladder lives in `specs/active/M5.9.md` § Gas tank inventory system (canonical owner). M5.9 owns the tier table + tank physics; M5.8 declares per-origin breathing requirements:

| Origin | Breathing requirement | Tank requirement |
|---|---|---|
| Human | O2 (25% O2 / 75% N2 nominal) | T1 Compressed (60 L) minimum for missions; T2 Cryogenic for vacuum operations |
| Android | O2 (organic side only) | Same as human (organic-side requirement); synthetic-side immune |
| Robot | NONE (no respiration) | Optional utility tank for chemical generators only |
| Powered organic | O2 + cybernetic O2 buffer | T1 minimum |
| Heavy biomech | O2 + bio_fluid recycle | T1 minimum |
| Insectoid | O2 OR CO2-rich (versatile) | T0 emergency OR T1 standard |
| Crystalline | Argon (Ar) preferred; vacuum-tolerant | Optional Ar tank for prolonged exposure |
| Photosynthetic | CO2 (exhales O2) | T1 CO2 tank for non-CO2 worlds |
| Aqueous | Dissolved O2 in water medium | Wet-suit with sealed water (special tank type) |
| Methane breather | Volatiles (CH4); O2 is POISON | T1 cryogenic methane tank |

See M5.9 for the 5-tier tank table (T0 Emergency / T1 Compressed / T2 Pressurized / T3 Cryogenic / T4 Closed-Loop Cycler) + tank physics (PV=nRT, overpressure rupture, cryogenic Joule-Thomson) + filter system (CO2 / Volatiles / Pollutant / Radiation / Composite).
```

#### Replace the Per-race environmental resistance matrix section

Find **"### Per-race environmental resistance matrix"** (or the section containing the 4 sub-tables: temperature / pressure / radiation / gas exposure). **Delete the entire section** (all 4 sub-tables). Replace with:

```markdown
### Per-race environmental resistance matrix (defined canonically in M5.10)

The 10-race × per-environmental-factor resistance matrix lives in `specs/active/M5.10.md` § Per-race environmental resistance matrix (canonical owner). M5.10's EnvironmentSignal aggregator computes per-actor exposure each tick; M5.8 just declares the 10 race definitions.

**Summary of race environmental dispositions (full matrix in M5.10):**

- **Human**: Earth-native; 18-25°C; 80-110 kPa; requires sealed suit + tank in vacuum
- **Android**: Earth-native; hybrid (organic side per human; synthetic side vacuum-tolerant)
- **Robot**: vacuum-immune; -50 to +60°C native; needs heat dissipation in vacuum
- **Powered organic**: Earth-native per human + cybernetic buffs
- **Heavy biomech**: 5-35°C; slow-clot blood; bio-regeneration
- **Insectoid**: cold-blooded; sluggish below 5°C; chitin armor; CO2-tolerant up to 50%
- **Crystalline**: silicon-based; -100 to +400°C wide range; **RADIATION IMMUNE**; vulnerable to acid + sonic + impact; native Argon atmosphere preferred
- **Photosynthetic**: 10-35°C; CO2 breather (exhales O2); needs sunlight to regen; immune to most poisons
- **Aqueous**: water-medium native (Europa subsurface ocean); immune to drowning + cold in water; FRAGILE in air
- **Methane breather**: cryogenic native (Mimas/Titan-class); **OXYGEN IS POISON**; requires volatiles atmosphere

See M5.10 for the full 120-cell matrix (10 races × per-factor — temperature / pressure / radiation / 10 gas types). M5.10's aggregator reads this matrix every tick per actor.
```

### Acceptance criteria for Edit 1.4

```bash
# Verify M5.8 no longer owns the moved tables
! grep -q "T1 — Small Lithium-Ion" specs/active/M5.8.md && echo "PASS: M5.8 doesn't define battery tier table" || echo "FAIL"
! grep -q "T0 — Emergency Bottle" specs/active/M5.8.md && echo "PASS: M5.8 doesn't define tank tier table" || echo "FAIL"

# Verify M7.6 now owns battery tier table
grep -q "Personal battery packs (per-actor; canonical owner)" specs/active/M7.6.md && echo "PASS: M7.6 owns battery tiers" || echo "FAIL"
grep -q "T1 — Small Lithium-Ion" specs/active/M7.6.md && echo "PASS: battery tier table moved to M7.6" || echo "FAIL"

# Verify M5.9 now owns tank tier table
grep -q "Gas tank inventory system (canonical owner)" specs/active/M5.9.md && echo "PASS: M5.9 owns tank tiers" || echo "FAIL"
grep -q "T0 — Emergency Bottle" specs/active/M5.9.md && echo "PASS: tank tier table moved to M5.9" || echo "FAIL"

# Verify M5.10 now owns race-env matrix
grep -q "Per-race environmental resistance matrix (canonical owner)" specs/active/M5.10.md && echo "PASS: M5.10 owns race-env matrix" || echo "FAIL"

# Verify M5.8 references the canonical owners
grep -q "defined canonically in M7.6" specs/active/M5.8.md && echo "PASS: M5.8 → M7.6 ref" || echo "FAIL"
grep -q "defined canonically in M5.9" specs/active/M5.8.md && echo "PASS: M5.8 → M5.9 ref" || echo "FAIL"
grep -q "defined canonically in M5.10" specs/active/M5.8.md && echo "PASS: M5.8 → M5.10 ref" || echo "FAIL"

# M5.8 should shrink ~150-250 lines
wc -l specs/active/M5.8.md
# Expected: somewhere in the 600-750 line range (was 894)
```

### Commit message for Edit 1.4

```
specs: Edit 1.4 — tighten M5.8 (move battery/tank/race-env tables to canonical owners)

M5.8 had 894 lines including 3 data tables that duplicated content
in their natural owners:

- 4 battery pack tiers → moved to M7.6 (power kernel)
- 5 gas tank tiers → moved to M5.9 (atmospherics kernel)
- 10-race × per-environment-factor resistance matrix → moved to M5.10
  (EnvironmentSignal aggregator)

M5.8 retains the per-origin contract (humans need O2, robots need power,
etc.) and references the canonical owners. This eliminates drift risk
between specs and clarifies ownership for implementers.

Acceptance criteria from COHERENCE-TIER-1.md § Edit 1.4 — all pass.
```

---

## Tier 1 — Full acceptance criteria (run before opening PR)

```bash
cd /Users/erol/projects/corefall

# Edit 1.1 checks
test -f specs/active/M7.6.5.md
! grep -q "^- M8.6 (mining" specs/active/M7.8.md
grep -q "M7.6.5" specs/active/M8.6.md
grep -q "active%20specs-36" README.md
grep -q "M7.6.5 — Basic Mining" README.md
test "$(ls specs/active/M*.md | wc -l | tr -d ' ')" = "36"

# Edit 1.2 checks
[ "$(grep -l 'SmelterFurnace.*fabrication station' specs/active/*.md | wc -l | tr -d ' ')" = "1" ]
grep -q "owned by M7.8" specs/active/M8.6.md
grep -q "M8.6.*does NOT define" specs/active/M7.8.md

# Edit 1.3 checks
grep -q "8 active slots" specs/active/M2.2A.md
grep -q "3 reserved tank slots" specs/active/M2.2A.md
grep -q "tank_primary" specs/active/M2.2A.md
grep -q "Scenario: Tank slots reserved" specs/active/M2.2A.md

# Edit 1.4 checks
! grep -q "T1 — Small Lithium-Ion" specs/active/M5.8.md
! grep -q "T0 — Emergency Bottle" specs/active/M5.8.md
grep -q "Personal battery packs (per-actor; canonical owner)" specs/active/M7.6.md
grep -q "Gas tank inventory system (canonical owner)" specs/active/M5.9.md
grep -q "Per-race environmental resistance matrix (canonical owner)" specs/active/M5.10.md
grep -q "defined canonically in M7.6" specs/active/M5.8.md
grep -q "defined canonically in M5.9" specs/active/M5.8.md
grep -q "defined canonically in M5.10" specs/active/M5.8.md

# Workspace still builds
cd game && cargo build && cargo clippy --all-targets -- -D warnings
cd ..

echo "TIER 1 — ALL CHECKS PASS"
```

All checks must complete without errors.

### Tier 1 PR template

**Title:** `specs: tier-1 coherence fixes (dependency inversion + data ownership + tank slots)`

**Body:**

```markdown
## Summary

Tier 1 of the spec coherence pass per `specs/COHERENCE-PLAN.md`. Fixes 4 hard issues blocking M2.2A implementation:

1. **Edit 1.1** — Split M8.6 into M7.6.5 (basic mining + smelting; BP7) + M8.6 (advanced; BP8) to fix the M7.8 ↔ M8.6 dependency inversion
2. **Edit 1.2** — Unify SmelterFurnace + EnrichmentReactor in M7.8 (canonical owner); cross-reference notes added to M8.6
3. **Edit 1.3** — Add 3 reserved tank slots to M2.2A inventory (placeholder; M5.8 + M5.9 fill at their milestones)
4. **Edit 1.4** — Tighten M5.8 by moving battery (4-tier) / tank (5-tier) / race-env (120-cell) tables to canonical owners (M7.6 / M5.9 / M5.10)

## Active spec count

- Before: 35
- After: 36 (added M7.6.5)
- README badge updated

## Verification

Ran all acceptance checks from `COHERENCE-TIER-1.md` § Tier 1 — Full acceptance criteria. All PASS.

`cargo build` + `cargo clippy --all-targets -- -D warnings` both green.

## Next

After this PR merges, Tier 2 (milestone splits) can start. See `specs/COHERENCE-TIER-2.md`.
```

---

## Done with Tier 1

Once the PR merges:
- ✅ M7.8 ↔ M8.6 dependency inversion fixed
- ✅ SmelterFurnace + EnrichmentReactor have one home (M7.8)
- ✅ M2.2A inventory has 3 reserved tank slots
- ✅ M5.8 tightened (~150-250 lines lighter); battery / tank / race-env tables live in canonical owners
- ✅ 36 active specs; README updated

**Proceed to `COHERENCE-TIER-2.md`** for milestone splits.
