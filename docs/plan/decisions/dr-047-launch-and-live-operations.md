---
type: decision
id: DR-047
status: closed-direction
priority: P0
closed_at: 2026-05-06
revisit_trigger: "Launch posture changes (e.g., publisher onboarded); compliance/legal blocker emerges; community/streaming engagement < expectations; live-ops becomes content-economy treadmill counter to DR-031."
---

← [[decisions/index|decision records]] · [[dashboards/decision-tracker|decision tracker]] · [[spec/telemetry-and-bug-tooling|telemetry]] · [[spec/playtest-program|playtest]] · [[spec/marketing-and-launch|marketing]] · [[spec/steam-and-platform-integration|Steam]] · [[spec/legal-and-compliance|legal]] · [[spec/liveops-and-endgame|liveops/endgame]] · [[spec/streaming-and-creator-features|streaming]]

# DR-047: Launch & Live Operations Direction

> [!success] Status: CLOSED-DIRECTION (project owner committed 2026-05-06)
> Bundles the launch posture: telemetry + crash reporting + bug tool + playtest cohort program + marketing + Steam Workshop/Achievements/Cloud/Deck Verified + EOS optional + legal/age-rating/privacy + ethical live-ops foundation (cosmetics post-launch, NEVER pay-to-win) + endgame/replayability + streaming/creator features. AI-driven where possible (telemetry analysis, marketing copy, social media scheduling, balance hot-patch decisions).

## Decision

### Telemetry, crash reporting, bug tool

| Component | Detail |
|---|---|
| **Crash reporting** | Sentry (free tier up to 5K events/month) OR self-hosted GlitchTip. Symbolicated stack traces. Auto-upload with consent prompt. |
| **Anonymous gameplay telemetry** | Opt-in (default off in EU per GDPR; default on elsewhere with clear privacy notice). Captures: scenario id, mission outcome, time-to-death, weapon-of-death, faction picked, mods loaded, hardware specs, crash signatures. NEVER captures: chat content, player names, inputs, file paths. |
| **In-game bug tool** | Press F12 → screenshot + last-30s replay snapshot + run-bundle attached + user description prompt → uploads to GitHub Issues / Sentry / dedicated bug-server. Privacy-cleaned. |
| **Performance telemetry** | Frame ms / sim ms / dropped events / GPU memory / load times. Aggregated. |
| **Balance telemetry** | TTK matrix per weapon/chassis combo. Per-faction win-rate. Per-mission completion-rate. Per-mode dropout-rate. Drives M-BALANCE post-launch hotfix decisions. |
| **AI-driven analysis** | Weekly auto-report by AI agent: anomaly detection (sudden spike in crashes, balance outliers, regression candidates), summary email, prioritized backlog suggestion. |

### Playtest cohort program

| Phase | Detail |
|---|---|
| **Closed alpha** | M7 acceptance + M9 server core. ~20-50 invited testers via Discord. NDA-light. Daily build cycle. AI agents auto-generate playtest reports + bug summaries. |
| **Closed beta** | M10 LAN co-op + M11 online co-op. ~200-500 testers. Steam closed beta build. Weekly cycle. |
| **Open beta / Steam Next Fest demo** | M12 public PvP + MMO shard live. ~5-50K wishlist conversions. Public demo, time-limited (~30-60 min play). |
| **Soak testing** | Multiplayer netcode 24h soak; MMO shard 7-day soak; replay-determinism 100K-tick soak; AI mission director 24h chaos run. AI-driven assertion harness. |
| **Live playtest events** | Monthly community Discord playtests post-beta, with project-owner present. |
| **AI-simulated playtests** | AI agent runs 1000s of scripted scenarios per night to surface balance outliers, AI-bot regressions, replay drift, perf regressions. Reports fed to weekly review. |

### Marketing & launch

