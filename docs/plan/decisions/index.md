← [[index|vault home]] · [[dashboards/navigation-map|navigation map]] · [[spec/index|spec section]] · [root plan](../../VAULT_PLAN.md)

# Decision Records

> [!info] Purpose
> Use this section when choosing how the future game should work. Research notes collect knowledge; decision records compare options, pros/cons, evidence, risks, and revisit triggers.

## How This Section Should Be Used

| Moment | Use This Section For |
|---|---|
| While writing the spec | Convert research into explicit product and technical decisions. |
| While building prototypes | Pick the smallest implementation that tests the real uncertainty. |
| While implementing features | Check why a direction was chosen and what tradeoffs were accepted. |
| When a feature feels wrong | Reopen the decision with new evidence instead of arguing from memory. |
| When adding ideas | Capture alternatives without forcing them into the active spec. |

## Decision Workflow

1. Start from a research note, source file, comparable game, or prototype result.
2. Write the decision context in plain terms.
3. List real options, including the boring conservative option.
4. Compare pros, cons, risks, player value, implementation cost, AI burden, UX burden, modding impact, and networking/replay impact.
5. Pick a current recommendation, mark the decision as open, or label it as a private prototype direction.
6. Link the decision from `spec/` as exploratory whenever useful; mark it settled only when evidence is strong enough.
7. Add a revisit trigger so the team knows when to reopen it.

[[spec/authoritative-game-spec-v0]] is the current canonical synthesis of these decisions. Treat it as the implementation-facing plan, while keeping every DR open until its listed evidence closes.

## Feature Evaluation Matrix

Use these lenses for every major feature.

| Lens | Question |
|---|---|
| Player value | Does this create better stories, clearer tactics, or stronger mastery? |
| Readability | Can the player tell what happened and why? |
| AI burden | Does it make friendly/enemy bots harder to trust? |
| Physics/destruction burden | Does it interact badly with terrain, debris, wounds, or delivery craft? |
| UX burden | Does it require new overlays, panels, tutorials, or warnings? |
| Performance risk | Does it add per-pixel, per-actor, pathfinding, or replay cost? |
| Modding impact | Can creators author, validate, and debug it? |
| Networking/replay impact | Can it be captured as events, replayed, or synchronized later? |
| Content cost | How much art, scripting, balance, testing, and mission work does it create? |
| Retention upside | Does it help players come back without grind or pay randomness? |
| Ethics/fairness | Could it undermine sandbox trust, modding, or progression fairness? |

## Decision Records

