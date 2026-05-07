# 2026-05-07 Bevy 0.18.1 Migration

## Root Cause

The planning vault now requires the project to stay on the latest verified Bevy release and pin it explicitly. The implementation repo was still pinned to Bevy 0.14, which made the code and roadmap diverge.

## Changes

- Updated `game/Cargo.toml` to exact-pin `bevy = "=0.18.1"`.
- Replaced the removed Bevy `zstd` feature with `zstd_rust`.
- Made `bevy_log` and `default_font` explicit because default Bevy features remain disabled.
- Updated workspace `rust-version` to `1.93` to match the pinned toolchain posture and exceed Bevy 0.18.1's MSRV.
- Migrated render/UI call sites from old bundles to Bevy 0.18 component-style APIs:
  - `Camera2dBundle` -> `Camera2d`.
  - `SpriteBundle` -> `Sprite` + `Transform` + optional `Visibility`.
  - `NodeBundle` / `TextBundle` / `Style` / `TextStyle` -> `Node`, `Text`, `TextFont`, `TextColor`, `BackgroundColor`.
  - `EventReader` / `EventWriter` -> `MessageReader` / `MessageWriter`.
- Updated Bevy version fallback metadata in `cf-control`.

## Validation

- `cargo update -p bevy --precise 0.18.1`: PASS.
- `cargo check --workspace --all-targets`: PASS.

Full M1 standard validation should run after this migration is committed with the rest of the M1 closeout loop.
