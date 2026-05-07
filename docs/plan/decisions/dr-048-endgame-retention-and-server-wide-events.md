---
type: decision
id: DR-048
status: closed-direction
priority: P0
closed_at: 2026-05-06
revisit_trigger: "100hr+ retention drops below threshold; or async PvP fails to attract daily-active players; or server-wide events overwhelm community-hosted shard capacity; or roguelite mode cannibalizes campaign engagement."
---

← [[decisions/index|decision records]] · [[dashboards/decision-tracker|decision tracker]] · [[spec/endgame-modes-and-retention-loops|endgame modes spec]] · [[spec/server-wide-events-and-meta-narrative|world events spec]] · [[decisions/dr-011-progression-retention-loop|DR-011]] · [[decisions/dr-031-content-economy-and-monetization-posture|DR-031]] · [[decisions/dr-047-launch-and-live-operations|DR-047]]

# DR-048: Endgame, Retention Loops & Server-Wide Events

> [!success] Status: CLOSED-DIRECTION (project owner committed 2026-05-06)
> Locks the deep retention layer: roguelite + Last Stand + endless wave modes + asynchronous PvP via ghost replays + time attack + persistent veterans deep loop + cross-shard world events + meta-narrative + community map jams + custom rule sets + bot-vs-bot tournaments. Solves the "what makes players come back at hour 100" gap from second-pass audit. NEVER pay-to-win, NEVER FOMO-mechanics-required (per DR-031).

## Decision

### Endgame modes (separate from campaign)

| Mode | Detail |
|---|---|
| **Roguelite mode (`mode_roguelite`)** | Permadeath operative pool; draft-pick faction at start; escalating per-mission difficulty; meta-progression unlocks new starting kits + named-NPC variants; runs 30-90 min; "death is content" loop per Hades/FTL precedent. |
| **Last Stand mode (`mode_last_stand`)** | Endless escalating waves at a fixed bunker. Per-wave intensity ramp + new-faction reveal. Co-op 1-4. Leaderboard per-faction-defended. Score = waves survived × multiplier. |
| **Endless mode (`mode_endless`)** | Procedural mission chain; each mission ramps stakes + introduces new faction comp; player can extract or push deeper for better loot. Cousin to roguelite but persistent loadout. |
| **Time-attack mode (`mode_time_attack`)** | Per-mission speedrun bracket; verified replays via cf-replay determinism; speedrun.com integration per DR-047. |
| **New Game Plus (`mode_ngp`)** | Campaign restart with carryover veterans + harder enemy comp + new starting factions unlocked + new mission variants. Loops infinitely with per-completion modifier stacking. |
| **Async PvP / Ghost replays (`mode_async_pvp`)** | Your bot-controlled bunker defenders battle other players' attacker squads asynchronously. Daily challenge-replay vs other players' best runs. Lower friction than live PvP. |
| **Custom rule sets (`mode_custom_rules`)** | Player-authored "house rules" via mod scenarios: damage scaling, time scale, no-heal, weapon restrictions, etc. Share to Workshop. |
| **Bot-vs-bot tournaments (`mode_bot_tournament`)** | Modders submit AI doctrines; tournaments run server-side; results cached; community votes; replay broadcasts. |
| **Community map jams (`mode_community_jam`)** | 48-72hr modder events; theme + constraint; community judges via replay vote; featured in launcher. |
| **Daily seed leaderboard (`mode_daily`)** | Same seed for all players that day. Leaderboard. Replay share. |

### Server-wide events + meta-narrative

| Event Type | Detail |
|---|---|
| **Cross-shard world events** | Limited-time anomaly bursts; community-wide objectives; cosmetic-only rewards per DR-031 (NEVER FOMO power). |
| **Player-driven faction wars** | On persistent MMO shards: per-faction territory control; community elects commanders; faction defeats persist. |
| **Anomaly outbreak events** | Cross-faction shared threat; cooperative; ends per server. |
| **Time-limited campaign chapters** | Anti-FOMO via post-event archive availability. Players who miss can play it later from the "archive missions" menu. |
| **Community lore voting** | Reddit/Discord polls determine in-game outcomes; influences next chapter. |
| **Live dev events** | Project-owner runs an MMO shard for an hour; players join. |
| **Twitch-driven crowd missions** | Chat votes objective. |
| **'World saved' / 'world fallen' state changes** | Reflected in next episode of narrative. |
| **Pre-launch ARG (alternate reality game)** | Discord-driven, Reddit puzzles, in-world clues, narrative-extension; runs ~3-6mo pre-launch. |

### Persistent veterans deep loop

