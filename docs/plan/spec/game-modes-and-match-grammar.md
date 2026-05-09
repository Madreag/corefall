---
type: spec
status: design-intent-post-m1
authority: "Canonical contract for the Match Grammar that defines every game mode: Bunker Defence (asymmetric attacker-vs-defender), Symmetric Arena (1v1 / 2v2 / 3v3 / NvN), Free-For-All (1v1v1 / 1v1v1v1), Asymmetric N-team (2v1 / 3v1 / etc.), Coop-vs-AI, Campaign. Coop within teams. AI fills empty player slots. Every mission director manifest declares a Match config. Server modes (`coop_room` / `pvp_arena` / `lan_room` / `mmo_shard`) accept Match configs."
ready_when: "Match config schema exists; mission manifest carries Match block; server validates Match on join; Bunker Defence E2E proof mission ships at M7; team configs (1v1 / 2v2 / 3v3 / 1v1v1 / 2v1) ship at M11; full MMO match grammar lands at M12."
feeds:
  - DR-002
  - DR-005
  - DR-009
  - DR-013
  - DR-014
  - DR-015
  - DR-016
  - DR-017
  - DR-022
  - DR-027
  - DR-029
  - DR-031
  - DR-034
  - DR-035
  - DR-042
---

← [[index|vault home]] · [[spec/index|spec section]] · [[spec/mission-director-slice-a|mission director]] · [[spec/server-app-architecture|server architecture]] · [[spec/persistent-mmo-architecture|persistent MMO]] · [[spec/celestial-bodies-and-worlds-model|worlds catalog]] · [[spec/environmental-conditions-model|environmental conditions]] · [[spec/comms-voice-and-radio-model|comms/voice/radio]] · [[spec/prototype-roadmap|native roadmap]] · [[decisions/dr-005-multiplayer-posture|DR-005]] · [[decisions/dr-027-combat-base-scope|DR-027]] · [[decisions/dr-034-dedicated-server-application|DR-034]] · [[decisions/dr-042-game-modes-and-match-grammar-direction|DR-042]]

# Game Modes And Match Grammar

> [!summary] What this page is
> The single grammar that describes EVERY playable match: who's on what team, what their objectives are, how they win, how they lose, where the spawn points are, whether AI fills empty slots, and what the comms / voice / coop posture is. Coverage:
>
> - **Bunker Defence** — flagship mode. 1+ defenders (with bunker, base power, turrets, shields, sealed life support per [[decisions/dr-027-combat-base-scope]]) vs 1+ attackers (with dropship, breach kit, pressure-attack tools). Coop on either or both sides. Mission director authors objectives, victory conditions, and dynamic events.
> - **Symmetric Arena** — 2-N teams with equal starts. 1v1, 2v2, 3v3, NvN.
> - **Free-For-All** — every player is their own team. 1v1v1, 1v1v1v1, etc.
> - **Asymmetric N-team** — 2v1, 3v1, 4v2, with different starting conditions per team.
> - **Coop-vs-AI** — all human players on one team vs AI-only opposition.
> - **Campaign** — solo or coop linear / branching mission progression.
>
> Every match is one Match record. Every server mode (`coop_room` / `pvp_arena` / `lan_room` / `mmo_shard`) accepts Match configs. Modders author new modes via Match grammar + content data.

> [!warning] Authority boundary
> Captured 2026-05-06 as **design intent**. The Match schema (team count, role kinds, objectives, victory conditions, spawn rules, AI fill, comms policy) is committed. Specific tuning (objective values, time limits, default team sizes per mode) stays open until prototype evidence backs them.

> [!important] Out of scope right now
> M0..M5.9 stay match-config-only. M7 (Mission Director) adds Match grammar to mission manifest + ships **Bunker Defence Proof Mission** as the M7 closure. M11 (Online Co-op) ships team configs (1v1 / 2v2 / 3v3 / FFA / 2v1). M12 (PvP + MMO) ships full Bunker Defence + community-hosted PvP arenas.

## Why This Page Exists

The vault has scattered references to game modes:

- DR-005 lists "solo + LAN + online co-op + community-hostable public PvP arenas + persistent MMO shards"
- DR-027 says "deep combat-base" which IS the bunker
- DR-034 lists `cf-server` modes: `coop_room`, `pvp_arena`, `lan_room`, `mmo_shard`, `lobby_directory`
- DR-035 talks about MMO shards
- M7 mission director has a "proof mission" objective

But there's no single page that says **"Bunker Defence is the flagship mode and here's its grammar"** OR **"any match is N teams of M players with these properties"**. Authors today would have to interpret across 5 spec pages. This page collapses that into one contract.

