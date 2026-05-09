---
type: decision
id: DR-046
status: closed-direction
priority: P0
closed_at: 2026-05-06
revisit_trigger: "Shell UI playtest reveals first-30-seconds friction; tutorial completion <70%; localization adds release-blocking complexity; narrative copy fails to land tone."
---

← [[decisions/index|decision records]] · [[dashboards/decision-tracker|decision tracker]] · [[spec/shell-ui-architecture|shell UI architecture]] · [[spec/tutorial-implementation|tutorial implementation]] · [[spec/narrative-bible|narrative bible]] · [[spec/localization-plan|localization plan]] · [[decisions/dr-019-visual-direction|DR-019]] · [[decisions/dr-023-tutorial-and-onboarding-strategy|DR-023]]

# DR-046: Player-Facing Surfaces — Shell UI, Tutorial, Narrative, Localization

> [!success] Status: CLOSED-DIRECTION (project owner committed 2026-05-06)
> Player-facing surfaces are first-class launch artifacts: title screen + main menu + pause menu + settings tree + lobby + workbench + debrief + map + achievements + tutorial + lab launcher + narrative comic panels + localization (10+ languages at launch via AI translation + community-localizable). Comic-noir presentation per DR-019. Flashy + punchy — animated transitions, screen shake, juicy feedback, satisfying click/hover/select sounds.

## Decision

### Shell UI architecture

| Surface | Done When | Notes |
|---|---|---|
| **Title screen + splash** | Animated logo (parallax pixel + comic-noir overlay), "press start," version badge, AI-generated cinematic background loop (AnimateDiff 4-6s). | First impression. Must feel "this is a real game." |
| **Main menu** | Campaign / Skirmish / Multiplayer / Workshop / Workbench / Tutorial / Lab Launcher / Settings / Credits / Quit. Animated transitions; hover SFX; selected-glow VFX. | Hub. Comic-panel layout. |
| **Pause menu** | Resume / Save / Load / Settings / Restart Mission / Quit to Menu / Quit to Desktop. ESC opens; pauses sim deterministically. | Always accessible. |
| **Settings menu** | Graphics / Audio / Controls / Accessibility / Gameplay / Language / Online. Each tab fully functional with cfctl parity per T-CONTROL. | Per spec/shell-ui-architecture full spec. |
| **Server browser** | List + filter (mode, region, ping, mods, slots) + favorites + history + direct-IP join. Steam/EOS adapters optional. | Per DR-005 + DR-034. |
| **Lobby** | Pre-match: team config, faction pick, loadout pick, ready-up, chat, vote-kick, host migration. Mid-match: spectator slot. | Per DR-042 match grammar. |
| **Loadout workbench** | Full drag/drop UI per [[spec/equipment-loadout-workbench-slice-a]]. AI preview ("Bot will refuse this if X..."). Capability strip. Diff vs preset. Hot-swap. Save loadout presets. | M5+ |
| **Mission briefing** | Comic-panel cards with AI-generated panel art (SDXL+ControlNet from mission manifest data) + voice-over caption + objective list + faction context + LZ risk preview. | Per DR-017 + DR-019. |
| **Mission debrief** | Comic-panel timeline of what happened ("show me why" handoff to replay). Death recap. Salvage summary. Veteran injuries. Replay CTA. Share button. | Per DR-018 + DR-023. |
| **Strategic map / world view** | Multi-world astrography (per DR-039) + per-world mission selector + faction state + comms light-lag visualization + ore deposit map + weather forecast. | Per DR-039 + DR-040. |
| **Achievements + collection** | Per-achievement comic-panel reveal. Collection of unlocked chassis variants, weapon skins, faction emblems, replay highlights, named-NPC encounters. | Per DR-031 cosmetics post-launch. |
| **Replay viewer** | Scrub + speed control + multi-camera (player POV / commander map / first-person) + bookmark + clip export + shareable link. | Per DR-002 + DR-024. |
| **Codex / lore browser** | In-game encyclopedia: factions, worlds, characters, weapons, materials. AI-generated lore copy + flavor text + comic-panel snapshots. Unlocked via play. | Replayability + worldbuilding payoff. |
| **Photo mode** | Free camera + freeze sim + filter presets (comic / noir / pixel-pure / dramatic-light) + screenshot export with credits stamp. | Streaming/creator support per DR-047 + [[spec/streaming-and-creator-features]]. |
| **Cosmetic locker** | Unlocked skins, decals, paint jobs, voice packs, victory poses, emblems. Earned via play, never paid. Cosmetics post-launch. | Per DR-031. |
| **Death cam** | Auto-replay last 5s on death from killer POV; "show me why" handoff. | Per DR-023. |
| **Mod manager** | Browse Workshop / Local mods. Subscribe / install / update / uninstall. Trust tiers per DR-034. Hot-load. | Per DR-006 + DR-050. |

