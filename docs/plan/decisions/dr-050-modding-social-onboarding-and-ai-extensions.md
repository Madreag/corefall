---
type: decision
id: DR-050
status: closed-direction
priority: P0
closed_at: 2026-05-06
revisit_trigger: "Modder count drops; social features create harassment vectors; new-player retention <40% past hour 1; AI training mode for modders proves too complex; AI-vs-AI tournaments fail to attract participants."
---

← [[decisions/index|decision records]] · [[dashboards/decision-tracker|decision tracker]] · [[spec/modding-ecosystem-extensions|modding ext spec]] · [[spec/social-and-onboarding-extensions|social/onboarding ext spec]] · [[decisions/dr-006-modding-data-model|DR-006]] · [[decisions/dr-008-ai-architecture|DR-008]] · [[decisions/dr-022-ai-humanlike-bar|DR-022]] · [[decisions/dr-023-tutorial-and-onboarding-strategy|DR-023]] · [[decisions/dr-031-content-economy-and-monetization-posture|DR-031]] · [[decisions/dr-046-player-facing-surfaces-direction|DR-046]]

# DR-050: Modding Ecosystem Extensions, Social Features, Onboarding-Plus, AI Quality Extensions

> [!success] Status: CLOSED-DIRECTION (project owner committed 2026-05-06)
> Locks: (1) modding extensions (versioning, conflict detection, analytics, voluntary tip jar, mod-of-the-week curation, modder collab tools, AI-driven mod test runs); (2) social features (guilds, in-game messaging, co-op campaign saves, voice party, gifting, mentor matching); (3) new-player onboarding-plus (mentor system, beginner matchmaking, first-30-min telemetry deep-dive, adaptive difficulty, demo→full carry-over); (4) AI quality extensions (difficulty visibility, faction personality, AI training mode, AI-vs-AI tournaments, transparency mode, play-as-Husk).

## Decision

### Modding ecosystem extensions

| Component | Detail |
|---|---|
| **Mod versioning + dependency management** | `mod_manifest.requires_version`, `mod_manifest.depends_on: [mod_id, version_range]`. Auto-resolve dependency graph. Block load on conflict. |
| **Mod conflict detection** | Per-asset override conflicts; load-order ranking; auto-resolve via priority + manual override. |
| **Mod-creator analytics (opt-in)** | Per-mod: usage rate, mission success rate, crash signature, average play time, top-conflicting mods. Privacy-by-default; modder opts in. |
| **Voluntary tip jar** | Modder-set tip URL via Stripe / Ko-Fi / Patreon. Project takes 0% cut per DR-031. Discoverable via mod-page in launcher. |
| **Mod-of-the-week curation** | Project-owner curated; rotating featured mod in main menu launcher banner; criteria public; community nominations welcome. |
| **Mod-private cloud** | Modder hosts pre-release content for friends-only access (e.g., paid playtest builds). |
| **Mod-test-run AI agents** | Modder submits chassis → AI agent generates test scenarios for it; auto-runs balance + AI behavior validation; reports issues. |
| **Mod compatibility-with-base-version warnings** | Auto-migrate to current version OR block with clear diagnostic. |
| **Mod-author-controlled localization** | Mod author submits per-locale `.ftl` packs per [[spec/localization-plan]]. |
| **Mod SDK auto-docs** | Generated from Rust trait impls; published to dedicated docs site. |
| **Modder collab tools** | In-Discord modder rooms; shared package projects; code-review tooling for mod-package PRs. |
| **Mod conflict resolution UI** | In-game UI for resolving conflicts: per-mod priority, per-asset override, "use this from mod X but stats from mod Y" granular control. |
| **Mod showcase events** | Monthly community + project-owner curated; featured in launcher + Discord + Reddit. |
| **Auto-update + rollback** | Mod auto-update via Workshop with rollback if breakage detected by mod-test-run AI agent. |

### Social features

| Component | Detail |
|---|---|
| **Guild / clan system** | 8-50 player groups; shared base designs; clan-vs-clan PvP; guild profile + emblem + member roster. |
| **In-game messaging beyond match lobby** | Offline DM + friends list with chat; channels per friend group; mod-discussion channels. |
| **Co-op campaign saves** | Bring friends through campaign together; 4-player party persists across sessions; per-player progress + party-state. |
| **Cross-shard friends list** | MMO mode: find friends across persistent shards; friend status visible across shards. |
| **Voice party** | Discord-style party voice independent of in-match voice; pre-match strategy. Steam/EOS adapter. |
| **Player-to-player gifting** | Gift game copy (Steam handles); gift cosmetics; tip jar (modder + creator). NEVER cash to player; per DR-031 no marketplace cut. |
| **Mission-share invite** | Steam-friend pop-up: "come join my Bunker Defence run"; one-button join. |
| **Guild-managed bunker designs** | Community-authored bases for guild defense missions; voted by members. |
| **Cross-Workshop coordination** | Modder collab tools (above); inter-mod dependencies; collaborative mod packages. |

### New-player onboarding-plus

