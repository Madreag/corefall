---
type: spec
status: closed-direction
authority: "Legal + compliance: trademark, business entity, EULA, ToS, Privacy Policy, age rating, GDPR/CCPA/LGPD, open-source attribution, AI-asset licensing, modding rights, accessibility compliance, no loot boxes."
ready_when: "Trademark filed; EULA/ToS/PP drafted by counsel; age ratings submitted; attribution screen auto-generated; AI-asset usage-ledger complete."
feeds:
  - DR-006
  - DR-010
  - DR-012
  - DR-031
  - DR-044
  - DR-046
  - DR-047
---

← [[spec/index|spec section]] · [[decisions/dr-047-launch-and-live-operations|DR-047]] · [[references/usage-ledger|usage-ledger]]

# Legal & Compliance

## Pre-Launch Items

| Item | Owner | When |
|---|---|---|
| **Trademark search + registration** | Legal counsel ($1-2K) | M-MARKETING phase (~6-12mo pre-launch) |
| **Domain registration** | Project owner | Pre-Steam-page launch (corefall.com / corefall.gg / corefall.dev) |
| **Business entity** | Project owner ($300-1K LLC formation) | Pre-launch (Wyoming or Delaware LLC, US-friendly tax) |
| **Bank account + Stripe** | Project owner | Pre-launch |
| **EULA + ToS + Privacy Policy** | Legal counsel ($2-5K) | Pre-Steam-page launch |
| **Age rating submission** | Project owner via IARC self-rating + ESRB/PEGI/USK certs | Pre-launch (4-8 weeks) |
| **Open-source attribution screen** | AI agent auto-generated from `Cargo.lock` via `cargo-about` | M0 (CI gate per release) |
| **AI-asset usage-ledger audit** | Project owner | Pre-launch (full audit + license verification) |

## EULA / ToS / Privacy Policy

Drafted by legal counsel. Must cover:

- Gameplay license (one-time-purchase, per DR-031)
- Modding rights (modders retain copyright; Workshop CC-BY-SA default)
- Data collection (GDPR / CCPA / LGPD compliant; opt-in in EU)
- Workshop content rights (DMCA process for IP claims)
- Dispute resolution (binding arbitration; small-claims carve-out)
- Age requirement (13+ COPPA; 16+ for EU full features)
- Prohibited conduct (cheating, harassment, illegal content)
- Termination clauses
- Limitation of liability
- Governing law

## Age Rating

| Region | Rating target |
|---|---|
| **ESRB (US/CA)** | Mature 17+ (violence, blood, mild language) |
| **PEGI (EU)** | 16-18 (violence, mild language) |
| **USK (DE)** | 16 |
| **CERO (JP)** | D (17+) |
| **ACB (AU)** | MA15+ |

Submission via IARC self-rating then per-region cert.

## Privacy / GDPR / CCPA / LGPD

- Privacy notice on first launch.
- Right-to-deletion endpoint per DR-047.
- Data Processing Agreements with: Steam, Sentry/GlitchTip, ElevenLabs (if used).
- Cookie/data prompts where required.
- Privacy-by-default in EU.

## Open-Source Attribution

Auto-generated from `Cargo.lock` via `cargo-about` Rust crate:

```bash
cargo about generate -c about.toml > attribution.html
```

In-game credits screen lists every OSS dependency + license. Per-release update via CI.

## AI-Asset Licensing

Per DR-044 + [[references/usage-ledger]]:

- Every AI-generated asset has an entry: prompt, seed, model, LoRA, license, regenerable Y/N.
- No open-weight model is assumed release-cleared without checking the exact model/weight license. Stable Audio Open uses Stability AI Community License / commercial registration rules; AudioCraft code is MIT but released MusicGen weights are CC-BY-NC 4.0.
- LoRAs sourced from Civitai with permissive licenses; verified pre-launch.
- Suno/Udio music subject to TOS review pre-launch (commercial use currently allowed; revisit).
- ElevenLabs voice subject to TOS review pre-launch.
- Tier-3 AI-agent cleanup doesn't change underlying model licensing.

## Modding Rights

- Modders retain copyright on their mod content.
- License them to other players via Workshop.
- Default license: CC-BY-SA 4.0 (Workshop allows other choices: GPL, MIT, custom).
- Workshop ToS handles IP claims via DMCA process.

## Anti-Harassment / Code of Conduct

- Discord ToS + in-game chat moderation.
- Reportable infractions per DR-047.

## Accessibility Compliance

Per DR-012 + T-ACCESSIBILITY:
- WCAG 2.1 AA targeted for UI surfaces.
- Caption support per ADA / EU Accessibility Act.

## Content Rating Disclosures

- Loot boxes: NONE (per DR-031 + DR-047).
- Gambling mechanics: NONE.
- In-app purchases: NONE at launch.
- Online interactions: yes.
- User-generated content: yes (Workshop).
- Disclosed on Steam page.

## Done-Criteria

- [ ] Trademark filed (US + EU).
- [ ] LLC formed.
- [ ] Bank + Stripe set up.
- [ ] EULA/ToS/PP drafted by counsel.
- [ ] Age ratings submitted + approved.
- [ ] Attribution screen auto-generated + linked from main menu.
- [ ] AI-asset usage-ledger audit passed.
- [ ] Privacy policy auto-generated from event definitions.
- [ ] DPAs signed with all third-party services.

## Source Trail

- [[decisions/dr-047-launch-and-live-operations]]
- [[decisions/dr-031-content-economy-and-monetization-posture]]
- [[decisions/dr-010-license-reuse-matrix]]
- cargo-about: https://github.com/EmbarkStudios/cargo-about
- WCAG 2.1: https://www.w3.org/TR/WCAG21/
