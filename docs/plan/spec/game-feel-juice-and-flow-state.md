---
type: spec
status: closed-direction
authority: "Game-feel layer beyond DR-046 button juice. Combat juice (hit-stop, recoil curves, screen shake, bass-thump, blood-spurt, slow-mo, chromatic, camera punch, vibration), animation polish (cancel windows, snap-to-target, weapon-IK, procedural recoil + knockback + limb-tracking + foot-IK), camera punch system (per-event lens shake + zoom snap + dolly + film grain), vibration / haptic patterns, flow state design per Csikszentmihalyi (skill-challenge match, clear goals, immediate feedback, concentration, control, time distortion, autotelic), per-mission difficulty curve (setup/build/push/peak/resolution), per-session pacing, reward cadence (intrinsic only), information overload prevention. CLI testable."
ready_when: "All juice rules trigger correctly per gameplay event; flow state difficulty curve verified in playtest; AI agent drives juice rule audit via cfctl; ACC-A reduce-motion respected per accessibility; modder parity for juice extension."
feeds:
  - DR-012
  - DR-014
  - DR-019
  - DR-020
  - DR-022
  - DR-031
  - DR-046
  - DR-050
  - DR-055
---

← [[spec/index|spec section]] · [[decisions/dr-055-game-feel-juice-and-flow-state|DR-055]] · [[spec/visual-direction|visual direction]] · [[spec/audio-identity|audio identity]] · [[spec/animation-system|animation]] · [[spec/vfx-and-particles|VFX]]

# Game Feel, Juice & Flow State

> [!summary] What this page is
> Comprehensive game-feel layer that makes the game punchy, satisfying, and flow-state-inducing. Beyond DR-046's button juice. Per gameplay event: visual + audio + UI + camera + haptic feedback. Per mission: difficulty curve + reward cadence + information overload prevention. Csikszentmihalyi flow state principles applied to tactical decision games.

## Combat Juice Catalog

| Event | Effect |
|---|---|
| Bullet hit body | Brief screen flash + crosshair pulse + bass thump + camera shake (impulse-scaled) |
| Critical hit / one-shot kill | Time freeze 80ms + flash white + bass thump + camera punch + chromatic aberration |
| Headshot | Above + slow-mo 0.3s + camera dolly + signature ding sound |
| Limb separation | Slow-mo 0.2s + bone-shatter sound + camera shake heavy + per-direction blood arc + dropped-limb collidable spawn |
| Player damage taken | Screen shake (impulse-scaled) + chromatic aberration + red vignette + heartbeat-bass sub-frequency |
| Player low HP | Persistent red vignette pulse + slow-mo on next damage threat + heartbeat sub-bass +20% |
| Player death | Slow-mo 0.3s + camera dolly-in + dramatic spotlight + dim ambient + "show me why" prompt |
| Reload | Magazine swap animation + shell-eject SFX + chamber-click SFX + UI ammo counter punch |
| Reload finish | Chamber-snap sound + crosshair flash + bass thump |
| Weapon recoil | Per-weapon curve (impulse + damping); torso-bone procedural rotation; aim-pitch shift; decay 0.3-0.8s |
| Weapon overheat | Heat-haze + audible sizzle + cool-down indicator |
| Throwing grenade | Pre-throw windup + arc trajectory preview + after-throw camera follow brief |
| Explosion | Multi-stage VFX (flash → fireball → smoke + debris) + heavy camera shake + bass thump + ear-ringing for nearby + concussion-blur + audio duck |
| EMP discharge | Cyan zigzag + screen static brief + electronics flicker + radio interference (cosmetic) |
| Dropship landing | Dropship cinematic 4s + LZ flash + camera follow + landing thump + dust kicked up |
| Bunker breach | Door opens → hot light pours through → silhouettes visible → breach charge sound + camera shake |
| Match start | Dropship cinematic + camera drift + LZ flash + objective banner unfurl |
| Match victory | Comic-page-flip transition + slow-mo final frame + adaptive music swell + confetti VFX (faction-tinted) |
| Match defeat | Scroll-of-failure transition + adaptive music dirge + dim camera + subdued color palette |
| Achievement unlock | Comic-panel pop-in + cheer sting + collection update visible |
| Critical state change (core uprooted, etc.) | Camera punch + flash + signature SFX + UI banner |

