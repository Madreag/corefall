---
type: spec
status: stub
ready_when: "Workbench V1 ships internally; 3 mods migrated."
---

← [[spec/index|spec section]] · [[systems/modding-package-and-workbench|workbench brief]] · [[engine/modding-data-lua|engine modding]] · [[comparables/openlierox-local-audit|OpenLieroX audit]]

# Modding Model

> [!warning] Stub

## What goes here when ready

- Package format (manifest + assets + entry points + signature).
- Validation contract (static, runtime, behavioral).
- Workbench V1 scope.
- Lua API surface (typed; sandboxed by default).
- Migration story.

## Exploratory Requirements

| Requirement | Evidence |
|---|---|
| Package manifests need per-file provenance and compatibility versions. | OpenLieroX asset provenance is unclear; future public release needs cleanup without blocking private prototyping. |
| Weapon/projectile behavior should have a visual effect graph. | OpenLieroX projectile actions can spawn children, carve, bounce, explode, change speed/radius, or chain additional actions. |
| Lua/script escape hatches must declare capabilities. | CCCP and OpenLieroX/Gusanos show scripting power; replay/network/AI validation need guardrails. |
| Pack workflows matter. | OpenLieroX downloads and `share/gamedir/` show level/mod/skin packs as major retention content. |

## Inputs

- [[systems/modding-package-and-workbench]]
- [[engine/modding-data-lua]]
- [[repos/cccp-vscode-extension]]
- [[repos/legacy-mod-converter]]
- [[decisions/dr-006-modding-data-model]]
- [[references/usage-ledger]]
- [[comparables/openlierox-local-audit]]
- [[comparables/the-powder-toy-local-audit]]
