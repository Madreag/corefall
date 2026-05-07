---
type: decision
id: DR-055
status: closed-direction
priority: P0
closed_at: 2026-05-06
revisit_trigger: "Game feel polish overrun budget; flow state difficulty curve fails playtest cohort; juice rules counter-readability per DR-019; adaptive difficulty regresses fairness."
---

← [[decisions/index|decision records]] · [[dashboards/decision-tracker|decision tracker]] · [[spec/game-feel-juice-and-flow-state|game feel spec]] · [[decisions/dr-014-tone-player-promise|DR-014]] · [[decisions/dr-019-visual-direction|DR-019]] · [[decisions/dr-020-audio-identity|DR-020]] · [[decisions/dr-022-ai-humanlike-bar|DR-022]] · [[decisions/dr-046-player-facing-surfaces-direction|DR-046]] · [[decisions/dr-050-modding-social-onboarding-and-ai-extensions|DR-050]]

# DR-055: Game Feel, Juice & Flow State Design

> [!success] Status: CLOSED-DIRECTION (project owner committed 2026-05-06)
> Comprehensive game-feel layer beyond DR-046 button juice. **Combat juice**: hit-stop / time-freeze on critical events; per-weapon recoil curves; per-weapon kickback animations; muzzle-flash + tracer + casing-eject + screen flash + chromatic aberration; impulse-based screen shake; bass-thump sub-frequency; impact-spark + smoke-poof + blood-spurt. **Flow state design** per Csikszentmihalyi: per-mission challenge-skill matched difficulty curve; per-session pacing tension/relief/tension; reward cadence; information overload prevention; per-player adaptive difficulty (extends DR-050 onboarding). **Animation polish**: animation cancel / interrupt patterns; snap-to-target vs free aim; aim-assist tuning; weapon-IK to hand sockets. **Camera punch**: per-event lens shake + zoom snap + dolly + film grain pulse. **Vibration patterns**: per-device per-event haptic feedback. All testable via cfctl + playtest cohort.

## Decision

### Combat juice (per DR-046 + DR-019 + DR-020)

| Event | Effect |
|---|---|
| **Bullet hit body** | Brief screen flash + crosshair pulse + bass thump + camera shake (magnitude × impulse) |
| **Critical hit / one-shot kill** | Time freeze 80ms + flash white + bass thump + camera punch toward target + chromatic aberration brief |
| **Headshot** | Above + slow-mo 0.3s + camera dolly toward kill + sound cue (signature high-pitched ding) |
| **Limb separation** | Slow-mo 0.2s + bone-shatter sound + camera shake heavy + per-direction blood arc + dropped-limb collidable spawn |
| **Player damage taken** | Screen shake (impulse-scaled) + chromatic aberration brief + red vignette + heartbeat-bass sub-frequency |
| **Player low HP** | Persistent red vignette pulse + slow-mo on next damage threat + heartbeat sub-bass +20% volume |
| **Player death** | Slow-mo 0.3s + camera dolly-in + dramatic spotlight on body + dim ambient + "show me why" prompt |
| **Reload** | Magazine swap animation + shell-eject SFX + chamber-click SFX + UI ammo counter punch |
| **Reload finish** | Chamber-snap sound + crosshair flash + bass thump |
| **Weapon recoil** | Per-weapon recoil curve (impulse + damping); torso-bone procedural rotation; aim-pitch shift; decay 0.3-0.8s |
| **Weapon overheat** | Heat-haze around weapon + audible sizzle + cool-down indicator |
| **Throwing grenade** | Pre-throw windup + arc trajectory preview + after-throw camera follow brief |
| **Explosion** | Multi-stage VFX (flash → fireball → smoke + debris) + heavy camera shake + bass thump + ear-ringing for nearby actor + concussion-blur + audio duck |
| **EMP discharge** | Cyan zigzag + screen static brief + electronics flicker + radio interference + cosmetic only |
| **Dropship landing** | Dropship cinematic 4s + LZ flash + camera follow + landing thump + dust kicked up |
| **Bunker breach** | Door opens → hot light pours through → silhouettes visible → breach charge sound + camera shake |
| **Match start** | Dropship cinematic 4s + camera drift + LZ flash + objective banner unfurl |
| **Match victory** | Comic-page-flip transition + slow-mo final frame + adaptive music swell + confetti VFX (faction-tinted) |
| **Match defeat** | Scroll-of-failure transition + adaptive music dirge + dim camera + subdued color palette |
| **Achievement unlock** | Comic-panel pop-in from corner + cheer sting + collection update visible |
| **Critical state change (core uprooted, etc.)** | Dramatic camera punch + flash + signature SFX + UI banner |

### Animation polish