## Animation Polish

> [!important] Actor presentation minimum bar
> Corefall actors must feel like bodies, not sliding icons. Controlled movement is animation-first: walk/run/crouch/climb/jet states read clearly, aim/weapon pose blends over locomotion, and recoil/body weight are visible. Disrupted movement is physics-first: knockdown, limb loss, explosions, pressure wind, death, and ragdoll increase physical authority. The blend must stay playable: jetpack/low-g limbs may trail under gravity/inertia, but aim/control limbs remain stable enough to play unless damage says otherwise.

| Element | Detail |
|---|---|
| Controlled locomotion | Walk/run/crouch/climb/jet state changes produce animation tags, foot anchors, body lean, and `cfctl observe actor` pose/stance fields. |
| Animation cancel windows | Per-action cancel windows (e.g., reload can be cancelled at frame 6+ for combat readiness; not earlier) |
| Animation interrupt patterns | High-priority animations (death, eject, limb-loss) interrupt lower-priority cleanly |
| Snap-to-target vs free aim | Per-weapon: snipers slight magnetic snap on ADS; pistols pure free aim. Aim-assist scales per-platform |
| Weapon-IK to hand socket | Skeletal hero chassis: weapon transform parented to hand bone; per-grip-point adjustment |
| Procedural recoil | Per-weapon impulse + damping; torso-bone rotation; aim-pitch shift; chassis-mass-scaled |
| Procedural knockback | Per-impulse from `cf-physics`; actor center pushback + secondary jiggle on bones (skeletal) or sprite scale punch (sprite) |
| Limb tracking (aim) | Skeletal: arm + weapon bones rotate to track aim_pitch. Sprite: pose-blend |
| Foot-IK | Footstep frame anchor; per-surface footstep SFX; foot-on-terrain physics |
| Physics authority transition | Controlled limbs use bounded secondary motion; knocked/stunned/dead/pressure/explosion/limb-detached states raise physics authority and emit replay-visible `physics.authority_changed` / `body.*` / `collision.*` events. |

## Camera Punch System

| Event | Camera effect |
|---|---|
| Hit confirm | Brief 1° rotation + 0.5px zoom |
| Critical hit | 3° rotation + 2px zoom + 0.05s freeze |
| Player damage | Magnitude-scaled 1-5° rotation + 1-3px zoom |
| Critical damage | 5° rotation + 5px zoom + chromatic aberration |
| Death | Slow camera dolly toward target + zoom + dim |
| Match victory | Dolly + zoom out + hold |
| Bunker breach | Sweep across breach point |
| Mission start | Drone-style fly-in to LZ |
| Pause | Subtle zoom + desaturate + ambient duck |

Trauma-based magnitude per [[spec/visual-direction]]. Magnitude scaled by `1.0 - settings.reduce_camera_shake_pct` per DR-012 + DR-051 accessibility.

## Vibration / Haptic Patterns

Per-platform (Steam Input default).

| Event | Pattern |
|---|---|
| Hit confirm | Brief 30ms tick |
| Critical hit | Punchy 60ms thump |
| Damage taken | Magnitude-scaled buzz |
| Reload | Subtle texture during animation |
| Footsteps | Subtle per-step accent |
| Vehicle ride | Continuous engine vibration |
| Weapon overheat | Warning rumble |
| Death | Sharp thump + fade |

Configurable: off / light / medium / heavy.

## Flow State Design (Csikszentmihalyi)

