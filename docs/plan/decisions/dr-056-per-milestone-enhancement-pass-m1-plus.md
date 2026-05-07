---
type: decision
id: DR-056
status: closed-direction
priority: P0
closed_at: 2026-05-06
revisit_trigger: "Per-milestone enhancement audit reveals additional gaps; perf budget exceeded; CLI test coverage insufficient; AI agents cannot drive milestone validation."
---

← [[decisions/index|decision records]] · [[dashboards/decision-tracker|decision tracker]] · [[spec/milestone-enhancement-pass-m1-plus|milestone enhancement pass spec]] · [[spec/prototype-roadmap|roadmap]] · [[decisions/dr-052-network-sync-rollback-and-cli-testable-determinism|DR-052]] · [[decisions/dr-053-ai-audio-pipeline-realtime-and-generative|DR-053]] · [[decisions/dr-054-performance-optimization-and-profiling|DR-054]] · [[decisions/dr-055-game-feel-juice-and-flow-state|DR-055]]

# DR-056: Per-Milestone Enhancement Pass M1+

> [!success] Status: CLOSED-DIRECTION (project owner committed 2026-05-06)
> Comprehensive enhancement pass for every M1..M12 milestone. Adds: perf gate per tier (per DR-054), network sync verification (per DR-052), CLI testability matrix (per T-CONTROL), AI audio pipeline integration (per DR-053), juice/feel coverage (per DR-055), accessibility ACC-A floor (per DR-012), localization keyed-string verification (per DR-046), modding parity (per DR-006 + DR-050), perf bench regression test (per DR-054). Every milestone is enhanced; every milestone has expanded done-criteria; every milestone has expanded cfctl test coverage.

## Decision

### Universal milestone enhancements (apply to ALL milestones M1+)

| Enhancement | Detail |
|---|---|
| **Per-tier perf gate** | Steam Deck 800p/60 + 1080p/60 + 4K/120 reference scenarios pass; AI-agent-driven perf analysis report; CI bench regression test (no >5% regression). |
| **Network sync verification** | Per [[spec/network-sync-rollback-and-determinism]]; deterministic replay across runs; `cfctl test sync-drift` passes for every milestone with multiplayer surface. |
| **CLI testability matrix** | Every player-facing surface scriptable via cfctl; observation reachable; action triggerable; assertion gates per surface. |
| **AI audio pipeline integration** | All audio cues for milestone events generated via [[spec/ai-audio-pipeline-realtime-and-generative]]; usage-ledger logged. |
| **Juice / feel coverage** | All gameplay events have juice rules per [[spec/game-feel-juice-and-flow-state]] (camera shake + audio + VFX + UI feedback). |
| **Accessibility ACC-A floor** | Per DR-012; UI scale 200% + high contrast + captions + reduced motion verified per milestone. |
| **Localization keyed strings** | Per DR-046 + [[spec/localization-plan]]; CI gate `cf-i18n-check` passes; zero hardcoded English. |
| **Modding parity** | Per DR-006 + DR-050; mod-author can extend per-milestone surface; modder-test-run AI agent validates. |
| **Memory leak soak** | Per DR-054 + DR-051; 24h+ soak run produces zero leaks. |
| **Replay determinism** | Per DR-002 + DR-052; same seed + inputs produce bit-identical final state. |
| **Anti-FOMO + anti-pay-to-win audit** | Per DR-031 + DR-046 + DR-049; no power gating; no FOMO mechanics. |
| **Captions for ALL audio** | Per DR-051 accessibility-plus full-subtitle option; not just critical audio. |

### Per-milestone enhancement specifics (M1+)

#### M1 — Actor Controller And Sim Core

Add:
- Input prediction for player-driven actor (per DR-052 client prediction)
- Recoil curves per weapon (per DR-055)
- Camera punch on damage taken
- Animation event tags fire correctly (per [[spec/animation-system]])
- Audio: footstep + reload + weapon-fire generated (per DR-053 Tier 1)
- cfctl `act move/aim/fire/reload` with assertion harness
- ACC-A: keyboard remap + reduced motion settings

