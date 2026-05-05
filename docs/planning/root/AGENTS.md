# AGENTS.md

Project-wide instructions for AI agents working in `/Users/erol/projects/cortex-command-repos-all`.

Loaders that respect AGENTS.md (Codex CLI/IDE, Factory Droid, Cursor, Claude Code) read this file as root-level project guidance. Keep it compact: route agents into the vault instead of duplicating it here. Nested `AGENTS.md` files in subdirectories override this file for their subtree (closest file wins). Add an `AGENTS.override.md` only if you want to layer rules without renaming the parent.

## Project Purpose

This directory is a Cortex Command research workspace. Its main job is to preserve knowledge for:

- Planning a future Cortex Command-like game.
- Writing a game spec inside the vault.
- Comparing feature options, pros/cons, risks, and implementation paths while building.
- Auditing Cortex-family repositories and comparable games without losing source trails.

The vault is the durable knowledge base. The game spec is one section of the vault, not a replacement for the vault.

The top priority is creating the best possible game, UX, UI, frontend, backend, systems, features, and player experience. License/reuse tracking exists to preserve future options; it must not block private prototyping, design exploration, or momentum unless the user explicitly asks for release-ready compliance.

## Root Directory Map

| Path | Role | Notes |
|---|---|---|
| `AGENTS.md` | Agent instructions | Root guidance for future AI agents. |
| `DIRECTORY.md` | Repo inventory | Quick root-level summary of cloned repos. |
| `VAULT_PLAN.md` | Operating plan | Execution board, readiness gates, research priorities, spec gates. |
| `cortext_command_vault/` | Obsidian research vault | Main knowledge base. Preserve this exact misspelled directory name unless the user explicitly asks to rename it. |
| `Cortex-Command-Community-Project/` | Active CCCP unified repo | Primary current Cortex source + data reference. |
| `Cortex-Command-Community-Project-Source/` | Archived CCCP source repo | History-only source archaeology. |
| `Cortex-Command-Community-Project-Data/` | Archived CCCP data repo | History-only data archaeology. |
| `Cortex-Command-Community-Project-VSCode-Extension/` | Modding extension | Creator tooling and schema/grammar reference. |
| `Cortex-Command-Legacy-Mod-Converter/` | Legacy converter | Migration and content compatibility reference. |
| `cortex-command-community.github.io/` | Community website source | Website/community funnel reference. |
| `Cortex-Command-Community-Continuation-Engine/` | C4 alternative fork | Comparison fork, especially old networking/dependency paths. |
| `comparables_repos/` | Comparable-game repo workspace | Created and ready for cloning OpenSoldat/OpenLiero/OpenLieroX/Powder Toy etc. See `comparables_repos/README.md`. |
| `prototype_workspaces/` | Prototype code/workspaces | Keep new experimental builds here or in another explicit prototype/fork workspace, not accidentally inside canonical reference repos. |
| `prototype_runs/` | Prototype run evidence | Checked run bundles with manifest/events/summary/notes; link important results back into `cortext_command_vault/prototypes/`. |
| `research_tools/` | Python tools | CLI utilities used to generate references (content-loader graph, equipment corpus, role records, AI scenarios, package diagnostics, prototype run checker). Run from repo root. |

Long-running agent passes that need their own checkpoint/audit/recovery files should write them under `cortext_command_vault/research-log/<date>-<purpose>-snapshot.md`, not at the repo root. This keeps the root tidy and puts agent-tracker artifacts in the same audit trail as normal research-log entries.

Treat cloned upstream repos as canonical research copies by default. You may copy, fork, branch, or prototype from them when it helps the game; do not accidentally mutate the reference copies unless the user explicitly asks for edits there.

## Vault Structure

Start at `cortext_command_vault/index.md`.

