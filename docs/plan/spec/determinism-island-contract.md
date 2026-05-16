# Determinism Island Contract

## Purpose

Every simulation tick in Corefall produces byte-identical output given the same
inputs (seed, scenario, control commands, tick rate). This document defines what
is inside the deterministic island, what is outside, and the cross-platform
float rules that keep the boundary stable across macOS aarch64, Linux x86_64,
and Windows x86_64 (per DR-052 § Cross-platform float-determinism).

## Inside the Determinism Island (must be identical across platforms)

| System | Boundary |
|---|---|
| `cf-sim-core` | `SimClock` tick counter, `Rng` (seeded xoshiro256++), `sim_state_v1` checksum |
| `cf-actor` | ActorState position/velocity/aim/hp/status/stability/knockdown, ControlIntent consumption, checksum_bytes |
| `cf-physics` | step_kinematics, apply_horizontal_motion, apply_jump, apply_recoil — all pure f32 math |
| `cf-equipment` | RifleState tick (fire cooldown, reload countdown, ammo count) — tick-rate-independent via seconds × hz |
| `cf-mission` | MissionState step (objective transitions, loss conditions, timer) |
| `cf-ai` | ReactiveGuard step (perception, tactic scoring, fire decision, miss roll via seeded Rng) |
| `cf-terrain` | ChunkedTerrain carve/blast (pixel mutation, dirty tracking, integrity grid) |
| `cf-chassis` | ChassisState damage/repair/eject (zone state, module state, pilot lifecycle) |
| `cf-replay` | Event envelope (tick + seq → event_id), Recorder append order, RecordId registry |

## Outside the Determinism Island (cosmetic-only)

| System | Why | Cosmetic flag |
|---|---|---|
| `cf-render-2d` | GPU rendering is not deterministic across Metal/Vulkan/DX12 | `cosmetic: true` on every emitted event |
| `cf-ui` | HUD text layout depends on font rasterizer | `cosmetic: true` |
| `cf-audio` | Sound playback timing is wall-clock driven | `cosmetic: true` |
| `cf-capture` | PNG encoding, frame timing | `cosmetic: true` |
| `cf-app` | Bevy frame scheduling, input polling cadence, window events, camera shake | `cosmetic: true` |
| `cf-control` server | WebSocket message ordering, connection timing | (irrelevant for sim checksum) |
| Wall clock | `WallClock` is for pacing only; never feeds sim state | (n/a) |

### Cosmetic event types (default-flagged)

Per M4 § Cosmetic events excluded from determinism scope:

- `terrain.debris_spawned` (visual particle; the underlying pixel removal is hashed)
- `hazard.tick` (batched per-tick visualization; the hazard grid is hashed)
- `affliction.tick` (batched per-tick damage application; affliction state is hashed)
- `ux.banner_raised` / `ux.banner_dismissed`
- `shield.hit` ripple (M13+ visual cosmetic; the GAMEPLAY `shield.hit` event is NOT cosmetic)
- render-2d particle / spark / dust events
- `capture.frame_emitted` (frame readback for offline review)

Rule: cosmetic events **describe** the change; the state itself is hashed via
`sim_state_v1`. A replay verifier with `cosmetic=true` events excluded MUST
still detect every gameplay drift because the underlying state is hashed.

## Checksum Contract

- **Algorithm**: BLAKE3 over `sim_state_v1` byte layout (fixed order, big-endian where applicable)
- **Cadence**: configurable via `M0EngineConfig.checksum_cadence_ticks` (default 60)
- **Validation**: cf-headless replays recorded control commands and compares every cadence checksum
- **Cross-platform**: same checksum expected on macOS aarch64, Linux x86_64, Windows x86_64

### `sim_state_v1` byte layout (M0..M4 layered additions)

| Milestone | Bytes added (fixed order) |
|---|---|
| M0 | `tick_counter (u64)`, `rng_state_bytes` |
| M1 | `+ actor_state_quantized` (per actor: id, pos_q16, vel_q16, hp_i16, status_u8, stance_u8, stability_q16, sharp_aim_q16, mass_q16, origin_id_u8) |
| M3 | `+ terrain_chunk_grid` (per dirty chunk: blake3 of material_grid) `+ terrain_integrity_grid` (per chunk: integrity u8) |
| M4 | `+ inventory_state` (per actor: slots + rifle ammo_in_mag + reloading) `+ mission_state` (phase + objective states + timer) |
| M9 (later) | `+ hazard_grid`, `+ affliction_state`, `+ armor_layer_state`, `+ internal_organ_state`, `+ concussion_dose_state`, `+ fluid_reservoir_state`, `+ origin_state`, `+ reactor_state`, `+ atmospherics_state`, `+ environment_signal_state` |
| M13 (later) | `+ chassis_state` (per-zone HP, module states, pilot state, eject ticks). Bumps scope to `sim_state_v2` with migration shim. |