## Principles (locked)

1. **One Match grammar covers everything.** Bunker Defence is a Match preset; symmetric arena is another preset; FFA is another. Modders author new modes by composing the same building blocks.
2. **Asymmetric is first-class.** The grammar does NOT assume two equal teams. 2v1, 3v1, 4v2 are valid. Per-team properties differ.
3. **Coop within teams is universal.** Any team with > 1 player slot allows coop; AI fills empty slots when configured.
4. **AI fills any empty slot.** A 4-player Bunker Defence with 1 human and 3 AI is the same Match as 4 humans; the Match config doesn't care.
5. **Mission director owns Match content.** Per [[spec/mission-director-slice-a]] mission manifest declares the Match block. Director can author dynamic events that change Match state mid-game (reinforcements, defectors, third-team appearance).
6. **Server mode is just a deployment shape.** `coop_room`, `pvp_arena`, `lan_room`, `mmo_shard` per [[spec/server-app-architecture]] all accept Match configs; the server enforces Match rules.
7. **Replay deterministic.** Match state is server-authoritative; replay records all Match events.
8. **Origin-aware Bunker Defence.** Defenders may be all-robot for vacuum-bunker scenarios; attackers may be all-human for "breach the corp citadel" scenarios per [[spec/origin-reaction-and-resource-model]].

## The Match Schema

```text
struct Match {
    id: MatchId,
    mode: ModePreset,                       // BunkerDefence | SymmetricArena | FreeForAll | AsymmetricNTeam | CoopVsAI | Campaign | Modder("custom_id")
    asymmetric: bool,
    coop_within_teams: bool,
    max_total_players: u32,
    duration_policy: DurationPolicy,        // FixedTimer | UntilObjective | UntilElimination | Endless

    teams: Vec<Team>,
    spectators: SpectatorPolicy,            // None | PostElimination | AdminAlways

    map: MapRef {
        world: WorldRef,                    // [[spec/celestial-bodies-and-worlds-model]]
        scenario: ScenarioRef,              // anchor + bounds
        weather_authority: WeatherAuthority, // mission_director | scenario_random | shard_persistent
    },

    comms_policy: CommsPolicy,              // [[spec/comms-voice-and-radio-model]]
                                            // realistic | proximity_only | global_chat | cross_team_disabled
}

struct Team {
    id: TeamId,
    kind: TeamKind,                         // Attacker | Defender | Neutral | Survivor | Hostile | Custom
    display_name: String,
    color: TeamColor,
    
    player_slots: u32,                      // 1..N humans
    ai_fill: AiFillPolicy {
        fill_empty: bool,                   // bot fills empty slots
        ai_doctrine: DoctrineRef,            // which doctrine the bots use
        difficulty: DifficultyBand,          // Recruit | Veteran | Elite
    },

    objectives: Vec<ObjectiveRef>,           // per-team objectives
    victory_conditions: Vec<Condition>,
    loss_conditions: Vec<Condition>,

    spawn_rules: SpawnRules {
        spawn_zones: Vec<ZoneRef>,
        respawn_policy: RespawnPolicy,      // None | TimedTickets | InfiniteWithCooldown
        starting_loadout: LoadoutRef,
        starting_chassis: Vec<ChassisRef>,
        deployment: DeploymentMode,          // Dropship | Walked | Spawn | Rooted
    },

    starting_resources: Resources {
        gold_or_credits: f32,
        starting_base: Option<BaseRef>,      // for defenders; references DR-027 base modules
    },

    bunker_owner: bool,                     // true if this team starts with the rooted bunker
}

enum TeamKind {
    Attacker,                               // breaches the bunker; usually dropship-deployed
    Defender,                               // owns the bunker; usually rooted
    Neutral,                                // coexists; doesn't initiate combat
    Survivor,                               // lives by environmental survival
    Hostile,                                // PvE-only adversary; does not have human players
    Custom(String),                         // modder-named role
}

enum DurationPolicy {
    FixedTimer { seconds: u32 },
    UntilObjective,
    UntilElimination,
    Endless,                                // for sandbox / persistent shards
}

enum CommsPolicy {
    Realistic,                              // full DR-043 voice + radio simulation
    ProximityOnly,                          // voice only; no radio
    GlobalChat,                             // always-on team chat (no realism)
    CrossTeamDisabled,                      // teams can't hear each other
}
```

## Mode Presets (Locked Launch Set)

### Bunker Defence (flagship)