#### M1.5 — Micro Breach Fun Slice

Add:
- Match feel-test playtest (project-owner + 3-5 testers)
- Adaptive difficulty toggle (per DR-050 onboarding)
- AI difficulty preset visible (Cakewalk / Tough Crowd / Veteran)
- Replay sharing prototype

#### M2 — Pixel Terrain And Materials

Add:
- GPU compute path investigation (deterministic backup; CPU baseline per DR-054)
- SIMD material kernel update (8 pixels per SIMD lane; deterministic; per DR-054)
- Streaming asset budget per scenario
- Cold-load benchmark in CI

#### M3 — Replay And Event Recorder

Add:
- Per-tick checksum (blake3); replay determinism CI matrix per platform
- Replay branching (multiple replay paths from same checkpoint)
- Replay editing tools prototype (replay-as-data per DR-002)
- Replay sharing infrastructure (per DR-063 streaming features)

#### M4 — HUD And Comic-Noir UI

Add:
- Reactive UI data binding (per Bevy state)
- UI testing harness (`cfctl ui assert`)
- All juice rules per DR-046 + DR-055
- Accessibility 200% UI scale + high contrast verified
- Localization keyed strings (Tier-A 11 languages)
- Animation system for UI panels (slide + skew per DR-046)
- Settings menu full tree (per [[spec/shell-ui-architecture]])

#### M5 — Equipment, Chassis, And Damage Grammar

Add:
- Hot-reload polish for equipment + chassis content (cf-mod reload <id>)
- Equipment validation in playtest scenarios
- Equipment AI behavior tests (utility scoring per weapon)
- Origin-resource integration (per DR-040 + M5.8)
- Damage stage juice (per DR-055 hit stop + camera punch + blood splatter)
- Audio per weapon fire / reload / hit (per DR-053)

#### M5.5 — Full Collision Gauntlet

Add:
- GJK/EPA SIMD optimization (per DR-054)
- Sleep island optimization (per DR-054)
- Spatial partitioning tuning + benchmarks
- Network sync for collision events (per DR-052; deterministic across clients)
- Audio per collision class (impulse-based mix per DR-053)

#### M5.6 — Material Kernel

Add:
- GPU compute path for material update (deterministic verified; CPU fallback per DR-054)
- Multi-threaded chunk dispatch (Bevy parallel system)
- Per-chunk LOD (sleeping chunks; idle reduce update frequency)
- Audio per material reaction (per DR-053; sub-bass for explosion; sizzle for steam)

#### M5.7 — Hazard Package

Add:
- Hazard intensity smoothing (per [[spec/atmospheric-effects-and-decals]])
- UI hazard overlay optimization
- Per-hazard caption + audio + VFX coverage
- Adaptive difficulty hazard scaling per DR-055

#### M5.8 — Origin Resource & Overclock Pass

Add:
- Visual feedback for resource state (HP bar overlay + status icons)
- Overclock animation polish (per DR-055; chassis-vent VFX + audio)
- Audio per resource depletion (heartbeat sub-bass + warning bleep)

#### M5.9 — Atmospherics-Grade Kernel

Add:
- Thermodynamics solver SIMD optimization
- Multi-thread atmosphere update + pipe-network parallel solve
- Audio per atmospheric event (door cycle + suit breach + vent)
- Network sync for atmosphere state (per DR-052; server-authoritative)

#### M5.10 — Worlds Catalog & Environmental Aggregation

