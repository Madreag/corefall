---
type: spec
status: closed-direction
authority: "Per-milestone enhancement pass for M1..M12: universal enhancement contract (perf gate per tier, network sync verification, CLI testability matrix, AI audio integration, juice/feel coverage, accessibility ACC-A floor, localization keyed strings, modding parity, memory leak soak, replay determinism, anti-FOMO + anti-pay-to-win audit, full-subtitle option) PLUS per-milestone specifics."
ready_when: "Every M1+ milestone done-criteria includes universal enhancement template; every milestone has expanded cfctl test coverage; AI agents drive milestone validation."
feeds:
  - DR-002
  - DR-005
  - DR-008
  - DR-012
  - DR-019
  - DR-020
  - DR-022
  - DR-031
  - DR-046
  - DR-052
  - DR-053
  - DR-054
  - DR-055
  - DR-056
  - DR-057
---

← [[spec/index|spec section]] · [[decisions/dr-056-per-milestone-enhancement-pass-m1-plus|DR-056]] · [[decisions/dr-057-optional-gacha-battle-pass-and-private-prototype-license-posture|DR-057]] · [[spec/prototype-roadmap|roadmap]] · [[decisions/dr-052-network-sync-rollback-and-cli-testable-determinism|DR-052]] · [[decisions/dr-053-ai-audio-pipeline-realtime-and-generative|DR-053]] · [[decisions/dr-054-performance-optimization-and-profiling|DR-054]] · [[decisions/dr-055-game-feel-juice-and-flow-state|DR-055]]

# Per-Milestone Enhancement Pass M1+

> [!summary] What this page is
> Universal enhancement contract layered onto every M1..M12 milestone. Closes the "are we sure each milestone is fully complete?" gap. Adds: perf gate + network sync + CLI testability + AI audio integration + juice/feel + accessibility + localization + modding parity + memory leak soak + replay determinism + anti-FOMO + full-subtitle.

## Universal Enhancement Done-Criteria (Add to ALL Milestones)

```
**Universal Enhancement Done-Criteria (per DR-056):**
- [ ] Per-tier perf gate: Steam Deck 800p/60 + 1080p/60 + 4K/120 reference scenarios.
- [ ] CI bench regression test (no >5% regression vs baseline) per DR-054.
- [ ] Memory leak soak (24h+) clean per DR-051 + DR-054.
- [ ] Network sync verified via cfctl test sync-drift per DR-052.
- [ ] Replay determinism CI matrix passes (per platform + per architecture) per DR-002 + DR-052.
- [ ] All player surfaces scriptable via cfctl per T-CONTROL.
- [ ] AI agent-driven validation report logged per DR-026 + DR-056.
- [ ] All audio cues generated via DR-053 pipeline + usage-ledger logged; private prototypes pass `cf-asset-ledger check --mode private`, and public sale/release candidates pass `cf-asset-ledger check --mode release` per DR-057.
- [ ] All gameplay events have juice rules per DR-055.
- [ ] Accessibility ACC-A floor verified (UI 200% + high contrast + captions + reduced motion) per DR-012.
- [ ] Localization keyed strings (Tier-A 11 languages) verified per DR-046.
- [ ] Modding parity verified (mod-author can extend; mod-test-run AI agent validates) per DR-006 + DR-050.
- [ ] Anti-FOMO + anti-pay-to-win audit passes per DR-031.
- [ ] Captions for ALL audio (full-subtitle option) per DR-051.
```

## Per-Milestone Enhancement Specifics

### M1 — Actor Controller And Sim Core

**Add to scope:**
- Input prediction for player-driven actor (per DR-052 client prediction).
- Bevy workspace pin updated to the latest verified crate (`0.18.1` as of 2026-05-07), exact-pinned, and compile/test-validated per DR-024.
- Recoil curves per weapon (per DR-055).
- Camera punch on damage taken.
- Animation event tags fire correctly (per [[spec/animation-system]]).
- Audio: footstep + reload + weapon-fire generated (per DR-053 Tier 1).
- cfctl `act move/aim/fire/reload` with assertion harness.
- ACC-A: keyboard remap + reduced motion settings.

**Add to done-criteria:** Universal enhancement template.
- `cargo update -p bevy --precise 0.18.1`, `cargo check --workspace --all-targets`, `cargo test --workspace`, and `cargo run -p cfctl -- observe --once` pass after the Bevy migration.

### M1.5 — Micro Breach Fun Slice

**Add to scope:**
- Match feel-test playtest (project-owner + 3-5 testers).
- Adaptive difficulty toggle (per DR-050 onboarding).
- AI difficulty preset visible (Cakewalk / Tough Crowd / Veteran).
- Replay sharing prototype.

