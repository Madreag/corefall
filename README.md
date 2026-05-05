# Corefall

Corefall is the implementation repo for the tactical 2D physics sandbox currently planned in the Cortex Command research vault.

Working title: **Corefall**.

## What This Repo Is

This repo is for building the actual game prototype and eventual game code.

The current design direction lives in the copied planning packet under:

- [docs/planning/spec/prototype-roadmap.md](docs/planning/spec/prototype-roadmap.md)
- [docs/planning/spec/native-implementation-backlog.md](docs/planning/spec/native-implementation-backlog.md)
- [docs/planning/spec/feature-completion-checklist.md](docs/planning/spec/feature-completion-checklist.md)
- [docs/planning/spec/ai-coder-reading-list.md](docs/planning/spec/ai-coder-reading-list.md)
- [docs/planning/spec/authoritative-game-spec-v0.md](docs/planning/spec/authoritative-game-spec-v0.md)
- [docs/planning/root/GAME_DESCRIPTION_FOR_FRIEND.md](docs/planning/root/GAME_DESCRIPTION_FOR_FRIEND.md)

The canonical research vault remains:

```text
/Users/erol/projects/cortex-command-repos-all/cortext_command_vault
```

Treat the copied files in this repo as an implementation snapshot. If the canonical vault changes materially, refresh the snapshot before assigning large milestones.

## Implementation Shape

The planned native game workspace is expected to live under:

```text
cortex-game/
```

The roadmap currently targets a Rust + Bevy/wgpu hybrid with custom core crates. Do not start a milestone by guessing. Read [AGENTS.md](AGENTS.md) first, then follow the roadmap reading order.

## First Milestones

The expected start is:

1. M0 — Engine Bootstrap
2. M1 — Actor Controller And Sim Core
3. M1.5 — Micro Breach Fun Slice

Do not jump to M2 until M0/M1/M1.5 are coherent enough to give terrain, replay, control, and fun-loop evidence somewhere to attach.
