---
type: spec
status: closed-direction
authority: "Endgame modes: roguelite + Last Stand + endless + time attack + NG+ + async PvP + custom rule sets + bot-vs-bot tournaments + community map jams + daily seeds. Persistent veterans deep loop. Bunker building meta. NEVER pay-to-win. NEVER FOMO."
ready_when: "Each endgame mode plays end-to-end + mode lobby + leaderboards + ghost-replay async PvP queue + persistent veteran roster + bunker meta carries across modes."
feeds:
  - DR-005
  - DR-011
  - DR-017
  - DR-018
  - DR-027
  - DR-031
  - DR-035
  - DR-042
  - DR-045
  - DR-047
  - DR-048
---

← [[spec/index|spec section]] · [[decisions/dr-048-endgame-retention-and-server-wide-events|DR-048]] · [[spec/game-modes-and-match-grammar|game modes]] · [[spec/server-wide-events-and-meta-narrative|world events]]

# Endgame Modes & Retention Loops

> [!summary] What this page is
> Solves the "what makes players come back at hour 100" gap. 10 distinct endgame modes layered atop the campaign + match grammar. Persistent veterans deepen across modes. Bunker designs persist. Asynchronous PvP via ghost replays gives daily-active engagement without scheduling friction.

## Modes

### `mode_roguelite` — Roguelite

Permadeath operative pool; draft-pick faction at start; escalating per-mission difficulty; meta-progression unlocks new starting kits + named-NPC variants; runs 30-90 min. Death is content (Hades / FTL / Slay the Spire pattern adapted to tactical sandbox).

- Per-run: pick 1 faction + 1 commander.
- Operative pool of 5; permadeath; can recruit 1 between missions.
- 6-12 missions per run; difficulty scales.
- Meta-progression: per-completed-run unlocks new starting kits + variants + named-NPC pool extension.
- Cosmetic-only meta rewards per DR-031.

### `mode_last_stand` — Last Stand

Endless escalating waves at a fixed bunker. Per-wave intensity ramp + new-faction reveal. Co-op 1-4. Leaderboard per faction defended.

- Each wave: enemy comp scales; new faction joins assault.
- Bunker repairs available between waves.
- Score = waves survived × multiplier per faction.
- Leaderboard per faction.

### `mode_endless` — Endless Mission Chain

Procedural mission chain; each mission ramps stakes + introduces new faction comp; player can extract or push deeper for better loot. Cousin to roguelite but persistent loadout.

### `mode_time_attack` — Time Attack

Per-mission speedrun bracket; verified replays via cf-replay determinism; speedrun.com integration per DR-047.

- Per-mission target time (default + community-set).
- Verified speedrun submissions to speedrun.com.
- Daily/weekly speedrun rotation.

### `mode_ngp` — New Game Plus

Campaign restart with carryover veterans + harder enemy comp + new starting factions unlocked + new mission variants. Loops infinitely with per-completion modifier stacking.

### `mode_async_pvp` — Asynchronous PvP / Ghost Replays

Your bot-controlled bunker defenders battle other players' attacker squads asynchronously. Daily challenge-replay vs other players' best runs.

- Player-A attacks Player-B's bunker (Player-B not online).
- Ghost-replay format: serialized bunker state + AI inputs + deterministic replay.
- Daily challenge: top-N attacker scores per opponent's bunker design.
- Cross-player ghost-replay match queue managed server-side.

### `mode_custom_rules` — Custom Rule Sets

Player-authored "house rules" via mod scenarios: damage scaling, time scale, no-heal, weapon restrictions, etc. Share to Workshop.

### `mode_bot_tournament` — Bot-vs-Bot Tournaments

Modders submit AI doctrines; tournaments run server-side; results cached; community votes; replay broadcasts.

- Per-tournament: faction selection + map + rules.
- Server-hosted match runner.
- Replay broadcast available to community.
- Modder credits + emblem reward.

### `mode_community_jam` — Community Map Jams

48-72hr modder events; theme + constraint; community judges via replay vote; featured in launcher.

### `mode_daily` — Daily Seed Leaderboard

Same seed for all players that day. Leaderboard. Replay share.

## Persistent Veterans Deep Loop

Per DR-018 + DR-011, extended:

| Aspect | Detail |
|---|---|
| **Persistence** | Surviving operatives become "named" + memorialized in codex + appear in subsequent missions. |
| **Death has weight** | Named NPC death triggers comic-panel obituary + faction reaction + memorial in base. |
| **Skill drift** | Per-100-mission survival, veteran develops quirks/specialties (sniper-veteran prefers long range; medic-veteran has higher heal rate; based on observed combat data). |
| **Cross-mode** | A veteran in campaign carries over to roguelite (with skill drift) + can be drafted in tournament mode. |
| **Roster cap** | 20 active + 50 retired (visible in codex). Retired veterans stored, not lost. |
| **Voice line evolution** | Per veteran-experience, voice-lines evolve (more confident, more cynical, etc.). Generated via AI per [[spec/music-and-soundtrack]]. |
| **Equipment-wear visualization** | Veteran sprites accrue wear (scratches, blood, faction-recolor patches). |

## Bunker Building Meta

Per DR-027, extended:

- **Bunker designs persist across runs** (carries across modes).
- **Veteran bunkers in Hall of Fame** (notable defenses get named entries).
- **Bunker sharing**: export RON; share to Workshop; subscribe to others' designs.
- **Bunker-vs-bunker async**: per `mode_async_pvp`, your bunker auto-defends.

## Done-Criteria

- [ ] Each of 10 modes plays end-to-end.
- [ ] Mode lobby screen functional.
- [ ] Leaderboards per mode.
- [ ] Ghost-replay async PvP queue functional.
- [ ] Persistent veteran roster persists across modes.
- [ ] Bunker meta carries across modes.
- [ ] Anti-FOMO compliance audit passes.
- [ ] Modder-submitted bot tournaments run server-side.

## Source Trail

- [[decisions/dr-048-endgame-retention-and-server-wide-events]]
- [[decisions/dr-031-content-economy-and-monetization-posture]]
- [[decisions/dr-018-death-meaning-and-consequence-ladder]]
- [[decisions/dr-011-progression-retention-loop]]
- Hades roguelite: https://www.supergiantgames.com/games/hades/
- FTL: https://subsetgames.com/ftl.html
- Slay the Spire: https://www.megacrit.com/slay-the-spire/
