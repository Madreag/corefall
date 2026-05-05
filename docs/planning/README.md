# Planning Snapshot

This folder contains a copied planning snapshot from:

```text
/Users/erol/projects/cortex-command-repos-all
```

Canonical vault:

```text
/Users/erol/projects/cortex-command-repos-all/cortext_command_vault
```

Snapshot date: 2026-05-05.

## Why This Exists

Implementation agents working in `~/projects/corefall` need the roadmap, backlog, checklist, spec, and decision context locally. These files are copied here so an agent can start from this repo while still knowing where the canonical vault lives.

## Primary Files

| File | Purpose |
|---|---|
| [spec/authoritative-game-spec-v0.md](spec/authoritative-game-spec-v0.md) | Product direction and current game promise. |
| [spec/prototype-roadmap.md](spec/prototype-roadmap.md) | Milestone map, side tracks, CLI/control contracts, validation matrix, and definition of done. |
| [spec/native-implementation-backlog.md](spec/native-implementation-backlog.md) | Concrete task cards per milestone. |
| [spec/feature-completion-checklist.md](spec/feature-completion-checklist.md) | Feature/task checklist with human and AI rating fields. |
| [spec/ai-coder-reading-list.md](spec/ai-coder-reading-list.md) | What to hand to an AI worker before a milestone. |
| [spec/ai-control-observability-layer.md](spec/ai-control-observability-layer.md) | Eyes/ears/hands layer for tests and AI control. |
| [references/prototype-run-bundle-schema.md](references/prototype-run-bundle-schema.md) | Required evidence bundle format. |
| [decisions/index.md](decisions/index.md) | Decision record index. |
| [dashboards/decision-tracker.md](dashboards/decision-tracker.md) | Current DR status and open gates. |
| [root/GAME_DESCRIPTION_FOR_FRIEND.md](root/GAME_DESCRIPTION_FOR_FRIEND.md) | Friend-readable game pitch. |

## Refresh Rule

If the canonical vault changes, refresh this folder before assigning major work. If an implementation agent finds contradictions between the snapshot and the canonical vault, treat the canonical vault as more current and ask before making architecture-changing assumptions.