### M2 — Pixel Terrain And Materials

**Add to scope:**
- GPU compute path investigation (deterministic backup; CPU baseline per DR-054).
- SIMD material kernel update (8 pixels/SIMD lane; deterministic; per DR-054).
- Streaming asset budget per scenario.
- Cold-load benchmark in CI.

### M3 — Replay And Event Recorder

**Add to scope:**
- Per-tick checksum (blake3); replay determinism CI matrix per platform.
- Replay branching (multiple replay paths from same checkpoint).
- Replay editing tools prototype (replay-as-data per DR-002).
- Replay sharing infrastructure (per DR-047 streaming/creator features).

### M4 — HUD And Comic-Noir UI

**Add to scope:**
- Reactive UI data binding (per Bevy state).
- UI testing harness (`cfctl ui assert`).
- All juice rules per DR-046 + DR-055.
- Accessibility 200% UI scale + high contrast verified.
- Localization keyed strings (Tier-A 11 languages).
- Animation system for UI panels (slide + skew per DR-046).
- Settings menu full tree (per [[spec/shell-ui-architecture]]).

### M5 — Equipment, Chassis, And Damage Grammar

**Add to scope:**
- Hot-reload polish (cf-mod reload <id>).
- Equipment validation in playtest scenarios.
- Equipment AI behavior tests (utility scoring per weapon).
- Origin-resource integration (per DR-040 + M5.8).
- Damage stage juice (per DR-055).
- Audio per weapon fire / reload / hit (per DR-053).

### M5.5 — Full Collision Gauntlet

**Add to scope:**
- GJK/EPA SIMD optimization (per DR-054).
- Sleep island optimization.
- Spatial partitioning tuning + benchmarks.
- Network sync for collision events (per DR-052).
- Audio per collision class (impulse-based mix per DR-053).

### M5.6 — Material Kernel

**Add to scope:**
- GPU compute path (deterministic verified; CPU fallback per DR-054).
- Multi-threaded chunk dispatch (Bevy parallel system).
- Per-chunk LOD (sleeping chunks).
- Audio per material reaction (per DR-053).

### M5.7 — Hazard Package

**Add to scope:**
- Hazard intensity smoothing.
- UI hazard overlay optimization.
- Per-hazard caption + audio + VFX coverage.
- Adaptive difficulty hazard scaling per DR-055.

### M5.8 — Origin Resource & Overclock Pass

**Add to scope:**
- Visual feedback for resource state.
- Overclock animation polish (per DR-055; chassis-vent VFX + audio).
- Audio per resource depletion (heartbeat sub-bass + warning bleep).

### M5.9 — Atmospherics-Grade Kernel

**Add to scope:**
- Thermodynamics solver SIMD optimization.
- Multi-thread atmosphere update + pipe-network parallel solve.
- Audio per atmospheric event (door cycle + suit breach + vent).
- Network sync for atmosphere state (per DR-052).

### M5.10 — Worlds Catalog & Environmental Aggregation

