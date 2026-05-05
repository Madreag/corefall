---
type: spec
status: implementation-bridge
ready_when: "Slice A implementation produces playtest notes, recorder exports, budget metrics, and DR-004 can choose repeat A, move to B, or split a focused A.1."
feeds:
  - DR-001
  - DR-002
  - DR-003
  - DR-004
  - DR-005
  - DR-007
  - DR-008
  - DR-009
---

← [[spec/index|spec section]] · [[spec/prototype-implementation-backlog-slice-a|implementation backlog]] · [[prototypes/actor-feel-lab-a0-bootstrap|A0 bootstrap evidence]] · [[prototypes/actor-feel-lab-a1-runtime-smoke|A1 runtime smoke]] · [[prototypes/actor-feel-lab-a1-ui-smoke|A1 UI smoke]] · [[prototypes/actor-feel-lab-a1-load-w-workbench-smoke|LOAD-W workbench smoke]] · [[prototypes/actor-feel-lab-a1-load-w-fixture-switch-smoke|LOAD-W fixture-switch smoke]] · [[prototypes/actor-feel-lab-a1-load-w-fixture-tab-input-smoke|LOAD-W fixture-tab input smoke]] · [[prototypes/actor-feel-lab-a1-load-w-fixture-traversal-smoke|LOAD-W fixture traversal smoke]] · [[spec/actor-feel-sandbox-slice-a|actor-feel Slice A]] · [[spec/replay-recorder-slice-a|recorder Slice A]] · [[systems/replay-determinism-and-run-evidence|determinism/run evidence]] · [[spec/terrain-material-sandbox-slice-a|terrain/material Slice A]] · [[spec/ux-wireframes-slice-a|UX wireframes Slice A]] · [[spec/equipment-loadout-workbench-slice-a|equipment workbench Slice A]] · [[references/prototype-run-bundle-schema|run-bundle schema]] · [[decisions/dr-004-first-playable-slice|DR-004]] · [VAULT_PLAN.md](../../VAULT_PLAN.md)

# Prototype Roadmap

> [!summary] Current posture
> The research vault has enough Slice A requirements to start implementation. This page is the bridge from research to build order: what to build first, which companion systems must exist from day one, what evidence to collect, and when to stop adding features.

> [!tip] Implementation handoff
> Use [[spec/prototype-implementation-backlog-slice-a]] when assigning actual tasks. It expands this roadmap into A0..A7 task cards, gates, run-bundle evidence requirements, and an explicit A5 equipment/loadout workbench milestone grounded in real CCCP device/loadout field artifacts.

> [!info] A0 status
> The first bootstrap run exists and validates the run-bundle/checker path: [[prototypes/actor-feel-lab-a0-bootstrap]]. [[prototypes/actor-feel-lab-a1-runtime-smoke]] adds a browser runtime and checked movement/aim/rifle/reload/status/snapshot event families. [[prototypes/actor-feel-lab-a1-ui-smoke]] adds deterministic screenshot/capture proof, selected `engineer_breach` LOAD-A fixture state, Light Digger/Constructor material-grid edits, checksum/path-refresh events, bounded path/collision query events, bounded actor-hull response events, probe overlays, and Timed Explosive refusal labels. [[prototypes/actor-feel-lab-a1-load-w-workbench-smoke]] adds generated workbench trace/source/AI rendering. [[prototypes/actor-feel-lab-a1-load-w-fixture-switch-smoke]] adds all-fixture LOAD-A imports and query-param fixture switching to the `medic_rescue` Medikit row. [[prototypes/actor-feel-lab-a1-load-w-fixture-tab-input-smoke]] adds fixture-tab click and focused-button keyboard switching evidence. [[prototypes/actor-feel-lab-a1-load-w-fixture-traversal-smoke]] adds Tab-to-fixture focus, ArrowRight fixture movement, Enter/Space activation, and focus restore evidence. These still do not prove human actor feel, integrated movement collision, global pathfinding, full workbench traversal, physical gamepad input, or AI competence; the next target is manual play notes, real collision/path integration, full workbench traversal/physical gamepad proof, and feeding LOAD-A/LOAD-W events into AI-H/replay/export.

## Why This Page Exists

