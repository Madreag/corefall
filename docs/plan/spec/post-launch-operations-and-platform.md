---
type: spec
status: closed-direction
authority: "Post-launch operations: customer support workflow + Stripe direct-sales + refund handling + tax + regional pricing + sales calendar + bundle strategy + affiliate program + sales reports + wishlist conversion telemetry. Marketing extras: pre-launch ARG + OST + art book + comic + wiki + dev streams + Q&A + roadmap voting + bug bounty + translator credits + schools / educational license. Console + cloud-gaming + TV-friendly + performance polish (network indicator, auto-quality-presets, cold-load benchmark, memory leak detection, save backup chain, anti-cheat heuristics, server health dashboard, network simulator, crash-recovery flow)."
ready_when: "Customer support ticket system live; Stripe direct-sales functional; sales calendar published; ARG infrastructure pre-launch ready; performance polish features integrated."
feeds:
  - DR-005
  - DR-013
  - DR-024
  - DR-025
  - DR-029
  - DR-031
  - DR-034
  - DR-047
  - DR-051
---

← [[spec/index|spec section]] · [[decisions/dr-051-accessibility-sustainability-platform-and-launch-polish|DR-051]] · [[spec/marketing-and-launch|marketing]] · [[spec/legal-and-compliance|legal]]

# Post-Launch Operations & Platform Extensions

## Customer Support + Sales Operations

### Customer support workflow

| Component | Detail |
|---|---|
| Ticket system | HelpScout / Zendesk free tier OR self-hosted GitLab issues. |
| AI-first triage | Per-category routing; auto-acknowledge; suggest solutions. |
| Escalation | Project-owner gets escalated tickets. |
| SLA | 24h response. |
| Cataloged FAQs | Auto-generated from telemetry + bug-tool patterns. |

### Stripe direct-sales

Alternative to Steam-only for itch.io / direct-download buyers.

| Aspect | Detail |
|---|---|
| Per-region pricing | Stripe handles VAT / sales tax. |
| Receipt generation | Auto. |
| Refund handling | 15-day window for direct sales; AI-triage. |
| Tax handling | Stripe Tax addon. |

### Refund handling

Steam handles 2hrs/14days. Document custom policy for direct sales (15-day window; AI-triage).

### Sale calendar

Steam Summer / Autumn / Winter Sales + per-region holidays + 1-yr anniversary.

### Bundle strategy

Humble Bundle + indie bundles + cross-promotion bundles (with Cortex-likes / Soldat-likes / Liero-likes).

### Affiliate / creator program

Revenue share for content creators (post-launch tier; not pre-launch); 5% revenue per attributed sale via Steam Affiliate or Stripe affiliate codes.

### Sales reports

Post-launch monthly review; AI-driven summary; trend analysis; campaign ROI.

### Wishlist conversion telemetry

Track per-channel; optimize spend (free spend = best; paid only if ROI proven).

## Marketing Extras (Beyond DR-047)

### Pre-launch ARG

Per [[spec/server-wide-events-and-meta-narrative]]. Discord-driven; Reddit puzzles; in-world clues; narrative-extension. Runs ~3-6mo pre-launch.

### OST release on Bandcamp / Spotify

Post-launch monetization (separate from game purchase). 30+ tracks per [[spec/music-and-soundtrack]].

### Art book release (digital + print)

Pixel-art + concept art + commentary. POD via Lulu / Blurb.

### Comic / graphic novel tie-in

Community-authored, official-curated; Webtoon / Tapas.

### Wiki integration

Community wiki at https://corefall.wiki/ (or Fandom); auto-mirrored to in-game codex.

### Twitch dev streams

Project-owner Affiliate; weekly long-form dev streams.

### Community Q&A / town halls

Monthly Discord; project-owner present.

### Roadmap voting

Community votes on next feature post-launch; non-binding influence on backlog.

### Bug bounty program

Community catches critical issues for credit + game keys + emblem.

### Translator credits + Hall of Fame

In-game credits screen + Discord pin; per-language coordinator.

### Schools / educational license

If applicable; or just public-availability + free demo.

