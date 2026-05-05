---
type: decision
id: DR-028
status: closed-direction
priority: P0
closed_at: 2026-05-04
revisit_trigger: "Steam Deck cannot hit 800p/60 floor; or strong-desktop ceiling becomes structurally unreachable on the chosen renderer; or a perf budget fight forces a new fidelity ladder."
---

← [[decisions/index|decision records]] · [[dashboards/decision-tracker|decision tracker]] · [[spec/prototype-roadmap|native build roadmap]] · [[decisions/dr-019-visual-direction|DR-019]] · [[decisions/dr-024-native-engine-stack|DR-024]] · [[decisions/dr-025-target-platforms|DR-025]]

# DR-028: Visual Fidelity Targets

> [!success] Status: CLOSED-DIRECTION (project owner committed 2026-05-04)
> **Ceiling: 4K @ 120 Hz on strong desktop. Floor: 1080p @ 60 Hz on mid-range desktop. Steam Deck floor: 800p @ 60 Hz.** Pixel-sim battlefield + comic-noir presentation per DR-019; SDF/vector text and 200% UI scaling for accessibility per DR-012.

## Decision

**Three-target perf ladder**, all on the same content, with explicit per-frame budgets:

| Target | Hardware | Resolution | Refresh | Per-Frame Budget |
|---|---|---|---|---|
| Ceiling | Strong desktop (modern dGPU, 16GB+ RAM) | 4K (3840×2160) | 120 Hz | 8.33 ms |
| Default | Mid-range desktop / standard gaming laptop | 1080p (1920×1080) | 60 Hz | 16.67 ms |
| Floor (Deck) | Steam Deck OLED (or LCD Deck if it fits) | 800p (1280×800) | 60 Hz | 16.67 ms with thermal margin |

Sim runs at **60 Hz fixed island** (or 120 Hz for high-refresh inputs); render is decoupled from sim per DR-002 + DR-024.

## What This Locks In

| Aspect | Commitment |
|---|---|
| Ceiling | 4K @ 120 Hz on strong desktop with full content and effects. |
| Default | 1080p @ 60 Hz on mid-range desktop. |
| Steam Deck floor | 800p @ 60 Hz with graceful effect scaling. |
| Sim tick | 60 Hz fixed-tick island; 120 Hz option for high-refresh input. |
| Render decoupling | Render runs ahead of sim; interpolation between sim states. |
| Text rendering | SDF/vector text for clean scaling per DR-019 + DR-012. |
| UI scaling | Up to 200% text scale with reflow per DR-012. |
| Pixel-art rendering | Sub-pixel-clean pixel sprites; integer scaling where possible per DR-019. |

## What This Does NOT Lock

- Specific GPU model thresholds for "strong" vs "mid-range" desktop.
- Whether HDR is supported (open).
- Whether ultrawide aspect ratios get a special HUD layout (open; UX deliverable).
- Whether high-refresh-rate (144/240 Hz) display modes are supported beyond 120 (open; capacity-permitting).
- Console/mobile fidelity (out of scope per DR-025).

## Why This Ladder

| Reason | Why |
|---|---|
| Steam Deck floor | Cortex/Liero/Soldat audience overlap with Deck users; portability is a tactile fit for the chassis pacing. |
| 1080p/60 default | Largest install base; predictable budget. |
| 4K/120 ceiling | Pixel-sim + custom wgpu renderer have headroom; gives the game a "looks great on a high-end rig" moment. |
| 60 Hz sim floor | Fixed-tick islands per DR-002 mandate predictable cadence; 60 Hz is the universal sweet spot. |

## Why Not The Alternatives

| Alternative | Why Rejected |
|---|---|
| 30 Hz floor | Tactile feel suffers; chassis pacing depends on responsive controls. |
| 4K-only ceiling without Deck floor | Cuts off the most-likely portable audience. |
| Variable sim tick | Breaks deterministic-island contracts for replay/network per DR-002 + DR-005. |
| 4K/240 ceiling | Diminishing returns; perf budget eats game-content quality. |

## Evidence Trail

- Project owner verbatim (2026-05-04 stack round): "Strong desktop 4K/120 ceiling; 1080p/60 floor; Deck 800p/60 portable floor. Steam Deck-class is the floor."
- Bevy/wgpu can hit 4K/120 in benchmark with custom hot-path shaders.
- Steam Deck Linux + Proton path inherits the Linux build target per DR-025.
- DR-019 visual direction (pixel-sim + comic-noir) is naturally efficient at this ladder.

## Risks

| Risk | Mitigation |
|---|---|
| Steam Deck thermal throttling drops below 60 Hz | T-PERF side track measures at every milestone; degrade particles/AI step. |
| 4K/120 not reachable on common dGPUs | Adaptive resolution; explicit "epic" tier separate from "high" tier. |
| Custom wgpu hot-path slips behind | Start with off-the-shelf Bevy renderer; add custom only where measured. |
| Render-ahead interpolation introduces lag | Tunable interpolation factor; capped at 1 sim tick. |
| HUD breaks at 200% scale | Accessibility tests in T-ACCESSIBILITY at every milestone; UI built from scalable primitives. |

## Prototype / Validation Plan

| Test | What It Proves |
|---|---|
| M0 — Empty Bevy app hits 120 FPS at 1080p on a mid-range GPU. | Baseline ceiling is realistic. |
| M2 — Pixel terrain + carving session sustains 120 FPS at 1080p mid-range. | Custom wgpu hot-path delivers. |
| M2 — Pixel terrain on Steam Deck sustains 60 FPS at 800p. | Deck floor is real. |
| M5 — Powered armor + light mech + module particles sustain target ladder. | Chassis grammar respects budget. |
| M7 — Breach Contract scene with 5 actors + active terrain + base systems hits 4K/120 on strong desktop. | Ceiling realized on real content. |

## Revisit Trigger

- Steam Deck cannot hit 800p/60 floor by M5.
- Strong-desktop ceiling cannot be reached on the chosen renderer.
- A perf budget fight forces a new fidelity ladder.
- The audience signal (post-launch) shows ultra-high refresh / HDR / ultrawide demand.

## Source Trail

- Project owner stack-round answers (2026-05-04).
- [[decisions/dr-019-visual-direction]]
- [[decisions/dr-024-native-engine-stack]]
- [[decisions/dr-025-target-platforms]]
- [[decisions/dr-012-accessibility-comfort-readability]]
- [[spec/prototype-roadmap]] — T-PERF side track + per-milestone perf budgets.
- [[research-log/2026-05-04-roadmap-rebuild-native-stack]]
