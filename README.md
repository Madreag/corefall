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
[![Milestones](https://img.shields.io/badge/milestones-49%20done%20%2F%20126%20active-2EA043?style=flat-square)](#roadmap)
[![Crates](https://img.shields.io/badge/crates-67-blueviolet?style=flat-square)](#workspace)
[![Lib tests](https://img.shields.io/badge/lib%20tests-3593%20passing-2EA043?style=flat-square)](#ci)
[![Ledger](https://img.shields.io/badge/ledger-7710%20fresh-2EA043?style=flat-square)](#asset-ledger)

**[Pillars](#headline-systems) · [Roadmap](#roadmap) · [Workspace](#workspace) · [Performance](#performance--determinism-contract) · [Getting started](#getting-started) · [CLI](#cli-reference)**

</div>

---

## What is Corefall

A 2D side-view physics sandbox where every gas, grain, bullet, body, world, transmission, and joint is real. You command actors, swap into chassis, breach bunkers, defend command cores, dig terrain, vent rooms, light atmospheres, lose limbs, and reason about damage through real causal graphs — not abstract HP bars.

It's a **best-of-genre synthesis** that takes Cortex Command's command-core / dropship / chassis / digging fantasy and sets it on top of Stationeers-grade-or-better atmospherics, ONI-grade closed-loop life support, Noita-grade systemic materials, full collision physics, universal gravity, ACRE2-tier voice + radio simulation, and War Thunder-grade armor + spalling. AI bots are first-class teammates and rivals. Replay is deterministic. Modding is data-first. Accessibility is a floor, not an afterthought.

> [!important]
> **Where we are (2026-06-14).** 49 milestones closed (`M1`–`M17`). The full **physics + damage train** is in (`M13` equipment/chassis/damage grammar → `M14` collision + impulse routing → `M14A`–`M14J`: limb-driven walking, gravity/wind producers, HEAT/APFSDS rounds, projectile-projectile CCD, per-pixel + lateral structural collapse, per-wound granularity, field-medic surgery, long-term scars/aging/veterans, advanced mobility). The **materials kernel** is in (`M15` Noita-grade per-pixel CA → `M15B` GPU kernel + precipitation, `M15C` 50+ material registry, `M15D` 55+ reaction matrix). The **hazard + affliction + biology layer** is in (`M16` hazards + 22 afflictions + anomalies + artifacts + swimming → `M16A` 11 environment-driven afflictions, `M16B` disease registry + lifecycle FSM + cure/vaccine/quarantine, `M16C` mental-health — Pain affliction + 8 conditions + PTSD + addiction + trauma + therapy). **Newest: `M17` Origin Reaction + Resource Model** — the canonical no-HP-bar survival layer: 11-race origin matrix, per-origin survival resources (blood / oil / power / caloric / bio-fluid / oxygen), per-origin shot-force-feedback + G-Force concussion bands (human full curve / android capped / robot internal-shock), helmet-breach + vacuum + oxygen-poisoning, robot overclock / downclock + INERT recovery, the per-damage-type × per-origin Time-To-Death table, and power/heat/vacuum-aware AI doctrine. 4,795 workspace tests pass; cross-OS determinism gate green. Next up: `M18`.

---

## At a Glance

| | |
|---|---|
| **Engine** | Bevy 0.18.1 + wgpu + custom sim core, Tokio for the JSON-RPC control plane |
| **Language** | Rust 1.95.0 (pinned via `game/rust-toolchain.toml`) |
| **Workspace** | 67 crates · **3,593 lib tests passing** |
| **Determinism** | Same seed + same inputs → byte-identical event stream on Linux + macOS + Windows |
| **Networking** | QUIC via `quinn`; LAN lockstep + internet rollback; semantic terrain events |
| **Asset ledger** | **7,710 fresh entries** — every shipped asset is regenerable from prompt + seed |
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

- **49 milestones closed** (`specs/done/`): the M1–M11A foundation + the M12/M12A–C aesthetic + audio + cinematic train + the full **M13–M14J physics / damage / wound / medic / long-term / mobility** train + the **M15–M15D materials kernel** (per-pixel CA + GPU + 50+ materials + 55+ reactions) + the **M16–M16C hazard / affliction / disease / mental-health layer** + the **M17 origin reaction + per-origin resource model**.
- **7,710 ledger entries** across all asset categories — every shipped asset is regenerable from prompt + seed via `cf-mod ledger regenerate <id>`.
- **4,795 workspace tests passing.** Cross-OS determinism gate (Linux + macOS + Windows, 60/120 Hz) green; sim crates remain `f64`-free, `thread_rng`-free, and default-hash-free per the 7 determinism hard rules.
- **CI gates:** `cf-mod ledger verify --strict` reports `total=7710 fresh=7710`; `cf-mod validate content/` passes; schema-drift + run-bundle validators clean.

### Up next

**`M17` — Origin Reaction + Resource Model is now CLOSED.** The canonical no-HP-bar layer shipped: 11-race origin matrix + per-origin survival resources (blood / oil / power / caloric / bio-fluid / oxygen), per-origin shot-force-feedback, G-Force concussion bands (human full curve / android capped at Moderate / robot internal-shock instead), helmet-breach + vacuum + oxygen-poisoning + dehydration, robot overclock / downclock + thermal throttle + INERT-and-recover, the per-damage-type × per-origin Time-To-Death table with difficulty multipliers + compound floor, `resource.*` event family, per-origin HUD bars + concussion vignette + heart-rate audio, per-origin death recap, and power/heat/vacuum-aware AI doctrine. Next milestone: **`M18`**.

In parallel: the production-tier tracks remain in flight — **`M24A`** VFX, **`M25A`** narrative/codex, **`M32A`** ComfyUI portraits, **`M33A`** tutorial labs, **`M37A`** the remaining 81 music tracks, **`M48A`/`M48B`** Tier 3 polish + Steam store.

---

## Asset Ledger

Every shipped asset is logged in `content/asset_ledger/ledger.jsonl` with prompt + seed + tool + tier + blake3 hash. Re-bake the entire game from scratch on a clean checkout via `cf-mod ledger regenerate --all`.

| Category | Count | Tier |
|---|---:|---|
| ActorSprite (per-faction × per-actor SVG poses) | 3,080 | Tier 1 SVG |
| UiIcon (HUD widgets, shell UI, faction emblems) | 1,709 | Tier 1 SVG |
| **Audio_SFX** (weapon / movement / impact / ambient / UI) | **1,222** | **Tier 2 ElevenLabs `eleven_text_to_sound_v2`** |
| Particle (VFX particle textures) | 284 | Tier 1 SVG |
| **Audio_Voice** (NPC + storyteller + boss + mission + tutorial + chatter) | 243 | **Tier 2 ElevenLabs `eleven_v3` / `eleven_flash_v2_5`** |
| BaseModuleSprite (workbenches, vents, generators) | 240 | Tier 1 SVG |
| WeaponSprite (per-class fire / dry / reload) | 210 | Tier 1 SVG |
| MaterialSwatch (per-material × per-state textures) | 170 | Tier 1 SVG |
| Animation (per-actor walk / hit / death frame strips) | 144 | Tier 1 SVG |
| **Audio_Music** (12 worlds + 8 factions + 5 storytellers + 5 bosses × 4 variants) | **121** | **Tier 2 ElevenLabs `music_v1` + Tier 1 procedural** |
| Cosmetic (helmets, paint kits, decals) | 106 | Tier 1 SVG |
| TerrainTile (per-world × per-biome tiles) | 85 | Tier 1 SVG |
| VehicleSprite (dropships, mechs, rockets) | 54 | Tier 1 SVG |
| ChassisSprite (3 archetypes × layered armor) | 40 | Tier 1 SVG |
| Mod_Custom (modder-authored ledger entries) | 2 | mod |
| **Total** | **7,710** | clean |

Remaining unfinished music tracks are documented per-file at `game/content/audio/music/MUSIC_LEDGER.md` with full prompts and generation paths (top-up ElevenLabs / local 5090 / AIVA).

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
| M3B | ⏳ | ONI-Grade Per-Tile Element Model + Heat Transfer | Per-tile `element_id + mass_kg + temp_K` substrate; ONI heat-transfer math with caps + thermal conductivity scaling; mass-based pressure damage; vacuum-as-perfect-insulator; one-element-per-cell rule |
| M3A | ✅ | Cross-OS Determinism | Linux x86_64 + Windows x86_64 + macOS aarch64 byte-identical event streams |
| M4 | ✅ | Event Recorder Core | 38 event categories, replay verifier, per-tick blake3 `sim_checksum`, cosmetic-flag backpressure |
| M4B | ✅ | Save Format Versioning + Schema Migration + Delta-Encoded Snapshots + Run-Bundle Ledger Deep | `SaveSchemaVersion` semver + forward-only migration chain + delta-encoded snapshots ≥4× compression + per-event BLAKE3 chain-of-custody for tamper-evident tournament bundles |
| M4A | ✅ | Asset Ledger Infrastructure | JSONL append-only ledger, 17 asset categories, 6 production tiers, regen + verify CLI, supersede chain |
| M5 | ✅ | Deep Damage Event Surface Lock | 74 deep-damage event schemas across 13 families (armor / internal / concussion / fluid / origin / hazard / atmos / shield / environment / thermal) |
| M6 | ✅ | Actor Depth + Equipment + Squad | 36 actions, 6 weapons, 4 grenades, 8-slot inventory, side-view facing, 1 friendly bot + 4 squad commands |
| M6B | ✅ | Item Schema Canonicalization + Per-Item Weight + Grid Dimensions + Inventory Encumbrance | Canonical ItemSpec schema (mass + grid + bulk + slot + container) + per-actor max_carry_kg + encumbrance compute + backpack tiers + container nesting |
| M6C | ✅ | Equipment Catalog Buildout (68 Launch SKUs) | 12 firearms + 8 melee + 10 throwables + 8 heavy weapons + 12 medical + 8 survival + 5 sensors + 15 PPE = 78 new SKUs at no-compromise depth |
| M7 | ✅ | AI Archetypes + Mission Director | 6 archetypes, 5-layer thinking stack, per-actor 22-task Priority Table, chatter scaffold, 20+ traits, 3 doctrines |
| M7B | ✅ | Deep Squad Command Grammar + Formation Orders + Combat Doctrine | 50+ squad commands (Advance/Suppress/Stack/Wedge/Echelon/Bound/Overwatch/Frag-Out/Smoke/Reinforce/Medic-Up/Fall-Back/etc) + 9 formation kinds with per-actor slot solver + 6× archetype behavior tree files (30+ nodes each) + Cortex-Command-style brain-hop preserving squad-state |
| M8 | ✅ | UX + Camera + Debug + L10n + Squad Control | Tab tactical overlay, Q-hold context wheel, "Why?" key, 10+ HUD widgets, 7 debug overlays, photo mode, replay scrubber |
| M8A | ✅ | Parallel Determinism + GPU Offload + Server | Bevy ECS scheduler, snapshot-read / compute-parallel / commit-serial, `cf-net` (QUIC + LAN lockstep + internet rollback), GPU cosmetic offload, semantic terrain events |
| M8B | ✅ | QUIC Wire Protocol + Rollback Prediction + NAT Punch-Through Deep | Locked QUIC frame format + semver negotiation + 6-frame rollback prediction + redundant-input + Reed-Solomon FEC + ICE-lite/STUN/TURN NAT punch + transport-select (server-auth vs P2P lockstep) |
| M9 | ✅ | Micro Reactor Defense Fun Slice | 60-90 s defend scenario, 5-tier terrain HP, 3-layer reactor armor, trench gameplay, parallel-foundation stress test |
| M9B | ✅ | Trench Networks + Defensive Position Authored Content | 6 trench cross-section variants (shallow_scrape / standard / deep / communication / fire_step / parapet_raised) + zigzag procgen + per-segment cover state + 6 embedded modules + 4 trench templates + 8 launch scenarios + AI-TRENCH-A-01 burst-and-duck doctrine + 8 replay event schemas |
| M9C | ✅ | Static Fortifications + Defensive Structures (MG Nests / Watchtowers / Mines / Wire) | MG nest + ammo box + spotter scope + 3-tier sandbag wall + 3-tier watchtower + spotlight + 4 mine kinds + minesweeper + bomb disposal robot + barbed/razor/electrified/concertina wire + anti-tank ditch + dragon's teeth + tank trap + camo netting + 4 AI doctrines (AI-MG-A-02/OBS-A-01/ENG-A-03/AT-A-04) + 10 launch scenarios + 16 replay event schemas |
| M9A | ✅ | Tier 1 SVG Asset Pipeline | Python + cairo-svg + procedural SVG composers, 8 faction style configs, per-origin palettes |
| M10 | ✅ | Replay Viewer + Debrief | Bundle viewer, cause-chain walker, 18-section debrief markdown |
| M10B | ✅ | Replay-as-MP4 Export + Replay Editor + Overlay Composition + Chapter Markers + Per-Scene Camera Control | `cf-replay-export` ffmpeg-next 8.1.0 pipeline + 5 codec presets (twitch_1080p60/youtube_4k60/discord_720p30/clip_compact/archival_lossless FFV1) + auto-derived chapter markers (M4+M9B+M9C taxonomies) + camera director with Catmull-Rom interpolation + 5 overlays (HUD/kill_feed/cause_chain/chapter_timeline/watermark) + commentary mixer at 48kHz stereo + tiny-skia software rasterizer + cf-app Export Last Replay CTA |
| M11 | ✅ | Readability + ACC-A Floor | 12 HUD nodes / 7 zones, 200% scale, high contrast, War Thunder angle widget, 25-scenario PASS verdict table |
| M11A | ✅ | Shell UI Foundation | `cf-shell` crate (12 modules, 56 tests), splash + title + main menu + pause + save/load + settings tree + credits + loading screen + FRE wizard + 48 shell-widget SVGs |
| M12 | ✅ | Vivid Color-Rich Illustrated Aesthetic + Juice | Hand-drawn ink-accent UI, 12 mission comic panels, death-recap-as-graphic-novel, juice rules per DR-055 |
| M12A | ✅ | Tier 1 Audio Pipeline | 1200+ SFX target via Stable Audio Open / AudioCraft + caption metadata · **Tier 2 ElevenLabs SFX shipped (242 / 242)** |
| M12B | ✅ | HRTF Spatial Audio + Per-Room Reverb + Per-Material Echo + Per-Source Occlusion | HRTF 3D positioning (MIT KEMAR) + per-room reverb derived from M19 volume + wall-material mix + per-material echo/decay/transmission-loss/low-pass for 14 materials + wall occlusion + Doppler + per-medium filtering (water/smoke/vacuum/ammonia) |
| M12C | ✅ | In-Engine Cinematic Cutscenes + Mission-Opening + Between-Mission + Ending Cinematics + Camera Cinematography | In-engine cinematic playback distinct from comic panels + per-mission opening 30-60s + between-mission 15-30s + ending 2-5min + per-storyteller bias profiles + 5 camera primitives (pan/dolly/zoom/orbit/shake) + ElevenLabs narration sync + chapter markers |
| M13 | ✅ | Equipment + Chassis + Damage Grammar | 3 chassis archetypes, 15-zone body graph, layered armor, module state machine, pilot eject, brain-hop API |
| M14 | ✅ | Full Collision + Impulse Routing | Universal gravity field, projectile-projectile CCD, War Thunder penetration ray, HE / HEAT / APFSDS, spalling |
| M14A | ✅ | Limb-Driven Walking + CC Feel + Sim Overlay + Mass + Jetpack + Heavy Armor + Quick Action | 123 PARITY gates: walking sim + per-pixel material overlay + Stationeers atm overlay + per-origin resource overlay |
| M14B | ✅ | Gravity Field + Wind Force Producers | Per-cell + per-region gravity overrides (wells / low-g labs / magnetic boots) + cf-atmos wind-from-ΔP producer + gas stratification |
| M14C | ✅ | Tank-Grade Rounds (HEAT + APFSDS Producers + Content) | Promote M14 forward-compat to producers + 1 HEAT launcher + 1 APFSDS chassis weapon + ERA module + kill-cam variants |
| M14D | ✅ | Projectile-Projectile CCD | Selective broadphase + narrowphase pair-collision; kinetic deflect / fuze-trigger / APS intercept; C-RAM base defense |
| M14E | ✅ | Per-Pixel Structural Integrity + Tunnel Collapse | Per-chunk integrity field + vibration accumulator + cave-in roll + falling-debris damage + cascade-to-neighbor-chunks |
| M14F | ✅ | Lateral Wall Collapse + Sidewall Structural Integrity | 90° rotation of M14E engine: vertical mineshafts + retaining walls + dam ruptures + bunker perimeter walls under siege |
| M14G | ✅ | Per-Wound-Type Granularity + Severity Bands + Visual Decals | 28 WoundKind variants + 6-band severity ladder + per-wound bleed/pain/infection/heal metadata + wound decal lifecycle |
| M14H | ✅ | Field Medic Workflow + Surgery + Defibrillator + Triage + Treatment Producer Catalog | 22 treatment producers + field-medic decision tree + 5-phase surgery minigame + cardiac arrest CPR/defib loop + patient queue UX |
| M14I | ✅ | Long-Term Consequences + Scars + Phantom Limbs + Aging + Veteran Persistence Functional Layer | Per-veteran scar timeline with functional debuff + biological aging clock + prosthetic loop + chronic conditions + retirement |
| M14J | ✅ | Actor Advanced Mobility (Climbing + Parkour + Grappling + Rope + Mounted Riding) | Auto-vault + wall-jump (3-chain cap) + grappling hook + verlet rope swing + ladder/cable/vine climb + zip line + mounted riding with rider+critter mass aggregation + refined swim integrating M16 water |
| M15 | ✅ | Active Material Kernel | Noita-grade per-pixel CA, 50+ materials, 30+ reactions, alchemy, flasks, GPU compute |
| M15B | ✅ | GPU Material Kernel + Precipitation Cycle | wgpu compute pipeline + CPU determinism fallback + steam → cloud → rain + acid rain on Vulcan |
| M15C | ✅ | Full Material Registry Buildout (50+ Materials with Complete Schemas) | Full ONI + Stationeers + Noita schemas: id/hardness/density/specific_heat/thermal_conductivity/melt/boil/molar_mass/9 affordance flags/toxicity/corrosiveness/radioactivity/electrical_conductivity per material |
| M15D | ✅ | Reaction Matrix Buildout (55+ Locked Reactions with Stoichiometry) | acid+iron→rust, water+lava→obsidian, H2+O2+fire→steam, oil+fire→smoke, gunpowder+spark→explosion, etc — full stoichiometry / ΔH / Ea / rate / autoignition per reaction |
| M16 | ✅ | Hazard Package + Afflictions | 18 afflictions, 6 STALKER anomalies, 20+ artifacts, swimming + underwater combat |
| M16A | ✅ | Atmospheric + Environmental Affliction Depth Layer | Per-condition consumer kernel between M16 roster + M19/M28/M9 environment events; 11 environment-driven afflictions (stuffiness / heatstroke / hypothermia / asphyxiation / refrigerant_inhalation / electrocution / illuminated / laceration / trench_foot / stamina_movement_cost / panic_freeze_env); accumulators + threshold transitions + race-aware TTD curves + per-affliction clear conditions |
| M16B | ✅ | Disease Registry + Per-Disease Lifecycle + Cure Recipe + Vaccine + Quarantine | 17 launch diseases + per-disease cure recipe + vaccine + isolation class + R0 spread + per-origin susceptibility matrix |
| M16C | ✅ | Mental Health Conditions + PTSD + Addiction + Trauma + Therapy + Pain Affliction | 8 mental-health conditions + Pain affliction + therapy NPC + 8 psych medications + comorbidity matrix |
| M17 | ✅ | Origin Reaction + Resource Model | 11-race origin matrix + no-HP-bar survival resources (blood / oil / power / caloric / bio-fluid / oxygen) + per-origin shot-force-feedback + G-Force concussion bands + helmet-breach / vacuum / oxygen-poisoning + robot overclock/downclock + INERT recovery + per-damage-type TTD table + power/heat/vacuum AI doctrine |
| M18 | ⏳ | Micro Sabotage Fun Slice | 60-90 s sabotage integrating collision + materials + hazards + origin |
| M18A | ⏳ | Animation Production Tier 1 | 1100+ frame strips (walk / hit reactions / death) via AnimateDiff |
| M19 | ⏳ | Atmospherics-Grade Kernel | PV=nRT, 10 gases, 6 combustion reactions, phase change, pipe networks, 6 launch worlds |
| M19B | ⏳ | Fuel Production + Refining Chain | Stationeers-grade extract → distill → crack → synthesize chain; diesel / kerosene / aviation_gas / rocket_fuel / RCS; slag byproduct; filling stations |
| M19C | ⏳ | Suit Life-Support Deep | 5-tier suit ladder + 5-tier oxygen tank ladder + 5-tier filter ladder + waste-tank management + helmet flush |
| M19D | ⏳ | Pipe Network Failure Cascade | Ice plug formation + over-pressure rupture chain + iso valves + leak detection + welded vs bolted pipe failure modes |
| M19E | ⏳ | Atmospheric Analyzer + Combustible Gas Detection + Auto-Response | Per-room hazard thresholds (combustible / hypoxic / hypercapnic / toxic / refrigerant / smoke) + IC10 alarm chip + multi-room quarantine cascade |
| M19F | ⏳ | Body Heat + Crew Heat Load + Humidity + Condensation | Per-actor 100W body heat output × crowding + humidity tracking + condensation/frost formation + dehumidifier + sweat hydration drain |
| M19G | ⏳ | Room ↔ Tile Atmospheric Bridge + Hybrid Sim Strategy | Canonical hybrid policy: Stationeers PV=nRT room model for sealed rooms + ONI per-tile model for outdoor/cave/breach + handshake protocol + ONI aperture flow formula |
| M19H | ⏳ | ONI Decor + Morale + Stress + Germs + Light + Disease | Per-tile decor + per-actor morale + per-actor stress accumulator + per-tile germ tracking (slimelung/food poisoning/radiation sickness) + per-tile light propagation + per-room aesthetic class grading |
| M19I | ⏳ | Critter Ranching + Plant Ecosystem Closed Loop | 7 ONI critters (Hatch / Slickster / Puft / Pip / Pacu / Drecko / Shine Bug) + 4 plant farms with full gas consumption/production cycles + grooming + morphing |
| M19J | ⏳ | ONI Overlay UI System (F1-F12 Toggle Layers) | 12 toggle overlays (oxygen / temperature / decor / germ / power / thermal_conductivity / pressure / tile_inspect / disease / priority / light / radiation) + stacking + accessibility palette |
| M19K | ⏳ | Per-Pixel Priority Queue + ONI-Style Task Allocation | Player paints priority 1-9 on dig/build/repair/mine tasks + AI workers consume queue in order + per-actor work_style preferences + AI commander emergency promotion |
| M19L | ⏳ | Gas + Liquid Registry Full (24 Gases + 33 Liquids with Complete Thermo Schemas) | 24 launch gases (O2/N2/CO2/Volatiles/Pollutant/N2O/H2O/H2/He/O3/Chlorine/Ammonia/Smoke/Ethanol vapor/CO/F2/Ar/Kr/Ne/Xe/Radon/SO2/H2S/Phosgene) + 33 launch liquids — molar mass, density, viscosity, surface tension, boiling/freezing temp per entry |
| M19N | ⏳ | Cooking + Food Chemistry + Per-Meal Nutrition + Food Storage + Per-Actor Preference + Brewing/Distilling | 4 cooking stations + 40+ meal recipes + 10+ beverages + 5 storage tiers + per-actor preferences + brewing + distilling + smoking + curing |
| M19O | ⏳ | Sleep Need + Bed Quality + Per-Actor 24h Schedule + Insomnia + Bedroom Assignment + Roommate Effects | RimWorld sleep_need + 7 bed tiers + 24h schedule grid + insomnia + roommate friction + nap zones + per-origin sleep behavior |
| M19P | ⏳ | Per-Actor Traits + Recreation + Social + Romantic + Pet Behavior + Mood Breakdown Variants | 40+ launch traits + 16 recreation activities + social interactions + romantic relationships + 4 pet types + 10 breakdown variants |
| M19Q | ⏳ | Per-Actor Memory + Grudges + Favors + Relationship Persistence + Witness Propagation + Memory Decay | Bounded `MemoryRing` (~50-120 slots with significance-aware eviction) + 15-tag launch roster (saw_steal/saw_kill/saw_kindness/received_gift/shared_meal/fought_together/saved_life/betrayed/insulted/praised/refused_aid/stolen_from/threatened/promoted/demoted) + per-tag exponential decay + grudge / favor accumulators with revenge (−4.0) and reciprocity (+3.0) thresholds + witness propagation within radius + gossip propagation through M26B chatter + deterministic save / replay persistence |
| M19R | ⏳ | Hydroponics + Soil Agriculture + Crop Lifecycle + Per-Plant Genetics + Pest + Greenhouse Climate | 8 launch crops (wheat/potato/tomato/mushroom/soy/hemp/bamboo/algae) + 5-phase per-plant FSM + per-tile soil model (NPK + pH) + NFT/DWC hydroponic tiers + 5 grow-lamp tiers + irrigation + greywater loop + 5 fertilizer recipes + 4 pest types (aphids/mites/locusts/fungal_rot) + greenhouse climate via M28D + crop rotation + seed genetics drift over generations + AI farmer doctrine |
| M20 | ⏳ | EnvironmentSignal Aggregator | Per-tick per-actor bundle: 11 slices (atmospheric / thermal / radiation / photic / EM / weather / water / acoustic / day-night / comms / gravitational) |
| M21 | ⏳ | Micro Pressure Hold Fun Slice | 60-90 s hold-the-room while atmosphere is breached |
| M22 | ⏳ | AI Pathfinding + Collision Avoidance | Hierarchical A* (tile / chunk / region), dynamic re-pathing on terrain dirty, per-team path costs |
| M22B | ⏳ | Hierarchical A* Pathfinding Deep + Cover-Finder + Per-Team Path Cost | 3-grain pathing (tile/chunk/region) auto-picked by Manhattan distance + per-team TeamCostProfile RON + M3B element-mass cover-finder + line-of-fire predicate + dirty-rect invalidator + M7B formation-slot moving-goal hook |
| M22C | ⏳ | Sound Propagation Per-Material Acoustic Dampening + Footstep Hearing + Per-Actor Hearing Sensitivity | Per-material `acoustic_dampening_db` field + BFS dampening kernel through M3B per-tile elements + conduit/duct parallel graph + per-actor `hearing_floor_db` modulated by Deafened/Heightened/Blinded + per-archetype frequency-band filters |
| M23C | ⏳ | Per-Biome Wildlife Catalog + Behavior + Combat Stats + Drop Tables + Food Chain + Migration | 26 wildlife SKUs across 7 populated worlds (Earth wolf/deer/bear/elk/fox/hawk/fish/hare/boar/spider; Mars dust-crawler/sand-vermin/scavenger-flock; Vulcan magma-skitter/heat-locust/lava-trout; Europa ice-eel/cryo-lobster/glacial-shrew; Mimas microgravity-crab/null-grub; Moon stripped) + 6 behavior archetypes + per-biome population caps + off-screen statistical food-chain predation + seasonal migration via M31 triggers + M16 disease vectors (rabies/venom/parasites) |
| M23D | ⏳ | Server-Side GPU AI Compute Path + Per-Tick Inference Budget + Multi-Actor Batched Inference | 3 inference paths (`local_heuristic`/`local_gpu`/`remote_llm`) + per-tier GPU AI budget (Workstation 1.5ms / VPS 0ms / Mini Lab 2.5ms) + multi-actor batched dispatch (sort by actor_id ASC) + 5 launch models (tactical_proposal / loadout_optimizer / commander_adaptation / pain_memory_update / chatter_select) all WGSL+CPU-paired + cross-OS replay parity + competitive-tier signed-model gate via M36F |
| M23 | ⏳ | AI MIND Layer | Async LLM mind never blocks local AI, per-actor 5 s ticks, doctrine proposals, sandbox safety |
| M23B | ⏳ | AI Memory + Persistent Commander Doctrine + LLM-Backed Tactical Plans | Per-actor BotMemory (32-deep pain ring buffer + weapon-threat ranker + 10-deep mission outcome log) + CommanderDoctrineState persisted across missions + 6 doctrine archetypes (Defensive/Aggressive/Attritional/Asymmetric/Decapitation/Scorched) + async 5s LLM tactical planner |
| M24 | ⏳ | AI Environmental Competence | 8-test AI-MAT suite, bots wear O2 in vacuum, retreat from radiation, downclock under heat |
| M24A | 🔄 | VFX Tier 1 | 600+ particle configs + 80+ textures (impact / spark / explosion / debris) · **Tier 1 partial: 120 Particle entries + 64 VfxFrame + 96 VfxDecal shipped** |
| M25 | ⏳ | Campaign + Base + Commander Spine | 30+ missions, 5 storytellers (Cassandra / Phoebe / Randy / Ironman / Sandbox), buy menu, 8 stratagems, persistent AI commander rivalry |
| M25A | 🔄 | Narrative + Codex Production | 5k-word bible → 80k words · 600 codex entries · 24 named NPCs · **Tier 1 narrative bible authored: 83 files (factions / worlds / NPCs / storytellers / bosses / missions / origins / 300 codex / 75 achievements / 60 loading tips / credits template)** |
| M25B | ⏳ | Narrative Tier 2 LLM Expansion | LLM pipeline expanding 83 Tier 1 files into 600 codex + 24 NPC dossiers + 30 mission briefings + 100+ chatter lines + 5 storyteller scripts + 60+ loading tips |
| M25C | ⏳ | Per-Storyteller AI Algorithm + Escalation Curves + Decision Logic + Event Budget Rules + Wealth/Threat Scaling + Test Harness | 5 storyteller algorithms (Cassandra Classic +0.0 bias 24h telegraph / Phoebe Chillax -0.2 bias 48h telegraph / Randy Random ±0.3 bias no telegraph + chaotic budget / Ironman +0.1 bias permadeath enforcement / Sandbox 0.0 opt-in-only) + per-storyteller budget rule + wealth/threat scaling + pacing guardrails + storyteller AI test harness (5×4 A/B regression matrix + same-seed determinism + ≥30% Hamming differentiability) |
| M26 | ⏳ | Factions + NPCs + Narrative | 8 factions, relationship matrix, quartermasters, diplomacy, dialog trees, 8 quest templates, 4 pet companions |
| M26B | ⏳ | Dialogue UI + Conversation Trees + Speaker Portraits + Voice Playback + Skip/Replay + Per-NPC Dialog State | Dialog UI with speaker portrait + name plate + typewriter text + 4-button choice budget + voice playback via M12A/M37A + skip + replay + per-NPC `npc_dialog_state` persisted in save + 128 launch templates (8 factions × 4 roles × 4 choices) + 8 quest templates wired as offer/checkin/complete sub-trees |
| M26C | ⏳ | Trading Post UX + Per-Faction Currency + Exchange Rates + Bartering Grammar + Contraband + Market Simulation | 8 faction currencies (Coalition Credits / Ronin Marks / Insectoid Chitin / Crystalline Shards / Methane-Breather Glyphs / Aqueous Pearls / Photosynth Seeds / Heavy-Biomech Bio-Tokens) + volatile 9×9 ExchangeRateMatrix + 4-tab Trading Post UX + structured bartering grammar + per-faction contraband + smuggling concealment + per-NPC vendor rotation + supply/demand market-curve simulation |
| M26D | ⏳ | Prisoner + Captive + Interrogation + Ransom + Parole + Breakout | Non-lethal capture (5 weapons + binding tools + per-actor knock-out threshold + 30s pre-bind escape window) + `prison_cell` zone via M28F + per-prisoner Need profile + 4 interrogation styles (humane/coercive/drugs/torture with DR-031 ethical opt-in) + ransom negotiation per-faction modifiers + parole + execution + recruitment offer + deterministic per-prisoner BreakoutPlanner FSM (observe → plan → acquire tool → execute) |
| M27 | ⏳ | Loot + Progression + RPG | 5 rarity tiers, 30+ affixes, set bonuses, XP / level, 30+ achievements, 20+ perks, 10+ curses, Tetris inventory |
| M27A | ⏳ | Player Game UI | Inventory Tetris, loadout, cosmetic locker, codex (600), achievements (75), tutorial menu, comparison tooltip |
| M27B | ⏳ | Loot Table Registry + Per-Enemy + Per-Container + Per-Location Drops | Per-enemy + per-container + per-mission + per-location + per-difficulty loot tables + luck stat + drop dedup + conditional drops |
| M27C | ⏳ | Identification UX + Magic Affix Prefix/Suffix Grammar + Naming + Reveal Pipeline | Diablo-grade unidentified state + identification station + affix grammar (50+ prefixes + 50+ suffixes) + 20+ named unique items |
| M28 | ⏳ | Base Atmospherics | Pumps, vents, pressure doors, breach repair, heaters, coolers, radiators, coolant loops, emergency venting |
| M28A | ⏳ | Base Build Mode UX | Palette, ghost preview, rotation, room detection, blueprints, demolish / repair, multiplayer co-build |
| M28B | ⏳ | Base Thermal Engineering + Breach Response | Heaters / coolers / radiators / coolant loops / pressure doors / breach-repair tool / emergency vent + F6 thermal heatmap |
| M28C | ⏳ | Vehicle Bay + Loading Dock | Vehicle bay airlock seal + loading dock + cargo manifest UX + forklift AI auto-load doctrine |
| M28D | ⏳ | Per-Room HVAC + Climate Control + Multi-Zone Comfort | Per-room thermostat + ductwork + dampers + diffusers + grilles + zone setpoints + climate presets (Combat / Cryo / Hot Spring / Standard) |
| M28E | ⏳ | Filter Maintenance + Refrigerant Cycle + Fire Suppression | Filter clog lifecycle on every powered base device + refrigerant chemistry (R-22 / ammonia / CO2 supercritical) + sprinklers / halon dump / CO2 flood / smoke detectors |
| M28F | ⏳ | Blueprints + Zoning + Pre-fab Modules + Drop-In Bunkers | Blueprint export/import + Steam Workshop integration + 8 zone types with per-faction permissions + 6 drop-in pre-fab modules (bunker_t1/t2 / observation_post / hospital_module / atmospheric_module / mining_outpost) deployable in 30s via dropship |
| M28G | ⏳ | Production Supply Chain + Resource Budget Accounting + Factorio-Style Throughput | Per-base resource budget table (power/water/atmosphere/food/ore/byproduct) + Factorio-style Sankey throughput + 60s sliding-window bottleneck auto-detection + IC10 conditional routing + per-resource over-production alarms |
| M29 | ⏳ | Power & Electrical Engineering | 8 generators, 6 cable tiers, 6 storage types, IC10 priority chips, brown-out cascades, per-actor personal power |
| M29A | ⏳ | Power Grid + IC10 Editor UX | Factorio-style overlay, IC10 editor with breakpoints, per-generator dashboard, brown-out cascade viz |
| M29B | ⏳ | Personal Equipment Power Datasheet | Locked numeric per-equipment power-draw table (25+ SKUs: NVG / radio / heated suit / jetpack / exoskeleton / plasma cutter / mining laser etc.) |
| M29C | ⏳ | Power Grid Failure Cascade + Recovery Scenarios | 8 flagship scenarios: reactor meltdown / battery attrition / brown-out under raid / cold snap / EMP raid / battery fire / solar eclipse / arc-fault chain |
| M30 | ⏳ | Basic Mining + Refining | 4 mining tools, 7 launch ores, 3-world ore distribution |
| M30B | ⏳ | Mining Tool Tier Ladder + Material-Aware Dig Speed | Material-aware try_carve + 5 missing tool tiers (BareHands / Shovel / Pickaxe / Jackhammer / PlasmaCutter / MiningLaserBeam / mech HeavyDrillArm) |
| M30C | ⏳ | Tunnel Collapse + Structural Support Gameplay | Support_beam placement UX + AI miner doctrine + vibration accumulator + Holdout Extraction flagship mission |
| M30D | ⏳ | Geological Survey + Prospecting Depth | 4-tier sampling ladder (T0 hand → T3 radar profiler) + multi-band echo reveal + ore-density heatmap overlay |
| M30E | ⏳ | Refining Byproducts + Slag Recovery | Slag back-pile (loose_fill) + secondary acid-bath extraction + waste gas to atmospherics + sulfur byproduct + radioactive waste containerization + dirty-bomb weaponization (M48 covert ops) |
| M31 | ⏳ | Weather + Day/Night Kernel | 7 weather states, 24-hour cycle, AI behavior shifts, 12 total worlds |
| M31B | ⏳ | Weather States Full Catalog + Per-World Variants + Storm Survival Loop | 17 weather states authored as data rows + per-world tables for all 12 worlds (Earth/Mars/Moon/Phobos/Deimos/Mimas/Europa/Vulcan/Venus/Sol-zone/Belt/Orbital) + 5-effect contract per state (visibility/traction/wear/temperature/atmosphere overlay) + storm survival FSM (warning→active→shelter) + AI shelter-seek doctrine |
| M31C | ⏳ | Multi-Day Calendar + Seasons + Per-World Year Length + Holidays + Birthdays + Crop Seasons + Per-Cycle Statistics | `cycle_id` u32 monotonic counter + per-world `day_length_hours` (Earth 24.0 / Mars 24.6 / Phobos 7.66 / Vulcan 1401.6 / Venus 2802.0) + `year_length_days` (Earth 365 / Mars 687 / Mimas 22.6 / Europa 3.6 / Vulcan 224 / Venus 117) + 4-season FSM on Earth-likes + 40 launch holidays (8 per major faction × 5) with recipe + cosmetic + event unlocks + per-NPC birthday with ×3 gift bonus + per-crop growing season + `cf-stats::CycleLedger` ledger (7 counters per cycle: kills/deaths/crafted/mined/harvested/built/lost) |
| M32 | ⏳ | Crafting Tiers + Fabrication Chain | 5-tier ladder (T0..T4), 150 launch recipes, fabrication chain, power-coupled, 30-node research tree |
| M32A | 🔄 | Tier 2 ComfyUI Pipeline | 4500+ production assets via SDXL + Flux + AnimateDiff + ControlNet + per-faction LoRAs · **Tier 1 NPC portrait placeholders shipped (44 entries)** |
| M32B | ⏳ | Crafting + Research + Salvage UX | 3-pane crafting (Stationeers / Terraria / Factorio hybrid), research tree pan / zoom, material flow Sankey |
| M32C | ⏳ | Per-Actor Crafting Skill + Quality Output Tiers + Auto-Restock + Linked Storage + Recipe Discovery | RimWorld 0-20 skill ladder × 6 categories + 7 quality tiers (Awful → Legendary) + Terraria linked-chest auto-restock + Noita recipe permutation |
| M33 | ⏳ | Modding Workbench | In-game scenario editor, Lua, IC10 chip editor, Steam Workshop, photo mode, replay browser, 8 tutorial labs |
| M33A | 🔄 | Tutorial Lab Production | 8 modular labs + First Contract + FRE wizard · **Tier 1 voice prompts authored (70+ tutorial narration lines)** |
| M33B | ⏳ | Lua Sandbox API Surface + Hot-Reload + Per-Callback Permissions + Modder Documentation | 10 canonical Lua callbacks (OnUpdate/OnFire/OnUse/OnDeath/OnHit/OnSpawn/OnTick/OnReload/OnCraft/OnTransition) + per-callback capability table + hard-banned forbidden APIs (file I/O / network / os.execute / shell / native libs) + curated `_ENV` whitelist + hot-reload pipeline with rollback + Lua-table↔Rust-struct marshal + per-tick CPU budget + API doc generator |
| M33C | ⏳ | Mod Permissions + Trust Tiers + Cert Chain + Conflict Resolution + Steam Workshop Deep | 4 trust tiers (vanilla/verified/community/experimental) + capability + competitive-eligibility matrix + cert chain (root CA pinned in binary → intermediate → leaf) + revocation list + deterministic load-order resolver + dependency graph + 5-category conflict diagnostic UI + per-mod resource quotas + Workshop upload/subscribe/auto-update/pin/rollback/collection/author-follow |
| M33D | ⏳ | FRE Wizard + Contextual Tooltips + Progressive Disclosure + Skill-Rating Bot + Tutorial Replay + Sandbox Mode + 8 System Mini-Tutorials | FRE 5-step wizard (Language / Accessibility presets / Difficulty recommender / Game-mode pick / Calibration arena) + per-widget contextual tooltips with 3 verbosity tiers + progressive disclosure across 8 advanced HUD surfaces + `cf-ai::skill_rating_bot` evaluating aim+pathing+resource_management on 30s rolling window + tutorial replay/rewind + sandbox mode + 8 system mini-tutorials (Atmospherics/Mining/Squad/Cooking/IC10/Crafting/Combat/Vehicles) |
| M33E | ⏳ | Mod Review + Curation + Recommendation Engine + Author Monetization (DR-031 Compliant) | In-game mod review widget + community report-and-flag system + per-mod 5-star rating + monthly mod-of-the-month pipeline (community-voted + dev-curated + 3 rising-stars + genre-spotlight) + per-player kNN recommendation engine + tip-jar + paid-cosmetic mod (DR-031 capability-only allow-list refusing on `sim_write`) + Workshop revenue share + anti-clone perceptual-hash detection |
| M34 | ⏳ | Material Lab | DR-036 workbench, brushes, 17 materials, 13 lab-unlocked, recipe inspector, stamp save / load |
| M35 | ⏳ | Advanced Mining + Extraction | 9 mining tools, 12 ores, AI-MINE-A 8-test suite, server-authoritative ledger |
| M36 | ⏳ | Dedicated Server + Determinism Islands | 5 server modes, `cf-server-ops` / `persistence` / `anti-cheat` / `admin`, 4 mod trust tiers |
| M36C | ⏳ | Server GPU Compute Path + Multi-Core Threading + Performance Budget Enforcement | Server tier detection (Workstation/Apple Silicon/Linux VPS/Apple Mini Lab) + GPU compute features per tier + per-tick budget enforcement (16.6 ms total) + 8 determinism hard rules |
| M36D | ⏳ | GPU Compute Job Scheduler + Cross-OS Determinism Gates + CPU/GPU Parity Validation | Per-tick GPU job queue with backpressure + WGSL shader registry + per-shader CPU fallback + per-frame parity check (1/15 cadence) + CI gate runs full simulation on CPU + GPU in parallel verifying byte-identical state |
| M36E | ⏳ | MMO Shard Meshing + State Handoff + Per-Shard Resource Budget | Persistent MMO architecture (DR-035): multi-shard mesh + per-shard authoritative actor list + state handoff at shard boundary + cross-shard event broadcast + per-shard CPU/GPU isolation + rollback at shard boundary + NTP-synchronized global time |
| M36A | ⏳ | Platform Integration | Steam SDK, Discord rich presence, EOS adapter, Workshop, Cloud, Achievements bridge |
| M36B | ⏳ | Telemetry + Crash + Bug Report | Opt-in privacy, per-shard analytics, in-game bug-submit |
| M36F | ⏳ | Anti-Cheat + Integrity Deep + Cheat Detection Heuristics + Banwave Protocols | Server-authoritative per-action validation via deterministic replay re-run + 3 behavioral heuristics (impossible-input / aim-bot consistency / wall-hack visibility-vs-render) + M4B chain-of-custody verification + M33C cert hierarchy + revocation list + 4-tier graduated ban state machine (warning → 24h → 7d → perma) + banwave batch validator + appeal portal |
| M36G | ⏳ | Performance Benchmark Suite + Per-Platform Reference Numbers + Regression Test Gates + Frame-Time Profiling Tools | `cf-bench` per-subsystem benchmarks + locked per-platform reference numbers (Workstation / Apple Silicon / Linux VPS / Apple Mini Lab) + per-PR p99 regression gate + auto-bisect bot naming offending commit + `cf-profile` frame-time profiler with per-system + per-mod attribution + `cf-tools bench-dashboard` rolling 30-day percentile charts |
| M36H | ⏳ | Per-Mission Analytics + A/B Test Framework + Opt-In Privacy Controls + Player Progression Tracking | Per-mission completion/median/loss-cause/quit-point heatmap + deterministic blake3-based A/B cohort assignment + chi-squared significance gating + 5 per-data-type opt-in toggles (basic/gameplay/chat/voice/replay — all default OFF, chat+voice locked behind extra confirmation modal) + retention cohort tracking (day-1/7/30) anonymized via per-account rotating salt + funnel analysis with drop-off detection + per-shard admin dashboard |
| M36I | ⏳ | Steam Achievements Deep + Cloud Saves + Workshop API Deep + Rich Presence + EOS Adapter Deep | Per-achievement schema (7 condition kinds, visibility states, dependent chains, cosmetic rewards) registered across 12 category files for 75+ achievements + versioned `CloudSaveEnvelope` with latest-tick-wins conflict resolution + manual-merge UI for ties + 7-day conflict-history retention + 100 MiB quota + Workshop revision/channel (stable/pre_release/archived) + reverse-dep warnings + atomic collection subscribe-all + 7-state rich-presence state machine + EOS account-link enabling unified Steam+EOS friend list |
| M37 | ⏳ | Voice + Radio + Comms | ACRE2-tier radio (distance / occlusion / frequency / antenna), EMP, vacuum no-voice, AI chatter, captions |
| M37A | 🔄 | Tier 2 Audio + Voice + Music | 7000+ voice clips target + 30+ music tracks + adaptive music engine · **Tier 2 ElevenLabs shipped: 242 voice + 39 music; 81 music remain (RTX 5090 / AIVA handoffs ready)** |
| M37B | ⏳ | Per-Equipment Radio Power Draw + Range Curve | 5-tier radio ladder (handheld VHF → squad UHF → vehicle HF → base LF → HQ MHz) + per-band propagation + AI Reduce Chatter Mode |
| M37C | ⏳ | Stealth Visibility + Light/Shadow + Sneak Attack + Distraction Items + Camouflage | Per-tile light bracket (Lit/Partial/Shadow/Concealed) from M19H + facing-cone behind-target evaluator + sneak-attack damage multiplier table (melee ×3.0+crit, suppressed ×1.8) + 5 distraction items (noisemaker_decoy / smoke_grenade_visual / flashbang_visual / decoy_actor / acoustic_decoy) wired through M22C + 4 camo pieces (ghillie / urban / arctic / desert) with per-biome multipliers |
| M37D | ⏳ | Adaptive Music Engine + Per-Scenario Intensity Scoring + Per-World Palette + Boss Override | Per-tick weighted intensity score (6 signals: enemies/damage/bullets/mission_critical/boss/storyteller) with 60-tick IIR smoothing + 3-band variant selector (calm 0.0-0.3 / buildup 0.3-0.6 / climax 0.6-1.0) with 0.04 hysteresis + 12s min-hold + beat-aligned cross-fades + 4-layer stack (world/faction/storyteller/boss) with boss-override priority + per-difficulty weight overrides |
| M38 | ⏳ | Server Config + Admin CLI + Settings | 200+ tunables, 7-tier hierarchy, 20+ admin commands, auto-gen settings UI, audit log |
| M38A | ⏳ | Localization (19 languages) | 11 Tier-A + 8 Tier-B, 380k+ translations via LLM auto-translation, ICU MessageFormat |
| M38B | ⏳ | Localization Runtime Deep + RTL/LTR + Font Fallback + Dynamic Text Expansion + Voice-Line Sync | UAX-9 RTL/LTR for Arabic+Hebrew + per-language font fallback chain via M9A asset bake + per-widget expansion budget (German 1.3×, Japanese 0.8×) with shrink/ellipsis/wrap policy + ICU plural+gender+date+number+currency+list resolvers + voice-line per-locale routing with deterministic fallback chain + locale-aware collation + player-text sanitization preventing ICU/markup injection |
| M39 | ⏳ | Universal Schema Locks | Manifest at `cf-mod/manifest/all_schemas.ron`, ~120 locked schemas, one-shot conformance check |
| M40 | ⏳ | LAN Co-op | 2-4 player co-op via `lan_room`, mDNS / UDP discovery, 7 squad roles, revive, death cam, mod hash sync |
| M40A | ⏳ | Spectator + Streamer Polish | Replay-to-MP4 via FFmpeg, 10+ overlay themes, Twitch / YouTube / Discord webhook integration |
| M40B | ⏳ | Online UX | Server browser, friends, party invite, lobby, admin web panel, voice chat UI |
| M41 | ⏳ | Online Co-op + Full Match Grammar | NAT punch-through, 10 endgame modes, persistent AI commander, 4+ squads × 7 role types × 12 commands, War Thunder kill cam |
| M41B | ⏳ | Mod Hash Sync + Server Authority + Lobby Discovery + Cross-Region Latency Compensation | Per-mod blake3 hash exchange + trust-tier-gated auto-download + authoritative `(global_tick, shard_id, seq)` ordering + lobby browser (region + ping + mod-compat + tier filters) + private-code resolver + cross-region MMR balancer + spectator latency-tolerant profile |
| M41C | ⏳ | Skill-Based Matchmaking + Rank Tiers + Leaderboards + Seasons + Tournament Format | Per-mode MMR/Elo + 6 rank tiers × 3 divisions + 0.4-pull-toward-1200 soft seasonal reset + weekly inactivity decay + non-punitive smurf detection + anti-throw 5-loss protection + 4 tournament formats (single-elim 16 / double-elim 16 / round-robin 8 / Swiss 16) + per-mode + per-faction top-100 leaderboards + DR-031-compliant ranking-only cosmetic decorations |
| M42 | ⏳ | Self-Hosted Server Deployment | 3 Docker tiers, systemd, launchd, Terraform, Ansible, Grafana / Prometheus / Loki, 15-min deploy target |
| M43 | ⏳ | PvE Survival Mode + Procgen | 1-8 player coop, 7-step procgen, 3 launch survival worlds, per-race difficulty matrix, acclimatization |
| M43A | ⏳ | Map + Mission + Campaign UX | World map, solar system map (12 worlds), mission select, campaign tree, briefing, travel planner |
| M44 | ⏳ | Inter-Planet Transport + Stations | 3 transport modes (dropship 8-phase / multi-stage rocket / paired teleporters), orbital stations, 7 new vehicles |
| M44A | ⏳ | Electric Ground Vehicles | 3 battery-powered SKUs (Electric ATV / Mining Hauler / Cargo Skid + Forklift) + per-circuit sub-batteries + cold-weather range penalty |
| M44B | ⏳ | Drone Logistics + Cargo Convoy AI | Multi-vehicle convoy AI + route designer + IC10 programmable routes + drone-swarm coordination + escort squads + intercept combat |
| M44C | ⏳ | Vehicle Physics + Tire Grip × Surface + Suspension + Per-Component Damage + Engine Subsystems | Per-surface tire grip table + per-vehicle suspension geometry + per-component damage (engine / transmission / radiator / fuel-tank / each wheel) + per-engine subsystem (cooling / fuel / electrical / drivetrain) + per-vehicle weight transfer |
| M44D | ⏳ | Combat Ground Vehicles (Tank + IFV + APC + SPG + Recon) | Per-vehicle SKU table for 5 launch combat vehicles + per-vehicle main gun + secondary + composite armor + reactive armor + per-component crew + per-vehicle role doctrine |
| M44E | ⏳ | Mech Walking + Tracked Vehicles + Bipedal Combat Mechs | Mech-leg walking on M14A engine + per-leg ground pressure + per-mech 6 weapon hardpoints + per-mech cockpit + mech vs tank engagement curves + jump-jet variant |
| M44F | ⏳ | Boats / Submarines / Underwater Combat | Per-boat hull + buoyancy + per-sub depth-pressure + torpedo + depth charge + sonar + underwater combat per M16 swimming + submarine bay docking |
| M44G | ⏳ | Combat Helicopters + Utility + Scout (Attack / Utility / Scout) | Per-helicopter rotor physics + per-rotor independent damage + ground effect + autorotation emergency landing + door-gunner role + Hellfire / chain gun / rocket pod / cargo sling + per-pilot skill curve |
| M44H | ⏳ | Fighter Jets + VTOL Combat Aircraft + Recon Drones | 4 launch aircraft (F-16 / A-10 / B-1B / Harrier-F-35B VTOL) + 3 recon drone SKUs (Micro / MALE / Stealth Strike) distinct from M44B cargo drones + turbojet/turbofan/ramjet subsystems + pulse-doppler/AESA radar + AIM-9/AIM-7/AIM-120/AGM-65/AGM-114/AGM-158 guidance + ACES II ejection seat + 6-mode autopilot + carrier-deck tailhook + air refueling |
| M44I | ⏳ | Helicopter vs Fighter Cross-Arm Air Combat + Mixed-Altitude Engagement Rules | 6 altitude bands (nap_of_earth → stratospheric) + radar masking by terrain + ground clutter + cross-arm missile envelope table per shooter×target×weapon + AI doctrine for fighter-vs-helo + helo-vs-fighter + asymmetric kill-cam (0.4× slow-mo + dual POV) + Stinger-class AGM-defense missile loadout for M44G helicopters |
| M45 | ⏳ | PvE Endgame Bosses + World Events | 5 named bosses (Hollow King / Frozen Heart / Crimson Tide / Eclipse Walker / Last Star), 12 dynamic world events |
| M45A | ⏳ | Cosmetic Production | 2200+ items, anti-pay-to-win audit (DR-031) |
| M45B | ⏳ | 5 Named Boss Deep Specs (Hollow King / Frozen Heart / Crimson Tide / Eclipse Walker / Last Star) | Per-boss: lore strap + arena rules + EscapeRule + 3-5 phase FSM with per-zone per-phase HP + 4-7 signature attacks (telegraph ms + hitbox + counter) + explicit kill conditions + loot tables (unique recipe + artifact + cosmetic + 5 codex unlocks) + 5 per-storyteller variants + 4-tier per-difficulty tuning + co-op `second_target` scaling + per-fight environmental hazards |
| M45C | ⏳ | 12 Dynamic World Events Deep Catalog | Per-event: trigger conditions (per-tick probability + storyteller hook + world-state gate) + phase machine + 3-5 response branches (engage / avoid / negotiate / sabotage / exploit) with per-branch reward and consequence + 12 per-world variant rows + 5 per-storyteller variants + 4-tier per-difficulty scaling + 4 new replay event schemas |
| M46 | ⏳ | Upkeep Economy (Opt-In) | BRP drain, bankruptcy cascade Day 1 → 30, rescue mechanisms, AI factions follow same rules |
| M46B | ⏳ | Mercenary Contracts + Bounty System + Taxes + Tariffs + Tribute + Faction-Specific Trade Routes + Caravan Escort | Signed MercenaryContract with payment terms / RoE / loyalty drift / breach penalty + per-faction BountyBoards with HP/notoriety/equipment-scaled rewards + TaxPolicy (income/transaction/property) with audit-evasion path + cross-border TariffPolicy + TributeDemand with patron-absorption + 8 faction-operated TradeLane routes with warden response + CaravanEscortMission with multi-factor reward scaling |
| M47 | ⏳ | Strategy Phase + Goals (Opt-In) | Per-cycle decision phase, 5 stances × 5 production × 3 logistics = 75 combos, 24 launch goals |
| M48 | ⏳ | Inter-Faction Intelligence (Opt-In) | Codebreaking + spy rings + 8 covert ops + counter-intel; AI factions follow same rules |
| M48A | 🔄 | Tier 3 Polish | Top 50 Aseprite hand-polish, top 20 Spine rigs, FMOD / Kira mix, Steam Deck Verified · **Tier 1 cinematic placeholders: 23 boss splashes + 20 loading bg + 19 key art + 44 portraits** |
| M48B | 🔄 | Steam Store + Marketing | Capsule art, 12 screenshots, 6 trailer types, press kit, tag taxonomy · **Tier 1 marketing placeholders: 19 KeyArt entries** |
| M48C | ⏳ | Endgame + Workshop UX Polish | Debrief, replay browser, photo mode, mech bay, pilot / commander dossier, faction diplomacy, quest log, NPC dialog, hub, mod manager |
| M49 | 🚀 | Public PvP + Persistent MMO + Bunker Defence | **Launch GA** · public PvP arenas · MMO shards · Bunker Defence flagship mode · 4 launch shards · cross-shard events · `v1.0.0` |

> Spec files in [`specs/active/`](specs/active/) · closed specs in [`specs/done/`](specs/done/) · per-spec workflow contract in [`AGENTS.md`](AGENTS.md).

### Depth-extension family (lettered suffixes are the no-compromise refinement layer)

The lettered-suffix companions (`MnA` / `MnB` / `MnC` / `MnD` / `MnE`) sit **next to** their parent milestone — they don't replace it. The parent ships the base contract; the lettered companions add depth + fidelity + content + UX without rewriting the parent. This pattern lets the roadmap reach Stationeers-grade / Noita-grade / ONI-grade / Cortex-Command-grade simulation depth without bloating the parent specs.

The current depth-extension waves:

- **Physics depth** — M14 (collision) → M14A (limb walking + sim overlay) + M14B (gravity overrides + wind producer) + M14C (HEAT + APFSDS tank rounds) + M14D (projectile-projectile CCD) + M14E (per-pixel structural integrity + tunnel collapse) + M14F (lateral wall collapse + sidewall integrity) + M14G (per-wound-type granularity + severity bands + decals) + M14H (field-medic surgery + defibrillator + triage + treatment catalog) + M14I (long-term scars + phantom limbs + aging + veteran persistence) + M14J (actor advanced mobility).
- **Materials depth** — M15 (active material kernel) → M15B (GPU material kernel + precipitation cycle) + M15C (50+ material registry buildout with complete schemas) + M15D (50+ locked reaction matrix — Noita alchemy + Stationeers stoichiometry).
- **Affliction / biology depth** — M16 (hazard package + 22 afflictions + anomalies + artifacts + swimming) → M16A (11 atmospheric + environmental afflictions) + M16B (disease registry + per-disease lifecycle FSM + cure recipe + vaccine + quarantine + per-origin susceptibility) + M16C (mental-health: 8 conditions + PTSD + addiction + trauma + therapy + Pain affliction + the affliction→aim/move-speed combat consumer).
- **Atmospherics depth** — M19 (PV=nRT kernel) → M19B (fuel production + refining chain) + M19C (suit life-support deep).
- **Narrative depth** — M25 (campaign spine) → M25A (Tier 1 bible) → M25B (Tier 2 LLM expansion to 600 codex / 24 NPCs / 30 mission briefings / 100+ chatter / 5 storyteller scripts / 60+ loading tips).
- **Building depth** — M28 (base atmospherics) → M28A (build mode UX) + M28B (thermal engineering + breach response) + M28C (vehicle bay + loading dock).
- **Power depth** — M29 (electrical kernel) → M29A (grid + IC10 editor UX) + M29B (per-equipment power datasheet) + M29C (8 cascade-recovery flagship scenarios).
- **Digging depth** — M30 (basic mining) + M35 (advanced mining) → M30B (tool tier ladder + material-aware dig speed) + M30C (tunnel collapse + structural support gameplay) + M30D (geological survey + prospecting depth).
- **Radio depth** — M37 (voice + radio) → M37A (audio production) + M37B (5-tier radio + per-band propagation + range curve).
- **Vehicle depth** — M44 (inter-planet transport) → M44A (electric ground vehicles: ATV / mining hauler / cargo skid + forklift).

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

**67 crates** under [`game/crates/`](game/crates/). Each crate has its own `AGENTS.md` boundary contract.

```text
sim core              cf-sim-core · cf-actor · cf-physics · cf-terrain · cf-material · cf-material-gpu · cf-chassis
                      cf-equipment · cf-internal · cf-mission · cf-perception · cf-squad · cf-ai · cf-environment
                      cf-atmos · cf-priority
damage + biology      cf-wound · cf-treatment · cf-scar · cf-aging · cf-prosthetic · cf-veteran · cf-affliction
                      cf-disease · cf-mental-health
world systems         cf-hazard · cf-anomaly · cf-artifact · cf-swim · cf-flask · cf-storyteller · cf-trench
                      cf-fortification · cf-procgen · cf-content
control + replay      cf-control · cfctl · cf-replay · cf-replay-scrub · cf-replay-export · cf-headless · cf-mod
                      cf-asset-ledger · cf-save · cf-e2e · cf-bench
runtime + presentation cf-app · cf-render-2d · cf-camera · cf-capture · cf-ui · cf-shell · cf-debug · cf-audio
                      cf-photo · cf-killcam · cf-cinematic · cf-localization · cf-squad-ui
networking + server   cf-net · cf-server · cf-server-ops · cf-server-persistence · cf-server-anti-cheat
                      cf-server-admin
tooling               cf-tools-editor · cf-tools-replay-viewer
```

The **damage + biology** and **world systems** families are the M13–M17 buildout: typed wounds + field surgery + scars/aging/prosthetics/veterans (`cf-wound`/`cf-treatment`/`cf-scar`/`cf-aging`/`cf-prosthetic`/`cf-veteran`), the 28-affliction roster + Pain consumer (`cf-affliction`), the 17-disease lifecycle (`cf-disease`), the 8-condition mental-health FSM (`cf-mental-health`), the M17 per-origin reaction + resource model (`cf-actor::{origin,concussion,oxygen,power,internal_shock,overclock}` + `cf-control::m17_origin`), plus hazards/anomalies/artifacts/swim/flasks and the trench + fortification authored-content engines.

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
cargo run -p cf-mod -- ledger verify --strict       # expects: total=7710 fresh=7710 stale=0 ...
cargo run -p cf-mod -- validate ../content/         # expects: pass>0 fail=0
cargo test --workspace --lib --no-fail-fast         # 3,593 lib tests
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
