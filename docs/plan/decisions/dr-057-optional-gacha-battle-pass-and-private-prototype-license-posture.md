---
type: decision
id: DR-057
status: closed-direction
priority: P0
closed_at: 2026-05-07
revisit_trigger: "Optional gacha/battle-pass architecture moves from dormant hooks into production implementation; monetization becomes release-facing; a public-sale decision is made; or optional economy pressure starts changing fairness, modding, accessibility, or roadmap priority."
---

← [[decisions/index|decision records]] · [[dashboards/decision-tracker|decision tracker]] · [[decisions/dr-010-license-reuse-matrix|DR-010]] · [[decisions/dr-011-progression-retention-loop|DR-011]] · [[decisions/dr-031-content-economy-and-monetization-posture|DR-031]] · [[decisions/dr-044-audiovisual-production-pipeline|DR-044]] · [[decisions/dr-053-ai-audio-pipeline-realtime-and-generative|DR-053]] · [[references/usage-ledger|usage ledger]]

# DR-057: Optional Gacha, Battle Pass, And Private-Prototype License Posture

> [!success] Status: CLOSED-DIRECTION (project owner clarified 2026-05-07)
> Corefall must **not** be architecturally hostile to optional cosmetic gacha-like collection mechanics or a cosmetic-only battle pass, but those systems are **late, optional, toggleable, and not an early roadmap focus**. Pay-to-win, gameplay power locks, paid mod marketplace cuts, and FOMO pressure remain forbidden. Private prototype asset/audio generation is **ledger-first, not license-gate-first**: use the best AI tools available, log provenance/license/status, and clean/replace/relicense before any public sale or release.

## Decision

**Design-in, defer-focus.** The project may reserve schemas, config flags, UI slots, event categories, and test harnesses for future optional gacha-like collection and cosmetic battle-pass systems, but those hooks must stay dormant until a late milestone or future DR intentionally activates them.

The default product economy remains intrinsic-first per DR-011 and community-friendly per DR-031. Optional economy hooks exist so the game can add these systems later without a painful rewrite, not so early milestones chase monetization.

Private prototypes can use the best audio, image, video, voice, and generation providers available, including ElevenLabs under the owner's subscription and any better current provider. The gate is not "license clears before private use"; the gate is "usage-ledger provenance exists before the asset is retained." Before any public sale or release, every retained asset must be cleared, replaced, relicensed, or regenerated through a release-safe source.

## What This Locks In

| Area | Commitment |
|---|---|
| Gacha-like collection mechanics | Allowed as a late optional architecture hook. Not a launch focus, not required for v1, not allowed to hide gameplay power or core counters. |
| Loot-box/randomized reward surfaces | May be prototyped privately if transparent and logged. Release-facing use needs a separate activation DR, legal/rating review, odds disclosure, age/regional controls, anti-FOMO archive, and no power locks. |
| Battle pass | Cosmetic-only battle-pass architecture is allowed and should be turn-off-able. It is late/post-launch/extra, not an early roadmap priority. |
| Battle-pass toggles | `battle_pass.enabled = false` by default for private servers, solo, and dev builds until explicitly activated. |
| Gacha toggles | `gacha.enabled = false` by default. Servers/mod packs can disable the surface entirely. |
| Pay-to-win | Still forbidden. No paid/random/progression track may grant exclusive combat power, required counters, deterministic advantage, or tournament advantage. |
| FOMO | Still forbidden. Any event/battle-pass/reward system must have archive or earn-back path and must not punish breaks. |
| Mod marketplace cut | Still forbidden. No project revenue cut on user-authored mods. |
| Private generation tools | Use the best tool for quality and speed. ElevenLabs is available but not exclusive. Suno/Udio/Stable Audio/MusicGen/XTTS/ComfyUI/other providers remain candidates. |
| License/provenance posture | Ledger-first for private prototypes; release-cleanup gate only before public sale/release. |

## Architecture Requirements

