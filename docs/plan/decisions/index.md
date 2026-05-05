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
| [[decisions/dr-005-multiplayer-posture|DR-005]] | Multiplayer architecture and launch scope | <span class="cc-flag cc-red">P0</span> | CLOSED-DIRECTION | Server-authoritative; one `cx-server` binary; full ladder at launch (solo + LAN + online co-op + community-hostable public PvP arenas + persistent MMO shards). Anyone can host. | Locks the multiplayer launch promise. See [[spec/server-app-architecture]] and [[spec/persistent-mmo-architecture]]. |
| [[decisions/dr-006-modding-data-model|DR-006]] | Modding data model | <span class="cc-flag cc-orange">P1</span> | OPEN | Schema-first + Lua escape hatches + workbench | Community longevity depends on authoring, validation, migration. |
| [[decisions/dr-007-terrain-material-model|DR-007]] | Terrain/material model | <span class="cc-flag cc-red">P0</span> | OPEN | Prototype solids + curated hazards first; keep Noita-grade materials as moonshot research | Controls performance, AI pathfinding, UX overlays, networking, scope. |
| [[decisions/dr-008-ai-architecture|DR-008]] | AI architecture | <span class="cc-flag cc-red">P0</span> | OPEN | Hybrid jobs + utility scoring + scripted hooks | Solo-first promise depends on commandable, recoverable, explainable bots. |
| [[decisions/dr-009-command-ux-style|DR-009]] | Command UX style | <span class="cc-flag cc-orange">P1</span> | OPEN | Direct control + slowdown overlay + optional tactical map | Direct + command must coexist without slowing the game. |
| [[decisions/dr-010-license-reuse-matrix|DR-010]] | License/reuse matrix | <span class="cc-flag cc-orange">P1</span> | OPEN | Personal/private reuse allowed; ledger tracks future release cleanup | Keeps public-release options open without blocking game creation. |
| [[decisions/dr-011-progression-retention-loop|DR-011]] | Progression/retention loop | <span class="cc-flag cc-orange">P1</span> | OPEN | Intrinsic-first hybrid: mastery + autonomy + veterans + salvage + replays + creator challenges | Defines why players return without letting gacha/live-service pressure lead the design. |
| [[decisions/dr-012-accessibility-comfort-readability|DR-012]] | Accessibility, comfort, and readability floor | <span class="cc-flag cc-orange">P1</span> | OPEN | Slice A accessibility/comfort floor, not late compliance | Keeps dense combat, loadout, replay, hub, and workbench UI readable, navigable, captioned, and comfortable while prototypes are cheap to change. |
| [[decisions/dr-013-backend-service-scope|DR-013]] | Backend services architecture | <span class="cc-flag cc-red">P0</span> | CLOSED-DIRECTION | Full backend service spine + dedicated server app + community hosting + optional first-party services. Local-first stays the default for solo/private play; `lobby_directory`, account adapter, persistence, anti-cheat foundation, and observability ship at launch. Steam/EOS/PlayFab/Unity Multiplay are optional adapters. No subscription, no marketplace, no live-service economy. | Locks the backend launch surface; supports DR-005, DR-034, DR-035. |
| [[decisions/dr-014-tone-player-promise|DR-014]] | Tone and player promise | <span class="cc-flag cc-red">P0</span> | CLOSED-DIRECTION | Tactical pulp sci-fi disaster sandbox | Guides art, audio, writing, mechanics, UX, mechs, armor, origins, and staged equipment/body damage. |
| [[decisions/dr-015-player-identity-control-posture|DR-015]] | Player identity and control posture | <span class="cc-flag cc-red">P0</span> | CLOSED-DIRECTION | Command-core operator; strategy-first, pilot-optional | Defines who the player is, how AI owns bodies by default, and why direct possession is optional rather than mandatory. |
| [[decisions/dr-016-setting-and-world-frame|DR-016]] | Setting and world frame | <span class="cc-flag cc-red">P0</span> | CLOSED-DIRECTION | Frontier disaster-contract sci-fi (merc/rescue/salvage outfit; command-core lore-flexible) | Locks world tone + faction grammar; specific factions/places/antagonists open. See [[spec/setting-and-world-frame]]. |
| [[decisions/dr-017-mission-generation-strategy|DR-017]] | Mission generation strategy | <span class="cc-flag cc-red">P0</span> | CLOSED-DIRECTION | Manifest-first hybrid: anchor missions + procedural contracts + first-class player-authored | Single typed manifest serves engine, editor, AI, replay, mod tools, and players. |
| [[decisions/dr-018-death-meaning-and-consequence-ladder|DR-018]] | Death meaning and consequence ladder | <span class="cc-flag cc-red">P0</span> | CLOSED-DIRECTION | Tiered consequence ladder, rescue-first default, scenario-configurable, per-origin meanings | Locks campaign feel; supports veteran/salvage/replay/AI rescue contracts. |
| [[decisions/dr-019-visual-direction|DR-019]] | Visual direction | <span class="cc-flag cc-red">P0</span> | CLOSED-DIRECTION | Pixel-sim battlefield + comic-noir UI presentation (Cortex pixel + Mark of the Ninja silhouette discipline) | Locks two-layer art pipeline; see [[spec/visual-direction]]. |
| [[decisions/dr-020-audio-identity|DR-020]] | Audio identity | <span class="cc-flag cc-red">P0</span> | CLOSED-DIRECTION | Diegetic industrial synth-dread + mandatory captions; audio is tactical UI | Locks SFX-first mix; situational synth tension; caption coverage required. See [[spec/audio-identity]]. |
| [[decisions/dr-021-mech-scale-and-archetypes|DR-021]] | Mech scale & archetypes | <span class="cc-flag cc-red">P0</span> | CLOSED-DIRECTION | Full ladder (powered armor → light → medium → heavy mech) + 8 archetypes + module system; constrained v1 roster | Locks chassis ambition; foot infantry still core; ~6-9 mechs at v1. |
| [[decisions/dr-022-ai-humanlike-bar|DR-022]] | AI humanlike-ness success bar | <span class="cc-flag cc-red">P0</span> | CLOSED-DIRECTION | Persistent teammate-and-rival AI: 8 criteria all-must-hold (intent, perception, doctrine, mistakes, recovery, strategic adaptation, replay proof, fairness) | Defines what "most humanlike AI in genre" means in testable terms. |
| [[decisions/dr-023-tutorial-and-onboarding-strategy|DR-023]] | Tutorial & onboarding | <span class="cc-flag cc-red">P0</span> | CLOSED-DIRECTION | Hybrid+: cinematic onboarding mission + 8 permanent labs + fading tooltips + "show me why" handoff | Locks how new players learn; labs serve veterans too. |
| [[decisions/dr-024-native-engine-stack|DR-024]] | Native engine stack | <span class="cc-flag cc-red">P0</span> | CLOSED-DIRECTION | Rust + Bevy/wgpu hybrid + custom core crates (modular cargo workspace) | Closes the engine-strategy "implementation specifics" subdecision left open by DR-001. |
| [[decisions/dr-025-target-platforms|DR-025]] | Target platforms | <span class="cc-flag cc-red">P0</span> | CLOSED-DIRECTION | Win + Linux + macOS desktop-first; Steam Deck floor; headless Linux server later; web only for labs/tools/demos; no mobile | Locks platform reach + perf-floor surface. See [[decisions/dr-028-visual-fidelity-targets]] for the per-target perf ladder. |
| [[decisions/dr-026-team-and-repo-model|DR-026]] | Team and repo model | <span class="cc-flag cc-red">P0</span> | CLOSED-DIRECTION | AI-augmented solo / small-core; modular cargo workspace where each crate is a feature/agent boundary | Defines how work is divided across human + AI ownership and how merge collisions are prevented. |
| [[decisions/dr-027-combat-base-scope|DR-027]] | Combat-base scope | <span class="cc-flag cc-red">P0</span> | CLOSED-DIRECTION | Deep combat-base (command core + power + shields + turrets + sensors + doors + repair pads + hangar + storage + traps + breachable structure). NOT full colony sim. | Bounds the base layer; supports DR-015 strategy-first identity without inheriting Rimworld/DF scope. |
| [[decisions/dr-028-visual-fidelity-targets|DR-028]] | Visual fidelity targets | <span class="cc-flag cc-red">P0</span> | CLOSED-DIRECTION | Ceiling 4K/120 strong desktop; default 1080p/60 mid-range; floor Deck 800p/60. 60 Hz fixed sim island, render decoupled. | Locks per-frame budgets and the perf-ladder content target. See [[decisions/dr-019-visual-direction]] for the visual style. |
| [[decisions/dr-029-save-game-model|DR-029]] | Save game model | <span class="cc-flag cc-red">P0</span> | CLOSED-DIRECTION | Versioned local-first `.cxsave` with replay archive linkage, multi-slot, autosave, ironman, scenario policies, migration handlers. Cloud post-launch. | Defines campaign/save architecture; ties replay (DR-002) and consequence ladder (DR-018) into a durable storage contract. |
| [[decisions/dr-030-scenario-editor-commitment|DR-030]] | Scenario editor first-class commitment | <span class="cc-flag cc-red">P0</span> | CLOSED-DIRECTION | First-class in-engine editor at launch using the same typed manifest as engine + director + procedural generator + player-authored content | Locks editor in the launch SKU and reuses the DR-017 manifest. |
| [[decisions/dr-031-content-economy-and-monetization-posture|DR-031]] | Content economy & monetization posture | <span class="cc-flag cc-red">P0</span> | CLOSED-DIRECTION | Premium one-time purchase + free modding. Expansions/DLC/cosmetics post-launch. No pay-to-win, gacha, gameplay-gating battle pass, or marketplace cut on user mods. | Locks the launch SKU and rules out predatory patterns; supports DR-006 modding and DR-011 retention principles. |
| [[decisions/dr-032-hybrid-llm-ai-direction|DR-032]] | Hybrid LLM AI direction | <span class="cc-flag cc-red">P0</span> | CLOSED-DIRECTION | Hybrid AI: classic local game AI owns the body at frame speed; async LLM "mind" workers run in the background and submit validated `AiMindProposal` schemas (doctrine, memory, personality, debriefs, commander adaptation). Local AI never blocks on an LLM. No API key required to ship, test, or play. | Captures the T-LLM side track + M6.5 milestone; bounds LLM scope so DR-022 humanlike bar can be tested without any LLM. See [[spec/hybrid-llm-ai-plan]]. |
| [[decisions/dr-033-full-collision-physics-direction|DR-033]] | Full collision physics direction | <span class="cc-flag cc-red">P0</span> | CLOSED-DIRECTION | Full physical collision is a core feel pillar: weapons, limbs, bodies, armor, mechs, terrain, objects, shields, debris, base parts, and projectiles collide by default unless explicit tested filters say otherwise. | Captures T-PHYS + M5.5 Full Collision Gauntlet; adds projectile-projectile, CCD tiers, impulse-to-damage, collision events, and perf/replay gates. See [[spec/full-collision-physics-plan]]. |
| [[decisions/dr-034-dedicated-server-application|DR-034]] | Dedicated server application & community hosting | <span class="cc-flag cc-red">P0</span> | CLOSED-DIRECTION | Single `cx-server` binary, multi-mode (`coop_room`/`pvp_arena`/`lan_room`/`mmo_shard`/`lobby_directory`); same Rust workspace; same sim path as the client; Linux + Windows; reference Docker image; documented hosting guide; no proprietary cloud lock-in. | Captures the dedicated-server launch artifact + T-SERVER side track + M9..M12 SERVER-001..SERVER-016 acceptance suite. See [[spec/server-app-architecture]]. |
| [[decisions/dr-035-persistent-mmo-architecture|DR-035]] | Persistent MMO architecture | <span class="cc-flag cc-red">P0</span> | CLOSED-DIRECTION | MMO shard is a launch-supported mode of `cx-server`; bounded shard-with-portal model (NOT seamless world); 50-200 concurrent target; community-hostable; persistent terrain/bases/veterans/factions/commander memory; account required for public shards, NOT for private LAN/co-op. **Not subscription-funded.** | Captures M12 MMO-001..MMO-012 acceptance suite. See [[spec/persistent-mmo-architecture]]. |

## Still-Open Topics (Not Yet A Record)

| Decision | Why It Matters | Source Notes |
|---|---|---|
| Localization plan | Strings, fonts, language packs, mod localization. | None yet. |
| Networking transport choice | Lightyear vs renet vs quinn for the `cx-net` crate. Decision deferred to M9/M10 prototyping. | [[decisions/dr-005-multiplayer-posture]], [[decisions/dr-024-native-engine-stack]] |
| Modding script host | mlua (Lua) vs Rhai. Decision deferred to M5 implementation work. | [[decisions/dr-006-modding-data-model]], [[decisions/dr-024-native-engine-stack]] |
| Cloud save backend | Provider + privacy + sync semantics. Post-launch. | [[decisions/dr-013-backend-service-scope]], [[decisions/dr-029-save-game-model]] |

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