Per DR-018 + DR-011, extended:
- **Veteran roster persists across runs**: surviving operatives become "named" + memorialized in codex + appear in subsequent missions + get unique dialogue + progressive scarring/equipment-wear + voice-line evolution.
- **Veteran death has weight**: named NPC death triggers comic-panel obituary + faction reaction + memorial in base.
- **Veteran skill drift**: per-100-mission survival, veteran develops quirks/specialties (sniper-veteran prefers long range; medic-veteran has higher heal rate; based on observed combat data).
- **Cross-mode veterans**: a veteran in campaign carries over to roguelite (with skill drift) + can be drafted in tournament mode.
- **Veteran roster cap**: 20 active + 50 retired (visible in codex). Retired veterans are NOT lost, just stored.

### Bunker building meta

Per DR-027, extended:
- **Bunker designs persist across runs**: player carries over bunker designs across modes.
- **Veteran bunkers in Hall of Fame**: notable defenses (long-survival, dramatic-rescue) get named entries.
- **Bunker sharing**: export bunker design as RON file; share to Workshop; subscribe to others' designs.
- **Bunker-vs-bunker async**: per the async PvP mode, your bunker auto-defends against other players' attackers.

## What This Locks In

| Spec Area | Implication |
|---|---|
| `cf-mission` | Match grammar extends with `mode_roguelite`, `mode_last_stand`, `mode_endless`, `mode_time_attack`, `mode_ngp`, `mode_async_pvp`, `mode_custom_rules`, `mode_bot_tournament`, `mode_community_jam`, `mode_daily`. |
| `cf-veteran` | New crate (or extension of `cf-mission`) for persistent veteran roster + skill-drift kernel + cross-mode tracking. |
| `cf-server` | Async PvP bunker storage + ghost-replay match queue + cross-shard world event broadcaster. |
| `cf-replay` | Ghost-replay format: serialized AI inputs + bunker state + replay event chain; deterministic re-run for async match. |
| `cf-mod` | Custom rule sets + community jam mods + bot-tournament submissions are first-class mod packages. |
| Meta-narrative | New `cf-meta-narrative` crate or extension of `cf-mission`; tracks community-vote state + world-event progression. |

## What This Does NOT Lock

| Non-Commitment | Why |
|---|---|
| Specific roguelite balance | Open until M-CONTENT-ENDGAME playtest. |
| Number of Last Stand wave types | Open. Target ~30 unique wave compositions launch-tier. |
| Async PvP matchmaking algorithm | Open. Default: random; future: ELO-weighted (per DR-049 ranked). |
| Cross-shard event cadence | Open. Default: 1 minor event/week; 1 major event/month. |
| ARG specific puzzles | Open. Author closer to launch. |

## Why This Direction

| Driver | Detail |
|---|---|
| 100hr+ retention | Without endgame modes, players exhaust campaign and leave. Roguelite + Last Stand + endless = proven retention loops (Hades, Vampire Survivors, FTL, Slay the Spire). |
| Community talking points | World events + meta-narrative = something to discuss on Discord/Reddit/Twitter daily. |
| Lower-friction multiplayer | Async PvP via ghost replays = daily-active engagement without scheduling friction. |
| Modder retention | Custom rule sets + community jams + bot tournaments = modder ecosystem alive long-term. |
| Veteran emotional investment | Per DR-018 named-veteran death is emotionally heavy. Persistent loop deepens that connection across 100+ hours. |
| Anti-FOMO compliance | Per DR-031, all events have post-event archive; no "miss it forever" mechanics. |

## Why Not The Alternatives

| Alternative | Why Rejected |
|---|---|
| Campaign-only endgame | Burns out at ~40-60 hours; loses long-tail. |
| Hard-mode campaign re-skin | Lazy; doesn't add new loops. |
| Procedural campaign generator only | Already in plan (DR-017 + procedural contracts); doesn't replace mode variety. |
| Time-limited content with hard FOMO | Forbidden by DR-031. Anti-FOMO archive is the substitute. |
| Live-service treadmill (battle pass / season XP grind) | Forbidden by DR-031. |

## Evidence Trail

- Project owner verbatim (2026-05-06): "what would keep players from playing the game after going through the entire roadmap? what could be better? what is missing? where are the gaps?"
- Hades retention: 30+ hr median play time per Steam; roguelite design pattern.
- Vampire Survivors retention: 50+ hr median play time per Steam; endless wave + meta-progression pattern.
- FTL retention: 40+ hr median play time per Steam; permadeath roguelite.
- Slay the Spire retention: 80+ hr median play time per Steam; daily climb / replay pattern.
- Captured in [[research-log/2026-05-06-second-pass-audit-followup]] (TBD).

## Revisit Trigger

- 100hr+ retention drops below 20% of player base in playtest cohort.
- Async PvP fails to attract daily-active players.
- Server-wide events overwhelm community-hosted shard capacity.
- Roguelite mode cannibalizes campaign engagement.
- Veteran roster cap proves too small / large.
