# Corefall Native Workspace

The native Rust workspace will live here. Directory name `corefall-game/` matches the canonical roadmap's [Repository Layout](https://.../prototype-roadmap.md#repository-layout) — no path mapping needed.

Do not scaffold this by guessing. Start M0 from the canonical roadmap and backlog:

```text
/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/spec/prototype-roadmap.md
/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/spec/native-implementation-backlog.md
/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/spec/feature-completion-checklist.md
```

Expected M0 work includes the Cargo workspace, toolchain files, CI, fixed-tick app shell, run-bundle writer, and initial `cx-control` / `cxctl` bootstrap.

Crate prefix `cx-` is preserved across the rename for stability of `cargo run -p cx-<name>` invocations and to keep existing AGENTS.md / decision records / task cards valid. The name `cx-` is just a short workspace prefix; renaming it would be a separate workspace-wide migration with its own DR.
