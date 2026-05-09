---
type: decision
id: DR-002
status: open
priority: P0
revisit_trigger: "When the recorder + viewer prototype runs against a 5-minute battle and reproduces death/breach causes."
---

← [[decisions/index|decision records]] · [[systems/replay-event-architecture|replay/event architecture]] · [[spec/replay-recorder-slice-a|recorder Slice A]] · [[systems/ai-trust-test-suite|AI trust]] · [[engine/network-terrain-replication-lifecycle|terrain replication]]

# DR-002: Replay And Event Architecture

> [!info] Status: OPEN; LEAN: event log + snapshots; defer deterministic replay

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

- Recorder/viewer prototype is benchmarked against a 5-minute battle.
- AI trust suite is running; we know whether deterministic AI scenarios are feasible.
- Multiplayer DR (DR-005) chooses an authority model.
- Mod community emits events that conflict with reserved namespaces.

## Source Trail

- [[systems/replay-event-architecture]]
- [[spec/replay-recorder-slice-a]]
- [[systems/ai-trust-test-suite]]
- [[engine/network-terrain-replication-lifecycle]]
- [[systems/networking-backend-frontend]]
- [[engine/projectile-to-impact-lifecycle]]
- [[engine/terrain-mutation-and-pathfinding-lifecycle]]
- [[engine/body-damage-wound-gib-lifecycle]]
