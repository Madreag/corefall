## Summary

<!-- One-paragraph summary of what this PR does and why. -->

## Milestone / BP

<!-- e.g., M5 / BP3 -->

## Checklist

- [ ] `cargo fmt --all --check` clean
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] `cargo test --workspace` passes
- [ ] `cargo run -p cf-control --example dump_schemas -- --check` passes
- [ ] `cargo run -p cf-mod -- validate content/` passes
- [ ] Status surfaces updated (README + checklist + roadmap + CHANGELOG)
- [ ] `bash game/tools/check_status_surfaces.sh` passes
- [ ] Run-bundle evidence produced (if gameplay surface changed)
- [ ] Per-crate AGENTS.md updated (if crate API changed)

## Acceptance Matrix

<!-- One row per done-criterion from the roadmap/backlog. -->

| Criterion | Status | Evidence |
|---|---|---|

## Contract Integrity Matrix

<!-- Shared code paths + negative/adversarial proof. -->

| Contract | Positive Proof | Negative Proof |
|---|---|---|
