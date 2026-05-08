---
type: spec
status: closed-direction
authority: "Animation system: hybrid sprite-sheet + skeletal-rigged hero chassis + procedural recoil/ragdoll/limb-tracking. Animation event tags drive replay events + audio + VFX synchronization."
ready_when: "Every roster actor has a complete animation set (idle/walk/run/jump/fire/reload/melee/death/limb-loss/eject); per-frame animation events fire correctly; ragdoll engages on death deterministically."
feeds:
  - DR-002
  - DR-003
  - DR-014
  - DR-019
  - DR-021
  - DR-024
  - DR-028
  - DR-033
  - DR-044
  - DR-045
---

← [[spec/index|spec section]] · [[spec/art-and-asset-pipeline|art pipeline]] · [[spec/visual-direction|visual direction]] · [[spec/vfx-and-particles|VFX/particles]] · [[spec/full-collision-physics-plan|full collision]] · [[decisions/dr-044-audiovisual-production-pipeline|DR-044]]

# Animation System

## Core Actor Presentation Rule

> [!important] Minimum bar
> The final design should not be a static sliding pawn. Controlled actors are **animation-first while responsive**, **physics-first while disrupted**, and **always replay/event-visible**. Normal locomotion uses readable walk/run/crouch/climb/jet animation; body physics layers on top through controlled secondary motion. When the actor is knocked down, stunned, dead, limb-damaged, depressurized, blasted, or wind-pushed, physics authority increases and the transition emits replay events.

| Situation | Expected behavior |
|---|---|
| Normal walking/running | Walk/run animation, foot contact, surface-aware footsteps, body weight, recoil sway. |
| Aiming while walking | Upper-body/arm aim pose blends over locomotion. Weapon tracks hand socket/IK. |
| Jetpack / low gravity | Body is moved by thrust + gravity. Legs/arms trail, swing, and react physically, while aim/control limbs remain stabilized enough to play. |
| Knocked down / stunned / dead | Physics takes over more strongly: ragdoll, limb collisions, tumbling, impact damage. |
| Limb damaged/lost | Animation changes: limp, one-arm weapon handling, crawl, fall, drop gear, disabled grip, slower climb. |
| Heavy armor / mech suit | Different gait, slower acceleration, more inertia, servos/hydraulics, different fall and landing forces. |
| Pressure/wind/explosion | Limbs/body can be pushed, spun, slammed, damaged, or pinned by physical forces. |

This is a playability rule, not only an art rule. `cf-anim`, `cf-actor`, `cf-physics`, `cf-chassis`, `cf-replay`, `cf-ui`, and `cf-control` must agree on the current posture/pose/physics-authority state.

## Approach

**Hybrid: sprite-sheet for non-hero + skeletal-rigged for hero chassis + procedural overlays for everyone.**

| Layer | Used For | Technology |
|---|---|---|
| **Sprite-sheet** | All non-hero actors (humans light/heavy, robots, drones, civilians, husks, turrets). 4-12 frames per action. | Bevy `bevy_sprite` + custom animation manifest. AI-generated per [[spec/art-and-asset-pipeline]]. |
| **Skeletal-rigged** | All 18 hero player chassis (3 PA tiers, 5 mech tiers, 4 robots, 4 androids, 1 drone) + named NPCs. Bone hierarchy + IK + animation curves. | `bevy_spine` (Spine runtime, free) OR `bevy_dragonbones` (DragonBones, free). Author in Spine Essential ($69) or DragonBones Pro (free). |
| **Procedural overlays** | Recoil, knockback, limb tracking (aim), ragdoll on death, weapon-IK to hand sockets, jet-flame intensity. | Bevy procedural transforms; physics engine integration via `cf-physics`. |

## Physics Authority Blend

| Mode | Primary owner | Physics behavior | Required events / observations |
|---|---|---|---|
| `controlled_locomotion` | Animation state machine | Root motion / velocity are controlled; limbs use secondary spring/inertia within stability limits. | `animation.state_changed`, `animation.tag_fired`, `actor.stance_changed`; `cfctl observe actor` shows pose + stance. |
| `controlled_airborne` | Movement controller + animation | Jet/jump thrust moves body; limbs trail under gravity/inertia/wind; aim arm remains stabilized unless damaged. | `actor.airborne_state_changed`, `animation.jet_thrust_pose`, `physics.secondary_limb_motion_sample` (sparse). |
| `braced_or_aiming` | Animation + weapon IK | Feet/torso/weapon sockets stabilize; recoil applies procedural overlay. | `animation.ik_target_updated`, `equipment.weapon_recoil_applied`. |
| `impaired_control` | Damage + animation | Limp, crawl, one-arm handling, disabled grip, reduced climb/jump/jet capability. | `body.limb_function_changed`, `actor.mobility_changed`, `equipment.grip_state_changed`. |
| `disrupted_physics` | Physics solver | Knockdown, stun, pressure gust, explosion, tumble, pinned/crush states raise physics authority. | `physics.authority_changed`, `collision.contact_impulse_applied`, parent-linked body/equipment damage. |
| `ragdoll_or_gib` | Physics solver | Death/limb detachment uses deterministic ragdoll/gib proxies. | `animation.ragdoll_begin`, `body.limb_detached`, `collision.*`, `body.gib_spawned`. |