The vault now has detailed requirement pages for actor feel, replay, terrain/materials, AI harnessing, UX wireframes, backend/hub, backend service scope, package tooling, equipment/loadouts, and retention. The risk is no longer "we do not know what to build"; the risk is building the pieces in the wrong order and losing the evidence trail.

This roadmap keeps the first playable lab focused:

- Build the smallest playable Cortex-like loop that can be judged by feel.
- Instrument it enough that failures are explainable.
- Keep moonshot/prototype freedom, but do not let moonshots hide the core result.
- Turn every test result into a decision-record update, spec update, or research-log note.

## Prototype Philosophy

| Principle | Roadmap Rule | Why It Matters |
|---|---|---|
| Feel before breadth | One excellent actor is more important than a large arsenal or mission wrapper. | Bad movement/shooting/digging makes every later system suspect. |
| Instrument from day one | The recorder ships with the sandbox, not after it. | Physics/destruction bugs are otherwise unreproducible stories. |
| Test mechanics before content | Use ugly fixtures, debug art, and fake UI before production assets. | The first goal is truth, not presentation polish. |
| Player comprehension is a feature | HUD, overlays, event tails, and death recaps are part of the prototype. | Cortex-like chaos only works if the player trusts what happened. |
| AI uses the same contracts | Even before friendly AI exists, control, item, terrain, and replay events must be AI-readable. | Great solo play requires bots that can explain choices and recover from failures. |
| Private reuse is allowed | Borrow/copy/adapt for private tests; log provenance if material enters the future project. | Speed matters, but the vault must keep release options visible. |

## External Research Pulled In

| Source | Roadmap Lesson | How It Changes Slice A |
|---|---|---|
| Game Design Skills, Game Feel | Break "feel" into responsiveness, intuitiveness, and viscerality; acknowledge player input quickly and make results readable. | Slice A should track input-to-feedback delay, material rule clarity, and tactile impact cues as separate checks. |
| GameDeveloper, Design 101: Playtesting | Use early scattershot tests for bright spots, then experience tests for holistic feel, stress tests for exploit discovery, and accessibility tests for comprehension. | Prototype sessions should be labeled by test stage instead of mixed into one vague "playtest" bucket. |
| Games User Research, Playtest Kit page | Treat playtesting as assumption testing: choose the method, gather real behavior, cut through opinion noise, and de-risk early. | Each lab session must name the assumption being tested and produce observations, not just impressions. |
| LoopKit session-replay product page | Modern playtest tooling pairs snapshots/events with session replay so teams can inspect what happened without chasing repro steps. | The recorder/viewer should align events, snapshots, input, and player notes in one run folder from the beginning. |

## Slice Ladder

| Slice | Status | Main Question | Required Artifact | Exit Decision |
|---|---|---|---|---|
| A0: Lab shell | Bootstrap pass / runtime shell open | Can we run, reset, tune, and record a tiny sandbox quickly? | [[prototypes/actor-feel-lab-a0-bootstrap]] plus next one-room interactive scene, debug controls, tunable constants, recorder header/export. | Continue only if A1 iteration is fast enough. |
| A1: Actor feel core | Runtime/UI smoke pass / human feel open | Is moving/aiming/shooting/digging readable for five minutes? | [[prototypes/actor-feel-lab-a1-runtime-smoke]], [[prototypes/actor-feel-lab-a1-ui-smoke]], plus next A-FEEL-01..04 human run evidence. | Repeat controls, or unlock terrain/UX detail. |
| A2: Terrain/material lab | Ready to build | Do material rules, carve/fill, hazards, and dirty regions stay understandable? | [[spec/terrain-material-sandbox-slice-a]] MAT-T-01..10. | Commit, simplify, or change terrain backend. |
| A3: Replay/debug viewer | Ready to build | Can failures be reconstructed without guesswork, and can deterministic claims be tested honestly? | [[spec/replay-recorder-slice-a]] REC-A-01..07 plus [[systems/replay-determinism-and-run-evidence]] DET-A-01..07. | Fix event taxonomy/checksum/snapshot gaps before adding squad AI. |
| A4: UX comprehension pass | Ready to build | Can a player tell what happened, what is selected, and why a tool failed? | [[spec/ux-wireframes-slice-a]] HUD/material/replay slices. | Lock first HUD vocabulary or redesign it. |
| A5: Equipment/loadout mini-workbench | Ready to build | Do item roles and bot usability labels survive a real fixture? | [[spec/equipment-loadout-workbench-slice-a]] + [[references/equipment-ai-behavior-contract]]. | Promote/loadout fields into core schema or revise. |
| A6: AI trust bootstrap | Requirements ready | Can a bot drive the same surfaces and explain failures? | [[spec/ai-trust-harness-slice-a]] AI-H-01..06 plus AI-EQ labels. | Promote simple utility behavior or revisit AI architecture. |
| A7: Breach contract proof mission | Requirements ready | Can actor feel, terrain, equipment, commander AI, objectives, UX, and replay form one repeatable mission? | [[spec/mission-director-slice-a]] MISSION-A-01..18. | Promote proof mission into Slice B/C or revise mission/director contract. |
| B: Small squad scenario | Planned | Are 2-3 actors, one objective, and replayed failures compelling? | New Slice B spec after A results. | Move toward first playable or repeat A/B. |
| C: Bunker breach demo | Planned | Does command + terrain + AI + logistics create the Cortex fantasy? | New Slice C spec after B. | Public demo candidate only if evidence supports it. |

