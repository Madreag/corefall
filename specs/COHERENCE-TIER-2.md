# Coherence Tier 2 — Milestone Splits

**Status:** `active` — should complete BEFORE BP7 implementation starts
**Prerequisite:** Tier 1 PR must be merged first
**Estimated effort:** AI-scale 60-90 minutes (single PR, 4 commits)
**Output:** 1 PR titled `specs: tier-2 coherence (M7 split + M11.5 split + boss schema + hunger afflictions)`

---

## Goals

Split 2 mega-milestones into focused sub-milestones, centralize boss schema, and reroute survival afflictions through M5.7:

1. **Edit 2.1** — Split M7 into M7 + M7.1 + M7.2 (campaign + factions/NPC + loot/RPG)
2. **Edit 2.2** — Split M11.5 into M11.5 + M11.6 + M11.7 (survival + transport + endgame)
3. **Edit 2.3** — Centralize boss schema in M7
4. **Edit 2.4** — Add hunger/thirst/sleep_dep/sanity_low as M5.7 affliction kinds (18 → 22)

After Tier 2 PR merges:
- M7 family covers campaign + base + commander; M7.1 covers factions + NPC; M7.2 covers loot + RPG
- M11.5 covers PvE Survival mode + procgen; M11.6 covers transport + stations + asteroid mining; M11.7 covers bosses + world events
- Boss schema (HP + N phases + abilities + arena + rewards) defined once in M7; M11.7 authors data rows
- M5.7 has 22 afflictions (added the 4 survival kinds); M11.5 references M5.7 instead of defining mechanics
- 36 → 40 active specs

---

## Edit 2.1 — Split M7 into M7 + M7.1 + M7.2

### Problem

`specs/active/M7.md` (354 lines, but conceptually massive) bundles **25+ subsystems**:

- Campaign progression
- 5 storytellers (Cassandra / Phoebe / Randy / Ironman / Sandbox)
- Base building + command core + module slots + avatar mode
- Buy menu + delivery craft + 8 stratagems
- AI commander persona with persistent rivalry
- 8 factions + quartermaster vendors + diplomacy + faction wars
- NPC dialog system + branching narratives + side missions
- Investigation + crime scene + hidden lore + codex
- Hostage / captives mechanics
- Mini-boss + boss patterns (multi-phase)
- Pet / companion animals (4 launch types)
- Perk + curse system (20+ perks, 10+ curses; stackable; Noita-style)
- Loot rarity tiers (Common / Magic / Rare / Legendary / Unique)
- 30+ item affixes; set bonuses at 2 / 4 / 6
- XP + level + perk-point system
- 30+ launch achievements
- Inventory grid Tetris (Stationeers)
- Manufacturing + cooking + plant growing
- Treasure maps + voyages (Sea of Thieves)

Implementing M7 = touching `cf-campaign`, `cf-storyteller`, `cf-base`, `cf-commander`, `cf-stratagem`, `cf-economy`, `cf-faction`, `cf-dialog`, `cf-investigation`, `cf-pet`, `cf-progression`, `cf-loot`, `cf-craft`, `cf-treasure`, `cf-replay`, `cf-content` = **16 new crates**. Massive scope.

### Fix

Split M7 into 3 focused milestones, all in BP7:

| Milestone | Owns | Crates |
|---|---|---|
| **M7 — Campaign + Base Spine** | Mission progression / 5 storytellers / base building / command core / module slots / avatar mode / buy menu / delivery craft / 8 stratagems / mini-boss + boss schema | cf-campaign, cf-storyteller, cf-base, cf-commander, cf-stratagem, cf-economy |
| **M7.1 — Factions + NPCs + Narrative** | 8 factions + quartermaster + diplomacy / NPC dialog + branching / quest generator / investigation / hostage / pet companions (4 types) | cf-faction, cf-dialog, cf-investigation, cf-pet |
| **M7.2 — Loot + Progression + RPG** | Loot rarity / 30+ affixes / set bonuses / XP+level / perks/curses (20+10+) / 30+ achievements / inventory grid Tetris / treasure maps + voyages | cf-loot, cf-progression, cf-treasure |

**Cooking / plants / manufacturing** stays in M7.8 (already its home per Tier 1's recipe ladder; M7's references should be removed).

### Files to modify

| File | Action |
|---|---|
| `specs/active/M7.md` | **MODIFY** (strip to spine subsystems only) |
| `specs/active/M7.1.md` | **CREATE** (Factions + NPCs + Narrative) |
| `specs/active/M7.2.md` | **CREATE** (Loot + Progression + RPG) |
| `README.md` | **MODIFY** (add M7.1 + M7.2 to BP7 table; update active spec count) |

### Step 1: Create `specs/active/M7.1.md`

```markdown
# M7.1 — Factions + NPCs + Narrative

## Status

`active`

## Intent

**M7.1 is the factions + NPC + narrative milestone** — the social + dialog + investigation layer that turns M7's campaign skeleton into a populated, living world. After M7.1, players encounter 8 distinct factions with diplomatic relationships, can talk to NPCs through branching dialog trees, accept side missions, investigate mysteries, rescue hostages, and adopt pet companions.

M7.1 splits this out of M7 because the social/narrative subsystem is its own coherent scope (16+ subsystems in original M7 would overwhelm any reviewer). M7 keeps the campaign spine; M7.1 fills the world with personality.

M7.1 promise: **"the world has 8 factions you can play diplomatic chess with, NPCs you can talk to with real branching choices, and side stories that aren't just kill quests."**

## Player-facing behavior

### 8-faction system (full mechanics)

Per DR-027 + M7 campaign baseline:

| Faction | Tagline | Default disposition | Key trait |
|---|---|---|---|
| **Hostile Corp** | Corporate military; player's primary enemy by default | -75 (hostile) | Industrial weapons; bunker-style bases |
| **Allied Resistance** | Underground rebels | +50 (allied) | Stealth + sabotage tactics; sympathetic NPCs |
| **Marauder Tribes** | Wasteland scavengers | -25 (suspicious) | Improvised weapons; territorial |
| **Religious Order** | Doctrinaire faith-based | 0 (neutral) | Ceremonial gear; ritual missions |
| **Scientist Order** | Knowledge-seekers | +20 (friendly) | Research drops + tech tree unlocks |
| **Mercenary Guild** | Hired guns | 0 (neutral; can be hired) | Highest-tier gear available for credits |
| **Pirates** | Sea/orbital raiders | -50 (hostile) | Naval + cargo-raiding focus |
| **Drone Collective** | AI-only faction | varies (per scenario) | Distributed swarm tactics |

**Per-faction relationship matrix** (-100 to +100):
- Updated by player actions (kill allied member → -30; rescue captive → +20; complete quartermaster contract → +15; betray faction → -50)
- Dynamic shifts emit `faction.relationship_changed { from, to, cause }`
- Threshold cascades: <-50 declares war; >+50 unlocks ally services; >+90 declares formal alliance

**Quartermaster vendors** (per faction):
- NPC at faction base sells faction-specific gear
- Inventory tiers gated by relationship score
- Faction-specific cosmetic differentiation (uniforms, weapon skins, base flags)
- Prices scale with relationship (allied = -25%; hostile = no sale)

**Faction territory map:**
- Per-region territorial control (M11.5 PvE Survival inherits this)
- Player can capture / liberate / cede territory
- Faction wars trigger mass attack events

**Diplomacy actions** (`act.player.faction.*`):
- `truce` — pause hostilities for N hours
- `surrender` — accept defeat; lose territory
- `alliance` — formal alliance; shared bases
- `attack` — declare war
- `negotiate` — open dialog tree for custom terms

### NPC dialog system + branching narratives

- Friendly NPCs in scenarios with dialog branches
- **4 default dialog options per NPC**: `ask_mission` / `trade` / `recruit` / `leave`
- Per-NPC dialog tree authored in `content/dialog/<npc_id>.dialog.ron`
- Dialog branches drive faction relationship + reveal lore + unlock side missions
- NPCs offer side missions (kill X bandits, deliver Y, escort Z, fetch artifact)
- Mission rewards: credits + gear + perks + faction reputation

**Per-NPC dialog state:**
- Greeting (first meeting per session)
- Familiarity track (per-NPC count of interactions)
- Mood (per-NPC + driven by relationship to player faction)
- Memory of past player actions (M11+ extends across sessions)

**Forward-compat for M9.5 voice dialog** — text-only at M7.1; voice clips added at M9.5.

### Procedural quest generator (per Sea of Thieves voyage pattern)

8 launch quest templates:
- **Scavenge** — Travel to specific zone; collect specific materials; return
- **Defend** — Protect NPC trader / base / cargo from raiders
- **Eliminate** — Kill specific faction commander or mini-boss
- **Rescue** — Extract captive from enemy zone (tutorial-safety auto-on)
- **Investigate** — Examine clues at multiple locations
- **Voyage** — Sequential 5-10 clue chain leading to rare loot
- **Trade** — Bring goods from world A to world B
- **Boss hunt** — Defeat specific endgame boss (cooperative)

Each quest scales difficulty per player tier; rewards in credits + loot + research points + faction reputation.

Per-quest replay events: `quest.accepted`, `quest.objective_started`, `quest.objective_completed`, `quest.completed`, `quest.abandoned`, `quest.failed`.

### Investigation + hidden lore + codex

- **Crime scene** scenarios: examine clues to solve mystery (5-10 clues per mystery)
- **Multi-clue chains** with branching reveals
- **Hidden lore items**: `lore_log_X` scattered in scenarios
- **Codex unlocks**: per-clue / per-NPC / per-faction; ~600 codex entries cumulative
- Investigation events: `investigation.clue_found`, `investigation.mystery_solved`, `investigation.lore_unlocked`

### Hostage / captives mechanics

- Mission objective: rescue captive OR capture target
- Captives are NPCs tied to scenery
- Player frees → captive follows player as weak NPC
- Captive can be killed by stray fire (mission fails)
- Captured enemy NPCs handcuffed + follow

Per Tutorial-safety policy (DR-018 + DR-023): hostage missions auto-enable tutorial_safety to prevent first-session frustration.

### Pet / companion animals (4 launch types)

Per Rimworld pattern:

| Pet | Role | Combat | Special |
|---|---|---|---|
| `dog` | Alert + attack | Medium | Detects hidden enemies; barks alarm |
| `cat` | Small + stealthy | Low | Distracts enemies; ignores hazards (jumps over) |
| `wolf` | Heavy combat | High | Pack tactics (multiplies if 2+) |
| `war_bear` | Tank melee | Very high | Knockback on hit; immune to electrified |

- Each pet has own HP + traits
- Pet follows player; can defend; can fetch items
- Pet training (M7+): feed + walk + commands
- Pet death = permanent in campaign mode
- Per-pet personality (traits per M5.8 origin → pet variant)

## Content roster at M7.1

| Content | Roster |
|---|---|
| **Factions** (toward 8) | All 8 factions cumulative |
| **NPCs** (toward 24+ named) | 16 named NPCs cumulative (8 quartermasters + 8 storyline NPCs) |
| **Missions** (toward 30+) | 18 missions cumulative (M7 had 10 main story; M7.1 adds 8 side missions across factions) |
| **Codex** (toward 600 entries) | 350 cumulative |
| **Pets** | 4 launch pet types |

## Crates / modules touched

| Crate | Status | What |
|---|---|---|
| `cf-faction` | NEW (deep) | 8 factions + relationship matrix + diplomacy + wars + quartermasters |
| `cf-dialog` | NEW | NPC dialog branches + quest generator + side missions |
| `cf-investigation` | NEW | Crime scene + clues + lore codex |
| `cf-pet` | NEW | Pet companions + training |
| `cf-replay` | MODIFY | faction.*, dialog.*, quest.*, investigation.*, pet.* event families |

## Acceptance criteria

```gherkin
Scenario: 8 factions + relationship matrix
  Given scenario with all 8 factions
  When player kills allied faction member:
    Then faction.relationship_changed fires with delta=-30
  When relationship < -50:
    Then faction.war_declared fires

