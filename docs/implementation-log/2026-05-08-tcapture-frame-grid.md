# T-CAPTURE — Frame Capture, Grid Composer, And BP Fun-Proof Automation (2026-05-08)

> [!summary]
> T-CAPTURE side track shipped on PR #6 (squash commit [`064c0a0`](https://github.com/Madreag/corefall/commit/064c0a0)). Closes the last gap in the Eyes/Ears/Hands rule by giving AI agents a way to **see** motion + physics + effects through PNG frame readbacks composed into LLM-readable grid images, on top of the existing JSON observe + cfctl input + cf-e2e `--expect` surface.

## Why this exists

The Eyes/Ears/Hands rule already gives an AI worker JSON observation + cfctl input + cf-e2e assertions, but JSON cannot prove a projectile *looked* like an arc, a breach strip *visibly* degraded, or an enemy AI *visibly* engaged. Per BP1 closure → BP2 setup, the per-BP human-playtest gate needed a visual proof artifact AI agents could read directly. T-CAPTURE adds that pipeline.

## What ships

### `cf-capture` crate (workspace member 30)

Owned API surface:

- `CfCapturePlugin` (Bevy plugin)
- `CaptureConfig`, `CaptureState`, `CaptureClock`, `CaptureMode`
- `CaptureKeyframeRequested` (Bevy `Message`)
- `CaptureFrameEntry`, `CaptureManifest`, `CaptureStateHandle`
- `write_capture_manifest_from_handle(...)`, `ensure_capture_dir(...)`, `frame_filename(...)`
- Constants: `DEFAULT_FRAMES_HZ = 10.0`, `DEFAULT_THUMBNAIL_W = 320`, `DEFAULT_THUMBNAIL_H = 180`, `SUMMARY_GRID_MAX_FRAMES = 64`, `COMPOSER_SCHEMA_REV = 1`

Tests (10): interval math at 60/120 Hz, NaN/Inf/negative/zero `frames_hz` returning `u64::MAX`, filename padding, manifest round-trip.

### `cf-app` flag set + reject guard

- `--capture-grid` (bool)
- `--capture-frames-hz <N>` (default 10.0)
- `--no-capture-events` (suppress event-triggered keyframes)
- `--headless-capture` (scope-limited; logs warning)
- `--headless-smoke + --capture-grid` rejected at startup with explanatory error

System wiring:

- `CfCapturePlugin` registered with `CaptureConfig` + `CaptureStateHandle` (the handle survives `Drop` so cf-app can write the manifest after `app.run()`).
- `sync_engine_tick_to_capture_clock` mirrors `engine.current_tick()` into `CaptureClock` each `Update`.
- `pump_recorder_events_into_capture_keyframes` reads new recorder events via `Recorder::events_since(after_idx)`, advances cursor by `new_events.len()` (single lock, no TOCTOU), and emits `CaptureKeyframeRequested` messages for any event whose `(category, event_type)` tuple is in `CAPTURE_KEYFRAME_EVENT_TYPES`.

Tests added: 3 (rejects combo, allows capture-only, allows headless-only).

### `game/tools/capture_grid.py` composer (Pillow-based)

- Reads `capture_manifest.json`.
- Downsamples PNGs to 320×180 thumbnails.
- Composes 8×8 `grid_NNN.png` files with tick + event-label overlays burned in.
- Emits `summary_grid.png` (one frame per major event, max 64 frames) for high-level agent review.
- Per-grid metadata in `grid_NNN.json` + `summary_grid.json`.
- Records `non_blank_ratio` for black-frame regression detection.
- Supports `--dry-run` for manifest validation without writing.

### `cf-e2e --capture-grid` flag set + composer invocation

- `--capture-grid`, `--capture-frames-hz`, `--no-capture-events`, `--composer-script`, `--python-bin`
- `LaunchOptions` struct threads capture flags into the spawn args.
- When `--capture-grid` is set, drops `--headless-smoke` automatically + force-issues a final `observe.once` to recover the engine `run_id` + sleeps 250 ms for screenshot observers to flush + invokes the composer + merges JSON output into the observation under the `capture` key.
- New `key>=value` and `key<=value` operators on `--expect` so capture thresholds (`capture.summary_grid.non_blank_ratio>=0.95`) resolve through the same lookup path as `mission.result=won`.

### `cf-replay::Recorder::events_since(after_idx)`

New helper exposing newly-recorded events without cloning the full event log. Used by cf-app's keyframe pump.

## Cadence policy

