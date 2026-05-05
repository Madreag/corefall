---
type: decision
id: DR-001
status: open
priority: P0
revisit_trigger: "When a build/run audit, license review, and a small actor-feel prototype are all complete."
---

← [[decisions/index|decision records]] · [[spec/index|spec section]] · [[dashboards/research-readiness|readiness]] · [[engine/cccp-build-run-audit|CCCP build/run audit]] · [[design/opportunities-for-our-fork|fork opportunities]]

# DR-001: Engine Strategy

> [!success] Status: DIRECTION CLOSED (project owner committed 2026-05-04). Implementation specifics still open.
> **Direction:** Greenfield native core + CCCP as reference lab. Build a new engine. Use CCCP for mechanics archaeology, feel comparison, content taxonomy, equipment roles, AI/pathfinding lessons, terrain/destruction behavior, mistakes-to-avoid, and possibly a one-way import/converter tool later. Do **not** let CCCP define the final data format, physics model, UI, AI architecture, mod API, or backend/replay/event model.
>
> **Constraint inherited from project owner:** "Best engine, best physics, best performance, best network latency, best UX, best UI, MOST HUMANLIKE AI IN THE GAME, all enjoyable." These are not soft hopes; they are the bar. The greenfield path is justified because no fork inherits all of them at the level required.
>
> Still open: language/runtime, ECS or OOP, renderer (WebGPU/wgpu/Vulkan/native), determinism boundary, exact data schema, repo structure, build/CI plan. These are implementation specifics that the actor-feel + recorder + chassis Slice-A prototypes inform.

## Context

We need to decide how the future Cortex Command-like game inherits, replaces, or sidesteps the existing engine families:

- CCCP unified repo (current active community continuation, AGPL-3.0).
- C4 alternative continuation engine (older dependency stack, networking emphasis).
- Greenfield engine in our preferred stack.
- Compatibility-oriented rewrite that keeps `.rte` data but replaces the engine.

This choice cascades across licensing, modding, networking posture, prototype velocity, scope risk, and community trust. It is the single most expensive reversible decision in the project.

## Options

| Option | Summary | Best Case | Worst Case |
|---|---|---|---|
| A. Fork CCCP | Build from active CCCP. | Inherit working engine, mods, content, mature Lua API. | AGPL-3.0 inheritance, opaque legacy code, slow modernization. |
| B. Fork C4 | Build from C4 fork. | Older dependencies easier to swap; multiplayer code visible. | Less active upstream; archived signals; still legacy code burden. |
| C. Greenfield engine | Build our own engine in modern stack. | Free of legacy debt; design replays/networking from day one. | Years before parity; loses moddability; community alienation risk. |
| D. Compatibility rewrite | New engine that loads `.rte` data and supports a Lua subset. | Preserves community content; modern tech. | Schema and AI parity is a long tail; never quite "works for old mod X". |
| E. Hybrid: greenfield core with optional CCCP-compat layer behind a flag | Modern core, opt-in compat. | Best of both with cost discipline. | Engineering complexity of two boundaries. |

## Pros And Cons

| Option | Pros | Cons | Unknowns |
|---|---|---|---|
| A | Working engine, mods, content, fast first prototype. | AGPL-3.0 viral; legacy network/AI/path code; modernization debt. | License compatibility with our distribution model. |
| B | Easier dependency replacement; visible multiplayer/NAT code. | Less momentum; archived; lower mod compatibility. | Whether C4 even builds locally today. |
| C | Clean architecture; replay/event/networking from day one; no legacy. | Loses community, content, modding momentum. | Time to first playable: 6-18 months. |
| D | Preserves community while shedding old code. | INI/Lua compatibility is a long tail; partial parity hurts trust. | Number of community mods that still work after launch. |
| E | Hybrid keeps escape hatch. | Two surfaces to maintain; fragmentation risk. | Whether a feature flag keeps both honest or just rotting. |

## Evaluation

| Lens | A. Fork CCCP | B. Fork C4 | C. Greenfield | D. Compat rewrite | E. Hybrid |
|---|---|---|---|---|---|
| Player value | Works on day one | Works on day one | Best long-term | Good | Best long-term |
| Readability | Inherits friction | Inherits friction | Designable | Designable | Designable |
| AI burden | Inherits hybrid Lua AI | Same | Build from scratch | Re-implement | Build from scratch + compat shim |
| UX burden | Inherits old UI | Same | Designable | Designable | Designable |
| Performance risk | Engine modernization debt | Same | Lowest | Medium | Medium |
| Modding impact | Best (existing mods) | Good | Low (new format) | Best (compat) | Best (compat layer) |
| Networking/replay | Inherits legacy code | Visible legacy | Architect from scratch | Architect from scratch | Architect from scratch |
| Content cost | Lowest | Low | Highest | High | High |
| Retention upside | Familiar | Familiar | Depends on hooks | Familiar | Familiar |
| Ethics/fairness | AGPL-3.0 protects open community | Same | Designable | Designable | Designable |

## Evidence

