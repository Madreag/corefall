---
type: spec
status: exploratory-reqs
ready_when: "Workbench V1 ships internally; 3 mods migrated; loader parity fixtures pass CONTENT-A and PACK-A checks."
---

← [[spec/index|spec section]] · [[spec/package-builder-workbench-slice-a|package-builder Slice A]] · [[systems/modding-package-and-workbench|workbench brief]] · [[engine/content-module-loading-lifecycle|content/module lifecycle]] · [[references/content-loader-graph-cccp|generated loader graph]] · [[references/equipment-source-trace-slice-a|equipment source trace]] · [[engine/modding-data-lua|engine modding]] · [[comparables/openlierox-local-audit|OpenLieroX audit]]

# Modding Model

> [!summary] Purpose
> Define the future game's modding posture: human-editable content, deterministic packages, loader-parity validation, visible provenance, script capability labels, migration tooling, and equipment/item metadata that AI, UI, balancing, replay, backend, and creators all consume consistently.

> [!important] Posture
> Private reuse, copying, adapting, and aggressive experimentation are allowed. The modding model should make the best game and fastest creator iteration possible. Provenance, license, and package-trust fields are there so we can clean up before public release or server-pure publication; they must not block private prototyping.

## Model Shape

| Layer | Editable Form | Machine Form | Trust Question |
|---|---|---|---|
| Dev mount | Folder with `.rte`/manifest/source assets. | Live indexed project. | Can it run locally and explain warnings? |
| Local package | Deterministic archive built from a dev mount. | Manifest + file hashes + generated metadata. | Can it reproduce a run/replay on this machine or a friend's machine? |
| Published package | Immutable package with provenance and compatibility metadata. | Signed/registered manifest and content hash. | Can public servers/replays/support tools trust it? |
| Legacy import | Existing `.rte` or `.zip` module. | Imported project plus diagnostics and migration notes. | What must be fixed before it is package-clean? |
| User content | Scenes, saves, scripts, local experiments. | Mutable userdata project/package tier. | Can it be loaded without pretending to be server-pure content? |

## Loader Parity Rule

The future package system must model the current loader before replacing it. [[engine/content-module-loading-lifecycle]] is the source note for:

- official module order and official fallback.
- sorted `Mods/*.rte` scan.
- userdata modules loaded after mods.
- `Index.ini` versus `MergedIndex.ini`.
- module metadata and `SupportedGameVersion`.
- `IncludeFile` stack and wrong-case path checks.
- `CopyOf` resolution and source-path retention.
- duplicate preset collision/overwrite behavior.
- `ScanFolderContents` caveats.
- `.zip` extraction behavior.
- module/entity/movable script reload.

[[references/content-loader-graph-cccp]] is the current generated fixture for that rule. It captures the active official order, root include edges, script paths, duplicate same-module preset keys, absent userdata modules, and machine-readable graph data for workbench prototypes. [[references/equipment-source-trace-slice-a]] is the first equipment consumer of that graph: it joins role-card facts back to source files, include parents, duplicate preset pressure, trace-tab refs, and source confidence.

## Contract Areas

| Contract | Minimum Requirement | Why It Matters |
|---|---|---|
| Manifest | ID, display name, version, engine range, package type, dependencies, authors, license/provenance policy, entry points, capabilities. | Backend, replay, support reports, and creator UX need stable identity. |
| Source graph | Include graph, `CopyOf` graph, script graph, asset path graph, dependency graph. | Bad content should be explainable before engine launch. |
| Diagnostics | File, line, column, include stack, severity, package-mode verdict, first fix action. | Creator tools need actionable errors, not generic load failure. |
| Provenance | Per-file source, copied/adapted/generated/original status, license notes, release-cleanup notes. | Private work stays fast while public release stays possible. |
| Script capability | Declared terrain/entity/audio/UI/backend/filesystem/network capabilities. | Replay/server/package trust depends on script visibility. |
| Equipment metadata | Resolved item role, AI summary, UI summary, balance fields, replay/backend fields, source provenance. | Bots, buy/loadout UI, mission checks, and package diagnostics need one shared item meaning. |
| Migration | Rule IDs, preview diff, backups, diagnostics, post-migration validation. | Old mods should be brought forward without blind text replacement. |

