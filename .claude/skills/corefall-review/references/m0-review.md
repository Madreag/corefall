# M0 Review Overlay

M0 is allowed to be a foundation milestone. Do not fail it for lacking gameplay, actors, terrain, equipment, AI, multiplayer, materials, or real renderer work.

## M0 Must Have

- Cargo workspace under `/Users/erol/projects/corefall/game/`.
- The 29 planned `cf-*` crates or a documented, roadmap-approved exception.
- Per-crate `AGENTS.md` files.
- `cf-app` no-op app shell.
- Fixed 60 Hz sim island in `cf-sim-core`.
- Seeded deterministic RNG policy.
- `cf-control` JSON-RPC 2.0 schema surface.
- `cfctl` commands for the M0 observe/run/pause/step flow.
- `m0_blank` scenario manifest.
- Run-bundle writer with manifest, events JSONL, summary, and notes.
- Panic hook and structured tracing.
- CI for Linux, macOS, and Windows, if the milestone promised it.
- Accessibility placeholder flags from DR-012.
- Implementation log, changelog, and feature checklist updates.

## M0 Must Not Grow Into

- No gameplay actor controller.
- No real terrain/material simulation.
- No AI behavior.
- No equipment, damage, or physics systems beyond M0 placeholders.
- No remote control bind beyond loopback.
- No networking transport decision.
- No modding script host.
- No cloud-save dependencies.

## M0 Not Yet Testable

Mark these as `Not yet testable`, not as failures:

- Actor feel and player movement: M1/M1.5.
- Collision/material/terrain behavior: M2/M5.5/M5.6/M5.7.
- Replay closure: M3, though M0 must provide run-bundle envelope evidence.
- HUD/accessibility closure: M4, though M0 must expose settings flags.
- AI trust harness: M6.
- Server/multiplayer: M9-M12.

## M0 Blockers

- Workspace does not compile.
- `cfctl` cannot drive the no-op scenario.
- Run bundle missing required files or invalid JSONL.
- Sim tick/RNG/event surfaces are nondeterministic without explanation.
- Open DR gates were silently assumed.
- Required checklist/changelog/implementation-log updates are missing.
- Validation was skipped without a concrete reason.
- Any verified Low/Medium/High/Blocker review finding remains unresolved without explicit user-approved deferral.
- `cf-app` and `cfctl` produce different manifest metadata or use different scenario/config source-of-truth paths.
- JSON-RPC required fields such as `schema_version` are accepted when missing or malformed.
- `scenario.load` returns accepted while ignoring requested state like `seed`.
- Required task-card events/tests are absent while checklist rows are checked.
