# M3B — Replay Viewer + Debrief

## Status

`active`

## Intent

A simple offline tool reads a run bundle and presents the events in a human-scannable form: scrub through events, filter by category, see the parent cause chain for any death or mission-resolved event, and emit a debrief markdown summarizing the run. DR-002 (replay/event architecture) closes when the viewer + cause-chain + debrief work end-to-end.

## Player-facing behavior

M3B is **offline review tooling** — read-only, never mutates bundles. Three primary use cases:

1. **Developer / agent debugging**: "what happened in this 5-minute run?" — `cf-tools-replay-viewer view <bundle>` prints a human-scannable chronological event list with tick, category, type, payload summary. Filterable by tick, category, actor, event type. No raw JSON dumps in the output.

2. **Player-facing death recap** ("show me why"): when M1.5+ mission_resolved fires with `result=lost`, the M3B viewer is invoked at the divergence event. The output renders the cause chain — input → fire → projectile → wound → status → death — in plain language so the player understands the loss.

3. **AI Self-Test grading**: the BP closure flow's AI-Agent Self-Test Report (per AGENTS.md) reads the M3B `debrief.md` first. The debrief is the authoritative summary of outcome + key events + checksum status + captures.

Subcommands:

- `view <bundle> [--at-tick N] [--filter <category>] [--actor <id>] [--event-type <type>] [--tail-len N] [--watch]` — chronological event list, optionally centered on a tick, optionally filtered.
- `cause-chain <bundle> [--event-type <type>] [--event-id <id>] [--render-png <path>]` — walks `parent_event_id` from any leaf event back to a root cause. Handles 4 termination kinds explicitly.
- `debrief <bundle>` — emits `debrief.md` next to the bundle with structured sections (outcome / mission state / key events / cause chain for losses / checksum status / captures / accessibility).
- `validate <bundle>` — runs the same 7 BundleError checks the loader does, emits a structured `validation.json`.
- `summary <bundle>` — one-line summary suitable for sweep verdicts ("micro_breach_win: result=won @ tick 4523, checksum=<hex>, 1247 events, 0 dropped").

All output is **color-independent + plain text** (markdown) so it satisfies DR-012 accessibility floor without an SDL/Bevy renderer. Future GUI layer (post-BP4) wraps the same library.

## Crates / modules touched

| Crate | Status | What changes |
|---|---|---|
| `cf-tools-replay-viewer` | NEW (promote from stub) | Binary + library. CLI subcommands: `view`, `cause-chain`, `debrief`, `validate`, `summary`. **Bundle loader** with 7 typed `BundleError` variants (per DR-002 closure): `MissingManifest`, `MissingEvents`, `MissingSummary`, `ManifestSummaryRunIdMismatch`, `EventRunIdMismatch { event_id, expected, actual }`, `NonMonotonicTicks { event_id, prev_tick, this_tick }`, `EventCountMismatch { summary_count, actual_count }`. **Cause-chain walker** with 4 typed termination kinds: `RootReached { root_event_id }`, `ParentMissingFromBundle { missing_id, last_resolved_id }`, `MaxDepthReached { depth, last_event_id }`, `CycleDetected { cycle_member_ids }`. **Debrief markdown emitter**. **Validate command** runs all 7 BundleError checks + 6 required notes.md headings + cross-file consistency rules from M3A. Pure-Rust library + thin CLI front-end; egui GUI is post-BP4 future work, not M3B. |
| `cf-replay` | MODIFY (small; mostly already in from M3A) | `parent_event_id` field on every causal event (locked in M3A envelope v0.1; M3B is the consumer). Read-side helpers: `find_event(id)`, `walk_parents(event, max_depth)`, `is_cosmetic(event)` for chain skip rules. Index builder: `EventIndex` with O(1) lookup by `event_id` so cause-chain walks are linear in chain depth not bundle size. |
| `cf-ui` | MODIFY (small) | **In-game death-recap popup** (per DR-023 "show me why" handoff): when mission.mission_resolved fires with result=lost, the cf-ui modal embeds the M3B cause-chain output for the player's death OR mission failure. No raw events shown; rendered as plain-language sentences ("You were killed by guard's rifle hit at tick 4521. Last action before death: stood still in guard's LOS for 30 ticks."). |
| `cf-app` | MODIFY (small) | Wire the "show me why" CTA from M1.5+ mission_resolved modal to spawn cf-tools-replay-viewer cause-chain on the current bundle, render output inline. |

## Files

