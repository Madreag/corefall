---
type: spec
status: closed-direction
authority: "Marketing + launch: Steam page, trailer, demo, press kit, wishlist drive, Discord, AI-driven outreach. Steam Next Fest demo. Direct-to-Steam indie launch (no publisher). Ad spend $0 baseline."
ready_when: "Steam page live with trailer + screenshots + 50K+ wishlists at launch; demo cut for Next Fest; press outreach started; Discord active; press kit complete."
feeds:
  - DR-019
  - DR-024
  - DR-026
  - DR-031
  - DR-047
---

← [[spec/index|spec section]] · [[decisions/dr-047-launch-and-live-operations|DR-047]] · [[spec/playtest-program|playtest]] · [[spec/steam-and-platform-integration|Steam]]

# Marketing & Launch

## Posture

**Direct-to-Steam indie launch. No publisher (per DR-047). Ad-spend $0 baseline. Organic + community + Steam algorithm + creator-driven discovery.**

## Steam Page

**Launched 6-12 months pre-release.**

- Title art (Tier 3 hand-polished)
- Capsule art (small/medium/large/header per Steam spec)
- 10+ screenshots (at-launch + Tier 3 polished)
- 90-second reveal trailer
- 30-second gameplay trailer
- 60-second "what is Corefall?" trailer
- Description copy (AI-generated; 2-3 paragraphs + 8 bullet features)
- System requirements
- Languages (Tier-A list per [[spec/localization-plan]])
- Tags: Sandbox, Pixel Art, Tactical, Sci-Fi, Multiplayer, Modding, Local Co-op, Online Co-op, PvE, PvP

## Trailer Production

| Trailer | Length | Method |
|---|---|---|
| **Reveal trailer** | 60-90s | Stable Video Diffusion + AnimateDiff clips + AI-composed Suno score + project-owner narrator (or hire VO) + locked-in shots from prototypes. |
| **Gameplay trailer** | 30s | Real gameplay clips + adaptive music + voice-over. |
| **"What is Corefall?" trailer** | 60-90s | Comic-noir storytelling style; explains the player promise. |
| **Launch day trailer** | 90-120s | Final product showcase; pinned. |

## Press Kit (presskit() format)

- Logo (multiple formats)
- Screenshots (high-res)
- Key art (4K)
- 3 trailers
- 1-pager fact sheet
- Contact info
- Demo build link
- Quotes (post-press-coverage)

## Demo Build (Steam Next Fest)

**30-60 minute slice.**

- Bunker Defence flagship + 1 onboarding mission + 1 lab + 4-player coop unlocked.
- Demo time-limited. Wishlist drive CTA at end.
- Demo persists save; carries achievements forward to full game.

## Wishlist Drive

| Goal | Target |
|---|---|
| Pre-launch 6mo | 10K wishlists |
| Pre-launch 3mo | 25K |
| Steam Next Fest | 50K-100K (festival amplification) |
| Launch day | Convert ~10-15% of wishlists |

## Channels

| Channel | Use |
|---|---|
| Reddit (r/IndieGaming, r/CortexCommand, r/games, r/gamedev, r/pixelart) | Devlogs, demo announce, launch |
| TikTok | Daily devlog clips (15-30s); AI-edited |
| Twitter / X | Daily updates; AI-drafted, project-owner approved |
| Bluesky | Mirror Twitter |
| YouTube devlogs | Weekly long-form (8-12 min) |
| IndieDB | Cross-post |
| itch.io | Mod-friendly demo + early build |
| Discord | Pre-launched at Steam page launch |

## Discord

| Channel | Use |
|---|---|
| announcements | Project-owner only |
| general | Community chat |
| playtest | Closed alpha/beta access |
| mod-creators | Modder community |
| language-X (per Tier-A locale) | Per-language community + translation |
| bug-reports | Bug discussion (separate from in-game F12 tool) |
| fan-art | Community art |
| screenshots | Community screenshots |
| support | Help requests |

AI-moderated baseline + community moderators.

## Press Outreach

| Tier | Recipients | When |
|---|---|---|
| **Tier-1** | RPS, PC Gamer, Eurogamer, Kotaku, IGN-indie | At demo + at launch |
| **Tier-2** | Regional gaming press | At launch |
| **Tier-3** | YouTubers, TikTok creators, Twitch streamers | At demo with creator-keys |

AI-generated personalized outreach emails per recipient (project-owner approves).

## AI-Driven Outreach + Social

- AI agent drafts daily devlog post template; project-owner approves.
- AI agent drafts press emails per recipient; project-owner approves.
- AI agent monitors community channels; surfaces noteworthy posts to project-owner.
- AI agent generates social-post schedule; project-owner approves cadence.

## Done-Criteria

- [ ] Steam page live 6-12mo pre-launch.
- [ ] Reveal trailer + gameplay trailer + "what is" trailer.
- [ ] Press kit complete.
- [ ] Demo cut for Steam Next Fest.
- [ ] 50K+ wishlists at launch.
- [ ] Discord active with all channels.
- [ ] Press outreach completed at demo + launch.
- [ ] Community-driven creator coverage post-launch.

## Source Trail

- [[decisions/dr-047-launch-and-live-operations]]
- Steam Next Fest 2026 prep checklist: https://gamineai.com/blog/steam-next-fest-2026-prep-checklist-indie-devs-wishlists-demo-press
- presskit(): https://dopresskit.com/
