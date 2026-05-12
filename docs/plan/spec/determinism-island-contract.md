# Determinism Island Contract

## Purpose

Every simulation tick in Corefall produces byte-identical output given the same
inputs (seed, scenario, control commands, tick rate). This document defines what
is inside the deterministic island and what is outside.

## Inside the Determinism Island (must be identical across platforms)

| System | Boundary |
|---|---|
| `cf-sim-core` | `SimClock` tick counter, `Rng` (seeded xoshiro256++), `sim_state_v1` checksum |
| `cf-actor` | ActorState position/velocity/aim/hp/status/stability/knockdown, ControlIntent consumption, checksum_bytes |
| `cf-physics` | step_kinematics, apply_horizontal_motion, apply_jump, apply_recoil — all pure f32 math |
| `cf-equipment` | RifleState tick (fire cooldown, reload countdown, ammo count) — tick-rate-independent via seconds × hz |
| `cf-mission` | MissionState step (objective transitions, loss conditions, timer) |
| `cf-ai` | ReactiveGuard step (perception, tactic scoring, fire decision, miss roll via seeded Rng) |
| `cf-terrain` | ChunkedTerrain carve/blast (pixel mutation, dirty tracking) |
| `cf-chassis` | ChassisState damage/repair/eject (zone state, module state, pilot lifecycle) |
| `cf-replay` | Event envelope (tick + seq → event_id), Recorder append order |

## Outside the Determinism Island (may vary across platforms)

| System | Why |
|---|---|
| `cf-render-2d` | GPU rendering is not deterministic across Metal/Vulkan/DX12 |
| `cf-ui` | HUD text layout depends on font rasterizer |
| `cf-capture` | PNG encoding, frame timing |
| `cf-app` | Bevy frame scheduling, input polling cadence, window events |
| `cf-control` server | WebSocket message ordering, connection timing |
| Wall clock | `WallClock` is for pacing only; never feeds sim state |

## Checksum Contract

- Algorithm: BLAKE3 over `sim_state_v1` byte layout
- Scope: tick counter + RNG state + actor world checksum_bytes + breach/terrain/reactor bytes
- Cadence: configurable via `M0EngineConfig.checksum_cadence_ticks` (default 60)
- Validation: cf-headless replays the recorded commands and compares every cadence checksum
- Cross-platform: same checksum expected on macOS aarch64, Linux x86_64, Windows x86_64

## Rules

1. No `rand::thread_rng()` inside sim crates — enforced by `clippy.toml` `disallowed-methods`
2. No `SystemTime::now()` inside sim crates — enforced by `clippy.toml`
3. No `HashMap` iteration order in sim crates — use `BTreeMap` for deterministic ordering
4. All f32 math uses the same operations across platforms (no platform-specific intrinsics)
5. `quantize_f32` (multiply by 10000, round, cast to i32) used in checksum_bytes for stability
6. Every public mutator in sim crates is pure (`&mut self` → state out) with no side effects