Source:
- `game/crates/cf-tools-replay-viewer/src/lib.rs` (NEW or MODIFY: public library API for viewer / cause-chain / debrief / validate)
- `game/crates/cf-tools-replay-viewer/src/main.rs` (NEW or MODIFY: CLI dispatch)
- `game/crates/cf-tools-replay-viewer/src/loader.rs` (NEW: bundle loader with 7 BundleError variants)
- `game/crates/cf-tools-replay-viewer/src/index.rs` (NEW: EventIndex for O(1) lookup)
- `game/crates/cf-tools-replay-viewer/src/view.rs` (NEW: event tail + filter + scrub + --watch live-tail loop)
- `game/crates/cf-tools-replay-viewer/src/cause_chain.rs` (NEW: 4-termination walker + plain-language renderer)
- `game/crates/cf-tools-replay-viewer/src/cause_chain_png.rs` (NEW: --render-png static cause-chain image emit)
- `game/crates/cf-tools-replay-viewer/src/debrief.rs` (NEW: markdown emitter with 8 required sections)
- `game/crates/cf-tools-replay-viewer/src/validate.rs` (NEW: 7 BundleError + 12 cross-file rules from M3A + 6 notes.md headings)
- `game/crates/cf-tools-replay-viewer/src/summary.rs` (NEW: one-line summary for sweep verdicts)
- `game/crates/cf-tools-replay-viewer/src/renderer.rs` (NEW: plain-language event renderer; replaces raw JSON dumps)
- `game/crates/cf-replay/src/event.rs` (MODIFY: locked in M3A; read-side helpers added)
- `game/crates/cf-replay/src/index.rs` (NEW or MODIFY: EventIndex public API)
- `game/crates/cf-ui/src/death_recap_modal.rs` (NEW: in-game popup wrapping cause-chain output)
- `game/crates/cf-app/src/main.rs` (MODIFY: wire "show me why" CTA to cf-tools-replay-viewer)

Tests + scripts:
- `game/crates/cf-tools-replay-viewer/tests/loader_tests.rs` (NEW: 7 BundleError adversarial bundles, each variant proven to reject)
- `game/crates/cf-tools-replay-viewer/tests/cause_chain_tests.rs` (NEW: 4 termination kinds proven; RootReached / ParentMissingFromBundle / MaxDepthReached / CycleDetected)
- `game/crates/cf-tools-replay-viewer/tests/debrief_tests.rs` (NEW: 8 required sections present in output for M1.5 + M2.5 + M5 bundles)
- `game/scripts/cfctl/m3b_viewer_smoke.cfctl.json` (NEW: runs a 30s scenario, then validates that viewer subcommands handle the bundle)