| Property | Value |
|---|---|
| `mode` | BunkerDefence |
| `asymmetric` | true |
| Default teams | 2 (Attacker + Defender) |
| Default players | 1-8 each side; coop within teams |
| Defender start | Rooted bunker (DR-027); base power; turrets; shields; sealed life support; pre-deployed AI guards |
| Attacker start | Dropship deployment OR walked-in approach; breach kit (charges, drills); buy menu for reinforcements |
| Defender objectives | Survive timer OR defeat all attackers OR protect command core |
| Attacker objectives | Destroy command core OR breach inner sanctum + extract mission item OR eliminate all defenders |
| Map | Any world (Earth → Mars → Mimas → Vulcan → modder); environment shapes the fight (vacuum bunker = airlock-cycle pressure attacks; lava bunker = thermal envelope; low-g bunker = grenade arcs) |
| Variants | 1v1 (skirmish), 2v2 (squad fireteam), 3v3 (full company), 4v4 (large skirmish), Coop-Defence (humans defend vs AI attackers), Coop-Attack (humans attack AI bunker) |

### Symmetric Arena

| Property | Value |
|---|---|
| `mode` | SymmetricArena |
| `asymmetric` | false |
| Default teams | 2-N (commonly 2) |
| Default players | 1-N per team (1v1, 2v2, 3v3, etc.) |
| Start | Equal symmetric spawn zones; same loadout buy budget; same map terrain access |
| Objectives | Eliminate enemy team; control center node; extract a marker |
| Variants | 1v1 quickplay, 2v2 ranked, 3v3 ranked, NvN league |

### Free-For-All

| Property | Value |
|---|---|
| `mode` | FreeForAll |
| `asymmetric` | false (per-player; treats each player as a team of 1) |
| Default players | 3-8 |
| Start | Distributed spawn zones; same starting loadout |
| Objectives | Last surviving player; or first to N kills; or first to extract a single MacGuffin |
| Variants | 1v1v1 three-way, 1v1v1v1 four-way, 1v1v1v1v1 five-way (rare) |

### Asymmetric N-Team

| Property | Value |
|---|---|
| `mode` | AsymmetricNTeam |
| `asymmetric` | true |
| Default teams | 2-3 |
| Default players | varies (e.g., 2v1, 3v1, 4v2) |
| Start | Per-team different conditions (one team has bunker; another has dropship; third arrives mid-mission) |
| Objectives | Per-team different |
| Variants | 2v1 ambush, 3v1 hunt, 4v2 holdout |

### Coop-vs-AI

| Property | Value |
|---|---|
| `mode` | CoopVsAI |
| `asymmetric` | true (humans vs AI-only adversary) |
| Default players | 1-8 humans on one team |
| Adversary | AI-only Hostile team(s); difficulty scales with player count |
| Variants | Tutorial co-op, story-mission co-op, survival co-op, horde wave defence |

### Campaign

| Property | Value |
|---|---|
| `mode` | Campaign |
| `asymmetric` | varies per mission |
| Default players | 1-4 (solo + small co-op) |
| Structure | Linear / branching mission progression with persistent state ([[spec/progression-retention]]); per-mission Match configs |
| Variants | Solo campaign, 2-player co-op campaign, 4-player co-op campaign |

## Bunker Defence — Detailed Grammar

Bunker Defence is the flagship. Locked details:

### Map archetype

- **Bunker zone**: defender-owned; rooted command core ([[spec/command-core-base-power]]); base power; turrets / shields / repair pads / hangar / storage / traps / breachable structure (DR-027); sealed atmosphere (DR-037).
- **Approach zone**: open ground / wreckage / debris field; attacker dropship LZ; bunker exterior surface.
- **Inner sanctum**: deepest defender position; mission item / command core target; collapse-on-loss trigger.

### Defender starting state

- Command core rooted (per DR-015); embedding into avatar = avatar boost + base power loss tradeoff.
- Base power generator(s) running; supplies turrets / shields / repair / sensors / doors.
- Atmosphere sealed at world.surface.atmosphere_ambient pressure (or argon/N2 inert if defender prefers low-combustion).
- Pre-deployed AI guards (configurable; default = number of player slots × 2).
- Buy menu for reinforcements (gold/credits per DR-031 economy).

### Attacker starting state

- Dropship deployed (Cortex Command-style descent) OR walked-in approach.
- Breach kit: charges, drills, plasma cutters, EMP pulse to disable shields, gas grenades to vent atmosphere, hazardous-material weapons.
- Buy menu for reinforcements.
- Initial spawn zone outside the bunker; respawn at LZ via dropship.

### Win/loss conditions