| Vault Folder | Purpose | Start Here |
|---|---|---|
| `dashboards/` | Navigation, system risk, readiness gates | `dashboards/navigation-map.md`, `dashboards/research-readiness.md`, `dashboards/system-heatmap.md` |
| `repos/` | Notes on each cloned Cortex-related repo | `repos/index.md` |
| `game/` | What Cortex Command is and how the player loop works | `game/what-is-cortex-command.md`, `game/player-loop-and-ux.md` |
| `engine/` | Code-level mechanics and lifecycle archaeology | `engine/architecture.md`, lifecycle notes |
| `systems/` | Design translation for mechanics, AI, UX, networking, retention | `systems/index.md`, `systems/mechanics-matrix.md` |
| `comparables/` | Similar games and external references | `comparables/index.md`, `comparables/comparison-matrix.md` |
| `comparisons/` | Direct repo/project comparisons | `comparisons/cccp-vs-c4.md` |
| `decisions/` | Pros/cons and build/spec decisions | `decisions/index.md`, `decisions/dr-001-engine-strategy.md`..`dr-013-backend-service-scope.md` |
| `spec/` | Future game spec section | `spec/index.md` |
| `design/` | Design principles and fork opportunities | `design/design-decisions.md`, `design/opportunities-for-our-fork.md` |
| `strategy/` | Roadmaps, principles, review passes | `strategy/research-to-spec-roadmap.md`, `strategy/best-cortex-like-game-principles.md` |
| `references/` | Sources, people, authors, citations, usage ledger | `references/sources.md`, `references/people-and-authors.md`, `references/usage-ledger.md` |
| `prototypes/` | Prototype evidence notes | `prototypes/index.md`, current run notes |
| `research-log/` | Chronological work log | Latest dated pass |
| `templates/` | Reusable note formats | `templates/note-template.md`, `templates/decision-record-template.md`, `comparables/audit-template.md` |

## Source Of Truth Order

Use this order when answering questions or making recommendations:

1. Local code evidence from the cloned repos.
2. Existing vault notes with source trails.
3. Decision records in `cortext_command_vault/decisions/`.
4. External sources listed in `cortext_command_vault/references/sources.md`.
5. New web research, when the fact may have changed or the vault lacks evidence.
6. Explicit assumptions marked as unproven.

Never present a design preference as a confirmed fact. Mark speculation, assumptions, and inspiration clearly.

## Creative Exploration And Evidence Standards

Exploration is allowed before evidence exists. Brainstorming notes, wishlists, speculative mechanics, moonshot features, copied experiments, and private prototypes may be added at any time if they are clearly labeled as `idea`, `hypothesis`, `prototype`, or `unproven`.

Do not promote an idea as a settled spec commitment or final decision unless it has at least one of:

- Local Cortex code evidence.
- Direct comparable repo code evidence.
- Public developer/source citation.
- Prototype result.
- Explicit design assumption marked as unproven.

Every major claim should link to a vault note, code path, source URL, decision record, or prototype artifact.

## Planning AI Agent

Use this role when asked to plan the game, write or organize the game spec, compare directions, or decide what to research next.

Workflow:

1. Read `VAULT_PLAN.md`.
2. Open `cortext_command_vault/index.md`.
3. Check `dashboards/research-readiness.md` before expanding `spec/`.
4. Check `dashboards/system-heatmap.md` for high-risk systems.
5. Use `decisions/index.md` before turning a feature into a spec claim.
6. Use `templates/decision-record-template.md` for major choices.
7. Write exploratory spec stubs whenever useful; treat readiness gates as blockers only for settled/authoritative spec commitments.
8. Link spec claims back to evidence instead of copying whole research notes.

Planning agent priorities:

- Preserve the vault as a knowledge base.
- Keep options and rejected alternatives visible.
- Separate player value from technical fascination.
- Treat AI trust, simulation readability, replay/event architecture, modding, and networking as high-risk areas.
- Defer shipping commitments around gacha and monetization until core fairness, modding, and retention loops are proven. Research and prototype retention mechanics freely when useful.

## Implementing AI Agent

Use this role when asked to build prototypes, modify code, run tests, or turn decisions into implementation.

Workflow:

1. Read the relevant `spec/` page if it exists.
2. Read linked `decisions/` records for the feature.
3. Read `cortext_command_vault/spec/feature-completion-checklist.md` before implementation and update it during final audit.
4. Read supporting `systems/`, `engine/`, and `comparables/` notes.
5. Inspect local source code before assuming behavior.
6. Keep cloned upstream repos as stable references unless the user explicitly asks for edits there; create separate prototype/fork workspaces when experimenting from their code.
7. When reusing code, data, sprites, sounds, assets, mechanics, or structure from research material, document the source and release-readiness status. Do not let reuse tracking block private prototyping when the user wants to move fast.
8. After implementation discoveries, update or propose updates to the vault with evidence, code paths, test results, and new risks.
9. When closing a feature, task card, or milestone, update every affected checklist row with evidence and AI self-ratings. Leave human rating columns blank unless the user provides them.

Implementation agent priorities:

