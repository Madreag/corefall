# Corefall

Corefall is the implementation repo for the tactical 2D physics sandbox planned in the Cortex Command research vault.

Working title: **Corefall**.

## What This Repo Is

This repo is for building the actual game prototype and eventual game code.

The canonical research and planning vault is not duplicated here:

```text
/Users/erol/projects/cortex-command-repos-all/cortext_command_vault
```

Primary planning entry points:

```text
/Users/erol/projects/cortex-command-repos-all/VAULT_PLAN.md
/Users/erol/projects/cortex-command-repos-all/DIRECTORY.md
/Users/erol/projects/cortex-command-repos-all/AGENTS.md
/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/spec/prototype-roadmap.md
/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/spec/native-implementation-backlog.md
/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/spec/feature-completion-checklist.md
/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/spec/ai-coder-reading-list.md
```

Use [AGENTS.md](AGENTS.md) for the exact AI worker read order.

## Repo Layout

```text
corefall-game/            # native game workspace will be scaffolded here (matches canonical roadmap name)
docs/implementation-log/  # milestone notes, evidence summaries, bug logs
prototype_runs/native/    # generated run bundles once prototypes exist
CHANGELOG.md              # repo-only implementation changelog
```

## First Milestones

The expected start is:

1. M0 - Engine Bootstrap
2. M1 - Actor Controller And Sim Core
3. M1.5 - Micro Breach Fun Slice

Do not jump to M2 until M0/M1/M1.5 are coherent enough to give terrain, replay, control, and fun-loop evidence somewhere to attach.

## Completion Discipline

After any feature or milestone implementation, update the canonical vault roadmap/checklist and the repo-local [CHANGELOG.md](CHANGELOG.md). The canonical roadmap and checklist remain in the research vault, not duplicated here.
