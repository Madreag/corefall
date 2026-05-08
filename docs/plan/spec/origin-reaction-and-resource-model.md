---
type: spec
status: design-intent-post-m1
authority: "Canonical contract for origin-specific shot reactions, damage branches, healing affordances, and resource systems. Captured as design intent during M1; implementation lands at M5/M5.5/M5.7 and a new origin resource milestone, never earlier."
ready_when: "M5 (chassis model), M5.5 (impulse-to-damage routing), M5.7 (affliction layer + HUD overlays) and the M5.8 origin resource pass have all promoted human/android/robot branches with replay-visible reason chains, and ORIGIN-A acceptance tests pass."
feeds:
  - DR-002
  - DR-003
  - DR-004
  - DR-008
  - DR-009
  - DR-011
  - DR-012
  - DR-014
  - DR-018
  - DR-027
  - DR-033
  - DR-036
---

← [[index|vault home]] · [[spec/index|spec section]] · [[spec/chassis-armor-mechs-and-origins|chassis/armor/mechs/origins]] · [[spec/body-damage-model|body damage model]] · [[spec/atmospherics-and-chemistry-model|atmospherics/chemistry]] · [[spec/gravity-and-ballistics-model|gravity/ballistics]] · [[spec/celestial-bodies-and-worlds-model|worlds catalog]] · [[spec/environmental-conditions-model|environmental conditions]] · [[spec/comms-voice-and-radio-model|comms/voice/radio]] · [[spec/full-collision-physics-plan|full collision plan]] · [[spec/equipment-loadout|equipment/loadout]] · [[spec/prototype-roadmap|native roadmap]] · [[spec/native-implementation-backlog|native backlog]] · [[decisions/dr-003-body-damage-readability|DR-003]] · [[decisions/dr-014-tone-player-promise|DR-014]] · [[decisions/dr-033-full-collision-physics-direction|DR-033]] · [[decisions/dr-036-systemic-material-simulation-direction|DR-036]] · [[decisions/dr-037-stationeers-grade-atmospherics-direction|DR-037]] · [[decisions/dr-038-universal-gravity-and-ballistics-direction|DR-038]] · [[decisions/dr-040-environmental-conditions-and-hazards-direction|DR-040]] · [[decisions/dr-043-voice-comms-and-radio-direction|DR-043]]

# Origin Reaction And Resource Model

> [!summary] What this page is
> The contract that says "a rifle shot does not feel the same in a human, an android, and a robot — and a hungry human, a battery-empty android, and an overclocked robot do not behave the same either". This page locks the per-origin branches that the chassis layer ([[spec/chassis-armor-mechs-and-origins]]), the body damage layer ([[spec/body-damage-model]]), the impulse-to-damage routing in M5.5, and the affliction layer in M5.7 must implement. It also defines the new origin-specific resource model (caloric energy / battery / power / overclock thermal) that is currently missing from the roadmap and that lands in a new M5.8 task slice.

> [!warning] Authority boundary
> Captured 2026-05-06 as **design intent**, not as a final settled value table. The grammar (which branches exist, which events fire, which HUD surfaces light up, which resources tick) is a commitment. The numbers (impulse thresholds, blackout curves, battery drain rates, thermal slopes) stay open until M5/M5.5/M5.7 prototype evidence backs them.

> [!important] Out of scope right now
> BP1 is closed and BP2 is active. **Nothing on this page is implemented in M0, M1, M1.5, or BP2.** Any worker reading this before M5 must NOT add origin-specific branches to actor / damage / resource code. The first implementation surface is M5 (chassis-bound origin field), M5.5 (origin-aware impulse routing), and M5.8 (origin reaction/resource runtime). Earlier milestones may carry placeholder origin tags on actors only if the actor record already needs an `origin_id` field for save-roundtrip; behavior must remain identity-equivalent across origins until M5.

## Why This Page Exists