Scenario: NPC dialog with side mission
  Given friendly NPC approached
  When player chooses "ask_mission":
    Then dialog.mission_accepted fires
    And new mission added to quest log
  When player chooses "trade":
    Then dialog.trade_opened; quartermaster inventory visible
  When player chooses "recruit":
    Then dialog.recruit_offered; NPC joins player squad
  When player chooses "leave":
    Then dialog closes; NPC remembers interaction

Scenario: Procedural quest — Scavenge
  Given quest generator + Scavenge template
  When quest generated:
    Then quest.accepted fires
    And specific target zone + material requirement displayed
  When player gathers requirement:
    Then quest.objective_completed fires
  When player returns to questgiver:
    Then quest.completed fires + reward distributed

Scenario: Crime scene investigation
  Given crime scene scenario
  When player examines 5 clues:
    Then investigation.clue_found fires each time
    And investigation.mystery_solved fires after final clue
  And lore codex entries unlocked

Scenario: Faction diplomacy
  Given Faction A relationship = +60
  When player invokes act.player.faction.alliance:
    Then faction.alliance_declared fires
    And quartermaster offers premium gear
    And Faction A bases now safe to enter

Scenario: Pet companion fetches item
  Given dog companion + item dropped
  When dog detects item within range:
    Then pet.fetches fires
    And dog brings item to player

Scenario: Hostage rescue
  Given captive NPC + tutorial_safety=true
  When player reaches captive + interacts:
    Then quest.objective_completed fires for "rescue"
    And captive follows player
  When stray fire hits captive:
    Then captive HP capped at DYING (tutorial_safety)
    And mission does NOT fail
```

## Dependencies

- **M7 (campaign spine; must close)**: provides mission director + scenarios + storytellers
- M5 + M5.5 + M5.6 + M5.7 + M5.8 + M5.9 + M5.10 (all close)
- M6 + M6.5 + M6.6 (AI must close — NPCs are AI actors)

## Closure procedure

Reference bundle + 30+ sweep rows + DR-027 closure (factions surface) + DR-031 closure (economy via quartermasters). PASS.

## Cross-DR

DR-002, DR-006, DR-008, DR-022, DR-023, DR-024, **DR-027 (partial closure — factions)**, **DR-031 (economy at launch)**, **DR-048 (retention loop)**.
```

### Step 2: Create `specs/active/M7.2.md`

