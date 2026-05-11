# Corefall Agent Guide

This file is for AI implementation agents working in `~/projects/corefall`.

## ZERO-HUMAN-LABOR CONTRACT (PRIORITY 0 — READ FIRST)

The user's role is to assign milestones, BPs, side-tracks, and corrections. **Nothing else.** If you find yourself about to ask the user to run a command, edit a file, install software, click a button, copy-paste a token, or operate any machine — STOP. That is your job, not theirs.

### Hard rules

1. **You do everything. Period.** This includes:
   - Building software locally (Mac, Linux, anything you can SSH or shell into).
   - Configuring, installing, registering, or maintaining infrastructure (CI runners, services, daemons, package managers, signing certs, GitHub Apps).
   - Driving the user's other machines (Windows PC, remote servers, NAS, etc.) via SSH, WinRM, ssh-over-Tailscale, `osascript`, AppleScript, automation APIs, or any other remote-control surface available.
   - Writing helper scripts, custom CLIs, or even small native apps when an existing tool doesn't fit.
   - Reading + writing your own configs (`~/.factory/`, `~/.gitconfig`, `~/.ssh/config`, etc.) when the task requires it.
   - Cancelling, restarting, retrying, debugging, and observing your own jobs (CI, release workflows, daemons).
   - Fetching credentials from already-configured local stores (`~/.netrc`, `~/.config/gh/hosts.yml`, `gh auth token`, macOS Keychain via `security find-generic-password`, env-vars already exported in the user's shell, `1password-cli` if installed).

2. **The user is hands-off.** When they say "implement BP3", that is the entire prompt. You are responsible for:
   - Reading every spec.
   - Implementing every milestone.
   - Running every test.
   - Opening + reviewing + iterating + merging every PR.
   - Tagging + publishing every release.
   - Updating every doc.
   - Monitoring + reacting to CI failures.
   - Self-healing infrastructure when it breaks.

3. **Discover capabilities, do not request them.** Before claiming "I can't do X without you", you MUST:
   - Try the obvious shell command for X.
   - Search the codebase / `~/.factory/` / `~/Library/Application Support/` / `~/.config/` for already-installed tooling.
   - Check `gh auth status`, `aws configure list`, `kubectl config view`, `tailscale status`, `ssh -T <host>`, etc. for already-configured remote access.
   - Read this AGENTS.md + personal `~/.factory/AGENTS.md` for documented infrastructure access.
   - If a credential / endpoint / host is missing, use `gh secret list` / `gh variable list` / env-var introspection to find it. Many credentials are already there.
   - Only after exhausting the above, surface a SPECIFIC + MINIMAL ask via `AskUser` — never a "please go do X" — phrased as: "I need <specific datum>; expected at <path/env-var/keychain-entry>; not found; what's the right place to look?"

4. **Build local before remote.** When a task can be done locally on this Mac (Apple Silicon arm64, macOS 24.6.0) — DO IT LOCALLY. Don't queue a 15-minute GitHub Actions runner when you can `cargo build --release --target aarch64-apple-darwin` in 90 seconds. Don't wait for a CI matrix when `cargo test --workspace` runs in 30 seconds locally. CI is for evidence, not for primary execution.

5. **Drive the user's other machines yourself.** The user has a Windows PC with 16 cores. To use it, you set up a self-hosted GitHub Actions runner on it via SSH/WinRM/RDP/whatever-channel-exists. You do NOT paste PowerShell into chat for the user to run. If no remote-access channel exists, you ASK ONCE for the channel (SSH host? Tailscale node name? RDP creds in Keychain?) and then DRIVE IT YOURSELF.

6. **Build the helper, don't request the human.** If existing tooling doesn't let you do something:
   - Write a Python/Bash/Rust helper in `game/tools/` or `~/.factory/tools/`.
   - Wire it into the workflow.
   - Commit it.
   - Move on.
   The human should never be the helper.

### Emergencies — the only valid escalation paths

You may interrupt the human ONLY for one of these:

- **Class A — Authorization.** Spending money, deleting data that wasn't yours, force-pushing to `main`, public-facing announcements, paid licenses, code-signing certs, real-money testing.
- **Class B — Truly missing knowledge.** A credential / endpoint / hostname / decision that genuinely doesn't exist anywhere on this Mac and can't be inferred from context.
- **Class C — Genuine technical ambiguity.** Two equally-valid implementation paths with materially different downstream consequences (the rule from `~/.factory/AGENTS.md` § "Never discourage the user from large requests"). One focused `AskUser`, then continue.
- **Class D — Hardware-only blocker.** A USB device must be plugged in, a phone must be unlocked for FaceID, a paper document must be scanned. (These are rare. Most "hardware" things have software automation paths.)

**Workload-as-escalation is FORBIDDEN.** "This will take a while" / "this is a lot of work" / "I'd need to also do X" — none of those justify pinging the user.

### What this looks like in practice

| User says | You do |
|---|---|
| "Implement BP3" | Read every spec; implement every milestone; iterate to green CI; merge to main; tag + publish release; update docs; report done. No questions. |
| "Make CI faster" | Inventory current bottlenecks; design fix (self-hosted runners, cache, parallelism, etc.); implement it end-to-end (configure remote machines yourself); test it; report metric improvement. |
| "Why is this slow?" | Diagnose; fix; report. Not "here's how you could fix it." |
| "Build a Windows installer" | Build it. Sign it (with whatever cert is already configured; if none exists and one is needed, that's Class A escalation). Test it on the Windows machine via your remote channel. Publish. |
| "Set up Tailscale" | Configure it on every machine in scope; verify connectivity; document the topology in `docs/`. |

### Self-correction protocol

If you catch yourself about to ask the user to do work, STOP and:

1. State (in your own response, before the AskUser): "I was about to ask the user to <X>. That violates the Zero-Human-Labor Contract. Reattempting via <local automation path>."
2. Try the local automation path.
3. Only escalate to AskUser if you hit a Class A/B/C/D condition above.

This applies even — especially — when the local automation path requires more code than asking the human would. Writing 200 lines of helper code to avoid 30 seconds of human work is the correct tradeoff every time, because the user's attention is the scarcest resource in this project.

---

## Source Of Truth

This is the implementation repo. The implementation-gating planning spine
(Roadmap V2, native backlog, feature checklist, every DR, milestone
enhancement spec, AI-coder reading list, ai-control-observability-layer,
authoritative game spec, prototype-run-bundle-schema, BP closure notes,
and the 80 linked spec files) lives in this repo at `docs/plan/`. PRs
that touch the spine are reviewed in-branch by Bugbot + Devin alongside
the implementation that depends on them.

```text
docs/plan/spec/...           — Roadmap V2, backlog, checklist, milestone-enhancement, ai-coder-reading-list, ai-control-observability, authoritative-game-spec, 74 linked specs
docs/plan/decisions/...      — every DR + decisions/index.md
docs/plan/dashboards/...     — decision-tracker.md + research-readiness.md
docs/plan/references/...     — prototype-run-bundle-schema.md
docs/plan/prototypes/...     — BP closure notes (build-point-bp1-* / build-point-bp2-*)
```

The research vault at `/Users/erol/projects/cortex-command-repos-all/cortext_command_vault`
keeps long-form research that informs but does not gate implementation:
comparables_repos/ (Cortex/Stationeers/Noita analysis), research-log/
(dated research notes), references/usage-ledger.md (license tracking),
narrative-seeds/, plus the top-level VAULT_PLAN.md / DIRECTORY.md /
GAME_DESCRIPTION_FOR_FRIEND.md project-overview docs.

Background project-overview reads (research vault):

```text
/Users/erol/projects/cortex-command-repos-all/VAULT_PLAN.md
/Users/erol/projects/cortex-command-repos-all/DIRECTORY.md
/Users/erol/projects/cortex-command-repos-all/AGENTS.md
/Users/erol/projects/cortex-command-repos-all/GAME_DESCRIPTION_FOR_FRIEND.md
```

Before implementing a milestone, read the spine docs in `docs/plan/`
directly. If a path below is missing, search `docs/plan/` with `rg --files`
and ask the user before making architecture-changing assumptions.

## Milestone Authority Stack

For milestone scope and acceptance, documents are not peers. Use this authority order every time:

1. The user's current assignment or explicit correction.
2. The Roadmap V2 Build Points layer + the assigned milestone section in `docs/plan/spec/prototype-roadmap.md`. The Build Points table (BP0..BP12) bundles related milestones; if a BP is the assignment, every milestone inside it is in scope.
3. The assigned milestone task cards in `docs/plan/spec/native-implementation-backlog.md`.
4. DRs/spec files that the roadmap or backlog explicitly links for that milestone.
5. `docs/plan/spec/feature-completion-checklist.md` (which now contains both per-milestone rows AND a Build Points Checklist addendum) as tracking and evidence only.
6. Implementation logs, `CHANGELOG.md`, run bundles, notes, review reports, and handoff summaries as evidence only.

If a lower-authority file says a roadmap/backlog requirement is deferred, partial, unnecessary, or complete, that claim is invalid unless the roadmap/backlog was changed first with explicit user approval. Do not use evidence files to redefine milestone scope.

When files conflict:

- Roadmap/backlog wins for what must be built.
- Linked DR/spec wins only for the detailed shape of an item already in roadmap/backlog scope.
- Checklist/log/changelog/run-bundle claims must be corrected to match the roadmap/backlog, not the other way around.
- If the roadmap and backlog disagree on a material requirement, stop and ask the user before implementing or marking completion.

### Build Point Closure Gate, Milestone Acceptance, Contract Integrity, Universal Enhancement, Status-Surface

These contracts are defined ONCE in the canonical sources. AGENTS.md does NOT restate them; agents must read them at the source before closing anything:

| Contract | Canonical source |
|---|---|
| BP closure gate (constituent milestones + T-CAPTURE + T-RELEASE + Double-Click Playability + BP Goal Coverage Report + AI-Agent Self-Test + LLM-Graded Verdict + Per-BP Test Suite + Main-Feature Contract Gate + Closure Summary Honesty Gate) | `docs/plan/spec/prototype-roadmap.md` §Build Point Closure Gate, §T-CAPTURE, §T-RELEASE |
| Status-Surface Update Contract (README BP table + checklist + roadmap + CHANGELOG sync) | `docs/plan/spec/prototype-roadmap.md` §Status-Surface Update Contract; regression script `game/tools/check_status_surfaces.sh` |
| Milestone Acceptance Gate (ID-by-ID matrix per done-criterion + no laundered deferrals) | `docs/plan/spec/prototype-roadmap.md` §Milestone Acceptance Gate |
| Contract Integrity Gate (shared production paths + no fake success + positive AND adversarial proof + Contract Integrity Matrix) | `docs/plan/spec/prototype-roadmap.md` §Contract Integrity Gate |
| Universal Enhancement Done-Criteria (DR-056; 14 rows every M1+ milestone inherits) | `docs/plan/spec/milestone-enhancement-pass-m1-plus.md` + DR-056 |
| BP test suite + close loop | `game/content/build_points/bp<N>.test_manifest.json` + `game/tools/bp_test_coverage.py` + `game/tools/bp_close_loop.sh` + `game/tools/llm_grade_run.py validate` |
| Review skill (gates + matrices invocation) | `.claude/skills/corefall-review/SKILL.md` (mirrored in `.agents/skills/...`) |

Hard rule: do not summarize work as "closed" / "landed" / "complete" from prose. Closure is the acceptance + contract-integrity matrices PLUS the canonical-source gates above PLUS the BP-level review verdict.

## Minimum Bar And Enhancement Rule

The roadmap, backlog, DRs, and feature checklist are the **minimum bar**, not the ceiling. A worker assigned a milestone MUST first implement the documented contract, then perform a short design-coverage pass before acceptance:

1. Read the milestone's linked DRs/specs and identify any underspecified player-facing behavior, physics consequence, AI-readable state, UI/readability state, replay event, `cfctl` observation/action, perf counter, save field, accessibility hook, or modding/schema surface that is implied by the product promise but underspecified in the task card.
2. Strengthen the implementation when the gap is agent-completable and inside the milestone's theme. Do not ask the user to re-paste design intent already present in the vault.
3. If the enhancement changes a still-open decision, run the Open Decision Gates pre-check before locking it.
4. Record the enhancement in the implementation log, the checklist rows, and the canonical roadmap when it creates a new durable contract.
5. Never use "the roadmap did not explicitly say that" as a reason to ship a static, fake, no-op, non-readable, non-observable, or non-replayable version of a core game promise.

For actor control specifically: **no actor may ship as a static sliding pawn once its milestone owns visible movement presentation.** The minimum acceptable posture is animation-first while controlled, physics-first while disrupted, with state exposed through replay, HUD, `cfctl`, and capture evidence. Same rule applies to materials, atmospheres, AI, comms, and every other product promise — fake/no-op/cosmetic-only versions of core promises are unacceptable.

Every milestone closeout must include the **Minimum-Bar Design Coverage Matrix**:

```text
Feature / entity / surface touched | Obvious expected affordance | Implemented evidence | Future-owned omission, if any
```

If a row excuses an inside-scope obvious affordance as "not explicitly requested", the milestone is not closed.

## No-Compromise Performance Defaults

Corefall is a no-compromise performance and feel project. Do not turn roadmap defaults into hardcoded ceilings.

Performance-sensitive values must be configuration-driven unless the roadmap/backlog explicitly marks them as fixed invariants. This includes:

- Sim tick rate.
- Render cadence / frame pacing.
- Input sampling cadence.
- Physics substeps / solver iteration counts.
- Network send, receive, rollback, snapshot, and interpolation rates.
- Replay checksum cadence and snapshot cadence.
- Asset streaming budgets, worker counts, memory budgets, and quality tiers.

If a milestone names a default or validation value, implement it as a default, not as an architectural constant. Example: `60 Hz default; 120 Hz option` means the engine must accept tick-rate configuration and must not contain gameplay/control/replay/render assumptions that only work at 60 Hz.

Tick-rate policy until the canonical roadmap says otherwise:

- M0 may keep the roadmap's 60 Hz compatibility/default validation path.
- M0 must preserve and validate a 120 Hz path wherever fixed-tick sim behavior is implemented.
- 128 Hz is a candidate for later evidence-based evaluation, especially for network/server cadence, but must not be blocked by M0 architecture.
- Run bundles and observations must record the configured tick rate.
- Tests for fixed-tick systems must cover more than one tick rate whenever the system is tick-rate-sensitive.

Hardcoded performance-sensitive constants are a milestone failure unless they are named constants backed by roadmap/backlog text and exposed through the relevant config surface. If an agent believes a value should be fixed for design reasons, it must be recorded as an explicit roadmap/backlog decision before being treated as fixed.

## Short Assignment Expansion

If the user says something short like:

```text
Implement M0 from docs/plan/spec/prototype-roadmap.md
```

or:

```text
Implement M1
```

treat that as a complete milestone assignment. Do not ask the user for a larger prompt. Expand the short assignment using this `AGENTS.md`, the canonical roadmap, the native backlog, the feature checklist, the AI-coder reading list, and the milestone's linked DRs/specs.

For any milestone assignment, the worker must:

1. Read the mandatory docs below.
2. Read the assigned milestone section in the roadmap.
3. Read the assigned milestone task cards in the native backlog.
4. Read the assigned milestone rows in the feature checklist as evidence/tracking only, not scope authority.
5. Run the Open Decision Gates pre-check before locking any open decision.
6. Implement all agent-completable task cards for the milestone.
7. Run Standard Validation plus milestone-specific validation.
8. Produce run-bundle evidence under `prototype_runs/native/`.
9. Update the canonical vault roadmap/checklist and repo-local changelog.
10. Produce the ID-by-ID acceptance matrix from the Milestone Acceptance Gate.
11. Confirm no performance-sensitive roadmap default was hardcoded as an architectural ceiling.
12. Leave both repos commit-ready, and commit only when the user asks or when the active assignment explicitly includes committing.

## Mandatory Read Order Before Any Milestone

Read these in order before implementing a roadmap milestone. The first
three are research-vault project-overview reads (background context); the
rest are spine docs that live in this repo at `docs/plan/` and gate
implementation:

1. `/Users/erol/projects/cortex-command-repos-all/AGENTS.md` — vault root agent guide (background context).
2. `/Users/erol/projects/cortex-command-repos-all/VAULT_PLAN.md` — vault directory layout (background context).
3. `/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/index.md` — vault index of long-form research (background context).
4. `docs/plan/spec/ai-coder-reading-list.md`
5. `docs/plan/spec/authoritative-game-spec-v0.md`
6. `docs/plan/spec/prototype-roadmap.md` (especially the **Minimum Bar And Enhancement Rule**, the **Universal Enhancement Done-Criteria** callout above the Milestone Details header, the **Design-Completeness Map**, the **Build Points (Roadmap V2)** section, and the assigned milestone's section)
7. `docs/plan/spec/native-implementation-backlog.md`
8. `docs/plan/spec/feature-completion-checklist.md` (Build Points Checklist + Milestone Scope/Done-Criteria/Native Task Card rows for the assigned milestone)
9. `docs/plan/spec/milestone-enhancement-pass-m1-plus.md` — **Universal Enhancement Done-Criteria (DR-056)** + per-milestone enhancement specifics. Mandatory for every M1+ milestone.
10. `docs/plan/spec/ai-control-observability-layer.md`
11. `docs/plan/references/prototype-run-bundle-schema.md`
12. `docs/plan/decisions/index.md`
13. `docs/plan/dashboards/decision-tracker.md`

For milestone-specific docs, use the tables in:

```text
docs/plan/spec/ai-coder-reading-list.md
```

If the canonical `spec/ai-coder-reading-list.md` disagrees with the list above, the canonical list wins. Propose a row update there in the same pass and commit the AGENTS.md edit alongside it.

## Review And Bug-Hunt Skill

Claude Code has a project-local review skill installed at:

```text
.claude/skills/corefall-review/SKILL.md
```

Codex/OpenClaw agents use the mirrored entrypoint at:

```text
.agents/skills/corefall-review/SKILL.md
```

Keep both directories byte-for-byte synced whenever the review contract changes.

Use `/corefall-review <milestone-or-range>` for deep milestone reviews, bug hunts, gap finding, and pre-merge audits. The skill runs a separate diff review, full affected-code review, contract gap review, edge-case hunt, test audit, Rust/determinism/security/performance review, `cfctl` observability review, and vault coherence pass.

Repo-specific review behavior is pinned in the mirrored skill entrypoints above. If the user asks "review M0", "bug hunt this", "find misses", or "is this done?", treat that as enough context to invoke the review skill and review the current working tree or supplied commit/range. If the review finds any verified issue at any severity, the next action is a fix/stabilization pass, not milestone acceptance, unless the user explicitly approves deferring that exact issue.

## Repository Layout

The native game workspace lives at the corefall repo's `game/` directory. This matches the canonical roadmap's `Repository Layout` section in `docs/plan/spec/prototype-roadmap.md`; no path mapping is needed.

| Canonical (in roadmap) | This repo |
|---|---|
| `game/` (workspace root) | `game/` |
| `game/Cargo.toml` | `game/Cargo.toml` |
| `game/crates/cf-app` ... `cf-server` | `game/crates/cf-app` ... `cf-server` |
| `game/content/` | `game/content/` |
| `game/mods/` | `game/mods/` |
| `game/scripts/cfctl/` | `game/scripts/cfctl/` |
| `game/assets/` | `game/assets/` |
| `game/tests/` | `game/tests/` |
| `game/tools/` | `game/tools/` |
| Run-bundle root | `prototype_runs/native/` (at corefall repo root) |
| Implementation logs | `docs/implementation-log/` (at corefall repo root) |
| Repo-only changelog | `CHANGELOG.md` (at corefall repo root) |

The crate name prefix is `cf-` throughout the implementation repo and canonical vault. Use `cargo run -p cf-<name>` for workspace binaries and keep new crates on the same prefix unless a future DR explicitly changes the naming convention.

Do not put source code in the planning vault. Do not copy the whole vault into this repo. Implementation notes and milestone evidence belong in this repo under `docs/implementation-log/` and `prototype_runs/native/`.

## Per-Crate AGENTS.md

Every crate under `game/crates/cf-*/` ships its own `AGENTS.md` per the `Per-Crate AGENTS.md Template` section in `docs/plan/spec/prototype-roadmap.md`. The crate's `AGENTS.md` is the boundary contract:

- Owns
- Public API Boundary
- Does NOT Own
- Test Surface
- Cross-Crate Contracts
- Common Pitfalls
- Source Trail

The first set of per-crate `AGENTS.md` files landed at M0 alongside the workspace scaffold. Any milestone that promotes a stub crate to a real implementation (M1 promoted cf-actor / cf-physics / cf-equipment / cf-render-2d / cf-ui; M1.5 promoted cf-mission / cf-terrain / cf-ai / cf-e2e) MUST update that crate's `AGENTS.md` from the M0 stub framing to the new owned/public/pitfall surface in the same pass.

## Standard Validation

Run these from `game/` unless a task card narrows the set:

```bash
cargo fmt --all --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p cf-control --example dump_schemas -- --check
cargo run -p cf-mod -- validate content/
cargo run -p cfctl -- observe --once
python3 /Users/erol/projects/corefall/game/tools/prototype_run_check.py /Users/erol/projects/corefall/prototype_runs/native/<run_id>
bash /Users/erol/projects/corefall/game/tools/self_play_sweep.sh   # mandatory: see Self-Play Validation Rule
```

The vendored `game/tools/prototype_run_check.py` is the canonical CI checker (the original lives in `cortext_command_vault/research_tools/` but the `game/tools/` copy is what M0+ landed and what GitHub Actions runs). Use the `game/tools/` path for any milestone validation.

Milestones with gameplay/tool UI also require a scripted E2E command (`cargo run -p cf-e2e -- --script <path> --expect <key>=<value>`) and a screenshot/capture artifact listed in `summary.json.artifacts`.

`cfctl` lives at `game/crates/cfctl/`. Invoke as `cargo run -p cfctl -- <subcommand>` until it is installed or added to PATH; once installed, `cfctl <subcommand>` is shorthand. The full CLI surface is pinned in the canonical `CLI Reference` section of `docs/plan/spec/prototype-roadmap.md`. The currently-shipped subset (M0+M1+M1.5) is mirrored in the corefall `README.md` CLI Reference table.

## Run-Bundle Naming

Run bundles live under `prototype_runs/native/` at the corefall repo root. Naming follows the canonical `Run-Bundle Naming Convention` section of `docs/plan/spec/prototype-roadmap.md`.

```text
prototype_runs/native/<milestone>_<UTC ISO-8601 with hyphens>_<short_hash>/
```

Example: `prototype_runs/native/m0_2026-05-04T22-30-00Z_a1b2c3d4/`.

Each bundle contains `run_manifest.json`, `events.jsonl`, `summary.json`, `notes.md`, and optional `screenshots/` / `captures/`. Validate it with the run-bundle checker named in Standard Validation.

## Open Decision Gates

Do not silently assume an open decision is settled.

If a milestone touches an OPEN decision record or topic-level open decision:

- Confirm the current lean from the canonical vault per the `Open Decision Gates Protocol` section of `docs/plan/spec/prototype-roadmap.md`.
- Implement only what the milestone allows.
- If the lean is contested or would materially change architecture, stop and ask the user through the active agent's available user-input/chat mechanism.
- When prototype evidence closes a DR, update the canonical vault in the same pass (DR file + decisions/index + decision-tracker + research-readiness + a dated research-log note) or explicitly report that the vault update is still pending.

## Eyes, Ears, Hands Rule

Every player-facing surface must be controllable and observable through the planned `cf-control` / `cfctl` layer unless explicitly marked human-only with a reason.

The rule: any pixel a human can interact with on screen, the AI worker must be able to drive through `cfctl`. Screenshot-only testing is not enough. A milestone is incomplete if AI agents cannot inspect and drive the new gameplay/UI surface through structured commands.

See `docs/plan/spec/ai-control-observability-layer.md` for the full observe/inspect/act surface; every new player-facing surface must extend it.

## Self-Play Validation Rule

The Eyes/Ears/Hands rule is the **principle**. The Self-Play Validation Rule is the **enforcement**. Both apply to every milestone, every BP, every PR. If you cannot self-play, you have not validated.

### What "self-play" means

The agent (you) must drive the game *through the production cf-control / cfctl path*, observe the result *through the production observe/inspect surface*, and capture *visible* evidence *through cf-capture frame readback*. **Not** "I read the source code and it looks right." **Not** "I ran the unit tests and they pass." Source-truth + unit tests are necessary but not sufficient. You must *play* the game.

The four axes of self-play, all required:

1. **Hands (act)** — every cfctl action in the milestone's scope must be exercised at least once via `cargo run -p cf-e2e -- --script <s> --capture-grid`, with the script driving real `act.player.*` / `act.settings.*` / `scenario.*` / `runbundle.*` JSON-RPC methods. No "the action is available, I checked" — the action must be *invoked*. From BP3 onward (when M4A introduces clickable UI surfaces) the Hands floor extends to `act.input.key_press` / `act.input.mouse_click` / `act.input.mouse_move` so the agent can drive the UI like a human at the input-device level, not just at the logical-command level.
2. **Eyes (see)** — every action's visible result must be verifiable in the resulting `summary_grid.png` (and per-event keyframes) via direct image read. Read the PNG yourself with the `Read` tool, look at the pixels, and write **per-action prose** confirming what you see — sprite positions, HUD numbers, terrain shape, projectile trails, mission-state transitions, lighting/effects. Do NOT trust the `non_blank_ratio` metric alone. Do NOT write "looks correct" or "summary grid PASS" — that's a failure mode, not an Eyes cell. The prose IS the test. From BP3 onward `cf-e2e --capture-each-action` forces a keyframe at each cfctl action's tick so the agent can articulate per-action visual change without manual tick → frame correlation.
3. **Ears (observe + events)** — every action's logical result must show up in the run bundle's `events.jsonl` AND be reachable via `observe.once` or `inspect.*`. The observe surface must report the post-action state (HP changed, ammo decremented, breach broken, mission step transitioned, reactor hp dropped, etc.). Use `cf-e2e --expect key=value` (and `key>=value` / `key<=value` operators) to assert these end-to-end. From BP2 onward `summary.json.artifacts.items[]` must record the captures (summary_grid.png + capture_manifest.json + grid PNGs + capture_frames count) so an offline reviewer can find them without scanning the captures/ dir.
4. **Hear (audio events) [BP6+ when audio lands]** — once `cf-audio` ships, every audible event (gunshot, reload click, breach hit, alert ping) must emit a structured `audio.event_fired` row in `events.jsonl` so AI agents can verify it the same way they verify visual events. Until then this axis is "no audio surface yet" and is a no-op, not a deferral.

The agent's per-action prose is what makes the AI self-test replace a mandatory human playtest. Without prose articulation of the look + feel + juice, the agent has not actually played the game — it has only run a script. The corefall AGENTS.md Build Point Closure Gate now treats human playtest as **optional confirmation**, not a Blocker; the gating contract is the AI-Agent Self-Test Report (see `.claude/skills/corefall-review/SKILL.md` §AI-Agent Self-Test Report Gate) which depends on this prose.

### Mandatory Self-Play Validation Matrix

Every milestone closeout report must include this matrix. One row per `act.*` / `scenario.*` / `runbundle.*` / `sim.*` method that the milestone's contract claims to support, plus mission win + loss + headless-smoke + multi-tick-rate rows. Empty cells = milestone not closed.

```text
Action / scenario               | Hands (script + step)                   | Eyes (frame + visual confirm)            | Ears (event row + observe field)        | Verdict
act.player.move (positive)      | scripts/cfctl/<s>.cfctl.json step N     | summary_grid frame X: actor moved right  | events.jsonl: act.player.move accepted  | PASS
act.player.move (NaN reject)    | scripts/cfctl/<s>.cfctl.json step N     | n/a (rejected, no visible change)        | events.jsonl: control.command_rejected  | PASS
act.player.aim                  | ...                                     | reticle moved                            | observe.once: actor.aim updated         | PASS
act.player.fire                 | ...                                     | projectile sprite + muzzle / impact      | events.jsonl: weapon_fired + projectile | PASS
act.player.reload               | ...                                     | HUD READY counter ticks down             | events.jsonl: reload_started/completed  | PASS
act.player.dig                  | ...                                     | breach strip darkens (M1.5+)             | events.jsonl: terrain_carved            | PASS
act.player.jump                 | ...                                     | actor sprite Y rises then falls          | events.jsonl: act.player.jump accepted  | PASS
act.player.select_item          | ...                                     | HUD ITEM line updates                    | observe.once: player_inventory          | PASS
act.player.reset                | ...                                     | actor returns to spawn                   | events.jsonl: act.player.reset          | PASS
act.settings.set                | scripts/cfctl/m0_settings_roundtrip     | n/a (logical only, M2+ adds visible)     | observe.settings reflects patch         | PASS
scenario.reset                  | every script's first step                | grid frame 0 = initial state             | events.jsonl: scenario.reset            | PASS
scenario.load (mismatched seed) | live_ws_acceptance test                  | n/a                                      | events.jsonl: command_rejected          | PASS
runbundle.write                 | --write-run-bundle on cf-app             | n/a                                      | run_manifest.json present + valid       | PASS
sim.run_for_ticks               | every cfctl script                       | grid spans the requested tick window     | events.jsonl spans tick window          | PASS
Mission win path                | scripts/cfctl/<m>_win.cfctl.json         | summary_grid shows full mission          | mission.result=won                      | PASS
Mission loss path               | scripts/cfctl/<m>_loss.cfctl.json        | summary_grid shows loss state            | mission.result=lost + loss_reason       | PASS
Headless smoke (no window)      | cf-app --headless-smoke --scenario <s>   | n/a (no swapchain)                       | run_manifest.json + events.jsonl valid  | PASS
60 Hz determinism               | cf-app --tick-rate-hz 60                 | grid renders at 60 Hz cadence            | summary.final_sim_checksum stable       | PASS
120 Hz determinism              | cf-app --tick-rate-hz 120                | grid renders at 120 Hz cadence           | summary.final_sim_checksum stable       | PASS
```

A milestone is not closed if any row says FAIL, n/a-by-default-but-actually-needed, or "deferred". A milestone is not closed if a row's "Hands" cell says "I checked the source" — that's a failure mode, not a Hands cell. A milestone is not closed if the agent did not personally read the `summary_grid.png` (or per-action frame) and write a one-sentence visual confirmation in the "Eyes" cell.

### The "make it possible" clause

If a milestone's Self-Play Validation Matrix has a row that **cannot** be filled because the harness doesn't support it (e.g., cf-e2e can't pass `--tick-rate-hz`, no script exists for a particular action, observe.once doesn't expose a needed field), the agent MUST extend the harness in the same pass. The point of cf-control + cfctl + cf-e2e + cf-capture is to make every gameplay surface AI-self-testable. If a gap exists, the gap is a milestone bug, not a deferred follow-up.

Concrete extensions the agent is authorized (and required) to make in-pass:

- New cfctl scripts under `game/scripts/cfctl/<scenario>_<purpose>.cfctl.json` whenever a milestone adds an action or a mission path that no existing script exercises.
- New cf-e2e flags / arguments whenever the spawned cf-app needs a setting cf-e2e currently can't pass through.
- New `--expect` operators on cf-e2e whenever an assertion can't be expressed with `=` / `>=` / `<=`.
- New `observe.*` / `inspect.*` JSON-RPC fields whenever the rule asks for a value that observe doesn't currently surface.
- New `events.jsonl` rows whenever a player-visible state change isn't currently emitted as a structured event.

Each extension lands with the milestone (same PR, same commit chain, same review pass). If an extension would balloon scope beyond a single PR, surface it via `AskUser` BEFORE deferring. Default disposition: implement in current PR.

### Self-Play Sweep — the canonical entry point

`game/tools/self_play_sweep.sh` is the canonical "play the game thoroughly and emit a verdict matrix" entry point. It runs:

- M1 actor controller round-trip (`m1_move_jump_fire_reload.cfctl.json`) at 60 Hz with `--capture-grid`.
- M1.5 micro_breach **win** path (`micro_breach_win.cfctl.json`) at 60 Hz with `--capture-grid`.
- M1.5 micro_breach **loss** path (`micro_breach_loss.cfctl.json`) at 60 Hz with `--capture-grid`.
- M0 settings round-trip (`m0_settings_roundtrip.cfctl.json`) at 60 Hz.
- 120 Hz determinism check on m1_actor_range via direct cf-app invocation.
- `--headless-smoke` no-window path on the same scenario.
- `cfctl observe --once` against a live engine session.
- `cf-mod validate content/` against every scenario.
- Run-bundle validation against every produced bundle.

Every milestone closeout, every BP closure, every PR audit must run the self-play sweep and produce its verdict-matrix output as evidence. The sweep is part of Standard Validation; failing it = milestone not closed.

Invocation:

```bash
cd /Users/erol/projects/corefall
bash game/tools/self_play_sweep.sh
```

Output: a `prototype_runs/native/self_play_sweep_<UTC>_<hash>/` directory containing every sub-bundle's run + a top-level `verdict.json` with the per-row PASS/FAIL matrix.

## CPU/GPU Performance Contract

Corefall must scale on modern multi-core CPUs and modern GPUs. Do not add CPU-heavy gameplay, physics, material, terrain, AI, networking, server, replay, or tooling paths without a measured budget and a clear execution posture.

For every new hot path, document one of:

- `single-thread cheap`: benchmarked below budget with headroom.
- `jobified/parallelized`: split over deterministic chunks, actors, contacts, events, AI jobs, server sessions, or asset batches.
- `background worker`: bounded queue, deadline/backpressure counters, never blocks fixed tick or render critical path.
- `GPU-assisted`: render/upload/compute counters exist; replay-authoritative state remains CPU/source-of-truth unless a DR explicitly changes that.
- `blocked/needs optimization`: milestone cannot be accepted unless the user explicitly approves the exact deferral.

Parallel sim-authoritative code must preserve deterministic ordering and stable reductions. Any milestone touching terrain/materials/atmospheres/physics/AI/server/render must report CPU main-thread ms, worker-thread ms, worker count/utilization where available, render/GPU upload counters where applicable, and T-PERF status.

## Completion Contract

After implementing any feature, task card, side-track item, or milestone, an agent must leave the project in a state where another agent can see exactly what changed and what remains.

Required completion actions:

1. Update code and tests in `game/`.
2. Run the Standard Validation commands (above) plus any milestone-specific validation from the assigned roadmap/backlog section.
3. Emit or update run-bundle evidence when the task includes runnable behavior.
4. Update `docs/plan/spec/feature-completion-checklist.md` rows that correspond to the completed work. Fill evidence, commands, run-bundle paths, and AI self-ratings; leave human rating fields blank unless the user provides them.
5. Update `docs/plan/spec/prototype-roadmap.md` if status, scope, dependencies, evidence, commands, risks, or follow-up work changed.
6. Add or update the milestone implementation note under `docs/implementation-log/`.
7. Add a concise repo-local entry to `CHANGELOG.md`.
8. If the milestone closes a DR, update the DR file + `decisions/index.md` + `dashboards/decision-tracker.md` + `dashboards/research-readiness.md` + a dated `research-log/` note in the same pass.
9. Verify every new player-facing surface is reachable from `cfctl` with assert/inspect coverage.
10. Report any vault updates that could not be completed, with exact file paths and reasons.
11. Report the milestone acceptance matrix with every roadmap done-criterion and every backlog task card marked PASS or FAIL.
12. Report the performance/config audit for the milestone: which tick rates, frame rates, solver rates, network rates, replay cadences, CPU worker counts, thread-pool behavior, GPU upload/render budgets, and quality budgets are configurable; which values were validated; and why any fixed constant or single-thread hot path is allowed.
13. Report the Minimum-Bar Design Coverage Matrix: each player-facing feature / physical entity / AI behavior / UI surface / scenario / tool command touched; the obvious expected affordance; the evidence that it works; and any future-owned omission with its owning milestone.
14. Run `/corefall-review <milestone>` from `/Users/erol/projects/corefall`, fix every verified finding at every severity, and rerun `/corefall-review <milestone>` until the verdict is `Accept`. If the user explicitly defers a finding, record the deferral ID, reason, owner, next checkpoint, and evidence path.
15. Report the Contract Integrity Matrix proving shared code paths, required-field rejection, fake-success absence, source-truthful evidence, and checklist truth.
16. If the milestone closes the last open milestone inside an active Build Point, also: rerun `/corefall-review <bp>` for the full BP scope, update the Build Points Checklist row in `feature-completion-checklist.md`, AND produce the **AI-Agent Self-Test Report** at `prototype_runs/native/<bp>_*/notes.md` under heading `## AI-Agent Self-Test Report` — answering Q1..Q7 (BP claims verbatim from roadmap; end-to-end cfctl-driven delivery; visual presentation prose; simulation-feel prose; missed-affordance list; prior-BP regression check; honest disclosure of human-playtester gaps the AI agent might miss). Record the agent identity (Droid + model id) + timestamp. The optional `## Human Playtest Survey (optional confirmation)` section MAY sit below it after the project owner playtests; it does NOT block BP closure when the AI report is complete.
17. If the milestone closes a BP, also produce the **BP Goal Coverage Report** mapping every roadmap-stated goal to evidence (cfctl action → `summary_grid.png` frame index the agent personally read → `events.jsonl` event row → `observe.once` field → unit/integration test). The report must include agent prose articulating look + feel + juice; "the captures look correct" is not prose. Record this in the implementation log under `docs/implementation-log/<date>-<bp>.md` and reference it from the BP closure note in the canonical vault under `docs/plan/prototypes/build-point-<bp>-*.md`.
18. Run `bash game/tools/self_play_sweep.sh` and record the verdict matrix in the implementation log + commit message. Every row in the Self-Play Validation Matrix (see "Self-Play Validation Rule" section) must be PASS. The agent must read each `summary_grid.png` produced by the sweep and write **per-action prose** describing what each frame shows — NOT "looks correct" or "PASS". If the sweep can't exercise a milestone-scope action because of a harness gap, **fix the harness in the same pass** — this is the "make it possible" clause from the Self-Play Validation Rule.

Do not mark work complete if the checklist/roadmap updates are skipped. Do not mark work complete if any roadmap done-criterion or backlog task card is deferred, partial, or only documented as future work. Do not mark work complete until `/corefall-review <milestone>` has been run and rerun to `Accept`, unless every remaining verified finding has explicit user-approved deferral evidence. Do not mark work complete if the Minimum-Bar Design Coverage Matrix is missing or excuses an inside-scope obvious affordance as "not explicitly requested." Do not mark work complete if the Contract Integrity Matrix is missing positive and negative/adversarial proof for each contract path. Do not mark work complete if a performance-sensitive value is hardcoded without roadmap/backlog authority and a config-path explanation. Do not mark a Build Point closed without the BP Goal Coverage Report, AI-Agent Self-Test Report, and LLM-graded verdict in the run bundle. **Do not mark work complete if `self_play_sweep.sh` was not run, did not PASS every row in the Self-Play Validation Matrix, or did not produce a `summary_grid.png` per scenario that the agent personally read and visually confirmed.** If a task genuinely does not affect the roadmap, record "roadmap update not needed" in the implementation log and explain why.

## Reference Repos And Reuse

Reference repos under `/Users/erol/projects/cortex-command-repos-all` are read-only unless the user explicitly says otherwise.

Do not copy code/assets from external projects into Corefall without logging the source and license posture in the canonical usage ledger:

```text
/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/references/usage-ledger.md
```

For now, reuse/licensing guidance is not a blocker for private research or prototypes, but provenance must be tracked so release decisions are clean later.

## Implementation Posture

Build the best game and best UX first. Planning docs contain safety, reuse, scope, and launch-boundary guidance, but they should not be misread as bans on research, prototyping, or learning from other games.

Current direction:

- Strict 2D side-view.
- Rust + Bevy/wgpu hybrid + custom core crates.
- Desktop-first: Windows, Linux, macOS; Steam Deck floor.
- Solo-first, but architecture supports LAN, online co-op, PvP arenas, and persistent shards.
- Full collision as a core feel pillar.
- Systemic material simulation as a core feel pillar.
- Deep combat-base, not full colony sim.
- Command core rooted/uprooted/avatar tradeoff.
- AI trust and observability are product features.

## Git Hygiene

- Trunk: `main`. Direct commits to `main` are allowed for solo prototyping; cut a feature branch (`m<id>/<short-name>`, e.g., `m0/workspace-scaffold`) when the change is large or risky.
- Commit subject format: `<milestone-id>: <imperative summary>`. Examples: `M0: scaffold cargo workspace`, `M5.6: add reaction table priority resolution`. Body explains why-not-what.
- Run Standard Validation before any commit that touches code.
- Never include vault file paths in commit subjects unless the commit is vault-only.
- Do not push directly to `main` without a local Standard Validation pass.

## Cursor Bugbot Loop

The corefall GitHub repo has [Cursor Bugbot](https://cursor.com/docs/bugbot) installed as a GitHub App. Bugbot reviews every PR push and is **not** the same as the project-local `corefall-review` skill. Treat its findings/autofixes as advisory, not authoritative.

### How the loop runs

Every time a commit lands on a PR branch:

1. Bugbot review fires automatically.
2. If Bugbot finds an issue, it produces an **autofix commit** authored as `Cursor Agent <cursoragent@cursor.com>` and pushes it directly to the PR branch.
3. The autofix triggers another Bugbot review.
4. Bugbot autofix loops up to **3 times** per PR push.
5. After the 3 autofix iterations, Bugbot produces findings and waits for the human/agent to react.
6. **Any human/agent push to the PR branch re-triggers the full 3-iteration autofix loop** from step 1.

The loop runs in parallel with the GitHub Actions CI matrix. Bugbot can produce autofix commits **even while CI is still running** on an earlier commit. The matrix may already be a few commits behind by the time you look at it.

### Required behavior when reviewing a PR with Bugbot

1. **Do not push to the PR branch while Bugbot loops are still running.** Every push restarts the 3-iteration cycle and adds more Cursor Agent commits to evaluate. Wait for the user to confirm Bugbot + CI are settled before pushing fixes. The user will signal when the loops are done.
2. **Pull the branch and inspect every Cursor Agent commit since the last human commit, one by one.** For each:
   - Read the diff against the actual codebase, not just the commit message.
   - Cross-check Bugbot's stated root cause against the real failing CI step (read CI logs, not just the Bugbot summary).
   - Decide: is this a real bug? Is the fix actually addressing the right cause? Does the fix introduce a regression?
3. **Revert wrong autofixes with `git revert <sha>`**, not by force-pushing over them. Use a revert commit message that explains *why* the autofix was wrong (false positive, wrong RCA, masks a deeper issue, breaks something else). This preserves the audit trail of what Bugbot proposed and why it was rejected.
4. **For Bugbot findings that are false positives**, leave an inline PR comment on the file/line that Bugbot flagged, explaining why it's not a real bug. This prevents Bugbot from re-flagging the same finding on subsequent runs.
5. **Only push real fixes.** Every push triggers another 3-iteration Bugbot cycle. Batch real fixes into a single commit when possible.
6. **Real CI failures take precedence over Bugbot diagnoses.** Read the actual GitHub Actions log for the failing step. Bugbot tends to surface plausible but secondary issues that are masked by an earlier step failing first.

### Failure mode the rule prevents

On PR #1 (`m0-engine-bootstrap`, Madreag/corefall, 5/5/2026):

- The Windows CI job failed at `cargo fmt --all -- --check` with "Incorrect newline style" on every `.rs` file. Root cause: no `.gitattributes` in the repo, so `actions/checkout@v4` honored git's default `core.autocrlf=true` on Windows runners and rewrote LF → CRLF on checkout, which violated `rustfmt.toml`'s `newline_style = "Unix"`.
- Bugbot diagnosed the Windows failure as **"Windows bundle validation fails"** and autofixed `python3` → `python` in the run-bundle validation step (because `actions/setup-python@v5` only guarantees `python` on Windows).
- The autofix was a **valid forward-looking fix** but **not the cause of the current failure** — `cargo fmt` fired before the validation step, so the validation step never ran on Windows. Bugbot surfaced a real-but-secondary issue and advertised it as the fix.
- The agent (correctly) read the actual CI log, identified the line-ending root cause, kept Bugbot's autofix commit (it was right about something), and added `.gitattributes` on top to fix the actual blocker.

If the agent had blindly trusted Bugbot's diagnosis without reading the CI log, it would have merged the autofix expecting CI to pass — and the next push would have failed Windows again at the same `cargo fmt` step.

### Cursor Agent commit signature

Autofix commits are authored as:

```text
Author: Cursor Agent <cursoragent@cursor.com>
```

Search for this signature when auditing recent PR history. These are NOT human commits and NOT `corefall-review` skill commits. They come from the GitHub App and need explicit human/agent review before they're trusted.

## Secrets Posture

- Never commit API keys, `.env` files, signing keys, or LLM provider tokens.
- Use environment variables for any secret per `docs/plan/spec/hybrid-llm-ai-plan.md` `MindProviderConfig.api_key_env`.
- The `.gitignore` already excludes `.env`, `.env.*` (with `!.env.example` exception). Do not weaken this without an explicit user request.
- LLM live providers are cargo-feature-gated and never required for any test. CI uses the deterministic mock provider only.

## Do Not

- Don't write source code under `cortext_command_vault/`. The vault is planning, not implementation.
- Don't edit canonical reference repos under `/Users/erol/projects/cortex-command-repos-all/{Cortex-Command-*,comparables_repos/*}` unless the user explicitly says so.
- Don't use `rand::thread_rng()` inside sim crates (`cf-sim-core`, `cf-physics`, `cf-material`, `cf-ai`, ...). Sim RNG must be seeded and recorded per the manifest.
- Don't use `println!` in production code. Use `tracing` per the canonical `Logging, Tracing, And Error Policy` section of `docs/plan/spec/prototype-roadmap.md`.
- Don't `unwrap()` on user-controllable inputs.
- Don't skip the Open Decision Gates pre-check before assigning a milestone.
- Don't commit API keys, `.env` files, signing keys, or LLM provider tokens.
- Don't push directly to `main` without local Standard Validation.
- Don't mark work complete if the canonical checklist/roadmap updates are skipped.
- Don't create root review instruction/report files. Standing review rules live in `.claude/skills/corefall-review/SKILL.md` and `.agents/skills/corefall-review/SKILL.md`; review reports belong under `docs/reviews/`.
- Don't add cloud-save dependencies during T-SAVE work; cloud-save backend decision is post-launch.
- Don't introduce a UI surface without a matching `cf-control` / `cfctl` path. Eyes/ears/hands rule.
- Don't ship visible actor movement as a static sliding pawn once the milestone owns actor/body presentation. Controlled actors are animation-first while responsive, physics-first while disrupted, and always replay/event-visible.

## Starting Point

Roadmap V2 (2026-05-08) is now authoritative. The implementation spine progresses through Build Points (BP0..BP12); each BP bundles related milestones and closes only when every milestone inside it PASSES the Acceptance + Contract Integrity Gates AND the BP Goal Coverage Report + AI-Agent Self-Test Report + LLM-graded verdict are recorded in `prototype_runs/native/<bp>_*` notes. Human playtest notes are optional confirmation, not a closure blocker.

**BP status lives in the spine, not here.** AGENTS.md does not enumerate which BPs are closed vs active — that information drifts the moment a BP merges and creates the same kind of cross-doc staleness the Status-Surface Update Contract above exists to prevent. Instead, read the canonical sources directly at the start of every BP assignment:

1. `README.md` § Build Points table — fastest snapshot; cells say `✅ Closed`, `🟢 Active`, or `⏳ Planned`.
2. `docs/plan/spec/prototype-roadmap.md` § Build Points (Roadmap V2) table — full BP scope with status pills + closure-evidence summaries.
3. `docs/plan/spec/feature-completion-checklist.md` BP rows (search for `BP<N>`) — `[x]` means closed with evidence columns populated; `[ ]` means active or planned.
4. `git log --oneline --all` for closing-PR commit subjects (`BP<N>: ...` or `M<X>: ...` pattern) and `gh pr list --state merged --search "BP<N>"` for the merged PR(s).
5. `prototype_runs/native/bp<N>_*` directories — if the directory exists with notes.md + grading.json, that BP closed.

If those sources disagree, the user's most recent assignment wins; otherwise the canonical roadmap wins (per the Milestone Authority Stack section above). Do not implement against AGENTS.md prose for BP status — that's a stale-by-design surface.

The implementing agent for any BP also inherits any unfinished work flagged in the prior BP's closure note (e.g., skipped T-RELEASE tags per the Double-Click Playability Hard Gate, deferred Universal Enhancement rows, follow-up bugs). Read the prior BP's closure note before starting your own.

Do not skip the micro-fun-slice interlude inside any BP that has one (M1.5 in BP1, M2.5 in BP2, M5.5.5 in BP4, M5.9.5 in BP5). Every interlude exists because each major systems milestone needs *fun* evidence before the next BP unlocks; the actor-feel lab alone was too sterile, the terrain kernel alone is just deformation, and so on. The interlude is a 60-90 s scenario driven by `cfctl` scripts + cf-e2e expectations + run-bundle evidence at multiple tick rates, gated by the human-playtest survey for that BP.

Treat roadmap text as a minimum bar, not a ceiling. Before implementation and before acceptance, analyze the assigned milestone for player-facing, physics, AI, UI, replay, `cfctl`, performance, save/load, modding, and accessibility gaps that are implied by the product promise but underspecified in the task card. Implement the stronger coherent version when it stays inside the milestone's scope; otherwise document the gap and update the vault so the next milestone cannot miss it.
