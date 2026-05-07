---
type: spec
status: closed-direction
authority: "Live ops + endgame: cosmetics + DLC + balance hot-patch + seasonal events + community challenges + procedural contracts + persistent veterans + mastery + Bunker Defence meta + ranked PvP + MMO + Workshop endless content + speedrun + daily seeds. NEVER pay-to-win."
ready_when: "Cosmetics pipeline live; DLC infrastructure ready; balance hot-patch deploy works; procedural contract generator active; mastery progression functional; replay-archive verified speedrun integrated."
feeds:
  - DR-005
  - DR-011
  - DR-017
  - DR-018
  - DR-024
  - DR-027
  - DR-031
  - DR-035
  - DR-042
  - DR-045
  - DR-046
  - DR-047
---

← [[spec/index|spec section]] · [[decisions/dr-047-launch-and-live-operations|DR-047]] · [[decisions/dr-031-content-economy-and-monetization-posture|DR-031]]

# Live Ops & Endgame

> [!important] Foundation only at launch
> Per DR-031: NO pay-to-win, NO gacha, NO marketplace cut. Live ops at launch is **infrastructure only**, not content-economy treadmill. Cosmetics are EARNED via play, NEVER paid.

## Cosmetics Pipeline

| Type | Earned via | Notes |
|---|---|---|
| Skins (chassis recolor) | Mastery rank, achievement, mission completion | Per-faction, per-chassis. ~50+ at launch. |
| Decals (chassis) | Mastery, replay shares, Bunker-Defence wins | Player can apply to chassis in workbench. |
| Paint jobs | Mastery; combat performance | Custom palette tools. |
| Voice packs | Per-faction completion | Different commander voice for player's commander. |
| Emblems / faction patches | Mission objectives + lore unlocks | Visible on chassis + briefings. |
| Victory poses | Match victory streaks | Animated end-of-match poses. |
| Replay highlight templates | Mastery + sharing | For streamer overlay templates. |

NEVER paid. NEVER gacha. NEVER FOMO.

## DLC Infrastructure

Ready at launch but no DLC ships v1.0. Post-launch evaluation:

- New campaign chapters (paid; OPTIONAL; never gates core).
- New factions (paid; visual + lore + signature gear).
- New worlds (paid; visual + biome + atmospheric).
- New cosmetic packs (paid).

Per DR-031: NEVER pays-for-power. NEVER pay-to-win.

## Balance Hot-Patch

- Signed content patch via Steam.
- ~Quarterly post-launch.
- Driven by telemetry + community feedback.
- AI agent generates patch-note draft from telemetry-driven balance candidates.

## Seasonal Events (Optional Post-Launch)

- Halloween scenario (October).
- Winter scenario (December).
- Anniversary scenario (launch+1yr).
- Time-limited but NO FOMO mechanics (ephemeral cosmetics OK if also obtainable later).

## Community Challenges

- Weekly leaderboard challenges: speedrun a mission, survive Bunker Defence wave, build a base.
- Replay-share-driven verification.
- AI-judge-assisted (replay deterministic verification + anti-cheat).
- Public leaderboards.

## Mod-Creator Support

- Featured mods (project-owner curated).
- Mod-spotlight in launcher.
- Modder credits on official channels.
- Discord mod-creator role.
- Per-modder revenue: 0% (per DR-031, no marketplace cut). Modders publish freely.

## Endgame / Replayability

### Procedural contracts

Per DR-017. Post-campaign players have endless procedural mission generator. AI-driven mission director assembles seed + objectives + faction + world + weather + comms policy. Each contract is unique.

### Persistent veterans

Survive enough missions, your operative becomes "named." Memorialized in codex. Per DR-018 + DR-011.

### Mastery progression

Per-chassis / per-faction / per-weapon mastery rank (1-30). Unlocks variants, paint, voice lines, lore entries. Intrinsic, no power.

### Bunker building meta

Per DR-027. Players carry over bunker designs across runs. Veteran bunkers in Hall of Fame.

### Ranked PvP (post-launch)

Per DR-005. Ranked PvP arenas post-launch with seasonal resets. NEVER pay-to-rank.

### Persistent MMO shards

Per DR-035. Long-running shards for community-hosted persistent play.

### Speedrun.com integration

Replay-archive verified speedruns. Anti-cheat foundation supports.

### Daily/weekly mission seeds

Same seed for all players that day. Leaderboard. Replay share.

### Steam Workshop endless content

Modder-published missions, factions, chassis, scenarios. Curated by community + featured by official.

## Done-Criteria

- [ ] Cosmetics pipeline live + earnable.
- [ ] DLC infrastructure ready (build flag).
- [ ] Balance hot-patch deploy works.
- [ ] Procedural contract generator active.
- [ ] Mastery progression functional.
- [ ] Speedrun.com replay verification works.
- [ ] Daily seed leaderboard live.
- [ ] Workshop endless content discoverable in launcher.
- [ ] Anti-FOMO + anti-pay-to-win audit passes.

## Source Trail

- [[decisions/dr-031-content-economy-and-monetization-posture]]
- [[decisions/dr-047-launch-and-live-operations]]
- [[decisions/dr-011-progression-retention-loop]]
