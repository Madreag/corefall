# Spec Coherence Pass — Overview & Order Of Operations

**Status:** `active` (this is a planning document, not a milestone)
**Created:** 2026-05-12
**Owner:** Agent assigned to spec-coherence-pass

---

## Why this document exists

A deep-read pass over all 38 milestone specs (3 closed + 35 active = 18,769 lines) surfaced **4 hard ordering bugs**, **6 coherence sprawl issues**, and **2 mega-milestones** that benefit from splitting. This plan executes the fixes in priority order, with each tier in its own file so an agent can pick up the work, follow the edits, and commit incrementally.

The fixes are organized into 4 tiers by urgency:

| Tier | Risk if skipped | When to do | File |
|---|---|---|---|
| **Tier 1** | Blocks M2.2A implementation cleanly (hard dependency inversion + slot reservations + duplicate data tables) | **Before M2.2A starts** | [`COHERENCE-TIER-1.md`](COHERENCE-TIER-1.md) |
| **Tier 2** | Mega-milestones get hard to implement + review surface bloats | **Before BP7 starts** (after Tier 1) | [`COHERENCE-TIER-2.md`](COHERENCE-TIER-2.md) |
| **Tier 3** | Polish — spec-maintenance friction increases over time | Can run in parallel with Tier 2 | [`COHERENCE-TIER-3.md`](COHERENCE-TIER-3.md) |
| **Tier 4** | Missing milestones (deployment, schema-lock) — work happens scattered without them | Optional but recommended | [`COHERENCE-TIER-4.md`](COHERENCE-TIER-4.md) |

---

## Order of operations (strict)

```
┌────────────────────────────────────────────────┐
│ Tier 1 (BLOCKING)                              │
│   1.1 Fix M7.8 ↔ M8.6 dependency inversion     │
│   1.2 Unify SmelterFurnace location            │
│   1.3 Add tank slots to M2.2A inventory        │
│   1.4 Tighten M5.8 (move battery/tank/race-env)│
└────────────────────────────────────────────────┘
                       │
                       ▼  must merge first
┌────────────────────────────────────────────────┐
│ Tier 2 (RECOMMENDED before BP7)                │
│   2.1 Split M7 into M7 + M7.1 + M7.2           │
│   2.2 Split M11.5 into M11.5 + M11.6 + M11.7   │
│   2.3 Centralize boss schema in M7             │
│   2.4 Add hunger/thirst as M5.7 afflictions    │
└────────────────────────────────────────────────┘
                       │
       ┌───────────────┼───────────────┐
       ▼ parallel      ▼ parallel
┌──────────────┐  ┌──────────────┐
│ Tier 3       │  │ Tier 4       │
│  3.1 M2.5 split│  │  4.1 M0.5    │
│  3.2 Storyteller│ │  4.2 M11.4   │
│  3.3 Cross-refs │ │              │
│  3.4 Procgen   │  │              │
└──────────────┘  └──────────────┘
```

**Critical rule:** Do not start Tier 2 until Tier 1 PR has merged. Tier 1 changes file ownership in ways that Tier 2 builds on.

---

## How an agent uses these files

For each tier:

1. **Read the tier file end-to-end.** Each tier file is self-contained — you do not need to read other tier files or this overview while executing a tier.
2. **For each edit (e.g., "Edit 1.1"):**
   - Read the **Goal** + **Files to modify** + **Specific changes** sections
   - Make the changes following the **before / after** snippets
   - Run the **Acceptance criteria** verification (commands listed)
   - Commit with the suggested message template
3. **At the end of each tier:**
   - Run the **Tier acceptance criteria** (full-tier verification)
   - Open a PR with all commits from this tier
   - Use the suggested PR title + body template
4. **Per PR autonomy:** all edits in a tier can ship in one PR (one PR per tier is recommended).

---

## Working agreements

### File locations

