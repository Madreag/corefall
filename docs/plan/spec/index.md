← [[index|vault home]] · [[dashboards/research-readiness|readiness]] · [[strategy/research-to-spec-roadmap|research roadmap]] · [root plan](../../VAULT_PLAN.md)

# Game Spec Section

> [!warning] Spec section is intentionally minimal
> Stub pages exist below for navigation and early thinking. Exploratory spec notes are allowed now. Full authoritative spec commitments wait on the gates in [[dashboards/research-readiness]] and the open decisions in [[dashboards/decision-tracker]]. Personal-project posture: continue prototyping; promote claims as settled only when evidence exists.

## Purpose

The spec section will turn the research vault into a concrete product and technical plan for our Cortex Command-like game.

It should be curated, opinionated, and implementation-facing, while the rest of the vault remains the broader knowledge base for:

- Cortex Command and CCCP/C4 source archaeology.
- Comparable game research.
- Mechanics, physics, AI, UX, networking, tooling, and retention notes.
- Source references and research logs.
- Design alternatives that were not chosen.

## Spec Writing Rules

| Rule | Why |
|---|---|
| Link every major settled claim back to evidence. | Prevents the spec from becoming detached from research. |
| Link every major decision back to a decision record. | Keeps pros/cons, risks, and rejected options visible during development. |
| Keep rejected alternatives visible. | Future decisions need to know why a route was avoided. |
| Separate product promise from implementation bets. | A good idea may be worth prototyping even before it is safe to promise. |
| Mark assumptions clearly. | Unknowns should drive prototypes, not hide inside prose. |
| Keep research notes alive after spec work starts. | The vault is the long-term knowledge base. |

## Planned Spec Pages

| Page | Status | Purpose | Source Areas | Gating Decisions |
|---|---|---|---|---|
| [[spec/product-promise]] | <span class="cc-flag cc-yellow">STUB</span> | Defines what this game is trying to be. | [[strategy/best-cortex-like-game-principles]], [[design/opportunities-for-our-fork]] | DR-001, DR-005 |
| [[spec/player-modes]] | <span class="cc-flag cc-yellow">STUB</span> | Solo, local co-op, online co-op posture; PvP as prototype track, not launch promise. | [[systems/ux-ui-and-retention]], [[systems/networking-backend-frontend]] | DR-005 |
| [[spec/core-loop]] | <span class="cc-flag cc-yellow">STUB</span> | Buy → deploy → command → fight → salvage → replay → improve. | [[game/player-loop-and-ux]], [[engine/loadout-delivery-economy-lifecycle]] | DR-004, DR-009 |
| [[spec/simulation-architecture]] | <span class="cc-flag cc-yellow">STUB</span> | Actor, terrain, materials, mobility affordances, physics, update order. | [[engine/architecture]], [[systems/physics-and-destruction-models]], [[systems/material-and-mobility-affordance-schema]] | DR-001, DR-007 |
| [[spec/terrain-material-sandbox-slice-a]] | <span class="cc-flag cc-blue">PROTOTYPE REQS</span> | Buildable terrain/material lab requirements, material fixture, overlay tests, dirty-region/path/replay event contract. | [[systems/material-and-mobility-affordance-schema]], [[engine/terrain-mutation-and-pathfinding-lifecycle]], [[spec/replay-recorder-slice-a]] | DR-007, DR-005, DR-008 |
| [[spec/replay-event-architecture]] | <span class="cc-flag cc-yellow">STUB</span> | Event log + snapshots + scrub. | [[systems/replay-event-architecture]] | DR-002 |
| [[spec/replay-recorder-slice-a]] | <span class="cc-flag cc-blue">PROTOTYPE REQS</span> | Buildable recorder/viewer requirements, event envelope, hook map, acceptance tests, first tickets. | [[systems/replay-event-architecture]], [[spec/actor-feel-sandbox-slice-a]], [[engine/direct-control-and-actor-feel-lifecycle]] | DR-002, DR-004 |
| [[spec/body-damage-model]] | <span class="cc-flag cc-yellow">STUB</span> | Wounds, armor, gibs, inventory fallout, readability. | [[engine/body-damage-wound-gib-lifecycle]], [[systems/damage-equipment-and-items]] | DR-003 |
| [[spec/ai-and-command]] | <span class="cc-flag cc-yellow">STUB</span> | Layered AI, jobs, utility, command UX. | [[systems/ai-and-bots]], [[systems/ai-trust-test-suite]], [[systems/ux-overlay-screen-brief]] | DR-008, DR-009 |
| [[spec/ai-trust-harness-slice-a]] | <span class="cc-flag cc-blue">PROTOTYPE REQS</span> | Buildable AI scenario runner, event contract, hook map, reports, overlay fields, AI-H tests. | [[systems/ai-trust-test-suite]], [[engine/ai-order-lifecycle]], [[spec/replay-recorder-slice-a]] | DR-008 |
| [[spec/equipment-loadout]] | <span class="cc-flag cc-yellow">STUB</span> | Roles, tools, weapons, delivery risk, AI metadata. | [[engine/loadout-delivery-economy-lifecycle]], [[systems/damage-equipment-and-items]] | DR-006 |
| [[spec/ux-ui-model]] | <span class="cc-flag cc-yellow">STUB</span> | HUD, squad panel, command overlay, replay viewer, workbench. | [[systems/ux-overlay-screen-brief]], [[systems/ux-ui-and-retention]] | DR-003, DR-009 |
| [[spec/modding-model]] | <span class="cc-flag cc-yellow">STUB</span> | Package, schema, validation, tooling, migration. | [[systems/modding-package-and-workbench]] | DR-006 |
| [[spec/backend-networking]] | <span class="cc-flag cc-yellow">STUB</span> | Launch posture; what's architected early; what's prototype-only for now. | [[systems/networking-backend-frontend]], [[engine/network-terrain-replication-lifecycle]], [[comparables/opensoldat-satellites-local-audit]] | DR-005 |
| [[spec/missions-and-objectives]] | <span class="cc-flag cc-yellow">STUB</span> | Destruction-aware mission patterns. | [[systems/destruction-objective-mission-patterns]], [[engine/activity-scenario-lifecycle]] | DR-007 |
| [[spec/actor-feel-sandbox-slice-a]] | <span class="cc-flag cc-blue">PROTOTYPE REQS</span> | Buildable requirements for the first actor-feel sandbox. | [[decisions/dr-004-first-playable-slice]], [[systems/material-and-mobility-affordance-schema]], [[systems/replay-event-architecture]] | DR-004 |
| [[spec/prototype-roadmap]] | <span class="cc-flag cc-yellow">STUB</span> | Build order, kill criteria, risk budget. | [VAULT_PLAN.md](../../VAULT_PLAN.md) | DR-004 |

## Readiness

Use [[dashboards/research-readiness]], [[dashboards/decision-tracker]], [[decisions/index]], and [VAULT_PLAN.md](../../VAULT_PLAN.md) as the gate before marking any stub as an authoritative spec page. Do not treat those gates as blockers for research notes, speculative sections, or private prototypes.
