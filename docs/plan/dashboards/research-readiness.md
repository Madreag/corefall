← [[index|vault home]] · [[dashboards/index|dashboard hub]] · [[dashboards/navigation-map|navigation map]] · [[dashboards/system-heatmap|system heatmap]]

# Research Readiness

> [!info] What this page is
> Tracker for **authoritative spec commitments** specifically. Vault expansion, ambitious research, feature brainstorming, reuse experiments, and private prototyping continue regardless. License/reuse is **not** a gate — it lives in [[references/usage-ledger]].

## Spec Readiness Gates

| Gate | Status | Evidence | Next Move |
|---|---|---|---|
| Repo inventory complete | <span class="cc-flag cc-green">DONE</span> | [[repos/index]] | Refresh commit snapshots if repos change. |
| Source list complete enough | <span class="cc-flag cc-green">DONE</span> | [[references/sources]] | Add article snapshots only if a spec claim needs exact citation. |
| Direct actor-control lifecycle | <span class="cc-flag cc-green">DONE</span> | [[engine/direct-control-and-actor-feel-lifecycle]], [[spec/actor-feel-sandbox-slice-a]] | Use this when implementing Slice A input/replay hooks. |
| Projectile lifecycle | <span class="cc-flag cc-green">DONE</span> | [[engine/projectile-to-impact-lifecycle]] | Tied to body damage downstream. |
| Body damage / wound / gib lifecycle | <span class="cc-flag cc-green">DONE</span> | [[engine/body-damage-wound-gib-lifecycle]] | Build HUD-01..HUD-03 prototypes. |
| Terrain mutation / pathfinding | <span class="cc-flag cc-green">DONE</span> | [[engine/terrain-mutation-and-pathfinding-lifecycle]] | Prototype dirty-region performance. |
| Activity / scenario lifecycle | <span class="cc-flag cc-green">DONE</span> | [[engine/activity-scenario-lifecycle]] | Audit `MetaFight.lua` save schema. |
| Actor AI lifecycle | <span class="cc-flag cc-green">DONE</span> | [[engine/ai-order-lifecycle]] | Build runtime AI trust harness. |
| Loadout / delivery lifecycle | <span class="cc-flag cc-green">DONE</span> | [[engine/loadout-delivery-economy-lifecycle]] | Convert into BUY-01 wireframes. |
| Networking terrain replication | <span class="cc-flag cc-green">DONE</span> | [[engine/network-terrain-replication-lifecycle]] | Prototype bandwidth at peak combat. |
| Replay/event architecture brief | <span class="cc-flag cc-green">DONE</span> | [[systems/replay-event-architecture]], [[spec/replay-recorder-slice-a]] | Implement recorder + viewer from Slice A requirements. |
| Destruction-objective patterns | <span class="cc-flag cc-green">DONE</span> | [[systems/destruction-objective-mission-patterns]] | Build first proof mission. |
| UX overlay / screen brief | <span class="cc-flag cc-green">DONE</span> | [[systems/ux-overlay-screen-brief]] | Wireframe + run acceptance tests. |
| Modding workbench brief | <span class="cc-flag cc-green">DONE</span> | [[systems/modding-package-and-workbench]] | Sketch workbench V1 scope. |
| AI trust suite (design) | <span class="cc-flag cc-yellow">DRAFT</span> | [[systems/ai-trust-test-suite]], [[spec/ai-trust-harness-slice-a]] | Implement runnable harness. |
| AI trust harness Slice A requirements | <span class="cc-flag cc-blue">READY TO BUILD</span> | [[spec/ai-trust-harness-slice-a]] | Implement AI-H-01..AI-H-06 after recorder basics exist. |
| Decision records (DR-001..DR-010) | <span class="cc-flag cc-green">DONE</span> | [[decisions/index]] | Resolve DRs as evidence accumulates. |
| Comparable workspace skeleton | <span class="cc-flag cc-green">STARTED</span> | `comparables_repos/README.md`, `comparables_repos/opensoldat`, `comparables_repos/the-powder-toy`, `comparables_repos/openlierox` | Add satellite repos only when needed. |
| Local comparable audits | <span class="cc-flag cc-yellow">PARTIAL</span> | [[comparables/opensoldat-local-audit]], [[comparables/opensoldat-satellites-local-audit]], [[comparables/the-powder-toy-local-audit]], [[comparables/openlierox-local-audit]], [[comparables/audit-template]], [[spec/actor-feel-sandbox-slice-a]] | Convert remaining audit findings into backend/frontend service requirements and workbench package-builder requirements. |
| Material/mobility schema proposal | <span class="cc-flag cc-yellow">DRAFT</span> | [[systems/material-and-mobility-affordance-schema]], [[spec/simulation-architecture]] | Test the smallest field set in actor-feel + terrain sandbox. |
| Replay/event recorder Slice A requirements | <span class="cc-flag cc-blue">READY TO BUILD</span> | [[spec/replay-recorder-slice-a]] | Implement ring buffer, JSONL export, snapshots, event tail, and death recap in the actor-feel sandbox. |
| Replay/event recorder prototype | <span class="cc-flag cc-orange">MISSING</span> | [[spec/replay-recorder-slice-a]] | Build alongside actor-feel sandbox. |
| Terrain/material sandbox Slice A requirements | <span class="cc-flag cc-blue">READY TO BUILD</span> | [[spec/terrain-material-sandbox-slice-a]] | Implement MAT-T-01..MAT-T-10 with recorder/path/AI metrics. |
| First playable slice (DR-004 A) | <span class="cc-flag cc-orange">MISSING</span> | [[decisions/dr-004-first-playable-slice]], [[spec/actor-feel-sandbox-slice-a]] | Implement time-boxed Slice A requirements. |
| Prototype roadmap | <span class="cc-flag cc-yellow">DRAFT</span> | Milestone estimates and kill criteria. | In [VAULT_PLAN.md](../../VAULT_PLAN.md). |
| Spec section shell | <span class="cc-flag cc-green">DONE</span> | [[spec/index]] | Stub pages added. |