- **Defender wins**: timer expires OR all attackers eliminated OR command core safe at end.
- **Attacker wins**: command core destroyed OR mission item extracted OR all defenders eliminated.
- **Mutual loss**: both sides eliminated; map declared draw.
- **Asymmetric draw**: attacker survives but doesn't extract; declared partial victory per scenario.

### Environment shapes the fight

- **Vacuum bunker** (Moon, Mimas, Phobos): defenders fight in suits; attackers can vent the bunker by breaching a wall — defender suits become critical. Materials: oxygen-rich room interior; vacuum exterior.
- **Hot bunker** (Vulcan): defender suits cooler; attackers risk autoignition with explosives in the volatile-rich atmosphere; thermal envelope matters.
- **Low-g bunker** (Phobos, Mimas): grenades have hour-long apex; ballistic arcs change; mech weight matters less; rocket recoil matters more.
- **Underwater bunker** (post-launch): submerged defender; pressure-suited attackers; comms via sonar (DR-043 supports underwater propagation as future flag).
- **Storm-active bunker** (Mars dust storm): visibility reduced for both; sensor advantage to defender; cover for attacker.

### Replay events

Per [[references/prototype-run-bundle-schema]], `match` event category:

| Event | Required Fields |
|---|---|
| `match.started` | match_id, mode, teams |
| `match.team_state_changed` | team_id, old_state, new_state |
| `match.objective_progressed` | team_id, objective_id, progress_0_1 |
| `match.victory_condition_met` | team_id, condition |
| `match.player_joined` | match_id, team_id, player_id |
| `match.player_left` | match_id, team_id, player_id, reason (disconnect / kicked / quit) |
| `match.ai_filled_slot` | match_id, team_id, slot_id, doctrine, difficulty |
| `match.bunker_breach_event` | (Bunker-Defence-specific) breach point, attacker_id, parent_event_id |
| `match.command_core_state_changed` | (Bunker-Defence-specific) old_state, new_state |
| `match.match_ended` | match_id, outcome |

## AI Doctrine Per Mode (M6.6 promoted to AI Environmental Competence)

| Mode | AI Doctrine Notes |
|---|---|
| Bunker Defence (Defender AI) | Patrol patterns, sensor sweep, hold positions, cycle airlocks, rebuild damaged turrets, escalate base power consumption when shields engaged. Attacker-aware: detect breach attempts, vent atmosphere defensively, retreat to inner sanctum. |
| Bunker Defence (Attacker AI) | Recon dropship, scout perimeter, plan breach point (gas line, pressure seal, weak structural panel), call coordinated assault, exploit storm cover, extract on success. |
| Symmetric Arena | Standard combat doctrine; no special "bunker" or "dropship" affordances. |
| Free-For-All | Treat every actor (human or AI) as a separate threat; no friendly chatter. Maybe hostility logic (avoid engaging if outnumbered, ambush opportunists). |
| Asymmetric N-Team | Per-team doctrine; AI commander reads its team's objectives. |
| Coop-vs-AI | Hostile AI escalates with player count; uses environment as weapon (gas, fire, breaches). |
| Campaign | Mission-author doctrine override (M7 mission director). |

## Modding Contract

- Add a new mode preset: data row in `content/match_modes/<id>.match_mode.ron` defining default teams, objectives, victory conditions, AI fill defaults.
- Custom team kinds via `Custom(String)` with affordance tags.
- Schema validates via `cargo run -p cf-mod -- validate content/match_modes/`.

## Server Routing

Per [[spec/server-app-architecture]]:

| Server Mode | Match Configs Accepted |
|---|---|
| `coop_room` | Coop-vs-AI; Campaign; Bunker Defence (coop sides only); Symmetric Arena (vs AI); Free-For-All (with AI fill) |
| `pvp_arena` | Bunker Defence (PvP both sides); Symmetric Arena; Free-For-All; Asymmetric N-team |
| `lan_room` | Any |
| `mmo_shard` | Any (but DurationPolicy=Endless usually); per-shard ruleset declares allowed modes |
| `lobby_directory` | Lists open Match instances by mode + map + player count + current state |

Match config is part of the room metadata; clients filter / search / queue by Match properties.

## Comms Policy Integration

Per [[spec/comms-voice-and-radio-model]]:

| Policy | Effect |
|---|---|
| `Realistic` | Full DR-043: voice attenuated by atmosphere/walls; radios with frequency tuning, antenna LOS, interference. Vacuum = no voice. Default for hardcore modes. |
| `ProximityOnly` | Voice (proximity 3D) but no radio simulation. Radios are "magic global team chat" within team. Default for casual modes. |
| `GlobalChat` | All-team always-on chat (training / tutorial mode). |
| `CrossTeamDisabled` | Teams can't hear each other regardless of comms hardware. PvP default. |