### Flashy + punchy juice rules

| Surface | Juice rule |
|---|---|
| Every button hover | Scale 1.0 → 1.05 over 80ms ease-out + glow halo + soft tick SFX (200Hz square 30ms). |
| Every button click | Scale punch 1.0 → 0.95 → 1.0 over 120ms + brighter flash (8-frame VFX) + click SFX (mid-frequency punch + sub-bass thump). |
| Menu transitions | Comic-panel slide-in from edge + slight skew + 200ms ease-in-out + ambient mix duck. |
| Match start | Drop-in dropship animation (4s cinematic) + camera drift + LZ flash + objective banner unfurl from comic-panel reveal. |
| Match victory/defeat | Comic-page-flip transition + slow-mo final frame + adaptive music swell + confetti VFX (victory) / scroll-of-failure (defeat). |
| Damage taken | Screen shake (magnitude scaled by impulse) + chromatic aberration brief + red vignette + heartbeat-bass sub-frequency. |
| Critical hit | Time freeze 80ms + flash white + bass thump + camera punch toward target. |
| Reload | Magazine swap animation + shell-eject SFX + chamber-click SFX + UI ammo counter punch. |
| Death | Slow-motion 0.3s + camera dolly-in + "show me why" prompt button. |
| Achievement unlock | Comic-panel pop-in from corner + cheer sting + shared collection update. |
| Settings change | Soft confirmation tick + animated value snap + savestate flash. |

### Tutorial implementation (closes DR-023 OPEN)

| Component | Scope | Done When |
|---|---|---|
| **Onboarding mission** | "First Contract" — 12-15 min cinematic mission. Direct-control body, fire weapon, dig/breach, command 1 AI teammate, call dropship, rescue wounded, see replay/debrief. AI-generated voice-over via ElevenLabs (consider TOS; fallback to text-only). | Player completes "First Contract" without external help in 80%+ playtest sessions. |
| **8 modular labs** | Per DR-023: Movement/Aim, Terrain/Materials, Loadout/Delivery, Squad Orders/AI, Command Core/Base, Avatar Mode, Chassis Damage, Replay/Debrief. ~2 min each. | Each lab passes ACC-A acceptance + completion telemetry > 60%. |
| **Contextual tooltips** | Per-tooltip use counter + mastery flag. Fade after 3 uses. Re-enable via settings. | All tutorial-relevant UI elements have tooltip data. |
| **"Show me why" handoff** | Every failure (death, mission loss, command-core lost, mech wrecked) opens "show me why" → replay viewer auto-scrubbed to cause + relevant lab launcher. | Triggered correctly across all 12 failure modes. |
| **Difficulty / accessibility presets** | Standard, Easy, Hard, Custom (sliders for damage taken/dealt, AI aggression, time scale, hint frequency). | Each preset playtested. |
| **Adaptive hints** | Hint engine reads `EnvironmentSignal` + AI bot scoring + player input patterns to surface hints. "Press G to throw grenade — enemy is in cover" type. Suppressible. | 95% accuracy in playtest cohort. |
| **AI-authored mission narrative** | Mission briefing/debrief copy generated by Claude Sonnet / GPT-4o per faction tone profile + reviewed by AI agent. Comic-panel art generated per [[spec/art-and-asset-pipeline]]. | Tone matches DR-014 across all 30 launch missions. |

### Narrative bible (closes DR-016 specifics)

Per [[spec/narrative-bible]], launch with:

