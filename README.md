<div align="center">

# Corefall

**A tactical pulp sci-fi disaster sandbox.**
**Server owns truth. GPU owns richness. Client owns feel.**
**Parallel-deterministic sim. Byte-identical replays across Linux, macOS, and Windows. 200 actors at 60 Hz.**

[![Rust 1.95](https://img.shields.io/badge/Rust-1.95.0-CE422B?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org)
[![Bevy 0.18.1](https://img.shields.io/badge/Bevy-0.18.1-232326?style=flat-square&logo=bevy&logoColor=white)](https://bevyengine.org)
[![wgpu](https://img.shields.io/badge/wgpu-render-FFCC00?style=flat-square&logo=webgpu&logoColor=black)](https://wgpu.rs)
[![Tokio](https://img.shields.io/badge/Tokio-async-3F8FFF?style=flat-square&logo=tokio&logoColor=white)](https://tokio.rs)
[![License](https://img.shields.io/badge/license-Apache--2.0%20OR%20MIT-blue?style=flat-square)](#license)

[![CI](https://img.shields.io/github/actions/workflow/status/Madreag/corefall/ci.yml?branch=main&style=flat-square&label=CI)](https://github.com/Madreag/corefall/actions)
[![Linux](https://img.shields.io/badge/Linux-supported-FCC624?style=flat-square&logo=linux&logoColor=black)](#)
[![macOS](https://img.shields.io/badge/macOS-supported-000?style=flat-square&logo=apple&logoColor=white)](#)
[![Windows](https://img.shields.io/badge/Windows-supported-0078D6?style=flat-square&logo=windows&logoColor=white)](#)

[![Status](https://img.shields.io/badge/status-prealpha-orange?style=flat-square)](#project-status)
[![Milestones](https://img.shields.io/badge/milestones-15%20done%20%2F%2059%20active-2EA043?style=flat-square)](#roadmap)
[![Crates](https://img.shields.io/badge/crates-44-blueviolet?style=flat-square)](#workspace)
[![Lib tests](https://img.shields.io/badge/lib%20tests-1248%20passing-2EA043?style=flat-square)](#ci)
[![Ledger](https://img.shields.io/badge/ledger-6718%20fresh-2EA043?style=flat-square)](#asset-ledger)

**[Pillars](#headline-systems) · [Roadmap](#roadmap) · [Workspace](#workspace) · [Performance](#performance--determinism-contract) · [Getting started](#getting-started) · [CLI](#cli-reference)**

</div>

---

## What is Corefall

A 2D side-view physics sandbox where every gas, grain, bullet, body, world, transmission, and joint is real. You command actors, swap into chassis, breach bunkers, defend command cores, dig terrain, vent rooms, light atmospheres, lose limbs, and reason about damage through real causal graphs — not abstract HP bars.

It's a **best-of-genre synthesis** that takes Cortex Command's command-core / dropship / chassis / digging fantasy and sets it on top of Stationeers-grade-or-better atmospherics, ONI-grade closed-loop life support, Noita-grade systemic materials, full collision physics, universal gravity, ACRE2-tier voice + radio simulation, and War Thunder-grade armor + spalling. AI bots are first-class teammates and rivals. Replay is deterministic. Modding is data-first. Accessibility is a floor, not an afterthought.

> [!important]
> **Where we are (2026-05-15).** 15 milestones closed (`M1`-`M11A` train + `M3A`/`M4A`/`M8A`/`M9A`). Tier 1 production backbone shipped: **6,718 fresh ledger entries** spanning 6,114 visual SVG/PNG assets across 23 composers + 242 SFX + 242 voice lines + 120 music WAVs + 83 narrative bible files. **Tier 2 ElevenLabs bake shipped** (`M12A` + `M37A` in flight): 242/242 SFX upgraded, 242/242 voice lines baked via `eleven_v3`, 39/120 music tracks baked via `music_v1`. New `cf-shell` crate (12 modules, 56 tests) + new `cf-audio::AudioRegistry` ship the shell UI foundation + adaptive-music index. Next up: `M12` (vivid color-rich illustrated juice + comic-panel transitions). Handoff docs for the remaining 81 music tracks live at `HANDOFF_LOCAL_MUSIC_BAKE.md` (RTX 5090 / ACE-Step v1.5) and `HANDOFF_AIVA_MUSIC.md` (AIVA Pro Playwright).

---

## At a Glance

| | |
|---|---|
| **Engine** | Bevy 0.18.1 + wgpu + custom sim core, Tokio for the JSON-RPC control plane |
| **Language** | Rust 1.95.0 (pinned via `game/rust-toolchain.toml`) |
| **Workspace** | 44 crates (33 real + 11 stubs) · **1,248 lib tests passing** |
| **Determinism** | Same seed + same inputs → byte-identical event stream on Linux + macOS + Windows |
| **Networking** | QUIC via `quinn`; LAN lockstep + internet rollback; semantic terrain events |
| **Asset ledger** | **6,718 fresh entries** — every shipped asset is regenerable from prompt + seed |
| **Reference platform** | AMD Ryzen 9 9950X3D + RTX 5090 + 48 GB DDR5 — 200 actors + 500 projectiles + 1000 hazard pixels at sustained 60 Hz |
| **Server tiers** | Workstation x86_64 · Apple Silicon (Mac mini M4 Pro+) · Linux VPS · Apple Mini Lab cluster |
| **License** | Apache-2.0 OR MIT (your choice) |

---

## Headline Systems

The pillars that make Corefall its own genre, not a clone of any of its inspirations.

| System | What's different about it |
|---|---|
| **No HP bar — origin survival resources** | Actors don't have abstract HP. Humans have **blood**; robots have **oil + power**; androids have **blood + power**; heavy biomechs have **bio-fluid + bio-energy**. Damage routes to specific resources via specific organs / circuits. Every death is causally explainable. |
| **War-Thunder-grade armor** | Real `effective_thickness = nominal / cos(impact_angle)` math. 9 armor types. AP/HE/HEAT/APFSDS round tiers. Spalling fragments + penetration ray traverses chassis modules in order. |
| **Side-view 2D body + functional limb loss** | True side-profile sprites (left/right facing). Per-stance AABB hit-zone tables. Arm lost = weapon dropped; both legs = forced crawl (25% speed); head destroyed = instant death; backpack lost = jetpack offline. |
| **Stationeers-grade-or-better atmospherics** | Real `PV = nRT`. 10 launch gases (O2 / N2 / CO2 / Volatiles / H2 / N2O / Steam / Ozone / He / Pollutant) with full combustion stoichiometry. Pipe networks as first-class atmospheres. Room atmospheres with door / breach / suit-puncture / bullet-hole apertures. |
| **Noita-grade systemic materials** | 50+ materials, 30+ reactions (acid + iron → rust; water + lava → obsidian; H2 + O2 + fire → steam + heat). Per-pixel cellular automata. Alchemy + flask system. GPU compute with CPU deterministic fallback. |
| **ACRE2-tier voice + radio** | Proximity voice. Realistic radio propagation (distance + occlusion + frequency band + antenna height). EMP disrupts radio 5-30s. Solar flares disrupt for minutes. Vacuum: sound doesn't propagate; radio still works. All voice + radio captioned. |
| **AI as teammate and rival** | 6 archetypes (Medic / Engineer / Rifleman / Sniper / Assault / Spotter). 5-layer thinking stack (Reactive → Utility → Behavior Tree → HTN Planner → optional LLM Mind) at 80 µs/bot on 16 cores. Per-actor `BotMemory` — bots remember where they got shot and avoid the same kill zone. Persistent AI commanders remember player tactics across missions. |
| **Smart commandable squad** | Three-layer control: per-actor **Autonomy mode** (FullAuto/Standard/Manual) + 22-task × 1-9 weight **Priority Table** (RimWorld + ONI pattern) + **Live Orders** (Q-hold context wheel + single-key panic + MMB tag-and-mark + Tab tactical overlay with 8-step Plan Composer). Every action emits a `reason_label` — F1 "Why?" key surfaces it in-mission. |
| **Brain-hopping** | Cortex Command's defining feature. Designated brain actor; transfer control to any friendly via `act.player.brain_hop`. Brain death = mission lost. |
| **Parallel-deterministic ECS** | Snapshot-read / compute-parallel / commit-serial (Factorio pattern). 50+ actors + 200 projectiles + 100 hazard pixels + reactor armor + per-pixel terrain integrity at p99 ≤ 16.6 ms on the reference platform. No `f64`, no `thread_rng()`, no default-hashed `HashMap` in sim crates — CI-enforced. |
| **GPU offload — cosmetic richness without sim cost** | Particles, debris, damage numbers, blood splatter, shell ejection — all GPU-side via WGSL compute + fragment shaders, all `cosmetic: true`, all droppable under backpressure with zero impact on sim. 10,000 sparks during a reactor breach; sim doesn't know. |
| **Server-authoritative, multi-tier hardware** | One `cf-server` binary, 5 launch modes (`coop_room`, `pvp_arena`, `lan_room`, `mmo_shard`, `lobby_directory`), 3 anti-cheat profiles, 4 mod trust tiers. Same binary on workstation x86_64, Apple Silicon, or Linux VPS — all pass the same cross-OS determinism gate. |
| **Modding as a first-class promise** | Every base-game feature is mod-reachable. Material Lab workbench authors systemic puzzles in <10 minutes. Mining + extraction pipeline. Lua + IC10 script hosts (sandboxed). Steam Workshop integration. |
| **10 playable races** | Human · Android · Robot · Powered Organic · Heavy Biomech · **Insectoid** (chitin armor) · **Crystalline** (radiation-immune; Sol-zone native) · **Photosynthetic** (CO2 breather) · **Aqueous** (Europa native) · **Methane Breather** (Mimas native; oxygen is poison). Per-race × per-environmental-factor matrix; each race has a **promised land** where they thrive. |
| **ONI-grade closed-loop life support** | Electrolyzer (water → 0.888 kg/s O2 + 0.112 kg/s H2). Per-tile thermal conductivity with material-specific values. 13 launch geyser types with eruption cycles + taming infrastructure. Polluted oxygen + polluted water bipartite resource model. Algae / plant farming + critter ranching for renewable food. |
| **Configurable everything** | 200+ tunables across 17 subsystems with a 7-tier hierarchy (engine → system → profile → scenario → server → CLI → runtime). Auto-generated settings UI from schema. Audit log. Settings included in the replay determinism contract. |
| **Accessibility as a floor** | 200% UI scale + high contrast + color-independent state labels + reduce motion / shake / flash + hold-to-confirm + focus traversal + key remap + captions for ALL audio. 8 color-blind protocols. G-Force vision blackout scales per origin. |

---

## Project Status

> [!warning]
> **Pre-alpha.** Not ready for non-technical players yet. The first friend-handoff release (`.dmg` / `.msi` / AppImage — double-click to play, no Terminal) lands per the [Double-Click Playability Hard Gate](#releases).

### What's shipped

- **15 milestones closed** (`specs/done/`): M1 / M2 / M3 / M3A / M4 / M4A / M5 / M6 / M7 / M8 / M8A / M9 / M9A / M10 / M11 / M11A.
- **6,718 ledger entries** across all asset categories — every shipped asset is regenerable from prompt + seed via `cf-mod ledger regenerate <id>`.
- **523 Tier 2 audio assets baked tonight** (`M12A` + `M37A`): 242 voice lines via ElevenLabs `eleven_v3` + `eleven_flash_v2_5`, 242 SFX via `eleven_text_to_sound_v2`, 39 music loops via `music_v1`. The remaining 81 music tracks have handoff docs ready for either RTX 5090 local (ACE-Step v1.5, Apache 2.0) or AIVA Pro Playwright generation.
- **1,248 / 1,253 lib tests passing.** 5 pre-existing failures isolated to `cf-ai` (1), `cf-perception` (1), `cf-render-2d` (1), `cf-server` (2). Tonight's work didn't introduce any new failures.
- **CI gates clean:** `cf-mod ledger verify --strict` reports `total=6718 fresh=6718 stale=0 drifted=0 missing=0 failed=0`; `cf-mod validate content/` reports `pass=1 warn=84 fail=0`.

### Up next

**`M12` — Vivid color-rich illustrated aesthetic + juice.** Hand-drawn ink-accent UI, 12 mission comic panels, death-recap-as-graphic-novel, juice rules per DR-055. Tier 1 visual placeholder backbone (6,066 entries) already covers every M12 surface; the runtime juice rules + animation system + comic-panel transitions remain.

In parallel: **`M12A`** finishing the SFX coverage gap, **`M37A`** baking the remaining 81 music tracks, **`M24A` / `M25A` / `M32A` / `M33A`** Tier 2 visual / narrative / portrait / tutorial production.

---

## Asset Ledger

Every shipped asset is logged in `content/asset_ledger/ledger.jsonl` with prompt + seed + tool + tier + blake3 hash. Re-bake the entire game from scratch on a clean checkout via `cf-mod ledger regenerate --all`.

| Category | Count | Tier |
|---|---:|---|
| ActorSprite (per-faction × per-actor SVG poses) | 3,080 | Tier 1 SVG |
| UiIcon (HUD widgets, shell UI, faction emblems) | 1,701 | Tier 1 SVG |
| Particle (VFX particle textures) | 284 | Tier 1 SVG |
| **Audio_Voice** (NPC + storyteller + boss + mission + tutorial + chatter) | 242 | **Tier 2 ElevenLabs `eleven_v3` / `eleven_flash_v2_5`** |
| **Audio_SFX** (weapon / movement / impact / ambient / UI) | 242 | **Tier 2 ElevenLabs `eleven_text_to_sound_v2`** |
| BaseModuleSprite (workbenches, vents, generators) | 240 | Tier 1 SVG |
| WeaponSprite (per-class fire / dry / reload) | 210 | Tier 1 SVG |
| MaterialSwatch (per-material × per-state textures) | 170 | Tier 1 SVG |
| Animation (per-actor walk / hit / death frame strips) | 144 | Tier 1 SVG |
| **Audio_Music** (12 worlds + 8 factions + 5 storytellers + 5 bosses × 4 variants) | **120** | **39 Tier 2 ElevenLabs `music_v1` + 81 Tier 1 procedural — handoffs pending** |
| Cosmetic (helmets, paint kits, decals) | 106 | Tier 1 SVG |
| TerrainTile (per-world × per-biome tiles) | 85 | Tier 1 SVG |
| VehicleSprite (dropships, mechs, rockets) | 54 | Tier 1 SVG |
| ChassisSprite (3 archetypes × layered armor) | 40 | Tier 1 SVG |
| **Total** | **6,718** | clean |

The 81 unfinished music tracks are documented per-file at `game/content/audio/music/MUSIC_LEDGER.md` with full prompts and 4 generation paths (top-up ElevenLabs / local 5090 / AIVA / delete-for-silence).

---

## Roadmap

> [!tip]
> One ordered table. Every row maps to a single spec file in [`specs/done/`](specs/done/) or [`specs/active/`](specs/active/) — no fictional sub-milestones, no marketing aliases.

**Legend:** ✅ closed (in `specs/done/`) · 🔄 in flight · ⏳ planned · 🚀 launch GA

| # | ⬤ | Milestone | What it ships |
|---|:---:|---|---|
| M1 | ✅ | Actor Controller + Sim Core | Playable actor, 5-state body machine, 9 JSON-RPC methods, tick-rate-independent timing |
| M2 | ✅ | Micro Breach Fun Slice | 60-90 s win/loss, ReactiveGuard FSM, 3 difficulty presets, cfctl-scriptable |
| M3 | ✅ | Pixel Terrain + Materials | 256×256 chunked deformable terrain, 8 launch materials, 9-flag affordance grid, per-pixel integrity |
| M3A | ✅ | Cross-OS Determinism | Linux x86_64 + Windows x86_64 + macOS aarch64 byte-identical event streams |
| M4 | ✅ | Event Recorder Core | 38 event categories, replay verifier, per-tick blake3 `sim_checksum`, cosmetic-flag backpressure |
| M4A | ✅ | Asset Ledger Infrastructure | JSONL append-only ledger, 17 asset categories, 6 production tiers, regen + verify CLI, supersede chain |
| M5 | ✅ | Deep Damage Event Surface Lock | 74 deep-damage event schemas across 13 families (armor / internal / concussion / fluid / origin / hazard / atmos / shield / environment / thermal) |
| M6 | ✅ | Actor Depth + Equipment + Squad | 36 actions, 6 weapons, 4 grenades, 8-slot inventory, side-view facing, 1 friendly bot + 4 squad commands |
| M7 | ✅ | AI Archetypes + Mission Director | 6 archetypes, 5-layer thinking stack, per-actor 22-task Priority Table, chatter scaffold, 20+ traits, 3 doctrines |
| M8 | ✅ | UX + Camera + Debug + L10n + Squad Control | Tab tactical overlay, Q-hold context wheel, "Why?" key, 10+ HUD widgets, 7 debug overlays, photo mode, replay scrubber |
| M8A | ✅ | Parallel Determinism + GPU Offload + Server | Bevy ECS scheduler, snapshot-read / compute-parallel / commit-serial, `cf-net` (QUIC + LAN lockstep + internet rollback), GPU cosmetic offload, semantic terrain events |
| M9 | ✅ | Micro Reactor Defense Fun Slice | 60-90 s defend scenario, 5-tier terrain HP, 3-layer reactor armor, trench gameplay, parallel-foundation stress test |
| M9A | ✅ | Tier 1 SVG Asset Pipeline | Python + cairo-svg + procedural SVG composers, 8 faction style configs, per-origin palettes |
| M10 | ✅ | Replay Viewer + Debrief | Bundle viewer, cause-chain walker, 18-section debrief markdown |
| M11 | ✅ | Readability + ACC-A Floor | 12 HUD nodes / 7 zones, 200% scale, high contrast, War Thunder angle widget, 25-scenario PASS verdict table |
| M11A | ✅ | Shell UI Foundation | `cf-shell` crate (12 modules, 56 tests), splash + title + main menu + pause + save/load + settings tree + credits + loading screen + FRE wizard + 48 shell-widget SVGs |
| M12 | 🔄 | Vivid Color-Rich Illustrated Aesthetic + Juice | Hand-drawn ink-accent UI, 12 mission comic panels, death-recap-as-graphic-novel, juice rules per DR-055 |
| M12A | 🔄 | Tier 1 Audio Pipeline | 1200+ SFX target via Stable Audio Open / AudioCraft + caption metadata · **Tier 2 ElevenLabs SFX shipped (242 / 242)** |
| M13 | ⏳ | Equipment + Chassis + Damage Grammar | 3 chassis archetypes, 15-zone body graph, layered armor, module state machine, pilot eject, brain-hop API |
| M14 | ⏳ | Full Collision + Impulse Routing | Universal gravity field, projectile-projectile CCD, War Thunder penetration ray, HE / HEAT / APFSDS, spalling |
| M15 | ⏳ | Active Material Kernel | Noita-grade per-pixel CA, 50+ materials, 30+ reactions, alchemy, flasks, GPU compute |
| M16 | ⏳ | Hazard Package + Afflictions | 18 afflictions, 6 STALKER anomalies, 20+ artifacts, swimming + underwater combat |
| M17 | ⏳ | Origin Reaction + Resource Model | No-HP-bar canonical, blood / oil / power / caloric / bio-fluid per origin, G-Force vision blackout |
| M18 | ⏳ | Micro Sabotage Fun Slice | 60-90 s sabotage integrating collision + materials + hazards + origin |
| M18A | ⏳ | Animation Production Tier 1 | 1100+ frame strips (walk / hit reactions / death) via AnimateDiff |
| M19 | ⏳ | Atmospherics-Grade Kernel | PV=nRT, 10 gases, 6 combustion reactions, phase change, pipe networks, 6 launch worlds |
| M20 | ⏳ | EnvironmentSignal Aggregator | Per-tick per-actor bundle: 11 slices (atmospheric / thermal / radiation / photic / EM / weather / water / acoustic / day-night / comms / gravitational) |
| M21 | ⏳ | Micro Pressure Hold Fun Slice | 60-90 s hold-the-room while atmosphere is breached |
| M22 | ⏳ | AI Pathfinding + Collision Avoidance | Hierarchical A* (tile / chunk / region), dynamic re-pathing on terrain dirty, per-team path costs |
| M23 | ⏳ | AI MIND Layer | Async LLM mind never blocks local AI, per-actor 5 s ticks, doctrine proposals, sandbox safety |
| M24 | ⏳ | AI Environmental Competence | 8-test AI-MAT suite, bots wear O2 in vacuum, retreat from radiation, downclock under heat |
| M24A | 🔄 | VFX Tier 1 | 600+ particle configs + 80+ textures (impact / spark / explosion / debris) · **Tier 1 partial: 120 Particle entries + 64 VfxFrame + 96 VfxDecal shipped** |
| M25 | ⏳ | Campaign + Base + Commander Spine | 30+ missions, 5 storytellers (Cassandra / Phoebe / Randy / Ironman / Sandbox), buy menu, 8 stratagems, persistent AI commander rivalry |
| M25A | 🔄 | Narrative + Codex Production | 5k-word bible → 80k words · 600 codex entries · 24 named NPCs · **Tier 1 narrative bible authored: 83 files (factions / worlds / NPCs / storytellers / bosses / missions / origins / 300 codex / 75 achievements / 60 loading tips / credits template)** |
| M26 | ⏳ | Factions + NPCs + Narrative | 8 factions, relationship matrix, quartermasters, diplomacy, dialog trees, 8 quest templates, 4 pet companions |
| M27 | ⏳ | Loot + Progression + RPG | 5 rarity tiers, 30+ affixes, set bonuses, XP / level, 30+ achievements, 20+ perks, 10+ curses, Tetris inventory |
| M27A | ⏳ | Player Game UI | Inventory Tetris, loadout, cosmetic locker, codex (600), achievements (75), tutorial menu, comparison tooltip |
| M28 | ⏳ | Base Atmospherics | Pumps, vents, pressure doors, breach repair, heaters, coolers, radiators, coolant loops, emergency venting |
| M28A | ⏳ | Base Build Mode UX | Palette, ghost preview, rotation, room detection, blueprints, demolish / repair, multiplayer co-build |
| M29 | ⏳ | Power & Electrical Engineering | 8 generators, 6 cable tiers, 6 storage types, IC10 priority chips, brown-out cascades, per-actor personal power |
| M29A | ⏳ | Power Grid + IC10 Editor UX | Factorio-style overlay, IC10 editor with breakpoints, per-generator dashboard, brown-out cascade viz |
| M30 | ⏳ | Basic Mining + Refining | 4 mining tools, 7 launch ores, 3-world ore distribution |
| M31 | ⏳ | Weather + Day/Night Kernel | 7 weather states, 24-hour cycle, AI behavior shifts, 12 total worlds |
| M32 | ⏳ | Crafting Tiers + Fabrication Chain | 5-tier ladder (T0..T4), 150 launch recipes, fabrication chain, power-coupled, 30-node research tree |
| M32A | 🔄 | Tier 2 ComfyUI Pipeline | 4500+ production assets via SDXL + Flux + AnimateDiff + ControlNet + per-faction LoRAs · **Tier 1 NPC portrait placeholders shipped (44 entries)** |
| M32B | ⏳ | Crafting + Research + Salvage UX | 3-pane crafting (Stationeers / Terraria / Factorio hybrid), research tree pan / zoom, material flow Sankey |
| M33 | ⏳ | Modding Workbench | In-game scenario editor, Lua, IC10 chip editor, Steam Workshop, photo mode, replay browser, 8 tutorial labs |
| M33A | 🔄 | Tutorial Lab Production | 8 modular labs + First Contract + FRE wizard · **Tier 1 voice prompts authored (70+ tutorial narration lines)** |
| M34 | ⏳ | Material Lab | DR-036 workbench, brushes, 17 materials, 13 lab-unlocked, recipe inspector, stamp save / load |
| M35 | ⏳ | Advanced Mining + Extraction | 9 mining tools, 12 ores, AI-MINE-A 8-test suite, server-authoritative ledger |
| M36 | ⏳ | Dedicated Server + Determinism Islands | 5 server modes, `cf-server-ops` / `persistence` / `anti-cheat` / `admin`, 4 mod trust tiers |
| M36A | ⏳ | Platform Integration | Steam SDK, Discord rich presence, EOS adapter, Workshop, Cloud, Achievements bridge |
| M36B | ⏳ | Telemetry + Crash + Bug Report | Opt-in privacy, per-shard analytics, in-game bug-submit |
| M37 | ⏳ | Voice + Radio + Comms | ACRE2-tier radio (distance / occlusion / frequency / antenna), EMP, vacuum no-voice, AI chatter, captions |
| M37A | 🔄 | Tier 2 Audio + Voice + Music | 7000+ voice clips target + 30+ music tracks + adaptive music engine · **Tier 2 ElevenLabs shipped: 242 voice + 39 music; 81 music remain (RTX 5090 / AIVA handoffs ready)** |
| M38 | ⏳ | Server Config + Admin CLI + Settings | 200+ tunables, 7-tier hierarchy, 20+ admin commands, auto-gen settings UI, audit log |
| M38A | ⏳ | Localization (19 languages) | 11 Tier-A + 8 Tier-B, 380k+ translations via LLM auto-translation, ICU MessageFormat |
| M39 | ⏳ | Universal Schema Locks | Manifest at `cf-mod/manifest/all_schemas.ron`, ~120 locked schemas, one-shot conformance check |
| M40 | ⏳ | LAN Co-op | 2-4 player co-op via `lan_room`, mDNS / UDP discovery, 7 squad roles, revive, death cam, mod hash sync |
| M40A | ⏳ | Spectator + Streamer Polish | Replay-to-MP4 via FFmpeg, 10+ overlay themes, Twitch / YouTube / Discord webhook integration |
| M40B | ⏳ | Online UX | Server browser, friends, party invite, lobby, admin web panel, voice chat UI |
| M41 | ⏳ | Online Co-op + Full Match Grammar | NAT punch-through, 10 endgame modes, persistent AI commander, 4+ squads × 7 role types × 12 commands, War Thunder kill cam |
| M42 | ⏳ | Self-Hosted Server Deployment | 3 Docker tiers, systemd, launchd, Terraform, Ansible, Grafana / Prometheus / Loki, 15-min deploy target |
| M43 | ⏳ | PvE Survival Mode + Procgen | 1-8 player coop, 7-step procgen, 3 launch survival worlds, per-race difficulty matrix, acclimatization |
| M43A | ⏳ | Map + Mission + Campaign UX | World map, solar system map (12 worlds), mission select, campaign tree, briefing, travel planner |
| M44 | ⏳ | Inter-Planet Transport + Stations | 3 transport modes (dropship 8-phase / multi-stage rocket / paired teleporters), orbital stations, 7 new vehicles |
| M45 | ⏳ | PvE Endgame Bosses + World Events | 5 named bosses (Hollow King / Frozen Heart / Crimson Tide / Eclipse Walker / Last Star), 12 dynamic world events |
| M45A | ⏳ | Cosmetic Production | 2200+ items, anti-pay-to-win audit (DR-031) |
| M46 | ⏳ | Upkeep Economy (Opt-In) | BRP drain, bankruptcy cascade Day 1 → 30, rescue mechanisms, AI factions follow same rules |
| M47 | ⏳ | Strategy Phase + Goals (Opt-In) | Per-cycle decision phase, 5 stances × 5 production × 3 logistics = 75 combos, 24 launch goals |
| M48 | ⏳ | Inter-Faction Intelligence (Opt-In) | Codebreaking + spy rings + 8 covert ops + counter-intel; AI factions follow same rules |
| M48A | 🔄 | Tier 3 Polish | Top 50 Aseprite hand-polish, top 20 Spine rigs, FMOD / Kira mix, Steam Deck Verified · **Tier 1 cinematic placeholders: 23 boss splashes + 20 loading bg + 19 key art + 44 portraits** |
| M48B | 🔄 | Steam Store + Marketing | Capsule art, 12 screenshots, 6 trailer types, press kit, tag taxonomy · **Tier 1 marketing placeholders: 19 KeyArt entries** |
| M48C | ⏳ | Endgame + Workshop UX Polish | Debrief, replay browser, photo mode, mech bay, pilot / commander dossier, faction diplomacy, quest log, NPC dialog, hub, mod manager |
| M49 | 🚀 | Public PvP + Persistent MMO + Bunker Defence | **Launch GA** · public PvP arenas · MMO shards · Bunker Defence flagship mode · 4 launch shards · cross-shard events · `v1.0.0` |

> Spec files in [`specs/active/`](specs/active/) · closed specs in [`specs/done/`](specs/done/) · per-spec workflow contract in [`AGENTS.md`](AGENTS.md).

---

## Performance + Determinism Contract

### Reference platforms

| Role | Hardware | Why this tier exists |
|---|---|---|
| **Client (single-player + LAN host + competitive)** | Ryzen 9 9950X3D · RTX 5090 · 48 GB DDR5 · Windows 11 / Linux 6.x | The no-compromise ceiling. 200 actors + 500 projectiles + 1000 hazard pixels at sustained 60 Hz. 120 Hz mode hits 120 FPS sustained. |
| **Server — workstation x86_64** | Ryzen 9 9950X3D / Threadripper 7000 · RTX 5090 / 4090 · 64+ GB DDR5 · 1+ Gbps NIC | Dedicated public servers, large LAN tournaments, server-side replay rendering. |
| **Server — Apple Silicon** | Mac Studio / Mac mini M4 Pro (10P+4E, 273 GB/s memory bandwidth) or M5 Pro / M5 Max | Indie-host shards, persistent-world nodes. High single-thread perf; unified memory; low TDP. |
| **Server — Linux VPS** | 16+ vCPU x86_64 (Hetzner AX102 / OVH Game / AWS c7i.4xlarge) · no GPU required | Cloud-hosted public servers, MMO shard backbone. `cf-headless serve` compiles with `cf-render-2d/stub` for zero-GPU images. |
| **Server — Apple Mini Lab** | 3-5 × Mac mini M4 Pro 64/24 GB, 10 GbE switch | Persistent-MMO dev cluster. Shard-aware mesh validation. |

All four server tiers pass the same cross-OS determinism CI gate. **No second-class server hardware.**

### Per-tick perf budget (60 Hz wall clock = 16.6 ms)

| Subsystem | p99 budget | Notes |
|---|---:|---|
| Actor sim | ≤ 1.5 ms | `par_iter` over actor entities; pre-rolled RNG |
| AI sim | ≤ 2.0 ms | `par_iter` over guard entities; pre-rolled RNG |
| Projectile sim | ≤ 1.0 ms | `par_iter` with snapshotted terrain reads |
| Terrain mutation + dirty batch | ≤ 2.5 ms | per-chunk `par_iter`; deterministic (cx,cy)-sorted boundary post-pass |
| Mission director | ≤ 0.2 ms | ECS resource tick |
| Recorder + checksum + merge | ≤ 0.5 ms | per-thread `RecorderShard` + canonical merge |
| Render dispatch | ≤ 4.0 ms | GPU compute particles + fragment-shader overlay + `Texture2DArray` |
| Headroom | ≥ 4.0 ms | rollback resimulation buffer (6-frame at p99 ≤ 2.5 ms / frame) |
| **Total** | **≤ 15.5 ms** | under the 16.6 ms 60 Hz wall-clock |

### Determinism — the 7 hard rules

CI gates enforce each:

1. **No `f64` in sim crates.** `f32` only — stable cross-platform bit-exact under IEEE 754.
2. **No `thread_rng()` in sim crates.** Every RNG call uses the engine's seeded RNG, pre-rolled into a `Vec<u64>` before any `par_iter` block.
3. **No `HashMap` with default `RandomState` in sim state.** Use `BTreeMap` (deterministic iteration) or `FxHashMap` (fixed seed; only when iteration order doesn't cross a checksum boundary).
4. **No FMA / wide AVX-512 intrinsics** that vary by hardware. Stable `f32` code path only.
5. **No `Instant::now()` / `SystemTime::now()`** in sim code. Use the engine's `Clock::tick().0`.
6. **No `Vec::new()` / `HashMap::new()` per tick** in hot paths. Pre-allocate, clear-and-reuse.
7. **Cross-thread state mutation MUST be per-worker buffer + deterministic merge**, or explicit single-threaded post-pass. Atomic-CAS loops on shared sim state are forbidden.

The combination is what allows a 9950X3D Windows client, an M4 Pro macOS server, and a Hetzner Linux x86_64 VPS to produce **byte-identical event streams** on the same inputs.

### Networking

| Mode | What it does |
|---|---|
| `cf-headless serve` — dedicated authoritative server | Same deterministic sim core as the client; no renderer; same run-bundle envelope. Server bundle is the lobby's source of truth for replay + dispute resolution + cross-client divergence detection. |
| LAN lockstep | Clients send inputs only; server broadcasts merged input set per tick; all clients sim-step locally with identical inputs. 8 clients on the same LAN see the same world. |
| Internet rollback | Client predicts forward, server confirms, client rolls back on misprediction + resimulates forward (6-frame budget at p99 ≤ 8 ms total). Resimulation re-uses the deterministic sim core — no parallel rollback codepath. |
| Semantic terrain events | Every terrain mutation emits `terrain.chunk_mutated` with `cause` + `chunk_coords` + `bbox` + `delta_materials` + `post_state_checksum`. ~5× smaller than bitmap deltas. Fallback per-chunk snapshot when a client falls > 256 ticks behind. |
| Snapshot envelope | Powder-Toy-derived field shape: `tick + rng_state + chunks + actors + projectiles + mission`. Cadence: every 64 ticks (configurable via `srv.set_cvar net.snapshot.cadence_ticks <n>`). |

---

## Workspace

**44 crates** under [`game/crates/`](game/crates/). Each crate has its own `AGENTS.md` boundary contract.

```text
sim core              cf-sim-core · cf-actor · cf-physics · cf-terrain · cf-material · cf-chassis · cf-equipment
                      cf-mission · cf-perception · cf-squad · cf-ai · cf-environment · cf-atmos · cf-priority
control + replay      cf-control · cfctl · cf-replay · cf-replay-scrub · cf-headless · cf-mod · cf-asset-ledger
                      cf-save · cf-e2e · cf-bench
runtime + presentation cf-app · cf-render-2d · cf-camera · cf-capture · cf-ui · cf-shell · cf-debug · cf-audio
                      cf-photo · cf-killcam · cf-localization · cf-squad-ui
networking + server   cf-net · cf-server · cf-server-ops · cf-server-persistence · cf-server-anti-cheat
                      cf-server-admin
tooling               cf-tools-editor · cf-tools-replay-viewer
```

Notable additions tonight:
- **`cf-shell`** — 12-module shell UI crate (splash / title / attract_mode / main_menu / pause_menu / save_load / save_slot_preview / settings_tree / credits / loading_screen / fre_wizard / shell_api / state). 56 lib tests. `ShellPlugin` wired into `cf-app`.
- **`cf-audio::registry::AudioRegistry`** — pure-data ledger hydration indexed by canonical_name × {voice / sfx / music}. `music_variant_for(track_id, intensity)` adaptive-music selector per the `[0.0,0.3)→calm / [0.3,0.6)→buildup / [0.6,1.0]→climax` schedule. 18 / 18 cf-audio tests pass.

---

## Tech Stack

| Layer | Tooling |
|---|---|
| Language | Rust edition 2021, MSRV / toolchain pinned to 1.95.0 |
| Engine | [Bevy](https://bevyengine.org) 0.18.1 + [wgpu](https://wgpu.rs) for 2D / GPU; custom core crates for sim |
| Physics | Custom collision + custom material kernel + Stationeers-grade-or-better atmospherics / thermal kernel + universal gravity field |
| Async | [Tokio](https://tokio.rs) for the JSON-RPC control plane and dedicated server |
| Networking | [quinn](https://github.com/quinn-rs/quinn) (QUIC over UDP). Reliable streams for the canonical event log; unreliable datagrams for low-latency inputs + snapshot deltas. Wire protocol `prototype-net-frame.v0.1` (additive-only). |
| Modding host (planned) | [mlua](https://github.com/khvzak/mlua) (Lua) candidate; IC10 chip editor in `M33` |
| Determinism | [BLAKE3](https://github.com/BLAKE3-team/BLAKE3) for state checksums; [`rand_xoshiro`](https://docs.rs/rand_xoshiro) for seeded RNG |
| Schemas | [serde](https://serde.rs) + [schemars](https://github.com/GREsau/schemars) + JSON Schema validation in CI |
| Audio bake | ElevenLabs (`eleven_v3` / `eleven_ttv_v3` / `eleven_text_to_sound_v2` / `music_v1`) + custom Python pipeline at `tools/audio_pipeline/` |
| Testing | `cargo test` matrix (Linux + macOS + Windows) + scripted E2E + run-bundle checker (Python `tools/prototype_run_check.py`) |

---

## Getting Started

### Prerequisites

| Tool | Version |
|---|---|
| Rust toolchain | 1.95.0 (pinned via `game/rust-toolchain.toml`) |
| Python | 3.11+ (for `tools/prototype_run_check.py` and the audio pipeline) |
| OS | Linux + macOS + Windows |

### Build + run

```bash
git clone https://github.com/Madreag/corefall.git
cd corefall/game

# Workspace sanity
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --lib --no-fail-fast

# Smoke runs
cargo run -p cfctl -- observe --once --scenario m0_blank
cargo run -p cfctl -- run --scenario m0_blank --ticks 300 --tick-rate-hz 60 --paced --write-run-bundle
cargo run -p cf-app -- --scenario m0_blank --headless-smoke --run-seconds 5 --write-run-bundle

# Play the M1 actor range (windowed): WASD = move, Space = jump, arrows = aim,
# Enter / J = fire, R = reload, L = reset, 1-4 = inventory slot, Esc = quit.
cargo run -p cf-app -- --scenario m1_actor_range

# Play the M2 micro breach
cargo run -p cf-app -- --scenario micro_breach

# Play the M9 micro reactor defense
cargo run -p cf-app -- --scenario micro_reactor_defense

# Validate any run bundle
python3 tools/prototype_run_check.py ../prototype_runs/native/m1_*
```

### Verify ledger + run all tests

```bash
cd game
cargo run -p cf-mod -- ledger verify --strict       # expects: total=6718 fresh=6718 stale=0 ...
cargo run -p cf-mod -- validate ../content/         # expects: pass=1 warn=84 fail=0
cargo test -p cf-audio -p cf-control -p cf-shell --lib   # 18 + 165 + 56 = 239 / 239 pass
```

---

## CLI Reference

`cfctl` is the operator + AI control client. The shipped subset:

| Command | Milestone | What |
|---|---|---|
| `cfctl observe --once --scenario <id>` | M1 | One-shot snapshot of game state |
| `cfctl observe --stream --hz <N>` | M1 | Stream observation frames at N Hz |
| `cfctl run --scenario <id> --ticks <N> --paced --write-run-bundle` | M1 | Run a scenario for N ticks paced to wall clock |
| `cfctl scenario load <id> [--seed <N>]` | M1 | Load a scenario |
| `cfctl pause` / `step --ticks <N>` / `version` | M1 | Sim control + protocol version |
| `cfctl act player-move --x <-1..1>` | M1 | Continuous horizontal movement intent (latest-value-wins) |
| `cfctl act player-jump` | M1 | Edge-triggered jump on the next tick |
| `cfctl act player-aim --x <f32> --y <f32>` | M1 | Set aim vector (NaN/Inf rejected) |
| `cfctl act player-fire [--pressed true\|false]` | M1 | Edge-triggered rifle fire |
| `cfctl act player-reload` | M1 | Begin reload (1.5 s real time at any tick rate) |
| `cfctl act player-select-item --slot <0..3>` | M1 | Switch inventory slot |
| `cfctl act player-reset` | M1 | Respawn at scenario position with full HP / ammo / slot 0 |
| `cfctl act player-dig [--target <breach_id>]` | M2 | Edge-triggered terrain dig |
| `cfctl act player-toggle-material-overlay [--mode <off\|integrity\|pathability\|mobility\|hazard\|build_repair>]` | M3 | Cycle 5-mode material overlay |
| `cfctl act player-crouch` / `player-climb` / `player-jet` / `player-eject` | M5 | Stance + chassis intent |
| `cfctl act chassis-repair --zone <zone>` | M5 | Repair a damaged chassis zone |
| `cfctl act chassis-salvage` / `chassis-clear-jam` | M5 | Salvage modules from a wreck / clear weapon jam |
| `cfctl inspect actor <id>` | M5 | Inspect full actor state including chassis + origin |
| `cfctl inspect chassis <id>` / `inspect material <id>` | M5 | Chassis or material introspection |
| `cfctl observe terrain` | M3 | Snapshot of terrain state |
| `cfctl script run <path>` | M1 | Replay a `.cfctl.json` script |

`cf-app --capture-grid --capture-frames-hz 10` + `python3 game/tools/capture_grid.py <run_dir>` produce the T-CAPTURE PNG-readback grids per BP closure exemplar.

---

## CI

GitHub Actions runs on every push and PR:

- `cargo fmt --all -- --check` (LF line-endings locked via `.gitattributes`)
- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace --lib --no-fail-fast`
- `cargo build --release`
- `cf-mod validate content/`
- `cf-mod ledger verify --strict`
- Schema drift check (`dump_schemas --check`)
- `cfctl observe smoke` + `cfctl run smoke` (60 Hz + 120 Hz)
- `cf-app --headless-smoke --run-seconds 5`
- M3A cross-OS determinism matrix (Linux + macOS + Windows, 60/120 Hz)
- Run-bundle validator (`tools/prototype_run_check.py`) on every produced bundle

Matrix: Linux + macOS + Windows.

---

## Releases

Released artifacts live at [github.com/Madreag/corefall/releases](https://github.com/Madreag/corefall/releases). Every Build Point closure publishes a tagged cross-platform release per the **T-RELEASE** side track.

### Double-Click Playability Hard Gate

A non-technical friend receiving the release file MUST be able to:

1. **Double-click the file** → standard OS extract/install (no Terminal, no `brew install`, no command-line decompression).
2. **Double-click the resulting app** → a Corefall game window opens (no `--scenario` flag, no PowerShell, no command-line args).

If either fails, the platform is omitted from the BP's release matrix; if no platform meets the gate, the BP **skips** its release tag entirely.

| Platform | Format | Friend's experience |
|---|---|---|
| **macOS** | `.dmg` containing `Corefall.app` | Mount the `.dmg`. Drag `Corefall.app` to Applications. Double-click → game window. |
| **Windows** | `.msi` installer or `.zip` with `Corefall.exe` (default-args launcher) | Run the `.msi` (Start Menu shortcut), or unzip + double-click `Corefall.exe` → game window. |
| **Linux** | AppImage (single double-click executable) | `chmod +x Corefall.AppImage` + double-click → game window. |

### Channel-based SemVer

| Channel | Tag form |
|---|---|
| prealpha | `v0.<N>.0-prealpha` |
| alpha | `v0.<N>.0-alpha` |
| beta | `v0.<N>.0-beta` |
| rc | `v0.<N>.0-rc` |
| GA | `v1.0.0` (BP12 launch) |

---

## Inspirations

Corefall is implemented from chemistry / physics / game-design first principles plus public documentation. **No code, no assets, no sprites, no audio** from any inspiration game is copied into Corefall. Each project below taught us something we built on:

[Cortex Command](https://datarealms.com) (command-core / chassis / brain-hop fantasy) · [Noita](https://noitagame.com) (per-pixel material simulation) · [Stationeers](https://stationeers.com) (PV=nRT atmospherics + IC10 chips) · [Oxygen Not Included](https://klei.com/games/oxygen-not-included) (closed-loop life support + per-tile thermal) · [Barotrauma](https://barotraumagame.com) (rooms-with-state) · [War Thunder](https://warthunder.com) (angled armor + spalling + module-ray penetration + kill cam) · [ACRE2](https://acre2.idi-systems.com) (radio propagation realism) · [The Powder Toy](https://powdertoy.co.uk) (element-grammar reaction tables) · [Soldat](https://forums.soldat.pl) / [OpenSoldat](https://opensoldat.org) (side-view multiplayer combat feel) · [Liero](https://liero.be) / [OpenLieroX](https://openlierox.sourceforge.io) (short intense arena combat) · [Teardown](https://teardowngame.com) (destruction as design) · [STALKER](https://gsc-game.com) (anomalies + artifacts + faction zones) · [Helldivers](https://helldivers.com) (stratagem call-ins) · [Diablo](https://diablo.blizzard.com) (loot rarity + affixes) · [Rimworld](https://rimworldgame.com) (storyteller pacing + traits) · [Subnautica Below Zero](https://unknownworlds.com/subnautica) (cold acclimatization).

---

## Contributing

Right now Corefall is in early implementation. The vault is the design source; this repo executes against it. If you want to contribute:

1. **File an issue first** — propose what you want to work on and we'll align it with the next milestone.
2. **Read [`AGENTS.md`](AGENTS.md)** — the agent contract (covers AI workers and human contributors equally). Especially the Milestone Authority Stack, Milestone Acceptance Gate, Contract Integrity Gate, and No-Compromise Performance Defaults sections.
3. **Branch from `origin/main`** with a milestone-prefixed name (e.g. `m12/comic-panel-system`). PRs are required for any non-trivial change.
4. **Run Standard Validation locally** (`cargo fmt`, `cargo check`, `cargo clippy -- -D warnings`, `cargo test --workspace --lib --no-fail-fast`) before pushing.

Per-crate `AGENTS.md` files in each `game/crates/cf-*/` directory describe owned APIs, public boundaries, common pitfalls, and source trails.

---

## License

Dual-licensed under your choice of:

- [Apache License 2.0](https://www.apache.org/licenses/LICENSE-2.0)
- [MIT License](https://opensource.org/licenses/MIT)

Standard Rust ecosystem dual-license — pick whichever is most compatible with your project. Workspace-level declaration in [`game/Cargo.toml`](game/Cargo.toml).

> No code, assets, sprites, or audio from any of the inspiration games is copied into Corefall. Provenance for any externally-derived material is tracked in the canonical vault's `references/usage-ledger.md`.

---

<div align="center">

**[Pillars](#headline-systems) · [Roadmap](#roadmap) · [Workspace](#workspace) · [Performance](#performance--determinism-contract) · [Getting started](#getting-started) · [CLI](#cli-reference) · [License](#license)**

*One field for gravity. One kernel for atmospheres. One source of truth for everything.*
*No HP bars — only blood, oil, and power.*

</div>
