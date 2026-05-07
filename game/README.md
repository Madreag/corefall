# Corefall Native Workspace

This is the native Rust workspace for Corefall. Directory name `game/` matches the canonical roadmap's `Repository Layout` section in `/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/spec/prototype-roadmap.md` — no path mapping needed.

M0 and M1 are both closed (PRs #1 and #2 merged); M1.5 — Micro Breach Fun Slice — is the active milestone. The root [README](../README.md) is the public project overview. Start implementation work from the canonical roadmap and backlog:

```text
/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/spec/prototype-roadmap.md
/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/spec/native-implementation-backlog.md
/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/spec/feature-completion-checklist.md
```

The workspace currently contains 29 `cf-*` crates, the pinned Rust toolchain, CI validation, fixed-tick app shell, run-bundle writer, JSON-RPC control plane, `cfctl`, content validation, schema drift checks, and dependency drift reporting.

Crate prefix `cf-` is the canonical workspace shorthand. Use it consistently for crates, commands, schemas, and scripts unless a future DR explicitly changes the naming convention.

Useful local checks:

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
python3 tools/dependency_drift_report.py --workspace-root . --format markdown
```
