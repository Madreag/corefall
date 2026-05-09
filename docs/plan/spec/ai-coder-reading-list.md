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

← [[index|vault home]] · [[spec/index|spec section]] · [[spec/prototype-roadmap|native roadmap]] · [[spec/native-implementation-backlog|native backlog]] · [[spec/feature-completion-checklist|feature checklist]] · [[dashboards/decision-tracker|decision tracker]] · [[dashboards/research-readiness|readiness]] · `cortex-command-repos-all/VAULT_PLAN.md` (research vault root)

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
| 3 | [[spec/prototype-roadmap]] | Native build roadmap. **Required subsections:** Read Order, Glossary, Agent Implementation Contract, [[spec/prototype-roadmap#Short Assignment Contract]], [[spec/prototype-roadmap#Minimum Bar And Enhancement Rule]], [[spec/prototype-roadmap#Open Decision Gates Protocol]], Milestone Handoff Template, Strategic Frame, Stack At A Glance, Repository Layout, Toolchain And Workspace Bootstrap, Per-Crate AGENTS.md Template, Logging/Tracing/Error Policy, Asset/Placeholder Strategy, Testing Layers, [[spec/prototype-roadmap#CLI Reference]], Control Transport And Envelope, Scenario Manifest Schema, Run-Bundle Naming Convention, Bug Log Format, Per-Milestone Kickoff Smoke, [[spec/prototype-roadmap#Build Points (Roadmap V2)]], [[spec/prototype-roadmap#Design-Completeness Map]], the assigned milestone's Detail section (with its **Open DR gates** row), Validation Command Matrix, Bug Hunt Checklist, Definition Of Done, Anti-Goals. |
| 4 | [[spec/native-implementation-backlog]] | Global Rules + Standard Validation + the assigned milestone's task cards. |
| 5 | [[spec/feature-completion-checklist]] | Update Rules For Agents + [[spec/feature-completion-checklist#Open Decision Gates Checklist]] + Server/MMO + Material/T-MAT addenda + Build Points Checklist row for the assigned BP + the assigned milestone's scope/done/task rows. |
| 6 | [[spec/milestone-enhancement-pass-m1-plus]] | **Universal Enhancement Done-Criteria (DR-056)** — the universal contract layered onto every M1..M12 milestone PLUS per-milestone specifics (perf gate per tier + network sync verification + CLI testability matrix + AI audio integration + juice/feel coverage + ACC-A floor + localization keyed strings + modding parity + 24h memory-leak soak + replay determinism + anti-FOMO + anti-pay-to-win audit + full-subtitle option). **A milestone is not complete unless every Universal row PASSES on top of the milestone's own Done-criteria.** |
| 7 | [[spec/ai-control-observability-layer]] | Eyes/ears/hands rule. Every player-facing surface MUST be reachable from `cfctl`. |
| 8 | [[references/prototype-run-bundle-schema]] | Run-bundle schema, event category baseline, native milestone acceptance gates. |
| 9 | [[decisions/index]] + [[dashboards/decision-tracker]] | Current DR status + still-open topics. |

For a dedicated review task, also read [[spec/ai-code-review-bug-hunt-skills]] before inspecting code. In Claude Code, the project-local skill lives at `/Users/erol/projects/corefall/.claude/skills/corefall-review/SKILL.md`.

## Read Conditionally Per Milestone

| Assigned Milestone | Also Read |
|---|---|
| **M0 — Engine Bootstrap** | [[decisions/dr-001-engine-strategy]], [[decisions/dr-024-native-engine-stack]], [[decisions/dr-025-target-platforms]], [[decisions/dr-026-team-and-repo-model]], [[decisions/dr-002-replay-event-architecture]]. |
| **M1 — Actor Controller And Sim Core** | [[decisions/dr-001-engine-strategy]], [[decisions/dr-003-body-damage-readability]], [[decisions/dr-004-first-playable-slice]], [[decisions/dr-024-native-engine-stack]], [[decisions/dr-026-team-and-repo-model]], [[decisions/dr-002-replay-event-architecture]], [[spec/actor-feel-sandbox-slice-a]]. |
| **M1.5 — Micro Breach Fun Slice** | [[decisions/dr-002-replay-event-architecture]], [[decisions/dr-004-first-playable-slice]], [[decisions/dr-007-terrain-material-model]], [[decisions/dr-008-ai-architecture]], [[decisions/dr-009-command-ux-style]], [[prototypes/actor-feel-lab-a1-human-playtest-2026-05-04]]. |
| **BP2 — Terrain & Replay Build** | Read the M2, M2.5, and M3A rows below as one unit, plus [[spec/prototype-roadmap#Build Points (Roadmap V2)]], [[spec/prototype-roadmap#T-CAPTURE — Frame Capture, Grid Composer, And BP Fun-Proof Automation]], [[spec/prototype-roadmap#T-RELEASE — Per-BP Cross-Platform GitHub Releases]], [[spec/milestone-enhancement-pass-m1-plus]] (Universal Enhancement Done-Criteria + per-milestone M2/M2.5/M3 specifics), [[decisions/dr-052-network-sync-rollback-and-cli-testable-determinism]] (M3A determinism CI matrix), [[decisions/dr-053-ai-audio-pipeline-realtime-and-generative]] (AI audio hooks), [[decisions/dr-054-performance-optimization-and-profiling]] (per-tier perf budgets + multicore + GPU posture), [[decisions/dr-055-game-feel-juice-and-flow-state]] (juice rules for digging/carving/hits), [[decisions/dr-056-per-milestone-enhancement-pass-m1-plus]] (universal enhancement contract), [[references/prototype-run-bundle-schema]], and the BP2 row in [[spec/feature-completion-checklist]]. BP2 is not closed until M2 + M2.5 + M3A all pass, the Universal Enhancement Done-Criteria PASS for each, and the BP-level note/review/playtest/release gates are satisfied. |
| **M2 — Pixel Terrain And Materials** | [[decisions/dr-007-terrain-material-model]], [[decisions/dr-036-systemic-material-simulation-direction]], [[comparables/noita-grade-material-simulation-research]], [[systems/material-and-mobility-affordance-schema]], [[spec/terrain-material-sandbox-slice-a]], [[spec/milestone-enhancement-pass-m1-plus#M2 — Pixel Terrain And Materials]] (M2 specifics: GPU compute path investigation, SIMD material kernel update, streaming asset budget per scenario, cold-load benchmark in CI), [[decisions/dr-054-performance-optimization-and-profiling]], [[decisions/dr-055-game-feel-juice-and-flow-state]] (carving juice rules: pixel debris, dust burst, tool-validity feedback). |
| **M2.5 — Micro Reactor Defense Fun Slice** | [[decisions/dr-002-replay-event-architecture]], [[decisions/dr-004-first-playable-slice]], [[decisions/dr-007-terrain-material-model]], [[decisions/dr-008-ai-architecture]], [[systems/material-and-mobility-affordance-schema]], [[spec/prototype-roadmap#M2.5 — Micro Reactor Defense Fun Slice]], [[spec/ai-control-observability-layer]], [[spec/milestone-enhancement-pass-m1-plus#M1.5 — Micro Breach Fun Slice]] (micro-fun-slice playtest pattern + adaptive difficulty toggle + AI difficulty preset visibility per DR-050), [[decisions/dr-055-game-feel-juice-and-flow-state]] (reactor hp juice + camera punch on hit). Use M1.5 as the pattern, but M2 chunked terrain must be the reason the scenario is fun. |
| **M3A — Event Recorder Core** | [[decisions/dr-002-replay-event-architecture]], [[systems/replay-event-architecture]], [[systems/replay-determinism-and-run-evidence]], [[spec/replay-recorder-slice-a]], [[references/prototype-run-bundle-schema]], [[spec/prototype-roadmap#M3 — Replay And Event Recorder]], [[spec/milestone-enhancement-pass-m1-plus#M3 — Replay And Event Recorder]] (per-tick checksum + replay determinism CI matrix per platform + replay branching + replay editing tools prototype + replay sharing infrastructure), [[decisions/dr-052-network-sync-rollback-and-cli-testable-determinism]] (cross-platform determinism CI matrix; `cfctl test sync-drift / replay-determinism / cross-platform-determinism / replay-bit-identical`). **M3A refreshes DR-002 but does not close it; M3B closes DR-002.** |
| **M3B — Replay Viewer And Debrief** | [[decisions/dr-002-replay-event-architecture]], [[systems/replay-event-architecture]], [[systems/replay-determinism-and-run-evidence]], [[spec/replay-recorder-slice-a]], [[spec/ux-wireframes-slice-a]]. **M3B closes DR-002 after viewer/debrief/cause-chain evidence passes.** |
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

1. **Roadmap is the minimum bar, not the ceiling**: the assigned milestone's documented Done-criteria are the floor. Per [[spec/prototype-roadmap#Minimum Bar And Enhancement Rule]] every worker MUST run a design-coverage pass before acceptance and strengthen any underspecified player-facing behavior, physics consequence, AI-readable state, UI/readability state, replay event, `cfctl` observation/action, perf counter, save field, accessibility hook, or modding/schema surface that is implied by the product promise but underspecified in the task card. Static, fake, no-op, non-readable, non-observable, non-replayable versions of core promises are unacceptable.
2. **Universal Enhancement Done-Criteria (DR-056) apply to every M1+ milestone**: per-tier perf gate (Steam Deck 800p/60 + 1080p/60 + 4K/120) + CI bench regression + 24h memory-leak soak + network sync + replay determinism CI matrix + all player surfaces scriptable via `cfctl` + AI-agent-driven validation report + AI audio cues per DR-053 + juice rules per DR-055 + ACC-A floor + Tier-A localization keys + modding parity + anti-FOMO + anti-pay-to-win + full-subtitle option. See [[spec/milestone-enhancement-pass-m1-plus]] for the universal rows AND the per-milestone specifics.
3. **[[spec/prototype-roadmap#Open Decision Gates Protocol|Open Decision Gates Protocol]]**: do not silently assume an OPEN DR's lean is locked. Confirm the lean from current evidence or ask the user through the active agent's available user-input/chat mechanism.
4. **Eyes/ears/hands rule + Self-Play Validation**: every player-facing surface MUST be reachable from `cfctl`. The agent (you) MUST drive the game through the production cf-control / cfctl path, observe the result through the production observe/inspect surface, and capture *visible* evidence through cf-capture frame readback. Source-truth + unit tests are necessary but not sufficient. Every milestone closeout report includes the Self-Play Validation Matrix from `corefall/AGENTS.md` § Self-Play Validation Rule. See [[spec/ai-control-observability-layer]].
5. **Events first**: every behavior that affects player understanding/replay/AI/networking/save/debugging emits an event. See [[references/prototype-run-bundle-schema]].
6. **Reference repos are read-only**: never edit CCCP/C4/comparable repos.
7. **Run-bundle evidence required**: every meaningful run emits a checked run bundle under `prototype_runs/native/`.
8. **Checklist update required**: every completed task updates [[spec/feature-completion-checklist]] rows with evidence + AI self-ratings.
9. **DR closure protocol**: when a milestone closes a DR, update the DR file + [[decisions/index]] + [[dashboards/decision-tracker]] + [[dashboards/research-readiness]] + a dated [[research-log/index|research-log]] note in the same pass.
10. **BP closure gate is additive**: every Build Point that bundles the milestone closes only when EVERY milestone inside it PASSES Acceptance Matrix + Contract Integrity Matrix + Universal Enhancement Done-Criteria + per-BP human-playtest survey + T-CAPTURE summary grid (BP2+) + T-RELEASE tagged release (BP1+) + `/corefall-review <bp>` Accept verdict.

## Source Trail

- [[spec/prototype-roadmap]] (Open Decision Gates Protocol).
- [[spec/native-implementation-backlog]] (Global Rules).
- [[spec/feature-completion-checklist]] (Open Decision Gates Checklist).
- [[spec/ai-control-observability-layer]] (eyes/ears/hands coverage rule).
- [[references/prototype-run-bundle-schema]] (event categories + acceptance gates).
- [[dashboards/decision-tracker]] (live DR status).