- Build the smallest prototype that tests the real uncertainty.
- Favor readable player feedback over hidden simulation complexity.
- Add replay/event/debug hooks early for AI, damage, terrain, and delivery failures.
- Do not lock live PvP as a launch promise until terrain/entity authority and bandwidth are proven. Research and prototype PvP/co-op/networking freely if it could improve the game.
- If implementation contradicts the vault, record the contradiction and update the relevant note or decision record.

## Vault Maintainer AI Agent

Use this role when asked to improve the vault, add research, organize notes, fix navigation, or keep the knowledge base coherent.

Workflow:

1. Start at `cortext_command_vault/index.md`.
2. Check `dashboards/navigation-map.md` to place new notes.
3. Use `templates/note-template.md` for general notes.
4. Use `templates/decision-record-template.md` for option comparisons.
5. Add or update backlinks at the top of notes.
6. Update indexes when adding a new folder or important note.
7. Update `dashboards/research-readiness.md` when a gate changes.
8. Update `research-log/` for substantial research passes.
9. Validate Obsidian wikilinks after structural changes.

Vault maintainer priorities:

- Keep facts, interpretations, and recommendations separate.
- Keep source trails close to claims.
- Prefer tables, matrices, route maps, and concise summaries for navigation.
- Preserve useful uncertainty instead of smoothing it away.
- Avoid dumping unsorted research into long notes without indexes or summaries.

## Obsidian Style Rules

- Use wiki links like `[[systems/ai-trust-test-suite]]` for vault notes.
- Keep a backlink/navigation row at the top of important notes.
- Use tables for comparisons, decisions, readiness, pros/cons, and risk.
- Use Mermaid diagrams where flows or lifecycles matter.
- Use callouts for status, warnings, danger, and summaries.
- Keep `references/sources.md` and `research-log/` current when adding web research.
- Do not rename `cortext_command_vault/` without explicit user approval.

## Web And Current Research

Use web research when:

- The fact may have changed.
- A source, repo, paper, article, or official page is referenced and not already captured.
- Recommendations depend on current tools, licensing, maintainers, releases, or comparable game repos.

When using web research, add durable source links to the appropriate vault note or `references/sources.md`.

## Reuse, Licensing, And Release Boundaries

- This is currently a personal/private project. Reuse and license notes are planning and future-release documentation, not blockers on creation.
- The user may override reuse caution at any point for private prototyping or exploration.
- Copying or adapting code, data, sprites, sounds, assets, mechanics, UI patterns, backend patterns, or feature ideas is allowed for private work when it helps build the best game.
- When anything external is used or copied, log it in `cortext_command_vault/references/usage-ledger.md` with what was used, where it came from, the apparent license if known, and whether it is `prototype-only`, `needs license review`, `needs permission`, or `release-ready`.
- If the project may become public later, use the usage ledger to obtain the right license agreements, permissions, replacements, or rewrites before release. See `cortext_command_vault/decisions/dr-010-license-reuse-matrix.md`.
- Do not let license/reuse caution reduce game quality, UX quality, feature ambition, or research depth during private development.
- Keep mod compatibility and data compatibility separate from code reuse.
- Preserve user edits and do not revert unrelated changes.

## Useful Validation Commands

Run these from the root when maintaining the vault:

```sh
find cortext_command_vault -maxdepth 2 -type f -name '*.md' | sort
rg -n "MISSING|TODO|FIXME|DRAFT|PARTIAL" VAULT_PLAN.md cortext_command_vault
python3 - <<'PY'
from pathlib import Path
import re
files = [Path('VAULT_PLAN.md'), *Path('cortext_command_vault').rglob('*.md')]
missing = []
for path in files:
    text = path.read_text(errors='ignore')
    for target in re.findall(r'\[\[([^\]|#]+)', text):
        target = target.strip()
        candidates = [
            Path('cortext_command_vault') / (target + '.md'),
            Path('cortext_command_vault') / target / 'index.md',
        ]
        if not any(candidate.exists() for candidate in candidates):
            missing.append((str(path), target))
print('missing_wikilinks', len(missing))
for path, target in missing:
    print(path, '->', target)
PY
```

## Do Not

- Do not treat the spec as the whole vault.
- Do not erase rejected options from decisions.
- Do not add unsourced claims as settled facts.
- Do not edit upstream cloned repos unless explicitly asked.
- Do not leave copied or adapted external material undocumented.
- Do not treat release-readiness concerns as blockers for private prototyping unless the user explicitly says the work must be public-release safe now.
- Do not interpret evidence gates, readiness gates, risk labels, or defer labels as bans on research, brainstorming, ambitious features, or private prototypes.
- Do not let the vault become a pile of pages without dashboards, indexes, or source trails.
