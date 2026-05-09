---
type: decision
id: DR-006
status: open
priority: P1
revisit_trigger: "When the workbench V1 ships internally and three external mods are migrated."
---

← [[decisions/index|decision records]] · [[systems/modding-package-and-workbench|modding workbench]] · [[engine/modding-data-lua|engine modding/data/lua]] · [[repos/cccp-vscode-extension|VS Code extension]] · [[comparables/openlierox-local-audit|OpenLieroX audit]]

# DR-006: Modding Data Model

> [!info] Status: OPEN; LEAN: schema-first + script escape hatches + workbench

## Context

Modding is product, not afterthought. Cortex's longevity is creator content. We must choose a data model that:
- Validates content before runtime errors.
- Supports a Lua-style escape hatch for unique behavior.
- Allows migration from existing CC mods.
- Plays well with replay/event capture (DR-002) and networking (DR-005).

See [[systems/modding-package-and-workbench]] and [[engine/modding-data-lua]].

## Options

| Option | Summary |
|---|---|
| A. Free-form INI + Lua (CCCP-style) | Status quo. |
| B. Schema-first (typed manifest) + Lua | Schema validates; Lua optional for unique behavior. |
| C. Editor-first (visual scenes/devices) + script | Drag-drop authoring; export to data. |
| D. Pure Lua with no schemas | Everything is script. |
| E. Domain-specific language (DSL) compiled to engine | New custom language. |

## Pros And Cons

| Option | Pros | Cons | Unknowns |
|---|---|---|---|
| A | Familiar to CC mod authors; lowest migration friction. | Validation fragile; broken mods at runtime. | Whether community accepts incremental tightening. |
| B | Strong validation; clear contract; mods scale. | Bigger initial dev cost; some authors miss freeform feel. | Schema scope creep. |
| C | Lower onboarding bar; visual feedback. | Editor maintenance is permanent. | Whether power users tolerate visual-first. |
| D | Total flexibility. | Brittle; hard to debug; poor IDE support. | None worth investing. |
| E | Tightest contract. | Long tail of language features needed; community resistance. | Tooling cost. |

## Evaluation

| Lens | A | B | C | D | E |
|---|---|---|---|---|---|
| Player value | Mods at all | Reliable mods | Many mods | Brittle | Few mods |
| Readability | Medium | High | Highest | Low | Highest |
| AI burden | Lua AI fragile | Typed AI inputs | Typed AI inputs | Most fragile | Most strict |
| UX burden | Authors fight INI | Authors fight schema diffs | Authors learn editor | Authors fight nothing | Authors learn DSL |
| Performance risk | Lua hot paths | Compiled-ish | Editor-built data | Lua heavy | Compiled |
| Modding impact | High volume, low quality | High volume + quality | Lower volume, higher quality | Highest volume, lowest quality | Lowest volume, highest quality |
| Networking/replay impact | Hard to capture state | Easy to capture | Easy | Hardest | Easiest |
| Content cost | Lowest | Medium | Highest | Lowest | Highest |
| Retention upside | Strong | Strongest | Medium | Medium | Lowest |
| Ethics/fairness | Hard to audit | Auditable | Auditable | Hardest | Easiest |

## Evidence

| Evidence | Source | Confidence |
|---|---|---|
| `Index.ini`, `CopyOf`, and pathing are fragile in CCCP. | `Data/Base.rte/Index.ini`, [[engine/modding-data-lua]] | High |
| VS Code extension already encodes grammar/snippets/path validation. | [[repos/cccp-vscode-extension]] | High |
| Legacy converter proves migration works as a workflow. | [[repos/legacy-mod-converter]] | High |
| Lua is everywhere in CCCP AI/activity behaviors. | `Data/Base.rte/AI/*`, `Data/Base.rte/Activities/*` | High |
| Modern engines tend to schema-first plus scripted escape hatches. | Industry standard. | Medium |
| Powder Toy source shows strong creator value from data-first element properties, Lua hooks, stamps/saves, and undo. | [[comparables/the-powder-toy-local-audit]] | High |
| OpenLieroX classic game scripts compile `main.txt`, weapon files, projectile files, rope/worm settings, and chained projectile actions; Gusanos adds event weapons and Lua/network bindings. | [[comparables/openlierox-local-audit]] | High |
| OpenLieroX bundled mods/levels/skins/packs show that content packaging is a retention surface, but asset provenance can become a public-release problem. | [[comparables/openlierox-local-audit]] | High |

## Current Recommendation

Recommendation: **B. Schema-first manifest + Lua escape hatches + workbench**.

- Manifest: typed, versioned, validated; required.
- Asset structure: keep INI-friendly hierarchy (Devices/, Actors/, Scenes/) so authoring stays familiar.
- Lua: opt-in, sandboxed, with typed bindings.
- Workbench: validated editing, asset preview, material lab, package/sign/publish.
- Weapon/effect graph: show projectile hit/timer/death actions, child projectile spawns, carve/damage events, and script hooks before runtime.
- Pack manager: support mod packs, level packs, skin packs, dependencies, provenance fields, and compatibility versions.
- Migration: legacy converter rules port forward; manual notes auto-generated.

Why: highest reliability without losing creator velocity; replay/event-friendly; supports networking; strongest community trust.

## Prototype Or Validation Plan

| Test | What It Proves | Pass/Fail |
|---|---|---|
| Workbench V1 + sample mod (one device, one activity). | Authoring + validation works. | Pass = no manual steps to publish. |
| Weapon-action graph preview. | Chained projectile/mod behavior is understandable before runtime. | Pass = a modder can explain every hit/timer/death outcome from the graph. |
| Pack provenance scan. | Public-release cleanup remains possible. | Pass = every imported file has source/license/status fields, even if marked unknown. |
| Migrate three real CCCP mods. | Migration is real. | Pass = playable; Fail = systemic gaps. |
| Static validator catches 90%+ of real-world mod errors. | Validation pays off. | Pass = > 90% in test suite. |
| Lua sandbox prevents filesystem/network by default. | Trust model. | Pass = blocked unless opt-in. |

## Risks

| Risk | Mitigation |
|---|---|
| Schema drift breaks community trust. | Versioned schemas; migrations bundled. |
| Lua escape hatches abused for filesystem/network. | Sandbox by default; capability declarations. |
| Workbench dev cost overruns runtime work. | Time-box editor scope; ship V1 minimal. |
| Compatibility with CCCP mods incomplete. | Document supported subset; converter rules. |

## Revisit Trigger

Reopen this decision when:

- Workbench V1 ships internally.
- Three external mods migrated; we know the long-tail issues.
- Networking model (DR-005) settles; affects mod hash sync.
- A community surveys finds a strong preference shift.

## Source Trail

- [[systems/modding-package-and-workbench]]
- [[engine/modding-data-lua]]
- [[repos/cccp-vscode-extension]]
- [[repos/legacy-mod-converter]]
- [[systems/replay-event-architecture]]
- [[comparables/the-powder-toy-local-audit]]
- [[comparables/openlierox-local-audit]]