```markdown
# M7.2 — Loot + Progression + RPG

## Status

`active`

## Intent

**M7.2 is the loot + progression + RPG layer** — the retention loop that makes 100-hour campaigns rewarding. After M7.2, every kill drops rated loot, every level grants perks, every mission completes achievements, inventory becomes a Tetris puzzle, and treasure maps lead to voyages.

M7.2 splits this out of M7 because retention mechanics are their own coherent scope distinct from campaign + factions. The systems here drive M7.1's quest rewards + M11.5's PvE survival progression + M11+ veteran identity.

M7.2 promise: **"every drop matters; every level changes the build; every hour invested unlocks new gameplay."**

## Player-facing behavior

### Loot rarity + affixes + set bonuses (Diablo-inspired)

**5 rarity tiers:**

| Tier | Drop chance | Color | Affix count |
|---|---|---|---|
| `Common` | 70% | Gray | 0 |
| `Magic` | 25% | Blue | 1-2 |
| `Rare` | 4% | Yellow | 3-4 |
| `Legendary` | 1% | Orange | 5-6 + 1 unique trait |
| `Unique` | 0.1% | Gold | 7+ + named lore + signature ability |

**30+ launch affixes** per item type (weapons / armor / chassis modules):

- **Weapon affixes** (per-shot modifiers; stacking is Noita-style per M5+ chassis modifier slots):
  - `+15% damage` / `+10% fire rate` / `+5% crit chance` / `+20% reload speed`
  - `+ chain lightning on hit` / `+ ignite on crit` / `+ life steal 5%`
  - `+ ricochet 1 enemy` / `+ explosive rounds` / `+ piercing`
  - `+ longer range` / `+ tighter spread` / `+ silenced` / `+ tracers visible to allies`

- **Armor affixes**:
  - `+ damage resist (kinetic / thermal / electric / radiation)`
  - `+ stamina regen` / `+ run speed` / `+ stealth bonus`
  - `+ HP / max HP` / `+ shield regen`

- **Chassis module affixes** (M5+ chassis-only):
  - `+ module HP` / `+ module overclock tolerance`
  - `+ jet fuel efficiency` / `+ sensor range`

**Set bonuses** at 2 / 4 / 6 set items:
- Wearing 2 items of same set → small bonus (e.g. +5% damage)
- 4 items → significant bonus
- 6 items → set-defining bonus (e.g. summons drone ally)

**Loot drops:**
- Enemies on death: rarity rolled per kill_value × storyteller_drop_multiplier
- Treasure chests: per-scenario placement; M7+ campaign + M11.5 PvE
- Boss kills: guaranteed rare+ drop; legendary chance per boss
- Quartermaster purchases (M7.1): seeded rarity per faction reputation

Events: `loot.dropped { item_id, rarity, affixes }`, `loot.picked_up`, `loot.identified` (rare+ requires identification at base).

### XP + level + perk-point system

**XP sources:**
- Kills (xp_value per enemy_tier × multiplier)
- Objectives completed (xp_value per objective)
- Missions completed (bonus xp + perks unlocked)
- Side quests (lower xp; more rewards from M7.1)
- Hidden codex discoveries (small xp + lore)

**Level threshold:** `xp_required = base * 1.5^level` (exponential)

**Per-level reward:** +1 perk point. Perks chosen from research tree (see Perks below).

**Achievements** (30+ launch):
- `first_kill` / `first_breach` / `first_mission_complete`
- `grenade_master` (10 grenade kills)
- `silent_predator` (mission completed undetected)
- `boss_slayer` (defeat any boss)
- `master_crafter` (craft 50 unique recipes)
- `100_kills` / `1000_kills` (escalating)
- HUD toast notification on unlock
- Per-achievement codex entry + Steam achievement (M9+ integration)

### Perk + curse system (Noita-style stackable)

**20+ launch perks** (selected at perk altar or per-level-up):

- `vampire` — life steal 5% per hit
- `berserker` — +20% damage at low HP
- `explosive_immunity` — immune to grenade self-damage
- `fast_reload` — -25% reload time
- `sharp_aim_faster` — sharp aim builds in 50% of normal time
- `sprint_speed+15%` — passive movement bonus
- `crit_chance+10%` — base crit boost
- `medkit_efficient` — medkit restores +25% HP
- `lucky_drops` — +20% loot rarity roll
- `psychic_immunity` — immune to STALKER psy_storm anomaly (per M5.7)
- `radiation_resist` — radiation tick × 0.5
- `night_vision` — natural low-light vision
- `quiet_steps` — reduce footstep loudness 50% (per M2.2A perception)
- `hot_swap_pro` — +50% faster weapon swap (per M2.2A)
- `iron_lungs` — oxygen drain × 0.5 (per M5.8 + M5.9)
- `chassis_tinkerer` — chassis repair +50% efficient (per M5)
- `negotiator` — faction reputation gains × 1.5 (per M7.1)
- `treasure_hunter` — treasure map clues reveal at -20% step count
- `power_efficient` — equipment power draw × 0.85 (per M5.8 + M7.6)
- `solar_savant` — solar panel output × 1.2 (per M7.6 robot bonus)

**10+ launch curses** (offered alongside perks at curse altars; tradeoff bonuses):

- `slow_reload` — +50% reload time (counterbalanced by +damage perk)
- `no_jet` — jetpack disabled (counterbalanced by +sprint)
- `vulnerable_to_fire` — burning affliction × 2 damage
- `low_stamina` — stamina max -25%
- `fragile` — HP max -15%
- `slow_swap` — weapon swap × 1.5 (paired with `hot_swap_pro` cancels)
- `loud_steps` — footsteps +50% loudness (paired with `quiet_steps` cancels)
- `unlucky` — loot rarity × 0.5
- `clumsy_aim` — bloom × 1.2
- `power_hungry` — equipment power draw × 1.5

**Perk altars in scenarios:**
- Per scenario placement; offer 3 perks (curated by storyteller)
- Player picks 1 perk OR 1 curse for bigger bonus
- Persist across campaign (M7-only feature; M11.5 inherits)

### Inventory grid Tetris (Stationeers-style)

- Inventory displayed as 4x4 grid (configurable per chassis weight class)
- Items occupy grid cells per shape:
  - `pistol` 1x1
  - `rifle` 2x1
  - `sniper` 3x1
  - `grenade` 1x1
  - `medical` 2x2
  - `tool_kit` 2x2
  - `large_battery` 2x2 (per M7.6 personal battery pack tiers)
  - `gas_tank_t1` 1x2 (per M5.9 tank tier)
  - `gas_tank_t3` 2x2 (cryogenic tank larger)
- Heavy items can't fit in small backpack
- Per chassis (M5):
  - Light chassis: 4x4 grid
  - Powered armor: 5x5 grid
  - Light mech: 6x5 grid
  - Heavy mech: 8x6 grid
  - Drone: 2x2 grid

Events: `inventory.item_placed { slot_position }`, `inventory.item_rejected { reason: no_space }`, `inventory.layout_changed`.

### Treasure maps + voyages (Sea of Thieves)

- Treasure maps drop from rare enemies (M11+ extension at low rates from common enemies)
- Map shows region with X marking treasure + 3-5 rotational clues
- Player travels to X → digs → finds treasure (rare item + gold + lore unlock)

**Voyage system** (multi-step quests):
- 5-10 steps per voyage
- Each step is a clue → location → action → next clue
- Voyages can span multiple worlds (M11.5 PvE survival inherits)
- Voyage rewards scale with step count (5-step = rare; 10-step = legendary)

Events: `treasure.map_acquired`, `treasure.clue_revealed`, `treasure.discovered`, `voyage.started`, `voyage.step_completed`, `voyage.completed`.

## Content roster at M7.2

| Content | Roster |
|---|---|
| **Affixes** | 30+ launch affixes (10 weapon + 10 armor + 10 chassis-module) |
| **Set bonuses** | 5 launch sets (each 6-item; ramped bonuses at 2/4/6) |
| **Perks** | 20+ launch perks |
| **Curses** | 10+ launch curses |
| **Achievements** | 30+ launch achievements |
| **Inventory shapes** | 15+ item shape definitions |
| **Treasure maps** | 5+ launch map types |
| **Voyages** | 8+ launch voyage chains |

## Crates / modules touched

| Crate | Status | What |
|---|---|---|
| `cf-loot` | NEW (deep) | Rarity + affixes + set bonuses + identification system |
| `cf-progression` | NEW (deep) | XP + level + perks + curses + achievements |
| `cf-treasure` | NEW | Treasure maps + voyages |
| `cf-inventory::grid` | NEW | Grid Tetris layout + shape system + per-chassis sizing |
| `cf-replay` | MODIFY | loot.*, progression.*, treasure.*, voyage.*, inventory.* event families |

## Acceptance criteria

```gherkin
Scenario: Loot rarity drop
  Given enemy killed
  When loot.dropped fires:
    Then rarity rolled (common 70% / magic 25% / rare 4% / legendary 1% / unique 0.1%)
    And item has rarity-specific border + glow visual

Scenario: Rare item requires identification
  Given a rare+ item dropped (yellow / orange / gold)
  When player picks up:
    Then loot.dropped fires + item shown as "Unidentified Rare"
  When player visits identification station at base:
    Then loot.identified fires + affixes revealed

Scenario: XP + level + perk point
  Given player kills with xp_value=50
  Then progression.xp_gained fires
  And player_xp += 50
  When level threshold crossed:
    Then progression.level_up fires (+1 perk point)
    And HUD toast notification

Scenario: Achievement unlocked
  Given player kills 10 enemies with grenades
  Then achievement.unlocked fires id="grenade_master"
  And HUD toast notification

Scenario: Perk altar offer
  Given perk altar in scenario
  When player approaches:
    Then progression.perk_offered fires (3 perks shown)
  When player picks:
    Then progression.perk_acquired fires (perk modifier applies)

Scenario: Curse altar tradeoff
  Given curse altar offering "slow_reload" curse + "double_damage" perk
  When player accepts both:
    Then progression.curse_acquired + progression.perk_acquired fire
    And reload × 1.5 + damage × 2 applies

Scenario: Inventory grid Tetris
  Given 4x4 backpack with 12 cells used
  When player adds 3x1 sniper (3 cells needed):
    Then inventory.item_placed fires (fits remaining 4 cells)
  When player adds 2x2 medical (4 cells needed):
    Then inventory.item_rejected fires (not enough contiguous space)

Scenario: Treasure map + voyage
  Given treasure map dropped
  When player views map:
    Then map UI shows X + 5 rotational clues
  When player digs at X:
    Then treasure.discovered fires + rare loot drops

Scenario: Set bonus 2 / 4 / 6 thresholds
  Given player wearing 2 items of "Stalker Set":
    Then loot.set_bonus_applied fires (small bonus: +5% stealth)
  When wearing 4 items:
    Then bonus upgrades to medium (+15% stealth + sound dampening)
  When wearing 6 items:
    Then bonus becomes set-defining (auto-stealth in shadows)
```

## Dependencies

- **M7 (campaign spine; must close)** — provides mission rewards
- **M7.1 (factions + NPCs; must close)** — quartermasters sell rated loot
- M5 + M5.5 + M5.6 (chassis + collision + materials must close)

## Closure procedure

Reference bundle + 30+ sweep rows + DR-031 closure (anti-pay-to-win audit). PASS.

## Cross-DR

DR-006, DR-008, DR-022, DR-024, **DR-031 (anti-pay-to-win)**, **DR-048 (retention loop)**.
```

### Step 3: Modify `specs/active/M7.md` (trim to spine)

In `specs/active/M7.md`, remove the following subsections (search for these headers and delete the section):

**Sections to REMOVE from M7:**
- `### Faction system full mechanics` — moved to M7.1
- `### NPC dialog system + branching narratives` — moved to M7.1
- `### Investigation + hidden lore + collectibles` — moved to M7.1
- `### Hostage / captives mechanics` — moved to M7.1
- `### Pet / companion animals (Rimworld)` — moved to M7.1
- `### Perk + curse system stackable (Noita; M2.2 deferred)` — moved to M7.2
- `### Loot rarity + affixes + set bonuses (Diablo)` — moved to M7.2
- `### XP + level + achievements` — moved to M7.2
- `### Inventory grid Tetris (Stationeers; M2.2 deferred)` — moved to M7.2
- `### Manufacturing + cooking + plant growing (Stationeers; M2.2 deferred)` — manufacturing moved to M7.8; cooking + plants ALSO move to M7.8 (single home for crafting)
- `### Treasure maps + voyages (Sea of Thieves)` — moved to M7.2

