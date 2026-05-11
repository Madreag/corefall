# Build Point BP3 — Combat Readability Build

**Status:** Active (closure gate pending Wave 1 foundation repair)

## Constituent Milestones

| Milestone | Status | Closing Evidence |
|---|---|---|
| M3B — Replay Viewer + Debrief | Landed | commit `50af435` |
| M4A — Readability + ACC-A Floor | Landed | PR #27 |
| M5 — Equipment, Chassis, Damage Grammar | Landed | commit `29edc1b` |
| T-RELEASE v0.3.0-prealpha | SKIPPED | Double-Click Playability Hard Gate not met |

## BP3 Closure Blockers

BP3 closure gate has NOT passed. The `docs/MISSING_FEATURES.md` Wave 1 inventory
identifies ~1,100 foundation gaps across M0-M3B closure debt that must be resolved
before `bp_close_loop.sh bp3` can produce an all-phases PASS verdict.

Key blocker categories:
- Status surface drift (README/checklist/roadmap claims don't match reality)
- Feature-completion-checklist BP2 rows were never updated with evidence
- Missing cfctl scripts, scenarios, schemas, tests
- Missing per-crate AGENTS.md updates for M5 promotions
- Missing documentation (implementation logs, review reports, BP closure note)
- Missing CI gates (status surface check, println/unwrap/thread_rng lints)

## DRs Closed at BP3

- DR-002 (Replay/event architecture) — closed at M3B
- DR-003 (Body damage readability) — closed at M4A
- DR-012 (Accessibility floor) — closed at M4A
- DR-014 (Tone / player promise) — closed at M5
- DR-021 (Mech ladder) — closed at M5

## AI-Agent Self-Test Report

**Pending.** Will be filled when Wave 1 completes and the self-play sweep
produces an all-PASS verdict matrix.

## BP Goal Coverage Report

**Pending.** Will be filled when Wave 1 completes.

## LLM-Graded Verdict

**Pending.** Current partial verdicts:
- m5_chassis_wreck_eject: 9.23/10 PASS
- m5_chassis_salvage: 9.13/10 PASS
- m4a_micro_breach_readability: 8.89/10 PASS

Full BP3 verdict requires Wave 1 completion.