**Add to scope:**
- Dynamic loading per planet (don't load all 12 at scenario start).
- World streaming (background load nearby worlds).
- Per-world audio ambient generated (per DR-053).
- Per-world adaptive music swap.

### M6 — AI Core And Trust Harness

**Add to scope:**
- AI threading (parallel system per DR-054).
- AI prediction visualization (utility scoring tree visible per DR-022).
- AI behavior diff tooling.
- AI difficulty preset visible (per DR-050).
- AI faction personality identifiability (per DR-050).
- Audio for AI commands + reason labels (per DR-053).

### M6.5 — LLM Mind Lab

**Add to scope:**
- LLM caching (per-prompt cache; deterministic responses for replay).
- LLM cost tracking + budget cap enforcement.
- LLM prompt versioning.
- LLM training mode for modders (per DR-050).
- Determinism: replay-deterministic via cached responses per session.

### M6.6 — AI Material / Environmental Competence

**Add to scope:**
- AI training data export (per DR-050 modder support).
- AI weather doctrine visible (per [[spec/social-and-onboarding-extensions]]).
- Per-faction AI material preference (per DR-050 faction personality).

### M7 — Mission Director / Breach Contract / Bunker Defence

**Add to scope:**
- Mission director threading (parallel system).
- Mission objective animation polish (per DR-055).
- Comic-panel briefing + debrief (per DR-046; AI-generated panels per DR-044).
- Audio: mission stings per phase (per DR-053).
- Per-faction mission narrative copy (per [[spec/narrative-bible]]).

### M7.5 — Base Atmospherics

**Add to scope:**
- Pipe network UI (visual diagram editor).
- Atmospheric overlay optimization (per-room overlay LOD).
- Audio per atmospheric event (door cycle + breach + alarm).

### M7.7 — Day/Night/Weather

**Add to scope:**
- Weather prediction visualization (HUD + map per [[spec/shell-ui-architecture]]).
- Per-shard sync (cross-shard weather coordination).
- Audio per weather event (per DR-053).
- Cinematic punch on weather event start (per DR-055).

### M8 — Scenario Editor And Mod Tools

**Add to scope:**
- Editor undo/redo polish (replay-stack-based; deterministic).
- Multi-user editing (collaborative per [[spec/modding-ecosystem-extensions]]).
- Visual scripting for non-coders (per DR-006 + DR-050).
- Mod-test-run AI agent validates submitted mods (per DR-050).

### M8.5 — Material Lab

**Add to scope:**
- Material export to JSON / RON.
- Material sharing via Workshop (per DR-050).
- AI-generated material previews (per DR-044).

### M8.6 — Mining and Extraction

**Add to scope:**
- Mining UI optimization (per-resource visualization).
- Ore depletion visualization (per [[spec/atmospheric-effects-and-decals]]).
- Audio per mining tool + ore type (per DR-053).
- Vendor / economy NPCs (per DR-049 + [[spec/customization-and-progression-depth]]).

### M9 — Dedicated Server App + Determinism Islands

**Add to scope:**
- Server lifecycle perf (cold start <5s; restart <30s).
- Server orchestration (per [[spec/post-launch-operations-and-platform]]).
- Server scaling (50-200 concurrent players per shard).
- Network simulator integration (per DR-052).
- Determinism CI matrix (per platform + per architecture).

### M9.5 — Voice & Radio Comms

**Add to scope:**
- Voice quality testing (per DR-053; XTTS quality vs ElevenLabs vs Tortoise comparison).
- Radio interference modeling (per DR-043; weather + EMP + jammer integration).
- Per-faction voice variety (per DR-050 voice variety per origin).
- Network sync for voice transmission (per DR-052; server-authoritative).

### M10 — LAN Co-op

**Add to scope:**
- LAN discovery polish (auto-detect; ping; mod-hash check).
- LAN troubleshooting tools (cfctl test sync-drift live).
- Deterministic lockstep verification (per DR-052).
- Replay sync verification across clients.

### M11 — Online Co-op (Self-Hosted Dedicated Servers)

**Add to scope:**
- NAT punch-through testing (cfctl test latency-injection).
- Relay fallback (per DR-052 transport).
- Latency masking (snapshot interpolation + lag compensation per DR-052).
- Per-mission save sync across party (per [[spec/social-and-onboarding-extensions]]).
- Server rewind for hit validation (per DR-052 lag compensation).

### M12 — Public PvP Arenas + Persistent MMO Shards

**Add to scope:**
- Load testing (50-200 concurrent per shard; cfctl test multi-shard).
- Sharding strategy (per [[spec/persistent-mmo-architecture]]).
- Matchmaking pool (per DR-049 ELO/MMR).
- Anti-cheat ML (per DR-051 anti-cheat heuristics; tournament-grade per DR-049).
- Rollback netcode for PvP (per DR-052).
- Cross-shard event broadcaster (per DR-048 + [[spec/server-wide-events-and-meta-narrative]]).

## Done-Criteria Per Milestone

Every M1..M12 milestone done-criteria block now adds:

```
**Universal Enhancement Done-Criteria (per DR-056):**
- [ ] All universal enhancement items pass.
**Per-Milestone Specifics (per this page):**
- [ ] All per-milestone enhancements above pass.
```

## CI Integration

Per-milestone CI:
- `cf-bench` regression vs baseline.
- `cfctl test sync-drift` per multiplayer milestone.
- `cf-i18n-check` per UI milestone.
- `cf-caption-check` per audio milestone.
- `cargo-allocator-stats` per hot-path milestone.
- Memory leak detection per long-soak milestone.

## Source Trail

- [[decisions/dr-056-per-milestone-enhancement-pass-m1-plus]]
- [[decisions/dr-052-network-sync-rollback-and-cli-testable-determinism]]
- [[decisions/dr-053-ai-audio-pipeline-realtime-and-generative]]
- [[decisions/dr-054-performance-optimization-and-profiling]]
- [[decisions/dr-055-game-feel-juice-and-flow-state]]
- [[spec/prototype-roadmap]]
- [[spec/native-implementation-backlog]]
- [[research-log/2026-05-07-comprehensive-audit-report]]
