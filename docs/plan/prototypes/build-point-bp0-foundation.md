---
type: prototype-evidence
status: closed
build_point: BP0
milestones:
  - M0
closes_or_refreshes:
  - DR-002 v1 event envelope locked (snapshots/checksums tracked; full event taxonomy closes at M3A in BP2)
  - DR-012 accessibility floor lock (six runtime flags surfaced via cf-control + observe.settings)
last_updated: 2026-05-09
created_retroactively: 2026-05-09
---

← [[index|vault home]] · [[prototypes/index|prototypes index]] · [[spec/prototype-roadmap#Build Points (Roadmap V2)|Build Points]]

# BP0 — Foundation Build (CLOSED, retroactive closure note)

> [!note] Retroactive closure note
> This note was authored on 2026-05-09 to fill a process gap identified by the junior review agent (issue #25). The engineering work for BP0 closed via PR #1 + the M0 implementation log (`docs/implementation-log/2026-05-05-m0-engine-bootstrap.md`); this note formalizes the closure per the BP completion gate in `spec/prototype-roadmap.md`.

> [!summary] One-line outcome
> M0 engine bootstrap shipped a 29-crate Rust workspace with deterministic 60 Hz / 120 Hz fixed-tick sim, JSON-RPC control plane, cfctl one-shot CLI, replay run-bundle writer (events.jsonl + summary.json + run_manifest.json), panic capture via cf-replay diagnostics, DR-012 accessibility floor lock, and CI matrix on Linux + macOS + Windows.

## Closed Milestones

| ID | Title | Status | Run-bundle ids (sample) |
|---|---|---|---|
| M0 | Engine Bootstrap | CLOSED | 50+ run bundles in `prototype_runs/native/m0_*` covering smoke runs at both 60 Hz and 120 Hz, plus headless+control-api modes |

## Playable Artifact

| Surface | Identifier |
|---|---|
| cfctl scripts | `scripts/cfctl/m0_settings_roundtrip.cfctl.json` (DR-012 settings round-trip) |
| Scenarios | `content/scenarios/m0_blank.ron` (single-actor smoke; no playable gameplay yet) |
| Smoke bundle | Any `prototype_runs/native/m0_*` bundle from the M0-era runs |

## T-CAPTURE Evidence

NOT APPLICABLE. T-CAPTURE shipped at BP1 (PR #6); BP0 predates the cf-capture crate. The T-CAPTURE evidence row in the BP completion gate is mandatory only from BP2 onward, so BP0's omission is per spec, not a gap.

## T-RELEASE Status

NEVER TAGGED. T-RELEASE infrastructure shipped at BP1 (PR #7); BP0 predates the `release.yml` workflow + `generate_release_notes.py` parser. The T-RELEASE row in the BP completion gate is mandatory only from BP1 onward, so BP0's omission is per spec.

If a retroactive `v0.0.0-prealpha` tag is desired for symmetry, it would be tagged on the M0 closure commit and would publish a binary-free placeholder release per the versioning axis row "BP0 → tag-only, no binaries". Whether to tag this retroactively is deferred to the BP3 implementing agent who owns the broader release-recovery work (see § T-RELEASE recovery plan below).

## Universal Enhancement (DR-056) Status

NOT APPLICABLE. DR-056 Universal Enhancement Done-Criteria became mandatory from M1+ onward (per `docs/plan/spec/milestone-enhancement-pass-m1-plus.md`). BP0 / M0 predates the contract.

## Acceptance Matrix

Per `docs/implementation-log/2026-05-05-m0-engine-bootstrap.md` § Acceptance Matrix: every M0 task card (M0-001 through M0-008) marked PASS with cited evidence (run bundles, schema dumps, cfctl smoke logs). Acceptance verdict: PASS.

## Contract Integrity Matrix

Per `docs/implementation-log/2026-05-05-m0-engine-bootstrap.md` § Contract Integrity: shared cf-control source-of-truth dispatch verified for cf-app + cfctl + run_m0_inline; required-field rejection (`schema_version != 1` returns -32602) covered by unit tests; run bundles source-truthful (no boilerplate); checklist truth verified. Verdict: PASS.

## T-RELEASE Recovery Plan (deferred to BP3)

The BP3 implementing agent owns the release engineering for the Double-Click Playability Hard Gate (AGENTS.md § Build Point Closure Gate). When that lands:
- BP3 produces `v0.3.0-prealpha` as the first friend-handoff release (`.dmg` / `.msi` / AppImage formats).
- BP3 retroactively re-publishes `v0.1.0-prealpha` (BP1) and `v0.2.0-prealpha` (BP2) using the same engineering.
- BP0's `v0.0.0-prealpha` placeholder tag is OPTIONAL — the versioning axis explicitly notes it as "tag-only, no binaries". Whether to publish it for symmetry is at the BP3 agent's discretion.

## Source Trail

- Implementation log: `docs/implementation-log/2026-05-05-m0-engine-bootstrap.md`.
- Closing PR: [#1](https://github.com/Madreag/corefall/pull/1) merged 2026-05-05.
- Roadmap row: `docs/plan/spec/prototype-roadmap.md` § Build Points (Roadmap V2) BP0 row.
- Checklist row: `docs/plan/spec/feature-completion-checklist.md` BP0 row (marked DONE [x]).
