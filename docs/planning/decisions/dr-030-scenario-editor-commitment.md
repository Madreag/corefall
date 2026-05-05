---
type: decision
id: DR-030
status: closed-direction
priority: P0
closed_at: 2026-05-04
revisit_trigger: "Editor authoring proves too steep for typical players; or the same-manifest contract collides with engine evolution; or an early playtest shows the editor doesn't drive return play."
---

← [[decisions/index|decision records]] · [[dashboards/decision-tracker|decision tracker]] · [[spec/prototype-roadmap|native build roadmap]] · [[decisions/dr-006-modding-data-model|DR-006]] · [[decisions/dr-017-mission-generation-strategy|DR-017]]

# DR-030: Scenario Editor First-Class Commitment

> [!success] Status: CLOSED-DIRECTION (project owner committed 2026-05-04)
> **First-class scenario editor at launch.** Same typed manifest format for official campaign missions, procedural contracts, and player-authored scenarios. The editor is part of the base game (not a paid DLC tool), runs in-engine, and exports `.cxpkg` packages that load like any other content.

## Decision

**Build the scenario editor as a first-class shipping feature, not a community-tool add-on.** It uses the exact same manifest format (per [[decisions/dr-017-mission-generation-strategy]]) the engine and director consume internally. Authoring is in-engine (workbench mode in `cx-tools-editor`) with hot-reload, test-run, and export.

## What This Locks In

| Aspect | Commitment |
|---|---|
| Launch surface | Editor ships in the base game (not paid DLC). |
| Manifest format | Single typed manifest for engine, director, editor, procedural generator, and player scenarios. |
| Authoring mode | In-engine workbench with hot-reload and test-run. |
| Export | Deterministic `.cxpkg` archives; loadable by any other player without recompilation. |
| Validation | Editor runs the same validators as `cx-mod` package validator: missing fields, broken refs, AI policy violations, accessibility floor checks per DR-012. |
| Procedural generation | Procedural contracts use the same manifest schema; the generator is one of multiple author paths. |
| Sharing | Local export at launch; backend-mediated sharing post-launch per [[decisions/dr-013-backend-service-scope]]. |
| Mod relationship | The editor IS the mod authoring tool for scenarios. Other content classes (chassis, equipment, AI doctrines) use sibling `cx-mod` workflows per DR-006. |

## What This Does NOT Lock

- Specific UI library for the editor (egui by default).
- Whether the editor supports collaborative editing (post-launch).
- The procedural generator's specific algorithms (ongoing R&D).
- Whether scenarios can be packaged with custom script logic at launch (depends on Lua/Rhai decision in M5).

## Why Not The Alternatives

| Alternative | Why Rejected |
|---|---|
| Editor as paid DLC | Cuts off the modding/UGC retention loop; competes with the "first-class" promise. |
| External-only editor (separate exe / web) | Can't hot-reload against the live engine; duplicates manifest schema. |
| Procedural-only (no editor) | Loses anchor missions (per DR-017) and player-authored scenarios. |
| Anchor-mission only (no procedural / no player editor) | Underdelivers replayability; loses retention from creator challenges. |
| Editor that uses a different manifest from the engine | Format drift kills the "one manifest" contract. |

## Evidence Trail

- Project owner verbatim (2026-05-04 stack round): "First-class scenario editor at launch. Same manifest format for official, procedural, and player-authored content."
- DR-017 already committed to manifest-first hybrid (anchor + procedural + player-authored).
- [[spec/package-builder-workbench-slice-a]] establishes the deterministic build surface.
- [[systems/modding-package-and-workbench]] catalogues authoring + validation primitives.
- [[research-log/moonshot-register]] flags creator/community amplification as a P1 retention vector.

## Risks

| Risk | Mitigation |
|---|---|
| Editor authoring is too steep | Onboarding labs per DR-023 include an editor mini-tutorial; templates for common scenarios. |
| Manifest schema collides with engine evolution | Migration handlers per DR-029; editor refuses to load incompatible schemas with a clear message. |
| AI-validation false positives block authors | Validators emit reasoned warnings, not silent fails. Workbench shows "fixable" vs "blocking" issues. |
| Sharing latency / quality control post-launch | Sharing posture is local-export at launch; backend-mediated sharing is a post-launch DR. |
| Editor competes with engine UI for `cx-render-2d` cycles | Editor mode is a workbench layer that can disable game rendering during heavy authoring. |

## Prototype / Validation Plan

| Test | What It Proves |
|---|---|
| M7 — Breach Contract proof mission ships from the same manifest the editor uses. | Manifest contract holds. |
| M8 — Player authors a Breach Contract variant in the editor; exports; loads back; runs. | Authoring round-trip works. |
| M8 — Validator catches a missing objective ref before export. | Validation is real. |
| M8 — Sample mod with editor-authored scenario passes the package builder. | Editor-mod relationship works. |
| M11 — Two players load the same player-authored scenario; package hash matches; mission runs. | Editor + multiplayer share schema. |

## Revisit Trigger

- Editor authoring proves too steep for typical players (M8 playtest signal).
- Same-manifest contract collides with engine evolution (a milestone forces a non-migrating schema change).
- Editor doesn't drive return play (post-launch retention signal).
- A specialized authoring tool (e.g., AI doctrines, chassis sculpting) needs a different surface.

## Source Trail

- Project owner stack-round answers (2026-05-04).
- [[decisions/dr-006-modding-data-model]]
- [[decisions/dr-017-mission-generation-strategy]]
- [[decisions/dr-023-tutorial-and-onboarding-strategy]]
- [[spec/missions-and-objectives]]
- [[spec/mission-director-slice-a]]
- [[spec/package-builder-workbench-slice-a]]
- [[systems/modding-package-and-workbench]]
- [[spec/prototype-roadmap]] — M8 Scenario Editor and Mod Tools.
- [[research-log/2026-05-04-roadmap-rebuild-native-stack]]
