# cf-tools-replay-viewer — AGENTS.md

## Owns
- M3B-001 viewer shell: loads a run bundle (`run_manifest.json` + `events.jsonl` + `summary.json`), exposes event tail / category filter / tick scrubber / pause-step state, and renders a deterministic markdown view of the bundle at any tick.
- M3B-002 cause-chain view: walks `parent_event_id` chains backwards from terminal events (`actor_died`, `mission_resolved`, `objective_failed`, `reactor_damaged` with `destroyed=true`, `terrain_carved` near breach proxies, `projectile_hit`) and renders the chain as markdown.
- M3B-003 debrief summary: composes outcome (mission_resolved.result + reason), objectives (objective_started / objective_failed / objective_completed), key events (configurable category mix, defaults to combat + mission + terrain), damage / death recap (actor_died / reactor_damaged accumulation), terrain changes (terrain_carved + chunk_dirtied counts + total carved pixels), and checksum status (final_sim_checksum + checksum_event_count + first_tick / last_tick).
- Corrupt-bundle rejection: typed `BundleError` variants for missing files, malformed JSON, mismatched run_ids between manifest+summary, schema-version mismatch, broken event chain (parent_event_id that doesn't resolve), or out-of-order ticks.

## Public API Boundary
- Library `cf_tools_replay_viewer` exposing: `bundle::Bundle::load`, `viewer::ViewerState` + `viewer::render_markdown`, `cause_chain::trace` + `cause_chain::render_markdown`, `debrief::compose` + `debrief::render_markdown`, `BundleError`.
- Binary `cf-tools-replay-viewer` with subcommands: `view`, `cause-chain`, `debrief`, `validate`.
- Markdown is the canonical output format. JSON output is exposed via `--json` for tooling.

## Does NOT Own
- The event recorder / run-bundle writer → `cf-replay`.
- The headless replay verifier → `cf-headless`.
- Live engine / sim loop → `cf-control` + `cf-sim-core`.
- Polished GUI (egui/bevy) — explicitly out of scope per M3B anti-scope ("No polished replay browser"). The library + markdown-output binary IS the viewer; future BPs may layer a GUI on top of the same library.

## Test Surface
- `cargo test -p cf-tools-replay-viewer` covers: bundle loading + corrupt-bundle rejection (missing files, bad JSON, mismatched run_id, broken parent chain), viewer tail/filter/scrub determinism (golden markdown snapshots from a synthetic 32-event fixture), cause-chain construction (synthetic actor_died chain, real M2.5 reactor_destroyed chain when fixture available, max-depth cap), and debrief composition (synthetic win + loss + reactor-destroyed bundles).
- Self-play sweep row `m3b_replay_viewer_smoke`: runs `cargo run -p cf-tools-replay-viewer -- debrief <m2.5_win_bundle>` against the latest M2.5 reactor-defense bundle and asserts the markdown contains an `## Outcome` section and a `## Checksum Status` row matching the bundle's `final_sim_checksum`.

## Cross-Crate Contracts
- Depends on: `cf-replay` (Event + RunManifest + RunSummary types via `serde_json::Value` round-trip; we deliberately re-deserialize from disk via the typed structs to inherit version-string validation).
- Reads run bundles produced by `cf-app` / `cfctl` / `cf-e2e` / `cf-control`; never writes back into the bundle.
- Markdown output is deterministic given the bundle (no timestamps, no random ordering); golden tests rely on this.

## Common Pitfalls
- `parent_event_id` is `Option<String>` — terminal events can legitimately have no parent (e.g., `mission_resolved` derived from a tick-driven check). Cause-chain rendering distinguishes "root reached" from "parent missing from bundle".
- `events.jsonl` can be very large (M2.5 win is 3 MB / 7777 events). The loader streams line-by-line; viewer indexing builds a `BTreeMap<event_id, usize>` for O(log n) lookup.
- `summary.json.event_counts.total` MUST match the actual `events.jsonl` line count; corrupt-bundle rejection treats a mismatch as a hard error.
- `run_id` MUST match across `run_manifest.json`, `events[*].run_id`, and `summary.json.run_id` / `manifest_run_id`; the loader rejects mismatches.
- Filtering by category accepts a comma-separated list; an empty filter means "all categories".
- The viewer's `--at-tick` is inclusive: events at exactly that tick are visible. The tail position is anchored to the last event with `tick <= at_tick`.

## Source Trail
- spec/prototype-roadmap §M3 — Replay And Event Recorder (M3B section).
- spec/native-implementation-backlog M3B-001..M3B-003.
- references/prototype-run-bundle-schema.
- DR-002 (replay/event architecture, M3B IS the closure milestone).
- corefall/docs/implementation-log/2026-05-09-m3b-replay-viewer-debrief.md.