**Sections M7 KEEPS:**
- `### Campaign architecture` (canonical here)
- `### Storyteller / incident director (Rimworld-inspired)` (canonical here)
- `### Base + command core identity` (canonical here)
- `### Buy menu + delivery craft` (canonical here)
- `### Stratagem call-ins` (canonical here)
- `### AI commander persona` (canonical here)
- `### Mini-boss + boss patterns` — keep but **enhance** in Edit 2.3 (centralize boss schema)

After the cuts, the **Intent** section needs updating:

**BEFORE:**
```markdown
## Intent

**M7 is the campaign + base + commander mega-milestone** — the gameplay vertical-slice anchor for BP7. After M7, players can run a campaign with multi-mission progression, build/defend/uproot bases with command core identity (DR-029), face an AI commander with persistent rivalry, use stratagem call-ins, encounter loot rarity drops, gain XP + perks, complete branching narratives with NPC dialog + investigation events, fight named mini-boss + boss encounters, manage pet companions, and consume the full faction system with quartermaster vendors.
```

**AFTER:**
```markdown
## Intent

**M7 is the campaign + base + commander spine** — the gameplay vertical-slice anchor for BP7. M7 ships:

- Multi-mission campaign progression
- 5 storyteller archetypes (Cassandra Classic / Phoebe Chillax / Randy Random / Ironman / Sandbox)
- Base building with command core identity (DR-029) + module slots + avatar mode
- Mid-mission buy menu + delivery craft (dropship)
- 8 stratagem call-ins (Helldivers-style)
- AI commander persona with persistent rivalry
- Mini-boss + boss patterns (multi-phase) + canonical **boss schema** (data model for all bosses across campaign + M11.7 PvE + M12 MMO)

**Sister milestones in BP7:**
- M7.1 — Factions + NPCs + Narrative (8 factions / dialog / quests / investigation / hostage / pets)
- M7.2 — Loot + Progression + RPG (loot rarity / affixes / XP+level / perks/curses / inventory grid / treasures)
- M7.5 — Base Atmospherics
- M7.6 — Power & Electrical Engineering Kernel
- M7.6.5 — Basic Mining + Smelting
- M7.7 — Day/Night + Weather + Dynamic Events
- M7.8 — Crafting Tiers & Fabrication Chain

M7 is the spine; the others fill specific scopes.

M7 promise: **"the campaign feels alive — base feels meaningful, AI commanders feel like rivals, every mission has a storyteller pulse."**
```

Update **Content roster at M7** to remove subsystems that moved:

**BEFORE (current roster):**
```markdown
### Content roster at M7 (the big content milestone)
... (lots of content including weapons, actors, vehicles, base objects, factions, missions, music, SFX, narrative, codex, achievements)
```

**AFTER:**
```markdown
### Content roster at M7 (campaign spine)

| Content | Roster |
|---|---|
| **Weapons** (toward 70+) | 30 weapons cumulative — campaign-specific weapons; M2.2A + M5 baseline + M7 adds 24 weapons; M7.2 doesn't add weapons but adds affixes |
| **Actors** (toward 44+) | 20 actors cumulative — campaign NPCs (named character actors are M7.1) |
| **Vehicles** (toward 18+) | 8 vehicles (jeep / ATV / motorcycle / hovercraft / boat / armored_truck / mech_walker / drone_ship) |
| **Base objects** (toward 60+) | 40 base objects |
| **Stratagems** | 8 launch stratagems |
| **Storytellers** | 5 launch storytellers |
| **Boss types** | Boss schema + 2 launch mini-bosses (Spotter from M2.2B + named commander mini-boss) |
| **Missions** (toward 30+) | 10 main story missions (M7.1 adds 8 side missions to reach 18) |
| **Music** (toward 30+ tracks) | 18 music tracks cumulative (12 launch ambient + 5 mission + 1 storyteller theme) |
| **Codex** (toward 600) | 300 codex entries (M7.1 extends to 350) |

Note: factions / NPCs / dialog content rosters in M7.1. Loot / progression / achievements rosters in M7.2.
```

Update **Dependencies** to add M7.1 + M7.2 forward-compat:

```markdown
## Dependencies

- M5 + M5.5 + M5.6 + M5.7 + M5.8 + M5.9 + M5.10 + M6 + M6.5 + M6.6 + M4B (must close)
- M5.5.5 + M5.9.5 (interlude must close)

**Sister milestones (concurrent in BP7 — no hard dep on each other):**

- M7.1 (Factions + NPCs + Narrative) — fills the social layer
- M7.2 (Loot + Progression + RPG) — fills the retention loop
- M7.5 (Base Atmospherics), M7.6 (Power Kernel), M7.6.5 (Basic Mining), M7.7 (Weather + Worlds), M7.8 (Crafting)

**BP7 closes only when M7 + M7.1 + M7.2 + M7.5 + M7.6 + M7.6.5 + M7.7 + M7.8 all close.**
```

### Step 4: Modify `README.md`

Find the active spec count badge:

**BEFORE:**
```markdown
[![Specs](https://img.shields.io/badge/active%20specs-36%20%28M2.2A..M12%29-blueviolet?style=flat-square)](specs/active/)
```

**AFTER:**
```markdown
[![Specs](https://img.shields.io/badge/active%20specs-38%20%28M2.2A..M12%29-blueviolet?style=flat-square)](specs/active/)
```

(38 because Edit 2.1 alone adds 2 specs; Edit 2.2 will bring it to 40.)

Find the BP7 row for M7 and update to reflect the split. Add new rows for M7.1 + M7.2:

```markdown
| BP7 | **M7 — Campaign + Base + Commander Spine** | Planned | Campaign + base building (DR-027/029) + 5 storytellers (Cassandra/Phoebe/Randy/Ironman/Sandbox) + buy menu + delivery craft + 8 stratagems + AI commander with persistent rivalry + boss schema (canonical data model for all bosses). Sister milestones M7.1 + M7.2 fill factions/NPCs and loot/progression. |
| BP7 | **M7.1 — Factions + NPCs + Narrative** | Planned | 8 factions full mechanics (Hostile Corp / Allied Resistance / Marauder / Religious / Scientist / Mercenary / Pirates / Drone Collective) + relationship matrix + quartermasters + diplomacy + faction wars + NPC dialog with branching trees + procedural quest generator (8 templates) + investigation (crime scenes + clues + codex) + hostage / captives + 4 pet companions. |
| BP7 | **M7.2 — Loot + Progression + RPG** | Planned | 5 loot rarity tiers (Common 70%/Magic 25%/Rare 4%/Legendary 1%/Unique 0.1%) + 30+ affixes (weapon/armor/chassis-module) + set bonuses at 2/4/6 + XP+level + 30+ launch achievements + 20+ perks + 10+ curses + inventory grid Tetris (per-chassis sizing) + treasure maps + voyages (Sea of Thieves). |
```

### Acceptance criteria for Edit 2.1

```bash
# Files exist
test -f specs/active/M7.1.md && echo "PASS: M7.1.md exists" || echo "FAIL"
test -f specs/active/M7.2.md && echo "PASS: M7.2.md exists" || echo "FAIL"

# M7 no longer mentions sub-system implementations (they moved)
! grep -q "### Faction system full mechanics" specs/active/M7.md && echo "PASS: factions removed from M7" || echo "FAIL"
! grep -q "### Loot rarity + affixes" specs/active/M7.md && echo "PASS: loot removed from M7" || echo "FAIL"
! grep -q "### XP + level + achievements" specs/active/M7.md && echo "PASS: XP removed from M7" || echo "FAIL"

# M7 still has the spine
grep -q "### Campaign architecture" specs/active/M7.md && echo "PASS: M7 keeps campaign" || echo "FAIL"
grep -q "### Storyteller" specs/active/M7.md && echo "PASS: M7 keeps storyteller" || echo "FAIL"
grep -q "### Base + command core identity" specs/active/M7.md && echo "PASS: M7 keeps base" || echo "FAIL"

# Sister milestone references added
grep -q "M7.1.*Factions" specs/active/M7.md && echo "PASS: M7 references M7.1" || echo "FAIL"
grep -q "M7.2.*Loot" specs/active/M7.md && echo "PASS: M7 references M7.2" || echo "FAIL"

# README updated
grep -q "active%20specs-38" README.md && echo "PASS: README badge 38" || echo "FAIL"
grep -q "M7.1 — Factions + NPCs" README.md && echo "PASS: README BP7 lists M7.1" || echo "FAIL"
grep -q "M7.2 — Loot + Progression" README.md && echo "PASS: README BP7 lists M7.2" || echo "FAIL"
```

