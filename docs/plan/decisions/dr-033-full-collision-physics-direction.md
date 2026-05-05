---
type: decision
id: DR-033
status: closed-direction
priority: P0
closed_at: 2026-05-05
revisit_trigger: "M5.5 cannot meet 1080p/60 after broadphase and pair budgets; bullet-bullet collision adds no readable fun after prototype proof; full friendly collision makes AI teammates feel unfair rather than tactical; or networking evidence in M9/M10 shows a subset of collision pairs must be server-only or event-authoritative."
---

← [[decisions/index|decision records]] · [[dashboards/decision-tracker|decision tracker]] · [[spec/full-collision-physics-plan|full collision plan]] · [[spec/prototype-roadmap|native roadmap]] · [[spec/native-implementation-backlog|native backlog]] · [[systems/physics-and-destruction-models|physics/destruction systems]]

# DR-033: Full Collision Physics Direction

## Decision

Commit to **full physical collision as a core feel pillar**:

- Weapons, limbs, bodies, armor zones, mechs, terrain, objects, base modules, shields, debris, and projectiles all get collision identity.
- Player/player, player/unit, unit/unit, AI/AI, enemy/enemy, ally/ally, limb/limb, limb/body, limb/weapon, projectile/body, projectile/terrain, projectile/equipment, projectile/shield, and projectile/projectile interactions are expected unless explicitly filtered.
- Kinetic bullets that hit other bullets should deflect, fragment, tumble, or lose energy. Explosive rounds may detonate, fuze-fail, or deflect based on authored projectile profile.
- Physics impulse can damage limbs, armor, body parts, equipment, chassis modules, terrain, doors, shields, and base objects.
- Every meaningful contact must emit replay/debug events so the player, AI harness, and implementation agents can inspect cause chains.

This is not a brute-force all-pairs promise. It is a **full consequence contract** implemented with collision classes, proxies, broadphase, CCD tiers, pair filters, contact budgets, deterministic replay checks, and AI/dev observation.

## Chosen Option

| Option | Description | Outcome |
|---|---|---|
| A. Arcade collision only | Actors/projectiles collide with terrain and bodies; most limb/item/projectile-projectile interactions are fake or ignored. | Rejected. Too weak for the project's physical sandbox promise. |
| B. Full collision promise with staged implementation | Everything physical has a class/proxy/material/contact policy; expensive cases use selective CCD and budgets; roadmap adds T-PHYS and M5.5. | **Chosen.** Best fit for Cortex-like feel plus 4K/120 constraints. |
| C. Fully physically accurate all-pairs/per-pixel rigid simulation | Simulate everything against everything at maximum fidelity. | Rejected as product direction. Keep as moonshot experiments only; likely too costly and unreadable. |

## Why

The user's design intent is explicit: this game should not treat collision as a small shooter detail. Physical interaction is part of the fantasy. A player should be able to trust that:

- a rifle barrel can hit a wall,
- a mech foot can crush infantry,
- a falling door can injure armor or limbs,
- bullets can hit other bullets when the game says they physically cross,
- debris and dropped equipment matter,
- friendly bodies are not ghosts,
- AI sees and reasons about the same physical obstacles the player sees.

This also supports existing closed directions:

| Existing Direction | Collision Link |
|---|---|
| DR-014 tactical pulp sci-fi disaster sandbox | Physical accidents, impact damage, ricochets, and body blocking create pulp disaster stories. |
| DR-018 tiered consequence ladder | Rescue, injury, chassis loss, and command-core risk need contact cause chains. |
| DR-021 full mech ladder | Heavy armor/mechs only matter if mass, crush, and module contacts are real. |
| DR-022 humanlike AI bar | Bots must react to blocked corridors, bodies, debris, doors, terrain edits, and projectile danger. |
| DR-028 4K/120 target | Full collision must be budgeted, staged, and profiled from the start. |

## Evidence

| Evidence | Implication |
|---|---|
| Box2D, Rapier, dyn4j, Chipmunk, Jolt, Godot, Unity, Bullet, and DigitalRune all frame collision as staged broadphase/narrowphase/CCD/filtering/contact callbacks. | Full collision is feasible only if staged and filtered. |
| Continuous-collision references consistently warn about tunneling and CCD cost. | Fast projectiles and limbs need selective sweep/TOI; slow bodies do not all need expensive CCD. |
| GPU Gems broadphase notes show brute force is O(n^2). | The roadmap must forbid naive all-pairs and require broadphase/pair budgets. |
| Noita and Powder Toy references show pixel material worlds need proxy/dirty-region strategies. | Terrain pixels should remain authoritative; collision uses chunk proxies and exact samples only where needed. |
| Cortex Command source/vault notes show atom/body/terrain interactions are central to its signature feel. | Our game should preserve and extend physical consequence, not flatten it into hitboxes. |

See [[spec/full-collision-physics-plan]] for the 30+ source synthesis table.

## Roadmap Impact

| File | Required Change |
|---|---|
| [[spec/prototype-roadmap]] | Add DR-033 feed, T-PHYS side track, M5.5 Full Collision Gauntlet, COLL-001..COLL-012 validation, dependency graph, risk rows, anti-goals. |
| [[spec/native-implementation-backlog]] | Add concrete M5.5 task cards assignable to implementation agents. |
| [[references/prototype-run-bundle-schema]] | Add `collision` event category and M5.5 acceptance gate. |
| [[spec/authoritative-game-spec-v0]] | Add full physical collision to launch commitments. |
| [[systems/physics-and-destruction-models]] | Point physics/destruction readers at this plan for the current implementation contract. |

## Validation

Direction is considered implementation-ready when:

- COLL-001..COLL-012 are present in the native backlog.
- M5.5 sits after M5 and before M6 so AI can consume real collision affordances.
- `collision.*` events are part of the run-bundle schema.
- The roadmap forbids silent collision exceptions.
- The validation command matrix has a Full Collision Gauntlet E2E and replay check.

Direction is considered technically proven only after M5.5 produces a checked run bundle and replay/perf report.

## Risks

| Risk | Mitigation |
|---|---|
| Collision complexity hurts 4K/120 target. | Broadphase, class filters, CCD tiers, low-value debris budgets, perf gates. |
| Full collision becomes frustrating body-blocking. | UI readability, AI spacing doctrine, shove/recovery states, scenario softening where deliberate. |
| Self-collision destabilizes animation/limbs. | Connected-owner self filters with explicit `collision_filter_reason`; detached parts collide normally. |
| Bullet-bullet collisions are too expensive. | Projectile lane cache and selective `collides_with_projectiles` masks; preserve important/projectile classes first. |
| Replay/determinism drifts from contact order. | Stable pair ordering, deterministic broadphase tie-breakers, contact ids, first-divergence reporting. |
| AI ignores physical consequences. | M6 requires collision-aware perception and reason labels; COLL-012 proves reaction to blocks/debris/doors. |

## Revisit Triggers

- M5.5 cannot meet 1080p/60 after broadphase and pair budgets.
- Bullet-bullet collision adds no readable fun after prototype proof.
- Full friendly collision makes AI teammates feel unfair rather than tactical.
- Networking evidence in M9/M10 shows a subset of collision pairs must be server-only or event-authoritative.

Revisiting this record should tune the implementation, not erase the design promise without explicit user direction.