- **Default baseline:** 10 Hz (every 6 ticks at 60 Hz / every 12 ticks at 120 Hz). Configurable.
- **Event-triggered keyframes (always-on unless `--no-capture-events`):** `mission.objective_started/completed/failed/mission_resolved`, `terrain.terrain_carved/tool_refused`, `combat.projectile_hit`, `actor.actor_status_changed`, `equipment.weapon_fired`, `ai.state_changed`, `system.panic`. Matched by full `(category, event_type)` tuple.
- **Per-grid layout:** 8 × 8 = 64 frames per `grid_NNN.png`; longer runs produce sequential grids; `summary_grid.png` always present.

## Determinism contract

- Capture path is read-only against the engine. **Does NOT** mutate sim state, RNG, or recorder ordering.
- Grid composition is deterministic given the same captures + composer version + overlay schema.
- Capture cadence computed from runtime tick rate, so 10 Hz works the same at 60 Hz and 120 Hz.

## Acceptance evidence

`prototype_runs/native/m1_2026-05-08T03-30-23Z_5703728c`:

- Bundle PASSES canonical checker (`errors 0`, 320 events, 1 test).
- 50 PNG frames captured at 10 Hz over 5 s.
- `capture_manifest.json` schema_rev=1; runtime_tick_rate_hz=60; mode=Windowed.
- `grid_001.png` composed with 50 frames in 8×7 layout, tick overlays visible in each cell.
- `summary_grid.png` composed; `non_blank_ratio: 0.98` (49 of 50 frames have visible content; one initial pre-render frame is black as expected).
- `cf-e2e --capture-grid --expect capture.summary_grid.non_blank_ratio>=0.95` would PASS.

## Bugbot loop summary (PR #6)

| # | Finding (severity) | Resolution |
|---|---|---|
| 1 | NaN/Infinity `frames_hz` bypasses disable guard (Low) | `is_finite()` + 4 regression tests |
| 2 | `seconds_to_ticks(0.0) = 1` silent delay (Low) | explicit 0.0 → 0 path |
| 3 | Recorder cursor TOCTOU race (Low) | `cursor.0 += new_events.len()` (single lock) |
| 4 | `alert_dwell` + `burst_pause` off-by-one (Low) | pre-decrement `prev_*` capture pattern + 2 regression tests |
| 5 | HudBreach material from wrong field (Medium) | `BreachRenderView.material` field added |
| 6 | `CaptureSystems` SystemSet ordering (Medium) | run AFTER clock sync + keyframe pump |
| 7 | Keyframe matching too broad (Low) | `(category, event_type)` tuple match |
| 8 | `--headless-smoke + --capture-grid` silent drop (Medium) | hard-reject at startup + 3 regression tests |

**Stale unresolved comments at merge:** 1 (about already-removed `event_log_len` helper, `isOutdated: true`).
**Live unresolved at merge:** 0.

## Final test count

- M0: 73
- M1: 86
- M1.5: 30
- T-CAPTURE: 15 (cf-capture 10 + cf-ai dwell/pause regressions + cf-app reject_capture_grid_with_headless_smoke + 3 capture cf-app tests)
- **Total: 204 tests passing**

## BP closure-gate role

- Every fun-proof scenario in BP2..BP12 must emit a `summary_grid.png` artifact, recorded in `summary.json.artifacts` with `type: capture-summary-grid`.
- The cf-e2e script must include `--expect capture.summary_grid.non_blank_ratio>=0.95` to catch black-frame regressions.
- `/corefall-review <bp>` reads the summary grid when issuing the BP-level Accept verdict.
- Per-BP human-playtest survey row in `prototype_runs/native/<bp>_*/notes.md` MUST reference the summary grid path it was answered against.

## Vault updates landed in same pass

- `cortext_command_vault/spec/prototype-roadmap.md` — new T-CAPTURE row in side-tracks summary table + new `### T-CAPTURE` section after T-PERF.
- `cortext_command_vault/references/prototype-run-bundle-schema.md` — `captures/*` row family added with `summary.json.artifacts[].type` values.
- `cortext_command_vault/spec/feature-completion-checklist.md` — new `### T-CAPTURE` section with 12 rows (5 S, 3 D, 4 O).
- `cortext_command_vault/prototypes/index.md` — new T-CAPTURE row pointing to evidence note.
- `cortext_command_vault/prototypes/native-tcapture-frame-grid.md` — new evidence note (this file's vault counterpart).

## Out-of-scope (explicit follow-ups)

Tracked as `T-CAPTURE-O01..O04` rows in feature-completion-checklist.md:

- O01: Animated WebP timeline export alongside PNG grid.
- O02: Side-by-side replay-vs-live diff grid for regression detection.
- O03: AI-readable `summary_grid.events.json` co-located with the grid.
- O04: True headless (offscreen RenderTarget) readback for `--headless-capture`.