## Integrated Slice A Build Order

| Order | Build | Consumes | Produces | Must Not Grow Into |
|---|---|---|---|---|
| 1 | Lab shell and run manifest | [[spec/actor-feel-sandbox-slice-a]], [[spec/replay-recorder-slice-a]] | `run_id`, seed, scene id, build id, tunable config dump. | Full campaign shell. |
| 2 | Explicit control intent | [[engine/direct-control-and-actor-feel-lifecycle]] | `input_intent`, input latency marker, replayable control trace. | Full netcode. |
| 3 | Movement/aim/rifle loop | [[spec/actor-feel-sandbox-slice-a]] | A-FEEL-01/02 test evidence, reticle state, recoil/reload metrics. | Large weapon catalog. |
| 4 | Minimal recorder ring buffer | [[spec/replay-recorder-slice-a]], [[systems/replay-determinism-and-run-evidence]] | JSONL export, event tail, dropped-event counters, input trace, checksums, actor/inventory/terrain snapshots. | Polished replay product. |
| 5 | Dirt/concrete/material query | [[systems/material-and-mobility-affordance-schema]] | material probe, tool validity, projectile threshold events. | Noita-grade chemistry. |
| 6 | Digger, carve, grenade/charge | [[spec/terrain-material-sandbox-slice-a]] | terrain dirty regions, carve masks, explosion cause chains. | Full demolition system. |
| 7 | Status/damage/death recap | [[engine/body-damage-wound-gib-lifecycle]], [[spec/replay-recorder-slice-a]] | wound/status/death events, recap panel, player explanation test. | Final gore/body model. |
| 8 | Material overlay and HUD pass | [[spec/ux-wireframes-slice-a]] | HUD-01..03, MAT-01A..D, material and item failure labels. | Final art/UI system. |
| 9 | Repair/fill and mobility lane | [[spec/terrain-material-sandbox-slice-a]], [[research-log/moonshot-register]] | repair/fill events, anchor/nohook results, A.1 split decision. | Feature creep if gun/dig loop is weak. |
| 10 | Equipment/loadout fixture behavior | [[spec/equipment-loadout-workbench-slice-a]], [[references/equipment-ai-behavior-contract]], [[prototypes/actor-feel-lab-a1-ui-smoke]], [[prototypes/actor-feel-lab-a1-load-w-fixture-traversal-smoke]] | Selected LOAD-A fixture state, material-grid dig/fill edits, checksum/path-refresh events, bounded route/collision probes, bounded actor-hull responses, probe overlays, explosive-refusal labels, generated workbench rows, query-param fixture switching, fixture-tab click/focused-keyboard switching, and fixture-control Tab/Arrow/Enter/Space traversal now exist; next connect these to integrated movement collision, global path planner output, full workbench traversal, physical gamepad input, and AI-H scoring. | Full economy/store. |
| 11 | AI trust bootstrap | [[spec/ai-trust-harness-slice-a]], [[references/equipment-ai-scenarios-slice-a]] | AI-H-01..06/AI-EQ first results. | Commander AI before basic bot competence. |
| 12 | Breach Contract proof mission | [[spec/mission-director-slice-a]], [[spec/missions-and-objectives]], [[spec/equipment-loadout-workbench-slice-a]] | MISSION-A events, commander reason strings, capability strip, save/replay roundtrip, debrief output. | Campaign/meta layer before one contract is repeatable. |

