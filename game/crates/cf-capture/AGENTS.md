# cf-capture — boundary contract

## Owns

- `CfCapturePlugin`, `CaptureConfig`, `CaptureState`, `CaptureClock`, `CaptureMode`,
  `CaptureKeyframeRequested`, `CaptureFrameEntry`, `CaptureFrameKind`, `CaptureManifest`.
- The frame-capture pipeline that takes per-tick PNG readbacks at a configurable
  baseline cadence (10 Hz default) plus event-triggered keyframes, and writes
  `captures/frame_<index>_t<tick>.png` + `captures/capture_manifest.json` next to
  each run-bundle.
- `frame_filename(...)`, `ensure_capture_dir(...)`,
  `write_capture_manifest_from_handle(...)` helpers.
- The `COMPOSER_SCHEMA_REV` constant + the `capture_manifest.json` shape that
  `game/tools/capture_grid.py` reads.

## Public API Boundary

- Stable: `CfCapturePlugin`, `CaptureConfig`, `CaptureState`, `CaptureClock`,
  `CaptureKeyframeRequested`, `CaptureMode`, `CaptureManifest`,
  `CaptureStateHandle`, `write_capture_manifest_from_handle`,
  `ensure_capture_dir`, `frame_filename`.
- Internal: per-frame Bevy systems (`capture_baseline_system`,
  `capture_keyframe_system`); subject to change without notice.
- Schema: `capture_manifest.json` is consumed by `game/tools/capture_grid.py`.
  Bumping `COMPOSER_SCHEMA_REV` requires updating the composer in the same pass.

## Does NOT Own

- The Bevy `Screenshot` API (lives in `bevy::render::view::screenshot`).
- The grid composition step (lives in `game/tools/capture_grid.py`; written in
  Python so artists/researchers can iterate without recompiling Rust).
- Run-bundle metadata / `summary.json` shape (owned by `cf-replay`).
- The cf-control event recorder (cf-capture only consumes
  `recorder.events_since(idx)` to emit `CaptureKeyframeRequested` messages from
  cf-app).
- Sim-authoritative state: the capture path is read-only against
  `ObserveFrame`-equivalent surfaces; it MUST NOT mutate sim state.

## Test Surface

- 6 unit tests in `src/lib.rs`:
  - `baseline_interval_at_60hz_default_returns_six_ticks`
  - `baseline_interval_at_120hz_default_returns_twelve_ticks`
  - `baseline_interval_at_120hz_60hz_capture_returns_two_ticks`
  - `baseline_interval_zero_hz_returns_max_to_disable`
  - `frame_filename_pads_index_and_tick`
  - `capture_manifest_round_trip`
- End-to-end via `cf-e2e --capture-grid`; emits real PNGs + grid composition
  artifacts; smoke tests live in `cf-e2e` and the canonical Standard Validation.

## Cross-Crate Contracts

- **cf-app** owns the wiring: it instantiates `CfCapturePlugin` with a
  `CaptureConfig`, mirrors the engine's tick into `CaptureClock` each `Update`,
  and pumps recorder events through `MessageWriter<CaptureKeyframeRequested>`
  for the named M1.5+ event types.
- **cf-replay** exposes `Recorder::events_since(idx) -> Vec<Event>` so cf-app
  can drive keyframes without cloning the full event log every frame.
- **cf-e2e** invokes `python3 game/tools/capture_grid.py <run_dir>` after the
  script run completes, parses the JSON output, and merges it into the
  observation under the `capture` key so `--expect capture.summary_grid.non_blank_ratio>=0.95`
  resolves through the same lookup path as `mission.result=won`.
- **T-CAPTURE** roadmap track in
  `cortext_command_vault/spec/prototype-roadmap.md` owns the policy: cadence
  defaults, keyframe types, BP closure-gate role, LLM input contract.

## Common Pitfalls

- **Do not capture from sim systems.** The Bevy `Screenshot` component must be
  spawned from a render-phase system (`Update` is fine; `FixedUpdate` is not).
  Sim-authoritative state cannot be read or written from the capture path.
- **Headless mode is scope-limited today.** Passing `--headless-capture` /
  `CaptureMode::OffscreenImage` logs a warning and skips the actual PNG spawn
  rather than emitting empty frames that would corrupt the grid composer's
  `non_blank_ratio` assertion. The offscreen-RenderTarget readback ships in a
  follow-up commit per the T-CAPTURE done-criteria.
- **PNGs land asynchronously.** `Screenshot::primary_window().observe(save_to_disk(...))`
  returns immediately; the actual PNG hits disk one or two frames later. cf-e2e
  sleeps ~250 ms after the run before invoking the composer to avoid racing the
  observers.
- **Capture cadence is computed from the runtime tick rate.** Setting
  `capture_frames_hz=10` at a 120 Hz tick rate produces every-12-tick captures
  (not every-6). The composer's overlay records the runtime tick rate so the
  agent can spot the discrepancy.
- **Determinism.** The capture path must not perturb sim state, RNG, or the
  recorder ordering. The `pump_recorder_events_into_capture_keyframes` system
  in cf-app calls `recorder.events_since(idx)` (read-only) and never writes
  back. If a future enhancement wants to mark frames in the recorder, that
  goes through a new event type, not by mutating existing events.

## Source Trail

- `cortext_command_vault/spec/prototype-roadmap.md` §T-CAPTURE — owns the policy.
- `cortext_command_vault/references/prototype-run-bundle-schema.md` —
  `captures/{frame_*.png, grid_NNN.png, summary_grid.png, capture_manifest.json}`
  contract.
- `game/tools/capture_grid.py` — composer (Python; Pillow-based).
- `cf-e2e --capture-grid` — entry point for AI-agent self-testing.
- `cf-app --capture-grid --capture-frames-hz <N> --no-capture-events` — flags.
- BP2 closure-gate row in `feature-completion-checklist.md` — "T-CAPTURE
  evidence" is mandatory for the M2.5 micro-fun-slice.