| ID | Title | Priority | Status | Lean | Why It Matters |
|---|---|---|---|---|---|
| [[decisions/dr-001-engine-strategy|DR-001]] | Engine strategy | <span class="cc-flag cc-red">P0</span> | OPEN | Sequence: build/run audit → reuse-ledger skim → 2-week prototype | Determines toolchain, data compatibility, schedule risk. |
| [[decisions/dr-002-replay-event-architecture|DR-002]] | Replay/event architecture | <span class="cc-flag cc-red">P0</span> | OPEN | Hybrid event log + snapshots | Needed for AI debugging, player learning, support, online architecture. |
| [[decisions/dr-003-body-damage-readability|DR-003]] | Body damage readability | <span class="cc-flag cc-red">P0</span> | OPEN | Hybrid: silhouette default + advanced HUD opt-in | Wounds/gibs are core flavor; readability is the difference between charm and pain. |
| [[decisions/dr-004-first-playable-slice|DR-004]] | First playable slice | <span class="cc-flag cc-red">P0</span> | OPEN | Sequenced single actor → squad → bunker breach | Anchors prototype order, hiring, milestone narrative. |
| [[decisions/dr-005-multiplayer-posture|DR-005]] | Multiplayer posture | <span class="cc-flag cc-red">P0</span> | OPEN | Solo-first + co-op-ready arch; prototype networking freely, no launch PvP promise yet | Protects sim freedom; respects solo-first promise. |
| [[decisions/dr-006-modding-data-model|DR-006]] | Modding data model | <span class="cc-flag cc-orange">P1</span> | OPEN | Schema-first + Lua escape hatches + workbench | Community longevity depends on authoring, validation, migration. |
| [[decisions/dr-007-terrain-material-model|DR-007]] | Terrain/material model | <span class="cc-flag cc-red">P0</span> | OPEN | Prototype solids + curated hazards first; keep Noita-grade materials as moonshot research | Controls performance, AI pathfinding, UX overlays, networking, scope. |
| [[decisions/dr-008-ai-architecture|DR-008]] | AI architecture | <span class="cc-flag cc-red">P0</span> | OPEN | Hybrid jobs + utility scoring + scripted hooks | Solo-first promise depends on commandable, recoverable, explainable bots. |
| [[decisions/dr-009-command-ux-style|DR-009]] | Command UX style | <span class="cc-flag cc-orange">P1</span> | OPEN | Direct control + slowdown overlay + optional tactical map | Direct + command must coexist without slowing the game. |
| [[decisions/dr-010-license-reuse-matrix|DR-010]] | License/reuse matrix | <span class="cc-flag cc-orange">P1</span> | OPEN | Personal/private reuse allowed; ledger tracks future release cleanup | Keeps public-release options open without blocking game creation. |
| [[decisions/dr-011-progression-retention-loop|DR-011]] | Progression/retention loop | <span class="cc-flag cc-orange">P1</span> | OPEN | Intrinsic-first hybrid: mastery + autonomy + veterans + salvage + replays + creator challenges | Defines why players return without letting gacha/live-service pressure lead the design. |
| [[decisions/dr-012-accessibility-comfort-readability|DR-012]] | Accessibility, comfort, and readability floor | <span class="cc-flag cc-orange">P1</span> | OPEN | Slice A accessibility/comfort floor, not late compliance | Keeps dense combat, loadout, replay, hub, and workbench UI readable, navigable, captioned, and comfortable while prototypes are cheap to change. |
| [[decisions/dr-013-backend-service-scope|DR-013]] | Backend service scope | <span class="cc-flag cc-orange">P1</span> | OPEN | Local-first service spine + optional adapters | Keeps backend work focused on play, packages, replay/debug, diagnostics, hub UX, and future co-op without prematurely committing accounts, matchmaking, economy, or public PvP. |
| [[decisions/dr-014-tone-player-promise|DR-014]] | Tone and player promise | <span class="cc-flag cc-red">P0</span> | CLOSED-DIRECTION | Tactical pulp sci-fi disaster sandbox | Guides art, audio, writing, mechanics, UX, mechs, armor, origins, and staged equipment/body damage. |
| [[decisions/dr-015-player-identity-control-posture|DR-015]] | Player identity and control posture | <span class="cc-flag cc-red">P0</span> | CLOSED-DIRECTION | Command-core operator; strategy-first, pilot-optional | Defines who the player is, how AI owns bodies by default, and why direct possession is optional rather than mandatory. |
| [[decisions/dr-016-setting-and-world-frame|DR-016]] | Setting and world frame | <span class="cc-flag cc-red">P0</span> | CLOSED-DIRECTION | Frontier disaster-contract sci-fi (merc/rescue/salvage outfit; command-core lore-flexible) | Locks world tone + faction grammar; specific factions/places/antagonists open. See [[spec/setting-and-world-frame]]. |
| [[decisions/dr-017-mission-generation-strategy|DR-017]] | Mission generation strategy | <span class="cc-flag cc-red">P0</span> | CLOSED-DIRECTION | Manifest-first hybrid: anchor missions + procedural contracts + first-class player-authored | Single typed manifest serves engine, editor, AI, replay, mod tools, and players. |
| [[decisions/dr-018-death-meaning-and-consequence-ladder|DR-018]] | Death meaning and consequence ladder | <span class="cc-flag cc-red">P0</span> | CLOSED-DIRECTION | Tiered consequence ladder, rescue-first default, scenario-configurable, per-origin meanings | Locks campaign feel; supports veteran/salvage/replay/AI rescue contracts. |

## Still-Open Topics (Not Yet A Record)

| Decision | Why It Matters | Source Notes |
|---|---|---|
| Monetization ethics | Long-term fairness with modding ecosystem; DR-011 captures launch-boundary posture but a release-facing economy DR is still needed before any commitment. | [[decisions/dr-011-progression-retention-loop]], [[strategy/best-cortex-like-game-principles]] |
| Localization plan | Strings, fonts, language packs, mod localization. | None yet. |
| Audio/music identity | Diegetic feedback + procedural soundtrack budget. DR-014 now sets tone, but the actual music/SFX identity still needs a dedicated pass. | [[decisions/dr-014-tone-player-promise]], [[spec/accessibility-comfort-slice-a]] |

## Decision Record Format

Use [[templates/decision-record-template]] for new decision notes.

Minimum required sections:

- Context.
- Options.
- Pros and cons.
- Evidence.
- Recommendation.
- Risks.
- Prototype or validation plan.
- Revisit trigger.

## Source Trail

- [[dashboards/research-readiness]]
- [[dashboards/system-heatmap]]
- [[spec/index]]
- [VAULT_PLAN.md](../../VAULT_PLAN.md)