| Element | Detail |
|---|---|
| **Animation cancel** | Per-action cancel windows (e.g., reload can be cancelled at frame 6+ for combat readiness; not earlier). Per-weapon configurable. |
| **Animation interrupt patterns** | High-priority animations (death, eject, limb-loss) interrupt lower-priority animations cleanly. |
| **Snap-to-target vs free aim** | Per-weapon: snipers have slight magnetic snap on aim-down-sight; pistols are pure free aim. Aim-assist scales per-platform (controller higher; KB/M minimal). |
| **Weapon-IK to hand socket** | Skeletal hero chassis: weapon transform parented to hand bone; per-grip-point adjustment per weapon spec. |
| **Procedural recoil** | Per-weapon impulse + damping; torso-bone rotation; aim-pitch shift; chassis-mass-scaled. |
| **Procedural knockback** | Per-impulse from `cf-physics`; actor center pushback + secondary jiggle on bones (skeletal) or sprite scale punch (sprite). |
| **Limb tracking (aim)** | Skeletal: arm + weapon bones rotate to track aim_pitch. Sprite: pose-blend between aim_up/aim_mid/aim_down. |
| **Foot-IK** | Footstep frame anchor; per-surface footstep SFX; foot-on-terrain physics. |

### Camera punch system

| Event | Camera effect |
|---|---|
| Hit confirm | Brief 1° rotation + 0.5px zoom |
| Critical hit | 3° rotation + 2px zoom + 0.05s freeze |
| Player damage | Magnitude-scaled rotation (1-5°) + 1-3px zoom |
| Critical damage | 5° rotation + 5px zoom + chromatic aberration |
| Death | Slow camera dolly toward target + zoom + dim |
| Match victory | Dolly + zoom out + hold |
| Bunker breach | Sweep across breach point |
| Mission start | Drone-style fly-in to LZ |
| Pause | Subtle zoom + desaturate + ambient duck |

Per-camera shake: trauma-based (per DR-019 reduce-motion accessibility setting respected). Magnitude scaled by `1.0 - settings.reduce_camera_shake_pct`.

### Vibration / haptic patterns

Per-platform (Steam Input default, controller-detected):

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

Configurable in settings (off / light / medium / heavy / off).

### Flow state design (per Csikszentmihalyi)

| Principle | Implementation |
|---|---|
| **Skill–challenge match** | Per-mission difficulty curve; AI difficulty preset (per DR-050); adaptive difficulty per session (opt-in). |
| **Clear goals** | Per-mission objective banner; per-phase objective shift; "show me what to do" hint engine. |
| **Immediate feedback** | Per DR-046 juice rules; every action has visual + audio + UI confirmation. |
| **Concentration** | Information overload prevention; HUD density setting; sensory-overload prevention per DR-051. |
| **Sense of control** | Per DR-015 strategy-first identity; per DR-046 cfctl parity; player always has agency. |
| **Loss of self-consciousness** | Camera punch + audio duck + slow-mo on critical events removes UI awareness. |
| **Time distortion** | Slow-mo on death + critical hits + flow-state moments. |
| **Autotelic experience** | Mastery rank intrinsic + cosmetic earn paths per DR-031 + DR-049. |

### Per-mission difficulty curve

Per [[spec/missions-and-objectives]] + DR-050 adaptive difficulty:

| Phase | Description |
|---|---|
| **Setup (0-15%)** | Easy onboarding; player learns mission objective; tutorial-safety policy honored. |
| **Build (15-40%)** | Increasing intensity; small wins; resource management. |
| **Push (40-60%)** | Sustained challenge; key decisions; high-stakes engagement. |
| **Peak (60-80%)** | Climactic encounter; biggest threat; player's mastery tested. |
| **Resolution (80-100%)** | De-escalation; salvage + extract; reflection beat. |

Adaptive difficulty (opt-in per DR-050): real-time difficulty adjusts if player struggling (more hints, lower enemy aggression, slower time scale) OR thriving (more hints disabled, higher enemy aggression).

### Per-session pacing

| Window | Detail |
|---|---|
| First 30 min | Onboarding + first mission; fast progression rewards. |
| 30 min - 2 hr | Campaign / lab exploration; mid-pace. |
| 2 hr - 10 hr | Procedural contracts + multiplayer; varied pace. |
| 10 hr+ | Endgame modes (per DR-048); replay sharing; mastery progression. |

### Reward cadence (intrinsic only per DR-031)

| Cadence | Reward type |
|---|---|
| Every match | Mastery XP + per-mission cosmetic potential |
| Every 5-10 matches | Cosmetic unlock (variant, paint, decal) |
| Per-day daily seed | Daily emblem if top-100 |
| Per-week | Tournament placement bonus + community-challenge reward |
| Per-month | Mod-of-the-week recognition + community Q&A |
| Per-season | Ranked-tier reset + cosmetic-only rewards |

