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

[![Status](https://img.shields.io/badge/status-prealpha-orange?style=flat-square)](#project-status)
[![Milestone](https://img.shields.io/badge/milestone-M5%20done%20%2F%20M6%20next-2EA043?style=flat-square)](#roadmap)
[![Tests](https://img.shields.io/badge/tests-677%20passing-2EA043?style=flat-square)](#ci)
[![Specs](https://img.shields.io/badge/milestones-6%20done%20%2F%2070%20planned-blueviolet?style=flat-square)](#roadmap)
[![Releases](https://img.shields.io/github/v/release/Madreag/corefall?include_prereleases&sort=semver&style=flat-square&label=release)](https://github.com/Madreag/corefall/releases)

> [!important]
> **Where we are:** `M5` (Deep Damage Event Surface Lock) closed after a two-pass audit-driven hardening cycle — 74 deep-damage event schemas locked at v0.1 + M5-A1 (17 findings) + M5-A2 (26 findings) shipped, plus `audio.event_requested` + `combat.melee_hit_mo` + `combat.explosive_hit_mo` + `snapshot.snapshot_shield` + `snapshot.snapshot_thermal`. M3 (Pixel Terrain + Materials), M4 (Event Recorder Core), and M4A (Asset Ledger Infrastructure) all closed before M5. 677 tests pass workspace-wide; `cf-mod validate cf-replay/schemas/` reports 134/134 pass. Next up: `M6` (Actor Depth + Equipment + Sound + Squad). Full ordered milestone table in [Roadmap](#roadmap); shipped-code detail in [Project status](#project-status); release policy in [Releases](#releases).
>
> **Where to play:** today, build from source via [Getting started](#getting-started). First friend-handoff release (`.dmg` / `.msi` / AppImage — double-click to play, no Terminal) lands once `M11` (Readability + ACC-A Floor) + release engineering close per the [Double-Click Playability Hard Gate](#releases).

**[Pillars](#headline-systems) · [Roadmap](#roadmap) · [Content](#launch-content-roster) · [Inspirations](#inspirations-and-credits) · [AWAW Layer](#awaw-inspired-grand-strategy-layer) · [Project status](#project-status) · [Releases](#releases) · [Getting started](#getting-started)**

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
| **PvE Survival mode + Inter-Planet Transport** | Terraria/Stationeers/Minecraft/Cortex hybrid solo + 2-8 player coop. **3 launch survival worlds** (Earth + Mars + Mimas; 9 more post-launch unlock). 7-step procgen pass (topology → biomes → ore → hazards → structures → AI raiders → validation). Survival mechanics per race (hunger / thirst / sleep / sanity / temperature). **3 inter-planet transport modes**: dropship (T1; 8-phase flight + 6 risk events) / multi-stage rocket (T2; 5-stage Kerbal-style) / paired teleporters (T3). Orbital stations + asteroid mining colonies. 5 PvE endgame bosses (Hollow King / Frozen Heart / Crimson Tide / Eclipse Walker / Last Star). 12 dynamic world events. Acclimatization mechanic (chitin fast; humans slow). Race-specific tech tree branches. **Owned by M11.5 (mode + procgen) + M11.6 (transport + stations) + M11.7 (bosses + events).** |
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

## Roadmap

> [!tip]
> **Where we are (2026-05-13):** just closed `M5` (Deep Damage Event Surface Lock) plus the M5-A1 + M5-A2 hardening passes that close 43 audit findings across 7 parallel-worker review reports. M5 ships 74 deep-damage event schemas + `audio.event_requested` + `combat.melee_hit_mo` + `combat.explosive_hit_mo` + `snapshot.snapshot_shield` + `snapshot.snapshot_thermal`. M4 (Event Recorder Core) + M4A (Asset Ledger Infrastructure) closed before M5. Next up: `M6` (Actor Depth + Equipment + Sound + Squad). One ordered table below — read top to bottom. Spec files in [`specs/active/`](specs/active/) · closed specs in [`specs/done/`](specs/done/).

**Legend:** ✅ done · 🔄 in progress · ⏳ planned · 🚀 launch GA
**Workspace:** 33 crates · 677 tests · 57 closed/directional DRs · 73 milestones across 13 Build Points (BP0..BP12).

| # | ⬤ | BP | Milestone | What it ships |
|---|:---:|---|---|---|
| **M1** | ✅ | BP1 | Actor Controller + Sim Core | Playable actor (WASD / jump / aim / fire / reload / dig) · 5-state body machine · 9 JSON-RPC methods · tick-rate-independent timing |
| **M2** | ✅ | BP1 | Micro Breach Fun Slice | 60-90s win/loss · ReactiveGuard FSM · 3 difficulty presets · cfctl-scriptable |
| **M3** | ✅ | BP2 | Pixel Terrain + Materials | 256×256 chunked deformable terrain · 8 launch materials · DR-007 9-flag affordance taxonomy · CPU-deterministic carving · per-pixel integrity · penetration formula · **anchor RPC (`act.player.anchor`)** · **per-tick coalesced `terrain.terrain_dirty_region_batch` with ≤25-rect budget + `unupdated_areas` + forced-refresh signal** |
| **M4** | ✅ | BP2 | Event Recorder Core | 38 event categories · `cf-headless` replay verifier · deterministic event log · cosmetic-flag backpressure (DR-052) · per-tick blake3 sim_checksum |
| **M4A** | ✅ | BP3 | Asset Ledger Infrastructure | `cf-asset-ledger` crate · JSONL append-only ledger · 17 asset categories · 6 production tiers · regen + verify CLI · deterministic freeze + supersede chain |
| **M5** | ✅ | BP2 | Deep Damage Event Surface Lock | 74 deep-damage event schemas across 13 families (armor / internal / concussion / fluid / origin / hazard / atmos / shield / environment / thermal / combat hit_mo) + `audio.event_requested` + `combat.melee_hit_mo` + `combat.explosive_hit_mo` + `snapshot.snapshot_shield` + `snapshot.snapshot_thermal` · `cf-mod validate` extended to walk schemas dir; M5-A1 + M5-A2 hardening passes closed 43 audit findings |
| **M6** | ⏳ | BP2 | Actor Depth + Equipment + Sound + Squad | 36 actions · 6 weapons · 4 grenades · 8-slot inventory · side-view facing · 1 friendly bot + 4 squad commands |
| **M7** | ⏳ | BP2 | AI Archetypes + Mission Director | 5 archetypes (Rifleman / Sniper / Assault / Engineer / Spotter) · 20+ traits · 3 doctrines · 3-faction matrix · multi-objective DAG · 4-phase pacing |
| **M8** | ⏳ | BP2 | UX + Camera + Debug + L10n + Accessibility | 10+ HUD widgets · pie menu · 7 debug overlays · 4 color-blind modes · photo mode · replay scrubber · killcam |
| **M9** | ⏳ | BP2 | Micro Reactor Defense Fun Slice | 60-90s defend scenario · 5-tier terrain HP · 3-layer reactor armor · trench gameplay |
| **M9A** | ⏳ | BP4 | Tier 1 SVG Asset Pipeline | 5000+ placeholders · Python + cairo-svg + LLM-prompted SVG · 8 faction style.json · per-origin palettes |
| **M10** | ⏳ | BP3 | Replay Viewer + Debrief | Bundle viewer · cause-chain walker · 18-section debrief markdown · plain-language event template rendering |
| **M11** | ⏳ | BP3 | Readability + ACC-A Floor | 12 HUD nodes / 7 zones · 200% scale · high contrast · 5-tier integrity colors · War Thunder angle widget |
| **M11A** | ⏳ | BP4 | Shell UI Foundation | Title · main menu · pause · save-load · settings tree · credits · loading screens |
| **M12** | ⏳ | BP4 | Comic-Noir Aesthetic + Juice | Hand-drawn ink-line UI · 12 mission comic panels · death-recap-as-graphic-novel · juice rules per DR-055 |
| **M12A** | ⏳ | BP4 | Tier 1 Audio Pipeline | 1200+ SFX via Stable Audio Open / AudioCraft · caption metadata |
| **M13** | ⏳ | BP3 | Equipment + Chassis + Damage Grammar | 3 chassis archetypes · 15-zone body graph · layered armor · module state machine · pilot eject · save/load · **brain-hop API** |
| **M14** | ⏳ | BP4 | Full Collision + Impulse Routing | Universal gravity field · projectile-projectile CCD · War Thunder penetration ray · HE / HEAT / APFSDS · spalling |
| **M15** | ⏳ | BP4 | Active Material Kernel | Noita-grade per-pixel CA · 50+ materials · 30+ reactions · alchemy · flasks · GPU compute |
| **M16** | ⏳ | BP4 | Hazard Package + Afflictions | 18 afflictions · 6 STALKER anomalies · 20+ artifacts · swimming + underwater combat |
| **M17** | ⏳ | BP4 | Origin Reaction + Resource Model | **No-HP-bar canonical** · blood / oil / power / caloric / bio-fluid per origin · G-Force vision blackout · vacuum scenarios |
| **M18** | ⏳ | BP4 | Micro Sabotage Fun Slice | 60-90s sabotage integrating collision + materials + hazards + origin (chemistry / physics / stealth / origin paths) |
| **M18A** | ⏳ | BP4 | Animation Production Tier 1 | 1100+ frame strips (walk / hit reactions / death) via AnimateDiff |
| **M19** | ⏳ | BP5 | Atmospherics-Grade Kernel | PV=nRT · 10 gases · 6 combustion reactions · phase change · pipe networks · 6 launch worlds (Earth / Mars / Moon / Phobos / Mimas / Vulcan) |
| **M20** | ⏳ | BP5 | EnvironmentSignal Aggregator | Per-tick per-actor bundle · 11 slices (atmospheric / gravitational / thermal / radiation / photic / EM / weather / water / acoustic / day-night / comms) |
| **M21** | ⏳ | BP5 | Micro Pressure Hold Fun Slice | 60-90s hold-the-room while atmosphere is breached · gases mix · fires propagate · suits compensate |
| **M22** | ⏳ | BP6 | AI Pathfinding + Collision Avoidance | Hierarchical A* (tile / chunk / region) · dynamic re-pathing on terrain dirty · per-team path costs · stuck recovery |
| **M23** | ⏳ | BP6 | AI MIND Layer | Async LLM mind never blocks local AI · per-actor 5s ticks · doctrine proposals · sandbox safety · no API key required |
| **M24** | ⏳ | BP6 | AI Environmental Competence | 8-test AI-MAT suite · bots wear O2 in vacuum · retreat from radiation · downclock under heat · reason labels |
| **M24A** | ⏳ | BP6 | VFX Tier 1 | 600+ particle configs · 80+ textures (impact / spark / explosion / debris) |
| **M25** | ⏳ | BP7 | Campaign + Base + Commander Spine | 30+ missions · 5 storytellers (Cassandra / Phoebe / Randy / Ironman / Sandbox) · buy menu · 8 stratagems · **persistent AI commander rivalry** |
| **M25A** | ⏳ | BP7 | Narrative + Codex Production | 5k-word LLM-driven bible → 80k words · 600 codex entries · 24 named NPCs |
| **M26** | ⏳ | BP7 | Factions + NPCs + Narrative | 8 factions · relationship matrix · quartermasters · diplomacy · dialog trees · 8 quest templates · 4 pet companions |
| **M27** | ⏳ | BP7 | Loot + Progression + RPG | 5 rarity tiers · 30+ affixes · set bonuses · XP / level · 30+ achievements · 20+ perks · 10+ curses · Tetris inventory |
| **M27A** | ⏳ | BP7 | Player Game UI | Inventory Tetris · loadout · cosmetic locker · codex (600) · achievements (75) · tutorial menu · comparison tooltip |
| **M28** | ⏳ | BP7 | Base Atmospherics | Pumps · vents · pressure doors · breach repair · heaters · coolers · radiators · coolant loops · emergency venting |
| **M28A** | ⏳ | BP7 | Base Build Mode UX | Palette · ghost preview · rotation · room detection · blueprints · demolish / repair · multiplayer co-build |
| **M29** | ⏳ | BP7 | Power & Electrical Engineering | 8 generators · 6 cable tiers · 6 storage types · IC10 priority chips · brown-out cascades · **brain rooted / uprooted / embedded** with per-origin avatar buffs |
| **M29A** | ⏳ | BP7 | Power Grid + IC10 Editor UX | Factorio-style overlay · IC10 editor with breakpoints · per-generator dashboard · brown-out cascade viz |
| **M30** | ⏳ | BP7 | Basic Mining + Refining | 4 mining tools · 7 launch ores · 3 worlds ore distribution · AI-MINE-A-01..05 acceptance subset |
| **M31** | ⏳ | BP7 | Weather + Day/Night Kernel | 7 weather states · 24-hour cycle · AI behavior shifts · remaining 6 worlds (Deimos / Europa / Venus / Sol / Belt / Orbital — 12 total) |
| **M32** | ⏳ | BP7 | Crafting Tiers + Fabrication Chain | 5-tier ladder (T0..T4) · 150 launch recipes · fabrication chain · power-coupled · 30-node research tree |
| **M32A** | ⏳ | BP8 | Tier 2 ComfyUI Pipeline | 4500+ production assets · SDXL + Flux + AnimateDiff + ControlNet + per-faction LoRAs |
| **M32B** | ⏳ | BP8 | Crafting + Research + Salvage UX | 3-pane crafting (Stationeers / Terraria / Factorio hybrid) · research tree pan / zoom · 30+ mod slots · material flow Sankey |
| **M33** | ⏳ | BP8 | Modding Workbench | In-game scenario editor · Lua · IC10 chip editor · Steam Workshop · photo mode · replay browser · 8 tutorial labs |
| **M33A** | ⏳ | BP8 | Tutorial Lab Production | 8 modular labs · First Contract · FRE wizard · adaptive hints |
| **M34** | ⏳ | BP8 | Material Lab | DR-036 workbench · brushes · 17 materials · 13 lab-unlocked · recipe inspector · stamp save / load · AI puppet validation |
| **M35** | ⏳ | BP8 | Advanced Mining + Extraction | 9 mining tools · 12 ores · AI-MINE-A 8-test suite · server-authoritative ledger |
| **M36** | ⏳ | BP9 | Dedicated Server + Determinism Islands | 5 server modes · `cf-server-ops` / `persistence` / `anti-cheat` / `admin` · 4 mod trust tiers · SERVER-001..016 suite |
| **M36A** | ⏳ | BP9 | Platform Integration | Steam SDK · Discord rich presence · EOS adapter · Workshop · Cloud · Achievements bridge |
| **M36B** | ⏳ | BP9 | Telemetry + Crash + Bug Report | Opt-in privacy · per-shard analytics · in-game bug-submit |
| **M37** | ⏳ | BP9 | Voice + Radio + Comms | ACRE2-tier radio (distance / occlusion / frequency / antenna) · EMP · vacuum no_voice · AI chatter · captions |
| **M37A** | ⏳ | BP9 | Tier 2 Audio + Voice + Music | 7000+ voice clips (ElevenLabs / Bark / XTTS) · 30+ music tracks (MusicGen) · adaptive music engine |
| **M38** | ⏳ | BP9 | Server Config + Admin CLI + Settings | 200+ tunables · 7-tier hierarchy · 20+ admin commands · auto-gen settings UI · audit log |
| **M38A** | ⏳ | BP9 | Localization (19 languages) | 11 Tier-A + 8 Tier-B · 380k+ translations via LLM auto-translation · ICU MessageFormat |
| **M39** | ⏳ | BP9 | Universal Schema Locks | Manifest at `cf-mod/manifest/all_schemas.ron` · ~120 locked schemas · one-shot conformance check · bump policy |
| **M40** | ⏳ | BP9 | LAN Co-op | 2-4 player co-op via `lan_room` · mDNS / UDP discovery · 7 squad roles · revive · death cam · mod hash sync |
| **M40A** | ⏳ | BP10 | Spectator + Streamer Polish | Replay-to-MP4 via FFmpeg · 10+ overlay themes · Twitch / YouTube / Discord webhook integration |
| **M40B** | ⏳ | BP10 | Online UX | Server browser · friends (Steam / Discord / in-game) · party invite · lobby · admin web panel · mod hash sync UI · voice chat UI |
| **M41** | ⏳ | BP10 | Online Co-op + Full Match Grammar | NAT punch-through · 10 endgame modes · persistent AI commander · **4+ squads × 7 role types × 12 commands** · War Thunder kill cam |
| **M42** | ⏳ | BP10 | Self-Hosted Server Deployment | 3 Docker tiers · systemd · launchd · Terraform · Ansible · Grafana / Prometheus / Loki · 15-min deploy target |
| **M43** | ⏳ | BP10 | PvE Survival Mode + Procgen | 1-8 player coop · 7-step procgen · 3 launch survival worlds · per-race difficulty matrix · acclimatization |
| **M43A** | ⏳ | BP10 | Map + Mission + Campaign UX | World map · solar system map (12 worlds) · mission select · campaign tree · briefing · travel planner |
| **M44** | ⏳ | BP10 | Inter-Planet Transport + Stations | 3 transport modes (dropship 8-phase / multi-stage rocket / paired teleporters) · orbital stations · 7 new vehicles |
| **M45** | ⏳ | BP10 | PvE Endgame Bosses + World Events | 5 named bosses (Hollow King / Frozen Heart / Crimson Tide / Eclipse Walker / Last Star) · 12 dynamic world events |
| **M45A** | ⏳ | BP10 | Cosmetic Production | 2200+ items · anti-pay-to-win audit (DR-031) |
| **M46** | ⏳ | BP11 | Upkeep Economy (Opt-In) | AWAW BRP drain · bankruptcy cascade Day 1 → 30 · rescue mechanisms · AI factions follow same rules |
| **M47** | ⏳ | BP11 | Strategy Phase + Goals (Opt-In) | AWAW Rule 8 turn sequence · 5 stances × 5 production × 3 logistics = 75 combos · 24 launch goals |
| **M48** | ⏳ | BP11 | Inter-Faction Intelligence (Opt-In) | AWAW codebreaking · spy rings · 8 covert ops · counter-intel · AI factions follow same rules · opt-in per server |
| **M48A** | ⏳ | BP12 | Tier 3 Polish | Top 50 Aseprite hand-polish · top 20 Spine rigs · FMOD / Kira mix · Steam Deck Verified |
| **M48B** | ⏳ | BP12 | Steam Store + Marketing | Capsule art · 12 screenshots · 6 trailer types · press kit · tag taxonomy |
| **M48C** | ⏳ | BP12 | Endgame + Workshop UX Polish | Debrief · replay browser · photo mode · mech bay · pilot / commander dossier · faction diplomacy · quest log · NPC dialog · hub · mod manager |
| **M49** | 🚀 | BP12 | Public PvP + Persistent MMO + Bunker Defence | **Launch GA** · public PvP arenas · MMO shards · **Bunker Defence flagship mode** · 4 launch shards · cross-shard events · `v1.0.0` |

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

> [!note]
> **Current status.** The diagram below shows the *target architecture* at full BP12 launch. As of `M3` close, the actor controller, equipment / chassis grammar, chunked terrain, mission state machine, replay / event recorder, and HUD are real. Atmospherics (PV=nRT), systemic materials (CA kernel), full collision physics, universal gravity, and multiplayer are planned systems with stub crates — they ship at `M14`+. The diagram is the design commitment; the [Roadmap](#roadmap) table shows what's real today.

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

> [!note]
> **Current status.** Pillars marked with *(planned)* have stub crates but no production implementation yet. They are design commitments, not shipping features. See [Roadmap](#roadmap) for the per-milestone status.

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

> [!important]
> **Reuse posture.** No code, no assets, no sprites, no audio, no scripting from any of the inspiration games is copied into Corefall. Everything is implemented from chemistry / physics / game-design first principles plus public documentation (wikis, GDC talks, modding docs, public source where applicable). Provenance is logged in the canonical vault's usage ledger when any specific snippet of public documentation is quoted in spec / research notes.

---

## AWAW-Inspired Grand-Strategy Layer

> [!note]
> Corefall ships a **fully opt-in grand-strategy layer** modeled on Bruce Harper's **A World at War** (Avalon Hill, 2001) — the deepest WWII grand-strategy wargame ever published. Server admins choose how much AWAW-derived depth to enable; defaults are OFF so vanilla Corefall plays as a tactical sandbox without forcing strategic accounting on every player. Spec sources: [`specs/COHERENCE-TIER-5.md`](specs/COHERENCE-TIER-5.md) + milestones `M32` / `M38` / `M43` / `M45` / `M46` / `M47` / `M48`.

### Why AWAW?

Every canonical grand-strategy game — A World at War, Hearts of Iron, Stellaris, Crusader Kings, Civilization, RimWorld, Oxygen Not Included — solves the same core tension: **the unit you can't afford to keep is worse than the one you never built**. Corefall's tactical sandbox loop (Cortex Command + Stationeers + Noita + ACRE2) creates per-mission strategy, but **no campaign-scale strategic decisions** about which assets to retire, which factions to ally with, or which research to invest in. AWAW's rule system — battle-tested across hundreds of hours of competitive play since 1993 (its predecessor *A World At War* / *Advanced Third Reich*) — is the canonical answer.

We pull eight AWAW rule families into Corefall's grand-strategy layer, all gated behind `server.ron::grand_strategy.awaw_rulesets.*` toggles. AI factions follow the same rules as human players, so every AWAW mechanic is exploitable in both directions.

### AWAW Rules Implemented

| AWAW Rule | Corefall Milestone | In-Game Effect |
|---|---|---|
| **Rule 8** — Strategic Campaign Turn Sequence | `M47` Strategy Phase + Goals | Per-cycle decision phase: Research → Diplomacy → DOW → Movement → Combat → Post-combat → Construction → Redeployment. 5 stances × 5 production focuses × 3 logistics priorities = 75 combos |
| **Rule 24.622** — Codebreaking combat modifier | `M48` Inter-Faction Intelligence | Codebreaking level 0-5 across 5 categories grants passive combat modifier ±1 to ±5 against the breached faction |
| **Rule 35.31** — BRP-Oil Coupling | `M38` Server config | Oil shortage cuts faction-wide growth rate 5% per missing fuel unit (per-tick economic drag) |
| **Rule 35.53** — BRP Deficits + Bankruptcy Cascade | `M46` Upkeep Economy | Cumulative deficit triggers Day 1 → 3 → 7 → 14 → 30 cascade: low morale → forced demobilization → building auto-shutdown → Resistance Level → faction collapse |
| **Rule 36** — Mobilization Phases | `M38` Server config | Civilian → military shift per year toggles industrial output multipliers |
| **Rule 37** — Industrial Center Evacuation | `M43` PvE Survival | Lose territory but keep IC via strategic redeployment; materials in transit go to enemy on intercept; scorched-earth option for unsalvageable ICs |
| **Rule 40** — BRP Grants | `M46` Upkeep Economy | Allied faction transfers BP to rescue a bankrupt ally (multiplayer co-op + cross-faction diplomacy) |
| **Rule 41.5** — Code-Name Research Secrecy | `M32` Crafting Tiers | Public dice rolls but **hidden project intent** — opponents see RP investment without knowing what's being researched. Decoy code names supported |
| **Rule 44-48** — Intelligence Subsystems | `M48` Inter-Faction Intelligence | **Four subsystems:** Codebreaking (passive combat modifier) + Spy Rings (placed in target factions; reveal BP/army/stance/goals + diplomatic die roll modifier) + Covert Operations (negate research / diplomacy / construction / supply / intel-steal / sabotage / assassinate / false-strategy; 1-2 per cycle depending on RP) + Counter-Intelligence (detect spies, block covert ops, reduce enemy codebreaking) |
| **Rule 46.411A** — Counter-Intel Reveal | `M48` Inter-Faction Intelligence | Detected spy rings become revealed and trigger diplomatic incident events |
| **Rule 48.5** — Code-Name Reveal | `M48` Inter-Faction Intelligence | Level-5 codebreaking exposes the underlying project a code name refers to (chains with Rule 41.5) |
| **Rule 49** — Diplomatic Results | `M45` PvE Endgame | Diplomatic outcomes: minor agreement / agreement / major agreement / alliance / pact-breaking / war declaration; per-faction reputation cascade |
| **Rule 49.21** — Secret DP Allocation | `M38` Server config | Faction DP allocation hidden from other factions until effect resolves |
| **Rule 49.31** — One-Third DP Limit | `M38` Server config | No faction can spend more than 1/3 of its DP on a single target per cycle (anti-runaway gating) |
| **Rule 49.4262** — Spy Ring Diplomatic Modifier | `M48` Inter-Faction Intelligence | Active spy ring grants ±1 modifier on diplomatic die rolls against target faction |
| **Rule 49.5** — Lesser Diplomatic Result Downgrade | `M38` Server config | When DP target faction has spy ring or counter-intel advantage, diplomatic results downgrade by one tier |
| **Rule 60** — Faction Resistance Levels | `M45` PvE Endgame | Graduated collapse: Level 0 (normal) → -1 (-20 BP cumulative) → -2 (-50) → -3 (-100). Resistance applies penalties to all faction activities; recovery via internal restructure or allied intervention |

### Opt-In Server Presets

The `M38` settings hierarchy ships four launch presets so server admins can pick their depth without writing config from scratch. Every AWAW rule is independently toggleable inside `grand_strategy.awaw_rulesets.*`.

| Preset | File | Upkeep Economy | AWAW Rulesets | Target audience |
|---|---|---|---|---|
| **Vanilla** | `server.ron.template-vanilla` | OFF | ALL OFF | Tactical sandbox players who want classic Corefall — Cortex Command + Stationeers + Noita with no strategic accounting |
| **Classic Upkeep** | `server.ron.template-classic-upkeep` | ON | mostly OFF (only Rule 35.31 BRP-Oil coupling) | RimWorld / ONI fans who want resource-pressure but not full grand-strategy decision phases |
| **AWAW-Lite** | `server.ron.template-awaw-lite` | ON | Rules 8 / 37 / 41.5 / 44-48 / 60 enabled | Players who want strategic decisions + intelligence + resistance + IC evacuation without full diplomatic-die-roll complexity |
| **AWAW-Full** | `server.ron.template-awaw-full` | ON | ALL rulesets ON | Grand-strategy veterans who want every AWAW rule active; PvE Survival hardcore + competitive MMO |

PvE Survival default = **Classic Upkeep**; PvP Arena default = **Vanilla** (combat-only); MMO shard default = configurable per shard.

### When AWAW Features Ship

The AWAW-inspired grand-strategy layer ships across **BP7 → BP11**. Foundational pieces (config toggle tree in `M38`; code-name secrecy in `M32`) ship before the heavy intelligence + upkeep + strategy phase layers, so by the time `M48` lands the player can flip on full AWAW mode end-to-end.

| Order | BP | Milestone | AWAW Rule(s) | What it adds |
|:---:|---|---|---|---|
| 1 | BP7 | `M32` Crafting Tiers | **Rule 41.5** | Code-name research secrecy (public dice, hidden project intent) |
| 2 | BP9 | `M38` Server Config | **Rule 8 / 35.31 / 36 / 49.21 / 49.31 / 49.5** | Config toggle tree + 4 launch presets (vanilla / classic-upkeep / awaw-lite / awaw-full) |
| 3 | BP10 | `M43` PvE Survival | **Rule 37** | Industrial Center Evacuation — strategic redeployment, scorched-earth option |
| 4 | BP10 | `M45` PvE Endgame | **Rule 49 / 60** | Diplomatic results + Faction Resistance Levels graduated collapse |
| 5 | BP11 | `M46` Upkeep Economy | **Rule 35.53 / 40** | BRP deficit cascade Day 1→30 + allied BP grants |
| 6 | BP11 | `M47` Strategy Phase | **Rule 8** | Per-cycle decision phase — 75 stance/production/logistics combos |
| 7 | BP11 | `M48` Intelligence | **Rule 44-48 / 24.622 / 46.411A / 48.5 / 49.4262** | Codebreaking + spy rings + 8 covert ops + counter-intel |

> [!tip]
> **For AWAW veterans:** Corefall is not a digital AWAW port — it's a tactical sandbox that adopts AWAW's strategic rule families as an opt-in metagame layer. The hex map, BRP economy, and unit roster are different (we're in a sci-fi setting with 12 worlds and 10 races, not 1939-1945 Europe), but the *decision shape* AWAW creates — bankruptcy pressure, intelligence-vs-counter-intelligence cat-and-mouse, IC-evacuation-or-scorch tradeoffs, resistance-level death spiral — all map directly into Corefall's faction system. You can lose a campaign because you over-invested in cosmetic loadouts and couldn't pay upkeep; you can win one because your level-5 codebreaking revealed an enemy's research focus three cycles before they completed it.

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

**33 crates today** (see [game/Cargo.toml](game/Cargo.toml)). Each crate carries its own `AGENTS.md` boundary contract. Crates marked **(real)** have shipped real implementations; the rest are stubs that will fill in at their owning milestone.

```text
game/crates/
├── cf-app                    # (real)  Bevy app shell + window + keyboard input + render bridge
├── cf-sim-core               # (real)  fixed-tick scheduler + RNG + checksum
├── cf-control                # (real)  JSON-RPC 2.0 control surface (cf-control engine + server)
├── cfctl                     # (real)  operator CLI (observe, run, scenario, settings, runbundle, system, act.player.*, act.chassis.*)
├── cf-replay                 # (real)  M4 recorder + 38-category event taxonomy + cosmetic backpressure (DR-052) + per-tick blake3 sim_checksum + M5 schema lock (134 schemas)
├── cf-asset-ledger           # (real)  M4A JSONL append-only ledger + 17 asset categories + 6 production tiers + supersede chain + deterministic freeze
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
├── cf-mod                    # (real)  content schema validator + manifest walker + material registry validator + M4A ledger CLI + M5 event-schema walker
├── cf-chassis                # (real)  3 launch chassis archetypes (infantry_v1 / powered_armor_v1 / light_mech_v1) + 15-zone body graph + layered armor + module state machine + pilot binding + eject + salvage
├── cf-save                   # (real)  SaveBlob v1 with actor + chassis + rifle serialization + blake3 checksum (M5 slice of T-SAVE)
├── cf-material               # (real)  systemic material kernel (M2 baseline; M5.6 extends to 50+ materials + reactions)
├── cf-headless               # (real)  CI-friendly headless runner (M4 replay verifier)
├── cf-tools-replay-viewer    # stub    M3B: bundle viewer + cause-chain walker + debrief markdown emitter + validate + summary
├── cf-environment            # stub    M20: EnvironmentSignal aggregator (atmospheric, gravitational, thermal, radiation, photic, EM, weather, water, acoustic, day/night, comms)
├── cf-atmos                  # stub    Stationeers-grade-or-better atmospherics + thermal kernel (M19)
├── cf-audio                  # stub    sound + captions (M4..M7); ACRE2-tier voice + radio at M9.5
├── cf-net                    # stub    client/server transport (M9; lightyear vs renet vs quinn locks at M9)
├── cf-tools-editor           # stub    in-engine scenario / package / mod editors + Material Lab (M8 + M8.5)
├── cf-bench                  # stub    perf benchmark harness
├── cf-server                 # stub    multi-mode dedicated server (M9: coop_room / pvp_arena / lan_room / mmo_shard / lobby_directory)
├── cf-server-ops             # stub    ops dashboards + observability + /health + /ready + Prometheus metrics + drain shutdown (M9)
├── cf-server-persistence     # stub    Per-shard resource ledger + snapshot store + event journal (M9 minimum; M12 fills full MMO)
├── cf-server-anti-cheat      # stub    Input validation + rate limits + 3 profiles (casual / competitive / tournament_strict) (M9 foundation; M11 extends)
└── cf-server-admin           # stub    Admin commands + capability gating + kick / save / restart / hot_load / ban (M9)
```

---

## Project Status

> [!warning]
> **Pre-alpha.** Corefall is in active development. The repo is public so CI can run unrestricted (free GitHub Actions minutes for public repos), but the game is **not** ready to play yet — first friend-handoff release lands when `M11` (Readability + ACC-A Floor) + release engineering land (see [Releases](#releases)).

**Workspace stats (2026-05-13 / commit `0951136`):** 33 crates · **677 tests passing** workspace-wide · cargo fmt + clippy `-D warnings` clean · M3 determinism CI matrix passes all 6 combinations · `cf-mod validate cf-replay/schemas/` reports **134/134** schemas pass.

**M5 closed (2026-05-13):** ships 74 deep-damage event schemas locked at the M4 v0.1 envelope across 13 families (armor / internal / concussion / internal_shock / fluid / origin / hazard / affliction / atmos / shield / environment / thermal + `combat.projectile_hit_mo` expanded payload). Two post-close audit-driven hardening passes by parallel worker teams closed 43 findings total:
- **M5-A1 (17 findings)**: bulk-rewrote all 74 schemas' `schema_version.const` from `"0.1"` → `"prototype-recorder-event.v0.1"` (matches `cf-replay/src/lib.rs::EVENT_SCHEMA_VERSION`); shipped `audio.event_requested` schema (M5 spec mandate); added `blinded` as 23rd affliction kind for M6 flash grenade; renamed `combat.projectile_hit_mo.payload.parent_event_id` → `parent_hit_event_id` (envelope-collision fix); locked Origin enum (Human / Android / Robot / PoweredOrganic / HeavyBiomech) via `oneOf` constraint; locked phase enums (gas / liquid / solid / supercritical / molten); locked `EnvironmentSignal` sub-struct (15-value HazardClass enum); tightened cosmetic flags to `const: true`; shipped `snapshot.snapshot_shield`. cf-mod gained envelope-shape conformance checks + payload `additionalProperties: false` rejection + envelope-version-dir regex `^v[0-9]+(_[0-9]+)?$`.
- **M5-A2 (26 findings)**: `armor.spalling.fragment_count` locked to 1..3; `hazard.spread` now requires `hazard_id` for M10 cause-chain; Pa/K/J floors added to atmos + thermal events; `armor.ricochet.ricochet_probability` capped at 1.0; `fluid.reservoir_warning/critical.level_pct` capped at 100; `concussion.ko_threshold_crossed.ko_duration_s` locked to 5..10; `internal_shock` dose ceiling 0..100; `applied_afflictions` array items enforce 23-affliction enum. cf-replay validator gained array-item enum support + nested-object property recursion. Shipped sibling `combat.melee_hit_mo` + `combat.explosive_hit_mo` schemas so M6 melee + grenade producers don't need new schemas mid-implementation. Shipped `snapshot.snapshot_thermal` placeholder.

**M4 closed (2026-05-13):** `cf-replay` recorder + 38-category event taxonomy + cosmetic-backpressure flag (DR-052) + per-tick blake3 `sim_checksum` + `first_divergence` event-type + 8-snapshot M9-firehose surface (snapshot_actor / snapshot_inventory / snapshot_chassis / snapshot_terrain_chunk / snapshot_terrain_summary + 5 placeholders for M5/M9/M17/M19/M20 ladder-up). `cf-headless` replay verifier + `cf-mod validate-bundle` walks `events.jsonl` and asserts every payload conforms to the per-event schemas under `cf-replay/schemas/event/`.

**M4A closed (2026-05-13):** `cf-asset-ledger` JSONL append-only ledger with 17 asset categories + 6 production tiers + supersede chain + deterministic freeze. `cf-mod ledger` CLI commands (add / list / show / verify / diff / regenerate / summary / compact / register-pack). 7-axis audit gaps closed (BLOCKERS + MAJORS).

**M3 re-closed (2026-05-13):** post-close audit found 4 Acceptance Criteria scenarios failing. Fixes shipped: (1) `act.player.anchor` JSON-RPC + `terrain.anchor_material_result` event emission; (2) per-tick coalesced `terrain.terrain_dirty_region_batch` via end-of-tick `flush_pending_dirty_batch` with deduplicated `source_event_ids[]`; (3) ≤25-rect coalescing budget cap via greedy AABB-union loop; (4) `unupdated_areas: u32` field on batch payload + new `terrain.forced_refresh_requested` event for M22 forward-compat.

**Up next:** `M6` — Actor Controller Depth + Equipment + Sound + Squad Slice. Closes the actor controller + equipment + inventory + sound + squad-of-two gap left after M1+M2+M3. Player-facing promise: "a single actor in a single scenario already feels like a modern tactical shooter." 36 actions · 6 weapons + 4 grenades + 4 melee + 7 tools · 8 active inventory slots + 3 reserved tank slots · centralized sound + perception kernel · 1 friendly bot + 4 squad commands · side-view facing. Spec: [`specs/active/M6.md`](specs/active/M6.md). AGENTS.md workflow: implementer reads `M6.md` + source under `cf-*` crates, audit-first gap-fills, commits per-scenario, then moves to `specs/done/M6.md` when every Gherkin acceptance scenario verdicts as `PASS` or `IMPLEMENTED`.

**Per-milestone status:** see [Roadmap](#roadmap) above for the full ordered table.

---

## Planning Spine + Research Vault

Corefall splits its plan-of-record across two locations:

### Planning spine — inside this repo at [`docs/plan/`](docs/plan/)

The **implementation-gating** planning layer lives in this repo so every PR that changes a roadmap row, checklist row, DR, milestone enhancement spec, or other gating contract is reviewed by Bugbot + Devin alongside the implementation that depends on it. Atomic plan + code PRs.

- **Decision records** (DR-001 through DR-057, plus future activation gates) — every major direction choice with pros, cons, evidence, revisit triggers. Lives at [`docs/plan/decisions/`](docs/plan/decisions/).
- **80 spec pages** for product promise, body damage, chassis/armor/mechs/origins, equipment/loadout, Stationeers-grade-or-better atmospherics & chemistry, thermal engineering, gravity & ballistics, AI, replay, mission director, full collision physics, accessibility-plus, localization, AI asset/audio production, modding, networking, launch operations, and more. Lives at [`docs/plan/spec/`](docs/plan/spec/).
- **Production roadmap** covering M0 through launch, side tracks, CLI/control contracts, DR-056 universal enhancement gates, and per-milestone Steam Deck/network/replay/accessibility/modding/testability budgets. Lives at [`docs/plan/spec/prototype-roadmap.md`](docs/plan/spec/prototype-roadmap.md).
- **67 active milestone specs** in [`specs/active/`](specs/active/) — `M6..M49` (43 core, dependency-ordered) plus 24 suffix-letter inserts (16 production-track + 8 UX/UI). The executable implementation contracts for the gameplay spine + asset/audio/narrative/localization pipelines + player-facing UI surfaces. Each is read-by-implementing-agent-only per AGENTS.md.
- **6 closed milestone specs** in [`specs/done/`](specs/done/) — `M1.md` (actor controller core), `M2.md` (micro breach fun slice), `M3.md` (pixel terrain + materials), `M4.md` (event recorder core), `M4A.md` (asset ledger infrastructure), `M5.md` (deep damage event surface lock). Kept for audit trail.
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
- `cargo test --workspace` (**677 tests passing** as of M5 + M5-A1 + M5-A2 close: workspace-wide coverage including NaN/Inf guards, dwell-pause off-by-one regressions, `--headless-smoke` + `--capture-grid` rejection, M2 + M2.5 + M3A + M5 scenario tests, cf-headless replay-determinism harness, channel-aware install-section tests, terrain material affordance + dirty-region coalescing, chassis layered armor + pilot eject lifecycle, save round-trip with chassis state, M4 cosmetic-flag backpressure + per-tick blake3 checksum, M4A asset-ledger v1 + supersede chain + deterministic freeze, M5 envelope-shape conformance + per-family happy-path + Origin enum + array-item enum + nested-object recursion)
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

**[Pillars](#headline-systems) · [Roadmap](#roadmap) · [Content](#launch-content-roster) · [Inspirations](#inspirations-and-credits) · [AWAW Layer](#awaw-inspired-grand-strategy-layer) · [Project status](#project-status) · [Releases](#releases) · [Getting started](#getting-started) · [License](#license)**

*One field for gravity. One kernel for atmospheres. One source of truth for everything. No HP bars — only blood, oil, and power.*

</div>
