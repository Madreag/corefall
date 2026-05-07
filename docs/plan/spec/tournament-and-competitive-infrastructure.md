---
type: spec
status: closed-direction
authority: "Tournament & competitive infrastructure: ELO/MMR + ranked PvP brackets + tournament bracket admin tools + observer/commentator tools + replay analysis (heatmap + decision tree + counterfactual) + match-history aggregate + anti-rage-quit + pre-match warm-up + coach mode + lobby spectator + tournament-grade anti-cheat. Community-hostable per DR-005."
ready_when: "ELO/MMR functional + per-mode brackets visible + tournament admin can host + observer cam + commentator overlay + replay analysis tools + coach mode + warm-up area; tournament-grade anti-cheat profile signed off."
feeds:
  - DR-002
  - DR-005
  - DR-022
  - DR-024
  - DR-031
  - DR-034
  - DR-042
  - DR-046
  - DR-047
  - DR-049
---

← [[spec/index|spec section]] · [[decisions/dr-049-customization-tournament-and-competitive|DR-049]] · [[spec/server-app-architecture|server app]] · [[spec/streaming-and-creator-features|streaming]]

# Tournament & Competitive Infrastructure

## ELO/MMR System

| Aspect | Detail |
|---|---|
| Algorithm | Glicko-2 (default) or TrueSkill (alternative). |
| Per-mode | Separate ELO per match mode (Bunker Defence ≠ Symmetric Arena ≠ FFA). |
| Per-faction sub-rating | Optional per-faction sub-rating; players have separate ratings per faction played. |
| Decay | Inactivity decay; -10 ELO per week not played (after 4-week grace). |
| Visibility | Visible to player; tier badges (Bronze / Silver / Gold / Platinum / Diamond / Master / Champion). |
| Anti-cheat | Per-match anti-cheat hash; ELO updates only on validated matches. |

## Ranked PvP Brackets

7 tiers: Bronze, Silver, Gold, Platinum, Diamond, Master, Champion. Per-season reset (cosmetic-only rewards per DR-031).

| Aspect | Detail |
|---|---|
| Season length | 12 weeks default. |
| Promotion / demotion | At ELO thresholds; promotion match optional. |
| Reset | Per-season soft reset (e.g., back to Silver baseline; placement matches re-determine). |
| Rewards | Cosmetic emblem + paint per tier achieved. NEVER paid. NEVER power. |

## Tournament Bracket Infrastructure

| Tournament Type | Detail |
|---|---|
| Invite-bracket | 8/16/32/64 players; project-owner or community admin. |
| Single-elimination | Standard. |
| Double-elimination | With losers bracket. |
| Swiss | All-play-all-X-rounds; for larger fields. |
| Round-robin | For small groups. |
| Per-tournament cf-server instance | Dedicated server. |

### Admin tools
- Player roster + check-in.
- Match-pairing + auto-bracket generation.
- Match-result entry (auto from cf-server).
- Disqualification + adjustment.
- Tournament metadata: name, sponsor, prize, rules.

## Observer Cam / Commentator Tools

| Component | Detail |
|---|---|
| Switchable POV | Player POV, commander map, first-person, free-cam, replay-scrub-during-live. |
| Draw-on-screen | Tactical annotations during live match. |
| Voiceover overlay | Commentator voice mic'd separately. |
| Multi-window (PiP) | Picture-in-picture for multiple POVs. |
| OBS overlay integration | Per [[spec/streaming-and-creator-features]]. |
| Stream delay | Configurable 5-30s for tournaments to prevent stream-sniping. |

## Replay Analysis Tools

| Tool | Detail |
|---|---|
| Heatmap | Per-player movement heatmap per match. |
| Decision-branching tree | Visualize utility scoring tree per AI decision. |
| Counterfactual replay | "What if you'd done X" — replay with hypothetical input. |
| Per-decision utility scoring | Visible AI reason labels per decision per DR-022. |
| Match-history aggregate stats | Per-mode + per-faction win-rate + per-map performance. |
| Per-player vs per-player analysis | Head-to-head record. |

## Anti-Rage-Quit Penalties

| Behavior | Penalty |
|---|---|
| Rage-quit | 5min queue penalty; escalates 30min; 2hr; 24hr; permaban after threshold. |
| Document disconnection | Grace period if disconnect cause documented (e.g., power outage). |
| Match-throwing | Detected via combat patterns (always-die, never-fire, AFK); flagged for review. |

## Pre-Match Warm-Up Area

1v1 against bot; 30-90s timer; for ranked queue while-waiting. Reduces queue-time friction.

## Coach Mode

Non-playing teammate watches your live match + sends advice (with consent).

| Aspect | Detail |
|---|---|
| Coach role | Spectator slot in match. |
| Communication | Voice (Steam Audio per DR-043) + text overlays. |
| Consent | Mutual opt-in; player can dismiss coach. |
| Reward | Cosmetic emblem for both. |

## Lobby Spectator Slot

Spectator joins lobby without playing; for press, content creators, friends.

## Tournament-Grade Anti-Cheat Profile

Per DR-005 post-launch evaluation; explicit milestone in M-COMP.

| Profile | Detail |
|---|---|
| `casual` (default) | Basic anti-cheat heuristics. |
| `competitive` | Stricter heuristics; per-match hash; strict mod-trust. |
| `tournament_strict` | Maximum heuristics; pre-match anti-cheat probe; replay-determinism enforced. |

## Done-Criteria

- [ ] ELO/MMR functional per mode.
- [ ] Bracket admin tools working.
- [ ] Tournament can be hosted by community.
- [ ] Observer cam + commentator overlay + multi-window functional.
- [ ] Replay analysis tools (heatmap + decision tree + counterfactual).
- [ ] Anti-rage-quit penalties trigger correctly.
- [ ] Pre-match warm-up area available.
- [ ] Coach mode functional.
- [ ] Lobby spectator slot.
- [ ] Tournament-grade anti-cheat profile signed off.

## Source Trail

- [[decisions/dr-049-customization-tournament-and-competitive]]
- Glicko-2: http://www.glicko.net/glicko/glicko2.pdf
- TrueSkill: https://en.wikipedia.org/wiki/TrueSkill
- StarCraft II observer tools: precedent for esports observer cam.
- Valorant ranked: ELO bracket precedent (and lessons; DR-049 closes anti-pay-to-rank).
- Rocket League replay analysis: precedent for heatmap + decision tree.