NEVER paid-power. NEVER FOMO-required.

### Information overload prevention

| Principle | Implementation |
|---|---|
| HUD density setting (per DR-046) | low / med / high |
| Captions for critical audio only (default); full-subtitle option | Per DR-051 accessibility-plus |
| Hint frequency setting | high / med / low / off |
| Per-mode minimal-HUD preset | Streamer mode hides enemy positions |
| Flash budget | Cap simultaneous flash effects |
| Overlay budget | Cap simultaneous overlay layers |

### CLI testability

Per DR-052 + T-CONTROL.

| Command | Purpose |
|---|---|
| `cfctl test game-feel-coverage --scenario X` | Assert all juice rules trigger correctly per gameplay event. |
| `cfctl test difficulty-curve --scenario X --difficulty preset` | Assert per-phase difficulty matches design intent. |
| `cfctl test flow-state-pacing --duration 30min` | Assert challenge-skill balance maintained. |
| `cfctl test reward-cadence --duration 10hr` | Assert reward cadence within target range. |
| `cfctl test hint-engine --player-pattern struggling` | Assert hint accuracy >95%. |
| `cfctl test camera-shake-budget --intensity high` | Assert per-tier shake within accessibility budget. |
| `cfctl test haptic-coverage --device controller` | Assert all events have haptic patterns. |

## What This Locks In

| Spec Area | Implication |
|---|---|
| `cf-game-feel` | Master orchestrator for juice rules + camera punch + haptic + flow state. |
| `cf-camera-punch` | Per-event camera shake / dolly / zoom system. |
| `cf-haptic` | Per-platform haptic pattern dispatcher. |
| `cf-difficulty-curve` | Per-mission phase tracker + adaptive difficulty kernel. |
| `cf-hint-engine` | Per [[spec/tutorial-implementation]] adaptive hints. |
| `cf-flow-state` | Per-session pacing engine + reward cadence tracker. |
| Per-milestone done-criteria | Updated to include juice rule audit. |

## What This Does NOT Lock

| Non-Commitment | Why |
|---|---|
| Specific juice tuning numbers | Open. Per-event magnitudes tuned in playtest. |
| Adaptive difficulty algorithm | Open. Default heuristic-based; possibly ML post-launch. |
| Per-controller haptic patterns | Open. Default Steam Input + per-controller mapping. |
| Console-specific juice | Open. Per DR-051 platform extensions; per-platform polish post-launch. |

## Why This Direction

| Driver | Detail |
|---|---|
| Genre fit | Tactical pulp sci-fi disaster sandbox (DR-014) demands punchy combat feel. |
| Player retention | Game feel directly correlates with retention (per Helldivers, Vampire Survivors precedents). |
| Accessibility-aware | Reduce-motion / camera-shake / flash settings respect DR-012 + DR-051. |
| Flow state evidence | Csikszentmihalyi's flow theory directly applicable to tactical decision games (per Jenova Chen's "Flow in Games" thesis). |
| AI-augmented tuning | AI agents drive juice-rule audits + difficulty-curve verification + flow-state pacing tests via cfctl. |

## Why Not The Alternatives

| Alternative | Why Rejected |
|---|---|
| No game-feel polish | Loses retention; tactical depth alone insufficient. |
| Static juice rules (no adaptive difficulty) | Per DR-050 onboarding-plus requires adaptive difficulty for new players. |
| Pay-for-power difficulty bypass | Forbidden by DR-031. |
| Fixed difficulty (no presets) | Per DR-050 named presets; player should feel intentional. |

## Evidence Trail

- Project owner verbatim (2026-05-06): "I want the best game possible with all the features!"
- Mihaly Csikszentmihalyi's Flow theory: https://medium.com/@icodewithben/mihaly-csikszentmihalyis-flow-theory-game-design-ideas-9a06306b0fb8
- Jenova Chen "Flow in Games" thesis: https://www.jenovachen.com/flowingames/Flow_in_games_final.pdf
- Game feel survey: https://pure.itu.dk/files/91131028/TG3072241.pdf
- Hit-stop / juice patterns: https://www.reddit.com/r/pcgaming/comments/1jd737y/why_does_hitting_things_in_some_games_feel_like_so_much_better_than/
- Captured in [[research-log/2026-05-06-third-pass-audit-followup]] (TBD).

## Revisit Trigger

- Game feel polish overrun budget.
- Flow state difficulty curve fails playtest cohort.
- Juice rules counter-readability per DR-019.
- Adaptive difficulty regresses fairness.