### Commit message for Edit 2.1

```
specs: Edit 2.1 — split M7 into M7 + M7.1 + M7.2

M7 was a 25+ subsystem mega-milestone covering campaign + factions +
NPCs + loot + XP + perks + treasures + 16 new crates. Split into 3
coherent scopes:

- M7 — Campaign + Base + Commander spine (campaign / storytellers /
  base building / buy menu / stratagems / AI commander / boss schema)
- M7.1 — Factions + NPCs + Narrative (8 factions / dialog / quests /
  investigation / hostage / pets)
- M7.2 — Loot + Progression + RPG (loot rarity / affixes / XP+level /
  perks/curses / inventory grid / treasures)

All three close in BP7 (sister milestones; closure runs across all).

- specs/active/M7.md trimmed to spine (removed 11 subsystems)
- specs/active/M7.1.md created
- specs/active/M7.2.md created
- README.md updated (badge 36 → 38; BP7 table adds M7.1 + M7.2)

Acceptance criteria from COHERENCE-TIER-2.md § Edit 2.1 — all pass.
```

---

## Edit 2.2 — Split M11.5 into M11.5 + M11.6 + M11.7

### Problem

`specs/active/M11.5.md` (1211 lines) bundles 8+ subsystems:

- PvE Survival mode preset
- 7-step procgen pipeline (per-world generation)
- 3 launch worlds with ore + hazard + structure placement
- **3 inter-planet transport modes** (dropship 8-phase + multi-stage rocket + paired teleporters)
- **Orbital stations** (zero-g manufacturing + modules)
- **Asteroid mining colonies**
- **5 PvE endgame bosses** (Hollow King / Frozen Heart / Crimson Tide / Eclipse Walker / Last Star)
- **12 dynamic world events** (solar flare / pirate raid / trader arrival / etc.)
- 7 new vehicles
- 120-cell race-environmental matrix
- Per-race tech tree branches
- Race-specific colony designs
- Acclimatization mechanics
- ONI-style room types
- NPC dweller management
- 8 procedural quest templates
- Hunger / thirst / sleep / sanity survival mechanics

### Fix

Split into 3 focused milestones in BP10:

| Milestone | Owns |
|---|---|
| **M11.5 — PvE Survival Mode + Procgen** | Game mode preset / 7-step procgen pipeline / 3 launch survival worlds / per-race difficulty matrix / acclimatization mechanic / race-specific tech tree branches / race-specific colony designs / ONI room types / NPC dwellers / procedural quest generator (8 templates) |
| **M11.6 — Inter-Planet Transport + Stations** | 3 transport modes (dropship 8-phase + multi-stage rocket + paired teleporters) / orbital stations (Foundation Kit + modules) / asteroid mining colonies / new vehicles (submarine / cargo freighter / orbital shuttle / amphibious truck) |
| **M11.7 — PvE Endgame Bosses + World Events** | 5 named PvE bosses (Hollow King / Frozen Heart / Crimson Tide / Eclipse Walker / Last Star) / 12 dynamic world events / endgame progression / boss rewards (recipes + artifacts + lore) |

### Files to modify

| File | Action |
|---|---|
| `specs/active/M11.5.md` | **MODIFY** (strip transport + bosses + events) |
| `specs/active/M11.6.md` | **CREATE** |
| `specs/active/M11.7.md` | **CREATE** |
| `README.md` | **MODIFY** (BP10 table; spec count) |

### Step 1: Create `specs/active/M11.6.md`

Extract the following from M11.5 and put in M11.6:

- **Three launch worlds to "build your civilization" on** section (just the worlds list — keep that in M11.5)
- **Inter-planet transport — 3 launch methods** entire section
- **Space stations + orbital bases** entire section
- **Asteroid mining colonies** entire section
- **Player-controllable vehicles (M7 baseline → M11.5 extends)** section (just the M11.6 additions)
- **Inter-planet travel — detailed flight mechanics** entire section
- **Vehicle tank progression (Stationeers + ONI combined)** section

Create the file with this header:

```markdown
# M11.6 — Inter-Planet Transport + Stations

## Status

`active`

## Intent

**M11.6 is the inter-planet transport + space station + asteroid mining milestone** — the system that turns PvE Survival from a single-world game into a multi-world civilization. After M11.6, players can travel between planets via dropship / rocket / teleporter, build orbital stations with zero-g manufacturing bonuses, claim asteroid mining colonies, and pilot specialized vehicles (submarine for Europa / cargo freighter for inter-planetary trade / amphibious truck for Earth).

M11.6 splits this out of M11.5 because transport infrastructure is its own coherent scope distinct from PvE survival mode mechanics. M11.5 ships the survival mode + procgen + race difficulty; M11.6 ships the cross-world systems.

M11.6 promise: **"build your bunker on Earth; mine asteroids in the Belt; trade ore at Mars orbital station — every world becomes part of your civilization."**

## Player-facing behavior

[PASTE all the moved sections here from M11.5]

## Dependencies

- **M11.5 (PvE Survival Mode; must close)** — provides the game mode
- M11 (online co-op infrastructure)
- M9 (server persistence)
- M5.9 (atmospherics — vehicle tank requirements)
- M7.5 (base atmospherics — atmospheric mixer for refilling)
- M7.6 (power kernel — vehicle power)
- M7.8 (crafting — vehicle recipes)
- M5.8 (origin reaction — per-race tank requirements)

## Closure procedure

Reference bundle + 25+ sweep rows (3 transport modes + station construction + asteroid claiming + vehicle catalog). PASS.

## Cross-DR

DR-005, DR-007, DR-024, DR-029, DR-031, **DR-035 (MMO architecture — orbital stations are persistent shards)**, DR-038 (gravity model for orbital math), **DR-039 (inter-planet travel)**, DR-042, DR-048.
```

### Step 2: Create `specs/active/M11.7.md`

Extract the following from M11.5 and put in M11.7:

- **Endgame bosses (per M7 boss patterns + M11.5 PvE-specific)** entire section (5 bosses)
- **World events (per M7 storyteller + M11.5 extends)** entire section (12 events)
- **Storyteller event detailed catalog (12 launch events)** entire section

Create the file:

```markdown
# M11.7 — PvE Endgame Bosses + World Events

## Status

`active`

## Intent

**M11.7 is the PvE endgame milestone** — the 5 named bosses + 12 dynamic world events that give 100+ hour PvE Survival runs their narrative spine. After M11.7, every PvE world has 1-2 boss encounters, every session has dynamic storyteller events, and endgame progression unlocks unique recipes + artifacts + lore.

M11.7 splits this out of M11.5 because boss content + storyteller events are their own coherent scope. M11.5 ships the survival mechanics + procgen; M11.6 ships transport; M11.7 ships endgame challenge.

M11.7 promise: **"100 hours in, you face the Hollow King in Earth's volcanic core, the Frozen Heart in Europa's caverns, the Last Star on Vulcan — each kill rewriting your civilization."**

## Player-facing behavior

### 5 PvE endgame bosses (uses M7 boss schema)

Per M7 boss schema (HP + N phases + special abilities + arena + rewards):

[PASTE the 5 endgame bosses table from M11.5 here]

### 12 dynamic world events

Per M7 storyteller integration:

[PASTE the storyteller event detailed catalog from M11.5 here]

## Content roster at M11.7

| Content | Roster |
|---|---|
| **Boss types** | 5 launch PvE bosses (cumulative with M7 boss schema) |
| **World events** | 12 launch dynamic events |
| **Achievements** | adds 10 PvE-boss-specific achievements |
| **Codex** (toward 600) | +20 entries for boss lore |
| **Music** (toward 30+) | +5 boss-specific tracks |

## Crates / modules touched

| Crate | Status | What |
|---|---|---|
| `cf-content::pve_bosses` | NEW | 5 boss data rows authored against M7 boss schema |
| `cf-storyteller::pve_events` | MODIFY (from M7) | 12 world events registered via M7's event registration API |
| `cf-replay` | MODIFY | boss.* and world_event.* event families (extend M7 schema) |

## Acceptance criteria

```gherkin
Scenario: 5 endgame bosses accessible
  Given M11.7 closure
  Then 5 boss encounters playable across 5 worlds
  And each uses M7 boss schema (multi-phase HP + abilities + arena)

Scenario: Hollow King kill
  Given player on Earth volcanic biome with T3 plasma weapon
  When player engages Hollow King boss:
    Then boss multi-phase fight per M7 boss patterns
  When boss HP = 0:
    Then boss.killed fires
    And unique recipe drops + artifact + lore unlock

Scenario: Solar flare event triggers
  Given player base on Earth + Cassandra Classic storyteller
  When solar_flare event fires:
    Then all electronics offline for 1 in-game hour
    And mass blackout cascade per M7.6
    And player must fall back to manual fallback OR battery reserves

