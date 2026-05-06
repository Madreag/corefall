# cf-mod — AGENTS.md

## Owns
- (M0 stub) Will own scenario/package manifest validation, deterministic `.cfpkg` builder, provenance scanner, loader graph, script-host integration.

## Common Pitfalls
- Modding script host (mlua vs Rhai) is OPEN. Confirm with user before locking the host (DR-006 closure milestone is M8).

## Source Trail
- spec/modding-model.
- spec/package-builder-workbench-slice-a.
- DR-006 / DR-010.