| Asset | Detail |
|---|---|
| **Steam page** | Launched 6-12 months pre-release. Title art, capsule art, screenshots (10+), trailer (60-90s), description copy, key features (8 bullets), system requirements, languages (locked from DR-046). All AI-generated/AI-authored. |
| **Trailer** | 60-90s reveal trailer (Stable Video Diffusion + AnimateDiff for clips + AI-composed score via Suno) + 30s gameplay trailer + 60s "what is Corefall?" trailer. Localized subtitles. |
| **Press kit** | Logo (multiple formats), screenshots (high-res), key art (4K), 3 trailers, 1-pager fact sheet, contact info, demo build link. presskit() format (Vlambeer's tool). |
| **Demo build** | 30-60 min slice for Steam Next Fest. Bunker Defence flagship + 1 onboarding mission + 1 lab + 4-player coop unlocked. Wishlist drive CTA. Per [[spec/marketing-and-launch]]. |
| **Wishlist drive** | Pre-launch goals: 10K wishlists at 6mo before launch, 50K at launch. Reddit (r/IndieGaming, r/CortexCommand, r/games), TikTok devlogs, Twitter, Bluesky, YouTube devlogs, IndieDB, itch.io. AI-generated daily devlog + social posts (project-owner reviewed). |
| **Launch trailer** | Day-of-launch 90-120s cinematic. Pinned. |
| **Discord** | Pre-launched at Steam-page launch. Channels: announcements, playtest, mod-creators, language-X (per Tier-A locale), bug-reports, fan-art, screenshots, support. AI-moderated baseline + community moderators. |
| **Press outreach** | Tier-1 (RPS, PC Gamer, Eurogamer, Kotaku, IGN-indie) at demo + launch; Tier-2 (regional) at launch; Tier-3 (YouTubers, TikTok creators, Twitch streamers) at demo with creator-keys. AI-generated personalized outreach emails. |

### Steam + platform integration

| Feature | Detail |
|---|---|
| **Steam Workshop** | Mod packages publishable from in-game. Community can subscribe + auto-install. Per DR-006 + DR-059. Trust tiers gate. |
| **Steam Achievements** | 60-100 achievements. Most are "play 1 of each chassis" / "complete each mission" type; ~10 hidden / lore / mastery achievements. |
| **Steam Cloud** | Saves + replay archive auto-sync. Encrypted. Per DR-029. |
| **Steam Friends + Invites** | Friend list, party invite to lobby, presence ("In Bunker Defence — Mars"). |
| **Steam Input** | Full controller/gamepad/Steam Deck support; community bindings sharable. |
| **Steam Deck Verified** | Target Verified rating: 800p/60 perf, controller-complete, readable text, no shader compilation hitches. |
| **Steam Trading Cards** | Non-monetized cosmetic. Earned via play. |
| **Steam Remote Play Together** | LAN co-op via Steam Remote Play (free). |
| **EOS adapter (optional)** | Cargo feature for Epic Games Store / cross-platform friends. Off by default; build flag. |
| **GOG.com** | Post-launch tier. Same binary; different DRM stripped (already DRM-free). |
| **Itch.io** | Demo + mod-friendly version. Same binary. |

### Legal & compliance

| Item | Detail |
|---|---|
| **Trademark search + registration** | "Corefall" + logo USPTO + EUIPO. Pre-launch (M-MARKETING phase). |
| **Domain** | corefall.com / corefall.gg / corefall.dev. Pre-purchased. |
| **Business entity** | LLC (US, Wyoming or Delaware), bank account, Stripe for direct sales. |
| **EULA + ToS + Privacy Policy** | Drafted by legal counsel (~$2-5K) covering: gameplay license, modding rights, data collection (GDPR + CCPA + LGPD), Workshop content rights, dispute resolution, age requirement (13+ COPPA, 16+ for full features). |
| **Age rating** | ESRB (Mature 17+ likely; violence + blood). PEGI (16-18). USK (DE; 16). CERO (JP; D 17+). Submission via IARC self-rating + ESRB cert. |
| **Privacy / GDPR / CCPA / LGPD** | Cookie/data prompts. Right-to-deletion. Privacy-by-default in EU. Data Processing Agreement with Sentry/Steam/etc. |
| **Open-source attribution** | In-game credits screen lists every OSS dependency + license. Auto-generated from `Cargo.lock` via `cargo-about`. |
| **Music + asset licensing** | Every Tier-2 AI-generated asset logged in usage-ledger with prompt+seed+model+license. Tier-3 hand-polish doesn't change licensing. Suno/Udio music subject to TOS review pre-launch (commercial use allowed currently; revisit). |
| **Modding rights** | Modders retain copyright on their mod content. License them to other players via Workshop (CC-BY-SA default; modder can pick). |
| **Anti-harassment + Code of Conduct** | Discord ToS + in-game chat moderation. Reportable infractions. |
| **Accessibility compliance** | Per DR-012 + T-ACCESSIBILITY. WCAG 2.1 AA targeted for UI. Caption support per ADA / EU Accessibility Act. |
| **Content rating disclosures** | Loot boxes: NONE (per DR-031). Gambling mechanics: NONE. In-app purchases: NONE at launch. Online interactions: yes. User-generated content: yes (Workshop). Disclosed on store page. |

### Live-ops foundation (post-launch)

Per DR-031 (no pay-to-win, no gacha, no marketplace cut). Live-ops at launch is **infrastructure only**, not content-economy treadmill.

| Component | Detail |
|---|---|
| **Cosmetics pipeline** | Skins, decals, paint jobs, voice packs, emblems, victory poses. Earned via play (achievements, mastery, mission completion, replay shares). NEVER paid. NEVER gacha. |
| **DLC infrastructure** | Optional paid expansions post-launch (new factions, campaign chapters, new worlds). NEVER core mechanics. NEVER pay-to-win. |
| **Balance hot-patch** | Post-launch balance changes via signed content patch. ~Quarterly. Driven by telemetry + community feedback. |
| **Content updates** | Quarterly: new missions, new mods spotlighted, new community challenges, new launch-tier-extension factions/weapons. NEVER paid for v1.0 owners. |
| **Seasonal events (optional)** | Holiday events (Halloween scenario, winter scenario) for community engagement. Time-limited. NO FOMO mechanics (ephemeral cosmetics OK if also obtainable later). |
| **Community challenges** | Weekly leaderboard challenges: speedrun a mission, survive Bunker Defence, build a base. Replay-share-driven. AI-judge-assisted. |
| **Mod-creator support** | Featured mods, mod-spotlight in launcher, modder credits on official channels. |

### Endgame / replayability

| Component | Detail |
|---|---|
| **Procedural contracts** | Per DR-017, post-campaign players have endless procedural mission generator. AI-driven mission director assembles seed + objectives + faction + world + weather + comms policy. |
| **Persistent veterans** | Survive enough missions, your operative becomes "named." Memorialized in codex. Per DR-018 + DR-011. |
| **Mastery progression** | Per-chassis / per-faction / per-weapon mastery rank (1-30). Unlocks variants, paint, voice lines, lore entries. Intrinsic, no power. |
| **Bunker building meta** | Per DR-027. Players carry over bunker designs across runs. Veteran bunkers in Hall of Fame. |
| **PvP ranked (post-launch)** | Per DR-005, ranked PvP arenas post-launch with seasonal resets. NEVER pay-to-rank. |
| **Persistent MMO shards** | Per DR-035. Long-running shards for community-hosted persistent play. |
| **Speedrun.com integration** | Replay-archive verified speedruns. Anti-cheat foundation supports. |
| **Daily/weekly mission seeds** | Same seed for all players that day. Leaderboard. Replay share. |
| **Steam Workshop endless content** | Modder-published missions, factions, chassis, scenarios. Curated by community + featured by official. |

### Streaming / creator features

| Component | Detail |
|---|---|
| **Replay viewer with streamer mode** | Hide enemy positions for delayed streams. |
| **Photo mode** | Per spec/shell-ui-architecture. Free camera, freeze sim, filters. |
| **Spectator mode** | Per DR-005 + M9. Multi-POV, free camera, replay-scrub during live match. |
| **Highlight reel** | Auto-detect interesting moments (kills, narrow escapes, base breaches) → 5-15s clip with auto-edit. Shareable as MP4. |
| **OBS overlay** | Streamer companion app: live match state for stream overlay. Per-streamer customizable. |
| **Twitch integration** | Twitch chat → in-game commands during streamer's matches (vote on AI doctrine, vote on next match seed). Optional. |
| **Replay sharing** | Upload replay to community server (cf-server hosts the replay-share endpoint). Sharable link, embeds. |
| **Press / influencer keys** | Time-limited Steam keys for press + content creators pre-launch. AI-managed CRM (one-button issue + revoke + track-coverage). |

## What This Locks In

| Spec Area | Implication |
|---|---|
| `cf-telemetry` | Crash + anonymous gameplay telemetry. Sentry/GlitchTip integration. |
| `cf-bug-tool` | F12 in-game bug reporter. |
| `cf-marketing` | Steam page asset pipeline (AI-generated copy, screenshots, trailer cuts). |
| `cf-steam` | Steam Workshop / Achievements / Cloud / Friends / Input / Deck adapters. |
| `cf-legal` | Auto-generated credits, license attribution from `Cargo.lock`. |
| `cf-liveops` | Cosmetics + DLC infrastructure + balance hot-patch + seasonal event hooks. |
| `cf-creator-tools` | Photo mode + replay sharing + highlight reel + streamer overlays. |
| Marketing budget | Ad-spend = $0 baseline (organic + community + Steam algorithm). Festival fees (Steam Next Fest free; PAX/GDC paid) budgeted separately. |

## What This Does NOT Lock

| Non-Commitment | Why |
|---|---|
| Whether to use a publisher | Open. Solo-indie default; consider partnership for marketing-only deal post-Next Fest. |
| Specific console ports | Open. Per DR-025 desktop-first; Switch/PS/Xbox post-launch evaluation. |
| Pricing | Open. Likely $24.99 USD launch tier per indie market median 2026. |
| Whether to do paid DLC | Open. v1.0 is complete game; paid expansions post-launch are evaluated based on community demand + financial viability. |
| Streaming exclusivity deals | Forbidden. Game is free to stream. |

## Why This Direction

| Driver | Detail |
|---|---|
| Solo-indie reality | Publisher overhead unjustified at this stage. Direct-to-Steam + community-driven launch is proven path (Vampire Survivors, Dwarf Fortress, Stardew Valley). |
| AI-augmented launch | AI agents handle marketing copy, social posts, press outreach drafting, telemetry analysis, bug triage, balance review. Solo-dev still in control of strategic decisions. |
| Ethical live-ops | DR-031 forbids gacha/p2w/marketplace cut. Community-first cosmetics + free updates + Workshop-driven content is the alternative model. |
| Streaming-friendly | Indie game discoverability in 2026 is heavily streaming-driven. Photo mode + replay sharing + Twitch integration + creator support = viral surface. |
| Mod-first ecosystem | Per DR-006 + DR-045, modding parity is a launch promise. Workshop integration + mod-spotlight + creator credits makes modders feel valued. |

## Why Not The Alternatives

| Alternative | Why Rejected |
|---|---|
| Publisher-led launch | Loses creative control + revenue split + deadline pressure. Maybe future sequel/console port. |
| Paid early-access | Considered. Risk of feeling "incomplete" + refund risk. Demo + Next Fest + closed-beta pipeline is safer. |
| Console launch day-1 | Engineering cost + cert overhead + niche genre risk. Post-launch evaluation. |
| Subscription / live-service | Forbidden by DR-031. Premium one-time + free-modding is locked. |
| No telemetry | Loss of balance + crash + perf signal. Can't ship updates blindly. Privacy-by-default opt-in is the compromise. |
| No Workshop | Loses modding ecosystem. Workshop is the de-facto distribution channel for Steam mods. |

## Evidence Trail

- Steam Next Fest 2026 prep checklist: https://gamineai.com/blog/steam-next-fest-2026-prep-checklist-indie-devs-wishlists-demo-press
- presskit() (Vlambeer): https://dopresskit.com/
- Sentry pricing: https://sentry.io/pricing/
- bevy_steamworks: https://docs.rs/bevy-steamworks
- Steam Workshop Implementation Guide: https://partner.steamgames.com/doc/features/workshop/implementation
- Project Fluent: https://projectfluent.org/
- WCAG 2.1 AA: https://www.w3.org/TR/WCAG21/
- Captured in [[research-log/2026-05-06-ai-driven-asset-pipeline-research]].

## Revisit Trigger

- Launch posture changes (publisher onboarded).
- Compliance/legal blocker emerges (e.g., loot-box law adopted in major market).
- Community/streaming engagement < expectations (need to revise marketing).
- Live-ops becomes content-economy treadmill counter to DR-031.
- Sentry / Steam Workshop / EOS becomes commercially unviable.
