---
type: decision
id: DR-029
status: closed-direction
priority: P0
closed_at: 2026-05-04
revisit_trigger: "Save migration breaks under a real upgrade; or replay-linked saves balloon disk usage; or cloud-save becomes a launch necessity."
---

← [[decisions/index|decision records]] · [[dashboards/decision-tracker|decision tracker]] · [[spec/prototype-roadmap|native build roadmap]] · [[decisions/dr-002-replay-event-architecture|DR-002]] · [[decisions/dr-018-death-meaning-and-consequence-ladder|DR-018]]

# DR-029: Save Game Model

> [!success] Status: CLOSED-DIRECTION (project owner committed 2026-05-04)
> **Versioned local-first saves + linked replay/run bundles.** Multiple slots, autosave, ironman policies, scenario policies persisted, mission suspend/resume, same-seed retry, migration-safe schema with version handlers. Cloud-save is **post-launch optional**, not a v1 commitment.

## Decision

**`.cfsave` is a versioned local file format with explicit migration handlers.** Every save carries a schema version and pointers to its replay archive. Saves capture the full campaign state needed to continue: command core state, base modules, actors/veterans, mechs, salvage, faction state, enemy commander memory, mission manifests, scenario policy, and replay archive references.

## What This Locks In

| Aspect | Commitment |
|---|---|
| Format | `.cfsave` versioned local file (binary or compressed JSON; TBD during M5). |
| Storage | Local disk by default. Cloud sync optional post-launch. |
| Slots | Multiple per profile; ironman locks one slot to in-place autosave. |
| Autosave | Before/after every contract; before risky branch points. |
| Mission suspend/resume | Mid-mission save/resume supported in solo and co-op singleplayer modes. |
| Same-seed retry | Failed missions can be retried with the same seed (scenario policy controls). |
| Ironman policy | Per-save flag; autosave only; no manual rollbacks. |
| Scenario policies | Tutorial-safety, lethal-on/off, rescue-vs-permanence per [[decisions/dr-018-death-meaning-and-consequence-ladder]] are persisted in the save. |
| Replay archive linkage | Saves reference run-bundle archives by ID; replay/death-recap available from save resume. |
| Migration | Every schema version has a forward migration handler. Loading an older save runs migrations in order. |
| Backup | A single rolling backup is kept per slot to recover from corrupted writes. |

## Save Contents (Reference List)

| Subsystem | What's Saved |
|---|---|
| Command core | State (rooted/uprooted), embedded chassis, stat boost. |
| Base | Module placement, power grid topology, HP, ammo, repair charges. |
| Actors / veterans | Roster, traits, injuries, equipment, AI doctrines. |
| Mechs | Chassis state, modules, paint/identity, damage history. |
| Salvage / inventory | Materials, parts, recovered modules. |
| Faction state | Reputation, contract pool, enemy commander memory across missions. |
| Mission manifests | Active contract, pending contracts, completed mission summaries. |
| Replay archive refs | Pointers to run-bundle IDs for completed missions. |
| Scenario policy | Lethal-on/off, ironman, tutorial-safety, rescue defaults. |
| Schema version + checksum | For migration + corruption detection. |

## What This Does NOT Lock

- Whether the file is binary or compressed JSON (decision in M5).
- Cloud-save provider (post-launch).
- Save sharing/exporting between players (post-launch).
- Whether co-op state syncs save state per-host or per-player (M11 design decision).

## Why Not The Alternatives

| Alternative | Why Rejected |
|---|---|
| Cloud-only save | Requires backend posture beyond DR-013 v1 scope; introduces availability/privacy issues. |
| Per-mission save snapshots only | Loses campaign continuity; replays lose scope. |
| No mission suspend/resume | Cuts off play patterns important for DR-023 onboarding labs and longer missions. |
| Replays separate from saves | Players expect to scrub the death recap for a mission they failed; tight coupling helps debug + retention. |
| No migration | First schema change kills all saves; community-killer. |

## Evidence Trail

- Project owner verbatim (2026-05-04 stack round): "Versioned local-first campaign saves + replay/run bundles. Multiple slots, autosave, ironman, scenario policies, migration-safe."
- DR-002 already commits to run-bundle replay format; saves reference those bundles by ID.
- DR-018 establishes the consequence ladder; saves persist the chosen policy.
- DR-013 backend service scope keeps cloud-save out of v1 commitment.

## Risks

| Risk | Mitigation |
|---|---|
| Save corruption from a crash mid-write | Atomic write (temp file + rename); rolling backup. |
| Migration handlers fall behind | Migration is part of every milestone's done-criteria when its data changes. CI test: load every old save in the corpus. |
| Save bloat from replay-linked archives | Replay archives live in their own directory with a configurable retention policy. |
| Ironman feels punishing without rescue paths | Ironman per-save flag is opt-in; rescue defaults per DR-018 still apply when policy permits. |
| Co-op save divergence | M11 work; document host-authoritative model when LAN/online co-op lands. |

## Prototype / Validation Plan

| Test | What It Proves |
|---|---|
| M5 — Save and reload mid-mission preserves chassis/inventory/replay state. | Suspend/resume works. |
| M7 — Save after contract; load; replay archive scrubs to the saved tick. | Save + replay coupling works. |
| M7 — Migration test: a v0.1 save loads on v0.2 with a declared handler. | Migration discipline holds. |
| M9 — Headless replay from save state produces identical checksums. | Save state is determinism-island-clean. |
| M11 — Online co-op host save preserves both clients' state. | Multi-player save model works. |

## Revisit Trigger

- Migration breaks under a real upgrade (a milestone change requires data loss).
- Replay-linked archives balloon disk usage past a comfortable budget.
- Cloud-save becomes a launch necessity (audience signal, not principle).
- Co-op save divergence shows up at M11.

## Source Trail

- Project owner stack-round answers (2026-05-04).
- [[decisions/dr-002-replay-event-architecture]]
- [[decisions/dr-013-backend-service-scope]]
- [[decisions/dr-018-death-meaning-and-consequence-ladder]]
- [[decisions/dr-023-tutorial-and-onboarding-strategy]]
- [[spec/prototype-roadmap]] — T-SAVE side track.
- [[research-log/2026-05-04-roadmap-rebuild-native-stack]]
