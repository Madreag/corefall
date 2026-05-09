# Corefall Planning Spine

This directory contains the **implementation-gating planning layer** for
Corefall. It lives in the implementation repo (rather than the separate
research vault) so that every PR that changes a roadmap row, checklist
row, DR, or other gating contract is reviewed by Bugbot + Devin alongside
the implementation that depends on it. Atomic plan + code PRs.

If you're an AI agent picking up a milestone assignment, start at:

1. [`spec/ai-coder-reading-list.md`](spec/ai-coder-reading-list.md) — entry point + per-milestone read list.
2. [`spec/prototype-roadmap.md`](spec/prototype-roadmap.md) — Roadmap V2 + Build Points BP0..BP12 + Design-Completeness Map.
3. [`spec/native-implementation-backlog.md`](spec/native-implementation-backlog.md) — per-milestone task cards.
4. [`spec/feature-completion-checklist.md`](spec/feature-completion-checklist.md) — per-milestone done-criteria rows + Build Points Checklist.
5. [`spec/milestone-enhancement-pass-m1-plus.md`](spec/milestone-enhancement-pass-m1-plus.md) — Universal Enhancement Done-Criteria (DR-056).
6. [`decisions/index.md`](decisions/index.md) — every DR + open/closed status.
7. [`dashboards/decision-tracker.md`](dashboards/decision-tracker.md) — DR status dashboard.
8. [`references/prototype-run-bundle-schema.md`](references/prototype-run-bundle-schema.md) — run-bundle event categories + acceptance gates.

The full mandatory read order for a milestone assignment is documented in
[`/Users/erol/projects/corefall/AGENTS.md`](../../AGENTS.md) §Mandatory Read Order Before Any Milestone.

## Layout

```
docs/plan/
├── spec/                 — 80 spec pages: roadmap, backlog, checklist,
│                            milestone enhancement, AI-coder reading list,
│                            ai-control-observability-layer, authoritative
│                            game spec, plus 74 linked system specs
│                            (atmospherics, body damage, chassis, equipment,
│                            mission director, etc.)
├── decisions/            — every DR (DR-001 through DR-057) + index.md
├── dashboards/           — decision-tracker.md + research-readiness.md
├── references/           — prototype-run-bundle-schema.md
└── prototypes/           — BP closure notes (build-point-bp1-* / build-point-bp2-*)
```

## What is NOT in here

The **research vault** at `~/projects/cortex-command-repos-all/cortext_command_vault`
keeps content that informs but does not gate implementation:

- `comparables/` — Cortex Command (CCCP), Stationeers, Noita, OpenSoldat, OpenLieroX, Powder Toy, Barotrauma, Oxygen Not Included audits.
- `research-log/` — dated research-pass notes.
- `references/usage-ledger.md` — license tracking + reuse provenance.
- `references/equipment.schema.json` + `equipment-overlay-seed*.json` — equipment data seeds.
- `narrative-seeds/`, `strategy/`, `systems/`, `templates/`, `glossary.md`.
- `prototypes/native-*.md` — per-milestone evidence narratives (vs. this repo's `docs/implementation-log/` which captures what changed in this repo at that milestone).
- `prototypes/index.md` — vault prototype index.
- Top-level `VAULT_PLAN.md`, `DIRECTORY.md`, `GAME_DESCRIPTION_FOR_FRIEND.md`, vault-root `AGENTS.md`.
- `comparables_repos/` (the actual cloned source trees).

The split is by purpose: spine = "contracts that gate implementation; must
be reviewed with code." vault = "long-form research that informs but
doesn't gate."

## How spine + vault interact

When a vault research note (e.g., a comparable-game audit) discovers
something that should change implementation behavior, the change lands
in `docs/plan/` (spine). The vault keeps the research evidence; the spine
keeps the implementation contract.

Cross-references inside `docs/plan/` files may still point at vault paths
when they reference vault content (research-log, comparables, etc.). Those
references are intentional and should not be rewritten to point inside
this repo because the content actually lives in the vault.

## Migration history

The spine moved from the vault to this repo on 2026-05-09 via
`git filter-repo --path cortext_command_vault/spec/ --path
cortext_command_vault/decisions/ --path
cortext_command_vault/dashboards/decision-tracker.md --path
cortext_command_vault/dashboards/research-readiness.md --path
cortext_command_vault/references/prototype-run-bundle-schema.md --path
cortext_command_vault/prototypes/build-point-bp[12]-*.md
--path-rename cortext_command_vault/:docs/plan/`. Full vault history
(every commit before the migration) is preserved — `git log` on any
spine file shows its full evolution.
