# Corefall Review Passes

Use these passes for every serious review. Run them as separate mental passes so diff review does not replace full code review.

## 1. Diff Impact Review

- Read every changed line and every deletion.
- Check public API changes against all callers.
- Check new dependencies, features, env vars, configs, schemas, and CLI flags.
- Check tests added or changed in the diff.
- Report only concrete issues with file/line evidence.

## 2. Full Affected-Code Review

- Open full touched files, not only hunks.
- Search usages with `rg`.
- Read tests, fixtures, scenario manifests, schemas, run-bundle consumers, crate `AGENTS.md`, and direct callers.
- Check crate ownership against the roadmap.
- Look for stale patterns, duplicate helpers, bad boundaries, dead code, hidden coupling, or abstraction drift.

## 3. Spec Contract Gap Review

Build this matrix:

| Contract | Source | Evidence | Status | Gap |
|---|---|---|---|---|
| Roadmap requirement | milestone section | code/test/run bundle/log | Pass/Fail/Partial/Not yet testable | exact missing work |
| Backlog task card | native backlog | code/test/run bundle/log | Pass/Fail/Partial/Not yet testable | exact missing work |
| Checklist row | feature checklist | evidence cell | Pass/Fail/Partial | missing update |
| DR gate | decision record | implementation/research-log evidence | Pass/Fail/Partial | confirm/ask/update |
| `cfctl` surface | AI control layer | command/result | Pass/Fail/Partial/Not yet testable | missing observe/inspect/act |

No milestone is complete if a required contract has no evidence.

## 4. Edge-Case Path Hunter

Check:

- Empty input, missing optional field, unknown field, invalid enum/value.
- 0, 1, maximum, negative, and overflow boundaries where relevant.
- Tick 0, final tick, paused sim, shutdown during work.
- Duplicate IDs and out-of-order events.
- Concurrent command arrival, lost connection, timeout, canceled request.
- Bad path, missing file, permission error, case-sensitive path differences.
- Floating point and integer overflow.
- Non-deterministic iteration.
- Panic paths and `unwrap` on user-controllable input.

## 5. Test Quality Audit

For each test:

- What behavior does it protect?
- What real bug would make it fail?
- Is the expected value independently derived?
- Does it cover an error path?
- Is it deterministic and isolated?
- Does the milestone need unit, integration, property, fuzz, replay, run-bundle, E2E, or `cfctl` tests?

Reject decorative tests that only prove code runs.

## 6. Rust Safety And Dependency Review

Check:

- `cargo fmt --all --check`
- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- New dependencies, feature flags, license posture, advisories, and build scripts.
- `unsafe`, panics, indexing, arithmetic overflow, thread safety, cross-platform assumptions.
- Stable serialization and schema versioning.

Use `cargo deny`, `cargo audit`, `miri`, `loom`, `proptest`, `cargo-fuzz`, and `nextest` once configured or relevant.

## 7. Corefall-Specific Review

Always check:

- Fixed tick is the only gameplay time source.
- Seeded deterministic RNG is used for sim decisions.
- Event IDs, event order, `schema_version`, and run-bundle files are stable.
- `run_manifest.json`, `events.jsonl`, `summary.json`, and `notes.md` exist when required.
- `cfctl` can observe/inspect/act on every player-facing surface added by the milestone.
- Accessibility flags are not bolted on late.
- Performance risks are called out against 4K/120 and fixed-tick budgets.
- Vault checklist, roadmap, implementation log, and changelog were updated.

Future M9-M12 networking will amplify every determinism weakness found now.

## 8. Zero Known Issues Gate

- Every verified finding at every severity must be fixed before milestone acceptance.
- Low severity means "small fix", not "safe to carry forward".
- Medium severity means "fix now", not "later by default".
- High/Blocker findings require a stabilization pass before any next milestone work.
- A finding can remain only with explicit user-approved deferral for that exact issue.
- User-approved deferrals must record issue ID, reason, owner, next milestone/checkpoint, and evidence path in the implementation log, changelog, checklist, and roadmap/DR docs when relevant.

## 9. Contract Integrity Gate

- Do not trust green commands without proving the contract path.
- Check app/tool/server paths for duplicated config, metadata, scenario loading, schema validation, and run-bundle logic.
- Check that every accepted command either changes state or clearly rejects unsupported semantics.
- Check mandatory fields by sending missing/malformed inputs through the live protocol.
- Check that run-bundle and checklist claims come from real runtime sources, not hardcoded defaults.
- Check every verified bug fix has a regression proof that would have failed before the fix.