Scenario: 12 events ship as registered storyteller events
  Given M11.7 closure
  Then 12 world events registered via M7 storyteller registration API
  And each event has typed severity + trigger + effect

Scenario: Boss rewards drive endgame progression
  Given player has defeated all 5 endgame bosses
  Then 5 unique boss-only recipes unlocked
  And 5 boss artifacts in inventory
  And M7 codex shows boss lore entries unlocked
  And achievement "boss_master" unlocked
```

## Dependencies

- **M11.5 (PvE Survival Mode; must close)**
- **M11.6 (Transport; must close)** — bosses on distant worlds require transport access
- **M7 (boss schema must be defined)** — M11.7 authors data rows against M7's schema
- M7.7 (worlds + weather)
- M5.5 + M5.6 + M5.7 + M5.8 + M5.9 (all close)

## Closure procedure

Reference bundle + 15 sweep rows (5 bosses × scripted kill + 10 storyteller events triggered). PASS.

## Cross-DR

DR-008, DR-024, DR-031, DR-042, **DR-048 (endgame retention)**, DR-052, DR-056.
```

### Step 3: Modify `specs/active/M11.5.md`

Remove the following sections (search for these headers and delete):

- `### Three launch worlds to "build your civilization" on` — keep the worlds list (M11.5 references) but cut detail
- `### Inter-planet transport — 3 launch methods` — moved to M11.6
- `### Space stations + orbital bases` — moved to M11.6
- `### Asteroid mining colonies` — moved to M11.6
- `### Player-controllable vehicles (M7 baseline → M11.5 extends)` — moved to M11.6
- `### Endgame bosses (per M7 boss patterns + M11.5 PvE-specific)` — moved to M11.7
- `### World events (per M7 storyteller + M11.5 extends)` — moved to M11.7
- `### Storyteller event detailed catalog (12 launch events)` — moved to M11.7
- `### Inter-planet travel — detailed flight mechanics` — moved to M11.6
- `### Vehicle tank progression (Stationeers + ONI combined)` — moved to M11.6

**M11.5 KEEPS:**
- PvE Survival mode game configuration
- Procedural world generation (7-step pipeline)
- 3 launch survival worlds (Earth + Mars + Mimas)
- Survival mechanics — hunger / thirst / sleep / sanity / temperature (Edit 2.4 reroutes these to M5.7)
- Race-specific play styles in PvE survival
- Per-race tech tree branches
- Race-specific colony designs (ONI + Cortex hybrid)
- Acclimatization + adaptation mechanics
- Per-world hazard zones + weather scaling
- Adaptive AI difficulty
- ONI-grade procgen — geyser placement + closed-loop colony progression
- Per-world resource scarcity gradients
- Closed-loop colony progression (ONI parity)
- ONI-style room types + dupe-like NPC management
- Mission generator — procedural quest generation (8 templates)
- Stationeers-grade environmental difficulty — per-world challenge ladder

Update **Intent** to reflect the split:

```markdown
## Intent

**M11.5 is the PvE Survival game mode + procgen milestone** — the Terraria / Stationeers / Minecraft / Cortex-Command hybrid PvE mode preset and procedural world generation. After M11.5, players can spawn on a procedurally-generated planet, survive across all 5 crafting tiers (M7.8), build/sustain race-specific colonies (ONI parity), face acclimatization to harsh worlds, and run solo or 2-8 player coop.

**Sister milestones in BP10:**
- M11.6 — Inter-Planet Transport + Stations (dropship / rocket / teleporter / orbital stations / asteroid mining)
- M11.7 — PvE Endgame Bosses + World Events (5 bosses + 12 storyteller events)

M11.5 is the mode + procgen spine; the others fill specific scopes.

M11.5 promise: **"if you survive 100 hours, you've built a multi-world civilization with your own bunker, your own avatar, and your own race-specific colony design."**
```

### Step 4: Modify `README.md`

Find the active spec count badge (was 38 after Edit 2.1):

**BEFORE:**
```markdown
[![Specs](https://img.shields.io/badge/active%20specs-38%20%28M2.2A..M12%29-blueviolet?style=flat-square)](specs/active/)
```

**AFTER:**
```markdown
[![Specs](https://img.shields.io/badge/active%20specs-40%20%28M2.2A..M12%29-blueviolet?style=flat-square)](specs/active/)
```

Find the BP10 row for M11.5 and update + add new rows:

```markdown
| BP10 | **M11.5 — PvE Survival Mode + Procgen** | Planned | PvE Survival match preset (1-8 player coop) + 7-step procgen pipeline + 3 launch survival worlds (Earth / Mars / Mimas) + per-race × per-environmental-factor difficulty matrix + acclimatization mechanic + race-specific tech tree branches + race-specific colony designs + ONI room types + NPC dwellers + procedural quest generator (8 templates). |
| BP10 | **M11.6 — Inter-Planet Transport + Stations** | Planned | 3 transport modes (dropship 8-phase + multi-stage rocket 5-stage + paired teleporters) + orbital stations (Foundation Kit + 8 modules including zero-g manufacturing) + asteroid mining colonies + 7 new vehicles (submarine / cargo freighter / orbital shuttle / amphibious truck / asteroid drill ship / etc.). |
| BP10 | **M11.7 — PvE Endgame Bosses + World Events** | Planned | 5 named PvE bosses (Hollow King volcanic / Frozen Heart cryogenic / Crimson Tide dust / Eclipse Walker microgravity / Last Star Vulcan) authored against M7 boss schema + 12 dynamic world events (solar flare / pirate raid / trader arrival / anomaly storm / etc.) registered via M7 storyteller API. |
```

### Acceptance criteria for Edit 2.2

```bash
# Files exist
test -f specs/active/M11.6.md && echo "PASS: M11.6.md exists" || echo "FAIL"
test -f specs/active/M11.7.md && echo "PASS: M11.7.md exists" || echo "FAIL"

# M11.5 no longer mentions moved sections
! grep -q "^### Inter-planet transport — 3 launch methods" specs/active/M11.5.md && echo "PASS: M11.5 → transport moved" || echo "FAIL"
! grep -q "^### Space stations + orbital bases" specs/active/M11.5.md && echo "PASS: M11.5 → stations moved" || echo "FAIL"
! grep -q "^### Endgame bosses" specs/active/M11.5.md && echo "PASS: M11.5 → bosses moved" || echo "FAIL"

# M11.6 has the transport content
grep -q "Inter-planet transport" specs/active/M11.6.md && echo "PASS: M11.6 has transport" || echo "FAIL"
grep -q "orbital stations" specs/active/M11.6.md && echo "PASS: M11.6 has stations" || echo "FAIL"
grep -q "asteroid mining" specs/active/M11.6.md && echo "PASS: M11.6 has asteroid mining" || echo "FAIL"

# M11.7 has the bosses + events
grep -q "Hollow King" specs/active/M11.7.md && echo "PASS: M11.7 has Hollow King" || echo "FAIL"
grep -q "Last Star" specs/active/M11.7.md && echo "PASS: M11.7 has Last Star" || echo "FAIL"
grep -q "solar flare" specs/active/M11.7.md && echo "PASS: M11.7 has solar flare event" || echo "FAIL"

# M11.5 still has procgen + survival
grep -q "7-step procgen" specs/active/M11.5.md && echo "PASS: M11.5 keeps procgen" || echo "FAIL"
grep -q "Acclimatization" specs/active/M11.5.md && echo "PASS: M11.5 keeps acclimatization" || echo "FAIL"

# README updated
grep -q "active%20specs-40" README.md && echo "PASS: README badge 40" || echo "FAIL"
grep -q "M11.6 — Inter-Planet Transport" README.md && echo "PASS: README BP10 lists M11.6" || echo "FAIL"
grep -q "M11.7 — PvE Endgame" README.md && echo "PASS: README BP10 lists M11.7" || echo "FAIL"
```

### Commit message for Edit 2.2

```
specs: Edit 2.2 — split M11.5 into M11.5 + M11.6 + M11.7

M11.5 was 1211 lines covering 16+ subsystems. Split into 3 coherent
scopes in BP10:

- M11.5 — PvE Survival Mode + Procgen (mode preset / 7-step procgen /
  3 launch worlds / race difficulty / acclimatization / colony designs)
- M11.6 — Inter-Planet Transport + Stations (3 transport modes /
  orbital stations / asteroid mining / 7 new vehicles)
- M11.7 — PvE Endgame Bosses + World Events (5 named bosses authored
  against M7 boss schema / 12 dynamic world events / endgame
  progression)

All three close in BP10 (sister milestones; closure runs across all).

- specs/active/M11.5.md trimmed (removed 10 subsystems)
- specs/active/M11.6.md created
- specs/active/M11.7.md created
- README.md updated (badge 38 → 40; BP10 table extended)

Acceptance criteria from COHERENCE-TIER-2.md § Edit 2.2 — all pass.
```

---