## V1 Requirements

| Requirement | Evidence |
|---|---|
| Package manifests need per-file provenance and compatibility versions. | OpenLieroX asset provenance is unclear; future public release needs cleanup without blocking private prototyping. |
| Loader graph must mirror active CCCP behavior. | [[engine/content-module-loading-lifecycle]] traces `PresetMan`, `DataModule`, `Reader`, `Entity`, and `System` behavior. |
| Loader graph needs a generated starting fixture. | [[references/content-loader-graph-cccp]] scans the active checkout into JSON/Markdown with module order, include edges, scripts, hot files, diagnostics, and CONTENT-A coverage. |
| Equipment source diagnostics need source-position joins. | [[references/equipment-source-trace-slice-a]] joins role cards to loader files, include parents, duplicate preset hits, trace-tab refs, and direct/inherited/inferred/missing field counts. |
| Weapon/projectile behavior should have a visual effect graph. | OpenLieroX projectile actions can spawn children, carve, bounce, explode, change speed/radius, or chain additional actions. |
| Lua/script escape hatches must declare capabilities. | CCCP and OpenLieroX/Gusanos show scripting power; replay/network/AI validation need guardrails. |
| Pack workflows matter. | OpenLieroX downloads and `share/gamedir/` show level/mod/skin packs as major retention content. |
| Equipment diagnostics must use the same resolved fields as item roles. | [[references/equipment-device-loadout-field-atlas]], [[references/equipment-ai-behavior-contract]], and [[references/equipment-consumer-traceability-matrix]] already define the shared consumer contract. |

## Package Mode Verdicts

| Verdict | Meaning |
|---|---|
| `dev_ok` | Runs locally with visible warnings. Private experiments can continue. |
| `local_package_ok` | Deterministic local archive builds, hashes, and test-launches. |
| `published_ready` | Provenance, license, dependency, script, and diagnostics policy are clean enough for public registry/server-pure review. |
| `bot_default_blocked` | Item/content can exist, but AI should not use it by default until capability/AI fields are proven. |
| `replay_backend_blocked` | Content can run locally, but replay/server compatibility claims are not yet valid. |
| `migration_needed` | Legacy module can be imported but needs converter rules or manual fixes. |

## Acceptance Tests

| Test ID | Assertion |
|---|---|
| MOD-A-01 | A clean `.rte` fixture imports into dev-mount mode with module graph, include graph, preset graph, script list, and source paths. |
| MOD-A-02 | Loader parity fixture passes CONTENT-A checks from [[engine/content-module-loading-lifecycle]]. |
| MOD-A-03 | Published package mode fails unresolved include, wrong-case path, unresolved `CopyOf`, undeclared script capability, and duplicate preset without `replaces`. |
| MOD-A-04 | Equipment fixture imports role-card fields from resolved direct/inherited/manual sources and emits package diagnostics from [[references/equipment-package-diagnostics-slice-a]]. |
| MOD-A-05 | Legacy `.zip` import preserves source archive hash, skipped-file report, extracted path list, and provenance prompt. |
| MOD-A-06 | Test launch can run officials plus one selected dev module and export package diagnostics into a prototype run bundle. |

## Inputs

- [[systems/modding-package-and-workbench]]
- [[spec/package-builder-workbench-slice-a]]
- [[engine/content-module-loading-lifecycle]]
- [[references/content-loader-graph-cccp]]
- [[references/equipment-source-trace-slice-a]]
- [[engine/modding-data-lua]]
- [[repos/cccp-vscode-extension]]
- [[repos/legacy-mod-converter]]
- [[decisions/dr-006-modding-data-model]]
- [[references/usage-ledger]]
- [[references/equipment-device-loadout-field-atlas]]
- [[references/equipment-ai-behavior-contract]]
- [[references/equipment-consumer-traceability-matrix]]
- [[comparables/openlierox-local-audit]]
- [[comparables/the-powder-toy-local-audit]]