## Acceptance Tests (MATCH-A)

| Test | Setup | Pass Condition |
|---|---|---|
| MATCH-A-01 | Load Bunker Defence 1v1 on Mars; both human players. | Match config validates; defender starts with rooted base + sealed atmosphere; attacker starts with dropship + breach kit; both can hear each other only if `comms_policy=Realistic` and within radio range. |
| MATCH-A-02 | Same Match with 1 human defender + 3 AI defenders + 1 human attacker + 3 AI attackers (4v4). | AI fills empty slots; doctrine matches team kind; replay records `match.ai_filled_slot` for each. |
| MATCH-A-03 | 1v1v1v1 Free-For-All on Mimas. | 4 teams of 1; no friendly chatter; victory when last team standing. |
| MATCH-A-04 | 2v1 Asymmetric N-team. | Team-1 starts in dropship LZ; Team-2 starts in dropship LZ at different angle; Team-3 (the lone defender) starts inside bunker with smaller starting force but higher quality gear (per scenario authoring). |
| MATCH-A-05 | Bunker Defence vacuum scenario (Moon). | Defenders in suits; attacker breaches wall; defender suits' oxygen tanks tick down; defender must seal breach or retreat to inner sanctum. EnvironmentSignal hazard `breach_decomp` fires. |
| MATCH-A-06 | Mid-match AI bot reconnects to slot vacated by player. | New AI doctrine continues; replay records `match.ai_filled_slot` mid-game. |
| MATCH-A-07 | Match config with invalid team count (0 teams). | Rejected at server-side load with structured error. |
| MATCH-A-08 | Determinism replay across full Bunker Defence match. | Same seed + same Match + same player inputs = byte-identical event stream. |

## Out Of Scope (during M0..M6.6)

- M0..M5.9: Match grammar exists in scenario manifest as a placeholder; runtime is no-op until M7 lands the Bunker Defence Proof Mission.
- M7 (Mission Director): Bunker Defence Proof Mission ships as the M7 closure; this is THE A-FEEL gate per [[spec/native-implementation-backlog#M7 — Mission Director And Breach Contract]]. Symmetric arena, FFA, asymmetric N-team configurations are stub-only at M7.
- M11 (Online Co-op): full team configs ship (1v1 / 2v2 / 3v3 / FFA / 2v1 etc.); Bunker Defence playable online co-op.
- M12 (Public PvP + MMO): full Bunker Defence PvP launch; community-hostable; MMO shards advertise allowed modes.

## Source Trail

- [[spec/mission-director-slice-a]]
- [[spec/server-app-architecture]]
- [[spec/persistent-mmo-architecture]]
- [[spec/celestial-bodies-and-worlds-model]]
- [[spec/environmental-conditions-model]]
- [[spec/comms-voice-and-radio-model]]
- [[spec/origin-reaction-and-resource-model]]
- [[spec/command-core-base-power]]
- [[spec/chassis-armor-mechs-and-origins]]
- [[references/prototype-run-bundle-schema]]
- [[decisions/dr-005-multiplayer-posture]]
- [[decisions/dr-014-tone-player-promise]]
- [[decisions/dr-015-player-identity-control-posture]]
- [[decisions/dr-017-mission-generation-strategy]]
- [[decisions/dr-022-ai-humanlike-bar]]
- [[decisions/dr-027-combat-base-scope]]
- [[decisions/dr-029-save-game-model]]
- [[decisions/dr-031-content-economy-and-monetization-posture]]
- [[decisions/dr-034-dedicated-server-application]]
- [[decisions/dr-035-persistent-mmo-architecture]]
- [[decisions/dr-042-game-modes-and-match-grammar-direction]]
- [[research-log/2026-05-06-celestial-bodies-environments-mining-bunker-defence-design-intent]]

## Change Log

- 2026-05-06: Captured during M1 from user-supplied design intent ("there will be defence of the bunker mode... a game mode where there is a team of attackers and a team of defenders... 1v1v1v1 or 2v2 or 3v3 or 1v1 or 2v1 basically any combo"). Status: `design-intent-post-m1`. Bunker Defence locked as flagship mode; full Match grammar covers symmetric / asymmetric / FFA / coop-vs-AI / campaign. Lands at M7 (proof mission), M11 (full team configs in online co-op), M12 (community-hostable PvP launch).
