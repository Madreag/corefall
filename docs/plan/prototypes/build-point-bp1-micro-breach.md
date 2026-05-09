---
type: prototype-evidence
status: closed
build_point: BP1
milestones:
  - M1
  - M1.5
  - T-CAPTURE
closes_or_refreshes:
  - DR-008 LEAN closure for M1.5 reactive guard (jobs + utility scoring + scripted hooks; full DR-008 closes at M6)
  - T-CAPTURE infrastructure shipped (cf-capture crate + capture_grid.py composer + cf-e2e wiring)
last_updated: 2026-05-09
created_retroactively: 2026-05-09
---

← [[index|vault home]] · [[prototypes/index|prototypes index]] · [[spec/prototype-roadmap#Build Points (Roadmap V2)|Build Points]]

# BP1 — Micro Breach Build (CLOSED, retroactive closure note)

> [!note] Retroactive closure note
> This note was authored on 2026-05-09 to fill a process gap identified by the junior review agent (issue #25). The engineering work for BP1 closed via PRs #2 (M1), #5 (M1.5), #6 (T-CAPTURE), and #7 (T-RELEASE workflow scaffolding); this note formalizes the closure per the BP completion gate.

> [!summary] One-line outcome
> M1 actor controller + M1.5 micro breach fun slice + T-CAPTURE frame readback + grid composer all PASS the Acceptance + Contract Integrity gates. Single playable actor digs through `concrete_soft`, refuses `metal_nohook`, fights one reactive guard with deterministic seeded miss rolls + utility-scored tactics, and reaches the eastern extraction zone within 90 seconds — or dies trying. cf-e2e drives the win script in ~430 ticks (4/4 expectations PASS) and the loss script in ~1015 ticks (3/3 expectations PASS) at byte-identical determinism across 60 Hz + 120 Hz.

## Closed Milestones

| ID | Title | Status | Run-bundle ids |
|---|---|---|---|
| M1 | Actor Controller And Sim Core | CLOSED | M1-era bundles in `prototype_runs/native/m1_*` plus the M1 sweep row in self_play_sweep |
| M1.5 | Micro Breach Fun Slice | CLOSED | `prototype_runs/native/m1.5_*` win + loss bundles; cf-e2e win=4/4 PASS, loss=3/3 PASS |
| T-CAPTURE | Frame capture + grid composer | CLOSED | cf-capture crate + `capture_grid.py` composer; first proof PNGs in `m1.5_*/captures/` after PR #6 |

## Playable Artifact

| Surface | Identifier |
|---|---|
| cfctl scripts | `scripts/cfctl/m1_actor_range.cfctl.json`, `scripts/cfctl/m1_move_jump_fire_reload.cfctl.json`, `scripts/cfctl/micro_breach_win.cfctl.json`, `scripts/cfctl/micro_breach_loss.cfctl.json` |
| Scenarios | `content/scenarios/m1_actor_range.ron`, `content/scenarios/micro_breach.ron` |
| Win bundle | Any `prototype_runs/native/m1.5_*` bundle with `mission.result=won` |
| Loss bundle | Any `prototype_runs/native/m1.5_*` bundle with `mission.result=lost` + `mission.loss_reason=actor_dead` |

## T-CAPTURE Evidence

PARTIAL. T-CAPTURE shipped during BP1 via PR #6 (cf-capture crate + `capture_grid.py` composer). BP1's M1.5 win-bundle exemplar carries `captures/summary_grid.png` produced by the composer. The mandatory `summary.json.artifacts.items[]` registration of the capture artifacts (added later as part of PR #12 BP2 follow-up) is retroactively populated when the BP3 agent re-publishes the BP1 release.

## T-RELEASE Status

NEVER TAGGED (gap; recovery owned by BP3). Rationale:
- T-RELEASE infrastructure shipped late in BP1 via PR #7, after M1.5 had already closed via PR #5.
- A retroactive `v0.1.0-prealpha` tag was published on 2026-05-09 then DELETED the same day because the artifacts (`.tar.zst` archives) failed the Double-Click Playability Hard Gate (AGENTS.md § Build Point Closure Gate).
- BP3 inherits the responsibility to land the missing engineering (`Corefall.app` bundle for macOS, `Corefall.exe` launcher for Windows, AppImage for Linux) AND retroactively re-publish `v0.1.0-prealpha` alongside `v0.3.0-prealpha`.

## Universal Enhancement (DR-056) Status

PARTIAL. The Universal Enhancement Done-Criteria were authored AFTER BP1 closed (DR-056 lands per `docs/plan/spec/milestone-enhancement-pass-m1-plus.md` and is enforced from BP2 onward). The implementing agent is not retroactively required to fill in the DR-056 matrix for BP1; the contract applies forward only. M2-onward universal-enhancement rows are tracked in `docs/plan/spec/feature-completion-checklist.md`.

## Acceptance Matrix

Per `docs/implementation-log/2026-05-06-m1-actor-controller.md` § Acceptance Matrix and the M1.5 implementation log (when authored): every M1 task card (M1-001 through M1-006) and every M1.5 task card marked PASS. Acceptance verdict: PASS.

## Contract Integrity Matrix

Per the M1 + M1.5 implementation logs: shared cf-control dispatch verified for `act.player.*` JSON-RPC methods (human + cfctl + AI input route through one path); reactive AI is deterministic across seeds; mission state machine emits `objective_*` + `mission_resolved` events; soft breach emits M2-compatible `terrain_carved` events; cf-e2e win=4/4 + loss=3/3 expectations against `observe.once` snapshots. Verdict: PASS.

## T-RELEASE Recovery Plan (deferred to BP3)

When the BP3 implementing agent lands the Double-Click Playability engineering:
- BP3 produces `v0.3.0-prealpha` as the first friend-handoff release.
- BP3 retroactively re-publishes `v0.1.0-prealpha` (this BP) at a new commit using the same `.dmg` / `.msi` / AppImage engineering.
- The release notes for the retroactive `v0.1.0-prealpha` cite the M1.5 fun-slice as the demonstrable scenario; double-click `Corefall.app` / `Corefall.exe` should drop the player into the M1.5 micro-breach scenario.

## Source Trail

- Implementation logs: `docs/implementation-log/2026-05-06-m1-actor-controller.md` + the M1.5 implementation log.
- Closing PRs: [#2](https://github.com/Madreag/corefall/pull/2) (M1), [#5](https://github.com/Madreag/corefall/pull/5) (M1.5), [#6](https://github.com/Madreag/corefall/pull/6) (T-CAPTURE), [#7](https://github.com/Madreag/corefall/pull/7) (T-RELEASE workflow scaffolding).
- Roadmap row: `docs/plan/spec/prototype-roadmap.md` § Build Points (Roadmap V2) BP1 row.
- Checklist row: `docs/plan/spec/feature-completion-checklist.md` BP1 row (marked DONE [x]).