## Readiness Progress

| Area | Progress |
|---|---|
| Repository research | `██████████` 100% |
| Cortex code diagrams | `██████████` 100% |
| Design briefs (replay, destruction, UX, modding) | `██████████` 100% |
| Decision records | `██████████` 100% |
| Online comparable research | `█████████░` 90% |
| AI trust plan | `████████░░` 80% |
| AI trust runnable harness | `░░░░░░░░░░` 0% |
| Local comparable code audit | `███████░░░` 70% |
| Replay/event recorder requirements | `██████░░░░` 60% |
| Replay/event recorder prototype | `░░░░░░░░░░` 0% |
| First playable prototype | `░░░░░░░░░░` 0% |
| UX wireframes | `░░░░░░░░░░` 0% |

## Next Three Artifacts

| Rank | Artifact | Why This Comes Next |
|---|---|---|
| 1 | [[spec/actor-feel-sandbox-slice-a|Actor-feel sandbox prototype requirements -> implementation]] | Requirements now exist; building it unlocks AI trust + body damage + replay tests and fastest path to "is this fun?" |
| 2 | [[spec/replay-recorder-slice-a|Replay/event recorder + viewer implementation]] | Requirements now exist; foundation for AI debugging, death recap, support, and future networking. |
| 3 | [[spec/terrain-material-sandbox-slice-a|Terrain/material sandbox implementation]] | Requirements now exist; next step is running MAT-T-01..MAT-T-10 with recorder/path/AI metrics. |

## Done Means

The vault is ready to mark spec pages as authoritative when:

- Every high-risk row in [[dashboards/system-heatmap]] has either code evidence, prototype evidence, or an explicit "not a launch promise yet" decision.
- Every spec-critical feature has a decision record with options, pros/cons, risks, and revisit triggers (DR-001..DR-010 satisfy this).
- No spec-critical gate above is `MISSING` without a linked next action (yellow/orange/draft is fine).
- Settled spec commitments can cite a note, code path, source URL, decision record, or prototype result for every major claim.
