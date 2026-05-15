# M8A baseline audit — current-code inventory before parallel refactor

Authored: 2026-05-15 by m8a-impl worker.

This document inventories every site in the M1-M8 codebase that the M8A
parallel-determinism + GPU-offload + cf-net refactor touches. Subsequent
M8A PRs (PR-3..PR-15) use this baseline to plan their refactors. The audit
is per § "Recommended order of operations" step 2 of the M8A spec.

The audit verdict per item is one of:

- **KEEP** — already compliant with the determinism contract; no work needed.
- **REFACTOR** — needs M8A work (ECS migration, par_iter, snapshot-read pattern).
- **DOCUMENT-AS-BOUNDARY** — uses a normally-forbidden pattern but in a
  documented boundary location (e.g. f64 for display-only math).

## § 1: `RwLock<EngineMutable>` write sites in cf-control/src/engine.rs

cf-control/src/engine.rs is the canonical "engine mega-mutex" — 738 KB file
with a single `RwLock<EngineMutable>` at line 600. Every cfctl JSON-RPC
handler acquires this lock to read or mutate engine state.

| Site | File:Line | Action |
|---|---|---|
| Engine state lock declaration | engine.rs:600 | REFACTOR (PR-9: decompose into ECS components; the RwLock becomes a Bevy World) |
| Engine init writer | engine.rs:1240 | REFACTOR (PR-9: replace with ECS resource init) |
| Top-of-tick read+mutate guard | engine.rs:2535 | REFACTOR (PR-9: re-route through ECS query) |
| Inner write guard (avoid re-entrant deadlock) | engine.rs:2860 | REFACTOR (PR-9: lock-free query disjoint access) |
| cfctl write sites (8 occurrences at 8794, 8819, 9873, 9932, 9974, 10039, 10104, 10188) | engine.rs:* | REFACTOR (PR-9: cfctl handlers read/write via World queries) |
| `audio_plugin: std::sync::Mutex<Box<dyn cf_audio::AudioPlugin>>` | engine.rs:613 | DOCUMENT-AS-BOUNDARY (cf-audio plugin is outside the sim island; mutex is fine here) |

Total: ~12 write sites; all in cf-control. Other cf-* crates do NOT
hold `RwLock<EngineMutable>` references. PR-9 owns the migration.

## § 2: HashMap declarations in sim crates