| Principle | Implementation |
|---|---|
| Skill–challenge match | Per-mission difficulty curve; AI difficulty preset (DR-050); adaptive difficulty per session (opt-in) |
| Clear goals | Per-mission objective banner; per-phase objective shift; "show me what to do" hint engine |
| Immediate feedback | Per DR-046 juice rules; every action has visual + audio + UI confirmation |
| Concentration | Information overload prevention; HUD density setting; sensory-overload prevention per DR-051 |
| Sense of control | Per DR-015 strategy-first identity; per DR-046 cfctl parity; player always has agency |
| Loss of self-consciousness | Camera punch + audio duck + slow-mo on critical events removes UI awareness |
| Time distortion | Slow-mo on death + critical hits + flow-state moments |
| Autotelic experience | Mastery rank intrinsic + cosmetic earn paths per DR-031 + DR-049 |

## Per-Mission Difficulty Curve

| Phase | Description |
|---|---|
| Setup (0-15%) | Easy onboarding; player learns mission objective; tutorial-safety policy honored |
| Build (15-40%) | Increasing intensity; small wins; resource management |
| Push (40-60%) | Sustained challenge; key decisions; high-stakes engagement |
| Peak (60-80%) | Climactic encounter; biggest threat; player's mastery tested |
| Resolution (80-100%) | De-escalation; salvage + extract; reflection beat |

## Per-Session Pacing

| Window | Detail |
|---|---|
| First 30 min | Onboarding + first mission; fast progression rewards |
| 30 min - 2 hr | Campaign / lab exploration; mid-pace |
| 2 hr - 10 hr | Procedural contracts + multiplayer; varied pace |
| 10 hr+ | Endgame modes (per DR-048); replay sharing; mastery progression |

## Reward Cadence (Intrinsic Only)

| Cadence | Reward type |
|---|---|
| Every match | Mastery XP + per-mission cosmetic potential |
| Every 5-10 matches | Cosmetic unlock |
| Per-day daily seed | Daily emblem if top-100 |
| Per-week | Tournament placement bonus + community-challenge reward |
| Per-month | Mod-of-the-week recognition + community Q&A |
| Per-season | Ranked-tier reset + cosmetic-only rewards |

## Information Overload Prevention

| Principle | Implementation |
|---|---|
| HUD density setting | low / med / high per DR-046 |
| Captions | Critical-only default; full-subtitle option per DR-051 |
| Hint frequency | high / med / low / off per DR-050 |
| Per-mode minimal-HUD preset | Streamer mode |
| Flash budget | Cap simultaneous flash effects |
| Overlay budget | Cap simultaneous overlay layers |

## CLI Testability

| Command | Purpose |
|---|---|
| `cfctl test game-feel-coverage --scenario X` | Assert all juice rules trigger correctly |
| `cfctl test difficulty-curve --scenario X --difficulty preset` | Assert per-phase difficulty matches design intent |
| `cfctl test flow-state-pacing --duration 30min` | Assert challenge-skill balance maintained |
| `cfctl test reward-cadence --duration 10hr` | Assert reward cadence within target range |
| `cfctl test hint-engine --player-pattern struggling` | Assert hint accuracy >95% |
| `cfctl test camera-shake-budget --intensity high` | Assert per-tier shake within accessibility budget |
| `cfctl test haptic-coverage --device controller` | Assert all events have haptic patterns |

## Done-Criteria

- [ ] All juice rules trigger correctly per gameplay event.
- [ ] Actor presentation is animation-first while controlled, physics-first while disrupted, and never degrades into a static sliding pawn after the milestone owns visible movement.
- [ ] Flow state difficulty curve verified in playtest.
- [ ] AI agent drives juice rule audit via cfctl.
- [ ] ACC-A reduce-motion respected per accessibility.
- [ ] Modder parity for juice extension.
- [ ] Information overload prevention verified.
- [ ] Reward cadence intrinsic only (per DR-031).

## Source Trail

- [[decisions/dr-055-game-feel-juice-and-flow-state]]
- [[decisions/dr-046-player-facing-surfaces-direction]]
- Csikszentmihalyi Flow Theory: https://medium.com/@icodewithben/mihaly-csikszentmihalyis-flow-theory-game-design-ideas-9a06306b0fb8
- Jenova Chen "Flow in Games": https://www.jenovachen.com/flowingames/Flow_in_games_final.pdf
- Game feel survey: https://pure.itu.dk/files/91131028/TG3072241.pdf
