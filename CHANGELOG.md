# Changelog

Repo-only implementation changelog for Corefall.

The canonical roadmap, checklist, specs, and decision records remain in:

```text
/Users/erol/projects/cortex-command-repos-all/cortext_command_vault
```

Use this file to summarize what changed in the implementation repo. Do not copy the whole vault here.

## Unreleased

### Changed

- Clarified that short milestone prompts such as "Implement M0 from the roadmap" are complete assignments; workers must expand them through `AGENTS.md`, the canonical roadmap, backlog, checklist, and linked DRs without requiring a giant pasted handoff prompt.
- Standardized the planned native workspace directory as `game/` so the corefall repo matches the canonical roadmap's Repository Layout name. No mapping table is needed; `game/` is the workspace root in both the canonical docs and this repo.
- Tightened `AGENTS.md` per a pre-implementation review: added Repository Layout (canonical = this repo), Per-Crate AGENTS.md mandate, Standard Validation block with exact commands, Run-Bundle Naming, Git Hygiene, Secrets Posture, and a Do-Not list. Added vault home to the mandatory read order. Pinned `cfctl` invocation path and run-bundle root.

### Added

- Repo-local changelog and completion discipline requiring implementation agents to update the canonical vault roadmap/checklist after feature or milestone work.

## 2026-05-05

### Added

- Created the private `Madreag/corefall` implementation repository.
- Added root implementation instructions, a future native workspace under `game/`, and `docs/implementation-log/` for milestone evidence.

### Changed

- Slimmed the repo to use the canonical vault directly instead of maintaining a duplicated planning snapshot.
