---
type: decision
id: DR-051
status: closed-direction
priority: P0
closed_at: 2026-05-06
revisit_trigger: "Accessibility-plus features impact perf budget; sustainability plan triggered (dev moves to next project); console cert blocks; sales ops becomes operational burden; performance polish degrades under launch load."
---

← [[decisions/index|decision records]] · [[dashboards/decision-tracker|decision tracker]] · [[spec/accessibility-plus-and-sustainability|accessibility+/sustainability spec]] · [[spec/post-launch-operations-and-platform|post-launch ops/platform spec]] · [[decisions/dr-012-accessibility-comfort-readability|DR-012]] · [[decisions/dr-025-target-platforms|DR-025]] · [[decisions/dr-031-content-economy-and-monetization-posture|DR-031]] · [[decisions/dr-047-launch-and-live-operations|DR-047]]

# DR-051: Accessibility-Plus, Sustainability, Customer Support, Platform Extensions, Performance Polish

> [!success] Status: CLOSED-DIRECTION (project owner committed 2026-05-06)
> Bundles four launch-polish areas: (1) accessibility-plus extensions beyond DR-012 floor (cognitive + motor + hearing + reading + sensory + ALL-audio captions); (2) sustainability + sunset planning + 5-year content plan + open-source path; (3) customer support + sales operations + Stripe direct-sales + refund + tax + bundle strategy + affiliate program + ARG + OST/art-book/wiki + bug bounty; (4) console + cloud-gaming + TV-friendly UI + performance polish (network indicator, auto-quality-presets, save backup chain, anti-cheat heuristics, server health dashboard, network simulator, crash-recovery flow).

## Decision

### Accessibility-plus extensions

| Category | Component |
|---|---|
| **Cognitive** | Lower stimulation mode (reduced VFX, slower pace, simpler UI, fewer simultaneous threats); 'simple HUD' preset; one-thing-at-a-time tutorial pacing; cognitive-load-reduction toggle. |
| **Motor** | Single-button play mode; gesture controls; eye tracking integration (Tobii); slow-mo / pause-during-input mode; one-handed mode; configurable hold-vs-toggle for every action. |
| **Hearing** | Sign language overlay for cinematics (community-authored ASL/BSL/etc.); visual sub-bass cues (screen pulse on bass thump); haptic feedback alternatives; full subtitle option (NOT just critical audio); audio description for visual events. |
| **Reading** | Dyslexic font option (OpenDyslexic); high-contrast text; reading speed control; per-paragraph TTS readout; large-print preset. |
| **Sensory** | Pause-on-window-loss; reduce-screen-shake (already in DR-012); low-violence mode (decals minimal; blood color black-white); sensory-overload prevention (fewer simultaneous VFX); anxiety-mode (slower combat cadence); confirmation prompts on irreversible actions. |
| **Color blind** | 8 protanope/deuteranope/tritanope/atypical/protocols; tested with actual color-blind testers per DR-012. |
| **Cinematic accessibility** | Audio description for cinematics (text+voice descriptions of visual events); skip-cinematic for low-bandwidth players. |

### Sustainability + sunset planning

| Component | Detail |
|---|---|
| **5-year content plan** | Y1: balance + cosmetics; Y2: 1-2 expansions (paid; never gates core); Y3: ranked PvP infrastructure + tournament season; Y4: console eval + post-launch DLC; Y5: open-source path evaluation + community handoff. |
| **Sunset plan** | If dev moves to next project: Workshop / community handoff; cf-server hosting infrastructure community-managed; engine + tooling open-sourced (per DR-001 ethical stance); content archive maintained on community mirror. |
| **Open-source path** | If commercial path fails OR after 5+ years: donate engine + content to community per Apache-2.0 / MIT; documentation handoff. |
| **Endless content guarantee** | Workshop + procedural generator MUST outlive first-party content; cf-server runs forever as community-hosted. |
| **Server hosting handoff** | Community can host MMO shards forever; cf-server free + open-source-able post-sunset. |
| **Content archival** | Replays + saves work after game-development ends. |
| **Documentation as legacy** | Every system documented; vault published as community wiki post-sunset. |
| **Revenue share for key contributors** | Post-launch DLC modders; fair % per DLC sold (negotiated per partner). |

### Customer support + sales operations