Each scope name is canonical; bumping requires a migration. `sim_state_v1`
accepts the M0..M9 layered additions because they're additive (extending the
byte stream doesn't change the algorithm — only the input).

## Cross-Platform Float Rules (DR-052)

These rules guarantee that the same `sim_state_v1` checksum is produced on every
target platform:

1. **`f32` only inside sim crates** — `cf-sim-core`, `cf-actor`, `cf-physics`,
   `cf-equipment`, `cf-mission`, `cf-ai`, `cf-terrain`, `cf-chassis`,
   `cf-material`, `cf-atmos`. No `f64` in these crates. Quantize via
   `quantize_f32` (multiply by 10000, round to i32) for checksum bytes.
2. **SSE2 + SSE4.2 target features on x86_64** — set
   `RUSTFLAGS="-C target-feature=+sse2,+sse4.2"` in CI. Disables x87 FPU
   80-bit-internal rounding inconsistencies between toolchains.
3. **LLVM `-ffast-math` MUST be disabled** in sim crates. `fast-math` reorders
   FP ops and breaks bit-determinism. Use `-Cllvm-args=-no-fast-math` or
   verify the absence of `fp-arg-with-recip`-style optimizations.
4. **No `rand::thread_rng()`** in sim crates — enforced by `clippy.toml`
   `disallowed-methods`. Use the engine-seeded `Rng`.
5. **No `SystemTime::now()`** in sim crates — enforced by `clippy.toml`.
6. **No `HashMap` iteration order** in sim crates — use `BTreeMap` for
   deterministic ordering.
7. **All public sim mutators are pure** (`&mut self` → state out) with no
   side effects, no callbacks, no script invocation.
8. **Recorder hooks emit inert data only** (per CCCP `Atom.cpp:96-99`). The
   recorder MUST NOT trigger subscriber callbacks or sim mutations from
   collision/script paths.
9. **Stable `RecordId(u64)` registry**, never raw pointers / pooled MOIDs
   (per CCCP `MovableMan.cpp:126-143` stale pointer warning).

## CI verification (M36+ scope)

At M4 the surface is in place: `cf-headless replay <bundle>` reproduces the
recorded checksums on whichever platform CI is currently running. The
cross-platform Linux + Windows + macOS matrix that *compares* checksums across
three target platforms lands at M36+ per DR-052. M4 ships the rules; M36+
ships the matrix.

## Rules summary

1. No `rand::thread_rng()` inside sim crates — enforced by `clippy.toml` `disallowed-methods`
2. No `SystemTime::now()` inside sim crates — enforced by `clippy.toml`
3. No `HashMap` iteration order in sim crates — use `BTreeMap` for deterministic ordering
4. All f32 math uses the same operations across platforms (no platform-specific intrinsics)
5. `quantize_f32` (multiply by 10000, round, cast to i32) used in checksum_bytes for stability
6. Every public mutator in sim crates is pure (`&mut self` → state out) with no side effects
7. Cosmetic events are flagged `cosmetic: true` and excluded from `sim_state_v1` hashing
8. Recorder hooks emit inert data only (no callbacks, no mutation)
9. Stable `RecordId(u64)` registry; raw pointers / MOIDs never serialized

## M8A extensions (added 5/15/2026 by m8a-impl)

The M8A milestone introduces parallel sim, GPU cosmetic offload, and the
`cf-net` authoritative-server stack. These additions extend the determinism
island with 10 architecture rules that the CI gate at
`game/scripts/ci/m8a_determinism_lint.sh` enforces.

### 10 architecture rules (M8A § Notes for the implementer / Architecture rules)

1. **No `f64` in sim crates.** Use `f32` only. f64 hardware-acceleration paths
   diverge subtly across vendors. Quantize via `quantize_f32` for checksum
   bytes. Allowed boundary uses (must produce f32 outputs and never enter the
   tick checksum):
   - `cf-ai/src/lib.rs:378,381,1473-1477` and `cf-ai/src/constants.rs:49,52` —
     53-bit-mantissa trick for uniform f32 in [0, 1]; result cast to f32 at
     call site.
   - `cf-material/src/loader.rs:192,214` — JSON parse-time only; never enters
     tick path.
   - `cf-sim-core/src/lib.rs:57,167` (`tick_dt_ms`, `sim_time_ms`) — display /
     event-payload only; non-tick-path.