## Animation Set per Chassis

Every actor MUST have these animations. Sprite-sheet variants are 4-12 frames; skeletal-rigged variants are continuous curves.

| Action | Frames (sprite-sheet) | Notes |
|---|---|---|
| `idle` | 4 (loop) | Subtle bob + breath. |
| `walk` | 8 (loop) | Cycle 8-frame walk; foot-anchor flag on frames 3+7. |
| `run` | 8 (loop) | Faster walk + lean. |
| `jump_takeoff` | 3 | Single-shot. |
| `jump_air` | 1 (loop) | Holds. |
| `jump_land` | 3 | Single-shot. |
| `crouch_idle` | 4 (loop) | |
| `crouch_walk` | 6 (loop) | |
| `prone_idle` | 4 (loop) | |
| `prone_crawl` | 6 (loop) | |
| `aim_up` / `aim_mid` / `aim_down` | 1 each | Pose; procedural blend per aim_pitch. |
| `fire` | 2-3 | Snap; per-weapon flash anchor. |
| `reload_short` | 6 | Magazine swap. |
| `reload_long` | 12 | Belt-fed / chamber reload. |
| `melee_strike` | 4 | |
| `melee_block` | 2 | |
| `throw_grenade` | 6 | |
| `damage_react_light` | 3 | Stagger. |
| `damage_react_heavy` | 5 | Knock-back. |
| `death_fall_back` | 6 | Then ragdoll. |
| `death_fall_forward` | 6 | Then ragdoll. |
| `death_explode` | 4 | Plus gib particles. |
| `limb_loss_arm` | 3 | Procedural ragdoll on limb. |
| `limb_loss_leg` | 4 | Plus crawl variant. |
| `eject_seat` | 8 | Mech ejection per DR-021. |
| `salvage_action` | 8 | Per DR-018. |
| `repair_action` | 6 | |
| `dig_action` | 6 (loop) | |
| `breach_action` | 4 | Door breach. |
| `revive_action` | 8 | |
| `interact_short` | 4 | Buttons, terminals. |
| `interact_long` | 12 | Briefcase, console. |
| `signal_wave` | 4 | Tactical signaling. |
| `signal_point` | 2 | |
| `voice_speak` | 4 (loop) | Mouth movement when speaking per DR-043. |

**Total per chassis:** ~30 animations × 4-12 frames = 120-360 frames per chassis.

## Animation Event Tags

Per-frame metadata that drives sim events, audio, VFX, and replay:

| Tag | Fires | Used For |
|---|---|---|
| `footstep_left` / `footstep_right` | Walk/run animation frame | Footstep SFX, dust trail VFX, audio source for AI hearing |
| `casing_eject` | Fire animation frame | Casing particle VFX, drop physics, SFX |
| `muzzle_flash_anchor` | Fire animation frame | Muzzle flash position; light emission |
| `breath_emit` | Idle animation frame (cold weather) | Breath particle (per [[spec/atmospheric-effects-and-decals]]) |
| `oil_drip` | Idle animation frame (damaged robot) | Oil decal drop |
| `coolant_leak` | Continuous (damaged robot module) | Coolant decal stream |
| `weapon_recoil_apply` | Fire animation frame | Procedural weapon kickback |
| `eject_capsule` | Eject animation frame | Mech eject capsule spawn per DR-021 |
| `limb_detach` | Limb-loss animation final frame | Detached limb spawn (collidable per DR-033) |
| `ragdoll_begin` | Death animation final frame | Switch to physics-driven ragdoll |
| `mouth_phoneme_a/e/i/o/u` | Voice-speak frame | Lip-sync per DR-043 voice synthesis |

Each tag emits a typed `animation.tag_fired` replay event per [[references/prototype-run-bundle-schema]].

## Procedural Overlays

