---
type: decision
id: DR-024
status: closed-direction
priority: P0
closed_at: 2026-05-04
revisit_trigger: "Bevy or wgpu blocks a critical capability we cannot work around with custom crates; or solo+AI throughput on this stack proves materially worse than an alternative."
---

← [[decisions/index|decision records]] · [[dashboards/decision-tracker|decision tracker]] · [[spec/prototype-roadmap|native build roadmap]] · [[decisions/dr-001-engine-strategy|DR-001]] · [[decisions/dr-026-team-and-repo-model|DR-026]]

# DR-024: Native Engine Stack

> [!success] Status: CLOSED-DIRECTION (project owner committed 2026-05-04)
> Native game engine: **Rust + Bevy/wgpu hybrid + custom core crates**. Bevy provides the app shell, ECS, asset pipeline, hot reload, and plugin system. wgpu (via Bevy and direct usage) is the renderer foundation. The systems that make this game special — sim core, pixel terrain, body/chassis grammar, AI, replay/event, networking, save — live in custom crates with explicit feature/agent boundaries.

## Decision

**Rust + Bevy + wgpu + custom core crates, in a modular cargo workspace.**

This closes the engine-stack subdecision that DR-001 intentionally left for the native planning round. DR-001 committed to greenfield native + CCCP as reference; this DR picks the language, runtime, renderer foundation, and the modular pattern.

## What This Locks In

| Layer | Choice |
|---|---|
| Language | Rust (edition 2021+). |
| App shell, ECS, windowing, input, asset pipeline, hot reload | Bevy. |
| Renderer foundation | wgpu via Bevy where it fits; **custom wgpu-first** for terrain/sprite/particle hot paths that need to hit 4K/120. |
| Sim core | Custom `cf-sim-core` crate with fixed-tick scheduler. |
| Pixel terrain | Custom `cf-terrain` crate (chunked, GPU-assisted carving). |
| Body/chassis/mech model | Custom `cf-chassis` crate. |
| AI | Custom `cf-ai` crate (perception, memory, doctrine, reason labels per [[decisions/dr-022-ai-humanlike-bar]]). |
| Replay/event | Custom `cf-replay` crate per [[decisions/dr-002-replay-event-architecture]]. |
| Networking | Custom `cf-net` crate built on a transport (lightyear / renet / quinn TBD). |
| Save | Custom `cf-save` crate per [[decisions/dr-029-save-game-model]]. |
| UI | egui (Bevy plugin) for tools/workbench; custom Bevy UI or egui-skinned for game HUD. |
| Audio | Bevy audio backend or kira. |
| Modding scripts | mlua (Lua) or Rhai — pick during M5 implementation. |
| Build / CI | cargo + GitHub Actions (Win/Linux/macOS matrix). |

## What This Does NOT Lock

- Specific Bevy version or upgrade cadence.
- Whether the custom 2D renderer eventually replaces Bevy's renderer or stays as a hot-path supplement.
- Lua vs Rhai for scripting (decision deferred to M5).
- Specific transport library for networking.
- UI library specifics (egui vs custom Bevy UI for HUD).

## Why Not The Alternatives

| Alternative | Why Rejected |
|---|---|
| C++ greenfield (Unreal/Unity/raw) | Slower iteration, weaker module/agent boundaries, AGPL contamination concerns from CCCP reference work. |
| Pure Rust without an engine | Reinventing windowing/input/ECS/asset pipeline costs weeks better spent on game-specific code. |
| Godot 4 (GDScript or Rust binding) | Rendering and ECS not at the level we need for 4K/120 + chunked pixel terrain; GDScript adds a non-Rust layer. |
| Unity / Unreal | Royalty/license posture, opaque internals, weaker fit for chunked pixel terrain + custom sim, less amenable to AI-augmented modular development. |
| Bevy alone (no custom crates) | Bevy is excellent but its renderer/audio/UI/networking subsystems do not deliver pixel-terrain + 4K/120 + deterministic-island sim out of the box. We use Bevy where it earns its keep and write custom for hot paths. |

## Evidence Trail

- Project owner verbatim (2026-05-04 stack round): chose Rust + Bevy/wgpu hybrid + custom core crates as the native stack.
- DR-001 already committed to greenfield native (2026-05-04).
- Bevy ecosystem maturity (versions 0.13+/0.14+) is sufficient for this workload.
- wgpu cross-platform parity (DX12/Vulkan/Metal/WebGPU) covers all DR-025 platform targets.
- DR-002 replay/event work fits naturally as a custom Bevy plugin/crate.

## Risks

| Risk | Mitigation |
|---|---|
| Bevy breaking changes | Pin version; treat upgrades as scheduled work; isolate Bevy-facing surface in `cf-app` plus a few thin plugins. |
| Custom wgpu hot-path is more work than expected | Start with off-the-shelf Bevy renderer; introduce custom wgpu only where perf demands. CPU fallback for terrain carving always present. |
| GPU-assisted terrain carving differs Metal/Vulkan/DX12 | wgpu abstracts; CPU fallback always present; CI tests all three platforms per DR-025. |
| Lua vs Rhai decision drift | Forced decision at M5; pick based on real script needs from chassis/AI work. |
| Determinism leaks through Bevy's frame loop | `cf-sim-core` runs fixed-tick on its own schedule; rendering decoupled. Audited at every milestone end via [[decisions/dr-002-replay-event-architecture]]. |

## Prototype / Validation Plan

| Test | What It Proves |
|---|---|
| M0 — `cargo build --release` succeeds on Win/Linux/macOS in CI. | Stack is buildable on all targets. |
| M0 — Bevy app launches and ticks at 60 Hz fixed island. | Sim core layer works on top of Bevy. |
| M0 — Run-bundle writer emits manifest+events+summary+notes from a no-op scene. | Replay envelope integrates with Bevy. |
| M1 — One actor playable for 5 minutes in-engine. | Custom sim + render + replay + UI integrate. |
| M2 — Pixel terrain digs at 4K/120 baseline. | Custom wgpu hot-path is justified. |

## Revisit Trigger

- Bevy or wgpu blocks a critical capability we cannot work around with custom crates.
- Solo+AI throughput on this stack is materially worse than an alternative.
- A milestone slips primarily due to the Bevy upgrade treadmill.

## Source Trail

- Project owner stack-round answers (2026-05-04).
- [[decisions/dr-001-engine-strategy]] — direction predecessor.
- [[decisions/dr-002-replay-event-architecture]]
- [[decisions/dr-022-ai-humanlike-bar]]
- [[spec/prototype-roadmap]] — Stack at a Glance + Repository Layout.
- [[research-log/2026-05-04-roadmap-rebuild-native-stack]]
