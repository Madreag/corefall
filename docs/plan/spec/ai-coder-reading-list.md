---
type: spec
status: live-handoff
authority: "Required reading order for AI implementation agents before starting any milestone or feature work."
last_updated: 2026-05-05
feeds:
  - DR-001
  - DR-002
  - DR-024
  - DR-026
---

← [[index|vault home]] · [[spec/index|spec section]] · [[spec/prototype-roadmap|native roadmap]] · [[spec/native-implementation-backlog|native backlog]] · [[spec/feature-completion-checklist|feature checklist]] · [[dashboards/decision-tracker|decision tracker]] · [[dashboards/research-readiness|readiness]] · [VAULT_PLAN.md](../../VAULT_PLAN.md)

# AI-Coder Reading List

> [!summary] Purpose
> The exact docs an AI implementation agent must read before starting any milestone or feature task. Use this when handing off work. Order matters; the first 8 are non-negotiable for every task.

> [!important] Hand this list (or a subset) to the worker
> When assigning a milestone or single roadmap feature, paste the relevant rows below into the agent prompt. The agent must confirm each read in the milestone vault note. The [[spec/prototype-roadmap#Open Decision Gates Protocol|Open Decision Gates Protocol]] applies to every task.

> [!tip] When assigning a review or bug hunt
> Add [[spec/ai-code-review-bug-hunt-skills]] to the read list. In Claude Code inside `/Users/erol/projects/corefall`, invoke `/corefall-review <milestone-or-range>`. Reviewer agents should run the separate diff review, full affected-code review, contract gap review, edge-case hunt, test audit, determinism/replay review, `cfctl` observability review, and final synthesis judge described there.
> Before any milestone is accepted, run `/corefall-review <milestone>`, fix every verified issue, and rerun `/corefall-review <milestone>` until it returns `Accept` or the user explicitly approves each exact remaining deferral.

## Mandatory For Every Task (Read In Order)

| # | Document | Why |
|---|---|---|
| 1 | `AGENTS.md` (root) | Project rules, license/reuse posture, vault structure, agent role definitions. |
| 2 | [[spec/authoritative-game-spec-v0]] | Canonical game spec: product promise, player fantasy, core loop, launch commitments, non-commitments, moonshots. |
| 3 | [[spec/prototype-roadmap]] | Native build roadmap. **Required subsections:** Read Order, Glossary, Agent Implementation Contract, [[spec/prototype-roadmap#Short Assignment Contract]], [[spec/prototype-roadmap#Open Decision Gates Protocol]], Milestone Handoff Template, Strategic Frame, Stack At A Glance, Repository Layout, Toolchain And Workspace Bootstrap, Per-Crate AGENTS.md Template, Logging/Tracing/Error Policy, Asset/Placeholder Strategy, Testing Layers, [[spec/prototype-roadmap#CLI Reference]], Control Transport And Envelope, Scenario Manifest Schema, Run-Bundle Naming Convention, Bug Log Format, Per-Milestone Kickoff Smoke, the assigned milestone's Detail section (with its **Open DR gates** row), Validation Command Matrix, Bug Hunt Checklist, Definition Of Done, Anti-Goals. |
| 4 | [[spec/native-implementation-backlog]] | Global Rules + Standard Validation + the assigned milestone's task cards. |
| 5 | [[spec/feature-completion-checklist]] | Update Rules For Agents + [[spec/feature-completion-checklist#Open Decision Gates Checklist]] + Server/MMO + Material/T-MAT addenda + the assigned milestone's scope/done/task rows. |
| 6 | [[spec/ai-control-observability-layer]] | Eyes/ears/hands rule. Every player-facing surface MUST be reachable from `cfctl`. |
| 7 | [[references/prototype-run-bundle-schema]] | Run-bundle schema, event category baseline, native milestone acceptance gates. |
| 8 | [[decisions/index]] + [[dashboards/decision-tracker]] | Current DR status + still-open topics. |

For a dedicated review task, also read [[spec/ai-code-review-bug-hunt-skills]] before inspecting code. In Claude Code, the project-local skill lives at `/Users/erol/projects/corefall/.claude/skills/corefall-review/SKILL.md`.

## Read Conditionally Per Milestone

| Assigned Milestone | Also Read |
|---|---|
| **M0 — Engine Bootstrap** | [[decisions/dr-001-engine-strategy]], [[decisions/dr-024-native-engine-stack]], [[decisions/dr-025-target-platforms]], [[decisions/dr-026-team-and-repo-model]], [[decisions/dr-002-replay-event-architecture]]. |
| **M1 — Actor Controller And Sim Core** | [[decisions/dr-001-engine-strategy]], [[decisions/dr-003-body-damage-readability]], [[decisions/dr-004-first-playable-slice]], [[decisions/dr-024-native-engine-stack]], [[decisions/dr-026-team-and-repo-model]], [[decisions/dr-002-replay-event-architecture]], [[spec/actor-feel-sandbox-slice-a]]. |
| **M1.5 — Micro Breach Fun Slice** | [[decisions/dr-002-replay-event-architecture]], [[decisions/dr-004-first-playable-slice]], [[decisions/dr-007-terrain-material-model]], [[decisions/dr-008-ai-architecture]], [[decisions/dr-009-command-ux-style]], [[prototypes/actor-feel-lab-a1-human-playtest-2026-05-04]]. |
| **M2 — Pixel Terrain And Materials** | [[decisions/dr-007-terrain-material-model]], [[decisions/dr-036-systemic-material-simulation-direction]], [[comparables/noita-grade-material-simulation-research]], [[systems/material-and-mobility-affordance-schema]], [[spec/terrain-material-sandbox-slice-a]]. |
| **M3 — Replay And Event Recorder** | [[decisions/dr-002-replay-event-architecture]], [[systems/replay-event-architecture]], [[systems/replay-determinism-and-run-evidence]], [[spec/replay-recorder-slice-a]]. **M3 closes DR-002.** |
| **M4 — HUD And Comic-Noir UI** | [[decisions/dr-003-body-damage-readability]], [[decisions/dr-009-command-ux-style]], [[decisions/dr-012-accessibility-comfort-readability]], [[decisions/dr-019-visual-direction]], [[systems/ux-overlay-screen-brief]], [[spec/ux-wireframes-slice-a]], [[spec/accessibility-comfort-slice-a]]. **M4 closes HUD-01..03 + ACC-A.** |
| **M5 — Equipment, Chassis, And Damage Grammar** | [[decisions/dr-003-body-damage-readability]], [[decisions/dr-014-tone-player-promise]], [[decisions/dr-018-death-meaning-and-consequence-ladder]], [[decisions/dr-021-mech-scale-and-archetypes]], [[spec/body-damage-model]], [[spec/chassis-armor-mechs-and-origins]], [[spec/equipment-loadout]], [[spec/equipment-loadout-workbench-slice-a]]. |
| **M5.5 — Full Collision Gauntlet** | [[decisions/dr-033-full-collision-physics-direction]], [[spec/full-collision-physics-plan]], [[systems/physics-and-destruction-models]]. |
| **M5.6 — Material Kernel** | [[decisions/dr-036-systemic-material-simulation-direction]], [[decisions/dr-007-terrain-material-model]], [[comparables/noita-grade-material-simulation-research]], [[systems/material-and-mobility-affordance-schema]]. |
| **M5.7 — Hazard Package** | [[decisions/dr-036-systemic-material-simulation-direction]], [[decisions/dr-003-body-damage-readability]], [[decisions/dr-018-death-meaning-and-consequence-ladder]], [[spec/body-damage-model]]. |
| **M6 — AI Core And Trust Harness** | [[decisions/dr-008-ai-architecture]], [[decisions/dr-022-ai-humanlike-bar]], [[systems/ai-and-bots]], [[systems/ai-trust-test-suite]], [[spec/ai-trust-harness-slice-a]]. **M6 closes DR-008.** |
| **M6.5 — LLM Mind Lab** | [[decisions/dr-032-hybrid-llm-ai-direction]], [[spec/hybrid-llm-ai-plan]]. |
| **M6.6 — AI Material Competence** | [[decisions/dr-036-systemic-material-simulation-direction]], [[decisions/dr-022-ai-humanlike-bar]], [[decisions/dr-008-ai-architecture]]. |
| **M7 — Mission Director And Breach Contract** | [[decisions/dr-004-first-playable-slice]], [[decisions/dr-011-progression-retention-loop]], [[decisions/dr-014-tone-player-promise]], [[decisions/dr-015-player-identity-control-posture]], [[decisions/dr-016-setting-and-world-frame]], [[decisions/dr-017-mission-generation-strategy]], [[decisions/dr-018-death-meaning-and-consequence-ladder]], [[decisions/dr-021-mech-scale-and-archetypes]], [[decisions/dr-022-ai-humanlike-bar]], [[decisions/dr-027-combat-base-scope]], [[spec/mission-director-slice-a]], [[spec/missions-and-objectives]], [[spec/command-core-base-power]], [[spec/progression-retention]]. **M7 closes DR-004.** |
| **M7.5 — Base Atmospherics** | [[decisions/dr-036-systemic-material-simulation-direction]], [[decisions/dr-027-combat-base-scope]], [[spec/server-app-architecture]] (server-authoritative atmosphere). |
| **M8 — Scenario Editor And Mod Tools** | [[decisions/dr-006-modding-data-model]], [[decisions/dr-010-license-reuse-matrix]], [[decisions/dr-017-mission-generation-strategy]], [[decisions/dr-030-scenario-editor-commitment]], [[spec/modding-model]], [[spec/package-builder-workbench-slice-a]], [[engine/content-module-loading-lifecycle]]. **M8 closes DR-006.** |
| **M8.5 — Material Lab** | [[decisions/dr-036-systemic-material-simulation-direction]], [[decisions/dr-006-modding-data-model]], [[decisions/dr-030-scenario-editor-commitment]]. |
| **M9 — Dedicated Server App** | [[decisions/dr-005-multiplayer-posture]], [[decisions/dr-013-backend-service-scope]], [[decisions/dr-034-dedicated-server-application]], [[spec/server-app-architecture]], [[spec/backend-networking]], [[spec/backend-service-hub-slice-a]]. Networking transport library MUST close in M9 or M10. |
| **M10 — LAN Co-op** | [[decisions/dr-005-multiplayer-posture]], [[decisions/dr-006-modding-data-model]], [[decisions/dr-013-backend-service-scope]], [[decisions/dr-034-dedicated-server-application]], [[spec/server-app-architecture]]. |
| **M11 — Online Co-op (Self-Hosted)** | [[decisions/dr-005-multiplayer-posture]], [[decisions/dr-006-modding-data-model]], [[decisions/dr-013-backend-service-scope]], [[decisions/dr-034-dedicated-server-application]], [[spec/server-app-architecture]]. |
| **M12 — Public PvP Arenas + Persistent MMO Shards** | [[decisions/dr-035-persistent-mmo-architecture]], [[decisions/dr-005-multiplayer-posture]], [[decisions/dr-011-progression-retention-loop]], [[decisions/dr-031-content-economy-and-monetization-posture]], [[decisions/dr-057-optional-gacha-battle-pass-and-private-prototype-license-posture]], [[spec/persistent-mmo-architecture]]. Anti-grind posture and DR-057 dormant/default-off optional economy hooks must be preserved. |

## Read Per Cross-Cutting Feature

| Feature Topic | Read |
|---|---|
| Any UI surface | [[spec/ai-control-observability-layer]] (must be reachable from `cfctl`), [[spec/ux-wireframes-slice-a]], [[spec/accessibility-comfort-slice-a]]. |
| Any AI behavior | [[decisions/dr-008-ai-architecture]], [[decisions/dr-022-ai-humanlike-bar]], [[decisions/dr-032-hybrid-llm-ai-direction]], [[spec/ai-trust-harness-slice-a]], [[spec/hybrid-llm-ai-plan]]. |
| Any equipment / loadout work | [[spec/equipment-loadout]], [[references/equipment-cccp-field-map]], [[references/equipment-device-loadout-field-atlas]], [[references/equipment-role-cards-slice-a]]. |
| Any networking work | [[spec/server-app-architecture]], [[spec/persistent-mmo-architecture]], [[decisions/dr-005-multiplayer-posture]], [[decisions/dr-013-backend-service-scope]], [[decisions/dr-034-dedicated-server-application]], [[decisions/dr-035-persistent-mmo-architecture]]. **Networking transport library is OPEN — confirm with user.** |
| Any modding / content work | [[decisions/dr-006-modding-data-model]], [[decisions/dr-010-license-reuse-matrix]], [[references/usage-ledger]], [[spec/modding-model]], [[spec/package-builder-workbench-slice-a]], [[engine/content-module-loading-lifecycle]], [[references/content-loader-graph-cccp]]. **Modding script host is OPEN — confirm with user.** |
| Any save / persistence work | [[decisions/dr-029-save-game-model]], [[spec/server-app-architecture]] (server persistence). **Cloud-save backend is OPEN (post-launch); do NOT add cloud deps.** |
| Any localization-affecting work | **Localization plan is OPEN** — flag any code path that bakes English-only strings; avoid hardcoded UI strings. |
| Any material / atmosphere / reaction work | [[decisions/dr-036-systemic-material-simulation-direction]], [[comparables/noita-grade-material-simulation-research]], [[systems/material-and-mobility-affordance-schema]]. |
| Any collision / physics work | [[decisions/dr-033-full-collision-physics-direction]], [[spec/full-collision-physics-plan]], [[systems/physics-and-destruction-models]]. |
| Any replay / event capture work | [[decisions/dr-002-replay-event-architecture]], [[systems/replay-event-architecture]], [[systems/replay-determinism-and-run-evidence]], [[spec/replay-recorder-slice-a]], [[references/prototype-run-bundle-schema]]. |
| Any LLM / mind worker | [[decisions/dr-032-hybrid-llm-ai-direction]], [[spec/hybrid-llm-ai-plan]]. |
| Any optional economy, cosmetic battle-pass, collection, or gacha-like hook | [[decisions/dr-011-progression-retention-loop]], [[decisions/dr-031-content-economy-and-monetization-posture]], [[decisions/dr-057-optional-gacha-battle-pass-and-private-prototype-license-posture]], [[spec/progression-retention]], [[spec/liveops-and-endgame]], [[spec/legal-and-compliance]]. Must default off, avoid power locks/FOMO, and expose `cfctl test economy-disabled`, `battle-pass-disabled`, and `no-power-locks`. |

## Hard Rules The Worker Must Always Follow

1. **[[spec/prototype-roadmap#Open Decision Gates Protocol|Open Decision Gates Protocol]]**: do not silently assume an OPEN DR's lean is locked. Confirm the lean from current evidence or ask the user through the active agent's available user-input/chat mechanism.
2. **Eyes/ears/hands rule**: every player-facing surface MUST be reachable from `cfctl`. If a UI lacks a `cf-control` path, the milestone is incomplete. See [[spec/ai-control-observability-layer]].
3. **Events first**: every behavior that affects player understanding/replay/AI/networking/save/debugging emits an event. See [[references/prototype-run-bundle-schema]].
4. **Reference repos are read-only**: never edit CCCP/C4/comparable repos.
5. **Run-bundle evidence required**: every meaningful run emits a checked run bundle under `prototype_runs/native/`.
6. **Checklist update required**: every completed task updates [[spec/feature-completion-checklist]] rows with evidence + AI self-ratings.
7. **DR closure protocol**: when a milestone closes a DR, update the DR file + [[decisions/index]] + [[dashboards/decision-tracker]] + [[dashboards/research-readiness]] + a dated [[research-log/index|research-log]] note in the same pass.

## Source Trail

- [[spec/prototype-roadmap]] (Open Decision Gates Protocol).
- [[spec/native-implementation-backlog]] (Global Rules).
- [[spec/feature-completion-checklist]] (Open Decision Gates Checklist).
- [[spec/ai-control-observability-layer]] (eyes/ears/hands coverage rule).
- [[references/prototype-run-bundle-schema]] (event categories + acceptance gates).
- [[dashboards/decision-tracker]] (live DR status).
