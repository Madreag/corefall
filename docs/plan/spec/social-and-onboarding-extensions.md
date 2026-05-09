---
type: spec
status: closed-direction
authority: "Social features (guilds, in-game messaging, co-op campaign saves, voice party, gifting, mission-share invite, cross-shard friends, guild-managed bunker designs) + new-player onboarding plus (mentor system, beginner matchmaking pool, first-30-min telemetry deep-dive, adaptive difficulty per session, demo→full carry-over, tip-of-the-day, adaptive hints, first-time guide PDF, narrative hook in onboarding mission, locale-aware hint pacing) + AI quality extensions (difficulty visibility, faction personality, mistake narration, AI training mode for modders, AI-vs-AI tournaments, transparency mode, play-as-Husk)."
ready_when: "Guilds functional + in-game messaging + co-op campaign saves persist; mentor matching + beginner pool + first-30-min telemetry; AI difficulty visible + faction personality identifiable + AI training mode."
feeds:
  - DR-005
  - DR-008
  - DR-022
  - DR-023
  - DR-024
  - DR-031
  - DR-046
  - DR-047
  - DR-050
---

← [[spec/index|spec section]] · [[decisions/dr-050-modding-social-onboarding-and-ai-extensions|DR-050]] · [[spec/tutorial-implementation|tutorial]]

# Social, Onboarding & AI Quality Extensions

## Social Features

### Guild / clan system

| Aspect | Detail |
|---|---|
| Size | 8-50 players (per-guild config). |
| Shared base designs | Guild bunker designs shared. |
| Clan-vs-clan PvP | Optional faction wars per [[spec/server-wide-events-and-meta-narrative]]. |
| Profile + emblem + roster | Per-guild profile page; member roster; activity stats. |
| Roles | Officer / member / probationary; permission tiers. |
| Optional | Per DR-015 solo-first; guild membership optional. |

### In-game messaging beyond match lobby

| Aspect | Detail |
|---|---|
| Offline DM | Friends list with chat. |
| Channels | Per-friend-group; mod-discussion. |
| Discord federation | Optional; Discord-bridge per-guild. |
| Privacy | Block list; private profile; report system. |

### Co-op campaign saves

Bring friends through campaign together.

| Aspect | Detail |
|---|---|
| Party size | Up to 4 players. |
| Persistence | Party-state persists across sessions. |
| Per-player progress | Each player has own progress; party progress tracks combined. |
| Host migration | If host disconnects, party can migrate. |

### Cross-shard friends list

MMO mode: find friends across persistent shards; friend status visible across shards.

### Voice party

Discord-style party voice independent of in-match voice; pre-match strategy. Steam/EOS adapter.

### Player-to-player gifting

| Type | Detail |
|---|---|
| Gift game copy | Steam handles. |
| Gift cosmetics | In-game cosmetic transfer. |
| Tip jar (modder + creator) | Per [[spec/modding-ecosystem-extensions]]. |
| NEVER cash to player | Per DR-031. |
| NEVER marketplace cut | Per DR-031. |

### Mission-share invite

Steam-friend pop-up: "come join my Bunker Defence run"; one-button join.

### Guild-managed bunker designs

Community-authored bases for guild defense missions; voted by members.

### Cross-Workshop coordination

Modder collab tools; inter-mod dependencies; collaborative mod packages.

## New-Player Onboarding Plus

### Mentor system

| Aspect | Detail |
|---|---|
| Veteran threshold | Mastery 20+ across multiple chassis OR completed campaign. |
| Mentor opt-in | Veteran enables mentor mode in profile. |
| Auto-match | New players (first 5 hours) auto-matched. |
| Mentor sees mentee's playtime | Reduced playtime visibility (anti-stalker). |
| Choose to invite | Mentor invites mentee to mission/training session. |
| Mutual reward | Cosmetic emblem + leaderboard recognition. |

### Beginner matchmaking pool

Beginner-only games for first N hours (default 10); separate matchmaking queue.

### First-30-minutes telemetry deep-dive

| Metric | Detail |
|---|---|
| Per-second drop-off detection | Where exactly do new players quit? |
| Per-action timing | How long do they spend on each tutorial beat? |
| Session abandonment cause | Did they crash? Get stuck? Get bored? |
| AI agent generates weekly report | Anomaly detection + recommendations for tutorial polish. |

### Adaptive difficulty per session

Auto-adjust if struggling (more hints, lower enemy aggression, slower time scale). Opt-in.

### Demo → full game carry-over