| System | Detail |
|---|---|
| **Recoil** | Per-weapon recoil curve (impulse + damping). Applied to actor's torso bone (skeletal) or torso anchor (sprite). Decays over 0.3-0.8s per weapon. Affects aim accuracy. |
| **Knockback** | Per impulse from `cf-physics`. Applied to actor center; secondary jiggle on bones (skeletal) or sprite scale punch (sprite). Reset to ground on land. |
| **Limb tracking (aim)** | Skeletal: arm + weapon bones rotate to track aim_pitch. Sprite: pose-blend between aim_up/aim_mid/aim_down sprites. |
| **Ragdoll on death** | Skeletal: switch to physics-driven ragdoll (Bevy's Rapier integration via `cf-physics`); bones become rigidbodies with joints. Sprite: transition to gib particles + ragdoll-sprite (single-frame upside-down body). |
| **Weapon-IK to hand socket** | Skeletal: weapon transform parented to hand bone; hand pose adjusts to weapon grip points. Sprite: anchor weapon offset per chassis pose. |
| **Jet flame intensity** | Particle emission rate scaled by jetpack thrust input. |
| **Wound deformation (skeletal hero)** | Bullet hits = small mesh-deform on impact point; fades over 0.5s. Visual feedback for damage zone. |
| **Cape / cloth simulation** | Optional. Per-faction cloak (Ronin scarves, Imperatus capes). Verlet-integration cloth (cheap). |

## Bevy Integration

Crate: `cf-anim`.

| Component | Owns |
|---|---|
| `AnimationStateMachine` | Per-actor; tracks current animation + queue + blend state. |
| `AnimationManifest` | Loaded from RON; defines animations, frame counts, durations, event tags per chassis. |
| `SpriteAnimator` (sprite-sheet) | Steps through frame indices; emits tag events. |
| `SkeletalAnimator` (Spine/DragonBones) | Drives bone transforms; emits tag events. |
| `ProceduralOverlayApplier` | Stacks recoil/knockback/limb-track/etc. on top of base animation. |
| `RagdollComponent` | Marker; transitions actor to physics-ragdoll mode. |

## File Format

```ron
// content/actors/coalition_soldier_light.ron (excerpt)
chassis: (
    id: "coalition_soldier_light",
    sprite_size: (24, 32),
    rig_type: "sprite_sheet",  // or "spine" for hero
    animations: {
        "idle": (frames: 4, duration_s: 1.6, loop: true, tags: [
            (frame: 1, tag: "breath_emit", payload: { faction: "coalition" }),
        ]),
        "walk": (frames: 8, duration_s: 0.8, loop: true, tags: [
            (frame: 2, tag: "footstep_left"),
            (frame: 6, tag: "footstep_right"),
        ]),
        "fire": (frames: 2, duration_s: 0.1, loop: false, tags: [
            (frame: 0, tag: "muzzle_flash_anchor"),
            (frame: 0, tag: "casing_eject"),
            (frame: 0, tag: "weapon_recoil_apply"),
        ]),
        // ... all 30+ animations
    },
    procedural_overlays: ["recoil", "knockback", "limb_track", "ragdoll"],
)
```

## Done-Criteria

- [ ] Every roster actor has a complete animation set defined.
- [ ] Actor locomotion never appears as a static sliding pawn once the milestone owns visible actor presentation.
- [ ] Walk/run/crouch/climb/jet states have animation state changes, event tags, and capture evidence.
- [ ] Jetpack/low-g motion demonstrates controlled secondary limb physics without destroying player aim/control.
- [ ] Knocked/stunned/dead/pressure/wind/explosion states demonstrate increased physics authority with replay events.
- [ ] Limb damage changes animation and capability: limp, one-arm handling, crawl, disabled grip, gear drop, or origin/chassis-specific equivalent.
- [ ] Animation event tags fire correctly on frame.
- [ ] Procedural overlays compose without visual jitter.
- [ ] Ragdoll engages deterministically on death.
- [ ] Skeletal hero chassis bones drive sub-pixel-clean rendering at 4K.
- [ ] AI-generated frames cleaned by Tier 3 pipeline.
- [ ] CI gate: every chassis has all 30+ required animations OR documented exceptions.

## Source Trail

- [[decisions/dr-044-audiovisual-production-pipeline]]
- [[spec/art-and-asset-pipeline]]
- [[spec/full-collision-physics-plan]] for ragdoll integration
- bevy_spine: https://github.com/jabuwu/bevy_spine
- DragonBones: https://github.com/DragonBones/
- Spine Essential: http://esotericsoftware.com/spine-purchase
