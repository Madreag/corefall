← [[index|vault home]] · [[dashboards/index|dashboard hub]] · [[decisions/index|decision records]] · [[dashboards/research-readiness|readiness]]

# Decision Tracker

> [!info] Purpose
> One page that summarizes every open decision record, its lean, the evidence still needed, and the trigger that closes it. Use this as the daily glance.

## Current Decision Board

| ID | Title | Priority | Status | Lean | Closes When |
|---|---|---|---|---|---|
| [[decisions/dr-001-engine-strategy|DR-001]] | Engine strategy | <span class="cc-flag cc-red">P0</span> | OPEN | Build/run audit → 2-week prototype → choose | Audit + prototype + reuse-ledger skim done. |
| [[decisions/dr-002-replay-event-architecture|DR-002]] | Replay/event architecture | <span class="cc-flag cc-red">P0</span> | OPEN | Hybrid event log + snapshots | Recorder + viewer reproduce 5-min battle. |
| [[decisions/dr-003-body-damage-readability|DR-003]] | Body damage readability | <span class="cc-flag cc-red">P0</span> | OPEN | Silhouette default + advanced HUD opt-in | HUD-01..HUD-03 acceptance pass. |
| [[decisions/dr-004-first-playable-slice|DR-004]] | First playable slice | <span class="cc-flag cc-red">P0</span> | OPEN | Sequenced single actor → squad → bunker breach | Slice A (single actor) playable for 5 minutes. |
| [[decisions/dr-005-multiplayer-posture|DR-005]] | Multiplayer posture | <span class="cc-flag cc-red">P0</span> | OPEN | Solo-first + co-op-ready; prototype networking freely, no launch PvP promise yet | Bandwidth/authority memo done. |
| [[decisions/dr-006-modding-data-model|DR-006]] | Modding data model | <span class="cc-flag cc-orange">P1</span> | OPEN | Schema-first + Lua escape hatches + workbench | Workbench V1 + 3 mods migrated. |
| [[decisions/dr-007-terrain-material-model|DR-007]] | Terrain/material model | <span class="cc-flag cc-red">P0</span> | OPEN | Prototype solids + curated hazards first; keep Noita-grade materials as moonshot research | Backend prototype hits perf budget. |
| [[decisions/dr-008-ai-architecture|DR-008]] | AI architecture | <span class="cc-flag cc-red">P0</span> | OPEN | Hybrid jobs + utility scoring + scripted hooks | AI-01..AI-12 pass with replays. |
| [[decisions/dr-009-command-ux-style|DR-009]] | Command UX style | <span class="cc-flag cc-orange">P1</span> | OPEN | Direct + slowdown overlay + optional tactical map | ORDER-01 acceptance pass. |
| [[decisions/dr-010-license-reuse-matrix|DR-010]] | License/reuse posture | <span class="cc-flag cc-orange">P1</span> | OPEN | Documentation only; ledger tracks usage | Public-release decision is made. |

## Evidence Backlog

| Evidence Item | Unblocks |
|---|---|
| [[comparables/opensoldat-local-audit]] first pass: control state, weapon feel, bot waypoints, snapshots/deltas, demo hooks, HUD. | DR-005, DR-008, DR-009 |
| [[comparables/opensoldat-satellites-local-audit]] first pass: deterministic base content archive, launcher UX, lobby API, server browser/deep links, mods/interfaces, package hash/purity lessons. | DR-005, DR-006, DR-010, backend service scope DR candidate |
| [[comparables/the-powder-toy-local-audit]] first pass: material schema, particle state, air/heat/gravity fields, Lua API, save/stamp/community loop, snapshot-delta undo. | DR-002, DR-006, DR-007, retention loop DR candidate |
| [[comparables/openlierox-local-audit]] first pass: rope movement, material hook/pass/damage flags, terrain carving, projectile action graphs, bot heuristics, Gusanos/Lua modding, legacy/NewNet caution. | DR-004, DR-005, DR-006, DR-007, DR-008 |
| [[spec/actor-feel-sandbox-slice-a]] prototype requirements: scope, material set, event hooks, acceptance tests, first tickets, kill criteria. | DR-001, DR-002, DR-003, DR-004, DR-005, DR-007, DR-008, DR-009 |
| [[spec/replay-recorder-slice-a]] prototype requirements: event envelope, hook map, stable-id caveat, causality model, snapshot cadence, viewer requirements, REC-A tests, first tickets. | DR-002, DR-003, DR-004, DR-005, DR-008 |
| [[spec/ai-trust-harness-slice-a]] prototype requirements: scenario manifest, AI event contract, local AI hook map, AI-H bootstrap scenarios, report/export shape, overlay fields, first tickets. | DR-002, DR-004, DR-008, DR-009 |
| [[spec/terrain-material-sandbox-slice-a]] prototype requirements: material fixture, overlay tests, terrain events, dirty-region/path refresh metrics, AI material labels, recorder export, MAT-T tests. | DR-002, DR-004, DR-005, DR-006, DR-007, DR-008, DR-009 |
| CCCP local build + run on Linux/macOS. | DR-001 |
| Greenfield actor-feel prototype (2 weeks, controller + small destruction). | DR-001, DR-004 |
| Replay/event recorder + viewer prototype from [[spec/replay-recorder-slice-a]]. | DR-002, DR-008 |
| HUD silhouette mockup + 5-user playtest. | DR-003 |
| Bandwidth measurement at peak combat density. | DR-005, DR-007 |
| Workbench V1 (module browser + INI editor + sandbox). | DR-006 |
| Material lab + MAT-T-01..MAT-T-10 terrain/material sandbox tests. | DR-007 |
| AI-H bootstrap harness from [[spec/ai-trust-harness-slice-a]], then AI-01..AI-12. | DR-008 |
| Command overlay prototype + slowdown ratio test. | DR-009 |

## Topics Still Without A Decision Record

These can become DRs when evidence accumulates:

| Topic | Why It's Not A DR Yet |
|---|---|
| Progression / retention loop | Research/prototype freely; promote to DR once core combat loop is proven. |
| Monetization ethics | Research/prototype retention or collection mechanics freely; promote to DR before any launch commitment, after modding/fairness boundaries are visible. |
| Backend service scope | Depends on DR-005 (multiplayer) outcome; sketch services freely. |
| Audio/music identity | Cosmetic-priority; no risk to core. |
| Localization plan | Pre-launch concern; research patterns now if convenient. |
| Accessibility plan | Not blocking solo-first; should still be tracked. |
| Moonshot register | Centralize wild ideas (Noita-grade materials, PvP variants, AI personality engines) in [[research-log/moonshot-register]] when one needs more than a paragraph. |

## Process

1. New decision arrives → use [[templates/decision-record-template]].
2. Add to this tracker with priority, lean, and closes-when trigger.
3. Cross-link from the relevant `systems/` or `engine/` note.
4. Update [[dashboards/research-readiness]] if it changes a gate.
5. When closed, mark `status: closed` in the record's frontmatter and move to a "closed" section here.

## Closed Decisions

_None yet._
