---
type: decision
id: DR-049
status: closed-direction
priority: P0
closed_at: 2026-05-06
revisit_trigger: "Customization depth proves balance-prohibitive; ELO/MMR system fails to retain ranked players; tournament infrastructure becomes server-cost burden; observer/commentator tools fail to attract competitive scene."
---

← [[decisions/index|decision records]] · [[dashboards/decision-tracker|decision tracker]] · [[spec/customization-and-progression-depth|customization spec]] · [[spec/tournament-and-competitive-infrastructure|tournament spec]] · [[decisions/dr-005-multiplayer-posture|DR-005]] · [[decisions/dr-031-content-economy-and-monetization-posture|DR-031]] · [[decisions/dr-042-game-modes-and-match-grammar-direction|DR-042]]

# DR-049: Customization, Tournament & Competitive Infrastructure

> [!success] Status: CLOSED-DIRECTION (project owner committed 2026-05-06)
> Customization: weapon attachments + salvage crafting + intrinsic mastery skill tree + loadout sharing + paint/decal/skin/voice/emblem variants + vendor economy. Competitive: ELO/MMR + tournament bracket infrastructure + observer/commentator tools + replay analysis + coach mode + pre-match warm-up. **NEVER pay-to-win** per DR-031; all customization gates intrinsic (mastery, achievement, mission); all competitive features support fair play.

## Decision

### Customization depth

| Layer | Detail |
|---|---|
| **Weapon attachments** | Sights (red dot, ACOG, holographic, iron, scope, thermal), suppressors, lasers, magazines (extended, drum, fast-mag), grips (vertical, angled), stocks (folding, retractable, heavy), barrels (long, short, heavy, light), barrel attachments (compensator, muzzle brake), chamber inserts (auto-loader). Per-weapon attachment slots data-driven via DR-006 schema. |
| **Salvage crafting** | Combine salvage + ore + materials → custom gear at base workbench. Recipes data-driven; modders extend. Per DR-041 mining + DR-027 base. |
| **Intrinsic mastery skill tree** | Per-chassis / per-faction / per-weapon mastery rank (1-30 per DR-047 endgame). Unlocks: variants (different weapon flavor), paint, voice lines, lore entries. **NEVER power upgrades.** Pure intrinsic. |
| **Loadout templates + sharing** | Save loadout as template; 5 quick-swap slots per profile; export/import RON; share to Workshop. |
| **Per-loadout custom hotbar** | Define hotkey scheme per loadout. |
| **Paint jobs + decals** | Per-chassis paint via alpha-mask painting on metallic regions. Custom decal placement. Faction emblems. |
| **Voice packs** | Per-faction voice variant for player's commander. |
| **Victory poses** | Animated end-of-match poses; earned via play. |
| **Vendor / economy NPCs** | Per CCCP precedent. Persistent currency (`oz` per Cortex precedent). Buy menu at base. NPC merchants travel between worlds with stock variation. |
| **Item-comparison side-by-side UI** | In workbench: select 2 items → side-by-side stat overlay + AI-generated "differs in X" callout. |

### Tournament & competitive infrastructure

| Component | Detail |
|---|---|
| **ELO/MMR system** | Per-mode ELO. Visible to player. Bayesian update per match. Decay for inactivity. Per-faction sub-ratings (player has separate ratings per faction). |
| **Ranked PvP brackets** | Bronze/Silver/Gold/Platinum/Diamond/Master/Champion. Per-season reset (cosmetic-only rewards per DR-031). |
| **Tournament bracket infrastructure** | Invite-bracket, double-elim, swiss; admin tools; community-organizer tools. Per-tournament cf-server instance. |
| **Observer cam / commentator tools** | Switchable POV (player POV, commander map, first-person, free-cam, replay-scrub during live match). Draw-on-screen. Voiceover overlay. Multi-window (PiP). |
| **Replay analysis tools** | Heatmap of player movement; decision-branching tree; counterfactual replay ("what if you'd done X"); per-decision utility scoring visible. |
| **Match-history aggregate stats** | Per-mode + per-faction win-rate + per-map performance. |
| **Anti-rage-quit penalties** | Matchmaking penalty for quitters (5min wait → 30min); grace period for documented disconnections. |
| **Pre-match warm-up area** | 1v1 against bot; 30-90s timer; for ranked queue while-waiting. |
| **Coach mode** | Non-playing teammate watches your live match + sends advice (with consent). |
| **Lobby spectator slot** | Spectator joins lobby without playing; for press, content creators, friends. |
| **Tournament-grade anti-cheat profile** | Per DR-005 post-launch; explicit milestone in M-COMP. |

