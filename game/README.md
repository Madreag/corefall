# Corefall Native Workspace

This is the native Rust workspace for Corefall. Directory name `game/` matches the canonical roadmap's `Repository Layout` section in `/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/spec/prototype-roadmap.md` — no path mapping needed.

BP1 is closed: M0 Engine Bootstrap, M1 Actor Controller And Sim Core, and M1.5 Micro Breach Fun Slice are all merged and reviewed. BP2 is the active build point: M2 Pixel Terrain And Materials, M2.5 Micro Reactor Defense, and M3A Event Recorder Core / headless replay. The root [README](../README.md) is the public project overview. Start implementation work from the canonical Roadmap V2 and backlog:

```text
/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/spec/prototype-roadmap.md
/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/spec/native-implementation-backlog.md
/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/spec/feature-completion-checklist.md
```

The workspace currently contains the `cf-*` crates, pinned Rust toolchain, CI validation, fixed-tick app shell, run-bundle writer, JSON-RPC control plane, `cfctl`, content validation, schema drift checks, dependency drift reporting, M1 actor controller, M1.5 micro-breach fun slice, and T-CAPTURE frame/grid evidence tooling.

Crate prefix `cf-` is the canonical workspace shorthand. Use it consistently for crates, commands, schemas, and scripts unless a future DR explicitly changes the naming convention.

Useful local checks:

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
python3 tools/dependency_drift_report.py --workspace-root . --format markdown
```