| Component | Detail |
|---|---|
| **Customer support workflow** | Ticket system (HelpScout / Zendesk free tier OR self-hosted GitLab issues); AI-first triage by category; escalation to project-owner; SLA: 24h response. |
| **Stripe direct-sales** | Alternative to Steam-only for itch.io / direct-download buyers. Project-owner controlled. |
| **Refund handling** | Steam handles 2hrs/14days; document custom policy for direct sales (15-day refund window; AI-triage). |
| **Tax handling** | US sales tax + EU VAT + per-region; Stripe Tax addon; quarterly filing. |
| **Pricing tier strategy** | Regional pricing (Steam handles); PWYW for itch.io demo; full-game $19.99-$24.99 USD launch. |
| **Sale calendar** | Steam Summer / Autumn / Winter Sales + per-region holidays + 1-yr anniversary. |
| **Bundle strategy** | Humble Bundle + indie bundles + cross-promotion bundles (with Cortex-likes / Soldat-likes / Liero-likes). |
| **Affiliate / creator program** | Revenue share for content creators (post-launch tier; not pre-launch); 5% revenue per attributed sale. |
| **Per-affiliate / creator key revenue tracking** | Steam refund-aware; affiliate codes unique per creator. |
| **Sales reports (post-launch monthly review)** | AI-driven summary; trend analysis; campaign ROI. |
| **Wishlist conversion telemetry** | Track per-channel; optimize spend (free spend = best; paid only if ROI proven). |

### Marketing + community extras (beyond DR-047)

| Component | Detail |
|---|---|
| **Pre-launch ARG (alternate reality game)** | Discord-driven; Reddit puzzles; in-world clues; narrative-extension; runs ~3-6mo pre-launch. |
| **OST release on Bandcamp / Spotify** | Post-launch monetization (separate from game purchase). 30+ tracks per [[spec/music-and-soundtrack]]. |
| **Art book release (digital + print)** | Pixel-art + concept art + commentary. POD via Lulu / Blurb. |
| **Comic / graphic novel tie-in** | Community-authored, official-curated; Webtoon / Tapas. |
| **Wiki integration** | Community wiki at https://corefall.wiki/ (or Fandom); auto-mirrored to in-game codex. |
| **Twitch dev streams** | Project-owner Affiliate; weekly long-form dev streams. |
| **Community Q&A / town halls** | Monthly Discord; project-owner present. |
| **Roadmap voting** | Community votes on next feature post-launch; non-binding influence on backlog. |
| **Bug bounty program** | Community catches critical issues for credit + game keys + emblem. |
| **Translator credits + Hall of Fame** | In-game credits screen + Discord pin; per-language coordinator. |
| **Schools / educational license** | If applicable; or just public-availability + free demo. |
| **Game-design-as-learning module** | Use Workshop + cf-asset-pipeline to teach game design; partner with educational orgs. |
| **Dev streams as content** | Auto-clip notable moments; YouTube secondary channel. |

### Console + cloud-gaming + TV-friendly evaluation (post-launch)

| Platform | Status |
|---|---|
| **Switch / Switch 2** | Post-launch evaluation; Switch 2 perf-budget compatible per DR-028; cert path. |
| **PS5** | Post-launch evaluation; cert path. |
| **Xbox Series** | Post-launch evaluation; cert path. |
| **Mac App Store** | Separate from Steam macOS build; post-launch. |
| **Linux native binary on Flathub / Snap / AppImage** | Post-launch packaging. |
| **Mobile / tablet** | Per DR-025 NO at launch; revisit post-launch with strict perf budget. |
| **Cloud gaming (GeForce Now / xCloud / etc.)** | Compatibility evaluation; minor changes if needed. |
| **Steam Big Picture / TV-friendly UI** | UI mode adaptation for couch-play; post-launch tier. |
| **Game streaming via Steam Link / Moonlight** | Native support via Steam Link; tested in M-PLAYTEST. |

### Performance polish

