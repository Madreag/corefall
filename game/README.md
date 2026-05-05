# Corefall Native Workspace

The native Rust workspace will live here. Directory name `game/` matches the canonical roadmap's [Repository Layout](https://.../prototype-roadmap.md#repository-layout) — no path mapping needed.

Do not scaffold this by guessing. Start M0 from the canonical roadmap and backlog:

```text
/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/spec/prototype-roadmap.md
/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/spec/native-implementation-backlog.md
/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/spec/feature-completion-checklist.md
```

Expected M0 work includes the Cargo workspace, toolchain files, CI, fixed-tick app shell, run-bundle writer, and initial `cf-control` / `cfctl` bootstrap.

Crate prefix `cf-` is the canonical workspace shorthand. Use it consistently for crates, commands, schemas, and scripts unless a future DR explicitly changes the naming convention.
