# M3A — Cross-OS Determinism Floor

## Status

`done`

## Intent

**M3A is the cross-OS determinism floor milestone** — the verification + CI gate that the Corefall simulation produces **byte-identical event streams** on `x86_64-linux` + `x86_64-windows-msvc` + `aarch64-darwin` for the same seeded inputs. This is the load-bearing invariant every other determinism-sensitive spec (M4 event recorder, M4B save versioning, M8A parallel ECS, M8B QUIC frame layout, M10 replay viewer, M10B replay-as-MP4 export, M36D GPU parity, M41 cross-platform replay) reads as a precondition.

M3A's work shipped incrementally across M3 + M5 + M8 + M8A — this spec file exists as the canonical record of the cross-OS contract that all downstream specs reference as `M3A (done)`.

## What shipped under M3A

- **Pinned numeric crate versions** — `rand_chacha` (ChaCha20-PRNG; bit-deterministic across platforms), `ordered-float`, `serde_json` with stable map ordering, `blake3` (SIMD path produces identical bytes on every supported ISA).
- **Banned non-deterministic primitives in sim crates** — no `thread_rng`, no `std::time::Instant`-derived seeds, no `HashMap` (replaced with `BTreeMap` for sortable iteration), no `f32`/`f64` non-IEEE-754 fast-math paths.
- **Canonical-JSON encoder** — every save / event / snapshot is encoded via a deterministic serializer that produces the same byte sequence regardless of host endianness or hashmap insertion order.
- **Per-tick blake3 `sim_checksum`** — recorded into every event bundle; verified across OS runs as the cross-OS gate.
- **CI matrix** — `cargo test --workspace` on `x86_64-linux` + `x86_64-windows-msvc` + `aarch64-darwin` runs the same 30-minute deterministic-replay corpus on each platform and asserts byte-identical event streams + identical per-tick `sim_checksum`.
- **WGSL shader portability** — material CA + atmospherics kernels compile to byte-equivalent SPIR-V / MSL / DXIL via `wgpu::naga` pinned version (consumed deep by M15B + M36D for the GPU parity gate).
- **CRLF/LF + path-separator normalization** — content files normalize line endings + use POSIX-style internal paths so save bundles produced on Windows replay byte-identically on Linux.

## Acceptance criteria (closed)

```gherkin
Scenario: Same seeded 30-minute mission replays byte-identical across 3 OS
  Given the `cross_os_corpus.cfreplay` bundle (30-minute mission, seeded)
  When the replay runs on x86_64-linux + x86_64-windows-msvc + aarch64-darwin
  Then the per-tick blake3 sim_checksum is byte-identical across all 3 platforms
  And every event bundle's canonical-JSON encoding is byte-identical

Scenario: CI gates on cross-OS divergence
  When a PR introduces a non-deterministic primitive (thread_rng / HashMap iter / time-based seed) in any cf-sim-* crate
  Then the cross-OS CI matrix fails the PR
  And the linter rule (banned-primitives) names the offending file + line

Scenario: WGSL shader compiles byte-equivalent across platforms
  Given a WGSL material kernel
  When compiled via wgpu::naga on Linux + Windows + macOS
  Then the resulting SPIR-V / MSL / DXIL produces identical material CA output for the same input snapshot
```

## Crates touched (already shipped)

- `cf-sim-core` — banned primitives lint
- `cf-replay` — per-tick `sim_checksum` + canonical-JSON
- `cf-save` — endianness + path-separator normalization
- `cf-material` + `cf-physics` + `cf-atmos` + `cf-ai` + `cf-terrain` — pure-determinism enforcement

## Dependencies

- M3 (done) — pixel terrain + materials baseline
- M5 (done) — actor + chassis + equipment baseline
- M8 + M8A (done) — parallel determinism + ECS scheduler

## Consumers (downstream specs that reference M3A as a done precondition)

M4, M4B, M8A, M8B, M10, M10B, M15B, M23D, M36C, M36D, M36G, M40, M40A, M41, M41B, M41C, M49 — all require M3A's cross-OS contract to hold before their own gates can be asserted.

## Notes

This spec was authored retrospectively to document the cross-OS contract that shipped incrementally across M3 + M5 + M8 + M8A. The README has tracked M3A as a `✅` milestone since the initial badge introduction; this file makes the dependency graph complete (no `(done)` reference points to a missing file).