## Required Run Folder

Each meaningful Slice A run should produce a folder or equivalent bundle:

Use [[references/prototype-run-bundle-schema]] and `research_tools/prototype_run_check.py` as the concrete contract for these files.

| File | Required Contents | Consumer |
|---|---|---|
| `run_manifest.json` | build id, git commit or prototype hash, seed, scene id, material schema version, config hash. | DRs, reproducibility, package/workbench. |
| `events.jsonl` | recorder envelope events and snapshots. | Replay, AI harness, terrain/UX debug. |
| `summary.json` | event counts, bytes/sec, dropped events, pass/fail counters, max frame cost. | DR-002, DR-005, performance budget. |
| `notes.md` | observer notes, tester quotes/paraphrases, Good/Bad/Meh moments, assumptions tested. | Research log and spec updates. |
| `screenshots/` or `captures/` | HUD, overlay, death recap, trace-tab or viewer screenshots. | UX review and future comparison. |

## Acceptance Evidence Matrix

| System | Minimum Evidence Before Slice A Exit | Target Notes |
|---|---|---|
| Actor feel | A-FEEL-01..06 results from at least three internal runs. | Include both subjective notes and recorded events. |
| Replay | REC-A-01..07 and DET-A-01..07, plus at least one death/failure recap reconstructed from event chain. | Event taxonomy, checksum, snapshot, or first-divergence failures block squad AI. |
| Terrain/material | MAT-T-01..10, dirty-region counts, terrain snapshot size, path refresh result or stale-path warning. | Keep semantic events first, bitmap snapshots second. |
| UX/HUD | HUD/material/death-recap screenshots, text fits, overlays explain failure reasons. | Accessibility floors from [[spec/ux-wireframes-slice-a]] apply even to debug UI. |
| Equipment/loadout | LOAD-A fixtures imported into the lab, selected item/tool behavior visible, first LOAD-W workbench render smoke, first query-param fixture-switch smoke, first fixture-tab click/focused-keyboard input smoke, and first fixture-control Tab/Arrow/Enter/Space traversal smoke. | Full workbench can wait, but full workbench traversal, physical gamepad input, squad comparison, AI-H scoring, 200% text scale, and replay/export preview still need evidence. |
| AI | One scripted or simple bot can drive control intent or produce a documented blocker. | Do not promote "great AI" until harness results exist. |
| Performance | 60-second spam run with event volume, dropped events, terrain edit cost, and frame-time notes. | Needed for DR-002, DR-005, DR-007. |

## Playtest Modes

| Mode | When To Use | Testers | What To Record |
|---|---|---|---|
| Concept smoke | First time a loop exists. | Builder + one observer. | Does the loop run; what breaks immediately. |
| Scattershot | Comparing movement constants, weapon feel, carve masks, mobility tools. | Internal, mechanically fluent testers. | Bright spots, bad feels, exact config values. |
| Experience | Actor can complete a five-minute loop. | Target players or genre-adjacent testers. | Good/Bad/Meh moments, event recap, confusion points. |
| Stress | After the loop is fun enough to break. | Optimizers, speedrunners, systems-minded testers. | Exploits, event volume, terrain abuse, degenerate loadouts. |
| Accessibility/comprehension | Before promoting UI/UX language. | Players less familiar with Cortex-like games. | Misread HUD/material/tool/death causes. |

## Risk Budget

| Risk | Budget Rule | Stop Signal |
|---|---|---|
| Mobility feature creep | Tether/grapple/jet is optional until gun/dig/recorder pass. | Mobility work blocks A-FEEL-01..04. |
| Terrain ambition | Eight materials max in A; no chemistry chain unless MS-01 is isolated. | Material rules need explanation longer than the overlay itself. |
| Recorder complexity | JSONL and simple viewer first. | Replay polishing delays event capture. |
| Arsenal breadth | Five item roles max in actor lab. | New item does not test a new role or failure label. |
| AI ambition | Scripted/simple bot first; reason labels before cleverness. | Bot behavior cannot say why it selected/refused an item. |
| UI polish | Debug UI may be plain, but must be readable and fit. | Pretty panels obscure status/material/tool consequences. |
| Networking | Serialize/count event sizes; do not implement online play in A. | Net work delays local replay and terrain evidence. |

