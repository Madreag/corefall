<div align="center">

# Corefall

**A Cortex-Command-grade tactical pulp sci-fi disaster sandbox.**

**Server owns truth. GPU owns richness. Client owns feel.**

A 2D side-view physics sandbox where every gas, grain, bullet, body, world, transmission, and joint is real. Bunker Defence is the flagship mode: attackers breach, defenders hold the command core, and every pressure seal, bullet-punched aperture, liquid jet, thermal leak, radio shadow, projectile, wound, fire, severed limb, and collapsing room is part of the same replayable simulation.

[![Rust 1.95](https://img.shields.io/badge/Rust-1.95.0-CE422B?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org)
[![Bevy 0.18.1](https://img.shields.io/badge/Bevy-0.18.1-232326?style=for-the-badge&logo=bevy&logoColor=white)](https://bevyengine.org)
[![wgpu](https://img.shields.io/badge/wgpu-render-FFCC00?style=for-the-badge&logo=webgpu&logoColor=black)](https://wgpu.rs)
[![Tokio](https://img.shields.io/badge/Tokio-async-3F8FFF?style=for-the-badge&logo=tokio&logoColor=white)](https://tokio.rs)
[![License](https://img.shields.io/badge/license-Apache--2.0%20OR%20MIT-blue?style=for-the-badge)](#license)

[![CI](https://img.shields.io/github/actions/workflow/status/Madreag/corefall/ci.yml?branch=main&style=flat-square&label=CI)](https://github.com/Madreag/corefall/actions)
[![Linux](https://img.shields.io/badge/Linux-supported-FCC624?style=flat-square&logo=linux&logoColor=black)](#)
[![macOS](https://img.shields.io/badge/macOS-supported-000?style=flat-square&logo=apple&logoColor=white)](#)
[![Windows](https://img.shields.io/badge/Windows-supported-0078D6?style=flat-square&logo=windows&logoColor=white)](#)
[![Steam Deck](https://img.shields.io/badge/Steam_Deck-floor_target-1A9FFF?style=flat-square&logo=steamdeck&logoColor=white)](#)

[![Status](https://img.shields.io/badge/status-prealpha%20%28BP3%20active%29-orange?style=flat-square)](#project-status)
[![Build Points](https://img.shields.io/badge/Build_Points-BP2%20closed%20%2F%20BP3%20active-2EA043?style=flat-square)](#build-points)
[![Tests](https://img.shields.io/badge/tests-545%20passing-2EA043?style=flat-square)](#ci)
[![Roadmap V2](https://img.shields.io/badge/roadmap-V2%202026--05--08%20additive-8A2BE2?style=flat-square)](#roadmap-shape)
[![Specs](https://img.shields.io/badge/active%20specs-35%20%28M2.2A..M12%29-blueviolet?style=flat-square)](specs/active/)
[![Releases](https://img.shields.io/github/v/release/Madreag/corefall?include_prereleases&sort=semver&style=flat-square&label=release)](https://github.com/Madreag/corefall/releases)

> **Where we are:** badges above are the source of truth. Full milestone matrix in [Project status](#project-status); BP scope table in [Build Points](#build-points); release policy in [Releases](#releases).
>
> **Where to play:** today, build from source via [Getting started](#getting-started). First friend-handoff release (`.dmg` / `.msi` / AppImage — double-click to play, no Terminal) lands when BP3 closes per the [Double-Click Playability Hard Gate](#releases).

**[Marketing highlights](#headline-systems) · [Project status](#project-status) · [Build Points](#build-points) · [Roadmap shape](#roadmap-shape) · [Releases](#releases) · [Tech stack](#tech-stack) · [Getting started](#getting-started)**

</div>

---

## Headline Systems

The pillars that make Corefall its own genre, not a clone of any of its inspirations.

| System | What's different about it |
|---|---|
| **No HP bar — origin-specific survival resources** | Cortex-Command-style canonical model. Actors don't have abstract HP; they have **blood** (humans), **oil + power** (robots), **blood + power** (androids), **bio-fluid + bio-energy** (heavy biomech). Damage routes to specific resources via specific organs / circuits / wounds. Heart wound drains blood at 5 ml/s; CPU destroyed = instant offline; oil reservoir empty = joints seize. Every death is causally explainable from resource graphs over time. **Owned by M5.8.** |
| **War-Thunder-grade armor — angled armor, spalling, penetration ray** | Real `effective_thickness = nominal / cos(impact_angle)` math. **9 armor types** (RHA, Composite, Ceramic, Reactive ERA, Spaced, Schurzen, Sloped Cast Steel, etc.). 4 round tiers (standard / AP / hardened-AP / discarding sabot) plus HE overpressure + HEAT shaped jet + APFSDS long-rod. **Spalling fragments** spawn into chassis interior on partial penetration. **Penetration ray** traces through chassis modules in order (cockpit → ammo rack → engine → fuel tank → optics) with module ray-order damage and backstop checks. **Owned by M2.5 (foundation) + M5 (chassis) + M5.5 (full physics) + M11 (kill cam).** |
| **Side-view 2D body + functional limb loss** | True side-profile sprites (left/right facing — never camera-facing). Per-stance **AABB hit-zone tables** (Standing / Crouching / Prone / Crawl) determine which zone a projectile strikes. **Limb loss has consequences**: arm lost = weapon dropped + two-handed weapons rejected; both legs lost = forced Crawl stance (25% speed); head destroyed = instant death; backpack lost = jetpack offline. **Multi-zone passthrough** routes high-velocity rounds through one zone into the next. **Owned by M5 + M5.5.** |
| **Stationeers-grade-or-better atmospherics** | Real **PV = nRT** (R = 8314.46 J/(kmol·K)). **10 launch gases** (O2, N2, CO2, Volatiles/Methane, Pollutant, H2, N2O, H2O/Steam, Ozone, He) with full **combustion stoichiometry** (6 deterministic reactions with autoignition temperatures). Pipe networks as first-class atmospheres. Room atmospheres with apertures (doors, breaches, suit punctures, bullet holes). Gradual phase change with latent heat. **Owned by M5.9.** |
| **Noita-grade systemic materials** | **50+ materials** spanning solids/liquids/gases/powders/plasmas. **30+ reactions** (acid + iron → rust; water + lava → obsidian; hydrogen + oxygen + fire → steam + heat). Per-pixel cellular automata. Alchemy table for crafting. Flask system (throw acid, water, oil, healing potions). GPU compute acceleration with CPU deterministic fallback. **Owned by M5.6.** |
| **ACRE2-tier voice + radio simulation** | Proximity voice chat. Radio sets with realistic propagation (distance attenuation + occlusion + frequency band + antenna height). EMP disrupts radio for 5-30s. Solar flares disrupt for minutes. **Vacuum no_voice** (sound doesn't propagate; radio still works). AI radio chatter ("Contact, west wall!"). All voice + radio captioned per accessibility floor. **Owned by M9.5.** |
| **AI as teammate and rival — 8-criteria humanlike bar** | Bots reason, plan, panic, recover, and explain themselves through replay-visible reason labels. **5 enemy archetypes** (Rifleman / Sniper / Assault / Engineer / Spotter). **Personality traits** affect AI (brave, coward, sharp_shooter, paranoid). Optional async LLM "mind" layer proposes doctrine without blocking the simulation. **Persistent AI commanders** remember player tactics across missions. **Owned by M2.2B + M6 + M6.5 + M6.6 + M7 + M11.** |
| **Brain-hopping / multi-actor control** | Cortex Command's defining feature, preserved. Player has designated **brain actor**; transfer control to any friendly actor via `act.player.brain_hop`. Brain death = mission lost. Squad cohesion: when brain is in danger, friendly bots automatically defend. **Owned by M5.** |
| **Bunker Defence flagship mode** | The most polished mode in the game. 2-4 defenders + 2-4 attackers OR vs AI commander. Build power grid + manage atmospherics + defend command core. Per-side persistent base across sessions. Per-server-shard tournament ladder. **Owned by M12.** |
| **Replay determinism, end-to-end** | Same seed + same inputs = byte-identical event stream. 27+ event categories with `parent_event_id` cause chains. Per-tick blake3 checksums anchor the contract. **Cross-platform** (Linux + macOS + Windows) + cross-rate (60Hz + 120Hz) verified. Death recap walks the full chain from death event back to player's last action. **Owned by M3A + M3B.** |
| **Server owns truth — same binary, 5 modes** | One `cf-server` binary, 5 launch modes (`coop_room`, `pvp_arena`, `lan_room`, `mmo_shard`, `lobby_directory`). 3 anti-cheat profiles (`casual`, `competitive`, `tournament_strict`). 4 mod trust tiers (`vanilla`, `verified`, `community`, `experimental`). Community-hostable. No proprietary cloud lock-in. **Owned by M9 + M10 + M11 + M12.** |
| **Modding as a first-class promise** | Every base-game feature is mod-reachable. **Material Lab** workbench authors systemic-material puzzles in <10 minutes. **Mining & Extraction** pipeline: sample → drill → refine → smelt. **Lua + IC10** script hosts (sandboxed). Steam Workshop integration. Auto-update + auto-docs. AI-driven test runs validate mods don't break determinism. **Owned by M8 + M8.5 + M8.6.** |
| **Power & electrical engineering — first-class simulation** | Real electrical sim with 8 generators (solar diurnal + wind cubic + chemical + RTG + nuclear + kinetic + hand-crank + geothermal), 6 cable tiers + voltage classes (LV/MV/HV/Critical) + transmission loss + transformers, 6 storage types (Li-Ion + Lead-Acid + Capacitor + Thermal Mass + Chemical + CAES), per-tick demand profiling (peak vs sustained + inrush), brown-out / black-out / surge / arc-fault cascades, fuses + breakers + ground-fault, IC10 priority chips (Stationeers MIPS). Real circuit physics: resistivity per material × length × temperature; 10 cable failure modes; 8-tier protection device hierarchy; cascading failure mathematics; three-phase power (T3+); power factor + reactive power (T4 exotic). **Per-actor personal power**: humans don't need power but their equipment does; androids need power for synthetic parts (organic side independent); robots fully power-dependent (battery empty = INERT recoverable). Brain rooted = +30% grid efficiency; uprooted = base falls to raw output. **Owned by M7.6 + M5.8.** |
| **10 playable races with full environmental resistance matrix** | Human + Android + Robot + Powered Organic + Heavy Biomech + **5 NEW races**: **Insectoid** (chitin armor; cold-blooded reaction speed) / **Crystalline** (silicon-based; IMMUNE to radiation; thrives on Sol-zone) / **Photosynthetic** (CO2 breather; immune to most poisons; needs sunlight) / **Aqueous** (water-based; immune to drowning; native to Europa subsurface ocean) / **Methane Breather** (Stationeers Zrilian-parity; oxygen IS POISON; native to Mimas). Per-race × per-environmental-factor matrix (temperature / pressure / radiation / 10 gas types). Each race has a "PROMISED LAND" world where they thrive — race choice meaningfully changes where you live + what tech you build. **Owned by M5.8.** |
| **ONI-grade closed-loop life support + thermal tile system** | Electrolyzer (water → 0.888 kg/s O2 + 0.112 kg/s H2 stoichiometric); closed-loop colony pattern (water → O2 + H2 → crew → CO2 → plants → O2). Per-tile thermal conductivity with per-material table (Steel 50 / Aluminum 237 / Copper 401 / Insulite 0.01 / Vacuum 0). Insulated tiles for thermal management. **13 launch geyser types** with eruption cycles + taming infrastructure (Steam Vent / Water Geyser / Hydrogen Vent / Cool Steam Vent / Methane Vent / Chlorine Gas Vent / Natural Gas Geyser / Liquid Hydrogen Vent / Oil Reservoir / Metal Volcano / Magma Volcano + 2). Polluted oxygen + polluted water bipartite resource model. Algae / plant farming (6 plant types) + critter ranching (6 critter types) for renewable food. **Owned by M5.9 + M7.5.** |
| **Stationeers-grade gas tank inventory + filter system** | 5-tier gas tank progression (T0 Emergency 5L → T1 Compressed 60L → T2 Pressurized 300L → T3 Cryogenic 500L → T4 Closed-Loop Cycler indefinite). Tank gas content (Pure O2 / N2 / CO2 / Volatiles / H2 / N2O / Ozone / He / Argon / specialty mix). Filter system (CO2 / Volatiles / Pollutant / Radiation / Composite). Tank physics computed via PV=nRT; overpressure rupture; cryogenic Joule-Thomson cooling. **Vehicles require tanks**: dropships need fuel + breathing + buffer; rockets need cryogenic O2 + H2; mechs need coolant + oil; submarines need closed-loop breathing. **Owned by M5.8 + M5.9 + M2.2A.** |
| **Crafting tiers + fabrication chain** | 5-tier ladder (T0 Primitive → T1 Industrial → T2 Advanced → T3 Endgame → T4 Exotic via Material Lab). 150 launch recipes covering weapons / armor / chassis / tools / base / power. Fabrication chain: raw ore → refined → component → equipment → assembly. Material purity grading (Stationeers parity) propagates through chain. Multi-step recipe chains (T3 plasma rifle = 8 steps). Power-coupled (fabricators consume kW; brown-out pauses crafting). Research-gated unlocks via 30-node tech tree. **Owned by M7.8.** |
| **PvE Survival mode + Inter-Planet Transport** | Terraria/Stationeers/Minecraft/Cortex hybrid solo + 2-8 player coop. **3 launch survival worlds** (Earth + Mars + Mimas; 9 more post-launch unlock). 7-step procgen pass (topology → biomes → ore → hazards → structures → AI raiders → validation). Survival mechanics per race (hunger / thirst / sleep / sanity / temperature). **3 inter-planet transport modes**: dropship (T1; 8-phase flight + 6 risk events) / multi-stage rocket (T2; 5-stage Kerbal-style) / paired teleporters (T3). Orbital stations + asteroid mining colonies. 5 PvE endgame bosses (Hollow King / Frozen Heart / Crimson Tide / Eclipse Walker / Last Star). 12 dynamic world events. Acclimatization mechanic (chitin fast; humans slow). Race-specific tech tree branches. **Owned by M11.5.** |
| **Crafting tiers + fabrication chain** | 5-tier ladder (T0 Primitive → T1 Industrial → T2 Advanced → T3 Endgame → T4 Exotic via Material Lab). 150 launch recipes covering weapons / armor / chassis / tools / base / power. Fabrication chain: raw ore → refined → component → equipment → assembly. Power-coupled (fabricators consume kW; brown-out pauses crafting). Research-gated unlocks via tech tree + boss drops + Material Lab puzzles. Stationeers + Terraria hybrid UX. **Owned by M7.8.** |
| **Configurable everything — 7-tier settings hierarchy** | Every dial is a settings key with typed schema + default + range + description + owner. 7-tier hierarchy (engine → system → profile → scenario → server → CLI → runtime) with key locking. Server admin's `server.ron` controls **200+ tunables** across 17 subsystems (anti-cheat, mods, AI, power, damage, crafting, atmospherics, hazards, survival, etc.). Player's `profile.ron` for personal prefs. 20+ admin CLI commands (kick / ban / save / restart / hot-load / lock / unlock / mod / broadcast). Schema migration on bump. Auto-generated settings UI from schema. Audit log + change history. Settings included in replay determinism contract. **Owned by M9.10.** |
| **Accessibility as a floor, not an afterthought** | 200% UI scale + high contrast + color-independent state labels + reduce motion / shake / flash + hold-to-confirm + focus traversal + key remap + captions for ALL audio. **G-Force vision blackout** scales per origin (humans full; androids 50%; robots immune). 8 color-blind protocols. ACC-A floor from M4A onwards; ACC-A-PLUS extension at BP9+. **Owned by M4A + ACC-A-PLUS.** |
| **Animation-first bodies — never sliding pawns** | Actors don't slide as static sprites. Controlled locomotion is readable animation with physical weight (walk / run / crouch / sprint / slide / vault / climb / dive / lean / swim / crawl). Stability scalar + recoil bloom + sharp aim + travel-impulse damage. Disrupted states become physical (ragdoll on death; gibs cascade per CCCP's `MOSRotating::CreateGibsWhenGibbing` pattern). **Owned by M1 + M2.2A + M5 + M5.5.** |

---

## Launch Content Roster

The full content target shipping by BP12 (every entry **functional + AI-readable + replay-recorded + caption-bound + balance-fixtured + localized + hot-reloadable + mod-parity** per DR-045).

| Category | Count | Status by Milestone |
|---|---:|---|
| **Weapons** | 70+ | 1 (M1) → 6 (M2.2A) → 14 (M5.5.5) → 35 (M7) → 50 (M11) → 70+ (M12) |
| **Actors** | 44+ | 1 (M1) → 9 (M5.5.5) → 28 (M7) → 35 (M11) → 44+ (M12) |
| **Vehicles** | 18+ | 0 → 8 (M7) → 12 (M11) → 18+ (M12) |
| **Base objects** | 60+ | 0 → 12 (M5.9.5) → 40 (M7) → 50 (M7.5) → 55 (M11) → 60+ (M12) |
| **Factions** | 8 | 0 → 3 (M2.2B) → 8 (M7) |
| **Missions** | 30+ | 2 (M1.5 + M2.5) → 6 (M5.9.5) → 8 (M6) → 24 (M7) → 28 (M10) → 30+ (M11) + 5 hidden endgame |
| **Worlds × biomes** | 12 × 3-5 | 6 (M5.9) → 9 (M7) → 12 + 36-60 biomes (M7.7) |
| **Materials** | 17 launch | 8 (M2) → 17 (M5.9; expansion 13 unlocked via Material Lab M8.5 → 50+) |
| **Ores** | 12 | 7 (M5.9) → 12 (M8.6) |
| **Music tracks** | 30+ | 0 → 8 (M4B) → 22 (M7) → 28 (M7.7) → 30+ (M11) |
| **SFX** | 400+ | 80 (M5.5.5) → 120 (M5.9) → 140 (M5.9.5) → 160 (M6) → 320 (M7) → 360 (M7.7) → 380 (M9.5) → 400+ (M12) |
| **Narrative** | ~80,000 words | 60,000 (M7) → 75,000 (M11) → 80,000 (M12) |
| **Codex entries** | 600 | 450 (M7) → 530 (M8.5) → 580 (M11) → 600 (M12) |
| **Achievements** | 75+ | 30 (M7) → 35 (M9) → 36 (M10) → 50 (M11) → 75+ (M12) |
| **Languages** | 11 Tier-A + 8 Tier-B | English baseline (M2.2C) → 19 total (M12) |
| **Endgame modes** | 10 | 10 launch modes (M11): Bunker Defence + Bunker Attack + Salvage Run + Tournament + Wave Survival + Boss Rush + Stealth Challenge + Time Attack + Permadeath Campaign + Cross-Shard Events |
| **Cosmetic categories** | 50+ per actor | 50+ items per actor at M12 launch |

---

## What Corefall Is

Corefall is the implementation repo for a **tactical pulp sci-fi disaster sandbox**. The player fantasy is Cortex Command's command-core, dropship, chassis, digging, and body-swapping chaos rebuilt around a stricter simulation contract: deterministic replay, server-authoritative multiplayer, real atmospherics, systemic materials, universal gravity, full collision, and scriptable AI/modder workflows from day one.

You will:

- **Defend the bunker** as 1-4 humans plus AI guards, or play the attackers and breach it with dropships, explosives, tunneling, pressure sabotage, fire, and misdirection.
- **Fight across 12 worlds** with different ambient atmospheres, gravity, weather, day length, and hazards: Earth, Mars, Phobos, Deimos, the Moon, Mimas, Europa, Vulcan, Venus, Sol-zone habitats, belt asteroids, and orbital stations.
- **Use physics as tactics**: vent rooms, overpressure corridors, shoot pressure holes, create liquid/gas jets, light flammable atmospheres, cut coolant lines, heat or cool rooms, collapse terrain, redirect gravity, burn oxygen, mine ore, and salvage wreckage.
- **Pick from 10 playable races**, each with distinct breathing requirements + temperature ranges + pressure tolerance + radiation resistance: humans / androids / robots / powered organics / heavy biomechs / **insectoids** (chitin armor + cold-blooded reaction speed) / **crystallines** (silicon-based; radiation-immune; native to Sol-zone) / **photosynthetics** (CO2 breathers; thrive in sunlight) / **aqueous** (water-based; immune to drowning; native to Europa subsurface ocean) / **methane breathers** (Stationeers Zrilian parity; oxygen IS POISON; native to Mimas).
- **Swap between origins**: humans breathe and concuss, androids bridge organic and synthetic failure chains, robots overclock, downclock, short, leak coolant, and ignore hazards that kill flesh.
- **Manage closed-loop life support** like ONI: electrolyzer splits water into O2 + H2; plants convert CO2 → O2; scrubbers clean polluted oxygen; tame **13 geyser types** (steam vents / oil reservoirs / metal volcanoes / chlorine vents / etc.) for renewable resources.
- **Lose limbs and feel it**: a severed arm drops your weapon and rejects two-handed firearms; both legs gone forces a crawl; head destroyed is instant death. Side-view sprites face left or right; armor angle vs facing direction matters tactically.
- **Coordinate over simulated comms** with ACRE2-tier radio propagation, interference, EMP failure, captions, accessibility fallbacks, and AI reason labels that explain what bots believe is happening.
- **Build, capture, or destroy bases** with real rooms, pipes, airlocks, pressure regulators, power, storage, fabrication, doors, platforms, and modder-defined modules.
- **Brain-hop** between controlled actors as in Cortex Command — but your designated brain dying ends the mission.

This is not a Cortex Command remake. It is a **best-of-genre synthesis** that takes Cortex Command's command-core / dropship / chassis / digging fantasy and sets it on top of Stationeers-grade-or-better atmospherics + gas tank progression + IC10 programmable chips, ONI-grade closed-loop life support + per-tile thermal conductivity + geyser harvesting, Noita-grade systemic materials, Subnautica-style temperature acclimatization, full collision physics, universal gravity, ACRE2-tier voice + radio simulation, War Thunder-grade armor + spalling, and a full astrography of 12 playable worlds with 10 distinct playable races. AI bots are first-class teammates and rivals. Replay is deterministic. Modding is data-first. Accessibility is a floor, not an afterthought.

---

## Roadmap Shape

The canonical roadmap now covers **57 closed/directional decision records**, **38 sequenced milestones** (3 closed + 35 active in `specs/active/M2.2A..M12.md`), **17 side tracks** including 4 dedicated production tracks (art, narrative, localization, live-ops), and a **13-step Build Points layer (BP0..BP12)** that bundles related milestones into shippable proof slices. The Roadmap V2 (2026-05-08) additive pass kept every prior milestone/DR id stable; BPs and micro-fun interludes layered *on top of* M0..M12 without renaming anything. The M2.2 mega-bridge slice was split into M2.2A (actor / equipment / inventory / sound) + M2.2B (AI archetypes / mission director / personality / faction) + M2.2C (camera / UX / debug / localization / pie menu) so each can be implemented and reviewed independently.

| Area | Current Direction |
|---|---|
| Authority model | Solo runs as an in-process server. Multiplayer uses `cf-server` as the canonical fixed-tick truth. Clients predict feel locally; server corrections preserve shared state. |
| GPU policy | GPU is used aggressively for rendering, particles, lighting, smoke, debug overlays, prediction, advisory maps, and optional certified server acceleration. CPU deterministic sim remains the required authoritative fallback. |
| Asset / audio pipeline | All project assets and audio are AI-agent-authored, ledgered, regenerable where possible, captioned where audible, and exposed to modders through the same pipeline. |
| Damage simulation | **Per-limb armor + AP rounds + organ / circuit internal damage + concussion dose + fluid leak channels + War Thunder angled armor + spalling + ricochet + multi-zone passthrough + per-origin resource depletion (no abstract HP bar)** integrated end-to-end from M2.5 forward. Every death is causally explainable. |
| Content target | 70+ weapons, 44+ actors, 18+ vehicles, 60+ base objects, 8 factions, 30+ missions, 12 worlds with biomes, 17 materials, 12 ores, 30+ tracks, 400+ SFX, 80,000 words, 600 codex, 75+ achievements, 11+8 languages, 10 endgame modes, 50+ cosmetics per actor. |
| Shell and platform | Title / menu / settings / lobby / workbench / briefing / debrief / map / achievements / replay viewer / codex / photo mode / cosmetic locker / death cam / mod manager, plus Steam Workshop, cloud, achievements, input, and Steam Deck readiness. |
| Accessibility | ACC-A floor plus cognitive, motor, hearing, reading, and sensory presets. Captions, full subtitles, reduced motion / shake / flash, UI scaling, high contrast, remap / hold alternatives, color-blind palettes, and screen-reader paths are roadmap gates. |
| Modding | Modders extend chassis, weapons, materials, atmospheres, missions, AI doctrines, audio, animations, scripts, factions, localization, and test runs with the same schema-first pipeline as the base game. 4 trust tiers (`vanilla` / `verified` / `community` / `experimental`). |
| Monetization posture | Premium one-time purchase direction, free modding, community-hostable servers, no pay-to-win. Optional cosmetic-only gacha / battle-pass hooks are late-cycle, dormant / default-off systems and require a future activation DR. |
| Production T-tracks | 4 dedicated late-anchored tracks ride alongside the gameplay spine: **T-CONTENT-ART** (AI-authored art / animation / VFX / decals / lighting / music / SFX / launch roster), **T-CONTENT-NARRATIVE** (~80,000 words bible / codex / dialogue), **T-LOCALIZATION** (Project Fluent + 11 Tier-A langs + 8 Tier-B UI-only + mod-localization), **T-LIVEOPS** (telemetry, marketing, Steam, legal, post-launch ops). Each finalizes at BP12; placeholder generation begins at BP3+. |
| Micro-fun-slice cadence | After every major systems milestone, a short interlude proves the new system is *fun* before the next one stacks on top: **M1.5** (breach), **M2.5** (reactor defense with the deep damage / armor / hazard / atmospherics surface), **M5.5.5** (sabotage combining collision + materials + hazards + origin), **M5.9.5** (pressure hold under breached atmosphere). Each is a 60-90 s scenario driven by cfctl, validated by run bundles. |
| AI-agent self-testing | **T-CAPTURE** layer (BP2+): cf-app emits PNG frame readbacks at 10 Hz baseline + event-triggered keyframes; `game/tools/capture_grid.py` composes machine-oriented 8×8 grid PNGs + `summary_grid.png` (one frame per major event, ≤64 frames) and a review-sized `review_grid.png` capped at 16 frames (4×4 when full). cf-e2e drives the loop end-to-end. AI agents read the grids to validate motion, physics, effects, and HUD readability without a human eyeballing every frame. |
| Actor presentation | Animation-first while controlled, physics-first while disrupted, and always replay/event-visible. Visible actor movement cannot remain a static sliding pawn once the milestone owns movement/body presentation: walking, running, crouching, climbing, jetting, aim blending, limb damage, knockdown, pressure/wind/explosion response, severance, and mech gait all need observable state, events, and capture evidence at their maturity level. |

---

## Build Points

The Roadmap V2 layer groups gameplay milestones into **13 Build Points (BP0..BP12)**. A Build Point is a shippable proof slice — when a BP closes, every milestone inside it has the Acceptance Matrix + Contract Integrity Matrix + Performance/Config Audit + run-bundle evidence, plus the AI-Agent Self-Test Report and LLM-graded verdict. Human playtest is welcome confirmation, not the blocking gate. BPs do not replace milestones; they bundle them.

| BP | Anchor | Bundles | Fun-Proof Slice | Status |
|---:|---|---|---|---|
| **BP0** | Engine bootstrap | M0 | (kickoff smoke) | Closed |
| **BP1** | Actor controller + breach fun proof | M1 + M1.5 + T-CAPTURE | M1.5 Micro Breach | Closed |
| **BP2** | Terrain & Replay Build | M2 + **M2.2A** + **M2.2B** + **M2.2C** + M2.5 + M3A | M2.5 Micro Reactor Defense + headless replay verifier | M2/M2.5/M3A Closed; **M2.2A/B/C bridge active** |
| **BP3** | Combat Readability Build | M3B + M4A + M5 + Double-Click Release Engineering | HUD / body / chassis proof + first friend-handoff release | Active (M3B/M4A/M5 landed; release engineering pending) |
| **BP4** | Physics Sandbox Alpha | M5.5 + M5.5.5 + M5.6 + M5.7 + M5.8 | M5.5.5 Micro Sabotage | Planned |
| **BP5** | Atmospherics & Worlds Alpha | M5.9 + M5.9.5 + M5.10 | M5.9.5 Micro Pressure Hold | Planned |
| **BP6** | AI Combat Alpha | M6 + M6.5 + M6.6 | AI-H / MIND / AI-MAT suites | Planned |
| **BP7** | Vertical Slice Alpha | M7 + M7.5 + **M7.6 (Power & Electrical)** + M7.7 + **M7.8 (Crafting Tiers)** + M4B | Breach Contract + Bunker Defence proof + power grid + tier ladder | Planned |
| **BP8** | Creator Alpha | M8 + **M8.5 (Material Lab)** + **M8.6 (Mining & Extraction)** | Modder parity smoke + AI-MINE-A suite | Planned |
| **BP9** | Server / LAN Alpha | M9 (Dedicated Server App) + **M9.10 (Server Config + Admin CLI)** + M10 (LAN Co-op) | LAN co-op smoke + DET-A suite + comprehensive config | Planned |
| **BP10** | Online Beta | M11 (Online Co-op + Full Match Grammar) + **M11.5 (PvE Survival + Inter-Planet)** + M9.5 (Voice + Radio) | Self-hosted online co-op + comms + PvE survival + planet travel | Planned |
| **BP11** | Public Systems Beta | M12 (Public PvP + MMO Shards + Bunker Defence Flagship) | PvP / MMO shard proof | Planned |
| **BP12** | Release Candidate | Production T-track finalization | Launch GA build | Planned |

**Build Point closure gate.** Every BP closeout requires (a) every milestone inside it PASS in the Acceptance Matrix, (b) the Minimum-Bar Design Coverage Matrix proving the worker handled obvious inside-scope game / UX affordances instead of only narrow task wording, (c) the Contract Integrity Matrix proving shared code paths + negative / adversarial proof, (d) the **Universal Enhancement Done-Criteria (DR-056)** matrix PASSing for every M1+ milestone inside the BP, (e) run-bundle evidence for every fun-proof slice at multiple tick rates, (f) **T-CAPTURE evidence** (each fun-proof script emits a `summary_grid.png`, review grid, and `capture_manifest.json` recorded in `summary.json.artifacts`; `--expect capture.summary_grid.non_blank_ratio>=0.95` mandatory from BP2 onward), (g) **T-RELEASE evidence** (tagged GitHub pre-release from BP1 onward with the BP exemplar bundle + `summary_grid.png` + SHA256SUMS), (h) `/corefall-review <bp>` verdict = `Accept`, and (i) the BP Goal Coverage Report + AI-Agent Self-Test Report + LLM-graded verdict. Human playtest notes are welcome confirmation, not a blocker when the AI report and grading gate pass.

**Universal Enhancement Done-Criteria (DR-056) — applies to every M1+ milestone.** Per-tier perf gate (Steam Deck 800p/60 + 1080p/60 + 4K/120) + CI bench regression + 24h memory-leak soak + network sync verified + replay determinism CI matrix per platform + every player surface scriptable via cfctl + AI-agent validation report + AI audio pipeline + juice rules per DR-055 + ACC-A floor + Tier-A localization keyed strings + modding parity + anti-FOMO + anti-pay-to-win audit + captions for ALL audio. Universal rows are not optional polish — a milestone is not closed if any Universal row FAILS unless the user explicitly approves that exact deferral. Per-milestone specifics layer on top of the universal contract (see [`docs/plan/spec/milestone-enhancement-pass-m1-plus.md`](docs/plan/spec/milestone-enhancement-pass-m1-plus.md)).

**Production-track wiring.** T-CONTENT-ART, T-CONTENT-NARRATIVE, T-LOCALIZATION, and T-LIVEOPS run alongside the BP spine but only finalize at BP12. They begin placeholder generation at BP3+ so the gameplay spine isn't blocked on art / audio / copy / legal.

See [`docs/plan/spec/prototype-roadmap.md`](docs/plan/spec/prototype-roadmap.md) for the full BP table, the **Design-Completeness Map** (every product surface mapped to its owning BP+milestone so you can verify "yes, by BP12 this is a complete game"), the milestone-map gap fills, the done-criteria summary, the kickoff smoke commands, and the inter-milestone bridge contracts.

---

## Design-Completeness Promise

By end of BP12 the game is a complete releasable candidate — every product surface a Steam buyer touches is functional, integrates with replay / save / server / modding / accessibility / captions, and is reachable from `cfctl`. Final balance tuning, marketing timing, and explicit post-launch expansions are the only work remaining. Concretely, **BP12 ships**:

| Surface | What It Means By BP12 |
|---|---|
| **Playable game** | Title → menu → settings → tutorial / labs → mission select / briefing → playable mission → debrief / reward / save loop works without developer intervention. |
| **Core promise** | Bunker Defence + Breach Contract + command-core / base stakes + chassis / equipment + terrain / materials + atmospherics + full collision + AI teammates / enemies + replay + modding all integrated, not isolated demos. |
| **Damage simulation** | No-HP-bar per-origin resource model + War Thunder angled armor + spalling + per-organ / circuit internal damage + concussion bands + fluid leaks + side-view limb-loss functional consequences + multi-zone passthrough + War Thunder kill cam — all integrated. |
| **Content roster** | 70+ weapons + 44+ actors + 18+ vehicles + 60+ base objects + 8 factions + 30+ missions + 12 worlds × 3-5 biomes + 17 materials + 12 ores + 30+ music tracks + 400+ SFX. Every entry functional + AI-readable + replay-recorded + caption-bound + balance-fixtured + localized + hot-reloadable + mod-parity (DR-045). |
| **UX / UI** | Title + main menu + pause + full settings tree + lobby + workbench + briefing + debrief + map + achievements + replay viewer + codex + photo mode + cosmetic locker + death cam + mod manager. All cfctl-parity. |
| **Accessibility** | DR-012 floor + DR-051 accessibility-plus presets (cognitive + motor + hearing + reading + sensory + 8 color-blind protocols + cinematic accessibility) functional. |
| **Multiplayer / server** | Dedicated server + LAN co-op + online co-op (community-hosted, NAT-traversed) + public PvP arenas + persistent MMO shards + admin / moderation basics validated. |
| **Modding** | First-class schema + Lua / Rhai / IC10 script host + package builder + auto-update + auto-docs + AI-driven test runs + community ecosystem extensions. |
| **Endgame + retention** | 10 endgame modes + persistent veterans + bunker meta + cross-shard world events + anti-FOMO archive + intrinsic-only progression (DR-031 + DR-048). |
| **Narrative + localization** | ~80,000 words narrative bible + 8 faction archives + 24+ named NPCs + ~600 codex entries + 11 Tier-A languages + 8 Tier-B UI-only + mod-localization layer. |
| **Audio + AI-authored content** | 30+ adaptive music tracks + 400+ SFX + ACRE2-tier radio + Steam Audio-tier voice + diegetic-first mix; all generated through the AI-only DR-053 pipeline + usage-ledger. |
| **Release operations** | T-RELEASE GA at v1.0.0 + T-LIVEOPS telemetry / crash / bug-tool + legal / license ledger + platform packaging + code signing + support docs + sustainability / sunset posture. |

If a row above is still missing a core system at BP12 closure time, BP12 cannot close by relabeling it as polish. Either implement it, write a user-approved scope-change DR, or keep BP12 open. See [`docs/plan/spec/prototype-roadmap.md`](docs/plan/spec/prototype-roadmap.md) **Design-Completeness Map** for the full surface-to-milestone matrix.

---

## The Layered Simulation

> **BP3 status:** The diagram below shows the *target architecture*. At BP3, the actor controller, equipment / chassis grammar, chunked terrain, mission state machine, replay / event recorder, and HUD are real. Atmospherics (PV=nRT), systemic materials (CA kernel), full collision physics, universal gravity, and multiplayer are planned systems with stub crates — they ship at BP4-BP9. The diagram is the design commitment; the [Build Points](#build-points) table shows what's real today.

Every system reads from one source of truth. Nothing is faked.

```
                 ┌─────────────────────────────────────────────────────────┐
                 │  AI Doctrine  •  Mission Director  •  Replay  •  HUD    │
                 └─────────────────────────────────────────────────────────┘
                                          ▲
        ┌─────────────────────────────────┴──────────────────────────────────┐
        │                                                                    │
        │  Equipment + Chassis (origin: human / android / robot)             │
        │  Per-origin resources (blood / oil / power / caloric / bio-fluid)  │
        │  Per-zone armor (External → Internal → Core) + War Thunder angle   │
        │  Module ray traversal + spalling + organ / circuit damage          │
        │  Limb-loss functional consequences + side-view facing direction    │
        │                                                                    │
        ├────────────────────────────────────────────────────────────────────┤
        │                                                                    │
        │  Stationeers-grade-or-better Atmospherics + Thermal Simulation     │
        │  • Real PV = nRT, R = 8314.46, per-gas moles + temperature         │
        │  • 10 launch gases + 6 liquid mixtures, expansion via material lab │
        │  • 6 deterministic combustion reactions with autoignition T        │
        │  • Gradual phase change with latent heat                           │
        │  • Pipe networks with pumps, valves, regulators, filtration        │
        │  • Door / vent / bullet-hole / blast-breach / pipe-rupture         │
        │    apertures with liquid/gas pressure jets and wind force          │
        │  • Heat transfer through materials, coolant loops, heaters,        │
        │    radiators, insulation, emergency venting, and thermal failure   │
        │  • Room atmospheres + airlock state machines + suit life-support   │
        │  • Per-planet ambient (Earth / Mars / Moon / Mimas / Europa /      │
        │    Vulcan / Venus / Phobos / Deimos / Sol / Belt / Orbital)        │
        │                                                                    │
        ├────────────────────────────────────────────────────────────────────┤
        │                                                                    │
        │  Systemic Materials (Noita-grade chunked CA kernel)                │
        │  50+ materials + 30+ reactions + alchemy + flasks + GPU compute    │
        │                                                                    │
        ├────────────────────────────────────────────────────────────────────┤
        │                                                                    │
        │  Full Collision Physics (everything physical collides by default)  │
        │  Limb / weapon / armor / chassis / projectile / terrain / debris   │
        │  Swept-volume collision + CCD tiers + impulse-to-damage routing    │
        │  Joint impulse → severance; ragdoll on death; gibs with authoring  │
        │                                                                    │
        ├────────────────────────────────────────────────────────────────────┤
        │                                                                    │
        │  Universal Gravity Field (one source; sampled per-cell per-tick)   │
        │  Per-planet ambient + per-cell overrides (gravity wells, low-g     │
        │  labs, magnetic boots, damaged grav generators, reverse-g rooms)   │
        │  Reads through to ballistic drag + atmospheric stratification +    │
        │  material settling + actor falls + every dropped casing            │
        │                                                                    │
        ├────────────────────────────────────────────────────────────────────┤
        │                                                                    │
        │  EnvironmentSignal Aggregator (M5.10)                              │
        │  One bundle per actor per tick — atmospheric, gravitational,       │
        │  thermal, radiation, photic, EM, weather, water, acoustic,         │
        │  day/night, comms. All consumers read from one source of truth.    │
        │                                                                    │
        └────────────────────────────────────────────────────────────────────┘
                                          ▲
                 ┌────────────────────────┴───────────────────────┐
                 │  Deterministic 60 Hz sim core (120 Hz path     │
                 │  validated; 128 Hz under evaluation)           │
                 │  Per-tick blake3 checksums + replay events     │
                 └────────────────────────────────────────────────┘
```

Every layer emits replay events. Every cause chain is reproducible. Every AI agent reads the same data the player sees. Every death is causally explainable from the resource graphs over time.

---

## Core Pillars

> **BP3 status:** Pillars marked with *(planned)* have stub crates but no production implementation yet. They are design commitments, not shipping features. See [Build Points](#build-points) for what's real today.

| Pillar | What It Means |
|---|---|
| **Real physics, end to end** | No arcade approximations. Stationeers-grade is the minimum bar: PV = nRT atmospheres, pressure apertures, liquid / gas jets, material heat transfer, universal gravity for everything, full collision by default, stoichiometric combustion, and gradual phase change. |
| **No HP bar — origin survival resources** | Actors don't have abstract HP; they have blood (humans), oil + power (robots), blood + power (androids). Damage routes to specific resources through specific organs / circuits. Every death is causally explainable from resource graphs. |
| **War-Thunder-grade armor** | Angled armor (effective_thickness = nominal / cos(angle)). 9 armor types. AP / HE / HEAT / APFSDS round tiers. Spalling fragments. Penetration ray traverses chassis modules in order. Ricochet probability scales by impact angle + ammo type. |
| **Side-view 2D body** | Side-profile sprites (left / right facing). Stance-aware AABB hit zones. Limb-loss functional consequences (arm = weapon dropped; both legs = crawl; head = death). Multi-zone passthrough for high-velocity rounds. |
| **Origin-aware bodies** | Humans, androids, and robots have **structurally different reaction chains**. Robots take internal-shock damage, leak coolant, and downclock under heat. Androids breathe, bleed, and overclock per installed module. Humans concuss, eat, and need oxygen tanks. |
| **Animation-first bodies** | Actors do not slide as static pawns. Controlled locomotion is readable animation with physical weight; disrupted states become more physical. Jetpack, pressure, wind, gravity, recoil, limb damage, armor mass, severance, and mech servos all change the body presentation without destroying responsiveness. |
| **AI as teammate and rival** | Bots are first-class. They reason, plan, panic, recover, and explain themselves through reason labels. The 8-criteria humanlike-AI bar is testable. An optional async LLM "mind" layer proposes doctrine without ever blocking the local AI. Persistent AI commanders remember player tactics across missions. |
| **Replay determinism** | Same seed + same inputs = byte-identical event stream. Debug with replay scrubbing. Network with confidence. Audit AI behavior with cause chains. *(Verified at 60+120 Hz on macOS aarch64; cross-platform CI matrix validates Linux+Windows on push.)* |
| **Server truth, rich clients** | Server owns authoritative state; clients own immediate feel and GPU-rich presentation. Prediction is allowed, divergence is not. Single player uses the same architecture in-process. |
| **AI-authored production** | Art, audio, captions, provenance, and regeneration metadata are pipeline artifacts, not side notes. No retained asset skips the ledger. |
| **Modding as a first-class promise** | Schema-first, Lua / IC10 escape hatches where useful, workbench tooling. Add a gas, a reaction, an origin, a planet — all data rows. Material Lab authors systemic puzzles in <10 minutes. |
| **Multiplayer ladder** | Solo + LAN co-op + online co-op + community-hostable public PvP arenas + persistent MMO shards. Same `cf-server` binary, multi-mode. Anyone can host. NAT traversal + relay for online play. |
| **Accessibility floor** | Captions, contrast, no-color-only UI, focus traversal, reduced motion, reduced shake, reduced flash, reduced G-Force blackout — all from Slice A onward. 8 color-blind protocols. ACC-A-PLUS extensions at BP9+. |
| **No-compromise performance defaults** | Performance-sensitive values are config-driven, never hardcoded. Steam Deck floor at 800p/60, 1080p/60 mid-tier, 4K/120 strong-desktop ceiling. |

---

## Inspirations And Credits

Corefall stands on the shoulders of an exceptional set of games that figured out parts of the genre we want to weave together. **None of the work here is a copy** — but each of these projects taught us something we built on, and they deserve explicit credit.

| Inspiration | What We Learned |
|---|---|
| **[Cortex Command](https://datarealms.com)** by Data Realms | The command-core / dropship / chassis / digging / pixel-actor fantasy. The tone of "every body is physical and damageable". The no-HP-bar wound-emitter death model. Brain-hopping and multi-actor control. The mod ecosystem grammar. The actor-status / wound-state / inventory-fallout triangle. Deep, deep love. |
| **[Noita](https://noitagame.com)** by Nolla Games | Per-pixel material simulation as a core feel pillar. Alchemy / reaction / emergence as a retention loop. Hidden chemistry that rewards experimentation. The replay-able cause-chain culture. Flask system. Perk + curse mechanics. |
| **[Stationeers](https://stationeers.com)** by RocketWerkz | The minimum bar for atmospherics feel: real ideal-gas-law atmospherics, specific heats, autoignition temperatures, combustion stoichiometry, pipe networks as first-class atmospheres, suit life-support with canister + filter + waste-tank slots, and per-planet ambient. IC10 programmable chips. Corefall aims beyond that with combat apertures, liquid jets, thermal engineering, and richer material coupling. |
| **[Barotrauma](https://barotraumagame.com)** by FakeFish + Undertow Games | Rooms-with-state architecture. Breach flooding. Crew dynamics where roles matter. Mission storytelling that emerges from system failure. |
| **[War Thunder](https://warthunder.com)** by Gaijin | Angled armor mechanics with effective_thickness math. Module-ray penetration through chassis interior. Spalling fragments. Ammo rack detonation cascades. The kill cam that visualizes exactly which module took the killing shot. |
| **[The Powder Toy](https://powdertoy.co.uk)** | Open-source falling-sand chemistry. The discipline of element-grammar reaction tables. Educational transparency. The stamp save/load pattern that the Material Lab inherits. |
| **[OpenSoldat](https://opensoldat.org) / [Soldat](https://forums.soldat.pl)** | Side-view multiplayer combat feel. Reticle bloom from movement / recoil / airborne / prone-transition. Tracer round cadence. Map mutability. Community-hosted server culture. |
| **[Liero](https://liero.be) / [OpenLieroX](https://openlierox.sourceforge.io)** | Short, intense, weapon-rich arena combat. Material affordance flags (passable, flow, breathability, hookable). The proof that small arenas + many extreme weapons + short rounds can produce decades of replay value. |
| **[ACRE2](https://acre2.idi-systems.com)** mod for Arma 3 | Radio propagation realism. Distance attenuation + occlusion + frequency band. The model for M9.5's voice + radio simulation. |
| **[Teardown](https://teardowngame.com)** by Tuxedo Labs | Tools that change the map become real tactics. Destruction is design. Per-pixel terrain integrity. |
| **[Oxygen Not Included](https://klei.com/games/oxygen-not-included)** by Klei | Per-cell atmospheric simulation at habitat scale. Pressure / temperature / gas density storytelling. **Electrolyzer** splitting water into O2 + H2. Closed-loop colony pattern. Per-tile thermal conductivity. Insulated tiles + Insulite material. 13 geyser types as renewable resource taps. Polluted oxygen / polluted water bipartite resource model. Critter ranching. |
| **[Stationeers Zrilian DLC](https://store.steampowered.com/app/1038400/)** by RocketWerkz | Volatile-breathing playable race; canonical pattern for non-O2 races. Inspired Corefall's methane breather origin. |
| **[Subnautica Below Zero](https://unknownworlds.com/subnautica)** by Unknown Worlds | Temperature meter (hot/cold). Cold suit insulation tiers. Race-specific acclimatization to extreme environments. |
| **[STALKER](https://gsc-game.com)** series by GSC Game World | Anomaly hazards (gravity / electric / time / chemical / psy_storm). Artifact loot from anomaly proximity. Faction relationships and zone-territoriality. |
| **[Helldivers](https://helldivers.com)** series by Arrowhead | Stratagem call-ins with D-pad combos and cooldowns. Cooperative chaos with friendly-fire risk. |
| **[Diablo](https://diablo.blizzard.com)** series by Blizzard | Loot rarity tiers (Common / Magic / Rare / Legendary / Unique). Item affixes. Set bonuses. Mini-boss + boss multi-phase patterns. |
| **[Rimworld](https://rimworldgame.com)** by Ludeon Studios | Storyteller / incident director pacing (Cassandra Classic / Phoebe Chillax / Randy Random / Ironman / Sandbox). Personality traits + mood / stress system. Pet companions. |
| **[Sea of Thieves](https://seaofthieves.com)** by Rare | Treasure maps + voyages with sequential clues. World boss events with coordinated takedown. |
| **[Rain World](https://rainworld.net)** by Videocult | Behavioral creatures that don't need stat-bars to feel real. |

We **also** lean on the open Rust gamedev ecosystem: [Bevy](https://bevyengine.org), [wgpu](https://wgpu.rs), [Rapier / Avian](https://rapier.rs), [Tokio](https://tokio.rs), [serde](https://serde.rs), [BLAKE3](https://github.com/BLAKE3-team/BLAKE3), and many more. See [game/Cargo.toml](game/Cargo.toml) for the full dependency tree.

> [!important] Reuse posture
> No code, no assets, no sprites, no audio, no scripting from any of the inspiration games is copied into Corefall. Everything is implemented from chemistry / physics / game-design first principles plus public documentation (wikis, GDC talks, modding docs, public source where applicable). Provenance is logged in the canonical vault's usage ledger when any specific snippet of public documentation is quoted in spec / research notes.

---

## Tech Stack

| Layer | Tooling |
|---|---|
| Language | [Rust](https://www.rust-lang.org) edition 2021, MSRV / toolchain pinned to 1.95.0 |
| Engine | [Bevy](https://bevyengine.org) 0.18.1 + [wgpu](https://wgpu.rs) for 2D / GPU; custom core crates for sim |
| Physics | Custom collision + custom material kernel + Stationeers-grade-or-better atmospherics / thermal kernel + universal gravity field |
| Async | [Tokio](https://tokio.rs) for the JSON-RPC control plane and dedicated server |
| Networking (planned) | TBD between [Lightyear](https://github.com/cBournhonesque/lightyear) / [renet](https://github.com/lucaspoffo/renet) / [quinn](https://github.com/quinn-rs/quinn); decision locks at M9 close per DR-052 |
| Modding host (planned) | [mlua](https://github.com/khvzak/mlua) (Lua) candidate; deferred to M8; IC10 chip editor in M8 |
| Determinism | [BLAKE3](https://github.com/BLAKE3-team/BLAKE3) for state checksums; [rand_xoshiro](https://docs.rs/rand_xoshiro) for seeded RNG |
| Schemas | [serde](https://serde.rs) + [schemars](https://github.com/GREsau/schemars) + JSON Schema validation in CI |
| Testing | `cargo test` matrix (Linux + macOS + Windows) + scripted E2E + run-bundle checker (Python `tools/prototype_run_check.py`) + AI-Agent Self-Test Report + LLM-graded verdicts |
| Editor | [Visual Studio Code](https://code.visualstudio.com) with [rust-analyzer](https://rust-analyzer.github.io); [Helix](https://helix-editor.com) and [Zed](https://zed.dev) supported via per-project `.gitignore` |

---

## The Workspace

**32 crates today** (see [game/Cargo.toml](game/Cargo.toml)). Each crate carries its own `AGENTS.md` boundary contract. Crates marked **(real)** have shipped real implementations; the rest are stubs that will fill in at their owning milestone.

```text
game/crates/
├── cf-app                    # (real)  Bevy app shell + window + keyboard input + render bridge
├── cf-sim-core               # (real)  fixed-tick scheduler + RNG + checksum
├── cf-control                # (real)  JSON-RPC 2.0 control surface (cf-control engine + server)
├── cfctl                     # (real)  operator CLI (observe, run, scenario, settings, runbundle, system, act.player.*, act.chassis.*)
├── cf-replay                 # (real)  run-bundle writer + event envelope + 27 categories
├── cf-actor                  # (real)  actor records + control intent + sim step + projectile + status state machine + chassis attachment
├── cf-equipment              # (real)  role records + rifle spec + tick-rate-independent timing + 3 LOAD-A loadouts (infantry / powered_armor / light_mech)
├── cf-physics                # (real)  kinematics + ground collision + jump + recoil; M5.5 swaps in DR-033 collision matrix
├── cf-terrain                # (real)  M2 chunked terrain: 8 launch materials + per-pixel integrity + dirty-region tracker + penetration formula
├── cf-mission                # (real)  Objective / MissionState / MissionView + objective state machine + LossReason typed enum
├── cf-ai                     # (real)  ReactiveGuard FSM + utility scoring + scripted aim-settle + DR-008 LEAN
├── cf-render-2d              # (real)  wgpu 2D pipeline + actor + chunked terrain + overlay + debris + dig preview + extraction zone sprites
├── cf-capture                # (real)  T-CAPTURE: PNG readbacks at 10 Hz baseline + event keyframes; capture_manifest.json for the composer
├── cf-ui                     # (real)  comic-noir UI presentation: STATUS / AMMO / STANCE / ITEM / OBJECTIVE / TIMER / EVENT / SILHOUETTE / MODULE STRIP / TOOL / REACTOR / BANNERS
├── cf-e2e                    # (real)  scripted end-to-end runner with auto-launch + --expect <key>=<value> assertions + --capture-grid
├── cf-mod                    # (real)  content schema validator + manifest walker + material registry validator
├── cf-chassis                # (real)  3 launch chassis archetypes (infantry_v1 / powered_armor_v1 / light_mech_v1) + 15-zone body graph + layered armor + module state machine + pilot binding + eject + salvage
├── cf-save                   # (real)  SaveBlob v1 with actor + chassis + rifle serialization + blake3 checksum (M5 slice of T-SAVE)
├── cf-material               # (real)  systemic material kernel (M2 baseline; M5.6 extends to 50+ materials + reactions)
├── cf-tools-replay-viewer    # stub    M3B: bundle viewer + cause-chain walker + debrief markdown emitter + validate + summary
├── cf-environment            # stub    M5.10: EnvironmentSignal aggregator (atmospheric, gravitational, thermal, radiation, photic, EM, weather, water, acoustic, day/night, comms)
├── cf-atmos                  # stub    Stationeers-grade-or-better atmospherics + thermal kernel (M5.9)
├── cf-audio                  # stub    sound + captions (M4..M7); ACRE2-tier voice + radio at M9.5
├── cf-net                    # stub    client/server transport (M9; lightyear vs renet vs quinn locks at M9)
├── cf-tools-editor           # stub    in-engine scenario / package / mod editors + Material Lab (M8 + M8.5)
├── cf-headless               # stub    CI-friendly headless runner (replay verifier)
├── cf-bench                  # stub    perf benchmark harness
├── cf-server                 # stub    multi-mode dedicated server (M9: coop_room / pvp_arena / lan_room / mmo_shard / lobby_directory)
├── cf-server-ops             # stub    ops dashboards + observability + /health + /ready + Prometheus metrics + drain shutdown (M9)
├── cf-server-persistence     # stub    Per-shard resource ledger + snapshot store + event journal (M9 minimum; M12 fills full MMO)
├── cf-server-anti-cheat      # stub    Input validation + rate limits + 3 profiles (casual / competitive / tournament_strict) (M9 foundation; M11 extends)
└── cf-server-admin           # stub    Admin commands + capability gating + kick / save / restart / hot_load / ban (M9)
```

---

## Project Status

> [!warning] Pre-alpha
> Corefall is in active development. The repo is public so CI can run unrestricted (free GitHub Actions minutes for public repos), but the game is **not** ready to play yet — first friend-handoff release lands at BP3 closure (see [Releases](#releases)).

**Workspace stats (last update 2026-05-12 / commit `3244a80`):** **545 tests passing** across 32 crates; cargo fmt + clippy `-D warnings` clean; `bp_test_coverage bp3` reports CLEAN with 0 gaps; M2 determinism CI matrix passes all 6 combinations.

**BP2 closure recap.** Chunked deformable terrain (M2) replaced the M1.5 soft-breach strip; M2.5 fun-proof scenario shows the player surviving a 60-90s reactor defense as terrain is dug + debris fields form; cf-headless re-runs any run bundle's `events.jsonl` deterministically (M3A). Engineering closed via PR #11 (BP2 milestone code) + PR #12 (source-truthful run bundles + AI-Agent Self-Test contract + LLM-graded test verdicts + per-BP test suite). M2 R2 round closed clippy `-D warnings` green, built the cf-render-2d M2 visual surface, and landed the determinism CI matrix. The **M2.2A/B/C bridge slice** is active in `specs/active/` and fills the gap between M2 baseline and M2.5 reactor defense with actor controller depth (36 actions), 6-weapon suite, 5 AI archetypes, mission director v0.5, 10+ HUD widgets, 7 debug overlays, color-blind variants, and pie menu UI.

**BP3 (active) constituent milestones.** M3B Replay Viewer + Debrief (commit `50af435`); M4A Readability + ACC-A Floor (PR #27, DR-003 + DR-012 closed); M5 Equipment / Chassis / Damage Grammar (commit `29edc1b`) — 3 chassis archetypes, body graph (15 zones, 14 joints), layered armor, module state machine, pilot binding + eject, save/load, 7 cfctl methods added, DR-014 + DR-021 closed. **BP3 closure gate has NOT passed:** MISSING_FEATURES Wave 1 inventory identifies ~1,100 foundation gaps; `bp_close_loop.sh bp3` has not produced an all-phases PASS verdict. Double-Click Playability release engineering not yet landed.

**Next up after BP3 closure.** BP4 (M5.5 Full Collision Gauntlet, M5.5.5 Micro Sabotage, M5.6 Material Kernel, M5.7 Hazard Package, M5.8 Origin Resource & Overclock Pass). BP3 must first close via MISSING_FEATURES Wave 1 completion + closure gate pass.

### Milestone matrix

| BP | Milestone | Status | What It Proves |
|---:|---|---|---|
| BP0 | **M0 — Engine Bootstrap** | Closed ([PR #1](https://github.com/Madreag/corefall/pull/1) merged) | 32-crate workspace, JSON-RPC control plane, cfctl, replay run-bundle writer, deterministic 60 Hz / 120 Hz sim, panic capture, CI matrix on Linux + macOS + Windows. |
| BP1 | **M1 — Actor Controller And Sim Core** | Closed ([PR #2](https://github.com/Madreag/corefall/pull/2) merged) | Single playable actor with movement, jump, aim, rifle fire, reload, status state machine (STABLE / UNSTABLE / DOWNED / DYING / DEAD), projectile flight, damage routing, stability + recoil + sharp aim + travel-impulse damage + DYING dwell + inventory drop. 9 `act.player.*` JSON-RPC methods. Tick-rate-independent rifle timing (10 RPS / 1.5 s reload identical at 60 Hz and 120 Hz). |
| BP1 | **M1.5 — Micro Breach Fun Slice** | Closed ([PR #5](https://github.com/Madreag/corefall/pull/5) merged) | 60-90 s win/loss scenario plays end-to-end via cfctl scripts. ReactiveGuard with DR-008 LEAN (jobs + utility + scripted hooks) fires deterministic seeded miss rolls. Soft breach emits M2-compatible `terrain_carved` events. Mission state machine + 4 outcomes (Won / Lost / Aborted / InProgress) + 3 difficulty presets (cakewalk / tough_crowd / veteran). AI-H-01 sentry-hears-threat scenario establishes the AI-H harness. |
| BP1 | **T-CAPTURE — Frame capture + grid composer** | Shipped ([PR #6](https://github.com/Madreag/corefall/pull/6) merged) | cf-capture crate + `capture_grid.py` composer + cf-e2e wiring. PNG readbacks at 10 Hz baseline + event-triggered keyframes. AI agents read `summary_grid.png` via the Read tool to validate motion + physics + effects without a human eyeballing every smoke run. T-CAPTURE evidence is mandatory at every BP closure gate from BP2 onward. |
| BP2 | **M2 — Pixel Terrain And Materials** | Closed ([PR #11](https://github.com/Madreag/corefall/pull/11) merged; M2 R2 finalized) | Deformable chunked terrain (256×256 chunks) + 8-material launch set (air / dirt / concrete / metal_nohook / hazard / loose_fill / repair_fill / anchor) + GPU-assisted carving + material overlay (5 modes) + tool-validity feedback + 9 affordance flags + penetration formula (impulse² > integrity²) + stickiness + spalling threshold. M2 R2 fixed engine RwLock re-entrancy, rebuilt cf-render-2d M2 visual surface (terrain.rs / overlay.rs / debris.rs / dig_preview.rs), and landed the determinism CI matrix. |
| BP2 | **M2.2A — Actor + Equipment + Inventory + Sound Bridge** | Active | 36 actor actions (sprint / crouch / prone / slide / vault / climb / dive / lean / stealth_kill / knife_throw / 26 more) + 6 weapons (rifle / SMG / shotgun / sniper / pistol / grenade_launcher) + 4 grenades + 4 melee + 7 tools + 8-slot inventory with weight + centralized sound propagation kernel + 1 friendly bot + 4 squad commands + side-view facing direction + limb-loss action restrictions. |
| BP2 | **M2.2B — AI Archetypes + Mission Director + Personality + Faction** | Active | 5 AI archetypes (Rifleman / Sniper / Assault / Engineer / Spotter) + cover seeking + suppression + retreat + squad comm + patrol + friendly fire awareness + 20+ personality traits + 3 squad doctrines (Defensive / Aggressive / Scout) + 3-faction relationship matrix + multi-objective DiGraph mission director + 4-phase pacing (Setup / Buildup / Climax / Debrief) + mini-boss patterns + AI-H-01..06 harness extension. |
| BP2 | **M2.2C — Camera + UX + Debug + Localization + Accessibility Extras** | Active | Camera (smooth follow + hit-stop + scope + free-look + slowdown) + 10+ HUD widgets (hotbar / minimap / compass / damage direction / squad strip / stamina / stealth meter / grenade arc / phase strip / branching banner) + settings menu (6 tabs) + 7 debug overlays (F1-F7) + localization (~500 keyed strings, English baseline) + color blind variants (4 modes) + aim assist + speedrun mode + damage numbers + custom HUD basic + pie menu radial UI + photo mode basic + replay scrubber + killcam + slow-mo kill cam. |
| BP2 | **M2.5 — Micro Reactor Defense + Deep Damage Surface** | Active | 60-90s defend-the-reactor scenario; chunked terrain-driven win/loss; cfctl-scripted. Plus the canonical **deep damage surface**: 5-tier terrain HP color states (Pristine / Scratched / Cracked / Critical / Destroyed); War Thunder angled armor foundation (effective_thickness = nominal / cos(angle); 9 armor types; HE / HEAT / APFSDS round tiers; spalling threshold + fragment count); per-limb armor + 15-zone body graph; internal organ + circuit damage routing; concussion + internal_shock dose bands; fluid leak channels (oil / coolant / fuel / electrolyte); origin force feedback surface; hazard taxonomy (5 launch hazards: fire / smoke / electric / wet / hot-cold); affliction taxonomy (18 kinds); atmospheric event surface (placeholder for M5.9); shield event surface (placeholder for M5+); environment.signal_delta surface (placeholder for M5.10). |
| BP2 | **M3A — Event Recorder Core + Headless Replay** | Closed ([PR #11](https://github.com/Madreag/corefall/pull/11) merged) | Deterministic event log + cf-headless replay verifier proves end-to-end determinism by re-running any run bundle's events.jsonl. **38 event categories** (27 baseline + 9 added at M2.5 + 1 at M5 + 1 at M5.8): input, control, mind, collision, server, anti_cheat, mmo, material, reaction, atmospherics, affliction, hazard, shield, thermal, environment, armor, module, resource, internal, concussion, fluid, origin, combat, body, terrain, ai, logistics, mission, system, snapshot, determinism, ux, accessibility, performance, equipment, chassis, actor, ability. M3B (Replay Viewer + Debrief) lands in BP3. |
| BP3 | **M3B — Replay Viewer + Debrief** | Closed ([commit `50af435`](https://github.com/Madreag/corefall/commit/50af435)) | Replay viewer (cf-tools-replay-viewer): view / cause-chain / debrief / validate / summary subcommands. 7 typed BundleError variants. 4 typed cause-chain terminations (RootReached / ParentMissingFromBundle / MaxDepthReached / CycleDetected). 18-section debrief markdown. Plain-language event template rendering (NEVER raw JSON to players). DR-002 closed. |
| BP3 | **M4A — Readability + ACC-A Floor** | Closed ([PR #27](https://github.com/Madreag/corefall/pull/27) merged) | 12 focusable HUD nodes across 7 zones. Body silhouette + module strip + ammo + objective + timer + last-event ticker + reactor pressure line + banner stack with severity glyphs. 5-tier integrity color signature reused from terrain to armor layers. Side-view body layout + pilot-inside-chassis dual silhouette + chassis HUD by weight class (Light / Medium / Heavy / Drone). Origin-specific resource bars (no master HP bar for chassis-bearing actors). War Thunder armor angle HUD widget. Internal silhouette overlay. Concussion vignette curves. Fluid drain HUD bars. Accessibility floor (200% scale + high contrast + reduced_* + focus traversal + hold-to-confirm + validated remap table + captions). DR-003 + DR-012 closed. **M4B comic-noir polish (mission cards, stylized banners, ink-line UI, juice rules) deferred to BP7 per Roadmap V2 split.** |
| BP3 | **M5 — Equipment, Chassis, And Damage Grammar** | Landed (commit `29edc1b`; BP3 closure gate pending) | 3 launch chassis archetypes (infantry_v1 / powered_armor_v1 / light_mech_v1), 15-zone body graph (head, torso, arms, forearms, hands, legs, shins, feet, backpack), 14 joints, 5 equipment sockets, layered armor (External / Internal / Core), module state machine (Nominal → Degraded → Warning → Failed → NotPresent), 11-stage damage pipeline, pilot binding + eject + bail-too-late + extraction lifecycle, save/load with chassis round-trip. 7 cfctl methods (`act.player.crouch / climb / jet / eject`, `act.chassis.repair / salvage / clear_jam`). 13 chassis event types. DR-014 + DR-021 closed. **Spec extends M2.5's reserved surfaces:** 5 chassis archetypes shipped (added crab/quadruped + drone), brain-hopping, cockpit camera anchor, chassis ability slots (8 launch abilities including time_stop / time_slow / cloak / EMP_pulse / gravity_well / overdrive), weapon modifier slots (30+ Noita-style stackable modifiers), drone allies (4 autonomous modes), hit reactions per body part, side-view body layout + facing direction, limb-loss functional consequences matrix, stance-aware hit-zone AABB tables, multi-zone passthrough, per-module positioning + War Thunder ray traversal, armor mounting angles per chassis archetype. |
| BP4 | M5.5 — Full Collision Gauntlet | Planned | DR-033 closure: full collision + projectile-projectile + CCD tiers + universal gravity field integration + swept-volume collision per CCCP `Atom::Travel` + limb detachment via joint impulse + gib spawning with authored data + ragdoll on death + per-organ internal damage routing + War Thunder penetration ray + HE overpressure + HEAT shaped jet + APFSDS long-rod + spalling fragment ray + hit-stop + camera punch + impact frame. |
| BP4 | **M5.5.5 — Micro Sabotage** | Planned | Fun-proof interlude after M5.5/5.6/5.7/5.8: 60-90 s sabotage scenario integrating collision + materials + hazards + origin. Chemistry path (oil ignition chain) + physics path (explosive breach) + stealth path (alarm suppressed) + origin path (human stealth + android overclock + robot vacuum-immune). |
| BP4 | M5.6 — Material Kernel | Planned | DR-036 closure: Noita-grade per-pixel cellular automata + 50+ materials + 30+ reactions (acid + iron → rust; water + lava → obsidian; gunpowder + fire → explosion) + alchemy table + flask system + GPU compute acceleration (CPU deterministic fallback). |
| BP4 | M5.7 — Hazard Package | Planned | 18-affliction full mechanics (burning / wet / electrified / poisoned / hypoxic / combustible_atmosphere / breach_decomp / hyperthermic / hypothermic / radiation / concussed / deafened / bleeding / internal_shock / low_battery / coolant_leaking / oil_leaking / overheating). 6 STALKER-inspired anomaly hazards (gravity / electric / time / chemical / bloodsucker / psy_storm). 20+ artifacts with passive bonuses. Swimming + underwater combat. |
| BP4 | M5.8 — Origin Resource & Overclock Pass | Planned | Per-origin reaction matrix runtime: humans concuss, androids battery-drain, robots overclock + leak coolant. **No-HP-bar canonical**: blood / oil / power / caloric / bio_fluid / oxygen_supply per origin. Per-organ + per-circuit cascade rules. G-Force vision blackout HUD per origin. Helmet + oxygen tank vacuum scenarios. Internal shock + downclock + overclock (voluntary + involuntary). |
| BP5 | M5.9 — Atmospherics-Grade Kernel | Planned | DR-037 closure: Stationeers-grade-or-better PV = nRT, 10 launch gases + 6 liquid mixtures, 6 deterministic combustion reactions with autoignition T, gradual phase change with latent heat, pipe networks with pumps / valves / regulators / filtration, suit life-support, pressure apertures, jets, heat transfer, universal-gravity ballistic drag. 6 launch worlds (Earth / Mars / Moon / Phobos / Mimas / Vulcan). |
| BP5 | **M5.9.5 — Micro Pressure Hold** | Planned | Fun-proof interlude after M5.9: 60-90 s hold a room while atmosphere is breached, gases mix, fires propagate, suits compensate. |
| BP5 | **M5.10 — Environmental Conditions Aggregation** | Planned | EnvironmentSignal aggregator (per DR-040): per-tick per-actor bundle aggregating atmospheric / gravitational / thermal / radiation / photic / EM / weather / water / acoustic / day-night / comms slices into one source of truth. 15-class hazard taxonomy + comms light-lag. |
| BP6 | M6 — AI Core And Pathfinding | Planned | Hierarchical Pathfinding A* (H-PA*) with 3 hierarchy levels (tile → chunk → region). Dynamic re-pathing on `terrain.terrain_dirty_region_batch`. Per-team path costs. Multi-actor collision avoidance via spatial hashing. Stuck recovery. Sleeping AI optimization. |
| BP6 | M6.5 — LLM Mind Lab | Planned | Async LLM mind layer; local AI never blocks; no API key required. Per-actor MIND ticks every 5 seconds (fully async). Doctrine proposal format with reason labels + confidence. Sandbox + safety (no file I/O, no network shell, no OS execute). AI Self-Test grading via MIND. |
| BP6 | M6.6 — AI Environmental Competence | Planned | 8-test AI-MAT acceptance suite. AI reads EnvironmentSignal per slice (atmospheric / gravitational / thermal / radiation / photic / EM / weather). Bots wear oxygen tanks in vacuum, retreat from radiation, seek cover from lightning, avoid combustible atmosphere fires, navigate gravity anomalies, downclock robots under heat, fall back to last-known orders in comms blackout. Reason labels for every environmental decision. |
| BP7 | M7 — Campaign + Base + Commander | Planned | Multi-mission progression. 5 storytellers (Cassandra / Phoebe / Randy / Ironman / Sandbox). Base building per DR-029 (command core + power grid + module slots + uproot/reroot). Buy menu + delivery craft. 8 stratagems (Helldivers-style). Persistent AI commander persona with named rivalry. 8 factions full system. NPC dialog + branching narratives. Crime scene investigation. Pet companions (4 types). Perk + curse altars (Noita-style). Loot rarity tiers (Common / Magic / Rare / Legendary / Unique). XP + level + achievements. Inventory grid Tetris (Stationeers). Manufacturing + cooking + plant growing. Treasure maps + voyages (Sea of Thieves). |
| BP7 | M7.5 — Base Atmospherics | Planned | Base modules wired into M5.9 kernel: pumps, vents, pressure doors, breach repair, heaters/coolers, radiators, coolant loops, emergency venting, and room-state mission objectives. |
| BP7 | **M7.7 — Day/Night, Weather & Dynamic Events** | Planned | Weather + day-night kernel. 7 weather states (clear / rain / storm / dust / fog / snow / acid_rain). 24-hour day/night cycle per world with AI behavior shifts. Remaining 3 worlds ship (Deimos / Europa / Venus + Sol-zone / Belt / Orbital = 12 total per README). |
| BP7 | **M4B — Comic-Noir Polish** | Planned | Comic-noir aesthetic (hand-drawn ink-line UI, comic-book speech bubbles, ink-style impact particles, dynamic color grading). Juice rules per DR-055 (button hover, click punch, banner slide, hit-stop, weapon swap whoosh, pickup glow). Mission briefing comic panels (12 launch). Death recap as graphic novel. All juice respects reduce_motion / shake / flash. |
| BP8 | M8 — Scenario Editor And Mod Tools | Planned | Full in-game scenario editor + Lua script editor + IC10 chip editor + mod manifest + Steam Workshop integration + photo mode (full filter palette + color grading + animation) + replay browser full scrubbable timeline + custom HUD editor + 8 modular tutorial labs + mod parity contract per DR-045. |
| BP8 | M8.5 — Material Lab | Planned | `cf-tools-editor --mode material_lab` workbench per DR-036. Brushes + material palette (17 launch + 13 expansion unlocked via lab) + recipe inspector + stamp save/load (Powder Toy pattern) + snapshot/delta undo + test-run with replay capture + AI puppet validation. 5 launch material-lab mod templates. |
| BP8 | **M8.6 — Mining And Extraction** | Planned | Full mining pipeline per DR-041: sample → drill → extract → refine → smelt → use. 9 mining tools (Sampler / LightDigger / HeavyDrill / CoreDrill / RefiningStation / SmelterFurnace / EnrichmentReactor / OreCargoBay / ConveyorBelt). 12 launch ores (iron / gold / copper / uranium / ice / sulfur / coal / lithium / titanium / lead / nickel / tin). AI-MINE-A 8-test acceptance suite. Server-authoritative resource ledger replicates across shards. |
| BP9 | M9 — Dedicated Server App + Determinism Islands | Planned | `cf-server` multi-mode binary with 5 launch modes (coop_room / pvp_arena / lan_room / mmo_shard / lobby_directory). cf-server-ops (config + health + readiness + metrics + drain shutdown). cf-server-persistence (snapshot writer + event journal). cf-server-anti-cheat (3 profiles: casual / competitive / tournament_strict). cf-server-admin (kick / save / restart / hot_load / ban). Mod whitelist/blacklist with 4 trust tiers. 5 server config templates (deathmatch / coop_campaign / pvp_arena / endless_wave / sandbox). SERVER-001..016 acceptance suite. DR-052 networking transport library decision locks at M9 (lightyear vs renet vs quinn). |
| BP9 | M10 — LAN Co-op | Planned | Local 2-4 player co-op through `cf-server --mode lan_room`. mDNS / UDP broadcast LAN discovery. Server-authoritative simulation with client prediction + reconciliation. Per-client replay bundles align tick-for-tick. Co-op friendly fire policy (configurable). 7 squad role types at lobby. Revive mechanic. Death cam. LAN scanner + quick-host wizard. Anti-cheat profile `casual` default. Mod hash sync with clean diff UI on mismatch. |
| BP10 | M11 — Online Co-op (Self-Hosted Dedicated Servers) | Planned | Community-hostable online co-op via `cf-server --mode coop_room`. NAT punch-through + relay (Steam Datagram Relay / EOS adapters available). `lobby_directory` for server registration + discovery. Latency masking at 50-150 ms RTT. Anti-cheat profile `competitive` default. **Full match grammar per DR-042**: 10 endgame modes (Bunker Defence / Bunker Attack / Salvage Run / Tournament / Wave Survival / Boss Rush / Stealth Challenge / Time Attack / Permadeath Campaign / Cross-Shard Events). Multi-squad command (4+ squads, 7 role types, 12 commands). Persistent AI commander rivalry across online sessions. Persistent veteran actors + scars. Bunker meta-game persistence. War Thunder-style polished kill cam. Spectator mode (full). Friend lobby + Steam Friends integration. Network sync verified per DR-056. |
| BP10 | **M9.5 — Voice + Radio Sim** | Planned | ACRE2-tier radio + Steam Audio-tier voice through atmospheric medium per DR-043. Proximity voice chat (3 channels: Squad / Public / Faction). Radio set propagation (distance attenuation + occlusion + frequency band + antenna height). Interference + EMP. Vacuum no_voice (sound doesn't propagate; radio still works). AI radio chatter with reason labels. Captions mandatory per DR-012. |
| BP11 | M12 — Public PvP Arenas + Persistent MMO Shards | Planned | Public PvP arena half (`cf-server --mode pvp_arena`). MMO shard half (`cf-server --mode mmo_shard`). **Bunker Defence flagship mode** (per DR-042) with persistent base across sessions + per-server-shard tournament ladder. Persistent world state (4 launch shards: NA / EU / APAC / SA). Cross-shard events. Faction-vs-faction war at large scale. MMO shard architecture (per DR-035). Persistent player progression. Anti-FOMO + anti-pay-to-win audit (DR-031) — all cosmetics earnable through play. Live operations (T-LIVEOPS finalization). DR-035 MMO-001..012 readiness gate. |
| BP12 | T-CONTENT-ART / T-CONTENT-NARRATIVE / T-LOCALIZATION / T-LIVEOPS finalization | Planned | All four production tracks reach launch GA. Full content roster verified (70+ weapons / 44+ actors / 18+ vehicles / 60+ base objects / 8 factions / 30+ missions / 12 worlds × 3-5 biomes / 17 materials / 12 ores / 30+ music / 400+ SFX / 80,000 words / 600 codex / 75+ achievements / 11+8 languages / 10 endgame modes / 50+ cosmetics per actor). T-RELEASE GA at v1.0.0. |

---

## Planning Spine + Research Vault

Corefall splits its plan-of-record across two locations:

### Planning spine — inside this repo at [`docs/plan/`](docs/plan/)

The **implementation-gating** planning layer lives in this repo so every PR that changes a roadmap row, checklist row, DR, milestone enhancement spec, or other gating contract is reviewed by Bugbot + Devin alongside the implementation that depends on it. Atomic plan + code PRs.

- **Decision records** (DR-001 through DR-057, plus future activation gates) — every major direction choice with pros, cons, evidence, revisit triggers. Lives at [`docs/plan/decisions/`](docs/plan/decisions/).
- **80 spec pages** for product promise, body damage, chassis/armor/mechs/origins, equipment/loadout, Stationeers-grade-or-better atmospherics & chemistry, thermal engineering, gravity & ballistics, AI, replay, mission director, full collision physics, accessibility-plus, localization, AI asset/audio production, modding, networking, launch operations, and more. Lives at [`docs/plan/spec/`](docs/plan/spec/).
- **Production roadmap** covering M0 through launch, side tracks, CLI/control contracts, DR-056 universal enhancement gates, and per-milestone Steam Deck/network/replay/accessibility/modding/testability budgets. Lives at [`docs/plan/spec/prototype-roadmap.md`](docs/plan/spec/prototype-roadmap.md).
- **35 active milestone specs** in [`specs/active/M2.2A..M12.md`](specs/active/) — the executable implementation contracts for the gameplay spine. Each is read-by-implementing-agent-only per AGENTS.md.
- **3 closed milestone specs** in [`specs/done/M1.md / M1.5.md / M2.md`](specs/done/) — kept for audit trail.
- **Native implementation backlog** + **feature completion checklist** + **milestone enhancement spec** + **AI-coder reading list** + **ai-control-observability-layer** + **authoritative-game-spec** + **prototype-run-bundle-schema** + **decision-tracker dashboard** + **research-readiness dashboard** — all under `docs/plan/spec/`, `docs/plan/dashboards/`, and `docs/plan/references/`.
- **BP closure notes** at [`docs/plan/prototypes/build-point-bp*.md`](docs/plan/prototypes/) — per-BP narrative + evidence trail.

Full file history is preserved via `git filter-repo` from the research vault (every commit before the migration is visible in `git blame` on each spine file).

### Research vault — outside this repo at `~/projects/cortex-command-repos-all/cortext_command_vault`

Long-form research that informs but does not gate implementation:

- **Comparable game audits** — local code audits of Cortex Command (CCCP), OpenSoldat, OpenLieroX, The Powder Toy, plus public-source / public-doc research on Noita, Stationeers, Barotrauma, War Thunder, ACRE2, Oxygen Not Included. Lives at `cortext_command_vault/comparables/`.
- **Research log** — chronological record of every research pass with source citations. Lives at `cortext_command_vault/research-log/`.
- **Per-milestone prototype evidence notes** (`prototypes/native-*.md`) — narrative log of what shipped at each milestone, distinct from the in-repo `docs/implementation-log/` which captures what changed in this repo at that milestone.
- **Narrative seeds**, **comparable repos** (CCCP, OpenSoldat, OpenLieroX, Powder Toy), **license usage ledger**, **equipment schema seeds**, **glossary**, **strategy docs**, **systems brainstorms**, **game-overview docs** (`VAULT_PLAN.md`, `GAME_DESCRIPTION_FOR_FRIEND.md`).

The vault stays separate so it can survive engine changes, language changes, or fork events. It's also where exploratory research lives without polluting the implementation repo's PR review surface.

> [!note]
> The vault is currently a private workspace. If you want to contribute design research or comparable-game audits, open an issue here so we can route the conversation.

---

## Releases

Released artifacts live at [github.com/Madreag/corefall/releases](https://github.com/Madreag/corefall/releases). Every Build Point closure publishes a tagged cross-platform release per the **T-RELEASE** side track.

### Double-Click Playability Hard Gate

A non-technical friend receiving the release file MUST be able to:

1. **Double-click the file** → standard OS extract/install (no Terminal, no `brew install`, no command-line decompression).
2. **Double-click the resulting app** → a Corefall game window opens (no `--scenario` flag, no PowerShell, no command-line args).

If either fails, the platform is omitted from the BP's release matrix; if no platform meets the gate, the BP **skips** its release tag entirely. Skipping is preferred over publishing an opaque archive. The next BP that lands the missing engineering recovers the skipped releases.

| Platform | Format | Friend's experience |
|---|---|---|
| **macOS** | `.dmg` containing `Corefall.app` | Mount the `.dmg`. Drag `Corefall.app` to Applications. Double-click → game window. |
| **Windows** | `.msi` installer OR `.zip` with `Corefall.exe` (default-args launcher) | Run the `.msi` (Start Menu shortcut), or unzip + double-click `Corefall.exe` → game window. |
| **Linux** | AppImage (single double-click executable) | `chmod +x Corefall.AppImage` + double-click → game window. |

Code signing (Apple notarization + Windows Authenticode) activates at BP10+ via T-LIVEOPS pre-launch wiring; through BP9, expect a one-time platform warning ("right-click → Open" on macOS; "More info → Run anyway" on Windows).

### Versioning

**Channel-based SemVer prerelease tags** — the channel suffix in the tag carries the quality signal so external observers (Steam buyers, contributors, package managers) can read it without consulting the BP table:

| Channel | Tag form | BPs |
|---|---|---|
| **prealpha** | `v0.<N>.0-prealpha` | BP0..BP3 (engine + first fun slices; major systems still missing) |
| **alpha** | `v0.<N>.0-alpha` | BP4..BP6 (full collision + atmospherics + AI combat) |
| **beta** | `v0.<N>.0-beta` | BP7..BP9 (mission director + creator alpha + server / LAN) |
| **rc** | `v0.<N>.0-rc` | BP10..BP11 (online + public systems beta) |
| **GA** | `v1.0.0` | BP12 launch |

Boundaries are enforced by `game/tools/generate_release_notes.py::parse_tag` (e.g., `v0.4.0-prealpha` is rejected because BP4 must ship under `alpha`). Legacy `v0.<N>.0-bp<N>` tags from earlier prototyping are still accepted for backward compat. The GitHub `prerelease` flag is intentionally NOT set — it would hide releases from the homepage Releases sidebar widget + `/releases/latest` endpoint, both of which filter out prereleases by GitHub's definition.

### Current release debt

`v0.1.0-prealpha` (BP1) and `v0.2.0-prealpha` (BP2) were tagged + published on 2026-05-09 then **DELETED the same day** because the artifacts were `.tar.zst` archives requiring `brew install zstd` + Terminal extraction (failed the Double-Click Playability Hard Gate). Both releases re-publish when the BP3 implementing agent lands the `.dmg` / `.msi` / AppImage engineering, alongside `v0.3.0-prealpha`.

See [`docs/plan/spec/prototype-roadmap.md` § T-RELEASE](docs/plan/spec/prototype-roadmap.md#t-release--per-bp-cross-platform-github-releases) for the full contract + AGENTS.md § Build Point Closure Gate for the agent's release-engineering responsibilities.

---

## Getting Started

### Prerequisites

| Tool | Version |
|---|---|
| Rust toolchain | 1.95.0 (pinned via `game/rust-toolchain.toml`) |
| Cargo | bundled with rustup |
| Python | 3.11+ (for `game/tools/prototype_run_check.py` and `game/tools/dependency_drift_report.py`) |
| OS | Linux + macOS + Windows; Steam Deck floor target |

### Build And Run

```bash
git clone https://github.com/Madreag/corefall.git
cd corefall/game

# Workspace sanity
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
python3 tools/dependency_drift_report.py --workspace-root . --format markdown

# Smoke runs (M0 blank + M1 actor range + M1.5 micro breach + M2 terrain + M2.5 reactor + M5 chassis)
cargo run -p cfctl -- observe --once --scenario m0_blank
cargo run -p cfctl -- run --scenario m0_blank --ticks 300 --tick-rate-hz 60 --paced --write-run-bundle
cargo run -p cf-app -- --scenario m0_blank --headless-smoke --run-seconds 5 --write-run-bundle

# Play the M1 actor range (windowed): WASD = move, Space = jump, arrows = aim,
# Enter / J = fire, R = reload, L = reset, 1-4 = inventory slot, Esc = quit.
cargo run -p cf-app -- --scenario m1_actor_range

# Play the M1.5 Micro Breach Fun Slice (windowed; same controls + KeyG to dig)
cargo run -p cf-app -- --scenario micro_breach

# Replay the win or loss script through cfctl
cargo run -p cfctl -- script run scripts/cfctl/micro_breach_win.cfctl.json --write-run-bundle
cargo run -p cfctl -- script run scripts/cfctl/micro_breach_loss.cfctl.json --write-run-bundle

# Play the M2 chunked terrain sandbox (M = cycle material overlay modes)
cargo run -p cf-app -- --scenario m2_material_lane

# Play the M2.5 Micro Reactor Defense Fun Slice
cargo run -p cf-app -- --scenario micro_reactor_defense

# Play the M5 chassis wreck-eject scenario
cargo run -p cf-app -- --scenario m5_chassis_wreck_eject

# Validate any run bundle
python3 tools/prototype_run_check.py ../prototype_runs/native/m0_*
python3 tools/prototype_run_check.py ../prototype_runs/native/m1_*
python3 tools/prototype_run_check.py ../prototype_runs/native/m1.5_*
python3 tools/prototype_run_check.py ../prototype_runs/native/m2_*
python3 tools/prototype_run_check.py ../prototype_runs/native/m5_*
```

### CLI Reference

`cfctl` is the operator + AI control client. The full surface is documented in the canonical roadmap (CLI Reference section); the currently-shipped subset is:

| Command | Milestone | What |
|---|---|---|
| `cfctl observe --once --scenario <id>` | M0 | One-shot snapshot of game state. |
| `cfctl observe --stream --hz <N>` | M0 | Stream observation frames at N Hz. |
| `cfctl run --scenario <id> --ticks <N> --paced --write-run-bundle` | M0 | Run a scenario for N ticks paced to wall clock. |
| `cfctl scenario load <id> [--seed <N>]` | M0 | Load a scenario (seed override is M0-rejected). |
| `cfctl pause` / `step --ticks <N>` / `version` | M0 | Sim control + protocol version. |
| `cfctl act player-move --x <-1..1>` | M1 | Continuous horizontal movement intent (latest-value-wins). |
| `cfctl act player-jump` | M1 | Edge-triggered jump on the next tick. |
| `cfctl act player-aim --x <f32> --y <f32>` | M1 | Set aim vector (NaN/Inf rejected). |
| `cfctl act player-fire [--pressed true\|false]` | M1 | Edge-triggered rifle fire. |
| `cfctl act player-reload` | M1 | Begin reload (1.5 s real time at any tick rate). |
| `cfctl act player-select-item --slot <0..3>` | M1 | Switch inventory slot. |
| `cfctl act player-reset` | M1 | Respawn at scenario position with full HP / ammo / slot 0. |
| `cfctl act player-dig [--target <breach_id>]` | M1.5 | Edge-triggered terrain dig. Rejects `out_of_range` / `material_metal_nohook` / `already_broken` / `unknown_target`. |
| `cfctl act player-toggle-material-overlay [--mode <off\|integrity\|pathability\|mobility\|hazard\|build_repair>]` | M2 | Cycle 5-mode material overlay. |
| `cfctl act player-crouch` | M5 | Toggle crouch stance. |
| `cfctl act player-climb` | M5 | Toggle climb stance. |
| `cfctl act player-jet` | M5 | Toggle jet thrust (requires Jet module on chassis). |
| `cfctl act player-eject` | M5 | Initiate pilot eject from chassis. |
| `cfctl act chassis-repair --zone <zone>` | M5 | Repair a damaged chassis zone. |
| `cfctl act chassis-salvage` | M5 | Salvage modules from a wreck. |
| `cfctl act chassis-clear-jam` | M5 | Clear weapon jam on chassis weapon mount. |
| `cfctl inspect actor <id>` | M5 | Inspect full actor state including chassis + origin. |
| `cfctl inspect chassis <id>` | M5 | Inspect chassis state (zones, modules, stage, pilot, eject window). |
| `cfctl inspect material <id>` | M2 | Inspect MaterialDef with full 9-flag affordance grid. |
| `cfctl observe terrain` | M2 | Snapshot of terrain state (chunk count, dirty chunks, material distribution, overlay mode). |
| `cfctl script run <path>` | M1 | Replay a `.cfctl.json` script (auto-launches `cf-app` with the right scenario). |
| `cf-app --capture-grid --capture-frames-hz 10` | T-CAPTURE | Emit PNG frame readbacks at 10 Hz baseline (configurable) + event-triggered keyframes into `<run_bundle>/captures/`. |
| `python3 game/tools/capture_grid.py <run_dir>` | T-CAPTURE | Compose `frame_*.png` into 8×8 `grid_NNN.png` + `summary_grid.png` with tick + event-label overlays. |
| `cf-e2e --script <path> --capture-grid --expect <key>=<value>` | T-CAPTURE | One-shot: launch cf-app windowed, replay script, compose grids, assert. |

Post-BP3 CLI extensions (atmospherics, materials reactions, gravity, ballistics, origin-state, suit, pipe-network, room, voice / radio, server, MMO) are planned for BP4+ and documented in [the canonical roadmap](docs/plan/spec/prototype-roadmap.md). They are NOT shipped at BP3.

---

## CI

GitHub Actions runs on every push and PR:

- `cargo fmt --all -- --check` (with `.gitattributes` locking LF line endings cross-OS)
- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace` (**545 tests passing** as of M2 R2 close: workspace-wide coverage including NaN/Inf guards, dwell-pause off-by-one regressions, `--headless-smoke` + `--capture-grid` rejection, M2 + M2.5 + M3A + M5 scenario tests, cf-headless replay-determinism harness, channel-aware install-section tests, terrain material affordance + dirty-region coalescing, chassis layered armor + pilot eject lifecycle, save round-trip with chassis state)
- `cargo build --release`
- Dependency drift report on the Linux leg (`tools/dependency_drift_report.py`)
- `cf-mod validate content/` (validates M0 + M1 + M1.5 + M2 scenario manifests + material registry)
- Schema drift check (`dump_schemas --check`) against static schemas under `crates/cf-control/schemas/v1/`
- `cfctl observe smoke` + `cfctl run smoke` (60 Hz + 120 Hz)
- `cf-app headless run-seconds 5`
- M1.5 cf-e2e win + loss script smoke
- M2 determinism matrix (`scripts/ci/m2_determinism_matrix.sh`): 6 combinations PASS
- M1.5 determinism matrix: 11/11 PASS
- M1 determinism matrix: 16/16 PASS
- Validate every produced run bundle through `tools/prototype_run_check.py`
- Enforce repo-root `prototype_runs/` path (M0.4-F7 guard)

Matrix: Linux + macOS + Windows.

The dependency drift report is intentionally advisory. It flags direct dependencies that have newer registry releases and prints a capped `cargo tree -d` sample, but it does not fail CI by default because duplicate transitive versions can be expected while upstream Bevy / wgpu / windows crates migrate.

---

## AI Code Review

This repo uses **Cursor Bugbot** as a GitHub App for advisory PR review. Bugbot's loop runs three iterations per push, and autofix commits are authored as `Cursor Agent <cursoragent@cursor.com>`. We treat Bugbot's findings and autofixes as **advisory**, not authoritative — every Cursor Agent commit is audited against the actual source before merge, and false positives are reverted via `git revert` (not force-push) so the audit trail stays intact. See [`AGENTS.md` § Cursor Bugbot Loop](AGENTS.md) for the full protocol.

The repo also ships a project-local Claude Code review skill at [`.claude/skills/corefall-review/`](.claude/skills/corefall-review/) (mirrored at `.agents/skills/corefall-review/`) that runs a deeper review pass (diff review, full affected-code review, contract gap review, edge-case hunt, test audit, determinism / replay review, security, performance, `cfctl` observability, vault coherence, synthesis judge). Invoke via `/corefall-review <milestone-or-range>`. The skill includes a milestone-ladder reference table mapping BP0..BP12 to specific milestones, canonical scope correction notes (so future agents don't misinterpret M8.5 as mod parity, M9 as multiplayer foundation, etc.), and cross-cutting contract verification matrices for the no-HP-bar / War Thunder armor / side-view body / stance-aware hit zones / War Thunder kill cam / penetration ray families.

---

## Contributing

Right now Corefall is in early implementation. The vault is the design source; this repo executes against it. If you want to contribute:

1. **File an issue first** — propose what you want to work on and we'll align it with the next milestone.
2. **Read [AGENTS.md](AGENTS.md)** — the full agent contract (covers AI workers and human contributors equally). Especially the Milestone Authority Stack, Milestone Acceptance Gate, Contract Integrity Gate, No-Compromise Performance Defaults, and Cursor Bugbot Loop sections.
3. **Branch from `origin/main`** with a milestone-prefixed name (`m1/scoped-feature` etc.). Direct commits to `main` are allowed for solo prototyping; PRs are required for any non-trivial change.
4. **Run Standard Validation locally** (`cargo fmt`, `cargo check`, `cargo clippy -- -D warnings`, `cargo test`) before pushing.

Per-crate `AGENTS.md` files in each `game/crates/cf-*/` directory describe owned APIs, public boundaries, common pitfalls, and source trails.

---

## License

Corefall is dual-licensed under your choice of:

- [Apache License 2.0](https://www.apache.org/licenses/LICENSE-2.0)
- [MIT License](https://opensource.org/licenses/MIT)

This is the standard Rust ecosystem dual-license, chosen so users can pick whichever license is most compatible with their project. Workspace-level license declaration lives in [game/Cargo.toml](game/Cargo.toml). Per-file `SPDX-License-Identifier` headers ship as the codebase grows.

> [!note]
> Inspiration credits and a usage ledger for any externally-derived material are tracked in the canonical vault's `references/usage-ledger.md`. No code, assets, sprites, or audio from the inspiration games is copied into Corefall.

---

## Acknowledgements

Built on Rust. Built on Bevy. Inspired by Cortex Command, Noita, Stationeers, Barotrauma, War Thunder, ACRE2, The Powder Toy, OpenSoldat, Liero / OpenLieroX, Teardown, Oxygen Not Included, STALKER, Helldivers, Diablo, Rimworld, Sea of Thieves, and Rain World. Made possible by every open-source maintainer who took the time to write a wiki, publish a GDC talk, push public source, or answer a Steam discussion thread at 2 AM.

---

<div align="center">

**[Headline Systems](#headline-systems) · [Launch Content Roster](#launch-content-roster) · [Project status](#project-status) · [Build Points](#build-points) · [Inspirations](#inspirations-and-credits) · [Tech stack](#tech-stack) · [Getting started](#getting-started) · [License](#license)**

*One field for gravity. One kernel for atmospheres. One source of truth for everything. No HP bars — only blood, oil, and power.*

</div>
