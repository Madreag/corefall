---
type: decision
id: DR-031
status: closed-direction
priority: P0
closed_at: 2026-05-04
revisit_trigger: "Premium price ceiling fails to support development; or community pressure forces a marketplace; or post-launch monetization signal warrants revisiting expansion/DLC scope."
---

← [[decisions/index|decision records]] · [[dashboards/decision-tracker|decision tracker]] · [[spec/prototype-roadmap|native build roadmap]] · [[decisions/dr-006-modding-data-model|DR-006]] · [[decisions/dr-010-license-reuse-matrix|DR-010]] · [[decisions/dr-011-progression-retention-loop|DR-011]]

# DR-031: Content Economy And Monetization Posture

> [!success] Status: CLOSED-DIRECTION (project owner committed 2026-05-04)
> **Premium one-time purchase + free modding.** Post-launch expansions, DLC, and cosmetics are allowed. **No core-mechanic monetization** (no pay-to-win, no gacha, no battle passes that gate gameplay, no paid mod marketplace cut). Modding stays first-class and free for both authors and players.

## Decision

**Premium game; community-friendly economy.** The launch SKU is a one-time purchase. Modding is free; mod authors keep their work and can use third-party storefronts (itch, Mod.io, Steam Workshop) without revenue cuts owed to the publisher. Future paid content is allowed in the form of expansions, cosmetic DLC, or designer-authored scenario packs — never as paid access to base mechanics or competitive advantage.

## What This Locks In

| Aspect | Commitment |
|---|---|
| Launch SKU | Premium one-time purchase. |
| Modding access | Free for authors; free for players. |
| Mod distribution | Third-party (Steam Workshop, itch, Mod.io, direct) at launch; first-party hub later, no revenue cut on user mods. |
| Cosmetic items | Allowed post-launch (skins, palettes, paint kits, name plates). |
| Expansions / DLC | Allowed post-launch (new chassis archetypes, mission packs, factions). |
| Pay-to-win / mechanical advantage | **Forbidden.** Period. |
| Gacha / loot boxes / random chance for gameplay | **Forbidden.** |
| Battle pass that gates gameplay | **Forbidden.** Cosmetic-only battle pass is open for post-launch consideration but not committed. |
| Paid scenario marketplace cut | **No publisher cut on user-authored scenarios.** Designer-authored scenario packs (made by us) are sold as DLC. |
| Cloud features | Free for save-sync (post-launch); never gated behind a subscription. |

## What This Does NOT Lock

- Specific launch price.
- Whether a cosmetic battle pass exists post-launch.
- Whether designer-authored expansion packs are episodic or large.
- Whether the game enters subscription bundles (Game Pass, etc.) — case-by-case.
- Cloud-save provider and pricing for that backend (post-launch DR).

## Why This Posture

| Reason | Why |
|---|---|
| Modding is core to retention per DR-006 / DR-011 | Charging modders or taking a marketplace cut undermines the loop. |
| Tactical pulp tone per DR-014 is incompatible with gacha/RNG monetization | The brand promise is competence over chance. |
| Genre alignment | Cortex / Liero / Soldat / Powder Toy audiences punish predatory monetization. |
| AI-augmented solo team per DR-026 | A premium SKU + expansion model is the clean revenue path that doesn't require live-ops headcount. |
| Solo-first posture per DR-005 + DR-013 | We don't have backend headcount for live-service economy. |

## Why Not The Alternatives

| Alternative | Why Rejected |
|---|---|
| Free-to-play with battle pass / gacha | Wrong genre fit; conflicts with retention principles. |
| Free-to-play with cosmetic shop only | Reduces development runway; pressure to monetize creeps into mechanics. |
| Subscription | Wrong category for the base game. MMO shards are community-hostable and not subscription-funded by the base SKU; any operator hosting fee is outside the core economy posture. |
| Marketplace with cut on user mods | Damages community trust; DR-010 license matrix already commits to community-friendly posture. |
| Crypto / NFT integration | Hard no. Conflicts with sandbox trust + modding ethics. |
| Console paid-only with PC piracy excuse | Wrong incentive; PC + Linux are first-class per DR-025. |

## Evidence Trail

- Project owner verbatim (2026-05-04 stack round): "Premium game + free modding. Expansions/DLC/cosmetics later. No core-mechanic monetization."
- DR-006 modding data model commits to first-class modding.
- DR-010 license/reuse matrix establishes community-friendly content posture.
- DR-011 progression/retention loop is intrinsic-first (mastery + replays + creator challenges), not gacha/grind.
- Comparable wins: Powder Toy, Cortex itself (premium + community), Factorio (premium + free major patches).
- Comparable losses: any sandbox-genre game that bolted on gacha/loot-box mechanics post-launch.

## Risks

| Risk | Mitigation |
|---|---|
| Premium price ceiling underfunds development | Expansion/DLC roadmap post-launch; cosmetic skins; avoid feature creep that bloats v1. |
| Community pressure to host first-party mod marketplace | Build the hub for **discovery** (no revenue cut on user mods). Designer-authored DLC is sold separately. |
| A future SKU / partner deal pushes a battle pass | DR-014 + this DR are the guardrails; revisit only with an explicit DR change. |
| Marketplace cut creep | Document non-promise; make it visible at every monetization conversation. |
| Cosmetic battle pass disrupts the brand | Keep cosmetic-only and avoid time-pressure psychology (no FOMO timers). |

## Prototype / Validation Plan

| Test | What It Proves |
|---|---|
| M7 — Premium SKU is technically buildable (Steam build hooks, demo, refund flow). | Launch SKU is real. |
| M8 — Sample mod is freely distributable; player loads from third-party source. | Modding is free in practice. |
| Post-launch — A cosmetic DLC ships without affecting core mechanics. | Cosmetic boundary holds. |
| Post-launch — Mod author publishes to Workshop / Mod.io / direct without paying us. | Marketplace-cut posture holds. |

## Revisit Trigger

- Premium price ceiling fails to support development.
- Community pressure forces a marketplace decision.
- Post-launch monetization signal warrants revisiting expansion/DLC scope.
- A new platform partnership requires a different SKU model.

## Source Trail

- Project owner stack-round answers (2026-05-04).
- [[decisions/dr-006-modding-data-model]]
- [[decisions/dr-010-license-reuse-matrix]]
- [[decisions/dr-011-progression-retention-loop]]
- [[decisions/dr-014-tone-player-promise]]
- [[strategy/best-cortex-like-game-principles]]
- [[references/usage-ledger]]
- [[spec/prototype-roadmap]] — Strategic Frame + Anti-Goals.
- [[research-log/2026-05-04-roadmap-rebuild-native-stack]]