Documentation:
- `docs/plan/spec/death-recap-ux-contract.md` (NEW: plain-language rendering rules so death recap doesn't dump JSON at the player; replaces raw event types with sentence templates per event category)

Schemas:
- `game/crates/cf-tools-replay-viewer/schemas/v1/validation_report.schema.json` (NEW: output of `validate` subcommand)
- `game/crates/cf-tools-replay-viewer/schemas/v1/cause_chain_report.schema.json` (NEW: output of `cause-chain` subcommand)
- `game/crates/cf-tools-replay-viewer/schemas/v1/debrief_md_sections.schema.json` (NEW: lists the 8 required section headings)

## Acceptance criteria

### View subcommand (event tail + filters + watch)

```gherkin
Scenario: View shows chronological events in human-scannable form
  Given a valid run bundle
  When `cargo run -p cf-tools-replay-viewer -- view <bundle>` runs
  Then stdout shows one event per line: "[tick] [category.event_type] [actor_id?] <plain-language summary>"
  And no raw JSON payload dumps appear (payload is rendered via cf-tools-replay-viewer::renderer)
  And the output is monospaced + plain ASCII (no color codes; satisfies DR-012 color-independent state labels)

Scenario: Filter by tick centers the output
  Given a 5-minute bundle
  When `view <bundle> --at-tick 1800 --tail-len 20` runs
  Then the output shows 20 events on either side of tick 1800 (40 events total)
  And no events outside the [tick 1800-N, tick 1800+N] window are emitted

Scenario: Filter by event category
  Given a bundle with mixed events
  When `view <bundle> --filter mission` runs
  Then the output contains only mission.* events
  When `--filter ai,combat` runs (comma-separated)
  Then the output contains only ai.* and combat.* events

Scenario: Filter by actor + event type
  Given a bundle with actor_id=42 firing weapons
  When `view <bundle> --actor 42 --event-type equipment.weapon_fired` runs
  Then the output contains only equipment.weapon_fired events with actor_id=42

Scenario: Watch mode tails an active run
  Given a cf-app is actively writing events.jsonl to a bundle
  When `view <bundle> --watch --tail-len 10` runs
  Then the viewer tails the file, printing new events as they're appended
  And updates every 100ms (configurable)
  And exits cleanly on Ctrl-C

Scenario: View rejects bundle with non-monotonic ticks
  Given a corrupt bundle where event_id=5 has tick=100 but event_id=6 has tick=80
  When `view <bundle>` runs
  Then the viewer reports BundleError::NonMonotonicTicks { event_id: 6, prev_tick: 100, this_tick: 80 }
  And exits non-zero
  And does NOT silently iterate past the bad event
```

### Cause-chain subcommand (4 typed terminations)

```gherkin
Scenario: Cause chain for actor_died walks the full chain
  Given a bundle where an enemy guard died from player fire
  When `cause-chain <bundle> --event-type actor_died` runs
  Then the output walks parent_event_id from the actor_died leaf back through:
    1. actor.actor_died (DEAD)
    2. actor.actor_status_changed (DYING)
    3. actor.actor_status_changed (DOWNED)
    4. actor.actor_status_changed (UNSTABLE)
    5. combat.wound_added
    6. combat.projectile_hit_mo
    7. combat.projectile_spawned
    8. equipment.weapon_fired
    9. ai.tactic_chosen / input.intent_received (root)
  And each step renders as plain language: "Tick 4521: guard's rifle hit player's torso (15 damage) — caused by guard's fire — caused by guard's target_acquired (player visible)"
  And the chain terminates with `RootReached { root_event_id }`

Scenario: Cause chain handles ParentMissingFromBundle
  Given a leaf event whose parent_event_id references an event NOT in the bundle (bundle was trimmed mid-event-chain)
  When cause-chain runs
  Then the walker reports ParentMissingFromBundle { missing_id, last_resolved_id }
  And prints honestly: "Chain terminated: parent event <id> referenced by <last resolved id> is not in this bundle (bundle may be partial)"
  And does NOT silently produce a short chain

Scenario: Cause chain handles MaxDepthReached
  Given a deep cause chain (>50 events in a single chain — edge case)
  When cause-chain runs with default max_depth=50
  Then the walker reports MaxDepthReached { depth: 50, last_event_id }
  And prints: "Chain terminated: depth limit (50) reached at event <id>"
  And exits 0 (this is a soft termination, not a corruption)

Scenario: Cause chain handles CycleDetected
  Given a corrupt bundle with parent_event_id cycle A→B→C→A
  When cause-chain runs
  Then the walker reports CycleDetected { cycle_member_ids: [A, B, C] }
  And prints: "Chain terminated: cycle detected through events [A, B, C]"
  And exits non-zero (corruption is a bug, not soft)
  And does NOT infinite-loop

Scenario: Cause chain for mission_resolved (terminal event, no projectile cause)
  Given a bundle where mission.mission_resolved fired from timer-expiry (no projectile cause)
  When `cause-chain <bundle> --event-type mission_resolved` runs
  Then the walker prints: "mission.mission_resolved at tick 3600 — cause: timer_expired (timer reached 0); parent: mission.timer_warning_threshold @ tick 3300"
  And resolves the chain back to mission.mission_started (root)
  And the result is RootReached, NOT a silent empty chain

Scenario: Render cause chain as PNG (for documentation / debrief embed)
  Given a bundle + a target event_id
  When `cause-chain <bundle> --event-id <id> --render-png cause_chain.png` runs
  Then cause_chain.png is written with a vertical timeline diagram (event boxes + parent arrows)
  And the PNG is checked into the bundle's captures/ folder
  (Optional polish; M3B ships markdown first, PNG as nice-to-have)
```

### Debrief markdown (8 required sections)

```gherkin
Scenario: Debrief markdown has 8 required sections
  Given a completed run bundle
  When `debrief <bundle>` runs
  Then debrief.md is written to <bundle>/debrief.md
  And the markdown contains these 8 ## sections in order:
    1. ## Outcome
    2. ## Mission state
    3. ## Key events
    4. ## Cause chain (for losses only)
    5. ## Checksum status
    6. ## Captures
    7. ## Accessibility surface
    8. ## Recorder health

Scenario: Outcome section
  Given a won bundle
  Then debrief.md ## Outcome section reads:
    - Result: WON
    - Time elapsed: <wall_seconds>
    - Ticks: <ticks_run>
    - End reason: <objective_completed reason>
  Given a lost bundle
  Then ## Outcome reads:
    - Result: LOST
    - Time elapsed: ...
    - Loss reason: <typed LossReason as_str()>
    - Final blow: <event_type @ tick> (link to ## Cause chain section)

Scenario: Mission state section
  Given a bundle with 3 objectives
  Then ## Mission state lists each objective with id, status (completed/failed/in_progress), tick_completed/failed, reason

Scenario: Key events section
  Given any bundle
  Then ## Key events shows: total count, count by category (mission/combat/ai/terrain/system), first event tick + summary, last event tick + summary

Scenario: Cause chain section (losses only)
  Given a lost bundle
  Then ## Cause chain shows the plain-language walk from mission.mission_resolved back to the root cause
  And the chain includes timestamps for each event
  Given a won bundle
  Then ## Cause chain section is omitted (or shows "N/A — mission won, no failure to explain")

Scenario: Checksum status section
  Given any bundle
  Then ## Checksum status lists:
    - Algorithm: blake3
    - Scope: sim_state_v1 (or higher)
    - Cadence: every N ticks
    - Final hex: <hex>
    - Event count: N
  And the values match summary.json

Scenario: Captures section
  Given a bundle with captures/
  Then ## Captures lists each PNG by filename with type tag (capture-frame / capture-grid / capture-summary-grid) + tick range
  Given a bundle without captures/
  Then ## Captures says "No captures in this bundle."

Scenario: Accessibility surface section
  Given a bundle with run_manifest.settings populated
  Then ## Accessibility surface lists:
    - ui_scale: <value>
    - high_contrast: <bool>
    - captions: <bool>
    - reduced_motion / reduced_shake / reduced_flash: <bool>
    - hold_to_confirm: <bool>
    - key_remap_enabled + key_bindings count
  (DR-012 audit trail for ACC-A regression checking)

Scenario: Recorder health section
  Given any bundle
  Then ## Recorder health lists:
    - Total events: N
    - Dropped events: M (or "0 — recorder under capacity")
    - Peak buffer depth: K
    - Categories active vs registered (from system.category_baseline)
  And flags any anomalies (dropped_total > 0 = WARNING)
```

### Validate subcommand

```gherkin
Scenario: validate runs all 7 BundleError checks + cross-file rules
  Given a bundle
  When `validate <bundle>` runs
  Then the viewer runs:
    - 7 typed BundleError checks (MissingManifest, MissingEvents, MissingSummary, ManifestSummaryRunIdMismatch, EventRunIdMismatch, NonMonotonicTicks, EventCountMismatch)
    - 12 cross-file rules from M3A (parent_event_id resolves, event_counts.by_category matches, dropped_total ≥ sum, etc.)
    - 6 required notes.md headings (Assumptions Tested / Good / Bad / Meh / Evidence Links / Next Actions)
    - expected_outcome matches system.run_finished.outcome
  And writes validation.json with `{ status: "pass" | "fail", errors: [...], warnings: [...] }`
  And exits 0 on pass, non-zero on fail (with structured error JSON on stderr)

Scenario: validate runs against all 8 BundleError adversarial test bundles
  Given test fixtures: bundles missing manifest / events / summary / with manifest-summary run_id mismatch / event run_id mismatch / non-monotonic ticks / event count mismatch / parent_event_id missing
  Then `validate` rejects each with the correct error variant
  And the test suite is checked into game/crates/cf-tools-replay-viewer/tests/loader_tests.rs
```

### Summary subcommand (for sweep verdicts)

```gherkin
Scenario: summary emits one-line sweep verdict
  Given a won micro_breach bundle
  When `summary <bundle>` runs
  Then stdout is exactly one line: "micro_breach @ <run_id>: result=won, ticks=4521, checksum=<short_hex>, events=1247, dropped=0, captures=8"
  And exit code is 0
  Given a lost bundle
  Then summary includes loss_reason: "micro_breach @ <run_id>: result=lost, loss_reason=PlayerDead, ticks=3214, ..."
  Given a corrupt bundle
  Then summary includes status=invalid + first error: "micro_breach @ <run_id>: result=INVALID, error=NonMonotonicTicks(event 42)"

Scenario: Sweep uses summary for verdict matrix
  Given a self_play_sweep run that produced 19 sub-bundles
  When the sweep verdict matrix renders
  Then it reads each bundle's `summary <bundle>` output and aggregates into the matrix
  And the verdict is reproducible (same bundle → same summary line)
```

### Death recap surface (DR-023 "show me why" handoff)

```gherkin
Scenario: In-game death recap modal renders cause chain in plain language
  Given a M1.5+ scenario where the player died
  When mission.mission_resolved fires with result=lost
  Then the cf-ui mission-resolved modal includes a "Show me why" button
  When the player clicks "Show me why"
  Then the modal expands to show the M3B cause chain rendered as plain-language sentences:
    - "Tick 4521: You died (HP=0)"
    - "Tick 4520: Guard's rifle hit your torso for 15 damage (3rd shot in burst)"
    - "Tick 4515: Guard fired (target=player, line of sight clear)"
    - "Tick 4490: Guard acquired you as target (saw you enter the room)"
    - "Tick 4485: You moved into the guard's sight cone"
  And the modal does NOT show raw JSON or event IDs (player-facing prose only)
  And the modal has a "View full debrief" button that opens debrief.md (or hands off to cf-tools-replay-viewer)

Scenario: Death recap plain-language rendering uses templates per event category
  Given an event of type combat.projectile_hit_mo with payload { target_id, damage, impact_point }
  Then the renderer outputs: "<source_actor_name>'s <weapon_name> hit <target_actor_name>'s <body_zone> for <damage> damage"
  And the renderer uses the template from docs/plan/spec/death-recap-ux-contract.md
  And NEVER falls back to raw JSON

Scenario: Death recap respects DR-012 accessibility
  Given a player with ui_scale=2.0 + high_contrast=true + captions_enabled=true
  When the death-recap modal renders
  Then text is at 2.0× scale + high-contrast palette + captions overlay active
  And no color-only state encoding is used (every state has a text label)
```

### Read-only invariant + accessibility

```gherkin
Scenario: Viewer never mutates the bundle
  Given any bundle
  When any viewer subcommand runs
  Then no file inside the bundle is modified
  And the ONLY new file written is debrief.md (next to the bundle) OR validation.json (if --output flag passed)
  And events.jsonl + run_manifest.json + summary.json + captures/ remain bit-identical

Scenario: Viewer output is plain-text + monospace
  Given any subcommand
  Then the output uses no ANSI color escape codes
  And no Unicode emoji (project rule)
  And every state has a text label (DR-012 color-independent state)
  And the output is markdown-parseable when stdout is redirected to .md

Scenario: --json flag for machine-readable output
  Given any subcommand
  When `--json` is passed
  Then stdout is structured JSON (not markdown)
  And the JSON schema is one of: view_report.schema.json / cause_chain_report.schema.json / validation_report.schema.json / debrief_report.schema.json / summary_report.schema.json
  And the AI Self-Test grading flow consumes the JSON, not the markdown
```

### Cross-cutting (M3A + DR-002 closure validation)

```gherkin
Scenario: M3B closes DR-002 (replay/event architecture)
  Given M3B done-criteria all pass
  Then docs/plan/decisions/dr-002-replay-event-architecture.md status is updated to CLOSED-DIRECTION-WITH-EVIDENCE
  And the closure evidence cites:
    - cf-tools-replay-viewer library + binary
    - 7 BundleError variants tested
    - 4 cause-chain terminations tested
    - debrief.md for M1.5 + M2.5 + M5 bundles
    - Self-play sweep row "m3b_replay_viewer_debrief" PASS

Scenario: M3B consumes M3A's full event taxonomy (27 categories)
  Given a bundle from any M1+M1.5+M2+M2.5+M3A+M4A+M5 scenario
  When `view <bundle> --filter <any of the 27 categories>` runs
  Then the viewer recognizes the category and renders events from it
  And categories with no events return "No events in this category" honestly (not an error)
  And categories not in the 27 list return BundleError::UnknownCategory

Scenario: All M2.5 + M5 chassis events render correctly
  Given a bundle with chassis events from M5 + reactor events from M2.5
  When `view <bundle>` and `cause-chain <bundle>` run
  Then chassis.stage_changed / pilot_ejected / module_state_changed render as plain language
  And reactor.reactor_destroyed / reactor_pressure_state_changed render correctly
  And cause-chain walks the chassis lifecycle (nominal → degraded → ... → wreck)
```

## Out of scope

- **Polished GUI replay browser** (egui / TUI / scrubbable timeline / waterfall viewer) — DR-002 revisit trigger / BP4+ (M3B ships the LIBRARY; future GUI wraps it)
- **Replay editing** (mutate events, branch from checkpoint) — DR-002 future / BP6+ modding ecosystem
- **Replay sharing** (community browser, social) — BP6+ post-launch
- **Live attach to a running engine** (mid-mission inspection from another process) — DR-052 / M9+ (M3B's `--watch` mode does file-tail; live attach is M9 + cf-server protocol)
- **Animated cause-chain visualization** (interactive waterfall, click-through) — BP4+ GUI polish
- **Comic-noir styling on the debrief** — M4B / BP7 (M3B ships plain markdown; M4B layers comic-noir CSS post-launch)
- **PNG cause-chain renderer beyond static diagram** (interactive zoomable visualization) — BP4+
- **Replay format compression / zstd / lz4 streaming** — BP6+ optimization (M3B reads plain JSONL)
- **Replay schema migration tools** (v0.1 → v0.2 converter) — BP6+ (M3A locks v0.1)
- **Cross-bundle diff** (compare two replays side-by-side) — BP6+ tooling
- **Network event replication smoke** ("two clients see same world") — DR-052 / M11
- **Mod-namespaced custom event rendering** (mod_id prefix discrimination) — BP6+ (DR-002 mod hook future)
- **Snapshot format drift compat tests** (cross-version replay) — BP6+ (M3A locks snapshot schema; M3B reads what M3A wrote)
- **Event-volume regression bench at BP4+ scale** — DR-054 / M5.5 (full collision + atmospherics events will spike volume; M3B's volume budget is M3A's surface)
- **Cross-platform determinism CI matrix** (verify cause-chains match across Linux/Windows/macOS aarch64) — DR-052 / M9+ CI infra
- **Real-player playtest of death recap** ("did this explain WHY?") — OPTIONAL per AGENTS.md (AI Self-Test = M3B viewer produces correct cause-chain output for known fixtures = primary gate)

## Dependencies

- **M3A event recorder (must be done)**: events.jsonl exists in v0.1 envelope, `parent_event_id` field surface is locked, snapshot cadence is established, 27-category baseline is registered, per-tick checksum is in the bundle, `cosmetic` flag is on render-only events. M3B IS the consumer of everything M3A produces.
- **M1 + M1.5 + M2 + M2.5 + M4A + M5 bundles exist**: M3B needs canonical fixture bundles to test the viewer against. Each milestone's win + loss + edge-case bundles feed M3B's loader_tests.rs adversarial coverage.

## Notes for the implementer

### Architecture rules

- **Library-first, CLI thin**. The viewer is a Rust **library** (`cf-tools-replay-viewer` lib.rs) with a thin CLI front-end. Library exposes typed functions: `load_bundle(path) -> Result<Bundle, BundleError>`, `view(bundle, filter) -> Vec<RenderedEvent>`, `cause_chain(bundle, event_id, max_depth) -> CauseChainResult`, `debrief(bundle) -> DebriefMarkdown`, `validate(bundle) -> ValidationReport`. The future GUI (BP4+) wraps the same library.
- **Read-only invariant**. The viewer NEVER mutates the bundle. Only side-effect file is `debrief.md` written next to the bundle, OR `validation.json` if `--output` is passed. Tests assert byte-identical bundle pre/post viewer invocation.
- **Plain-language rendering, never raw JSON to players**. `docs/plan/spec/death-recap-ux-contract.md` (NEW) defines templates per event category. Renderer looks up the template by event_type, fills in payload fields, emits a sentence. Falls back to "event <category>.<type> at tick <N>" if no template — never raw JSON.
- **EventIndex for O(1) lookup**. Cause-chain walking has linear time complexity in chain depth; bundle scan is O(1) per event lookup. Build the index once at load time; walk it many times.
- **4 typed cause-chain terminations**. Per DR-002 closure: every cause-chain walk returns one of `RootReached / ParentMissingFromBundle / MaxDepthReached / CycleDetected`. NEVER a silent empty result. The walker has a HashSet<event_id> visited tracker to detect cycles.
- **7 typed BundleError variants**. Per DR-002 closure: bundle loader rejects with explicit error types so callers know exactly what's wrong. Adversarial test bundles for each variant in `loader_tests.rs`.
- **`--json` flag for machine-readable output**. Sweep verdicts + AI Self-Test grading consume JSON; humans read markdown. Both paths share the same library functions.

### DR-002 closure evidence (this milestone CLOSES the DR)

When M3B done-criteria all pass:

1. Update `docs/plan/decisions/dr-002-replay-event-architecture.md` status field from OPEN to **CLOSED-DIRECTION-WITH-EVIDENCE** in the same commit chain.
2. Add `closed_at: 2026-...` and `closed_by_milestone: M3B` to the DR's frontmatter.
3. Populate `closed_evidence:` list with:
   - cf-tools-replay-viewer library + binary
   - 7 BundleError variants adversarial-tested
   - 4 cause-chain termination kinds tested
   - debrief.md for M1.5 + M2.5 + M5 fixture bundles
   - Self-play sweep row `m3b_replay_viewer_debrief` PASS
4. Reference bundle path: `prototype_runs/native/m3b_<UTC>_<hash>/`.

### Plain-language template examples

For each event category, the renderer has a template. Examples:

| Event type | Template |
|---|---|
| `input.intent_received` | "Tick {tick}: you pressed {action_name}" |
| `equipment.weapon_fired` | "Tick {tick}: {actor_name}'s {weapon_name} fired ({rounds_count} round{s})" |
| `combat.projectile_spawned` | "Tick {tick}: projectile spawned at {pos} ({velocity})" |
| `combat.projectile_hit_mo` | "Tick {tick}: {source_name}'s shot hit {target_name}'s {body_zone} for {damage} damage" |
| `combat.wound_added` | "Tick {tick}: {target_name} wounded ({severity})" |
| `actor.actor_status_changed` | "Tick {tick}: {actor_name} {from} → {to} ({cause})" |
| `actor.inventory_dropped` | "Tick {tick}: {actor_name} dropped {item_name} at hand position" |
| `terrain.terrain_carved` | "Tick {tick}: {tool_name} carved {pixel_count} pixels of {material_name}" |
| `terrain.tool_refused` | "Tick {tick}: {tool_name} refused on {material_name} ({reason})" |
| `mission.objective_started` | "Tick {tick}: objective started — {objective_text}" |
| `mission.objective_completed` | "Tick {tick}: objective '{objective_id}' completed" |
| `mission.objective_failed` | "Tick {tick}: objective '{objective_id}' failed ({reason})" |
| `mission.mission_resolved` | "Tick {tick}: mission ended — {result} ({loss_reason or win_reason})" |
| `mission.reactor_destroyed` | "Tick {tick}: reactor destroyed by {source_name}" |
| `ai.state_changed` | "Tick {tick}: {actor_name} {from_state} → {to_state} ({reason})" |
| `ai.target_acquired` | "Tick {tick}: {actor_name} now targeting {target_name} ({reason})" |
| `ai.missed_shot_reason` | "Tick {tick}: {actor_name}'s shot missed ({reason})" |
| `chassis.stage_changed` | "Tick {tick}: {actor_name}'s chassis {from_stage} → {to_stage} ({reason})" |
| `chassis.pilot_ejected` | "Tick {tick}: {actor_name}'s pilot ejected" |

Templates live in `docs/plan/spec/death-recap-ux-contract.md` as a canonical reference + are duplicated in the Rust renderer as `const TEMPLATES: &[(EventType, &str)] = &[...]`.

### CCCP source-of-truth references

When implementing, cross-check these patterns:

- **CCCP Demo viewer pattern**: CCCP has demo recording (`Demo.cpp`) but no semantic-event viewer — they replay raw packets. Lesson: **semantic events** + **cause-chain walking** is the upgrade over packet replay (per Soldat `comparables/opensoldat-local-audit.md` recommendation "opaque packet-only demos: good for playback, weak for AI explanations, death recaps, player learning").
- **BunkerBreach death recap**: `Data/Base.rte/Activities/BunkerBreach.lua` doesn't have a death recap surface — Cortex players guess what killed them. Our M3B is the upgrade.
- **CCCP `Actor.cpp:1167` wound accumulation**: wounds accumulate damage into the actor; M3B's wound_added → actor_status_changed → actor_died chain walking is the semantic surface for the same lifecycle.
- **OpenLieroX NewNet `NewNetEngine.cpp:47` RestoreState**: aspirational replay rollback that was never finished. M3B does NOT attempt rollback — read-only viewer only.
- **Chrome DevTools Performance tab pattern**: timeline scrubber + filter-by-category + drill-down to event detail. M3B's CLI surface is a flat version of this; future GUI wraps it.
- **Sentry / DataDog APM cause-chain**: parent-child span walking is the same shape as M3B's cause-chain. The 4 termination kinds + plain-language rendering match those tools' UX.

### Decision-record alignment

- **DR-002 (Replay/Event Architecture / OPEN → CLOSED at this milestone)**: M3B IS the closure milestone. The hybrid event-log + snapshots architecture from option C is proven workable end-to-end: cf-replay envelope writes → cf-headless replays → cf-tools-replay-viewer renders + cause-chains + debriefs.
- **DR-005 (Multiplayer Posture / OPEN)**: M3B's library functions feed M11+ networking — replay events are the same shape co-op + PvP will replicate. Cross-client cause-chain comparison is a M10+ test.
- **DR-008 (AI Architecture / OPEN)**: M3B is the **debug substrate** for AI-01..AI-12 trust failures. Every AI decision in any bundle is walkable via cause-chain (ai.tactic_chosen → ai.target_scored → ai.perception_signal).
- **DR-012 (Accessibility / closed at M4A)**: M3B output is plain text + monospace + color-independent state labels. UI scale doesn't apply to markdown (renderer is text); high-contrast doesn't apply (no color codes). The in-game death-recap modal (cf-ui) respects M4A's ui_scale + contrast + captions.
- **DR-018 (Death Meaning / closed)**: M3B is the SURFACE for "why did Lt. Hernandez die" — DR-018 explicitly lists cause-chain rendering as a closure requirement. Per-origin death meaning (organic / android / mech / clone / command core) is rendered through the same templates with role-specific phrasing.
- **DR-022 (AI Humanlike Bar / closed)**: criterion #7 (Replay Proof) requires every AI decision be inspectable. M3B's cause-chain walks ai.* events for the chosen action + perception + scored alternatives.
- **DR-023 (Tutorial / closed)**: "show me why" handoff is the player-facing path through M3B's cause-chain. cf-ui's death-recap modal embeds it.
- **DR-024 (Native Engine Stack / closed)**: cf-tools-replay-viewer is Rust-only (no Bevy dependency in the library); CLI uses clap. Future GUI uses egui (per DR-024 UI lean).
- **DR-052 (Network Sync / closed)**: M3B's `--watch` mode is single-process file-tail; multi-process live attach is M9+. M3B's library produces the surface that M11 co-op replay-diff will consume.

### Existing M3B work to credit during audit

Per MISSING_FEATURES W1.2 entries, several M3B items already landed:

| Item | Status |
|---|---|
| `cf-tools-replay-viewer --at-tick N` CLI flag exists (item #268) | PASS (already in) |
| Cause-chain works for `actor_died` (item #269) — guards die in m1.5 micro_breach_loss | PASS (already in) |
| `debrief.rs` has `## Checksum Status` section with algorithm/scope/cadence/final_hex/event_count (item #270) | PASS (already in) |
| `cause-chain` CycleDetected variant handles cycles (item #1639) | PASS (already in) |
| 7 BundleError variants (DR-002 closure) | PASS (already in per DR-002 closure note) |
| 4 cause-chain terminations (DR-002 closure) | PASS (already in per DR-002 closure note) |
| Self-play sweep row `m3b_replay_viewer_debrief` PASS | PASS (already in) |
| Bundle path `prototype_runs/native/m3b_2026-05-10T01-37-50Z_c078e31d/` exists | PASS (already in) |

The audit should mark these STILL FAILING and implement:

- Plain-language renderer with template lookup (most events still render as JSON-ish summaries; need `docs/plan/spec/death-recap-ux-contract.md` + Rust template const)
- `--filter actor` + `--filter event-type` flags beyond just category
- `--watch` live-tail mode (item #898)
- `--render-png` for cause-chain static image (item #899)
- `validate` subcommand with full 7 BundleError + 12 cross-file rules
- `summary` subcommand for sweep verdicts
- In-game death-recap modal (cf-ui) wrapping cause-chain output
- 8 required debrief.md sections (current debrief may be missing ## Cause chain, ## Accessibility surface, ## Recorder health)
- Non-monotonic ticks rejection (item #1640 — currently iterates regardless)
- `--json` flag for machine-readable output
- Tier-A 11-language localization (item #1638 — defer to M4A localization gate; M3B ships English templates)

### Pitfalls / things that have bitten us before

- **Cause-chain infinite loop on corrupt bundle**: ALWAYS use a HashSet<event_id> visited tracker. CycleDetected variant exists for a reason.
- **Raw JSON to players in death recap**: viewer's renderer must NEVER fall back to JSON.dump(). Either render via template, or fall back to "event {category}.{type} at tick {N}" — never raw payload.
- **Walking parents past bundle boundary**: ParentMissingFromBundle MUST be a typed variant. If the chain walker silently returns a short chain, the user thinks they have full evidence.
- **Mutating the bundle**: viewer is read-only. NEVER call any writer on bundle paths. Tests assert byte-identical pre/post.
- **Color codes in output**: ANSI escapes break markdown redirection + DR-012 color-independent accessibility. Plain ASCII only.
- **Emoji in output**: project rule — never.
- **EventIndex not built**: O(N) cause-chain walk on a 5-minute bundle (18000 ticks × ~5 events/tick = 90K events) is slow. Build the index once.
- **Loading bundle without validation**: if you skip the 7 BundleError checks, you'll panic deep inside cause-chain. Validate first, render second.
- **Debrief missing sections**: 8 required sections per AGENTS.md AI-Agent Self-Test Report Gate. Test asserts every section exists in output for known fixtures.
- **Cross-platform path separators**: bundles produced on Windows have `\` in capture paths; viewer must normalize to `/` when reading + writing markdown (debrief.md is consumed cross-platform).
- **DR-002 closure forgotten**: when M3B done-criteria pass, the implementer MUST update the DR-002 file in the same commit chain. Missing the closure update means future agents don't know the DR is closed.
