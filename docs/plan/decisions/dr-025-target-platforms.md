---
type: decision
id: DR-025
status: closed-direction
priority: P0
closed_at: 2026-05-04
revisit_trigger: "Steam Deck floor cannot be hit; or mobile becomes a strategic priority for retention/distribution; or headless Linux server requirements force a different stack."
---

← [[decisions/index|decision records]] · [[dashboards/decision-tracker|decision tracker]] · [[spec/prototype-roadmap|native build roadmap]] · [[decisions/dr-024-native-engine-stack|DR-024]] · [[decisions/dr-028-visual-fidelity-targets|DR-028]]

# DR-025: Target Platforms

> [!success] Status: CLOSED-DIRECTION (project owner committed 2026-05-04)
> Desktop-first: **Windows + Linux + macOS** at launch. **Steam Deck (800p/60) floor**. Headless **Linux server** for online/MMO experiments. Web only for labs/tools/demos. **No mobile** at launch.

## Decision

**Native desktop is the launch surface. Steam Deck is a first-class compatibility target. Headless Linux server is a milestone-9 deliverable. Mobile is out of scope for v1.**

This DR closes the platform reach question and defines the perf floors and ceilings (per [[decisions/dr-028-visual-fidelity-targets]]).

## What This Locks In

| Surface | Status |
|---|---|
| Windows (10+/11) | First-class launch target. |
| Linux (Ubuntu LTS + Steam Runtime baseline) | First-class launch target. |
| macOS (Apple Silicon + Intel) | First-class launch target. |
| Steam Deck (Linux + Proton) | First-class compatibility target. 800p/60 floor per DR-028. |
| Headless Linux server | Milestone 9 deliverable; runs `cf-headless` binary without graphics drivers per [[decisions/dr-005-multiplayer-posture]]. |
| Web (lab, tools, demos) | Optional; build via wasm if needed; not a launch surface. |
| Mobile (iOS/Android) | NOT a launch target. Possible after launch as a separate spinoff if the design proves it can shrink. |
| Console (Switch/PS/Xbox) | Not committed. Possible after launch via Steam Deck-tested portable build. |
| VR/AR | Out of scope. |

## What This Does NOT Lock

- Specific Linux distros beyond "Steam Runtime + Ubuntu LTS baseline".
- Whether Steam Deck ships day one or shortly after.
- Cloud-save sync (covered by [[decisions/dr-029-save-game-model]] as future work).
- Console-specific ports (post-launch evaluation).

## Why Not The Alternatives

| Alternative | Why Rejected |
|---|---|
| Mobile-first | Wrong tactile model for chunked pixel sim + chassis grammar; would water down the design. |
| Web-first | wgpu webgpu support is improving but inadequate for the perf ceiling we need. |
| Windows-only | Cuts off Linux/Steam Deck audience that overlaps with Cortex/Liero/Soldat veterans. |
| Console-first | Console certs add cost/time we can't carry as solo+AI. Reachable post-launch via Steam Deck-tested portable. |

## Evidence Trail

- Project owner verbatim (2026-05-04 stack round): "Windows + Linux + macOS desktop-first; Steam Deck floor; headless Linux server later; web only for labs/tools/demos; no mobile."
- Cross-platform CI is enforceable on all three desktop targets via GitHub Actions per DR-024.
- Steam Deck inherits Linux build path; perf budget per DR-028.
- Headless Linux server is a natural outcome of the deterministic-island sim core per DR-002 and the cf-net crate per DR-024.

## Risks

| Risk | Mitigation |
|---|---|
| Apple Silicon perf parity gaps | wgpu Metal backend; per-milestone macOS perf pass; no shader paths that target only DX/Vulkan. |
| Steam Deck thermal/perf budget too tight | T-PERF side track measures at every milestone; degrade gracefully (lower-res particles, smaller terrain chunk count). |
| Linux audio/input quirks | Test on Ubuntu LTS + Steam Deck explicitly at every milestone; document fallbacks. |
| Web demo expectation creep | Web is a "if convenient" target only; explicit non-promise. |
| Mobile audience demand | Document non-promise; revisit if the genre lands a mobile fit (post-launch). |

## Prototype / Validation Plan

| Test | What It Proves |
|---|---|
| M0 — CI matrix green on Win/Linux/macOS. | All three desktop targets build and test. |
| M0 — Project owner runs the build on macOS personally. | macOS is not just a CI green — it works locally. |
| M2/M5 — Steam Deck reference scene runs at 800p/60. | Floor target is hit with carving + chassis. |
| M9 — Headless `cf-headless` binary on a Linux VPS. | Server target works without graphics. |

## Revisit Trigger

- Steam Deck cannot hit 800p/60 floor at M5 or later.
- Mobile becomes a strategic priority for retention/distribution (post-launch reassessment).
- A console partnership offer arrives (post-launch case-by-case).
- macOS or Linux drops materially below Win in stability/perf without recovery path.

## Source Trail

- Project owner stack-round answers (2026-05-04).
- [[decisions/dr-024-native-engine-stack]]
- [[decisions/dr-028-visual-fidelity-targets]]
- [[decisions/dr-005-multiplayer-posture]]
- [[spec/prototype-roadmap]] — T-PLATFORM side track.
- [[research-log/2026-05-04-roadmap-rebuild-native-stack]]