Add:
- Dynamic loading per planet (don't load all 12 at scenario start)
- World streaming (background load nearby worlds)
- Per-world audio ambient generated (per DR-053)
- Per-world adaptive music swap

#### M6 — AI Core And Trust Harness

Add:
- AI threading (parallel system per DR-054)
- AI prediction visualization (utility scoring tree visible per DR-022)
- AI behavior diff tooling (compare doctrines side-by-side)
- AI difficulty preset visible (per DR-050)
- AI faction personality identifiability (per DR-050)
- Audio for AI commands + reason labels (per DR-053)

#### M6.5 — LLM Mind Lab

Add:
- LLM caching (per-prompt cache; deterministic responses for replay)
- LLM cost tracking + budget cap enforcement
- LLM prompt versioning
- LLM training mode for modders (per DR-050)
- Determinism: replay-deterministic via cached responses per session

#### M6.6 — AI Material / Environmental Competence

Add:
- AI training data export (per DR-050 modder support)
- AI weather doctrine visible (per [[spec/social-and-onboarding-extensions]])
- Per-faction AI material preference (per DR-050 faction personality)

#### M7 — Mission Director / Breach Contract / Bunker Defence

Add:
- Mission director threading (parallel system)
- Mission objective animation polish (per DR-055)
- Comic-panel briefing + debrief (per DR-046; AI-generated panels per DR-044)
- Audio: mission stings per phase (per DR-053)
- Per-faction mission narrative copy (per [[spec/narrative-bible]])

#### M7.5 — Base Atmospherics

Add:
- Pipe network UI (visual diagram editor)
- Atmospheric overlay optimization (per-room overlay LOD)
- Audio per atmospheric event (door cycle + breach + alarm)

#### M7.7 — Day/Night/Weather

Add:
- Weather prediction visualization (HUD + map per [[spec/shell-ui-architecture]])
- Per-shard sync (cross-shard weather coordination)
- Audio per weather event (per DR-053; rain + dust storm + thunder)
- Cinematic punch on weather event start (per DR-055)

#### M8 — Scenario Editor And Mod Tools

Add:
- Editor undo/redo polish (replay-stack-based; deterministic)
- Multi-user editing (collaborative per [[spec/modding-ecosystem-extensions]])
- Visual scripting for non-coders (per DR-006 + DR-050)
- Mod-test-run AI agent validates submitted mods (per DR-050)

#### M8.5 — Material Lab

Add:
- Material export to JSON / RON
- Material sharing via Workshop (per DR-050)
- AI-generated material previews (per DR-044)

#### M8.6 — Mining and Extraction

Add:
- Mining UI optimization (per-resource visualization)
- Ore depletion visualization (per [[spec/atmospheric-effects-and-decals]])
- Audio per mining tool + ore type (per DR-053)
- Vendor / economy NPCs (per DR-049 + [[spec/customization-and-progression-depth]])

#### M9 — Dedicated Server App + Determinism Islands

Add:
- Server lifecycle perf (cold start <5s; restart <30s)
- Server orchestration (per [[spec/post-launch-operations-and-platform]])
- Server scaling (50-200 concurrent players per shard)
- Network simulator integration (per DR-052)
- Determinism CI matrix (per platform + per architecture)

#### M9.5 — Voice & Radio Comms

Add:
- Voice quality testing (per DR-053; XTTS quality vs ElevenLabs vs Tortoise comparison)
- Radio interference modeling (per DR-043; weather + EMP + jammer integration)
- Per-faction voice variety (per DR-050 voice variety per origin)
- Network sync for voice transmission (per DR-052; server-authoritative)

#### M10 — LAN Co-op

Add:
- LAN discovery polish (auto-detect; ping; mod-hash check)
- LAN troubleshooting tools (cfctl test sync-drift live)
- Deterministic lockstep verification (per DR-052)
- Replay sync verification across clients

#### M11 — Online Co-op (Self-Hosted Dedicated Servers)

Add:
- NAT punch-through testing (cfctl test latency-injection)
- Relay fallback (per DR-052 transport)
- Latency masking (snapshot interpolation + lag compensation per DR-052)
- Per-mission save sync across party (per [[spec/social-and-onboarding-extensions]] co-op campaign)
- Server rewind for hit validation (per DR-052 lag compensation)

#### M12 — Public PvP Arenas + Persistent MMO Shards

Add:
- Load testing (50-200 concurrent per shard; cfctl test multi-shard)
- Sharding strategy (per [[spec/persistent-mmo-architecture]])
- Matchmaking pool (per DR-049 ELO/MMR)
- Anti-cheat ML (per DR-051 anti-cheat heuristics; tournament-grade per DR-049)
- Rollback netcode for PvP (per DR-052)
- Cross-shard event broadcaster (per DR-048 + [[spec/server-wide-events-and-meta-narrative]])

### Per-milestone done-criteria template (added to ALL milestones)

```
**Universal Enhancement Done-Criteria:**
- [ ] Per-tier perf gate: Steam Deck 800p/60 + 1080p/60 + 4K/120.
- [ ] CI bench regression test (no >5% regression vs baseline).
- [ ] Memory leak soak (24h+) clean.
- [ ] Network sync verified via cfctl test sync-drift.
- [ ] Replay determinism CI matrix passes (per platform).
- [ ] All player surfaces scriptable via cfctl.
- [ ] AI agent-driven validation report logged.
- [ ] All audio cues generated via DR-053 pipeline + usage-ledger logged.
- [ ] All gameplay events have juice rules per DR-055.
- [ ] Accessibility ACC-A floor verified (UI 200% + high contrast + captions + reduced motion).
- [ ] Localization keyed strings (Tier-A 11 languages) verified.
- [ ] Modding parity verified (mod-author can extend; mod-test-run AI agent validates).
- [ ] Anti-FOMO + anti-pay-to-win audit passes.
```

## What This Locks In

| Spec Area | Implication |
|---|---|
| Per-milestone task cards | Updated with universal enhancement done-criteria. |
| `cf-bench` | Per-milestone regression test. |
| `cfctl` | Per-milestone test commands. |
| Modder parity | Per-milestone modder validation. |
| AI agent validation | Per-milestone agent-driven sign-off. |

## What This Does NOT Lock

| Non-Commitment | Why |
|---|---|
| Specific perf-tier numbers per milestone | Open. Per-milestone target tuned in playtest. |
| Specific CLI test command count | Open. Per-milestone scope. |
| Voice acting commitment | Open. Default AI-generated. |

## Why This Direction

| Driver | Detail |
|---|---|
| Quality floor | Without universal enhancement template, each milestone risks regressing on perf / sync / accessibility / localization. |
| AI-agent-driven validation | Per DR-026; AI agents drive milestone validation; CLI test coverage mandatory. |
| Modder retention | Per DR-050; modders need per-milestone validation pipeline. |
| Replay determinism | Per DR-002 + DR-052; mandatory per milestone. |
| User pattern | Project owner: "make sure all our features are well implemented, coherent to the game, optimized." |

## Why Not The Alternatives

| Alternative | Why Rejected |
|---|---|
| Add enhancements per-milestone individually | Regression risk; some milestones already in-flight (M0 done; M1 in-flight). Universal template avoids gap. |
| Defer enhancement pass to post-launch | Per perf budget + sync requirements; cannot defer. |

## Evidence Trail

- Project owner verbatim (2026-05-06): "make sure we got everything planned and fully complete. all features well thought out for user retention, while staying true to the vision. UX should be perfect. UI should be perfect. all features should be perfect."
- DR-052..DR-055 establish the foundations (network sync, AI audio, perf, game feel).
- DR-056 binds them into per-milestone enhancement contract.
- Captured in [[research-log/2026-05-06-third-pass-audit-followup]] (TBD).

## Revisit Trigger

- Per-milestone enhancement audit reveals additional gaps.
- Perf budget exceeded.
- CLI test coverage insufficient.
- AI agents cannot drive milestone validation.
