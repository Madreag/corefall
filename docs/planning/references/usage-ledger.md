← [[references/sources|sources]] · [[decisions/dr-010-license-reuse-matrix|license/reuse posture]] · [[index|vault home]]

# Usage Ledger

> [!info] Purpose
> Track what was actually copied, reused, depended on, or directly inspired-by. Personal/private use is fine; this ledger exists so a future public-release decision is easy: walk this list, classify each item, replace or relicense as needed.
>
> See [[decisions/dr-010-license-reuse-matrix|DR-010]] for tiering.

## How To Use This Ledger

When code, an asset, a dependency, or a prose fragment from an external source enters our future project:

1. Add a row to the appropriate table below.
2. Fill in: what, where it came from, where it landed, license tier, and (only if planning public release) replacement plan.
3. Keep entries concise but specific; "from CCCP" is too vague — give the file path and commit.

## Code Reuse

| Date | What | Source repo + path + commit | Where used in our project | License | Tier | Replacement plan if public release |
|---|---|---|---|---|---|---|
| _none yet_ |  |  |  |  |  |  |

## Asset Reuse (art / audio / sprites / sounds)

| Date | Asset | Source | Where used | License | Tier | Replacement plan |
|---|---|---|---|---|---|---|
| _none yet_ |  |  |  |  |  |  |

## Schema / Convention Inspiration

> Schemas and conventions (e.g. INI shape, `CopyOf` semantics, naming) are largely uncopyrightable. Track them anyway for audit clarity.

| Date | Convention | Source | Where applied | Notes |
|---|---|---|---|---|
| _none yet_ |  |  |  |  |

## Dependencies

| Date | Dependency | Version | License | Tier | Replacement plan |
|---|---|---|---|---|---|
| _none yet_ |  |  |  |  |  |

## Prose / Documentation Reuse

| Date | What | Source | Where used | Notes |
|---|---|---|---|---|
| _none yet_ |  |  |  |  |

## Pre-Release Audit Checklist

Use this when a public release becomes plausible:

- [ ] Walk every row above.
- [ ] Confirm license per the source's current `LICENSE` (it may have changed).
- [ ] For Tier 2 (GPL/AGPL): decide to accept terms or replace.
- [ ] For Tier 3 (proprietary): negotiate license or replace.
- [ ] For Tier 4 (unknown): clarify with upstream or replace.
- [ ] Build a NOTICES file from Tier 0/1 entries.
- [ ] Commission or replace any borrowed asset that isn't cleared.
- [ ] Engage actual legal review.

## Source Trail

- [[decisions/dr-010-license-reuse-matrix]]
- [[references/sources]]
- [[systems/modding-package-and-workbench]]