| Component | Detail |
|---|---|
| **Networking quality indicator** | In-lobby + in-match; ping bars + packet loss + jitter visible. |
| **Auto-quality-presets** | Based on hardware (GPU detection on first launch); suggest preset (Steam Deck / Low / Med / High / Ultra). |
| **Cold-load benchmark** | In CI; first-launch perf measurement. |
| **Memory leak detection** | In CI long-soak (24h+ run); report leaks. |
| **Replay-archive compression** | Smaller share size; codec tuning. |
| **Save backup chain** | Rolling 5 saves auto-archived; corruption detection; one-button restore. |
| **Anti-cheat heuristics per session** | Detect speed hacking, mod-tampering, ESP, replay-clip-fakery. Per-mode profile. |
| **Server health dashboard** | Community-hostable cf-server: shows shard load, mod conflicts, replay drift, perf counters. |
| **Network simulator** | For development testing; artificial latency / packet loss / jitter. |
| **Crash-recovery flow** | Auto-resume from latest snapshot on crash; offer 'revert to checkpoint'. |
| **Cold-resume from save** | Fast load time even with large saves. |
| **Streaming asset budget** | Budget per scenario; verified at M-PLAYTEST soak. |

## What This Locks In

| Spec Area | Implication |
|---|---|
| `cf-accessibility-plus` | Extended `cf-accessibility`; cognitive + motor + hearing + reading + sensory presets. |
| `cf-customer-support` | Ticket system integration. |
| `cf-stripe` | Direct-sales adapter; receipts + refunds. |
| `cf-platform-eval` | Plan + checklists for Switch/PS/Xbox/Mac/Linux Flathub. |
| `cf-perf-monitor` | Network indicator + auto-quality + cold-load + memory leak detection. |
| `cf-server-health-dashboard` | Community hostable; per-shard status. |
| `cf-network-sim` | Dev-tooling; simulated latency/loss. |
| `cf-crash-recovery` | Auto-snapshot + revert-to-checkpoint UX. |
| `cf-sustainability-plan` | RON manifest of 5-year content plan; community-versioned. |
| `cf-archive` | Replay + save format guaranteed forward-compatible. |
| Marketing extras | ARG, OST, art book, wiki, dev streams, Q&A, roadmap voting, bug bounty, translator credits — all tracked in `cf-marketing` extension. |

## What This Does NOT Lock

| Non-Commitment | Why |
|---|---|
| Console specific cert paths | Open. Per DR-025 desktop-first; post-launch evaluation per platform. |
| Specific affiliate revenue % | Open. Default 5%; can tune. |
| Sunset trigger | Open. Project-owner discretion based on financial + interest. |
| OST pricing | Open. Bandcamp default $5-10 album. |
| Art book pricing | Open. POD pricing model. |
| ARG specific puzzles | Open. Author closer to launch. |
| Wiki platform (Fandom vs custom) | Open. Default Fandom; custom if community pressure. |

## Why This Direction

| Driver | Detail |
|---|---|
| Accessibility long tail | DR-012 floor doesn't cover ~10% of disabled players (cognitive, motor, sensory). Closing that gap = wider audience + ethical floor. |
| Sustainability insurance | If commercial path fails, open-source path retains community trust + legacy. |
| Customer support fundamentals | Without a ticket system, customer support becomes project-owner inbox = unsustainable. |
| Platform extension | DR-025 says no consoles at launch; post-launch evaluation matters for long-tail revenue + console community. |
| Performance polish | Steam Deck Verified + Cloud Gaming + TV-friendly = wider audience + better reviews. |
| Marketing extras | ARG + OST + art book + wiki + community = long-tail engagement beyond launch month. |

## Why Not The Alternatives

| Alternative | Why Rejected |
|---|---|
| No accessibility-plus | Loses ~10% of player base; ethical floor incomplete. |
| No sunset planning | Project-owner moves on; community is left without infrastructure. |
| Manual customer support | Unsustainable solo-dev. |
| Steam-only sales | Loses GOG/itch.io audiences + flexibility. |
| Skipping ARG/OST/art book | Loses long-tail engagement + alternative revenue. |
| Centralized server-only | Per DR-005 community-hostable. |

## Evidence Trail

- Project owner verbatim (2026-05-06): "what is missing? where are the gaps? There has to be features and design ideas and stuff we are missing. find them all!"
- Tobii eye-tracking integration: viable for motor-accessibility per Microsoft Inclusive Design.
- WCAG 2.1 AAA: aspirational beyond DR-012 floor (AA targeted).
- Hades sustainability: ongoing community support 5+ years post-launch.
- Stardew Valley sustainability: 8+ years post-launch; active modding ecosystem.
- Captured in [[research-log/2026-05-07-comprehensive-audit-report]].

## Revisit Trigger

- Accessibility-plus features impact perf budget.
- Sustainability plan triggered (dev moves to next project).
- Console cert blocks.
- Sales ops becomes operational burden.
- Performance polish degrades under launch load.
