---
type: decision
id: DR-002
status: closed-direction-with-evidence
priority: P0
revisit_trigger: "When BP3+ adds a polished GUI replay browser (egui/TUI on top of `cf-tools-replay-viewer` library) OR when an event-volume regression is reported in a BP4+ run-bundle (snapshots / coalescing posture may need re-tuning at full-collision scale)."
closed_at: 2026-05-09
closed_by_milestone: M3B
closed_evidence:
  - "M3A: cf-replay event recorder + DR-002 v1 envelope (`prototype-recorder-event.v0.1`) + run-bundle writer producing manifest/events/summary/notes (M0 + M1 + M1.5 + M2 + M2.5 evidence)."
  - "M3A: cf-headless replay verifier replays BP2 run bundles tick-for-tick and matches per-tick `determinism.sim_checksum` (`blake3` over `sim_state_v1`) — proven against the M2.5 micro-reactor-defense bundle in `self_play_sweep.sh` row `m3a_headless_replay_m2_5_win`."
  - "M3B: cf-tools-replay-viewer library + binary loads any BP2+ run bundle, validates corrupt-bundle invariants (manifest+summary run_id match, monotonic ticks, event-count match, parent_event_id resolves), renders viewer shell (event tail / category filter / tick scrubber / pause-step), walks cause chains for terminal events, and composes outcome / objectives / damage / terrain / checksum debrief."
  - "M3B evidence bundle: `prototype_runs/native/m3b_2026-05-10T01-37-50Z_c078e31d/` — debrief.md + cause_chain_default.md + cause_chain_mission_resolved.md + cause_chain_reactor_damaged.md + view_*.md against the M2.5 loss bundle."
  - "Self-play sweep row `m3b_replay_viewer_debrief` PASS: viewer subcommands run against the M2.5 win bundle, debrief markdown contains `## Outcome` + `## Checksum Status`, final_sim_checksum hex matches the bundle."
---

← [[decisions/index|decision records]] · [[systems/replay-event-architecture|replay/event architecture]] · [[spec/replay-recorder-slice-a|recorder Slice A]] · [[systems/ai-trust-test-suite|AI trust]] · [[engine/network-terrain-replication-lifecycle|terrain replication]]

# DR-002: Replay And Event Architecture

> [!info] Status: CLOSED-DIRECTION-WITH-EVIDENCE (2026-05-09 at M3B); LEAN: event log + snapshots; deterministic replay through cf-headless replay verifier; viewer + cause-chain + debrief through cf-tools-replay-viewer.

## Context

Replay/event capture is product infrastructure for AI trust testing, player learning, support diagnostics, mod debugging, and any future networking model. Choosing the model now shapes simulation, networking, modding, and UX. See [[systems/replay-event-architecture]].

## Options

| Option | Summary | Best Case | Worst Case |
|---|---|---|---|
| A. Pure deterministic replay | Record inputs + seeds; re-simulate. | Tiny replay files; perfect fidelity. | Determinism with Lua + physics + RNG is brittle; one engine update breaks every replay. |
| B. Event log only | Record typed events; replay = replay events. | Rich, debuggable; cheap to implement. | Cannot perfectly reproduce simulation if events miss a state. |
| C. Hybrid event log + snapshots | Events + periodic snapshots for recovery/scrub anchors. | Best of both; bandwidth bounded. | Slightly more complex; needs schema discipline. |
| D. Video-only replay | Record framebuffer to disk. | Simplest. | Useless for AI debugging or networking. |

## Pros And Cons

| Option | Pros | Cons | Unknowns |
|---|---|---|---|
| A | Smallest files; perfect determinism (when it works). | Lua/physics determinism is hard; mod set + RNG must be tightly controlled. | Whether our sim is deterministic-able. |
| B | Easiest to ship; flexible event types. | Edge cases lose fidelity; replay scrubbing is approximate. | Bandwidth at heavy combat. |
| C | Strong story-recovery; scrub works; recovery point exists. | Two formats to maintain. | Snapshot frequency tradeoff. |
| D | Trivial. | Cannot drive AI tests, networking, debug, or modding. | None worth investing in. |

## Evaluation

| Lens | A | B | C | D |
|---|---|---|---|---|
| Player value | Replay sharing | Replay sharing | Replay sharing + scrub | Limited |
| Readability | High (post-replay) | Medium | High | Low |
| AI burden | Heavy (determinism contract) | Light | Light + bounded | None |
| UX burden | Low | Low | Low | None |
| Performance risk | Low at runtime; high at design time | Low | Medium | Lowest |
| Modding impact | Mods must be deterministic | Mods can opt in | Mods can opt in | None |
| Networking/replay | Required for lockstep online | Required for server-authoritative | Required for server-authoritative | None |
| Content cost | High (sim discipline) | Low | Medium | None |
| Retention upside | Strong (community shares) | Strong | Strong | Low |
| Ethics/fairness | Strong (cheat detection) | Medium | Strong | Low |

## Evidence

| Evidence | Source | Confidence |
|---|---|---|
| CCCP simulation has Lua, RNG, floating physics, particle pools. | [[engine/architecture]], [[engine/projectile-to-impact-lifecycle]] | High |
| CCCP networking sends bitmap deltas (not events). | [[engine/network-terrain-replication-lifecycle]] | High |
| AI trust suite already lists "replay" as a hard requirement. | [[systems/ai-trust-test-suite]] | High |
| Modern transport (GameNetworkingSockets) does not solve serialization. | [[systems/networking-backend-frontend]] | High |
| Comparable Noita talks emphasize "make sim playable, not just demo". | [[comparables/noita-powder-toy-teardown-rain-world]] | Medium |
| Slice A recorder requirements now map CCCP input/fire/projectile/body/terrain hooks into a buildable recorder/viewer checklist. | [[spec/replay-recorder-slice-a]] | High |