| Surface | Required Shape |
|---|---|
| Config | `economy.enabled`, `battle_pass.enabled`, `gacha.enabled`, and `limited_event_archive.enabled` are explicit config fields, defaulting to off for battle pass/gacha. |
| Data model | Cosmetic/collection rewards use data-driven `collection_entry` records with source event, unlock path, odds group if any, release status, accessibility caption, localization keys, and mod provenance. |
| Server authority | Any activated economy surface is server-authoritative for public servers. Solo uses the same local in-process authority path. |
| CLI | `cfctl observe economy`, `cfctl act economy.disable`, `cfctl test economy-disabled`, `cfctl test battle-pass-disabled`, and `cfctl test no-power-locks` must exist before activation. |
| Modder parity | Modders can define cosmetic tracks and collections through the same schema, but public servers can disable them and tournament profiles ignore them. |
| Accessibility | Battle-pass/collection UI must satisfy DR-012 + DR-051: 200% scale, high contrast, no color-only rarity, screen-reader labels, reduced motion, cognitive-friendly summary, and no pressure timers. |
| Localization | Every visible label, reward name, odds disclosure, archive message, and purchase warning is keyed per DR-046. |
| Telemetry | Optional and privacy-gated. No dark-pattern optimization target such as "increase purchase pressure" is allowed. |

## Validation Plan

| Test | What It Proves |
|---|---|
| `cfctl test economy-disabled --scenario X` | Battle pass/gacha hooks can be compiled in but fully disabled with no UI orphan, no state mutation, and no network traffic. |
| `cfctl test battle-pass-disabled --server-config content/server/no_economy.ron` | Server config can remove the battle-pass surface from client navigation and observations. |
| `cfctl test no-power-locks --reward-table X` | Reward tables cannot contain gameplay-required counters, exclusive power upgrades, or tournament-effective stats. |
| `cfctl test odds-disclosure --collection X --locale en-US` | Any randomized collection surface has visible, localized, screen-reader-readable odds before activation. |
| `cf-asset-ledger check --mode private` | Private prototype assets pass provenance completeness without blocking on commercial release clearance. |
| `cf-asset-ledger check --mode release` | Public release candidates fail unless every retained asset is cleared, replaced, relicensed, or regenerated. |

## What This Does NOT Lock

- Actually shipping gacha, loot boxes, paid random rewards, a battle pass, or a paid cosmetics store.
- A free-to-play business model.
- Any early-roadmap implementation priority for battle pass/gacha beyond preserving clean extension seams.
- A specific AI audio/art/voice provider.
- A public sale. If a sale happens later, release cleanup becomes mandatory before sale.
- Any weakening of DR-031's no-pay-to-win, no-marketplace-cut, and anti-FOMO requirements.

## Risks

| Risk | Mitigation |
|---|---|
| Optional economy hooks become design gravity too early. | Roadmap marks them late/extra; v1 gates continue to prioritize core feel, sync, modding, accessibility, and intrinsic retention. |
| Gacha/battle-pass presence damages player trust. | Default disabled; activation requires a future DR with fairness, legal/rating, accessibility, localization, modding, and anti-FOMO evidence. |
| License debt accumulates during private prototyping. | Ledger completeness is mandatory before retaining any asset; release check mode fails unclear assets before public sale/release. |
| Modded/tournament environments inherit unwanted economy surfaces. | Server config and tournament profile must be able to disable all economy UI/state. |
| "Cosmetic" rewards harm readability. | Cosmetic schemas include combat-readability flags and tournament-safe profile filters. |

## Source Trail

- Project owner clarification (2026-05-07): gacha is not banned outright; cosmetic-only battle pass should be architecturally possible and toggleable; license/provenance should not block private use of the best generation providers; ElevenLabs subscription is available but not exclusive.
- [[decisions/dr-010-license-reuse-matrix]]
- [[decisions/dr-011-progression-retention-loop]]
- [[decisions/dr-031-content-economy-and-monetization-posture]]
- [[decisions/dr-044-audiovisual-production-pipeline]]
- [[decisions/dr-053-ai-audio-pipeline-realtime-and-generative]]
- [[references/usage-ledger]]
- [[research-log/2026-05-07-comprehensive-audit-report]]
