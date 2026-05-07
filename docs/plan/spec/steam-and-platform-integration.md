---
type: spec
status: closed-direction
authority: "Steam + EOS + GOG + itch.io integration: Workshop, Achievements, Cloud, Friends, Input, Deck Verified, EOS adapter (optional), GOG/itch.io builds. bevy_steamworks. Reference Docker image."
ready_when: "Steam Workshop accepts mods; achievements unlock; cloud saves sync; Deck Verified achieved; EOS optional cargo feature builds clean."
feeds:
  - DR-005
  - DR-006
  - DR-024
  - DR-025
  - DR-029
  - DR-031
  - DR-034
  - DR-047
---

← [[spec/index|spec section]] · [[decisions/dr-047-launch-and-live-operations|DR-047]] · [[decisions/dr-006-modding-data-model|DR-006]] · [[spec/server-app-architecture|server app]]

# Steam & Platform Integration

## Steam Features

| Feature | Detail |
|---|---|
| **Workshop** | Mod packages publishable from in-game (one-button publish per DR-006 + DR-045). Community subscribe + auto-install. Trust tiers per DR-034. |
| **Achievements** | 60-100 achievements. Most "play 1 of each chassis" / "complete each mission"; ~10 hidden lore/mastery. |
| **Cloud** | Saves + replay archive auto-sync. Encrypted. Per DR-029. |
| **Friends + Invites** | Friend list, party invite to lobby, presence ("In Bunker Defence — Mars"). |
| **Input** | Full controller / gamepad / Steam Deck support; community bindings sharable via Steam Input. |
| **Deck Verified** | Target Verified rating: 800p/60 perf, controller-complete, readable text, no shader compilation hitches. |
| **Trading Cards** | Non-monetized cosmetic. Earned via play. |
| **Remote Play Together** | LAN co-op via Steam Remote Play (free). |
| **Stats** | Per-player aggregate stats; appears on player profile. |
| **Leaderboards** | Per-mission speedrun + per-mode (Bunker Defence wave) + daily seed. |

## Tech Stack

| Component | Detail |
|---|---|
| `bevy_steamworks` | Bevy plugin. https://docs.rs/bevy-steamworks |
| `steamworks` | Underlying SDK wrapper. |
| Steamworks SDK | Bundled with `steamworks` crate. |

## EOS Adapter (Optional)

Cargo feature `--feature eos`. Off by default. Provides cross-platform Friends + Lobby for Epic Games Store users. Per DR-005 + DR-013, optional adapter behind feature flag.

## GOG.com (Post-Launch)

DRM-free build for GOG. Same binary; no DRM stripped (game is already DRM-free per DR-031). Releases post-launch with established Steam audience.

## itch.io

Mod-friendly demo + early-access build. Same binary. Free or pay-what-you-want demo strategy.

## Console Ports (Post-Launch Evaluation)

Per DR-025, no console at launch. Post-launch evaluation:
- Switch (Switch 2 friendly given perf budget); Nintendo cert path.
- PS5; Sony cert path.
- Xbox Series; Microsoft cert path.

## Reference Docker Image

`cf-server:latest` runs unchanged. Per [[spec/server-app-architecture]] M9-017. Linux + Windows. Hosting guide documented.

## Done-Criteria

- [ ] Steam Workshop accepts + distributes mods.
- [ ] All 60+ achievements unlock.
- [ ] Cloud saves sync.
- [ ] Steam Friends + Invites work.
- [ ] Steam Input bindings work.
- [ ] Steam Deck Verified achieved.
- [ ] Trading Cards earnable.
- [ ] Remote Play Together works.
- [ ] EOS adapter cargo feature builds clean.
- [ ] Reference Docker image runs.

## Source Trail

- [[decisions/dr-047-launch-and-live-operations]]
- bevy_steamworks: https://docs.rs/bevy-steamworks
- Steam Workshop docs: https://partner.steamgames.com/doc/features/workshop/implementation
- Steam Deck Verified: https://partner.steamgames.com/doc/deckverified