| Component | Detail |
|---|---|
| **Mentor system** | Veteran players (mastery 20+) opt-in to mentor; new players (first 5 hours) auto-matched; mentor sees mentee's playtime + chooses to invite to mission/training; reward both with cosmetic emblem + leaderboard. |
| **Beginner matchmaking pool** | Beginner-only games for first N hours (default 10); separate matchmaking queue. |
| **First-30-minutes telemetry deep-dive** | Per-second drop-off detection; per-action timing; session abandonment cause analysis (where did they quit?). AI agent generates weekly report. |
| **Adaptive difficulty per session** | Auto-adjust if struggling (more hints, lower enemy aggression, slower time scale). Opt-in. |
| **Demo → full game carry-over** | Achievements, saves, cosmetics carry over. Verified in M-STEAM. |
| **Tip-of-the-day on launch screen** | AI-rotated; 50+ tips initial roster; modder-extensible. |
| **Adaptive hints** | Already in DR-046 / [[spec/tutorial-implementation]]; this DR adds: per-session learning rate detection. |
| **First-time player guide PDF** | Auto-generated from tooltip + tutorial data; downloadable from Steam page. |
| **Onboarding mission narrative hook** | Add explicit "why am I doing this" hook to "First Contract" mission; tested in playtest. |
| **Cultural/locale-aware hint pacing** | Hint frequency adapts to per-locale norms (Asian mobile-style ≠ Western indie). |

### AI quality extensions (per DR-008 + DR-022)

| Component | Detail |
|---|---|
| **AI difficulty visibility** | Named AI presets ("Cakewalk", "Tough Crowd", "Veteran", "Nightmare", "Demonic"). Visible to player; matches feel intentional. |
| **Faction AI personality identifiability** | At hour 5, player should be able to tell "that's Browncoat doctrine" from behavior alone. Per-faction style flag in `cf-ai`. |
| **AI mistake narration** | Debrief: "the enemy commander made a tactical blunder when X" with replay-scrub link. Drives narrative payoff. |
| **AI training mode for modders** | Modder runs scenarios against AI; AI learns + adapts within strict bounds; submits doctrine tweaks. |
| **AI-vs-AI tournament mode** | Community submits AI doctrines; tournaments run server-side; results cached; community votes; replay broadcasts. |
| **AI transparency mode** | Show AI reason labels live in HUD (opt-in setting); per DR-022 humanlike bar. |
| **Play-as-Husk mode** | Player controls antagonist faction in custom scenarios; PvE-vs-AI variant; reverses normal asymmetry. |
| **AI personality voice variety per origin** | Humans = different voice families (regional accents); robots = synth varieties. AI-generated via ElevenLabs / XTTS. |
| **AI bot-loadout-hostility** | AI bots can refuse to use specific equipment per DR-008 + DR-022 (refusal reasons surfaced); modders can add new refusal predicates. |

## What This Locks In

| Spec Area | Implication |
|---|---|
| `cf-mod` | Extended with versioning, dependency, conflict detection, analytics. |
| `cf-mod-tip-jar` | New crate or extension; modder tip URL aggregation. |
| `cf-mod-test-runner` | New crate; AI agent runs scenarios against mod content. |
| `cf-social` | New crate; guilds + messaging + cross-shard friends + voice party + gifting. |
| `cf-onboarding` | New crate; mentor matching + beginner pool + first-30-min telemetry + adaptive hints + tips of the day. |
| `cf-ai-extensions` | Extended `cf-ai`; difficulty presets + faction personality + transparency + play-as-Husk + AI tournament submission. |
| `cf-mentor` | Sub-system; mentor opt-in registry + auto-match + reward tracking. |

## What This Does NOT Lock

| Non-Commitment | Why |
|---|---|
| Guild size cap | Open. Default 8-50; tunable. |
| Voice party platform (Steam vs EOS vs custom) | Open. Default Steam adapter; EOS optional. |
| Specific AI difficulty preset numbers | Open. Default 5 presets (Cakewalk, Tough Crowd, Veteran, Nightmare, Demonic). |
| AI tournament cadence | Open. Default monthly. |
| Mentor matching algorithm | Open. Default skill-similarity + region + language. |

## Why This Direction

| Driver | Detail |
|---|---|
| Modder retention | Versioning + analytics + tip jar = modders feel valued + supported = ecosystem alive long-term. |
| Solo-player onboarding | Mentor + beginner pool + adaptive difficulty = lower churn at hour 1-5. |
| Social glue | Guilds + co-op campaign saves + voice party = players bring friends; 5-player retention vs 1-player retention. |
| AI transparency | Per DR-022 humanlike bar requires AI to be inspectable; transparency mode + faction personality + difficulty visibility close the gap. |
| Anti-toxicity | Mentor system + beginner pool + reportable infractions = healthier community; per anti-harassment per DR-047. |

## Why Not The Alternatives

| Alternative | Why Rejected |
|---|---|
| Modder revenue sharing via project-cut | Forbidden by DR-031 marketplace cut. Voluntary tip jar is the workaround. |
| Hard guild commitment (mandatory) | Solo-first per DR-015; guilds are optional. |
| No mentor system | Loses early-game retention. |
| Hidden AI difficulty | Player feels arbitrary; intentional difficulty matters per Souls precedent. |
| AI black-box | Per DR-022 humanlike bar requires inspectability. |

## Evidence Trail

- Project owner verbatim (2026-05-06): "what would keep players from playing the game after going through the entire roadmap?"
- Helldivers 2 mentor / community-driven onboarding: 60%+ first-month retention.
- Path of Exile mod ecosystem: voluntary tip jars + modder credits = active ecosystem 10+ years.
- Souls / Elden Ring AI difficulty: named presets + intentional-feeling difficulty = mass-appeal hard mode.
- Captured in [[research-log/2026-05-06-second-pass-audit-followup]] (TBD).

## Revisit Trigger

- Modder count drops below threshold (mod activity / month).
- Social features create harassment vectors (in-game DM stalking).
- New-player retention <40% past hour 1.
- AI training mode for modders proves too complex.
- AI-vs-AI tournaments fail to attract participants.