## Edit 2.3 — Centralize boss schema in M7

### Problem

Boss patterns are mentioned in:
- M2.2B (Spotter mini-boss; "multi-phase HP thresholds")
- M7 (Faction Boss / World Boss / Hidden Boss with brief descriptions)
- M11.5 (5 PvE-specific bosses — now M11.7 after Edit 2.2)
- M11 (Boss Rush endgame mode)
- M12 (faction war + persistent bosses)

Each mention has slightly different schema (HP + phases / special abilities / drops). No canonical data model.

### Fix

Define the **canonical boss schema** in M7 (the campaign milestone that owns combat content). Other milestones reference M7's schema and author data rows.

### Files to modify

| File | Action |
|---|---|
| `specs/active/M7.md` | **MODIFY** (add canonical boss schema section) |
| `specs/active/M11.7.md` | **MODIFY** (reference M7 boss schema) |
| `specs/active/M2.2B.md` | **MODIFY** (Spotter authored against M7 schema; add note) |

### Step 1: Modify `specs/active/M7.md`

Find the **Mini-boss + boss patterns** section. Replace its content with:

```markdown
### Mini-boss + boss patterns (canonical boss schema lives here)

M7 owns the **canonical boss schema** — the data model every boss in every milestone (M2.2B Spotter, M11.7 PvE endgame, M12 MMO bosses) authors against.

#### Boss schema (locked v0.1)

```rust
pub struct BossDef {
  pub id: BossId,
  pub name: String,
  pub kind: BossKind,                     // MiniBoss | FactionBoss | WorldBoss | HiddenBoss | EndgameBoss
  pub max_hp: f32,
  pub phases: Vec<BossPhase>,             // 2-5 phases per boss
  pub special_abilities: Vec<SpecialAbility>,
  pub arena: BossArena,                   // arena bounds + obstacles + escape rules
  pub rewards: BossRewards,
  pub lore_codex_unlock: Vec<CodexId>,
  pub achievement_unlock: Option<AchievementId>,
  pub immunity_tags: Vec<DamageKind>,     // some bosses immune to specific damage kinds
  pub mission_critical: bool,             // boss cannot be one-shot per CCCP pattern
  pub dialog_intro: Option<DialogId>,     // per M7.1 dialog system
}

pub struct BossPhase {
  pub id: u8,                              // 1, 2, 3, ...
  pub hp_threshold_pct: f32,               // phase activates when HP crosses below this
  pub doctrine_shift: AiDoctrine,          // boss AI tactic per phase
  pub ability_unlocks: Vec<SpecialAbility>,
  pub arena_changes: Vec<ArenaChange>,     // e.g. "spawn additional cover" / "open new exits"
  pub minions_spawn: Vec<MinionWave>,
}

pub struct SpecialAbility {
  pub id: AbilityId,
  pub name: String,                        // e.g. "Pyroclastic Burst" for Hollow King
  pub damage_kind: DamageKind,
  pub cooldown_ticks: u32,
  pub telegraph_ticks: u32,                // visual warning before activation
  pub area_affected: AreaShape,
  pub counter_play: Option<CounterPlay>,   // e.g. "break shield with electric weapon"
}

pub struct BossArena {
  pub bounds: Aabb,
  pub escape_rule: EscapeRule,             // OneWayCutsceneEntry | TwoWayDoor | TimedRetreat
  pub spawn_zones: Vec<SpawnZone>,         // for reinforcements
  pub hazard_zones: Vec<HazardZone>,
}

pub struct BossRewards {
  pub guaranteed_drops: Vec<ItemId>,       // always drop on kill
  pub legendary_drop_chance: f32,          // bonus legendary roll
  pub xp_bonus: u32,
  pub faction_reputation_delta: i32,
  pub recipe_unlocks: Vec<RecipeId>,
  pub artifact_unlocks: Vec<ArtifactId>,
}

pub enum BossKind {
  MiniBoss,                                // M2.2B baseline; one per scenario
  FactionBoss,                             // M7 launch; tied to faction
  WorldBoss,                               // M11+ cross-mission shared
  HiddenBoss,                              // M7+ secret unlock chain
  EndgameBoss,                             // M11.7 PvE-survival named bosses
}

pub enum EscapeRule {
  OneWayCutsceneEntry,                     // player commits; no retreat (e.g. Hollow King)
  TwoWayDoor,                              // player can retreat anytime
  TimedRetreat,                            // arena seals after N seconds
}
```

#### Boss authoring contract

All bosses across milestones MUST:

1. Author a `<boss_id>.boss.ron` file at `content/bosses/`
2. Validate via `cargo run -p cf-mod -- validate content/bosses/`
3. Reference M7's schema (no custom variant fields)
4. Implement 2-5 phases (no single-phase bosses; that's just a tough enemy)
5. Include `dialog_intro` if the boss has lore (per M7.1 dialog system)
6. Emit `boss.*` event family per the locked schema

#### Boss event family (locked v0.1; M3A locks; producers ladder up)

- `boss.entered_arena { boss_id, actor_id }`
- `boss.phase_changed { boss_id, from_phase, to_phase, hp_pct }`
- `boss.special_ability_telegraphed { boss_id, ability_id, telegraph_duration }`
- `boss.special_ability_triggered { boss_id, ability_id, area_affected }`
- `boss.counter_play_succeeded { boss_id, ability_id, player_action }`
- `boss.minions_spawned { boss_id, wave_id, minion_count }`
- `boss.killed { boss_id, killer_actor_id, time_in_arena_ticks }`
- `boss.player_died_in_arena { boss_id, retry_offered }`
- `boss.escaped_by_player { boss_id, reason }` (only valid for TwoWayDoor)

#### Bosses authored across milestones

| Milestone | Bosses authored against M7 schema |
|---|---|
| M2.2B | Spotter (mini-boss) |
| **M7** | Faction Boss × 8 (per faction) + 1 Hidden Boss + 1 World Boss (campaign endgame) |
| M11.7 | Hollow King + Frozen Heart + Crimson Tide + Eclipse Walker + Last Star |
| M12 | MMO World Bosses (cross-shard coordinated takedown) |

Each authoring milestone ships boss DATA, not boss CODE — `cf-boss` engine code lives in M7.
```

### Step 2: Modify `specs/active/M11.7.md`

In the "5 PvE endgame bosses" section, add at the top:

```markdown
**Schema:** All 5 bosses are authored against the canonical `BossDef` schema defined in `specs/active/M7.md` § Mini-boss + boss patterns. M11.7 ships DATA rows; the boss engine lives in M7's `cf-boss` crate.
```

### Step 3: Modify `specs/active/M2.2B.md`

In the "Mini-boss patterns" section, add at the top:

```markdown
**Schema (forward-compat):** Spotter mini-boss is authored against the canonical `BossDef` schema defined in M7. At M2.2B, the schema doesn't exist yet (M7 closes later), so Spotter uses a pre-schema-locked form. When M7 closes, M2.2B's Spotter data row migrates to the canonical schema (no behavior change; schema-conformance only).
```

### Acceptance criteria for Edit 2.3

```bash
# M7 has the canonical boss schema
grep -q "Boss schema (locked v0.1)" specs/active/M7.md && echo "PASS: M7 has boss schema" || echo "FAIL"
grep -q "pub struct BossDef" specs/active/M7.md && echo "PASS: M7 has BossDef" || echo "FAIL"

# Cross-references in M11.7 + M2.2B
grep -q "canonical BossDef" specs/active/M11.7.md && echo "PASS: M11.7 references schema" || echo "FAIL"
grep -q "canonical BossDef" specs/active/M2.2B.md && echo "PASS: M2.2B references schema" || echo "FAIL"
```

### Commit message for Edit 2.3

```
specs: Edit 2.3 — centralize boss schema in M7

Boss patterns were defined ad-hoc across M2.2B (Spotter), M7 (campaign
bosses), and M11.5 (PvE bosses, now M11.7). No canonical data model.

Defined the canonical BossDef schema in M7 § Mini-boss + boss patterns
(BossDef + BossPhase + SpecialAbility + BossArena + BossRewards +
BossKind + EscapeRule). Boss event family (locked v0.1) defined too.

M11.7 + M2.2B now reference M7's schema; they author data rows, not
schema.
```

---

## Edit 2.4 — Add hunger/thirst/sleep_dep/sanity_low as M5.7 afflictions

### Problem

`specs/active/M11.5.md` defines survival mechanics for hunger / thirst / sleep / sanity / temperature in a "Survival affliction cascades" section. But these are **afflictions** that stack/escalate/clear per the M5.7 affliction system. They should live in M5.7.

`specs/active/M5.7.md` currently has 18 affliction kinds. Survival afflictions belong with them.

### Fix

Extend M5.7's affliction list to 22 (add 4 new kinds: `hunger`, `thirst`, `sleep_dep`, `sanity_low`). M11.5 references M5.7 instead of defining its own affliction mechanics.

