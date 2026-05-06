# Contract Integrity Review

This pass catches "green but wrong" AI implementation failures. Run it after diff/full-code review and before verdict.

## Failures To Hunt

- **Parallel path drift:** `cf-app`, `cfctl`, `cf-control`, server, replay, scenario loading, or metadata generation each builds similar config/state differently.
- **Fake success:** a command returns accepted/ok/PASS but ignores the requested field or does not mutate state.
- **Permissive required fields:** a mandatory field can be missing or malformed and still passes.
- **Source-truth mismatch:** bundle/observation/checklist claims use defaults or hardcodes instead of the loaded scenario, active config, current binary, or current git state.
- **Checklist laundering:** a row is checked while notes say required work is deferred, reserved, missing, fake, stubbed, placeholder, or "later".
- **Happy-path-only proof:** tests prove default behavior but not negative inputs, alternate paths, or cross-path equivalence.

## Required Probes

For each contract path, produce both positive and negative/adversarial proof:

| Contract Path | Positive Proof | Negative / Adversarial Proof |
|---|---|---|
| Scenario/config loading | CLI/app/control path reads the same loaded manifest values. | Change seed/expected_tests/tick rate and prove every path reflects it or rejects unsupported override. |
| Control API validation | Valid request succeeds with expected state/event. | Missing/malformed mandatory `schema_version` and invalid params reject with structured error. |
| Command semantics | Accepted command changes state or emits the promised event. | Unsupported field or unsupported state transition rejects; no silent no-op success. |
| Run-bundle metadata | Bundle records real scenario/build/config/runtime metadata. | Tool path and app path produce equivalent metadata for same scenario/config. |
| Event evidence | Required task-card events appear in `events.jsonl`. | Missing event would fail a test/checker/contract assertion. |
| Checklist/docs | Checked row has direct evidence and no hidden deferral wording. | Search notes for deferred/follow-up/stub/reserved/fake/missing and uncheck or fix rows. |

## Fix Rule

Every verified issue fixed during stabilization must add or update a regression proof that would have failed before the fix. If the proof is a live command rather than a unit test, record the exact command and run-bundle/result path.