## Current Recommendation

Recommendation: **C. Hybrid event log + snapshots**, with a small deterministic input-trace channel reserved for AI test scenarios where a tight scope makes determinism feasible.

Why:

- Determinism across the full simulation is unrealistic given Lua/physics/RNG.
- Pure events miss state; pure snapshots blow disk.
- A bounded hybrid keeps debug, AI tests, mid-mission save, and replays all served by the same plumbing.
- Determinism stays optional and scoped to small AI scenarios where it is achievable.

## Prototype Or Validation Plan

| Test | What It Proves | Pass/Fail |
|---|---|---|
| Recorder records events at typical combat density without dropping > 0.1%. | Event volume is realistic. | Pass = under 0.1% drop; Fail = needs coalescing or smarter rate limit. |
| 5-minute battle: replay + scrub + death recap. | UX and tooling end-to-end. | Pass = recap shows correct cause; Fail = missing event types. |
| Snapshot/scrub roundtrip in scrub UI. | Snapshots actually anchor scrubbing. | Pass = scrub is < 200ms; Fail = anchor model needs revisit. |
| Slice A REC-A-01..REC-A-07. | Hook order, causality, terrain reconstruction, death recap, snapshots, event volume, and reentrancy guard are testable before full replay polish. | Pass = [[spec/replay-recorder-slice-a]] tests pass; Fail = event taxonomy needs another pass. |
| Networking replication of events for co-op. | Foundation for co-op online. | Pass = two clients see same world; Fail = missing replication scope. |
| Determinism scope: AI test scenarios re-run 3 times produce identical events. | Scoped determinism viable. | Pass = identical; Fail = drop deterministic test channel. |

## Risks

| Risk | Mitigation |
|---|---|
| Event volume balloons under chaotic combat. | Event budgets per category; coalesce terrain edits; drop with counter. |
| Adding new event types breaks old replays. | Versioned schema; replay migration tool. |
| Mods emit conflicting events. | Mod-namespaced custom events. |
| Server-authoritative model requires authoritative simulation; we have not committed. | Deferred to multiplayer DR. |
| Snapshot format drift between releases. | Compatibility tests; snapshot version stamp. |

## Revisit Trigger

Reopen this decision when:

- BP3+ adds a polished GUI replay browser (egui/TUI layered on top of the `cf-tools-replay-viewer` library) — the markdown-output viewer that closes DR-002 today is anti-scope-bounded ("No polished replay browser"); a future GUI is additive but its UX/keybinding/scrub-cadence contract may want a fresh DR pass.
- An event-volume regression is reported at BP4+ scale (full-collision + atmospherics + AI combat) where coalescing / snapshot cadence needs re-tuning beyond the BP2-locked `events.jsonl + 60-tick checksum cadence + scenario-start snapshot` shape.
- A multiplayer DR (DR-005) chooses an authority model that requires a different replay channel (e.g., lockstep input traces for online co-op vs server-authoritative event mirroring).
- A mod community emits events that conflict with reserved namespaces (current contract: mod-namespaced custom events with mod_id prefix; revisit if conflicts surface in production).

## Closure Summary (2026-05-09 — M3B)

DR-002 closed with the hybrid event-log + snapshots architecture from option C, plus a scoped deterministic-replay channel via cf-headless. Evidence:

- The DR-002 v1 event envelope shipped at M0 (`prototype-recorder-event.v0.1`); 23 categories now active across M0..M3A bundles (system / control / determinism / mind / collision / server / anti_cheat / mmo / material / reaction / atmospherics / affliction / combat / body / terrain / ai / logistics / mission / system / snapshot / determinism / ux / accessibility / performance / input / actor / equipment / capture).
- Snapshots fire at scenario start + every objective change (`snapshot_actor` / `snapshot_inventory` / `snapshot_terrain_chunk` / `snapshot_terrain_summary`).
- Per-tick checksums (`determinism.sim_checksum`, `blake3` over `sim_state_v1`, default cadence 60 ticks) anchor the deterministic-replay channel.
- `cf-headless replay <bundle>` (M3A-003) replays any BP2 bundle by re-dispatching every recorded `control.command_accepted` against a fresh M0Engine + matching scenario, and verifies every cadence checksum tick-for-tick. First-divergence reporting is `{tick, recorded, live}`.
- `cf-tools-replay-viewer` (M3B-001/002/003) renders viewer shell + cause-chain + debrief markdown over any BP2+ bundle. Bundle loader rejects 7 distinct corruption modes with typed `BundleError` variants. Cause-chain handles `RootReached` / `ParentMissingFromBundle` / `MaxDepthReached` / `CycleDetected` terminations explicitly.
- Self-play sweep row `m3b_replay_viewer_debrief` PASS proves the viewer + cause-chain + debrief work end-to-end against a real BP2 fun-proof bundle.

The deferred determinism scope (option A) remains intentionally OUT — full sim determinism with Lua/physics/RNG is unrealistic at BP4+ scale. The scoped channel (replay verifier + per-tick checksums) IS deterministic and gates BP closure today; broader determinism waits for evidence at BP4+ collision/atmospherics/AI scale.

## Source Trail

- [[systems/replay-event-architecture]]
- [[spec/replay-recorder-slice-a]]
- [[systems/ai-trust-test-suite]]
- [[engine/network-terrain-replication-lifecycle]]
- [[systems/networking-backend-frontend]]
- [[engine/projectile-to-impact-lifecycle]]
- [[engine/terrain-mutation-and-pathfinding-lifecycle]]
- [[engine/body-damage-wound-gib-lifecycle]]