| Evidence | Source | Confidence |
|---|---|---|
| CCCP is actively maintained and has unified source/data. | [[repos/cccp-active-unified-repo]] | High |
| CCCP is AGPL-3.0. | CCCP repo `LICENSE` | High |
| C4 has visible RakNet/NAT and multiplayer code. | [[repos/c4-continuation-engine]], [[engine/network-terrain-replication-lifecycle]] | High |
| C4 last local commit is 2023-04-07; online metadata indicates archived. | [[repos/c4-continuation-engine]] | Medium |
| CCCP networking code exists but is RakNet-era and bitmap-delta-based. | [[engine/network-terrain-replication-lifecycle]] | High |
| Modding and migration are real product surfaces. | [[repos/legacy-mod-converter]], [[repos/cccp-vscode-extension]] | High |
| CCCP modding ecosystem: mod.io CCCP, GitHub releases. | [[references/sources]] | High |
| Local CCCP native macOS configure/compile now succeeds on this host after installing the README Homebrew stack; bounded menu/tutorial startup launches stay alive until wrapper timeout. | [[engine/cccp-build-run-audit]] | High |
| Full runtime proof remains open: WindowServer sees an on-screen `Cortex Command` window, but screenshots did not capture game pixels and no interactive vanilla mission screenshot/video, input smoke test, or played-session log bundle has been captured yet. | [[engine/cccp-build-run-audit]], [[engine/cccp-runtime-window-capture-troubleshooting]] | High |
| Current README and Meson workflow still point to Meson/Ninja builds; native macOS requires GCC 13, and CI macOS uses an osxcross path rather than proving native Apple Silicon Homebrew builds. | [[engine/cccp-build-run-audit]], [[references/sources]] | High |

## Current Recommendation

**Closed direction (2026-05-04): Option C — Greenfield native core + CCCP as reference lab.**

Implementation sequencing now in effect:

1. CCCP local build verified (see [[engine/cccp-build-run-audit]]). Use it for mechanics archaeology, feel comparison, equipment-role taxonomy, AI lessons. **Do not edit it.** Interactive mission proof still useful as a feel-comparison reference, not as an engine commitment.
2. Continue browser/canvas A1+ prototyping for fast feel iteration; treat the browser lab as a **harness**, not the engine answer.
3. Stand up a small greenfield experiment in the candidate native stack (TBD per Slice-A evidence) the moment browser-only stops being honest about feel/perf. Compare side by side.
4. License/usage ledger applies: anything copied from CCCP into the greenfield core gets logged in [[references/usage-ledger]] with a replacement plan.

Options A, B, D, E remain in the record below for audit, but are no longer the leading direction.

## Prototype Or Validation Plan

| Test | What It Proves | Pass/Fail |
|---|---|---|
| CCCP Linux build, run a vanilla mission. | Inheriting works on a reproducible CI-like target. | Pass = playable; Fail = build/runtime broken. |
| CCCP native macOS configure/compile/startup. | Whether this machine can build and launch the active repo. | Current = configure/compile pass; bounded menu/tutorial startup pass/partial; see [[engine/cccp-build-run-audit]]. |
| CCCP native macOS interactive mission proof. | Whether the local build is actually playable. | Open = capture menu screenshot/video, start one vanilla mission, test input/audio/rendering, and archive logs. |
| License review report (AGPL + deps). | Distribution feasibility. | Clean = continue; Conflicts = re-scope. |
| Greenfield actor-feel prototype (controller + 200x200 terrain destruction). | Time-to-fun in our stack. | Pass = under 2 weeks to "fun"; Fail = > 4 weeks. |
| `.rte` data parser proof-of-concept. | Compatibility option (D) is viable. | Pass = parses Base.rte without crashes; Fail = unblocked schema work needed. |

## Risks

| Risk | Mitigation |
|---|---|
| AGPL-3.0 viral concerns block our distribution model. | License review before any code reuse decision. |
| Greenfield blows out schedule. | Strict time-box on prototype; explicit kill criteria. |
| Compatibility rewrite never reaches parity. | Define a "supported subset"; document gracefully unsupported features. |
| Choosing Option A locks us into legacy AI/network code that we then have to rewrite anyway. | Audit AI + networking before locking; budget rewrite as Phase 2. |
| Community read of "fork vs new game" affects momentum. | Communicate honestly; preserve credit. |

## Revisit Trigger

Reopen this decision when:

- Build/runtime audit completes (or fails).
- Build/runtime compile proof is paired with a real interactive mission run result.
- License/reuse matrix decision (DR-010) is recorded.
- Greenfield actor-feel prototype is benchmarked.
- A new player-promise emerges (e.g. competitive PvP, paid mods) that materially changes the calculus.

## Source Trail

- [[repos/cccp-active-unified-repo]]
- [[repos/c4-continuation-engine]]
- [[comparisons/cccp-vs-c4]]
- [[design/opportunities-for-our-fork]]
- [[engine/architecture]]
- [[engine/cccp-build-run-audit]]
- [[engine/network-terrain-replication-lifecycle]]
- [[systems/networking-backend-frontend]]
- [[systems/modding-package-and-workbench]]