### Cosmetic earn paths (NEVER paid; per DR-031)

| Path | Reward |
|---|---|
| Mastery rank | Variants, paint, voice lines, lore entries |
| Achievement | Skin, decal, emblem |
| Mission completion | Mission-specific cosmetic (per-mission unique) |
| Replay share count | "Star creator" emblem, voice pack |
| Bunker-Defence wins | Defender-elite paint, victory pose |
| Speedrun verification | Speedrun-elite emblem |
| Daily seed leaderboard top-100 | Daily-elite emblem, paint |
| Tournament placement | Tournament-elite paint, voice pack |
| Modder published mod | Modder-elite emblem |
| Translator credit | Translator-credit emblem |
| Bug bounty acceptance | Bug-finder emblem |

## What This Locks In

| Spec Area | Implication |
|---|---|
| `cf-equipment` | Extended for attachment slots; data-driven schema per DR-006. |
| `cf-crafting` | New crate (or extension of `cf-mission`) for salvage crafting recipes. |
| `cf-mastery` | New crate for per-chassis/per-faction/per-weapon mastery + intrinsic skill tree. |
| `cf-economy` | New crate for currency + vendor NPCs + buy menu. |
| `cf-server-tournament` | New crate (or extension of `cf-server`) for ELO/MMR + bracket admin + observer/commentator tools. |
| `cf-replay-analysis` | New crate for heatmap + decision tree + counterfactual replay. |
| `cf-cosmetic` | Cosmetic locker + earn-path tracker + paint mask system. |

## What This Does NOT Lock

| Non-Commitment | Why |
|---|---|
| Specific weapon attachment list | Open. Target ~80 attachments at launch + 30 mod-extension slots. |
| ELO/MMR specific math | Open. Default Glicko-2; can tune. |
| Number of seasonal tiers | Open. Default 7 tiers (Bronze-Champion). |
| Voice acting for voice packs | Open. AI-generated default; commission post-launch if budget allows. |

## Why This Direction

| Driver | Detail |
|---|---|
| Solo-player retention | Without customization depth, players plateau at hour 30-40. Per Helldivers/Deep Rock Galactic precedent, attachment + paint + voice variety drives 100hr+ play. |
| Competitive scene | Without tournament + observer + replay-analysis tools, no PvP scene crystallizes. Cf. esports infrastructure needs (LoL/Valorant precedents adapted to indie scale). |
| Modder ecosystem | Modders need attachment slots + crafting recipes as data + replay analysis tools to author. |
| Anti-pay-to-win | All customization is intrinsic (per DR-031). No microtransaction; no battle pass; no marketplace. |

## Why Not The Alternatives

| Alternative | Why Rejected |
|---|---|
| No customization depth | Loses retention long-tail. |
| Pay-for-customization | Forbidden by DR-031. |
| Skill tree with power upgrades | Forbidden by DR-031 (intrinsic-first). |
| No tournament tools | PvP scene doesn't crystallize without observer/commentator. |
| Centralized ranked-only matchmaking | Per DR-005 community-hostable; tournaments must be self-hostable too. |

## Evidence Trail

- Helldivers 2 customization: ~80 attachments per weapon class drives 100hr+ retention.
- Deep Rock Galactic: extensive cosmetic + mastery system; intrinsic-only; 200hr+ median play time.
- LoL/Valorant esports infrastructure: observer/commentator tools required for scene crystallization.
- Captured in [[research-log/2026-05-07-comprehensive-audit-report]].

## Revisit Trigger

- Customization depth proves balance-prohibitive (some attachments dominate).
- ELO/MMR system fails to retain ranked players (queue times explode).
- Tournament infrastructure becomes server-cost burden.
- Observer/commentator tools fail to attract competitive scene.