### Game-design-as-learning module

Use Workshop + cf-asset-pipeline to teach game design; partner with educational orgs.

### Dev streams as content

Auto-clip notable moments; YouTube secondary channel.

## Console + Cloud Gaming + TV-Friendly Evaluation

| Platform | Status |
|---|---|
| Switch / Switch 2 | Post-launch evaluation. Switch 2 perf-budget compatible per DR-028. |
| PS5 | Post-launch evaluation. |
| Xbox Series | Post-launch evaluation. |
| Mac App Store | Separate from Steam macOS build; post-launch. |
| Linux native binary on Flathub / Snap / AppImage | Post-launch packaging. |
| Mobile / tablet | Per DR-025 NO at launch; revisit post-launch. |
| Cloud gaming (GeForce Now / xCloud) | Compatibility evaluation; minor changes. |
| Steam Big Picture / TV-friendly UI | UI mode adaptation for couch-play. |
| Steam Link / Moonlight | Native support; tested in M-PLAYTEST. |

### Console evaluation criteria

- Cert path duration (3-6 months typical).
- Engineering effort (porting + cert + ongoing patches).
- Audience size for genre.
- Revenue share (Sony/MS/Nintendo: 30% standard).
- Cross-platform sync potential.

## Performance Polish

| Component | Detail |
|---|---|
| **Networking quality indicator** | In-lobby + in-match; ping bars + packet loss + jitter visible. |
| **Auto-quality-presets** | Based on hardware (GPU detection on first launch); suggest preset (Steam Deck / Low / Med / High / Ultra). |
| **Cold-load benchmark** | In CI; first-launch perf measurement. |
| **Memory leak detection** | In CI long-soak (24h+ run); report leaks. |
| **Replay-archive compression** | Smaller share size; codec tuning. |
| **Save backup chain** | Rolling 5 saves auto-archived; corruption detection; one-button restore. |
| **Anti-cheat heuristics per session** | Detect speed hacking, mod-tampering, ESP, replay-clip-fakery. |
| **Server health dashboard** | Community-hostable cf-server: shows shard load, mod conflicts, replay drift, perf counters. |
| **Network simulator** | For development testing; artificial latency / packet loss / jitter. |
| **Crash-recovery flow** | Auto-resume from latest snapshot on crash; offer 'revert to checkpoint'. |
| **Cold-resume from save** | Fast load time even with large saves. |
| **Streaming asset budget** | Budget per scenario; verified at M-PLAYTEST soak. |

## Done-Criteria

### Customer support + sales

- [ ] Ticket system live + AI-triage.
- [ ] Stripe direct-sales functional.
- [ ] Refund policy documented.
- [ ] Sale calendar published.
- [ ] Bundle partnerships established.
- [ ] Affiliate program tracking.
- [ ] Wishlist conversion telemetry.

### Marketing extras

- [ ] Pre-launch ARG infrastructure ready.
- [ ] OST release on Bandcamp / Spotify (post-launch).
- [ ] Art book published.
- [ ] Wiki integration.
- [ ] Dev streams cadence established.
- [ ] Bug bounty program live.
- [ ] Translator credits in-game.

### Platform + perf

- [ ] Console evaluation complete (per platform).
- [ ] Steam Big Picture / TV-friendly UI tested.
- [ ] Cloud gaming compatible.
- [ ] Network indicator visible.
- [ ] Auto-quality-presets functional.
- [ ] Cold-load benchmark in CI.
- [ ] Memory leak detection in CI.
- [ ] Save backup chain functional.
- [ ] Server health dashboard.
- [ ] Crash-recovery flow tested.

## Source Trail

- [[decisions/dr-051-accessibility-sustainability-platform-and-launch-polish]]
- [[decisions/dr-047-launch-and-live-operations]]
- [[spec/marketing-and-launch]]
- [[spec/steam-and-platform-integration]]
- HelpScout: https://www.helpscout.com/
- Stripe: https://stripe.com/
- Stripe Tax: https://stripe.com/tax
- Steam Affiliate Program: https://partner.steamgames.com/
