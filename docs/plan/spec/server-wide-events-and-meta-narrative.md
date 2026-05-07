---
type: spec
status: closed-direction
authority: "Server-wide events: cross-shard world events, faction wars, anomaly outbreaks, time-limited campaign chapters (anti-FOMO archive), community lore voting, live dev events, Twitch-driven crowd missions, world state changes. Pre-launch ARG."
ready_when: "Cross-shard event broadcaster live; per-shard event scheduler; community vote integration; anti-FOMO archive; ARG infrastructure pre-launch."
feeds:
  - DR-005
  - DR-013
  - DR-014
  - DR-016
  - DR-017
  - DR-022
  - DR-031
  - DR-035
  - DR-042
  - DR-046
  - DR-047
  - DR-048
---

← [[spec/index|spec section]] · [[decisions/dr-048-endgame-retention-and-server-wide-events|DR-048]] · [[spec/endgame-modes-and-retention-loops|endgame modes]] · [[spec/persistent-mmo-architecture|MMO architecture]]

# Server-Wide Events & Meta-Narrative

> [!summary] What this page is
> Cross-shard world events + meta-narrative system. Solves "community talking point" gap from second-pass audit. Players have something to discuss daily on Discord/Reddit/Twitter. Anti-FOMO compliant per DR-031: every event has post-event archive. Pre-launch ARG drives marketing hype.

## Event Types

### Cross-shard world events

Limited-time anomaly bursts; community-wide objectives; cosmetic-only rewards per DR-031.

| Event | Detail |
|---|---|
| **Anomaly outbreak** | Cross-faction shared threat; cooperative; ends per server (when objective met or time expires). |
| **Solar storm** | Disrupts radio across Earth-orbit shards (per DR-043); cross-shard tactical implication. |
| **Faction conquest event** | Faction expansion attempt across shards; defenders cooperate to repel. |
| **Black market run** | Limited-window in-world vendor with rare equipment; archived/earn-back equivalent remains available after the event. Never paid power. |
| **Ghost ship contact** | Mystery dropship arrives; players investigate; lore unlock. |
| **Community challenge week** | Weekly leaderboard challenges; speedrun + survival + bunker design. |

### Player-driven faction wars

On persistent MMO shards: per-faction territory control; community elects commanders; faction defeats persist. Per DR-035.

### Time-limited campaign chapters

Anti-FOMO via post-event archive availability. Players who miss can play it later from "archive missions" menu.

| Aspect | Detail |
|---|---|
| **Live phase** | 4-8 weeks; community-wide narrative + missions. |
| **Archive phase** | Available forever via "archive missions" menu. |
| **Reward gating** | Cosmetic emblems for live-phase players; cosmetic + lore for archive players. NEVER FOMO power. |

### Community lore voting

Reddit/Discord polls determine in-game outcomes; influences next chapter.

| Aspect | Detail |
|---|---|
| **Voting platform** | Discord poll bot + Reddit thread cross-link; AI-aggregated. |
| **Outcome integration** | Vote result modifies subsequent campaign manifest; replay-deterministic per shard. |
| **Cadence** | Quarterly major votes; monthly minor. |

### Live dev events

Project-owner runs an MMO shard for an hour; players join.

### Twitch-driven crowd missions

Chat votes objective during streamer's match.

### World state changes

'World saved' / 'world fallen' state changes reflected in next episode of narrative.

### Pre-launch ARG (Alternate Reality Game)

Discord-driven, Reddit puzzles, in-world clues, narrative-extension. Runs ~3-6mo pre-launch.

| Aspect | Detail |
|---|---|
| **Hooks** | Mysterious "Patient Zero" social-media account; Reddit cipher puzzles; in-world data leaks via project-owner Twitter; faction "press releases"; QR codes in trailers. |
| **Resolution** | ARG culminates in launch trailer reveal; participants get early access + commemorative emblem. |
| **Goal** | Drive 5K-10K wishlist signal pre-Steam-page launch via narrative engagement. |

## Tech Architecture

### Cross-shard event broadcaster

`cf-server-event-broadcaster` (new crate or extension of `cf-server`):
- Centralized event-state authority per project-owner-managed coordinator service (community-hostable; optional).
- Per-shard event scheduler reads + applies event-state.
- Event manifest published via signed JSON; community shards verify + apply.
- Anti-FOMO: every event has post-event archive in `cf-archive`.

### Community vote integration

`cf-community-vote` (new crate):
- Discord bot reads polls.
- Reddit thread scraper aggregates votes (anti-bot heuristics).
- AI agent normalizes votes; produces vote-result manifest.
- Vote-result manifest applied to subsequent campaign manifest.

### Live dev event runner

Streamer mode (per DR-047) + spectator broadcast.

### ARG infrastructure (pre-launch)

`cf-arg-engine` (new tool):
- Puzzle generator (AI-authored cipher puzzles).
- Discord bot + Reddit thread cross-poster.
- Commemorative emblem unlock for participants.

## Anti-FOMO Compliance

Per DR-031:

| Rule | Enforcement |
|---|---|
| Every event has post-event archive | `cf-archive` validates pre-launch |
| No power-only event rewards | All event rewards are cosmetic-only |
| No exclusive content for live participants | Live participants get emblem + early access; archive participants get same lore + cosmetic + leaderboard |
| Event participation is opt-in | Shard operators can disable events per shard |

## Done-Criteria

- [ ] Cross-shard event broadcaster live.
- [ ] Per-shard event scheduler functional.
- [ ] Community vote integration (Discord + Reddit).
- [ ] Live dev event broadcasts work.
- [ ] Twitch-driven crowd mission integration.
- [ ] World state changes propagate to subsequent missions.
- [ ] Anti-FOMO archive validates.
- [ ] Pre-launch ARG infrastructure ready.

## Source Trail

- [[decisions/dr-048-endgame-retention-and-server-wide-events]]
- [[decisions/dr-031-content-economy-and-monetization-posture]]
- [[decisions/dr-035-persistent-mmo-architecture]]
- [[spec/persistent-mmo-architecture]]
- Halo 3 ARG (pre-launch): "I Love Bees" — pre-launch narrative engagement precedent.
- Cyberpunk 2077 launch ARG: precedent.
- Final Fantasy XIV cross-shard events: persistent MMO event precedent.
