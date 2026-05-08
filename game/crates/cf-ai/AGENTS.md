# cf-ai — AGENTS.md

## Owns
- M1.5 reactive enemy controller: `ReactiveGuard`, `ReactiveGuardParams`, `GuardState`, `Tactic`, `step`, `EnemyTickReport`, `PerceptionRecord`, `TacticRecord`, `FireRecord`, `GuardStateTransition`.
- DR-008 LEAN encoded as code: `Idle → Alerted → Engaged → Dead` job FSM (intent layer), `score_tactics`/`pick_tactic` utility scoring (tactic layer), aim-settle/miss-roll/burst pacing scripted hooks (custom layer). Closure of DR-008 happens at M6 — M1.5 implements inside the LEAN, not beyond it.
- Per-actor controller state cloned cheaply across ticks. `ReactiveGuard::reset` rewinds memory + ammo + cooldowns on `scenario.reset`. `checksum_bytes` participates in deterministic divergence guarantee.
- Anti-scope: no full AI doctrine system, no commander AI, no LLM mind layer, no AI-H acceptance suite, no humanlike-AI bar — those land at M6 / M6.5.

## Public API Boundary
- Types: `ReactiveGuardParams`, `ReactiveGuard`, `GuardState`, `Tactic`, `GuardTickInputs`, `EnemyTickReport`, `PerceptionRecord`, `TacticRecord`, `FireRecord`, `GuardStateTransition`, `ReactiveGuardView`.
- Functions: `step(&mut ReactiveGuard, GuardTickInputs, &mut Rng) -> EnemyTickReport`, `ReactiveGuard::new`, `ReactiveGuard::reset`, `ReactiveGuard::checksum_bytes`.

## Does NOT Own
- Recorder events / run-bundle writing → `cf-control` engine emits `ai.*` and `equipment.*`/`combat.*` events from the `EnemyTickReport`.
- Player input → `cf-actor` `ControlIntent` (the engine wraps both player + AI intent through the same sim path).
- Damage routing / chassis modules / armor layers → `cf-chassis` at M5.
- LLM mind / async cloud reasoning → `cf-ai::mind::*` at M6.5 (T-LLM).

## Test Surface
- Unit tests: `cargo test -p cf-ai` covers idle when player absent, engagement on player visible, no-fire during aim settle, fire after aim settles, out-of-cone non-engagement, dead-actor lock to `Dead`, deterministic same-seed playback, out-of-ammo reload, reset.

## Cross-Crate Contracts
- Depends on: `cf-actor` (`ActorState`/`ActorId`/`Status`/`Vec2`), `cf-sim-core` (`Rng`).
- Depended on by: `cf-control` (engine + scenario + observe envelope + run-bundle event emission).
- Events emitted by the engine from an `EnemyTickReport`: `ai.ai_perception`, `ai.tactic_chosen`, `ai.state_changed`, plus `equipment.weapon_reload_started`/`weapon_reloaded`/`weapon_dry_fire`/`weapon_fired` and `combat.projectile_spawned` when fire fires this tick.

## Common Pitfalls
- The miss roll uses the engine's seeded `Rng` (one `next_u64` per fire). Do NOT call `rand::thread_rng()` here — the determinism contract requires identical replay outputs from identical seeds.
- `ReactiveGuardParams` defaults are tuned for the M1.5 micro_breach scenario (sight 480, cone 120°, miss 0.35, burst 3, mag 12). Per-scenario overrides flow through `cf-control::scenario::ScenarioEnemy::build_params`.
- `step` early-exits to `GuardState::Dead` when the underlying actor's HP reached 0; future calls return an empty report. Resurrection requires `scenario.reset`.
- Burst pacing alternates miss-drift sign by `burst_shots_fired % 2` so visually-varied misses stay deterministic. Use `is_multiple_of(2)` in code instead of `% 2 == 0` to avoid the clippy lint.

## Source Trail
- spec/prototype-roadmap §M1.5 — Micro Breach Fun Slice (M1.5-002 reactive enemy).
- spec/native-implementation-backlog M1.5-002.
- DR-008 (AI architecture, OPEN — hybrid jobs + utility scoring + scripted hooks lean; M6 closes the DR; M1.5 implements inside the LEAN).
- DR-022 (humanlike AI bar, OPEN — closes at M6).