### Files to modify

| File | Action |
|---|---|
| `specs/active/M5.7.md` | **MODIFY** (extend affliction list to 22) |
| `specs/active/M11.5.md` | **MODIFY** (replace survival affliction definitions with M5.7 references) |

### Step 1: Modify `specs/active/M5.7.md`

Find the **18 affliction kinds — full mechanics** table and add 4 new rows at the bottom:

```markdown
| `hunger` | caloric_energy < 20 (per M5.8) | -0.1 HP/s + speed × 0.8 + caloric drain × 2 (death spiral) | Eat food (per M7.8 cooking) |
| `thirst` | water reservoir empty | aim wobble + mental fog + speed × 0.85 | Drink water (base condenser per M7.5; lake/river extraction) |
| `sleep_dep` | 4+ in-game hours missed sleep | vision tunneling + reaction time +50% | Sleep at bed (per M7 base object) |
| `sanity_low` | sanity accumulator < 30% | panic affliction + AI decisions impaired (player actor) + storyteller intensity spike | Recreation per M7 schedule + therapy NPC (M7.1) |
```

Update the section header from "18 affliction kinds" to "22 affliction kinds":

**BEFORE:**
```markdown
### 18 affliction kinds — full mechanics
```

**AFTER:**
```markdown
### 22 affliction kinds — full mechanics (18 baseline + 4 survival)

Baseline 18 afflictions cover combat + environment. Survival-mode afflictions (4 new: hunger / thirst / sleep_dep / sanity_low) are required when PvE Survival mode is active (per M11.5). All 22 use the same affliction system (stack / escalate / clear) defined below.
```

Add acceptance criterion at the end of M5.7's acceptance section:

```gherkin
Scenario: Survival afflictions only active in PvE Survival mode
  Given PvP arena mode (per DR-042 match grammar)
  Then hunger / thirst / sleep_dep / sanity_low afflictions are NOT spawned (PvP doesn't have survival mechanics)
  Given PvE Survival mode (per M11.5)
  Then survival afflictions are tracked per actor + escalate per M11.5 timer
  And affliction.applied fires when threshold crossed
```

### Step 2: Modify `specs/active/M11.5.md`

Find the **Survival mechanics — hunger / thirst / sleep / sanity / temperature** section. Replace the affliction table with a reference:

**BEFORE:**
```markdown
**Survival affliction cascades (per M5.7 affliction layer):**

- Hunger > 80%: hunger affliction; speed × 0.8; caloric drain × 2 (death spiral)
- Thirst > 80%: thirst affliction; aim wobble; mental fog
- Sleep deprivation > 4h missed: vision tunneling; reaction time +50%
- Sanity < 30%: panic affliction; AI decisions impaired
- Hypothermia (cold environment + suit fail): hypothermic affliction stacks
- Hyperthermia (hot environment): hyperthermic stacks
```

**AFTER:**
```markdown
**Survival afflictions (defined canonically in M5.7):**

PvE Survival mode activates 4 affliction kinds from M5.7's 22-affliction taxonomy:

- `hunger` — caloric_energy < 20 (per M5.8 origin matrix) → -0.1 HP/s + speed reduction + caloric drain × 2
- `thirst` — water reservoir empty → aim wobble + mental fog + speed reduction
- `sleep_dep` — 4+ in-game hours missed sleep → vision tunneling + reaction time +50%
- `sanity_low` — sanity accumulator < 30% → panic + AI decisions impaired (player actor)

See `specs/active/M5.7.md` § 22 affliction kinds for full mechanics, escalation rules, and clear conditions.

**Temperature afflictions (hyperthermic / hypothermic)** are baseline M5.7 afflictions — PvE Survival doesn't add new behavior; it just makes them more common via M11.5's per-world environmental difficulty matrix.
```

### Acceptance criteria for Edit 2.4

```bash
# M5.7 has 22 afflictions
grep -q "22 affliction kinds" specs/active/M5.7.md && echo "PASS: M5.7 has 22 afflictions" || echo "FAIL"
grep -q "| \`hunger\`" specs/active/M5.7.md && echo "PASS: hunger added" || echo "FAIL"
grep -q "| \`thirst\`" specs/active/M5.7.md && echo "PASS: thirst added" || echo "FAIL"
grep -q "| \`sleep_dep\`" specs/active/M5.7.md && echo "PASS: sleep_dep added" || echo "FAIL"
grep -q "| \`sanity_low\`" specs/active/M5.7.md && echo "PASS: sanity_low added" || echo "FAIL"

# M5.7 has the PvE-only acceptance scenario
grep -q "Survival afflictions only active in PvE Survival mode" specs/active/M5.7.md && echo "PASS: PvE-only scenario" || echo "FAIL"

# M11.5 references M5.7 instead of defining mechanics
grep -q "defined canonically in M5.7" specs/active/M11.5.md && echo "PASS: M11.5 references M5.7" || echo "FAIL"
```

### Commit message for Edit 2.4

```
specs: Edit 2.4 — add hunger/thirst/sleep_dep/sanity_low to M5.7

M11.5 defined survival mechanics for hunger / thirst / sleep / sanity
as their own stacking/clearing rules. But these are afflictions and
belong with M5.7's 18-affliction taxonomy.

Extended M5.7 to 22 affliction kinds (added 4 survival-mode afflictions).
M11.5 now references M5.7 instead of redefining mechanics.

Survival afflictions are only spawned in PvE Survival mode (per
M11.5); PvP doesn't activate them.
```

---

## Tier 2 — Full acceptance criteria

```bash
cd /Users/erol/projects/corefall

# Edit 2.1 checks
test -f specs/active/M7.1.md
test -f specs/active/M7.2.md
! grep -q "### Faction system full mechanics" specs/active/M7.md
! grep -q "### Loot rarity + affixes" specs/active/M7.md
grep -q "### Campaign architecture" specs/active/M7.md
grep -q "M7.1.*Factions" specs/active/M7.md
grep -q "active%20specs-40" README.md
grep -q "M7.1 — Factions" README.md
grep -q "M7.2 — Loot" README.md

# Edit 2.2 checks
test -f specs/active/M11.6.md
test -f specs/active/M11.7.md
! grep -q "^### Inter-planet transport — 3 launch methods" specs/active/M11.5.md
! grep -q "^### Endgame bosses" specs/active/M11.5.md
grep -q "Hollow King" specs/active/M11.7.md
grep -q "M11.6 — Inter-Planet Transport" README.md
grep -q "M11.7 — PvE Endgame" README.md

# Edit 2.3 checks
grep -q "Boss schema (locked v0.1)" specs/active/M7.md
grep -q "pub struct BossDef" specs/active/M7.md
grep -q "canonical BossDef" specs/active/M11.7.md

# Edit 2.4 checks
grep -q "22 affliction kinds" specs/active/M5.7.md
grep -q "| \`hunger\`" specs/active/M5.7.md
grep -q "defined canonically in M5.7" specs/active/M11.5.md

# File count
test "$(ls specs/active/M*.md | wc -l | tr -d ' ')" = "40"

# Workspace still builds
cd game && cargo build && cargo clippy --all-targets -- -D warnings
cd ..

echo "TIER 2 — ALL CHECKS PASS"
```

### Tier 2 PR template

**Title:** `specs: tier-2 coherence (M7 split + M11.5 split + boss schema + hunger afflictions)`

**Body:**

```markdown
## Summary

Tier 2 of the spec coherence pass per `specs/COHERENCE-PLAN.md`. Splits 2 mega-milestones, centralizes boss schema, and reroutes survival afflictions:

1. **Edit 2.1** — Split M7 into M7 + M7.1 + M7.2 (campaign + factions/NPC + loot/RPG)
2. **Edit 2.2** — Split M11.5 into M11.5 + M11.6 + M11.7 (survival + transport + endgame)
3. **Edit 2.3** — Centralize boss schema in M7 (BossDef + phases + abilities + arena + rewards)
4. **Edit 2.4** — Add hunger/thirst/sleep_dep/sanity_low as M5.7 affliction kinds (18 → 22)

## Active spec count

- Before: 36
- After: 40 (added M7.1, M7.2, M11.6, M11.7)

## Verification

All acceptance checks from `COHERENCE-TIER-2.md` § Tier 2 — Full acceptance criteria. All PASS.

## Next

Tier 3 (polish) + Tier 4 (gaps) can now run in parallel. See `specs/COHERENCE-TIER-3.md` and `specs/COHERENCE-TIER-4.md`.
```

---

## Done with Tier 2

Once the PR merges:
- ✅ M7 + M7.1 + M7.2 each cover focused scope
- ✅ M11.5 + M11.6 + M11.7 each cover focused scope
- ✅ Boss schema centralized in M7 (canonical owner)
- ✅ M5.7 has 22 afflictions (4 survival kinds added)
- ✅ 40 active specs

**Proceed to `COHERENCE-TIER-3.md`** for polish and consolidation.