The existing spec already names origins ([[spec/chassis-armor-mechs-and-origins#Origins / Races]]) and damage stages ([[spec/body-damage-model#Status Vocabulary]]), but neither page locks down what changes between origins when a shot lands, when a fall happens, or when a body part takes too much damage too fast. Without that contract:

- The HUD will get a single damage feedback model that lies for two of three origins.
- Impulse-to-damage routing in M5.5 will collapse into "humans with re-skins" and miss the robot-specific internal-shock branch.
- The affliction layer in M5.7 will hard-code `concussed` against all origins and produce a robot blackout effect that has no in-fiction reason.
- The retention loop and combat economy will lack origin-specific resource pressure (food / battery / power), which was a stated promise of the chassis-and-origins direction in [[decisions/dr-027-combat-base-scope]] and the chassis tone of [[decisions/dr-014-tone-player-promise]].

This page is the single contract. Other pages cross-link here instead of restating it.

## User-Captured Design Intent (verbatim 2026-05-06)

> When heavy armor or mechs etc... when getting shot, the user will feel the force of the shot not only via physics, but depending on their race they will have different reactions. Also different things like humans and android can eat food.
>
> **Humans** are susceptible to G-Force, and getting concussed when taking too much damage too fast. They take the most fall damage, limbs can break, bleed etc... Humans can eat food, use medical stuff, buff drugs, etc... Humans have normal caloric energy.
>
> **Robot** are not susceptible to G-Force, and getting concussed when taking too much damage too fast. Instead they can take internal damage from the shock. Internal damage to different modules in their body. Also they can break by falling hard, but not as easily as humans. Their internal models takes the shock. Robots can leak coolant/oil if shot by penetrating rounds or if a body part is heavily damage (about to break off). Robots can't eat food or heal using medical stuff or buff via a drug... they can instead overclock their processor enhancing their ability, but slowly heats components (too much heat will cause damage over time)... also consumes more power (robot resource to act etc...)
>
> **Androids** are in between humans and robots... just as susceptible to physics, bleeding etc... G-Force doesn't affect them as much. Can eat, use medical stuff, buff drugs, etc... they can also overclock depending on the android they are. Androids are part human so they can only overclock certain modules they have installed. Some androids also have batteries to power their modules, and when they run out of energy they are slower and lose abilities, etc...
>
> **G-Force simulation** screen starts slowly blacking out the more concussion damage is taken, etc...

### Round 2 — Environment Resistance (verbatim 2026-05-06)

> Robots also have resistance to environment... ie if we are playing on a map with no oxygen, vacuum only, humans need to bring helmet and oxygen tank which is consumed over time... robots don't need that. Androids need it as well. Robots are more resistant to heat, but it can overheat components downclocking them. Androids are in between, where depending on the type of android certain modules can be affected...

These two blocks are the source of truth for the matrices below. If a future agent reads only one block on this page, read these two.

## Origin Reaction Matrix

This is the core contract. Every column is a branch that M5.5 / M5.7 / M5.8 code paths must implement (or explicitly opt out of with a recorded reason).

| Branch | Human | Android | Robot |
|---|---|---|---|
| **Shot force feedback (camera/HUD)** | Full physical impulse + soft-tissue jolt overlay (recoil-direction screen kick, pain flash). | Full physical impulse + soft-tissue jolt overlay (slightly softer pain flash; android nervous system is partial). | Full physical impulse + servo-jolt overlay (mechanical clank, frame ring, no pain flash). |
| **G-Force susceptibility** | High. Sustained high-G or rapid-onset acceleration accumulates `g_load_dose`. | Reduced. G-load accumulates at ~30-50% of human rate (open value, prototype-driven). | None. Robots ignore G-load entirely. (They take internal-shock damage from the same impulse instead — see next row.) |
| **Concussion (rapid damage stacking)** | `concussed` affliction stacks on rapid burst damage (multiple impacts within a window) or G-load spike. Drives screen-blackout HUD. | `concussed` stacks at reduced rate; partial blackout. | NEVER. Robots do not get `concussed`. Same input event routes to `internal_shock` instead. |
| **Internal-shock damage (robot-only)** | n/a. | n/a. (Or partial — see Open Questions for "android partial internal shock".) | Each impact above an impulse threshold rolls damage onto a random un-armored internal module (sensor / actuator / power-bus / processor). Module damage is independent of armor health and accumulates even when no plate is breached. |
| **Fall damage** | Highest tolerance ceiling lowest; legs break, spine wounds, severe bleed risk. | Mid. Susceptible to physics like humans (limbs can break, can bleed) but slightly tougher frame. | Low rate of bone-equivalent damage, but a hard fall damages internal modules via internal-shock branch. Robot frame absorbs the shock instead of transmitting it. |
| **Limb wounds (skeletal/soft-tissue)** | Yes. Limbs can break, can be detached, can bleed. Per [[spec/body-damage-model#Body Part Contract]]. | Yes. Same wound grammar as humans (synthetic bio-skin + partial endoskeleton). Bleeds. | No. Robots have armor + module damage instead of limb wounds. A "limb-equivalent" failure is `module-failed: actuator.left_arm`. |
| **Bleeding** | Yes. Per [[spec/body-damage-model#Wound Contract]] `bleed_rate`. | Yes. Reduced bleed rate; android blood is engineered (slower coag profile but lower volume). | No (blood). YES (coolant/oil leak). See "Leak channels" row. |
| **Leak channels** | Blood (per wound). | Blood (reduced rate). | **Coolant** (penetrating rounds OR heavily-damaged body part about to detach). **Oil** (joint/actuator damage). Both are visible particle/material emissions; both feed the M5.6 material kernel; both can ignite under fire/electricity material reactions. |
| **Healing affordances** | Food (caloric replenishment), medical kits (wound treatment, bleed stop), drugs (buff/debuff, pain suppression). All consumable, all per [[spec/body-damage-model]] treatment hooks. | Food, medical kits, drugs (same as humans). Subject to android-installed-module compatibility (some buff drugs need synth-blood compatibility; some don't). | NEVER eats food. NEVER uses medkits. NEVER takes drugs. Repaired via repair tools / coolant refills / oil refills / module swaps. |
| **Buffs (active enhancement)** | Drugs only. Combat stims, pain suppressants, focus drugs, etc. | Drugs (organic side) AND limited overclock (synthetic side, gated by installed modules). | Overclock only. No drug compatibility. |
| **Overclock subsystem** | n/a. | Per-module overclock, gated by module type (`processor`, `actuator`, `sensor`, `weapon-mount`). Only modules the android has installed can be overclocked. | Whole-processor overclock affecting movement / aim / reload / sensor speed. Deeper boost ceiling than android. |
| **Overclock cost** | n/a. | Power draw (battery models) + per-module heat. Heat over threshold damages the module. | Power draw (always) + global heat. Sustained heat damages internal modules over time. |
| **Resource model** | `caloric_energy` (food-fed, depletes via action and over time). Hunger affliction below thresholds. | Hybrid: `caloric_energy` (organic side) + optional `battery_charge` (synthetic side, only on android variants that ship with batteries). When battery empties, modules go slow / lose abilities; basic mobility from organic side persists. | `power` (robot-only resource that gates *every* action — move, aim, fire, observe). Recharged via base power, generators, or salvage. No power = inert. |
| **Resource depletion penalty** | Slowdown, aim wobble, vision blur, eventually `weak`/`exhausted` afflictions. | Organic side: same as humans. Synthetic side (battery): slowdown, ability lockout, eventually module shutdown. | Power below threshold: action cost rejection (cannot fire / cannot move at full speed) → eventually full inert state. |
| **Death readability** | Recap names cause chain (hit → wound → bleed/concussion/G-load → status). | Recap names cause chain on the active side (organic side or synthetic side, whichever failed first). | Recap names cause chain (hit → module failure → coolant leak → fire reaction OR power depletion OR catastrophic frame). |

Every cell is a contract row. If a milestone-implementing agent finds they cannot back a row with a code path or a recorded reason for skipping it, that is a milestone failure, not an "edge case".

## Environment Resistance Matrix

The combat-reaction matrix above answers "what happens when something hits the actor". This matrix answers "what happens when the actor is in a hostile environment" — vacuum maps, low-oxygen pockets, hot-zone scenarios (lava-adjacent, foundry, fire propagation), cold/cryo zones, irradiated areas. Environment is a per-tick *exposure* signal, not a discrete event.

| Branch | Human | Android | Robot |
|---|---|---|---|
| **Vacuum / no-oxygen ambient** | MUST wear sealed helmet + oxygen tank to survive. Tank is a consumable equipment item that drains `oxygen_supply` over time while in vacuum. Without seal/tank: `hypoxia` affliction stacks → unconsciousness → death. | Same as humans. Sealed helmet + oxygen tank required; tank drains the same `oxygen_supply` resource. (Some android variants may ship with built-in seal/reserve — open question; default is "needs the same gear as humans".) | Immune. No oxygen consumption, no `hypoxia` affliction, no helmet/tank requirement. Robot frame is sealed by design. |
| **Low-oxygen but non-zero ambient** | Helmet/tank not strictly required, but `oxygen_supply` drains at reduced rate; `hypoxia` stacks slowly. | Same as humans. | Immune. |
| **High-temperature ambient (foundry / lava-adjacent / fire propagation)** | `burning` affliction risk on direct flame contact (existing M5.7); ambient heat causes `heat_exhaustion` affliction stack (caloric drain + slowdown). High vulnerability. | Per-module heat ring rises depending on android type — combat-spec androids have shielded modules, civilian-spec androids don't. Affected modules can `overheat` and lock out per the existing overclock/heat path; organic side takes `heat_exhaustion`. Mid vulnerability; module-specific. | Most resistant to heat. Global `heat` accumulates from ambient exposure on top of overclock/fire/friction. When `heat` crosses the throttle band, robot **downclocks** involuntarily — opposite of overclock — slowing aim/move/reload until heat drops. At critical, modules take overheat damage per the existing path. |
| **Cold / cryo ambient** | Slowdown + caloric drain spike; eventual `frostbite` affliction (deferred — not part of this contract; flagged for future). | Organic side: same as humans. Synthetic side: viscosity penalty on actuators (slower movement) but no module damage at typical cold zones. | Reduced viscosity in joints; minor slowdown. No frostbite analog. (Cold can also DELAY overheat damage by accelerating cooling — design opportunity.) |
| **Irradiated ambient** | `irradiated` affliction stacks (deferred — not part of this contract; flagged for future). | Organic side: same as humans. Synthetic side: sensor/processor noise (degraded perception) at high doses. | Sensor/processor noise; logic faults at very high doses. No organic radiation injury. |
| **Hostile material exposure (acid pool / toxic gas / electrified water)** | Per [[spec/native-implementation-backlog#M5.7 — Hazard Package]] — `corroded`, `poisoned`, `electrified` afflictions. | Same as humans for organic afflictions; synthetic side adds module-level corrosion at sustained acid contact. | No organic afflictions. Acid corrodes plates/modules at material-defined rates; toxic gas is mostly inert (some gases attack circuits — data-driven); electrified water arcs into chassis modules. |

Environment resistance is checked **per tick on actor's current cell/zone**, not per discrete event. The exposure signal is computed by the M5.6 material kernel + the M7.5 atmospherics layer, and routed into actor state by the M5.8 origin resource pass. That keeps the contract data-driven (mod authors can add new environments) without spreading per-environment branches across actor code.

### Helmet + Oxygen Tank — Equipment Contract

Per [[spec/equipment-loadout]] role records, `helmet` and `oxygen_tank` are equipment items with these origin-relevant fields:

| Field | Used By |
|---|---|
| `provides_seal: bool` | Helmets that seal the head against vacuum/toxic gas. |
| `oxygen_capacity_seconds: u32` | Oxygen tanks' reserve time at standard consumption rate. |
| `consumption_modifier_running: f32` | Faster drain while sprinting/digging (humans/androids). |
| `consumption_modifier_combat: f32` | Faster drain under stress (humans/androids; android rate is reduced). |
| `origin_compatibility: [origin_id]` | Robots cannot equip oxygen tanks (they get rejected at slot-assign with `wrong_origin_for_equipment`); helmets they CAN wear (cosmetic + visor optics) but tanks they cannot. |
| `breakable: bool, breach_event` | Penetrating round to a sealed helmet emits `helmet_breach`; oxygen drains at multiplied rate; eventually `hypoxia` stacks. |

This is the equipment-side view. The M5.8 origin resource pass owns the actor-side `oxygen_supply` accumulator and the `hypoxia` affliction wiring.

### Heat Tolerance — Robot Downclock vs Overclock

The robot has TWO heat-driven state paths and they must not be confused in code or in the HUD:

| Path | Direction | Trigger | Player Visible |
|---|---|---|---|
| **Overclock** | Voluntary boost. | Player or AI requests overclock tier. | "Boosting" pip; heat ring rising fast; player chose this. |
| **Downclock** | Involuntary throttle. | `heat` crosses throttle band from passive sources (ambient, sustained fire, friction) without active overclock. | "Throttling" pip; reduced action speeds; player did NOT choose this. |

Both share the `heat` resource accumulator and the `chassis_module_damaged{cause=overheat}` damage path at critical heat. They differ in:

- The state-change events (`chassis_overclock_*` vs `chassis_thermal_throttle_*`).
- The HUD chip shown to the player.
- The AI doctrine response (an AI bot under involuntary downclock should retreat from heat source; an AI bot that engaged overclock voluntarily should drop overclock first).

Androids inherit a per-module version of downclock — under sustained heat, individual modules can throttle without the rest of the body throttling. The `chassis_thermal_throttle_*` events use the same `scope: global | module:<id>` field as the overclock events.

## Force-Feedback Contract

Every chassis-bearing actor that takes a hit MUST emit `body_force_feedback` regardless of origin. Origin only determines the *content* of the feedback, not whether it fires.

| Field | Required | Origin-Specific |
|---|---|---|
| `event_id`, `parent_hit_event_id`, `tick`, `actor_id` | yes | no |
| `impulse_vector`, `impulse_magnitude` | yes | no |
| `origin_id` | yes | yes (`human` / `android` / `robot` / origin variant) |
| `chassis_layer` (`armor_plate` / `frame` / `pilot`) | yes | no |
| `feedback_kind` | yes | yes (`pain_jolt` / `servo_jolt` / `frame_ring`) |
| `g_load_delta` | yes (always emit, even if 0) | yes (humans accumulate; androids accumulate at reduced rate; robots emit 0 by design) |
| `internal_shock_module_id`, `internal_shock_damage` | conditional | robots only — emit when impulse passes threshold |
| `concussion_dose_delta` | yes (always emit, even if 0) | humans full; androids reduced; robots 0 by design |
| `leak_channel`, `leak_rate` | conditional | robots: coolant/oil; humans/androids: blood |
| `screen_kick_intensity` | yes | yes (servo kick is sharper but shorter than pain jolt) |

The reason an `_delta` field is `0` instead of absent is so the replay viewer and run-bundle checker can both confirm "the engine did not forget to compute this; it computed 0 because origin says so". Absent fields are bugs.

## G-Force Vision Blackout (HUD Effect)

The user's "screen starts slowly blacking out the more concussion damage is taken" line is part of the accessibility surface, not a free-floating shader effect.

| Aspect | Contract |
|---|---|
| Trigger | `concussion_dose` accumulator on the player-controlled actor crosses banded thresholds (`mild`, `moderate`, `severe`, `out`). |
| Visual | Vignette darkens from the edges inward in proportion to dose. At `severe`, peripheral vision is gone. At `out`, full blackout for a fixed duration; player loses direct control until recovery. |
| Audio | Heart-rate sound layer mixes louder; ambient duck. Captions on for accessibility. |
| Origin gate | Only humans get the full blackout curve. Androids get a reduced curve (e.g. capped at `moderate`). Robots NEVER blacken from this; their concussion dose is structurally 0. |
| Accessibility floors | Per [[decisions/dr-012-accessibility-comfort-readability]] and [[spec/accessibility-comfort-slice-a]]: the blackout effect MUST be reducible/disable-able by `--reduced-motion` and the new `--reduced-g-force-blackout` toggle. There MUST be a non-visual fallback (caption + HUD icon) for players who disable the visual. |
| Replay | Concussion-dose changes emit `affliction.concussed_set` / `affliction.concussed_intensified` / `affliction.concussed_cleared`. Blackout rendering is reproducible from the event stream. |
| Recovery | Dose decays over time; medical treatment (humans / androids) can accelerate recovery. Robots have no analog (their internal-shock damage is treated differently). |

## Robot-Specific: Internal Shock + Leaks + Overclock

### Internal Shock

When a robot takes a kinetic / blast impact above an impulse threshold (per-class, data-driven), the M5.5 impulse-to-damage router MUST roll internal-shock damage onto a random module weighted by:

- Hit zone proximity to the module's mount point.
- Whether the relevant armor plate is `armor-cracked` or breached.
- Module's own `shock_resistance` field (data-driven).

Internal-shock damage is independent of armor HP. A robot with intact armor can still lose a sensor module to repeated heavy impacts, and the replay must say so.

Events: `chassis_module_damaged` with `cause = internal_shock`, `parent_hit_event_id = <hit>`, `module_id`, `damage_amount`. Already part of the M5.5 task card, this page locks the `cause` enum extension.

### Leak Channels

Penetrating rounds OR a body part transitioning to `armor-cracked` / `module-failed: structural` triggers a leak event:

- `coolant` — emitted from heat-dissipation lines.
- `oil` — emitted from joint actuators.

Leaks emit physical particles into the M5.6 material kernel (one of the launch 17 materials from DR-036). Leaks can:

- Pool on the ground (M5.6 material kernel handles density layering).
- Ignite via fire / electricity reactions (oil) and steam-flash (coolant + heat).
- Reduce the robot's `power` resource over time (coolant loss → forced thermal throttle → power drain to compensate).
- Mark a salvage trail visible to AI doctrine.

Events: `chassis_leak_started` / `chassis_leak_rate_changed` / `chassis_leak_stopped`, payload includes channel, rate, source module/zone, parent hit/damage event.

### Overclock

`overclock` is a chassis state that boosts target subsystems at the cost of `power` and `heat`.

| Field | Robot | Android |
|---|---|---|
| Scope | Whole processor (single global tier). | Per-module, gated by the module being installed. |
| Boost | Aim speed, movement speed, reload speed, sensor refresh — all scale up by tier multiplier. | Same axes, but only for modules the android has installed; un-installed module overclock requests are rejected with reason `module_not_installed`. |
| Power cost | Continuous drain on `power` resource while active. | Continuous drain on `battery_charge` (if android has battery) AND on organic `caloric_energy`. |
| Heat cost | Global `heat` accumulator rises faster than passive cool rate. | Per-module `heat` accumulator rises. |
| Damage path | When `heat` crosses critical threshold, modules take damage-over-time until heat drops below safe band OR the overclock is dropped. | Same, but per-module. A single overclocked module overheating doesn't damage the others. |
| Player UX | HUD overclock pip with heat ring and power drain rate. Audible whine. | Same per-module, with per-module heat rings on the body silhouette. |
| Replay | `chassis_overclock_started/tier_changed/stopped`, `chassis_heat_*`, `chassis_module_damaged{cause=overheat}`. | Same family, scoped to module. |

Robots and androids share `chassis_overclock_*` event names; payload distinguishes scope (`global` vs `module:<id>`).

## Resource Model Extension

The existing chassis spec implies resources but doesn't lock them. This page locks them.

| Resource | Origin | Source | Sink | Penalty Below Threshold | Replay Events |
|---|---|---|---|---|---|
| `caloric_energy` | Human, Android (organic side) | Food consumption (item interaction), passive recovery while resting. | All actions cost calories at low rate; injuries spike cost; heavy lifting/digging spikes cost. | `weak` affliction → `exhausted` affliction → forced status downgrade to `UNSTABLE`. | `resource.caloric_changed`, `affliction.weak_*`, `affliction.exhausted_*`. |
| `battery_charge` | Android (only on variants that ship with batteries) | Recharge stations, generators, salvage power packs, base power tap. | Module operation; overclock; sensor draw. | Modules go slow → ability lockout → module shutdown. Organic side persists at reduced capability. | `resource.battery_changed`, `affliction.low_battery_*`, `chassis_module_state_changed{cause=battery_lockout}`. |
| `power` | Robot | Base power tap, generators, salvage power packs, dedicated charge action. | Every action: move, aim, fire, observe. Overclock multiplies cost. | Action cost rejection (the M0.2-F3 reject pattern: command returns `power_below_threshold` with structured reason). At zero, full inert state. | `resource.power_changed`, `resource.power_action_rejected`, `chassis_state_changed{stage=inert}`. |
| `heat` | Robot (global), Android (per-module) | Overclock, sustained fire, hot environment, friction/impact. | Passive radiation (varies by ambient + chassis design); coolant flow if intact. | At critical: damage-over-time to relevant module(s). | `chassis_heat_*`, `chassis_module_damaged{cause=overheat}`. |
| `g_load_dose` | Human, Android (reduced) | High-G acceleration / deceleration, repeated heavy impacts. | Passive decay; medical treatment accelerates. | `concussed` affliction; vision blackout; eventual blackout (out). | `affliction.concussed_*`, `body_g_load_dose_changed`. |
| `concussion_dose` | Human, Android (reduced); Robot 0 by design | Burst damage stacking; head/torso heavy impacts. | Passive decay; medical treatment. | Same as `g_load_dose`. (Or merged into `g_load_dose` per Open Questions.) | `affliction.concussed_*`. |
| `oxygen_supply` | Human, Android (organic side) | Equipped oxygen tank reserve. Standard atmosphere ambient also "tops off" passively via breathing (no tank consumption when ambient is breathable). | Per-tick consumption when ambient is vacuum / low-O2 / toxic-and-sealed. Multiplied under sprint/dig/combat (per equipment fields). | `hypoxia` affliction stacks once supply hits zero AND ambient is non-breathable → unconsciousness → death. Robots emit nothing. | `resource.oxygen_changed`, `affliction.hypoxia_*`, `helmet_breach` (if applicable). |

`power` is the new cross-cutting resource that has no analog in the existing spec; M5.8 (proposed) is the milestone that lands it. Until M5.8, robot scenarios are limited to "robots that don't need to track power" (debug fixtures only).

## Affliction Layer Extensions

[[spec/native-implementation-backlog#M5.7 — Hazard Package]] already names `concussed`, `corroded`, `electrified`, `wetness`, `burning`, `poisoned`, `asphyxiating`. This page extends the affliction set with origin-gated entries:

| Affliction | Applies To | Source |
|---|---|---|
| `concussed` (existing) | Human, Android (reduced) | Burst damage / G-load. NEVER fires on robots. |
| `internal_shock` (NEW) | Robot | Robot-only equivalent of `concussed`; per-module damage stack. |
| `bleeding` (existing, organic) | Human, Android | Wound `bleed_rate`. |
| `coolant_leaking` (NEW) | Robot | Penetrating round / structural module damage. |
| `oil_leaking` (NEW) | Robot | Joint/actuator damage. |
| `overheating` (NEW) | Robot (global), Android (per-module) | Overclock + ambient heat + fire material. |
| `low_battery` (NEW) | Android (battery variants only) | Battery resource crossing low threshold. |
| `power_starved` (NEW) | Robot | `power` resource below action-threshold band. |
| `weak` / `exhausted` (NEW) | Human, Android (organic side) | `caloric_energy` low / depleted. |
| `hypoxia` (NEW) | Human, Android (organic side) | `oxygen_supply` depleted while ambient is vacuum / low-O2 / sealed-toxic. |
| `downclocked` (NEW) | Robot (global), Android (per-module) | `heat` crossed throttle band from PASSIVE heat sources without active overclock. Distinct from `overheating` (which is the damage-imminent state); `downclocked` is the "throttling to avoid `overheating`" state. |
| `heat_exhaustion` (NEW) | Human, Android (organic side) | Sustained high ambient temperature; caloric drain spike + slowdown. |
| `frostbite` (FUTURE — flagged) | Human, Android (organic side) | Sustained cold ambient. Not in this contract; promote when cold-zone scenarios ship. |
| `irradiated` (FUTURE — flagged) | All (origin-gated severity) | Radiation ambient. Not in this contract; promote when irradiated-zone scenarios ship. |

All afflictions follow the existing affliction-layer rules from M5.7 (HUD-visible, decay/clear rules, replay events).

## Event Family Extensions

The existing event families (per [[references/prototype-run-bundle-schema]] and [[spec/body-damage-model#Replay / Run Evidence Events]]) are extended with the rows below. Use category `chassis` for all of them.

| Event | Required Fields | Notes |
|---|---|---|
| `body_force_feedback` | per Force-Feedback Contract above | Always emitted on hit. Origin gates payload content. |
| `body_g_load_dose_changed` | actor_id, old_dose, new_dose, cause_event_id | Humans + reduced androids only. Robots never emit. |
| `chassis_internal_shock_applied` | actor_id, module_id, damage_amount, parent_hit_event_id | Robots only. |
| `chassis_leak_started` / `_rate_changed` / `_stopped` | actor_id, channel (`coolant`/`oil`/`blood`), source zone/module, rate, parent event | Robots emit coolant/oil; humans/androids emit blood (already partial via wound model — this consolidates). |
| `chassis_overclock_started` / `_tier_changed` / `_stopped` | actor_id, scope (`global`/`module:<id>`), tier, power_cost_per_sec, heat_gain_per_sec | Robots use `global`; androids use `module:<id>`. |
| `chassis_heat_changed` | actor_id, scope, old_heat, new_heat, cooling_rate, cause | Drives overheat affliction. |
| `chassis_module_damaged` (existing extension) | adds `cause` enum: `internal_shock`, `overheat`, `battery_lockout`, `power_starved` | Reuses existing event; only enum extended. |
| `resource.caloric_changed` | actor_id, old, new, source/sink reason, cause_event_id | Humans + androids organic side. |
| `resource.battery_changed` | actor_id, old, new, source/sink reason | Android battery variants. |
| `resource.power_changed` | actor_id, old, new, source/sink reason | Robots. |
| `resource.power_action_rejected` | actor_id, attempted_action, required_power, available_power | The action-cost-reject path; mirrors M0.2-F3 reject pattern semantics. |
| `resource.oxygen_changed` | actor_id, old, new, source/sink reason, cause_event_id | Humans + androids organic side. Robots never emit. |
| `helmet_breach` | actor_id, helmet_item_id, breach_zone, parent_hit_event_id | Penetrating round to a sealed helmet; subsequent oxygen drains at multiplied rate. |
| `chassis_thermal_throttle_started` / `_tier_changed` / `_stopped` | actor_id, scope (`global`/`module:<id>`), tier, ambient_heat_source, cause_event_id | Robots use `global`; androids use `module:<id>`. Distinct from `chassis_overclock_*` — this is involuntary throttle. |
| `affliction.internal_shock_*` / `coolant_leaking_*` / `oil_leaking_*` / `overheating_*` / `low_battery_*` / `power_starved_*` / `weak_*` / `exhausted_*` / `hypoxia_*` / `downclocked_*` / `heat_exhaustion_*` | actor_id, intensity, cause_event_id | Standard affliction event shape. |

All new event types preserve the canonical schema envelope rules: `schema_version`, `category`, `event_type`, `tick`, `parent_event_id` chain back to the originating hit/contact event.

## AI Doctrine Implications

[[spec/native-implementation-backlog#M6 — AI Core And Trust Harness]] and [[spec/native-implementation-backlog#M6.6 — AI Material Competence]] need origin awareness to act sensibly:

| AI Need | Origin-Specific Behavior |
|---|---|
| Self-preservation under sustained fire | Human/Android: seek cover, avoid burst-stack concussion; medic if friendly. Robot: same, plus monitor `internal_shock` accumulation per module; retreat if processor module is at-risk. |
| Resource management | Human/Android: eat if `weak`; refill medkits; manage drug cooldowns. Robot: refill `power` at stations; manage overclock thermal envelope; refill coolant/oil if leaking. |
| Use of healing | Human/Android: bot can use medkits + drugs on self/allies. Robot: bot uses repair tools + coolant/oil refill + module swap, NEVER medkits. The reason label `wrong_origin_for_treatment` MUST exist for "robot tries medkit" rejections. |
| Use of overclock | Robot: bot may engage overclock when target threat is high AND thermal envelope allows; must drop overclock when `overheating` fires. Android: same per-module; bot must know which modules it has. |
| Friendly-fire avoidance with leaks | Bot must avoid igniting friendly-coolant/oil pools with fire/electricity — a robot ally bleeding oil + an enemy fire-thrower is a known hazard. |

## UX / HUD Contract

| Surface | Origin Gate |
|---|---|
| Body silhouette with armor zones | All origins; underlying skeleton differs (human/android shows wound markers; robot shows module bays). |
| Module strip (4-6 icons with stage colors) | All origins; robots show more module bays by default. |
| Pilot health pip | Humans (always), Androids (always), Robots (n/a — robot is the chassis). |
| Resource bar(s) | Human: caloric. Android: caloric + (battery if installed). Robot: power (always) + heat (always). |
| Overclock pip + heat ring | Robot: global. Android: per-module ring on silhouette. Human: none. |
| G-Force / concussion dose meter | Human: full. Android: reduced (capped at moderate). Robot: hidden. |
| Vision blackout effect | Human: full curve. Android: reduced curve. Robot: never. |
| Leak indicator | Robot coolant (cyan particle) / oil (dark particle). Human/Android blood (already covered). |
| Oxygen meter | Human, Android (in non-breathable ambient). Hidden in breathable ambient (no clutter). Auto-shows when ambient is vacuum/low-O2/sealed-toxic. Robot: never. |
| Helmet seal indicator | Human, Android (when helmet is equipped). Shows `sealed` / `breached`. Robot: never. |
| Throttle pip (involuntary downclock) | Robot global; Android per-module. Visually distinct from overclock pip — same ring widget, different color/icon — to avoid confusing voluntary boost with involuntary throttle. |
| Heat exhaustion / cold indicator | Human, Android (organic side) when ambient extremes apply. Robot: never (uses heat ring instead). |
| Affliction chips | All; chip set is origin-filtered (no `concussed` chip on robot, no `internal_shock` chip on human, no `hypoxia` chip on robot, no `downclocked` chip on human). |

Accessibility:

- The vision blackout, the overclock heat ring, and the leak particles all need non-color, non-screen-effect fallbacks (text caption / HUD icon / sound cue) per [[decisions/dr-012-accessibility-comfort-readability]] and [[spec/accessibility-comfort-slice-a]].
- A new accessibility flag `--reduced-g-force-blackout` MUST land in [[spec/prototype-roadmap#CLI Reference]] when the M5.7 hazard overlay UI lands; treat as a placeholder during M0/M1 (no behavior).

## Modding Contract

Mod authors must be able to:

- Define a new origin (a row in the matrix above) by declaring which branches it inherits from human / android / robot and which it overrides.
- Declare an origin's resource model (which of `caloric`, `battery`, `power`, `heat` apply, and at what rates).
- Declare an origin's affliction set (which afflictions can fire on this origin).
- Declare an origin's healing affordances (which item types can be used on this origin).
- Declare an origin's overclock posture (none / global / per-module).
- Declare an origin's G-Force / concussion posture (full / reduced / immune).

Schema is data-first per [[decisions/dr-006-modding-data-model]] and [[spec/modding-model]]. Lua escape hatches for affliction logic are allowed but the schema covers the common case.

## Milestone Routing

| Branch | Owning Milestone | Task Card |
|---|---|---|
| Origin `id` field on chassis record | M5 | M5-002 chassis model — extend with `origin_id` enum + per-origin reaction table. |
| Per-origin impulse-to-damage routing (concussion / internal-shock / fall) | M5.5 | M5.5-008 impulse-to-damage routing — already lists "threshold tests by material/origin"; this page makes the origin branches explicit. |
| Affliction set extensions (`internal_shock`, `coolant_leaking`, `oil_leaking`, `overheating`, `low_battery`, `power_starved`, `weak`, `exhausted`) | M5.7 | MAT-affliction layer — extend existing card. |
| G-Force vision blackout HUD effect | M5.7 | MAT-hazard overlay UI — extend existing card with the human-only blackout curve + accessibility fallback. |
| Leak channels (coolant/oil) routed into M5.6 material kernel | M5.7 | New row: ORIGIN-LEAK fixture under M5.7 OR M5.6, depending on whether the leak particles are first-class kernel materials (recommended) or actor-emitted scripted particles. |
| Overclock subsystem | NEW M5.8 | New milestone (proposed): origin resource & overclock pass — per-module heat, global heat, action-cost rejection on `power`, overclock event family. |
| Resource model (`caloric`, `battery`, `power`, `heat`) | NEW M5.8 | Same milestone; lands resource bars + affliction wires. |
| Food / medical / drug item interactions (origin-gated) | M5 + M5.8 | M5-001 role records: extend with `origin_compatibility` field. M5.8 wires bot-side `wrong_origin_for_treatment` rejection. |
| AI doctrine origin awareness | M6.6 | Already partial; this page locks the contract. |
| Vacuum / oxygen ambient signal | M7.5 | [[spec/native-implementation-backlog#M7.5 — Base Atmospherics]] — already owns atmosphere; extend with `oxygen_level` per-cell signal that the M5.8 origin resource pass consumes. |
| `oxygen_supply` actor accumulator + `hypoxia` affliction wiring | NEW M5.8 | Same M5.8 milestone as caloric/battery/power; lands the consumption rate, sprint/combat multipliers, and helmet/tank equipment hooks. |
| `chassis_thermal_throttle_*` events + involuntary downclock state | NEW M5.8 | Same M5.8 milestone; lands the passive-heat throttle distinct from the voluntary overclock state machine. |
| Helmet + oxygen tank equipment items (origin-gated) | M5 + M5.8 | M5-001 role records: extend equipment schema with `provides_seal`, `oxygen_capacity_seconds`, `consumption_modifier_*`, `origin_compatibility`. M5.8 wires the runtime consumption + `helmet_breach` + `wrong_origin_for_equipment` rejection. |
| High-heat / cold / irradiated ambient scenarios | M8.5 (Material Lab) + scenario library | M8.5 is where ambient-zone scenarios get built and balanced; this page is the contract those scenarios must satisfy. |

> [!important] Proposed M5.8 — Origin Resource & Overclock Pass
> The roadmap currently has no slot for the resource and overclock subsystem this page requires. Land it as **M5.8** between M5.7 (Hazard Package) and M6 (AI Core). Owns: resource bars, overclock state machine, `power`/`battery`/`caloric`/`heat` accumulators, action-cost rejection, leak particle routing into M5.6 materials, and the `wrong_origin_for_treatment` rejection contract. The first canonical worker that picks up M5.8 must add it to [[spec/prototype-roadmap]] §Milestones and to [[spec/native-implementation-backlog]] §M5.8 with task cards mirroring the existing M5/M5.5/M5.7 shape. Until then, this page is the only authoritative description of M5.8's scope.

## Acceptance Tests (ORIGIN-A)

These are the prototype acceptance tests that close this contract end-to-end. They run alongside (not replacing) BODY-A / CHASSIS-A / COLL-A / MAT-* / AI-H tests.

| Test | Setup | Pass Condition |
|---|---|---|
| ORIGIN-A-01 | Same impulse hits human, android, robot. | Replay shows three different reaction event chains; HUD shows three different feedback overlays; no origin emits an event meant for another origin. |
| ORIGIN-A-02 | Burst damage stacks on human, android, robot. | Human → `concussed` + blackout. Android → `concussed` reduced. Robot → `internal_shock` only, no `concussed`. |
| ORIGIN-A-03 | Identical fall onto same actor in each origin. | Human worst leg-break + bleed; android partial leg-break + reduced bleed; robot internal-shock to legs/actuators, no bleed. |
| ORIGIN-A-04 | Penetrating round into robot torso. | `chassis_leak_started{channel: coolant}` fires; particle visible; ground pool grows in M5.6 material kernel; particle ignites under fire reaction. |
| ORIGIN-A-05 | Robot enters overclock; sustains fire; thermal envelope crosses threshold. | `chassis_overclock_started` → `chassis_heat_changed` rising → `affliction.overheating_set` → `chassis_module_damaged{cause=overheat}`. Drop overclock; heat decays; affliction clears. |
| ORIGIN-A-06 | Android with batteries runs `caloric_energy` and `battery_charge` to zero independently. | Caloric depletion → `weak`/`exhausted`. Battery depletion → `low_battery` → module slowdown → module shutdown. Organic side persists at reduced capability when battery is empty. Recap names which side failed first. |
| ORIGIN-A-07 | Robot tries to fire when `power < required_power`. | Action rejected with structured reason `power_below_threshold`. `resource.power_action_rejected` event recorded. No silent no-op. Mirrors M0.2-F3 reject contract pattern. |
| ORIGIN-A-08 | Bot attempts medkit on robot ally. | Bot rejects with reason label `wrong_origin_for_treatment`. Event `ai_item_refusal` records refusal. |
| ORIGIN-A-09 | G-Force blackout test on human, repeated rapid impacts. | `g_load_dose` accumulates → vignette darkens in proportion → at `severe` peripheral vision gone → at `out` full blackout for fixed duration → recovery via dose decay or medical treatment. Accessibility flag `--reduced-g-force-blackout` reduces curve; non-visual caption fallback fires. |
| ORIGIN-A-10 | Same scenario as ORIGIN-A-09 on robot. | NO blackout event fires. Replay confirms `g_load_dose` is structurally absent (or 0 with origin-gate reason). |
| ORIGIN-A-11 | Vacuum-ambient scenario; one human, one android (organic+battery), one robot, all stationary. | Human + android: oxygen meter visible; `resource.oxygen_changed` ticks down at scenario rate; eventually `affliction.hypoxia_set` fires; eventually status reaches `DYING`. Robot: oxygen meter hidden; no `resource.oxygen_changed` events; no `hypoxia`; status stays `STABLE`. |
| ORIGIN-A-12 | Sealed human + sealed android in vacuum; tank reserve = 60s; sprint penalty = 2x. | Both consume reserve at standard rate while idle; rate doubles while sprinting. When reserve hits 0 in vacuum, hypoxia stacks. Penetrating shot to helmet emits `helmet_breach`; reserve drains at multiplied rate per equipment field. |
| ORIGIN-A-13 | High-ambient-heat scenario (foundry / lava-adjacent); one human, one android (mixed-spec), one robot, all idle. | Human: `heat_exhaustion` affliction stacks; caloric drains. Android: per-module heat rings rise on un-shielded modules; shielded modules are unaffected; eventually un-shielded modules `overheat` lockout; organic side gets `heat_exhaustion`. Robot: global heat rises; at throttle band, `chassis_thermal_throttle_started{scope: global}` fires AND `affliction.downclocked_set` fires; aim/move/reload speeds reduced; player did not request overclock; HUD throttle pip visible (distinct from overclock pip). |
| ORIGIN-A-14 | Robot enters foundry while already in voluntary overclock. | Heat accumulates from BOTH sources. Replay shows both `chassis_overclock_started` (from earlier) AND `chassis_thermal_throttle_started` (from passive heat) coexisting OR with documented precedence. Damage at critical heat fires correctly per existing `chassis_module_damaged{cause=overheat}` path. |
| ORIGIN-A-15 | Robot tries to equip an oxygen tank. | Slot-assign rejects with structured reason `wrong_origin_for_equipment`. `ai_item_refusal` event records refusal when bot attempts. No silent slot. |

## Open Questions

| Question | Cheapest Evidence |
|---|---|
| Should `g_load_dose` and `concussion_dose` be one accumulator or two? | Prototype with one; if blackout pacing diverges from concussion pacing, split. |
| Do androids have a partial internal-shock branch, or is that strictly robot-only? | Prototype: try emitting reduced-rate internal-shock on android module hits and measure tactical readability; default to robot-only until evidence supports the split. |
| What's the per-origin fall-damage curve? | Same drop test on three origins; tune to a player-readable difference. |
| Should `power` be a single resource or split into `power_reactor` + `power_capacitor` (long-term reserve vs burst)? | M5.8 prototype with split; demote if no tactical readability gain. |
| Should overclock be cancelable mid-tick or only at tick boundaries? | M5.8 determinism test; default to tick boundaries for deterministic replay. |
| Coolant ignition vs steam-flash priority — which reaction wins first when coolant pool is on a hot surface near fire? | M5.6 reaction-table priority test; lock priority before MAT-04..05 hazards ship. |
| Drug compatibility per android variant — global tag list or per-installed-module gate? | Prototype global tags; promote to module gates if narrative readability demands. |
| Robot `inert` state — is it a salvageable wreck immediately or a recoverable state that can be re-powered? | Mission scenario evidence; both should be possible but defaults must be explicit per origin variant. |
| Are sealed-helmet androids a thing (built-in seal, no tank needed for short vacuum exposure)? | Per-android-variant data field; default to "needs the same gear as humans" until variant evidence supports the split. |
| Is `frostbite` a launch affliction or deferred? | Cold-zone scenario evidence; flag in this contract but do not promote until a cold-zone mission ships. |
| Is `irradiated` a launch affliction or deferred? | Same as cold; flag here, promote when an irradiated-zone scenario ships. |
| Heat exhaustion vs `concussed` — can both stack and produce simultaneous blackout-style HUD effects? | Test combined exposure scenario; if HUD becomes unreadable, gate one effect at a time with a stacking priority. |
| Does ambient cold accelerate robot cooling enough to be tactically meaningful (use cold zones to enable longer overclock)? | M5.8 prototype; cap if it trivializes overclock cost. |
| Are oxygen tanks consumed at the same rate by androids as by humans, or do android-specific organic models drain slower? | Prototype with same rate; tune per variant if narrative readability demands. |

## Anti-Scope (during M0..M4)

- No `origin_id` field on the actor record before M5 (M0/M1/M2/M3/M4 must remain origin-agnostic; placeholder field is allowed only if needed for save-roundtrip and behavior is identity-equivalent across origins).
- No origin-specific HUD widgets in M4 (M4 owns the silhouette + module strip + pilot pip; origin-specific resource bars and overclock pips wait for M5.8).
- No origin-specific damage routing in M3 replay events (M3 records the events the engine emits; the engine doesn't emit origin-gated events until M5).
- No origin-specific item interactions in M5-001 role records BEYOND adding the `origin_compatibility` field as data; runtime gate landing waits for M5.8.

## Source Trail

- [[spec/chassis-armor-mechs-and-origins]]
- [[spec/body-damage-model]]
- [[spec/full-collision-physics-plan]]
- [[spec/equipment-loadout]]
- [[systems/damage-equipment-and-items]]
- [[systems/physics-and-destruction-models]]
- [[references/prototype-run-bundle-schema]]
- [[decisions/dr-003-body-damage-readability]]
- [[decisions/dr-014-tone-player-promise]]
- [[decisions/dr-027-combat-base-scope]]
- [[decisions/dr-033-full-collision-physics-direction]]
- [[decisions/dr-036-systemic-material-simulation-direction]]
- [[research-log/2026-05-06-origin-reaction-and-resource-design-intent]]

## Origin Radio Gating (Cross-Reference Per DR-043)

Per [[spec/comms-voice-and-radio-model]] origin gating:

| Origin | Radio access | How |
|---|---|---|
| **Human** | Equipped radio (occupies equipment slot) | Player chooses radio at loadout; uses suit power OR battery cell. |
| **Robot** | **Built-in radio** | Powered by chassis `power` resource. Frequency tuning via UI. Built-in antenna may be omnidirectional or chassis-shape-dependent. |
| **Android** | **Built-in OR modular upgrade** | Some android variants ship with built-in (default frequency-tuneable; powered by `battery_charge`). Modular upgrade adds the radio without taking an equipment slot. |
| Modder origin | Per modder spec | Schema declares radio access. |

Slot-assign rejects with `wrong_origin_for_equipment` when humans attempt to equip a robot's built-in-radio item or vice versa. AI bot picks emit `wrong_origin_for_treatment` for inappropriate radio assignments. Origin-gated equipment validation per [[spec/native-implementation-backlog#M5.8 — Origin Resource & Overclock Pass]]; full radio runtime kernel lands at M9.5 per [[decisions/dr-043-voice-comms-and-radio-direction]].

## Change Log

- 2026-05-06: Captured during M1 from user-supplied design intent (Round 1: combat reactions, healing affordances, resources, overclock). Status: `design-intent-post-m1`. M5/M5.5/M5.7 task cards cross-link here; M5.8 is now wired into Roadmap V2 as the origin resource and overclock pass.
- 2026-05-06 (later same session): Round 2 added — Environment Resistance Matrix (vacuum/oxygen consumption, heat tolerance, involuntary downclock vs voluntary overclock). New resources `oxygen_supply`; new afflictions `hypoxia`, `downclocked`, `heat_exhaustion`; future-flagged `frostbite`, `irradiated`. New events `resource.oxygen_changed`, `helmet_breach`, `chassis_thermal_throttle_*`. New tests ORIGIN-A-11..15. Helmet + oxygen tank equipment contract added. M7.5 atmospherics tagged as the environment-signal owner; M5.8 lands the actor-side accumulator and throttle state machine.