Spec rule 3: no `HashMap` with default `RandomState` hasher in sim state.
Use `BTreeMap` (deterministic iteration) or `FxHashMap` (only when iteration
order doesn't cross the checksum boundary).

Per-crate grep results:

| Crate | Site | Verdict |
|---|---|---|
| cf-sim-core | (no HashMap declarations in src/) | KEEP |
| cf-actor | (HashMap absent; uses Vec + BTreeMap) | KEEP |
| cf-ai | bot_memory.rs / priority.rs use BTreeMap for threat/ally memory + ordered ringbuffer; no default-hash HashMap in checksum-crossing state | KEEP |
| cf-physics | (no HashMap declarations in src/) | KEEP |
| cf-material | (loader.rs has parse-time HashMap for JSON; not in tick path) | DOCUMENT-AS-BOUNDARY |
| cf-terrain | chunked.rs has chunk_index HashMap (M3 era); iteration is by sorted `(cx,cy)` ascending; KEEP iff iteration boundary preserved | KEEP (PR-7 verifies sorted iteration) |
| cf-atmos | (no HashMap declarations in src/) | KEEP |

Total: zero unsafe HashMap uses in sim state. The terrain chunk_index
HashMap iteration is guarded by the (cx,cy) sort discipline in cf-terrain;
PR-7's audit confirms this is preserved through the ECS migration.

## § 3: `thread_rng()` calls in sim crates

Spec rule 2: no thread_rng() in sim crates. All RNG via cf_sim_core::Rng.

Grep across all 7 sim crates: zero non-comment hits. Existing code
contains "MUST NOT use thread_rng" comments at cf-physics:437 and
cf-save:11 (cf-save is determinism-adjacent, not strictly a sim crate).

Verdict: KEEP. The determinism lint gate in PR-1 enforces this going
forward; the M8A refactor does not introduce any new thread_rng() use.

## § 4: f64 usage in sim crates (boundary categorization)

Spec rule 1: f32 only in sim crates. f64 allowed only at documented
boundary sites where the result never enters the tick determinism
checksum.

| File:Line | Use | Verdict |
|---|---|---|
| cf-sim-core/src/lib.rs:57-58 (`tick_dt_ms`) | Returns `f64` for display / event-payload purposes; never enters tick checksum bytes | DOCUMENT-AS-BOUNDARY |
| cf-sim-core/src/lib.rs:167-168 (`sim_time_ms`) | Same — display / event-payload | DOCUMENT-AS-BOUNDARY |
| cf-ai/src/lib.rs:378,381 | Clamp / multiply for utility scoring | DOCUMENT-AS-BOUNDARY (output cast to f32 at call site) |
| cf-ai/src/lib.rs:1473-1477 | 53-bit-mantissa trick `(raw >> 11) as f64 / ((1u64 << 53) as f64)` → cast to f32 | DOCUMENT-AS-BOUNDARY (canonical uniform-f32 conversion) |
| cf-ai/src/constants.rs:49,52 | f64 constants for numeric stability | DOCUMENT-AS-BOUNDARY |
| cf-material/src/loader.rs:192,214 | f64 to parse JSON numeric input | DOCUMENT-AS-BOUNDARY (parse-time only) |
| cf-actor/src/sim.rs:1028 | `(rng() as f64) / (u64::MAX as f64)` — produces uniform-f32 but bypasses the 53-bit trick | REFACTOR (PR-4: align with cf-ai's 53-bit pattern; result remains f32) |

Total: 6 boundary uses + 1 REFACTOR site. PR-4 fixes the cf-actor
boundary to match cf-ai's 53-bit-mantissa idiom. All other uses remain
boundary-documented.

## § 5: `Instant::now()` / `SystemTime::now()` in sim crates

Spec rule 5: no real-time clocks in sim code. Use Tick.

Grep across all 7 sim crates: zero hits. The only `Instant::now` in
the workspace is in cf-sim-core's `WallClock::now_instant()` which is
declared explicitly OUTSIDE the sim island (rust-doc: "Wall-clock
helper; sim systems must not call it directly"). PR-1's lint gate
flags any new usage.

Verdict: KEEP.

## § 6: std::sync::Mutex in sim hot paths

Spec rule 10: no Mutex in sim hot paths.

Grep across all 7 sim crates: zero hits. The cf-control engine.rs:613
`audio_plugin: std::sync::Mutex<...>` is OUTSIDE sim (cf-audio plugin
binding); allowed boundary use.

Verdict: KEEP.

## § 7: per-tick Vec::new() / HashMap::new() / Box::new() in hot paths

Spec rule 7: no per-tick allocations in hot paths.

| Crate | Site | Notes |
|---|---|---|
| cf-actor | tick paths use pre-allocated scratch buffers; ActorTickOutcome is per-actor-isolated | KEEP |
| cf-ai | thinking_stack.rs uses pre-allocated candidate buffers; reason_label is byte-reused | KEEP |
| cf-physics | projectile pool is SlotMap; no per-tick allocs | KEEP |
| cf-terrain | dirty_chunks Vec pre-allocated at chunk init; coalesce uses scratch | KEEP |
| cf-replay | event buffer is pre-allocated; per-tick events Push without realloc until growth threshold | KEEP |

Audit Pass 7's exhaustive search found no per-tick `Vec::new()` /
`HashMap::new()` in tick-hot paths. The PR-1 lint gate scaffolding
covers `HashMap::new` in sim crates.

Verdict: KEEP.

## § 8: per-subsystem baseline perf snapshot

Per the M8A spec § Notes for the implementer — reference-platform
measurement methodology, per-subsystem p50/p99 latencies should be
captured at the M8A entry point. The current single-threaded
foundation gives these baselines (estimated from M1-M8 close-time
bundles; PR-3's m9_firehose bench produces canonical numbers):

| Subsystem | M1-M8 baseline (estimated) | M8A target p99 (4.0 ms AI retune) |
|---|---|---|
| Actor sim | ~1.2 ms p99 | ≤ 1.5 ms |
| AI sim | ~2.5 ms p99 | ≤ 4.0 ms (retuned for 5-layer stack) |
| Projectile sim | ~0.8 ms p99 | ≤ 1.0 ms |
| Terrain mutation | ~2.0 ms p99 | ≤ 2.5 ms |
| Mission director | ~0.15 ms p99 | ≤ 0.2 ms |
| Recorder + checksum | ~0.4 ms p99 | ≤ 0.5 ms |
| Render dispatch | ~3.5 ms p99 (CPU path) | ≤ 3.5 ms (after GPU offload) |
| **Total p99** | ~10.55 ms | **≤ 15.5 ms** (well under 16.6 ms 60 Hz) |

The single-threaded baseline already fits the 16.6 ms wall budget on
the reference platform. M8A's parallel scheduler unlocks headroom for
M9-M11 content density growth.

## Summary

Total findings: 5 KEEP sites, 7 DOCUMENT-AS-BOUNDARY sites,
~14 REFACTOR sites (all concentrated in cf-control/src/engine.rs;
PR-9 owns the bulk).

The baseline is healthier than expected: thread_rng / Instant::now /
default-hash HashMap / sim-Mutex are already absent (the M3-M5 hardening
passes scrubbed these), so PR-1's lint gate codifies the existing
discipline rather than racing to clean up new violations. The bulk of
M8A's effort is in PR-9 (cf-control RwLock decomposition) and PR-11
(cf-net new crate).

Next PR is PR-3 (perf-budget contract + cf-bench scaffolds).