2. **No `thread_rng()`.** Every RNG call in a sim crate uses
   `cf_sim_core::Rng` seeded from the engine seed. Pre-roll RNG into a
   `Vec<u64>` of length N BEFORE entering a `par_iter` block; workers index by
   stable entity id.
3. **No `HashMap` with default `RandomState` hasher in sim state.** Use
   `BTreeMap` (deterministic iteration) OR `FxHashMap` (only when iteration
   order does not cross the checksum boundary). Audit every cf-* crate.
4. **No FMA / wide AVX-512 intrinsics that vary by hardware.** Use the stable
   f32 code path. Workspace `RUSTFLAGS` includes `-C target-feature=+sse2,+sse4.2`
   on x86_64; LLVM `-ffast-math` is disabled.
5. **No `Instant::now()` / `SystemTime::now()` in sim code.** Real time is
   non-deterministic. Use the engine's `Clock::tick().0` for any time-like
   value.
6. **No `format!()` / `String::from()` allocations in tick-hot paths.** Use
   `tracing::Span` with structured fields.
7. **No `Vec::new()` / `HashMap::new()` per tick in hot paths.** Pre-allocate
   in the relevant ECS resource at engine init. Clear-and-reuse.
8. **Recorder events crossing the determinism boundary MUST have stable
   `event_id`s.** Per-shard `event_id = (tick, shard_id, monotonic_seq)`
   re-stamps to canonical `(tick, canonical_seq)` at merge time.
9. **Cross-thread state mutation MUST be either (a) per-worker buffer with
   deterministic merge, or (b) explicit single-threaded post-pass.**
   Atomic-CAS loops on shared sim state are FORBIDDEN.
10. **No `std::sync::Mutex` in sim hot paths.** Use Bevy ECS query system for
    lock-free disjoint access. Allowed: `std::sync::Mutex` outside the sim
    island (e.g. cf-audio plugin, cf-control's audio_plugin field).

### Smart-AI 4.0 ms p99 budget (added 5/15/2026)

M8A retunes the AI sim CPU budget from 2.0 ms p99 to **4.0 ms p99** to cover
the M7-shipped 5-layer thinking stack (Reactive + Utility + Behavior Tree +
HTN + LLM-prior), 16 KB BotMemory per bot, 22-task PriorityTable, and
deterministic reason-label structured strings.

Per-bot 80 µs effective parallel slice covers:

- Perception update + memory grid write (15-20 µs)
- Memory aging + event ingestion (5-10 µs)
- Utility scoring × 22 candidate tasks × priority weight multiply (20-25 µs)
- Behavior tree traversal (10-15 µs; 3-5 nodes deep)
- HTN sub-goal evaluation (5-10 µs; cached unless goal changes)
- Reason-label structured-string construction (5-10 µs)
- Chatter selection (2-5 µs; lookup + cooldown check)

LLM mind layer (M23, optional) runs out-of-band in a separate process at 5 s
tick per bot. Sim sees only the cached doctrine string. 0% sim cost when LLM
disabled. The total per-tick budget remains ≤ 15.5 ms (under the 16.6 ms 60 Hz
wall budget).

### Recorder shard merge contract

Per-thread `RecorderShard`s emit events with `event_id = (tick,
monotonic_seq_in_shard)`. At the `RecorderMerge` stage:

1. Each shard sorts its events by `monotonic_seq_in_shard` (already in
   insertion order).
2. Cross-shard merge orders events by `(tick, shard_id,
   monotonic_seq_in_shard)`. `shard_id` is a stable per-system-stage
   assignment, NOT the runtime thread id.
3. After merge, every event is re-stamped with a canonical `event_id = (tick,
   canonical_seq)` where `canonical_seq` is the post-merge position.
4. `parent_event_id` references are re-mapped from shard-local ids to
   canonical ids during merge.

This preserves determinism across thread-scheduling variance.

### CI gate

The `game/scripts/ci/m8a_determinism_lint.sh` script enforces these rules:

- Runs `cargo clippy --workspace --all-targets -- -D warnings` (the existing
  clippy gate already rejects `rand::thread_rng`, `Instant::now`,
  `SystemTime::now` via `clippy.toml`'s `disallowed-methods` list).
- Greps each sim crate (`cf-sim-core`, `cf-actor`, `cf-ai`, `cf-physics`,
  `cf-material`, `cf-terrain`, `cf-atmos`) for `f64` outside the documented
  boundary-use lines; exits non-zero on hit.
- Greps for `std::sync::Mutex` and `HashMap::new` in sim hot paths.

The gate is invoked from `game/scripts/ci/m8a_close_gates.sh` as the first
check before the perf gate / cross-OS gate / backfill gate.