- All milestone specs: `/Users/erol/projects/corefall/specs/active/<id>.md`
- Closed specs: `/Users/erol/projects/corefall/specs/done/<id>.md` (do NOT modify these — they're audit-trail)
- README: `/Users/erol/projects/corefall/README.md`
- Coherence plan files: `/Users/erol/projects/corefall/specs/COHERENCE-*.md`

### Edit conventions

- **Move = cut from source + paste to target + add cross-reference at source.** The source spec should say "Defined canonically in `<target spec> § <section>`" instead of duplicating content.
- **Split = create new file + cut subsystem from source + update source to reference new file.** The original spec stays focused on its core scope.
- **Surface lock = schema/event/contract stays at locking milestone; producers ladder up at later milestones.** (This is the existing pattern — don't fight it.)

### Active spec count tracking

After each tier, the active spec count changes:

| State | Active milestone count | README badge |
|---|---|---|
| Now | 35 | `35 (M2.2A..M12)` |
| After Tier 1 | 36 (added M7.6.5) | `36 (M2.2A..M12)` |
| After Tier 2 | 40 (added M7.1, M7.2, M11.6, M11.7) | `40 (M2.2A..M12)` |
| After Tier 3 | 41 (added M2.5-SCHEMA) | `41 (M2.2A..M12)` |
| After Tier 4 | 43 (added M0.5, M11.4) | `43 (M0.5..M12)` |

Every tier's PR **must** update the README badge + the planning spine reference + the BP table.

### Commit discipline

- One logical change per commit (e.g., "Edit 1.1: split M8.6 into M7.6.5 + M8.6" is one commit even if it touches 4 files).
- Use the commit message templates in each tier file (under each edit's "**Commit message**" section).
- All commits in a tier go in one PR.

### Rust workspace impact

The spec edits do NOT change any Rust code. Crate definitions (`game/crates/cf-*/`) are unaffected by spec reorganization until implementers consume the new specs. So `cargo build`, `cargo test`, `cargo clippy` should all pass before AND after each tier's PR.

If your tier's PR breaks the build, you've over-reached. Stop and revert.

---

## Risk register — known concerns + mitigations

| Risk | Mitigation |
|---|---|
| Tier 2's M7 split fragments the campaign vertical-slice work | Split files reference one canonical "M7 family" closure procedure; closure runs across M7 + M7.1 + M7.2 together |
| Tier 2's M11.5 split fragments PvE Survival mode | Same — closure runs across M11.5 + M11.6 + M11.7 together |
| Cross-references between specs break if a file is renamed | Every edit lists exact file paths; verify all `[link](path)` references after each edit |
| Active spec README badge drift | Each tier's PR has a mandatory README update step in acceptance criteria |
| Reviewers struggle to follow a big PR | Each commit in a PR is ONE logical edit (per the convention above); review per-commit, not per-PR |

---

## Cross-references

Companion documents that supplement this plan (consult only if blocked):

- **`AGENTS.md`** — workflow for implementation (NOT for spec edits, but useful for context on how implementers consume specs)
- **`README.md`** — current BP table + spec counts (will be updated by each tier's PR)
- **`docs/plan/decisions/`** — decision records (cross-referenced from many specs)

You do NOT need to read these to execute the coherence pass. They are background.

---

## Final acceptance — all tiers complete

When Tier 1, 2, 3, 4 PRs have all merged:

1. ✅ M7.8 has no hard dependency on M8.6 (Tier 1)
2. ✅ SmelterFurnace appears in exactly one spec (Tier 1)
3. ✅ M2.2A inventory has 3 reserved tank slots (Tier 1)
4. ✅ M5.8 references M7.6 / M5.9 / M5.10 for battery / tank / race-env data (Tier 1)
5. ✅ M7 + M7.1 + M7.2 each cover one coherent scope (Tier 2)
6. ✅ M11.5 + M11.6 + M11.7 each cover one coherent scope (Tier 2)
7. ✅ Boss schema defined once in M7, referenced elsewhere (Tier 2)
8. ✅ M5.7 has 22 afflictions (was 18; added hunger/thirst/sleep_dep/sanity_low) (Tier 2)
9. ✅ M2.5 + M2.5-SCHEMA split cleanly (Tier 3)
10. ✅ Storyteller API documented in M7 (Tier 3)
11. ✅ Damage-model specs have cross-reference headers (Tier 3)
12. ✅ M11.5 procgen acceptance covers all 12 worlds (Tier 3)
13. ✅ M0.5 — Schema Locks milestone exists (Tier 4)
14. ✅ M11.4 — Self-Hosted Server Deployment milestone exists (Tier 4)
15. ✅ README badge shows 43 active specs
16. ✅ README BP table reflects all new + split milestones
17. ✅ `cargo build` + `cargo test` + `cargo clippy` all green
18. ✅ `cargo run -p cf-mod -- validate content/` exits 0 (no spec/content drift)

When all 18 boxes are checked, the spec coherence pass is complete and M2.2A implementation can proceed cleanly.

---

## After this pass: next agent's instruction

When the coherence pass is fully merged, the next agent's instruction is to:

1. Begin M2.2A implementation per `AGENTS.md` workflow
2. Read `specs/active/M2.2A.md` only (no longer encumbered by coherence sprawl)
3. Audit current code; fill gaps; report per-scenario verdict table

The coherence pass is done. M2.2A implementation begins clean.