## Decision Handoff

| Decision | Slice A Evidence Needed | Possible Outcome |
|---|---|---|
| [[decisions/dr-001-engine-strategy]] | Iteration speed, terrain edit cost, recorder integration friction. | Continue current engine path, fork, or isolate prototype tech. |
| [[decisions/dr-002-replay-event-architecture]] | Event chain, snapshot cadence, JSONL size, dropped-event behavior. | Promote hybrid event+snapshot format or revise. |
| [[decisions/dr-003-body-damage-readability]] | Damage/death recap comprehension. | Coarse body state enough, or limb/wound UI needed. |
| [[decisions/dr-004-first-playable-slice]] | A-FEEL/MAT/REC/UX results and playtest notes. | Move to B, repeat A, or split A.1. |
| [[decisions/dr-005-multiplayer-posture]] | Event volume and terrain snapshot sizes. | Keep co-op as prototype-only, promote LAN/co-op track, or defer. |
| [[decisions/dr-007-terrain-material-model]] | Material readability and dirty-region performance. | Keep curated semantic materials, simplify, or research richer model. |
| [[decisions/dr-008-ai-architecture]] | Control-intent/event/item-label compatibility with bot harness. | Utility first, GOAP/commander later, or hybrid. |
| [[decisions/dr-009-command-ux-style]] | HUD/material/death/item explanation vocabulary. | Promote overlay language into command UX or revise. |

## First Implementation Agent Brief

Give an implementing agent this minimum brief:

1. Read `AGENTS.md`, [VAULT_PLAN.md](../../VAULT_PLAN.md), this page, [[spec/prototype-implementation-backlog-slice-a]], [[spec/actor-feel-sandbox-slice-a]], [[spec/replay-recorder-slice-a]], [[systems/replay-determinism-and-run-evidence]], [[spec/terrain-material-sandbox-slice-a]], and [[spec/ux-wireframes-slice-a]].
2. Create the prototype outside canonical reference repos.
3. Build A0..A3 first: lab shell, control intent, rifle loop, recorder ring buffer/export.
4. Do not add extra weapons, campaign systems, networking, or production art before A-FEEL-01/02 and REC-A-01/02 are testable.
5. Emit run bundles with manifest/events/summary/notes.
6. Update this vault with results, not just code.

## Open Follow-Ups

| Follow-Up | Destination |
|---|---|
| Add manual A1 play notes and connect LOAD-A bounded material responses to integrated collision/global path/AI-H evidence. | `prototype_runs/actor_feel_lab/`, [[prototypes/index]], [[spec/equipment-loadout-workbench-slice-a]], and future A1 manual feel evidence note. |
| Create a Slice B spec only after Slice A has run evidence. | `spec/squad-scenario-slice-b.md` later. |
| Keep run-bundle schemas/checker aligned with implementation output. | [[references/prototype-run-bundle-schema]] and `research_tools/prototype_run_check.py`. |
| Pick one moonshot to test in parallel without blocking A0..A3. | [[research-log/moonshot-register]] |

## Source Trail

- [[spec/actor-feel-sandbox-slice-a]]
- [[spec/prototype-implementation-backlog-slice-a]]
- [[spec/replay-recorder-slice-a]]
- [[systems/replay-determinism-and-run-evidence]]
- [[spec/terrain-material-sandbox-slice-a]]
- [[spec/ux-wireframes-slice-a]]
- [[spec/equipment-loadout-workbench-slice-a]]
- [[spec/ai-trust-harness-slice-a]]
- [[references/equipment-ai-behavior-contract]]
- [[references/prototype-run-bundle-schema]]
- [[prototypes/actor-feel-lab-a0-bootstrap]]
- [[prototypes/actor-feel-lab-a1-runtime-smoke]]
- [[prototypes/actor-feel-lab-a1-ui-smoke]]
- [[dashboards/research-readiness]]
- Game Design Skills, Game Feel: `https://gamedesignskills.com/game-design/game-feel/`
- GameDeveloper, Design 101: Playtesting: `https://www.gamedeveloper.com/design/design-101-playtesting`
- Games User Research, Playtest Kit: `https://gamesuserresearch.com/playtest-kit/`
- LoopKit session replay/playtesting product page: `https://loopkit.dev/`