- **Setting bible**: 10-page worldbuilding doc covering Trade Star backstory, Coalition rise, Browncoat clone wars, Ronin frontier, Tek-Mart corruption, Imperatus expansion, Free Hold rebellion, Husk anomaly. Authored by AI agent from DR-016 + faction-flavor seed prompts; project-owner reviewed.
- **Named NPCs**: 24+ named characters across factions (heroes, antagonists, broker NPCs, mission-givers, hostages, defectors). Each has bio + visual reference (Tier 2 generated portrait) + dialogue tone + signature loadout.
- **Faction archives**: 1-page per faction covering history, doctrine, visual register, signature equipment, quirks, weakness, mission types.
- **Mission narrative copy**: Per-mission briefing (3-5 panels) + debrief (3-5 panels) + 5-10 in-mission dialogue lines.
- **Codex entries**: Per-weapon, per-chassis, per-material, per-faction, per-world, per-named-NPC. Unlocks via play. AI-generated flavor copy ~50-200 words each.
- **Tutorial narrative**: First-contract mission script + 8 lab intros.
- **Achievement copy**: 60-100 achievements with 1-2 sentence flavor text.
- **Total launch words**: ~80,000 words of narrative copy. AI-authored + human-reviewed. Localized.

### Localization (closes localization OPEN gate from DR-046 dependencies)

Launch language set:

| Tier | Languages | Method |
|---|---|---|
| **Tier-A (UI fully localized)** | English, Spanish (LATAM), Brazilian Portuguese, German, French, Italian, Russian, Polish, Simplified Chinese, Japanese, Korean | AI translation via GPT-4o / Claude Sonnet, reviewed by AI agent for consistency, validated by community (Discord channels per language); revised post-launch via mod-localization layer. |
| **Tier-B (UI only, narrative English)** | Turkish, Czech, Dutch, Ukrainian, Arabic, Vietnamese, Thai, Indonesian | AI-translated UI strings; narrative comic panels remain English with subtitle option. |
| **Mod-localization** | Any language community provides | First-class moddable layer; localizers can submit string packs via Steam Workshop. |

**Localization technical:**
- All player-visible strings keyed via `t!("key.id")` Rust macro + Fluent (Project Fluent) format.
- Font selection: Noto Sans + Noto Sans CJK + Noto Naskh Arabic (multi-script coverage, OFL license).
- RTL support for Arabic; CJK input/render verified.
- String externalization audit: zero hardcoded English in UI/HUD/captions/error messages.
- Mod packages can ship `.ftl` files per language.
- Locale switcher in settings; live-reload without restart.

## What This Locks In

| Spec Area | Implication |
|---|---|
| `cf-ui` | Owns shell UI surfaces. Bevy + egui + custom comic-noir theme. |
| `cf-narrative` | Owns codex + dialogue + briefing/debrief data. RON + Fluent. |
| `cf-i18n` | Owns Fluent integration + locale switcher + RTL handling. |
| `cf-tutorial` | Owns mission/lab/tooltip/hint state machine. |
| `assets/cinematics/` | Cinematic loops + briefing panels + debrief panels generated per [[spec/art-and-asset-pipeline]]. |
| `content/narrative/` | Codex + dialogue + bios + tutorial scripts. AI-authored, schema-validated. |
| `content/i18n/<lang>/` | Per-language `.ftl` string packs. AI-translated, community-reviewed. |
| `content/missions/<id>/briefing.json` | Per-mission narrative payload. |
| Cosmetics + achievements | Stub at launch; unlock via play; cosmetics and achievements remain earned, anti-FOMO, and non-paid per DR-031 + DR-047 + DR-049. |

## What This Does NOT Lock

| Non-Commitment | Why |
|---|---|
| Voice acting | Open. Default text-only with subtitle. AI-voice (ElevenLabs) for hero NPCs only if license clears. |
| Final achievement count | Open. Target 60-100 at launch. |
| Cinematics length | Open. Per-mission AI cinematic budget = 4-8s loops. Hero campaign = 12-20s pre-rendered (Stable Video Diffusion). |
| Locale font specifics | Open. Noto family default; per-locale fallback as needed. |

## Evidence Trail

- Project owner verbatim (2026-05-06): "I want the game to be flashy and punchy with actions and menus."
- DR-019 (visual direction) + DR-020 (audio) + DR-023 (tutorial) close foundation.
- ElevenLabs Voice for AI-generated voiceover: https://elevenlabs.io/ (review TOS pre-launch).
- Project Fluent localization spec: https://projectfluent.org/.
- Noto Sans family: https://fonts.google.com/noto (OFL).
- Captured in [[research-log/2026-05-06-ai-driven-asset-pipeline-research]].

## Revisit Trigger

- Shell UI playtest reveals first-30-seconds friction.
- Tutorial completion <70% in playtest cohort.
- Localization adds release-blocking complexity (e.g., RTL renderer breaks).
- Narrative copy fails to land tactical-pulp tone.
- Comic-panel briefing UX confuses new players.
