<div align="center">

# Corefall

**Cortex-Command-like tactical pulp sci-fi disaster sandbox**

**Server owns truth. GPU owns richness. Client owns feel.**

A 2D side-view physics sandbox where every gas, grain, bullet, body, world, and transmission is real. Bunker Defence is the flagship mode: attackers breach, defenders hold the command core, and every pressure seal, bullet-punched aperture, liquid jet, thermal leak, radio shadow, projectile, wound, fire, and collapsing room is part of the same replayable simulation.

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

[![Status](https://img.shields.io/badge/status-prealpha%20%28BP2%20%E2%9C%93%20closed%2C%20BP3%20next%29-orange?style=flat-square)](#project-status)
[![Build Points](https://img.shields.io/badge/Build_Points-BP2%20closed%20%2F%20BP3%20active-2EA043?style=flat-square)](#build-points)
[![Roadmap V2](https://img.shields.io/badge/roadmap-V2%202026--05--08%20additive-8A2BE2?style=flat-square)](#roadmap-shape)
[![Releases](https://img.shields.io/github/v/release/Madreag/corefall?include_prereleases&sort=semver&style=flat-square&label=release)](https://github.com/Madreag/corefall/releases)
[![Vault](https://img.shields.io/badge/research-research%20vault-purple?style=flat-square)](https://github.com/Madreag/corefall#research-vault)

> **Where we are:** badges above are the source of truth — they auto-update on every BP closure. Full milestone matrix in [Project status](#project-status); BP scope table in [Build Points](#build-points); release policy + download-when-ready in [Releases](#releases).
>
> **Where to play:** today, build from source via [Getting started](#getting-started). First friend-handoff release (`.dmg` / `.msi` / AppImage — double-click to play, no Terminal) lands at the BP3 closure per the [Double-Click Playability Hard Gate](#releases).

**[Project status](#project-status) · [Build Points](#build-points) · [Roadmap shape](#roadmap-shape) · [Releases](#releases) · [Tech stack](#tech-stack) · [Getting started](#getting-started) · [CI](#ci)**

</div>

---

## What Corefall Is

Corefall is the implementation repo for a **tactical pulp sci-fi disaster sandbox**. The player fantasy is Cortex Command's command-core, dropship, chassis, digging, and body-swapping chaos rebuilt around a stricter simulation contract: deterministic replay, server-authoritative multiplayer, real atmospherics, systemic materials, universal gravity, full collision, and scriptable AI/modder workflows from day one.

You will:

- **Defend the bunker** as 1-4 humans plus AI guards, or play the attackers and breach it with dropships, explosives, tunneling, pressure sabotage, fire, and misdirection.
- **Fight across worlds** with different ambient atmospheres, gravity, weather, day length, and hazards: Earth, Mars, Phobos, Deimos, the Moon, Mimas, Europa, Vulcan, Venus, Sol-zone habitats, belt asteroids, and orbital stations.
- **Use physics as tactics**: vent rooms, overpressure corridors, shoot pressure holes, create liquid/gas jets, light flammable atmospheres, cut coolant lines, heat or cool rooms, collapse terrain, redirect gravity, burn oxygen, mine ore, and salvage wreckage.
- **Swap between origins**: humans breathe and concuss, androids bridge organic and synthetic failure chains, robots overclock, downclock, short, leak coolant, and ignore hazards that kill flesh.
- **Coordinate over simulated comms** with radio propagation, interference, EMP failure, captions, accessibility fallbacks, and AI reason labels that explain what bots believe is happening.
- **Build, capture, or destroy bases** with real rooms, pipes, airlocks, pressure regulators, power, storage, fabrication, doors, platforms, and modder-defined modules.

This is not a Cortex Command remake. It is a **best-of-genre synthesis** that takes Cortex Command's command-core / dropship / chassis / digging fantasy and sets it on top of Stationeers-grade-or-better atmospherics and thermal engineering, Noita-grade systemic materials, full collision physics, universal gravity, ACRE2-tier voice + radio simulation, and a full astrography of playable worlds. AI bots are first-class teammates and rivals. Replay is deterministic. Modding is data-first. Accessibility is a floor, not an afterthought.

---

## Roadmap Shape

The canonical roadmap now covers **57 closed/directional decision records**, **31 sequenced milestones** (24 gameplay-spine + 3 micro-fun-slice interludes + 4 split sub-milestones from M3/M4), **17 side tracks** including 4 dedicated production tracks (art, narrative, localization, live-ops), and a **13-step Build Points layer (BP0..BP12)** that bundles related milestones into shippable proof slices. The Roadmap V2 (2026-05-08) additive pass kept every prior milestone/DR id stable; BPs and micro-fun interludes layered *on top of* M0..M12 without renaming anything. The repo should be read as the executable slice of that plan, not the whole design database.

| Area | Current Direction |
|---|---|
| Authority model | Solo runs as an in-process server. Multiplayer uses `cf-server` as the canonical fixed-tick truth. Clients predict feel locally; server corrections preserve shared state. |
| GPU policy | GPU is used aggressively for rendering, particles, lighting, smoke, debug overlays, prediction, advisory maps, and optional certified server acceleration. CPU deterministic sim remains the required authoritative fallback. |
| Asset/audio pipeline | All project assets and audio are AI-agent-authored, ledgered, regenerable where possible, captioned where audible, and exposed to modders through the same pipeline. |
| Content target | Launch roadmap tracks 70+ weapons, 44+ actors, 18+ vehicles, 60+ base objects, 8 factions, 30+ missions, 12 worlds with biomes, 17 materials, 12 ores, 30+ tracks, and 400+ SFX. |
| Shell and platform | Title/menu/settings/lobby/workbench/briefing/debrief/map/achievements/replay viewer/codex/photo mode/cosmetic locker/death cam/mod manager, plus Steam Workshop, cloud, achievements, input, and Deck readiness. |
| Accessibility | ACC-A floor plus cognitive, motor, hearing, reading, and sensory presets. Captions, full subtitles, reduced motion/shake/flash, UI scaling, high contrast, remap/hold alternatives, and screen-reader paths are roadmap gates. |
| Modding | Modders extend chassis, weapons, materials, atmospheres, missions, AI doctrines, audio, animations, scripts, factions, localization, and test runs with the same schema-first pipeline as the base game. |
| Monetization posture | Premium one-time purchase direction, free modding, community-hostable servers, no pay-to-win. Optional cosmetic-only gacha/battle-pass hooks are late-cycle, dormant/default-off systems and require a future activation DR. |
| Production T-tracks | 4 dedicated late-anchored tracks ride alongside the gameplay spine: **T-CONTENT-ART** (AI-authored art/animation/VFX/decals/lighting/music/SFX/launch roster), **T-CONTENT-NARRATIVE** (~80,000 words bible/codex/dialogue), **T-LOCALIZATION** (Project Fluent + 11 Tier-A langs + 8 Tier-B UI-only + mod-localization), **T-LIVEOPS** (telemetry, marketing, Steam, legal, post-launch ops). Each finalizes at BP12; placeholder generation begins at BP3+. |
| Micro-fun-slice cadence | After every major systems milestone, a short interlude proves the new system is *fun* before the next one stacks on top: M1.5 (breach), M2.5 (reactor defense), M5.5.5 (sabotage with collision + breach), M5.9.5 (pressure hold). Each is a 60-90 s scenario driven by cfctl, validated by run bundles, gated by a human-playtest survey before the next BP unlocks. |
| AI-agent self-testing | **T-CAPTURE** layer (BP2+): cf-app emits PNG frame readbacks at 10 Hz baseline + event-triggered keyframes; `game/tools/capture_grid.py` composes them into 8×8 grid PNGs + a `summary_grid.png` (one frame per major event, ≤64 frames). cf-e2e drives the loop end-to-end. AI agents read the summary grid via the `Read` tool to validate motion + physics + effects without a human eyeballing every smoke run. |
| Actor presentation | Animation-first while controlled, physics-first while disrupted, and always replay/event-visible. Visible actor movement cannot remain a static sliding pawn once the milestone owns movement/body presentation: walking, running, crouching, climbing, jetting, aim blending, limb damage, knockdown, pressure/wind/explosion response, and mech gait all need observable state, events, and capture evidence at their maturity level. |

---

## Build Points

The Roadmap V2 layer groups gameplay milestones into **13 Build Points (BP0..BP12)**. A Build Point is a shippable proof slice — when a BP closes, every milestone inside it has the Acceptance Matrix + Contract Integrity Matrix + Performance/Config Audit + run-bundle evidence, *and* the human-playtest gate has been passed. BPs do not replace milestones; they bundle them.

| BP | Anchor | Bundles | Fun-Proof Slice | Status |
|---:|---|---|---|---|
| **BP0** | Engine bootstrap | M0 | (kickoff smoke) | ✅ Closed |
| **BP1** | Actor controller + breach fun proof | M1 + M1.5 | M1.5 Micro Breach | ✅ Closed |
| **BP2** | Terrain & Replay Build | M2 + M2.5 + M3A | M2.5 Micro Reactor Defense + headless replay verifier | ✅ **Closed (current)** |
| **BP3** | Combat Readability Build | M3B + M4A + M5 + Double-Click Release Engineering | HUD/body/chassis proof + first friend-handoff release | 🟢 Active |
| **BP4** | Physics Sandbox Alpha | M5.5 + M5.5.5 + M5.6 + M5.7 + M5.8 | M5.5.5 Micro Sabotage | ⏳ Planned |
| **BP5** | Atmospherics & Worlds Alpha | M5.9 + M5.9.5 + M5.10 | M5.9.5 Micro Pressure Hold | ⏳ Planned |
| **BP6** | AI Combat Alpha | M6 + M6.5 + M6.6 | AI-H / MIND / AI-MAT suites | ⏳ Planned |
| **BP7** | Vertical Slice Alpha | M7 + M7.5 + M7.7 + M4B | Breach Contract + Bunker Defence proof | ⏳ Planned |
| **BP8** | Creator Alpha | M8 + M8.5 + M8.6 | modder parity smoke | ⏳ Planned |
| **BP9** | Server/LAN Alpha | M9 + M10 | LAN co-op smoke | ⏳ Planned |
| **BP10** | Online Beta | M11 + M9.5 | self-hosted online co-op + comms | ⏳ Planned |
| **BP11** | Public Systems Beta | M12 | PvP/MMO shard proof | ⏳ Planned |
| **BP12** | Release Candidate | production T-track finalization | launch GA build | ⏳ Planned |

**Build Point closure gate:** Every BP closeout requires (a) every milestone inside it PASS in the Acceptance Matrix, (b) the Minimum-Bar Design Coverage Matrix proving the worker handled obvious inside-scope game/UX affordances instead of only narrow task wording, (c) the Contract Integrity Matrix proving shared code paths + negative/adversarial proof, (d) the **Universal Enhancement Done-Criteria (DR-056)** matrix PASSing for every M1+ milestone inside the BP, (e) run-bundle evidence for every fun-proof slice at multiple tick rates, (f) **T-CAPTURE evidence** (each fun-proof script emits a `summary_grid.png` + `capture_manifest.json` recorded in `summary.json.artifacts`; `--expect capture.summary_grid.non_blank_ratio>=0.95` mandatory from BP2 onward), (g) **T-RELEASE evidence** (tagged GitHub pre-release from BP1 onward with the BP exemplar bundle + `summary_grid.png` + SHA256SUMS), (h) `/corefall-review <bp>` verdict = `Accept`, and (i) the per-BP human-playtest gate (Did the new systems make the game more fun than the previous BP? — recorded in `prototype_runs/native/<bp>_*` notes; the answer must reference the summary grid path it was answered against).

**Universal Enhancement Done-Criteria (DR-056) — applies to every M1+ milestone:** per-tier perf gate (Steam Deck 800p/60 + 1080p/60 + 4K/120) + CI bench regression + 24h memory-leak soak + network sync verified + replay determinism CI matrix per platform + every player surface scriptable via cfctl + AI-agent validation report + AI audio pipeline + juice rules per DR-055 + ACC-A floor + Tier-A localization keyed strings + modding parity + anti-FOMO + anti-pay-to-win audit + captions for ALL audio. Universal rows are not optional polish — a milestone is not closed if any Universal row FAILS unless the user explicitly approves that exact deferral. Per-milestone specifics layer on top of the universal contract (see [`docs/plan/spec/milestone-enhancement-pass-m1-plus.md`](docs/plan/spec/milestone-enhancement-pass-m1-plus.md)).

**Production-track wiring:** T-CONTENT-ART, T-CONTENT-NARRATIVE, T-LOCALIZATION, and T-LIVEOPS run alongside the BP spine but only finalize at BP12. They begin placeholder generation at BP3+ so the gameplay spine isn't blocked on art/audio/copy/legal.

See [`docs/plan/spec/prototype-roadmap.md`](docs/plan/spec/prototype-roadmap.md) for the full BP table, the **Design-Completeness Map** (every product surface mapped to its owning BP+milestone so you can verify "yes, by BP12 this is a complete game"), the milestone-map gap fills, the done-criteria summary, the kickoff smoke commands, and the inter-milestone bridge contracts.

## Design-Completeness Promise

By end of BP12 the game is a complete releasable candidate — every product surface a Steam buyer touches is functional, integrates with replay/save/server/modding/accessibility/captions, and is reachable from `cfctl`. Final balance tuning, marketing timing, and explicit post-launch expansions are the only work remaining. Concretely, **BP12 ships:**

| Surface | What It Means By BP12 |
|---|---|
| **Playable game** | Title → menu → settings → tutorial/labs → mission select/briefing → playable mission → debrief/reward/save loop works without developer intervention. |
| **Core promise** | Bunker Defence + Breach Contract + command-core/base stakes + chassis/equipment + terrain/materials + atmospherics + full collision + AI teammates/enemies + replay + modding all integrated, not isolated demos. |
| **Content roster** | 70+ weapons + 44+ actors + 18+ vehicles + 60+ base objects + 8 factions + 30+ missions + 12 worlds × 3-5 biomes + 17 materials + 12 ores + 30+ music tracks + 400+ SFX. **Every entry FUNCTIONAL + AI-readable + replay-recorded + caption-bound + balance-fixtured + localized + hot-reloadable + mod-parity** (DR-045). |
| **UX/UI** | Title + main menu + pause + full settings tree + lobby + workbench + briefing + debrief + map + achievements + replay viewer + codex + photo mode + cosmetic locker + death cam + mod manager. All cfctl-parity. |
| **Accessibility** | DR-012 floor + DR-051 accessibility-plus presets (cognitive + motor + hearing + reading + sensory + 8 color-blind protocols + cinematic accessibility) functional. |
| **Multiplayer/server** | Dedicated server + LAN co-op + online co-op (community-hosted) + public PvP arenas + persistent MMO shards + admin/moderation basics validated. |
| **Modding** | First-class schema + Lua/Rhai script host + package builder + auto-update + auto-docs + AI-driven test runs + community ecosystem extensions. |
| **Endgame + retention** | 10 endgame modes + persistent veterans + bunker meta + cross-shard world events + anti-FOMO archive + intrinsic-only progression (DR-031 + DR-048). |
| **Narrative + localization** | ~80,000 words narrative bible + 8 faction archives + 24+ named NPCs + ~600 codex entries + 11 Tier-A languages + 8 Tier-B UI-only + mod-localization layer. |
| **Audio + AI-authored content** | 30+ adaptive music tracks + 400+ SFX + ACRE2-tier radio + Steam Audio-tier voice + diegetic-first mix; all generated through the AI-only DR-053 pipeline + usage-ledger. |
| **Release operations** | T-RELEASE GA at v1.0.0 + T-LIVEOPS telemetry/crash/bug-tool + legal/license ledger + platform packaging + code signing + support docs + sustainability/sunset posture. |

If a row above is still missing a core system at BP12 closure time, BP12 cannot close by relabeling it as polish. Either implement it, write a user-approved scope-change DR, or keep BP12 open. See [`docs/plan/spec/prototype-roadmap.md`](docs/plan/spec/prototype-roadmap.md) **Design-Completeness Map** for the full surface-to-milestone matrix.

---

## The Layered Simulation

Every system reads from one source of truth. Nothing is faked.

```
                 ┌────────────────────────────────────────────────────────┐
                 │  AI Doctrine  •  Mission Director  •  Replay  •  HUD   │
                 └────────────────────────────────────────────────────────┘
                                          ▲
        ┌─────────────────────────────────┴─────────────────────────────────┐
        │                                                                   │
        │  Equipment + Chassis (origin: human / android / robot)            │
        │  Body damage + Wound model + Module damage + Origin reactions     │
        │                                                                   │
        ├───────────────────────────────────────────────────────────────────┤
        │                                                                   │
        │  Stationeers-grade-or-better Atmospherics + Thermal Simulation    │
        │  • Real PV = nRT, R = 8314.46, per-gas moles + temperature        │
        │  • 10 launch gases + 6 liquid mixtures, expansion via material lab │
        │  • 6 deterministic combustion reactions with autoignition T       │
        │  • Gradual phase change with latent heat                          │
        │  • Pipe networks with pumps, valves, regulators, filtration       │
        │  • Door / vent / bullet-hole / blast-breach / pipe-rupture        │
        │    apertures with liquid/gas pressure jets and wind force         │
        │  • Heat transfer through materials, coolant loops, heaters,       │
        │    radiators, insulation, emergency venting, and thermal failure  │
        │  • Room atmospheres + airlock state machines + suit life-support  │
        │  • Per-planet ambient (Earth / Mars / Moon / Mimas / Europa /     │
        │    Vulcan / Venus) and modder-defined planets                     │
        │                                                                   │
        ├───────────────────────────────────────────────────────────────────┤
        │                                                                   │
        │  Systemic Materials (Noita-grade chunked CA kernel)               │
        │  17 launch materials + reaction table + density layering          │
        │                                                                   │
        ├───────────────────────────────────────────────────────────────────┤
        │                                                                   │
        │  Full Collision Physics (everything physical collides by default) │
        │  Limb / weapon / armor / chassis / projectile / terrain / debris  │
        │  CCD tiers + impulse-to-damage routing                            │
        │                                                                   │
        ├───────────────────────────────────────────────────────────────────┤
        │                                                                   │
        │  Universal Gravity Field (one source; sampled per-cell per-tick)  │
        │  Per-planet ambient + per-cell overrides (gravity wells, low-g    │
        │  labs, magnetic boots, damaged grav generators, reverse-g rooms)  │
        │  Reads through to ballistic drag + atmospheric stratification +   │
        │  material settling + actor falls + every dropped casing           │
        │                                                                   │
        └───────────────────────────────────────────────────────────────────┘
                                          ▲
                 ┌────────────────────────┴───────────────────────┐
                 │  Deterministic 60 Hz sim core (120 Hz path     │
                 │  validated; 128 Hz under evaluation)           │
                 └────────────────────────────────────────────────┘
```

Every layer emits replay events. Every cause chain is reproducible. Every AI agent reads the same data the player sees.

---

## Core Pillars

| Pillar | What It Means |
|---|---|
| **Real physics, end to end** | No arcade approximations. Stationeers-grade is the minimum bar: PV = nRT atmospheres, pressure apertures, liquid/gas jets, material heat transfer, universal gravity for everything, full collision by default, stoichiometric combustion, and gradual phase change. |
| **Origin-aware bodies** | Humans, androids, and robots have **structurally different reaction chains**. Robots take internal-shock damage, leak coolant, and downclock under heat. Androids breathe, bleed, and overclock per installed module. Humans concuss, eat, and need oxygen tanks. |
| **Animation-first bodies** | Actors do not slide as static pawns. Controlled locomotion is readable animation with physical weight; disrupted states become more physical. Jetpack, pressure, wind, gravity, recoil, limb damage, armor mass, and mech servos all change the body presentation without destroying responsiveness. |
| **AI as teammate and rival** | Bots are first-class. They reason, plan, panic, recover, and explain themselves through reason labels. The 8-criteria humanlike-AI bar is testable. An optional async LLM "mind" layer proposes doctrine without ever blocking the local AI. |
| **Replay determinism** | Same seed + same inputs = byte-identical event stream. Debug with replay scrubbing. Network with confidence. Audit AI behavior with cause chains. |
| **Server truth, rich clients** | Server owns authoritative state; clients own immediate feel and GPU-rich presentation. Prediction is allowed, divergence is not. Single player uses the same architecture in-process. |
| **AI-authored production** | Art, audio, captions, provenance, and regeneration metadata are pipeline artifacts, not side notes. No retained asset skips the ledger. |
| **Modding as a first-class promise** | Schema-first, Lua escape hatches where useful, workbench tooling. Add a gas, a reaction, an origin, a planet — all data rows. |
| **Multiplayer ladder** | Solo + LAN co-op + online co-op + community-hostable public PvP arenas + persistent MMO shards. Same `cf-server` binary, multi-mode. Anyone can host. |
| **Accessibility floor** | Captions, contrast, no-color-only UI, focus traversal, reduced motion, reduced shake, reduced flash, reduced G-Force blackout — all from Slice A onward. |
| **No-compromise performance defaults** | Performance-sensitive values are config-driven, never hardcoded. Steam Deck floor at 800p/60, 1080p/60 mid-tier, 4K/120 strong-desktop ceiling. |

---

## Inspirations And Credits

Corefall stands on the shoulders of an exceptional set of games that figured out parts of the genre we want to weave together. **None of the work here is a copy** — but each of these projects taught us something we built on, and they deserve explicit credit.

| Inspiration | What We Learned |
|---|---|
| **[Cortex Command](https://datarealms.com)** by Data Realms | The command-core / dropship / chassis / digging / pixel-actor fantasy. The tone of "every body is physical and damageable". The mod ecosystem grammar. The actor-status / wound-state / inventory-fallout triangle. Deep, deep love. |
| **[Noita](https://noitagame.com)** by Nolla Games | Per-pixel material simulation as a core feel pillar. Alchemy / reaction / emergence as a retention loop. Hidden chemistry that rewards experimentation. The replay-able cause-chain culture. |
| **[Stationeers](https://stationeers.com)** by RocketWerkz | The minimum bar for atmospherics feel: real ideal-gas-law atmospherics, specific heats, autoignition temperatures, combustion stoichiometry, pipe networks as first-class atmospheres, suit life-support with canister + filter + waste-tank slots, and per-planet ambient. Corefall aims beyond that with combat apertures, liquid jets, thermal engineering, and richer material coupling. |
| **[Barotrauma](https://barotraumagame.com)** by FakeFish + Undertow Games | Rooms-with-state architecture. Breach flooding. Crew dynamics where roles matter. Mission storytelling that emerges from system failure. |
| **[The Powder Toy](https://powdertoy.co.uk)** | Open-source falling-sand chemistry. The discipline of element-grammar reaction tables. Educational transparency. |
| **[OpenSoldat](https://opensoldat.org) / [Soldat](https://forums.soldat.pl)** | Side-view multiplayer combat feel. Movement nuance. Map mutability. Community-hosted server culture. |
| **[Liero](https://liero.be) / [OpenLieroX](https://openlierox.sourceforge.io)** | Short, intense, weapon-rich arena combat. The proof that small arenas + many extreme weapons + short rounds can produce decades of replay value. |
| **[Teardown](https://teardowngame.com)** by Tuxedo Labs | Tools that change the map become real tactics. Destruction is design. |
| **[Oxygen Not Included](https://klei.com/games/oxygen-not-included)** by Klei | Per-cell atmospheric simulation at habitat scale. Pressure / temperature / gas density storytelling. |
| **[Rain World](https://rainworld.net)** by Videocult | Behavioral creatures that don't need stat-bars to feel real. |

We **also** lean on the open Rust gamedev ecosystem: [Bevy](https://bevyengine.org), [wgpu](https://wgpu.rs), [Rapier / Avian](https://rapier.rs), [Tokio](https://tokio.rs), [serde](https://serde.rs), [BLAKE3](https://github.com/BLAKE3-team/BLAKE3), and many more. See [game/Cargo.toml](game/Cargo.toml) for the full dependency tree.

> [!important] Reuse posture
> No code, no assets, no sprites, no audio, no scripting from any of the inspiration games is copied into Corefall. Everything is implemented from chemistry/physics/game-design first principles plus public documentation (wikis, GDC talks, modding docs, public source where applicable). Provenance is logged in the canonical vault's [usage ledger](https://github.com/Madreag/corefall#research-vault) when any specific snippet of public documentation is quoted in spec/research notes.

---

## Tech Stack

| Layer | Tooling |
|---|---|
| Language | [Rust](https://www.rust-lang.org) edition 2021, MSRV/toolchain pinned to 1.95.0 |
| Engine | [Bevy](https://bevyengine.org) 0.18.1 + [wgpu](https://wgpu.rs) for 2D / GPU; custom core crates for sim |
| Physics | Custom collision + custom material kernel + Stationeers-grade-or-better atmospherics/thermal kernel + universal gravity field |
| Async | [Tokio](https://tokio.rs) for the JSON-RPC control plane and dedicated server |
| Networking (planned) | TBD between [Lightyear](https://github.com/cBournhonesque/lightyear) / [renet](https://github.com/lucaspoffo/renet) / [quinn](https://github.com/quinn-rs/quinn); decision deferred to M9/M10 |
| Modding host (planned) | [mlua](https://github.com/khvzak/mlua) (Lua) candidate; deferred to M5 |
| Determinism | [BLAKE3](https://github.com/BLAKE3-team/BLAKE3) for state checksums; [rand_xoshiro](https://docs.rs/rand_xoshiro) for seeded RNG |
| Schemas | [serde](https://serde.rs) + [schemars](https://github.com/GREsau/schemars) + JSON Schema validation in CI |
| Testing | `cargo test` matrix (Linux + macOS + Windows) + scripted E2E + run-bundle checker (Python `tools/prototype_run_check.py`) |
| Editor | [Visual Studio Code](https://code.visualstudio.com) with [rust-analyzer](https://rust-analyzer.github.io); [Helix](https://helix-editor.com) and [Zed](https://zed.dev) supported via per-project `.gitignore` |

---

## The Workspace

30 crates today (see [game/Cargo.toml](game/Cargo.toml)). Each crate carries its own `AGENTS.md` boundary contract. Crates marked **(real)** have shipped real implementations; the rest are stubs that will fill in at their owning milestone.

```text
game/crates/
├── cf-app                  # (real)  Bevy app shell + window + keyboard input + render bridge
├── cf-sim-core             # (real)  fixed-tick scheduler + RNG + checksum
├── cf-control              # (real)  JSON-RPC 2.0 control surface (cf-control engine + server)
├── cfctl                   # (real)  operator CLI (observe, run, scenario, settings, runbundle, system, act.player.*)
├── cf-replay               # (real)  run-bundle writer + event envelope
├── cf-actor                # (real)  actor records + control intent + sim step + projectile + status state machine
├── cf-equipment            # (real)  role records + rifle spec + tick-rate-independent timing
├── cf-physics              # (real)  kinematics + ground collision + jump + recoil; M5.5 swaps in DR-033 collision matrix
├── cf-terrain              # (real)  M1.5: BreachStrip + BreachWorld + try_dig with M2-compatible event payloads; M2 replaces with chunked terrain
├── cf-mission              # (real)  M1.5: Objective/MissionState/MissionView + objective state machine
├── cf-ai                   # (real)  M1.5: ReactiveGuard FSM + utility scoring + scripted aim-settle + DR-008 LEAN
├── cf-render-2d            # (real)  wgpu 2D pipeline + actor + breach + extraction-zone sprite systems
├── cf-capture              # (real)  T-CAPTURE: PNG readbacks at 10 Hz baseline + event keyframes; capture_manifest.json for the composer
├── cf-ui                   # (real)  comic-noir UI presentation (10-line HUD: STATUS / ITEM / HP / Reticle / OBJECTIVE / MISSION / ENEMY / BREACH / EVENT)
├── cf-e2e                  # (real)  M1.5: scripted end-to-end runner with auto-launch + --expect <key>=<value> assertions
├── cf-mod                  # (real)  content schema validator + manifest walker
├── cf-chassis              # stub    armor zones + modules + pilot binding (M5)
├── cf-material             # stub    systemic material kernel (M5.6 / DR-036)
├── cf-atmos                # stub    Stationeers-grade-or-better atmospherics + thermal kernel (M5.9 / DR-037)
├── cf-net                  # stub    client/server transport (M9)
├── cf-audio                # stub    sound + captions (M4..M7)
├── cf-save                 # stub    versioned .cfsave format (M5..M9)
├── cf-tools-editor         # stub    in-engine scenario / package / mod editors (M8)
├── cf-headless             # stub    CI-friendly headless runner
├── cf-bench                # stub    perf benchmark harness
├── cf-server               # stub    multi-mode dedicated server (M9: coop_room/pvp_arena/lan_room/mmo_shard/lobby_directory)
├── cf-server-ops           # stub    ops dashboards + observability (M9)
├── cf-server-persistence   # stub    MMO shard persistence (M12)
├── cf-server-anti-cheat    # stub    anti-cheat foundation (M11)
└── cf-server-admin         # stub    admin tooling (M9)
```

---

## Project Status

> [!warning] Pre-alpha
> Corefall is in active development. The repo is public so CI can run unrestricted (free GitHub Actions minutes for public repos), but the game is **not** ready to play yet — first friend-handoff release lands at BP3 closure (see [Releases](#releases)).

**Workspace stats (last update 2026-05-09 / commit `3fe8ac8`):** 253 tests passing across 29 crates; cargo fmt + clippy `-D warnings` clean; `bp_test_coverage bp2` reports CLEAN with 0 gaps; M2.5 LLM-graded verdict 7.86/10 PASS_WITH_FUTURE_POLISH (gameplay 9-10/10, visual layer future-owned by M4A).

**BP2 closure recap (the most recent BP to close):** Chunked deformable terrain (M2) replaced the M1.5 soft-breach strip; M2.5 fun-proof scenario shows the player surviving a 60-90s reactor defense as terrain is dug + debris fields form; cf-headless re-runs any run bundle's `events.jsonl` deterministically (M3A). Closed across PR [#11](https://github.com/Madreag/corefall/pull/11) (BP2 engineering) + PR [#12](https://github.com/Madreag/corefall/pull/12) (BP2 follow-up: source-truthful run bundles + AI-Agent-Primary Self-Test contract + LLM-graded test verdicts + per-BP test suite + AI-agent self-correcting loop) + PR [#13](https://github.com/Madreag/corefall/pull/13) (planning spine migration into `docs/plan/`) + PR [#14](https://github.com/Madreag/corefall/pull/14) (SemVer prerelease channel versioning).

**BP3 (active) ownership:** M3B Replay Viewer + Debrief, M4A Readability + ACC-A Floor, M5 Equipment/Chassis/Damage Grammar, **AND** the Double-Click Playability release engineering (`cf-app` opens a game window with no command-line args; release.yml produces `.dmg` / `.msi` / AppImage). The first friend-handoff `v0.3.0-prealpha` release ships at BP3 closure, alongside retroactive re-publication of `v0.1.0-prealpha` + `v0.2.0-prealpha` (deleted 2026-05-09 for failing the gate; see [Releases](#releases)).

| BP | Milestone | Status | What It Proves |
|---:|---|---|---|
| BP0 | **M0 — Engine Bootstrap** | ✅ **Closed** ([PR #1](https://github.com/Madreag/corefall/pull/1) merged) | 29-crate workspace, JSON-RPC control plane, cfctl, replay run-bundle writer, deterministic 60 Hz / 120 Hz sim, panic capture, CI matrix on Linux + macOS + Windows. |
| BP1 | **M1 — Actor Controller And Sim Core** | ✅ **Closed** ([PR #2](https://github.com/Madreag/corefall/pull/2) merged) | Single playable actor with movement, jump, aim, rifle fire, reload, status state machine, projectile flight, and damage routing — all through the fixed-tick sim. Seven `act.player.*` JSON-RPC methods route human + cfctl + AI input through one shared dispatch path. Tick-rate-independent rifle timing (10 RPS / 1.5 s reload identical at 60 Hz and 120 Hz). |
| BP1 | **M1.5 — Micro Breach Fun Slice** | ✅ **Closed** ([PR #5](https://github.com/Madreag/corefall/pull/5) merged) | 60-90 s win/loss scenario plays end-to-end via cfctl scripts driving the same dispatch path as the keyboard; reactive guard with DR-008 LEAN (jobs + utility + scripted hooks) fires deterministic seeded miss rolls; soft breach emits M2-compatible `terrain_carved` events; mission state machine emits `objective_*` + `mission_resolved`; cf-e2e asserts win=4/4 + loss=3/3 expectations against observe.once snapshots. |
| BP1 | **T-CAPTURE — Frame capture + grid composer** | ✅ **Shipped** ([PR #6](https://github.com/Madreag/corefall/pull/6) merged) | cf-capture crate + `capture_grid.py` composer + cf-e2e wiring. PNG readbacks at 10 Hz baseline + event-triggered keyframes. AI agents read `summary_grid.png` via the Read tool to validate motion + physics + effects without a human eyeballing every smoke run. T-CAPTURE evidence is mandatory at every BP closure gate from BP2 onward. |
| BP2 | **M2 — Pixel Terrain And Materials** | ✅ **Closed** ([PR #11](https://github.com/Madreag/corefall/pull/11) merged) | Deformable chunked terrain + 8-material launch set + GPU-assisted carving + material overlay + tool-validity feedback. Replaces M1.5 soft-breach strip without breaking replay consumers. |
| BP2 | **M2.5 — Micro Reactor Defense** | ✅ **Closed** ([PR #11](https://github.com/Madreag/corefall/pull/11) merged) | 60-90s defend-the-reactor scenario; chunked terrain-driven win/loss; cfctl-scripted; same Acceptance + Contract Integrity gates as M1.5; LLM-graded verdict 7.86/10 PASS_WITH_FUTURE_POLISH (gameplay 9-10/10, visual 3-6/10 future-owned by M4A). |
| BP2 | **M3A — Event Recorder Core + Headless Replay** | ✅ **Closed** ([PR #11](https://github.com/Madreag/corefall/pull/11) merged) | Deterministic event log + cf-headless replay verifier proves end-to-end determinism by re-running any run bundle's events.jsonl. M3B (Replay Viewer + Debrief) lands in BP3. |
| BP3 | **M3B — Replay Viewer + Debrief** | 🆕 V2 split | Replay viewer scrubbing, event filters, debrief summary, parent-chain cause view, and death recap. |
| BP3 | **M4A — Readability + ACC-A Floor** | 🆕 V2 split | Silhouette HUD, module strip, movement/stance readability, material overlay, and accessibility floor. |
| BP3 | M5 — Equipment, Chassis, And Damage Grammar | ⏳ Planned | Body graph, equipment sockets, armor coverage, limb consequences, per-origin chassis records, damage stages, wreck/eject/salvage. |
| BP4 | M5.5 — Full Collision Gauntlet | ⏳ Planned | DR-033 closure: full collision + projectile-projectile + CCD tiers + universal gravity field integration. |
| BP4 | **M5.5.5 — Micro Sabotage** | 🆕 V2 | Fun-proof interlude after M5.5: 60-90 s breach + sabotage scenario combining collision physics with the M1.5 breach pattern. |
| BP4 | M5.6 — Material Kernel | ⏳ Planned | DR-036 partial closure: chunked CA + reaction table + density layering. |
| BP4 | M5.7 — Hazard Package | ⏳ Planned | Acid + electricity + debris + ingestion + affliction layer. |
| BP4 | M5.8 — Origin Resource & Overclock Pass | ⏳ Planned | Per-origin reaction matrix runtime: humans concuss, androids battery-drain, robots overclock + leak coolant. G-Force vision blackout HUD. |
| BP5 | M5.9 — Atmospherics-Grade Kernel | ⏳ Planned | DR-037 closure: Stationeers-grade-or-better PV=nRT, gases/liquids, combustion, phase change, pipe networks, suit life-support, pressure apertures, jets, heat transfer, and universal-gravity ballistic drag. |
| BP5 | **M5.9.5 — Micro Pressure Hold** | 🆕 V2 | Fun-proof interlude after M5.9: 60-90 s hold a room while atmosphere is breached, gases mix, fires propagate, suits compensate. |
| BP5 | **M5.10 — Environmental Conditions Aggregation** | 🆕 V2 | 12-world catalog + astrography + `EnvironmentSignal` aggregator + 15-class hazard taxonomy + comms light-lag. |
| BP6 | M6 — AI Core And Trust Harness | ⏳ Planned | DR-022 8-criteria humanlike bar testable. |
| BP6 | M6.5 — LLM Mind Lab | ⏳ Planned | Async LLM mind layer; local AI never blocks; no API key required. |
| BP6 | M6.6 — AI Environmental Competence | ⏳ Planned | AI reads material, atmosphere, gravity, thermal, radiation, photic, EM, weather, and comms signals with reason labels. |
| BP7 | M7 — Mission Director And Breach Contract | ⏳ Planned | Proof mission and A-FEEL gate. |
| BP7 | M7.5 — Base Atmospherics | ⏳ Planned | Base modules wired into M5.9 kernel: pumps, vents, pressure doors, breach repair, heaters/coolers, radiators, coolant loops, emergency venting, and room-state mission objectives. |
| BP7 | **M7.7 — Day/Night, Weather & Dynamic Events** | 🆕 V2 | Weather + day-night kernel, AI weather doctrine, and dynamic event hooks. |
| BP7 | **M4B — Comic-Noir Polish** | 🆕 V2 split | Comic-noir mission cards, stylized event banners, slowdown overlay, and tactical-map polish. |
| BP8 | M8 — Scenario Editor And Mod Tools | ⏳ Planned | First-class in-engine editor at launch. |
| BP8 | M8.5 — Material Lab | ⏳ Planned | Material/reaction lab for promotions to launch set. |
| BP8 | **M8.6 — Mining And Extraction** | 🆕 V2 | Ore registry, deposits, sampling, drilling, extraction, refining, smelting, trade ledger, and AI miner doctrine. |
| BP9 | M9 — Dedicated Server App | ⏳ Planned | `cf-server` multi-mode binary; SERVER-001..016 acceptance suite begins. |
| BP9 | M10 — LAN Co-op | ⏳ Planned | Local 2-4 player co-op through `cf-server --mode lan_room`. |
| BP10 | M11 — Online Co-op (Self-Hosted Dedicated Servers) | ⏳ Planned | Community-hostable online co-op. |
| BP10 | **M9.5 — Voice + Radio Sim** | 🆕 V2 | ACRE2-tier radio + Steam Audio-tier voice through atmospheric medium; captions mandatory. |
| BP11 | M12 — Public PvP Arenas + Persistent MMO Shards | ⏳ Planned | DR-035 MMO-001..012 readiness gate. |
| BP12 | T-CONTENT-ART / T-CONTENT-NARRATIVE / T-LOCALIZATION / T-LIVEOPS finalization | ⏳ Planned | All four production tracks reach launch GA. |

---

## Planning Spine + Research Vault

Corefall splits its plan-of-record across two locations:

### Planning spine — inside this repo at [`docs/plan/`](docs/plan/)

The **implementation-gating** planning layer lives in this repo so every PR that changes a roadmap row, checklist row, DR, milestone enhancement spec, or other gating contract is reviewed by Bugbot + Devin alongside the implementation that depends on it. Atomic plan + code PRs.

- **Decision records** (DR-001 through DR-057, plus future activation gates) — every major direction choice with pros, cons, evidence, revisit triggers. Lives at [`docs/plan/decisions/`](docs/plan/decisions/).
- **80 spec pages** for product promise, body damage, chassis/armor/mechs/origins, equipment/loadout, Stationeers-grade-or-better atmospherics & chemistry, thermal engineering, gravity & ballistics, AI, replay, mission director, full collision physics, accessibility-plus, localization, AI asset/audio production, modding, networking, launch operations, and more. Lives at [`docs/plan/spec/`](docs/plan/spec/).
- **Production roadmap** covering M0 through launch, side tracks, CLI/control contracts, DR-056 universal enhancement gates, and per-milestone Steam Deck/network/replay/accessibility/modding/testability budgets. Lives at [`docs/plan/spec/prototype-roadmap.md`](docs/plan/spec/prototype-roadmap.md).
- **Native implementation backlog** + **feature completion checklist** + **milestone enhancement spec** + **AI-coder reading list** + **ai-control-observability-layer** + **authoritative-game-spec** + **prototype-run-bundle-schema** + **decision-tracker dashboard** + **research-readiness dashboard** — all under `docs/plan/spec/`, `docs/plan/dashboards/`, and `docs/plan/references/`.
- **BP closure notes** at [`docs/plan/prototypes/build-point-bp*.md`](docs/plan/prototypes/) — per-BP narrative + evidence trail.

Full file history is preserved via `git filter-repo` from the research vault (every commit before the migration is visible in `git blame` on each spine file).

### Research vault — outside this repo at `~/projects/cortex-command-repos-all/cortext_command_vault`

Long-form research that informs but does not gate implementation:

- **Comparable game audits** — local code audits of Cortex Command (CCCP), OpenSoldat, OpenLieroX, The Powder Toy, plus public-source / public-doc research on Noita, Stationeers, Barotrauma, Oxygen Not Included. Lives at `cortext_command_vault/comparables/`.
- **Research log** — chronological record of every research pass with source citations. Lives at `cortext_command_vault/research-log/`.
- **Per-milestone prototype evidence notes** (`prototypes/native-*.md`) — narrative log of what shipped at each milestone, distinct from the in-repo `docs/implementation-log/` which captures what changed in this repo at that milestone.
- **Narrative seeds**, **comparable repos** (CCCP, OpenSoldat, OpenLieroX), **license usage ledger**, **equipment schema seeds**, **glossary**, **strategy docs**, **systems brainstorms**, **game-overview docs** (`VAULT_PLAN.md`, `GAME_DESCRIPTION_FOR_FRIEND.md`).

The vault stays separate so it can survive engine changes, language changes, or fork events. It's also where exploratory research lives without polluting the implementation repo's PR review surface.

> [!note]
> The vault is currently a private workspace. If you want to contribute design research or comparable-game audits, open an issue here so we can route the conversation.

---

## Releases

Released artifacts live at [github.com/Madreag/corefall/releases](https://github.com/Madreag/corefall/releases). Every Build Point closure publishes a tagged cross-platform release per the **T-RELEASE** side track.

### Double-Click Playability Hard Gate

A non-technical friend receiving the release file MUST be able to:

1. **Double-click the file** → standard OS extract/install (no Terminal, no `brew install`, no command-line decompression).
2. **Double-click the resulting app** → a corefall game window opens (no `--scenario` flag, no PowerShell, no command-line args).

If either fails, the platform is omitted from the BP's release matrix; if no platform meets the gate, the BP **skips** its release tag entirely. Skipping is preferred over publishing an opaque archive. The next BP that lands the missing engineering recovers the skipped releases.

| Platform | Format | Friend's experience |
|---|---|---|
| **macOS** | `.dmg` containing `Corefall.app` | Mount the `.dmg`. Drag `Corefall.app` to Applications. Double-click → game window. |
| **Windows** | `.msi` installer OR `.zip` with `Corefall.exe` (default-args launcher) | Run the `.msi` (Start Menu shortcut), or unzip + double-click `Corefall.exe` → game window. |
| **Linux** | AppImage (single double-click executable) | `chmod +x Corefall.AppImage` + double-click → game window. |

Code signing (Apple notarization + Windows Authenticode) activates at BP10+ via T-LIVEOPS pre-launch wiring; through BP9, expect a one-time platform warning ("right-click → Open" on macOS; "More info → Run anyway" on Windows).

### Versioning

SemVer prerelease channels — the channel suffix in the tag carries the quality signal so external observers (Steam buyers, contributors, package managers) can read it without consulting the BP table:

| Channel | Tag form | BPs |
|---|---|---|
| **prealpha** | `v0.<N>.0-prealpha` | BP0..BP3 (engine + first fun slices; major systems still missing) |
| **alpha** | `v0.<N>.0-alpha` | BP4..BP6 (full collision + atmospherics + AI combat) |
| **beta** | `v0.<N>.0-beta` | BP7..BP9 (mission director + creator alpha + server/LAN) |
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

# Smoke runs (M0 blank + M1 actor range + M1.5 micro breach)
cargo run -p cfctl -- observe --once --scenario m0_blank
cargo run -p cfctl -- run --scenario m0_blank --ticks 300 --tick-rate-hz 60 --paced --write-run-bundle
cargo run -p cf-app -- --scenario m0_blank --headless-smoke --run-seconds 5 --write-run-bundle

# Play the M1 actor range (windowed): WASD = move, Space = jump, arrows = aim,
# Enter / J = fire, R = reload, L = reset, 1-4 = inventory slot, Esc = quit.
cargo run -p cf-app -- --scenario m1_actor_range

# Drive the M1 scenario through cfctl (same dispatch path the keyboard uses)
cargo run -p cfctl -- run --scenario m1_actor_range --ticks 300 --tick-rate-hz 60 --paced --write-run-bundle

# Play the M1.5 Micro Breach Fun Slice (windowed; same controls + KeyG to dig)
cargo run -p cf-app -- --scenario micro_breach

# Replay the win or loss script through cf-e2e (auto-launches cf-app + asserts expectations)
cargo run -p cfctl -- script run scripts/cfctl/micro_breach_win.cfctl.json --write-run-bundle
cargo run -p cfctl -- script run scripts/cfctl/micro_breach_loss.cfctl.json --write-run-bundle

# Validate any run bundle
python3 tools/prototype_run_check.py ../prototype_runs/native/m0_*
python3 tools/prototype_run_check.py ../prototype_runs/native/m1_*
python3 tools/prototype_run_check.py ../prototype_runs/native/m1.5_*
```

### CLI Reference

`cfctl` is the operator + AI control client. The full surface is documented in the canonical vault roadmap (CLI Reference section); the currently-shipped subset is:

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
| `cfctl act player-fire [--pressed true\|false]` | M1 | Edge-triggered rifle fire (release is a no-op for M1's single-press rifle). |
| `cfctl act player-reload` | M1 | Begin reload (1.5 s real time at any tick rate). |
| `cfctl act player-select-item --slot <0..3>` | M1 | Switch inventory slot. |
| `cfctl act player-reset` | M1 | Respawn at scenario position with full HP / ammo / slot 0. |
| `cfctl act player-dig [--target <breach_id>]` | M1.5 | Edge-triggered terrain dig. With no target, picks the nearest in-range breach strip; rejects `out_of_range` / `material_metal_nohook` / `already_broken` / `unknown_target`. |
| `cfctl script run <path>` | M1 | Replay a `.cfctl.json` script (auto-launches `cf-app` with the right scenario, polls until ticks advance between commands). |
| `cf-app --capture-grid --capture-frames-hz 10` | T-CAPTURE | Emit PNG frame readbacks at 10 Hz baseline (configurable) + event-triggered keyframes into `<run_bundle>/captures/`. Add `--no-capture-events` to suppress keyframes; `--headless-capture` for the (scope-limited) offscreen-RenderTarget path. |
| `python3 game/tools/capture_grid.py <run_dir>` | T-CAPTURE | Compose `frame_*.png` into 8×8 `grid_NNN.png` + `summary_grid.png` with tick + event-label overlays. Requires Pillow. |
| `cf-e2e --script <path> --capture-grid --expect capture.summary_grid.non_blank_ratio>=0.95` | T-CAPTURE | One-shot: launch cf-app windowed, replay script, compose grids, assert. New `key>=value` and `key<=value` operators on `--expect` for numeric thresholds. |

**Run the M1.5 fun slice:**

```bash
# Win path (player breaches outer wall, neutralizes guard 2, reaches extraction zone in ~430 ticks)
cargo run -p cfctl -- script run scripts/cfctl/micro_breach_win.cfctl.json --write-run-bundle

# Loss path (player dies at guard 2 in ~1015 ticks)
cargo run -p cfctl -- script run scripts/cfctl/micro_breach_loss.cfctl.json --write-run-bundle

# Or drive cf-app windowed and use KeyG to dig:
cargo run -p cf-app -- --scenario micro_breach
# WASD = move, Space = jump, arrows = aim, Enter/J = fire, R = reload, G = dig, L = reset, 1-4 = inventory slot, Esc = quit.

# Run the cf-e2e harness (auto-launches cf-app, replays script, asserts expectations)
cargo run -p cf-e2e -- --script scripts/cfctl/micro_breach_win.cfctl.json \
    --expect mission.result=won \
    --expect "objective.reach_extraction=Completed" \
    --expect breach.outer_wall.broken=true \
    --expect "enemy.guard_2.state=Dead"
```

**Run the M1.5 fun slice:**

```bash
# Win path (player breaches outer wall, neutralizes guard 2, reaches extraction zone in ~430 ticks)
cargo run -p cfctl -- script run scripts/cfctl/micro_breach_win.cfctl.json --write-run-bundle

# Loss path (player dies at guard 2 in ~1015 ticks)
cargo run -p cfctl -- script run scripts/cfctl/micro_breach_loss.cfctl.json --write-run-bundle

# Or drive cf-app windowed and use KeyG to dig:
cargo run -p cf-app -- --scenario micro_breach
# WASD = move, Space = jump, arrows = aim, Enter/J = fire, R = reload, G = dig, L = reset, 1-4 = inventory slot, Esc = quit.

# Run the cf-e2e harness (auto-launches cf-app, replays script, asserts expectations)
cargo run -p cf-e2e -- --script scripts/cfctl/micro_breach_win.cfctl.json \
    --expect mission.result=won \
    --expect "objective.extract=Completed" \
    --expect breach.outer_wall.broken=true \
    --expect "enemy.2.state=Dead"
```

Post-M5+ CLI extensions (atmospherics, materials, gravity, ballistics, origin-state, suit, pipe-network, room) are documented in [the canonical roadmap](https://github.com/Madreag/corefall#research-vault).

---

## CI

GitHub Actions runs on every push and PR:

- `cargo fmt --all -- --check` (with `.gitattributes` locking LF line endings cross-OS)
- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace` (**253 tests passing** as of BP2 closure: workspace-wide coverage including NaN/Inf guards, dwell-pause off-by-one regressions, `--headless-smoke` + `--capture-grid` rejection, M2.5 + M3A scenario tests, cf-headless replay-determinism harness, and 8 channel-aware install-section tests)
- `cargo build --release`
- Dependency drift report on the Linux leg (`tools/dependency_drift_report.py`) for direct registry deps and transitive duplicate review
- `cf-mod validate content/` (validates M0 + M1 + M1.5 scenario manifests)
- Schema drift check (`dump_schemas --check`) against 26 static schemas under `crates/cf-control/schemas/v1/` (M1.5 added `act_player_dig_params.schema.json`)
- `cfctl observe smoke` + `cfctl run smoke` (60 Hz + 120 Hz)
- `cf-app headless run-seconds 5`
- M1.5 cf-e2e win + loss script smoke (auto-launches `cf-app --headless-smoke --control-api`, replays scripts, asserts every `--expect <key>=<value>`)
- Validate every produced run bundle through `tools/prototype_run_check.py`
- Enforce repo-root `prototype_runs/` path (M0.4-F7 guard)

Matrix: Linux + macOS + Windows.

The dependency drift report is intentionally advisory. It flags direct dependencies that have newer registry releases and prints a capped `cargo tree -d` sample, but it does not fail CI by default because duplicate transitive versions can be expected while upstream Bevy/wgpu/windows crates migrate. Use `--deny-outdated` only for explicit upgrade sweeps and `--duplicate-line-limit 0` when you need the full duplicate tree.

---

## AI Code Review

This repo uses **Cursor Bugbot** as a GitHub App for advisory PR review. Bugbot's loop runs three iterations per push, and autofix commits are authored as `Cursor Agent <cursoragent@cursor.com>`. We treat Bugbot's findings and autofixes as **advisory**, not authoritative — every Cursor Agent commit is audited against the actual source before merge, and false positives are reverted via `git revert` (not force-push) so the audit trail stays intact. See [`AGENTS.md` § Cursor Bugbot Loop](AGENTS.md) for the full protocol.

The repo also ships a project-local Claude Code review skill at [`.claude/skills/corefall-review/`](https://github.com/Madreag/corefall/tree/main/.claude/skills/corefall-review) that runs a deeper review pass (diff review, full affected-code review, contract gap review, edge-case hunt, test audit, determinism / replay review, security, performance, `cfctl` observability, vault coherence, synthesis judge). Invoke via `/corefall-review <milestone-or-range>`.

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
> Inspiration credits and a usage ledger for any externally-derived material are tracked in the canonical vault's [`references/usage-ledger.md`](https://github.com/Madreag/corefall#research-vault). No code, assets, sprites, or audio from the inspiration games is copied into Corefall.

---

## Acknowledgements

Built on Rust. Built on Bevy. Inspired by Cortex Command, Noita, Stationeers, Barotrauma, The Powder Toy, OpenSoldat, Liero/OpenLieroX, Teardown, Oxygen Not Included, and Rain World. Made possible by every open-source maintainer who took the time to write a wiki, publish a GDC talk, push public source, or answer a Steam discussion thread at 2 AM.

---

<div align="center">

**[Project status](#project-status) · [Build Points](#build-points) · [Inspirations](#inspirations-and-credits) · [Tech stack](#tech-stack) · [Getting started](#getting-started) · [License](#license)**

*One field for gravity. One kernel for atmospheres. One source of truth for everything.*

</div>
