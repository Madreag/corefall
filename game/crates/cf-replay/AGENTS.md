# cf-replay — AGENTS.md

## Owns
- DR-002 v1 event envelope (`Event`, schema string `prototype-recorder-event.v0.1`).
- `Recorder` (append-friendly with severity counters + dropped counters + checksum tracking).
- Run-bundle writer (`write_run_bundle`) producing `run_manifest.json`/`events.jsonl`/`summary.json`/`notes.md`.
- Manifest/summary types (`prototype-run-manifest.v0.1`, `prototype-run-summary.v0.1`) with the M0 lock additions:
  - `manifest.checksum.{algorithm, scope, cadence_ticks}`
  - `manifest.settings.{ui_scale,high_contrast,captions,reduced_motion,reduced_shake,reduced_flash}`
  - `summary.{final_sim_checksum, checksum_event_count, first_tick, last_tick}`
- Shared `diagnostics::init`/`set_panic_reporter` (M0-008 panic hook + tracing init).

## Public API Boundary
- `Event`, `RunManifest`, `RunSummary`, `BundleInputs`, `BundleError`, `Recorder`, `write_run_bundle`.
- Schema-version constants (`MANIFEST_SCHEMA_VERSION`, `EVENT_SCHEMA_VERSION`, `SUMMARY_SCHEMA_VERSION`).
- `diagnostics::{init, set_panic_reporter}`.

## Does NOT Own
- The actual sim loop / clock → `cf-sim-core`.
- Control envelope / WebSocket → `cf-control`.
- Snapshot category payloads (open at M3) → cross-crate via `category = "snapshot"`.

## Test Surface
- Unit tests: `cargo test -p cf-replay` (envelope roundtrip + bundle write basics).
- Required event categories in run bundles produced by this crate: depends on caller (M0 emits `system`, `control`, `determinism`).

## Cross-Crate Contracts
- Depends on: `cf-sim-core`.
- Depended on by: `cf-control`, `cf-app`, `cfctl`, every binary's diagnostics init.
- Event names this crate emits: none directly; consumers call `Recorder::record(...)`.

## Common Pitfalls
- The run-bundle checker (`research_tools/prototype_run_check.py`) enforces version STRINGS, not integers. Don't accept `schema_version: 1` from anywhere.
- Every event must have a unique `event_id` of the form `<run_id>:<tick>:<seq>` and ticks must be monotonic.
- `summary.event_counts.by_category`/`by_type` MUST exactly match the events; the checker compares them.
- `tests[*].evidence_event_ids` MUST reference real event ids that exist in `events.jsonl`.
- `notes.md` MUST contain `## Assumptions Tested`, `## Good`, `## Bad`, `## Meh`, `## Evidence Links`, `## Next Actions`.

## Source Trail
- DR-002 (replay/event architecture, OPEN — closes at M3).
- references/prototype-run-bundle-schema.
- research_tools/prototype_run_check.py.
- docs/implementation-log/2026-05-05-m0-engine-bootstrap.md.