Achievements, saves, cosmetics carry over per DR-047.

### Tip-of-the-day on launch screen

| Aspect | Detail |
|---|---|
| 50+ tips | Initial roster; AI-rotated. |
| Modder-extensible | Per [[spec/modding-ecosystem-extensions]]. |
| Per-locale | AI-translated. |

### Adaptive hints

Per [[spec/tutorial-implementation]] base + per-session learning rate detection.

### First-time player guide PDF

Auto-generated from tooltip + tutorial data; downloadable from Steam page; per-language.

### Onboarding mission narrative hook

"First Contract" mission: explicit "why am I doing this" hook. Tested in playtest.

### Cultural / locale-aware hint pacing

Hint frequency adapts to per-locale norms (Asian mobile-style ≠ Western indie). AI agent applies locale-pacing-profile per `EnvironmentSignal.player_locale`.

## AI Quality Extensions (Per DR-008 + DR-022)

### AI difficulty visibility

Named AI presets visible to player.

| Preset | Aggression | Resource usage | Tactical depth |
|---|---|---|---|
| Cakewalk | low | abundant | minimal |
| Tough Crowd | medium-low | adequate | basic |
| Veteran | medium | adequate | moderate |
| Nightmare | high | strict | advanced |
| Demonic | very high | strict | maximum + LLM mind augmentation |

### Faction AI personality identifiability

At hour 5, player should be able to tell "that's Browncoat doctrine" from behavior alone.

| Faction | AI tells |
|---|---|
| Trade Star | Calculated; clean energy; rare risk-taking. |
| Coalition | Pincer maneuvers; medic + engineer support; decisive. |
| Browncoats | Frontline grinder; never-retreat; super-soldier discipline. |
| Ronin | Stealth; duelist; signature kills. |
| Tek-Mart | Improvised; modular; chaotic. |
| Imperatus | Hierarchical; legion formations; autocrat. |
| Free Hold | Asymmetric defense; bunker-first. |
| Husks | Swarming; biotoxin; relentless. |

Per-faction style flag in `cf-ai`.

### AI mistake narration

Debrief: "the enemy commander made a tactical blunder when X" with replay-scrub link. Drives narrative payoff.

### AI training mode for modders

Modder runs scenarios against AI; AI learns + adapts within strict bounds; submits doctrine tweaks.

### AI-vs-AI tournament mode

Community submits AI doctrines; tournaments run server-side; results cached; community votes; replay broadcasts. Per [[spec/endgame-modes-and-retention-loops]] `mode_bot_tournament`.

### AI transparency mode

Show AI reason labels live in HUD (opt-in setting); per DR-022 humanlike bar.

### Play-as-Husk mode

Player controls antagonist faction in custom scenarios; PvE-vs-AI variant; reverses normal asymmetry.

### AI personality voice variety per origin

Humans = different voice families (regional accents); robots = synth varieties. AI-generated via ElevenLabs / XTTS.

### AI bot-loadout-hostility

AI bots can refuse to use specific equipment per DR-008 + DR-022 (refusal reasons surfaced); modders can add new refusal predicates.

## Done-Criteria

- [ ] Guilds functional + per-guild profile.
- [ ] In-game messaging + DM + channels.
- [ ] Co-op campaign saves persist across sessions.
- [ ] Cross-shard friends list.
- [ ] Voice party functional.
- [ ] Mentor matching + beginner pool active.
- [ ] First-30-min telemetry deep-dive runs nightly.
- [ ] Adaptive difficulty per session.
- [ ] Demo → full game carry-over verified.
- [ ] Tip-of-the-day rotates.
- [ ] AI difficulty presets visible + named.
- [ ] Faction AI personality identifiable in playtest.
- [ ] AI mistake narration in debrief.
- [ ] AI training mode for modders functional.
- [ ] AI-vs-AI tournament mode runs.
- [ ] AI transparency mode togglable.
- [ ] Play-as-Husk mode functional.

## Source Trail

- [[decisions/dr-050-modding-social-onboarding-and-ai-extensions]]
- [[decisions/dr-008-ai-architecture]]
- [[decisions/dr-022-ai-humanlike-bar]]
- [[decisions/dr-023-tutorial-and-onboarding-strategy]]
- [[spec/tutorial-implementation]]
- Helldivers 2 mentor pattern: 60%+ first-month retention.
- Souls / Elden Ring AI difficulty: named presets + intentional-feeling difficulty.
- Path of Exile mod ecosystem: voluntary collaboration.
