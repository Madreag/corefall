---
type: spec
status: closed-direction
authority: "Atmospheric effects + persistent decals: human breath in cold weather, robot vents, oil/coolant pools, blood splatters, frost patches, scorch marks, dust trails, weather precipitation, casing physics, crater/bullet-hole terrain decals."
ready_when: "All effects fire correctly per EnvironmentSignal + replay events; performance budget met at Steam Deck floor; effects feed into gameplay (slip on frost, ignite oil, AI affordance)."
feeds:
  - DR-002
  - DR-003
  - DR-014
  - DR-019
  - DR-024
  - DR-028
  - DR-036
  - DR-037
  - DR-038
  - DR-040
  - DR-044
---

← [[spec/index|spec section]] · [[spec/art-and-asset-pipeline|art pipeline]] · [[spec/vfx-and-particles|VFX/particles]] · [[spec/lighting-and-shadows|lighting/shadows]] · [[spec/environmental-conditions-model|environmental conditions]] · [[spec/atmospherics-and-chemistry-model|atmospherics]] · [[decisions/dr-044-audiovisual-production-pipeline|DR-044]]

# Atmospheric Effects & Decals

> [!summary] What this page is
> Player-asked specifically: "we should be able to see human breath in cold weather, blood stains, ..." This page is the canonical authority for breath/blood/oil/frost/scorch/dust/weather and other persistent + dynamic atmospheric/environmental effects. Every effect feeds into gameplay (slip mechanic, ignition, AI affordance), not just visual flair.

## Effects Catalog

### Human Breath (Cold)

**Trigger:** Per-actor when `actor.body_temperature_K - environment.air_temperature_K > BREATH_VISIBLE_THRESHOLD` AND `environment.atmosphere.kPa > VACUUM_THRESHOLD` AND actor's helmet is open OR actor has no helmet.

**Visual:** 6-frame breath-cloud sprite loop (Tier 2: AnimateDiff-generated; faction-tinted slightly). Emits from actor's mouth/face anchor every 2-4s on inhale-exhale cycle.

**Gameplay tie-ins:**
- AI hearing system (per DR-008): visible breath signals "actor is here" to enemy AI vision.
- Stealth: in cold biomes, holding breath momentarily is a stealth tactic.
- Affliction warning: heavy breathing in cold = actor is panicked or wounded.

**Origin variants:**
- Humans: standard white cloud
- Androids: modular; some have cooling vents that emit breath-like vapor
- Robots: NO breath; they vent (separate effect, see below)

### Robot Vent (Overheat)

**Trigger:** Per-robot/mech when `chassis.heat_K > VENT_THRESHOLD_K` per [[spec/origin-reaction-and-resource-model]] heat tolerance.

**Visual:** Steam plume from chassis vent ports; intensity scales with `chassis.heat`. White-blue tint. Continuous if overclocking.

**Gameplay tie-ins:**
- Visible signal of "robot is overclocking; might downclock soon."
- AI hearing system.
- Heat haze visual distortion (small) for nearby actors.

### Blood Splatter (Wound)

**Trigger:** `wound_added` event on humans, androids per `body_damage_channel`. Direction-aligned per projectile vector.

**Visual:** 8-frame splatter animation; per-projectile-vector + impact-position; spawns persistent blood decal on terrain after burst.

**Faction variants:**
- Humans: red
- Androids: red on organic side, oil-blue on synthetic side
- Husks: green-yellow
- Aliens: purple

**Gameplay tie-ins:**
- Forensic trail: AI scout can follow blood trail.
- Status indicator: blood pool grows over time if actor is bleeding-out (per DR-018).
- Affliction "infection" possible from contaminated wounds.

### Oil / Coolant Burst (Robot/Mech Wound)

**Trigger:** `chassis_leak_started` event on robots/mechs per [[spec/origin-reaction-and-resource-model]] coolant + oil leak channels.

**Visual:** Direction-aligned spray (oil = black; coolant = green); spawns persistent puddle.

**Gameplay tie-ins:**
- Oil ignites on contact with fire per `cf-material` reaction.
- Coolant pool freezes at low temperature → frost patch (slip).
- Resource drain: actor's `chassis.oil` / `chassis.coolant` resources drop.

### Persistent Blood Decal

**Trigger:** Blood splatter VFX completion.

**Visual:** Per-`cf-material` decal placed on terrain. Per-faction blood color.

**Persistence:**
- Default 5 minutes real-time.
- Configurable per scenario.
- Cleanup budget governor drops oldest first under perf pressure.

**Gameplay tie-ins:**
- Forensic trail.
- Infection vector for fresh wounds in contact.

### Persistent Oil Pool

**Trigger:** Oil burst VFX OR oil pipe rupture per [[spec/atmospherics-and-chemistry-model]].

**Visual:** Black puddle decal; spreads per gravity field per DR-038; pools in low spots.

**Gameplay tie-ins:**
- Ignites with any fire/spark/projectile-with-tracer → fire pool.
- Slipping mechanic: actors lose footing crossing oil.
- Robot consume: damaged robots drink oil to refill `chassis.oil` per [[spec/origin-reaction-and-resource-model]].

### Persistent Coolant Pool

**Trigger:** Coolant burst VFX OR coolant pipe rupture.

**Visual:** Green puddle decal; freezes if `air_temperature < FREEZING_K`.

**Gameplay tie-ins:**
- Frozen variant = frost patch (slip).
- Robot consume.

### Frost Patch

**Trigger:** Spawns on cold surfaces (`material.surface_temp < FROST_FORM_K`) OR coolant pool freezing.

**Visual:** Light-blue crystalline pattern decal.

**Gameplay tie-ins:**
- Slip mechanic: actors lose footing; chassis with magnetic boots (per DR-038) bypass.
- Visibility: covers material details, harder to see what's underneath.

### Scorch Mark / Burn

**Trigger:** Explosion / fire / plasma weapon impact.

**Visual:** Black burn pattern decal; faction-specific shape (clean military burn ≠ improvised explosive).

**Persistence:** Permanent until terrain mutated per `cf-material`.

**Gameplay tie-ins:**
- Forensic: indicates explosion origin.
- Material affordance: scorched dirt may have different material properties (more prone to ignition; reduced traction).

### Dust Trail (Movement)

**Trigger:** Animation tag `footstep_*` on movement.

**Visual:** Per-actor footstep dust puff; intensity per movement speed + ground material; faction-tinted.

**Gameplay tie-ins:**
- Stealth: heavy dust trail = AI vision detection.
- Material indication: sand kicks up more dust than concrete.
- In vacuum: NO dust (no atmosphere to suspend particles).

### Casing Eject (Physics-Based)

**Trigger:** Animation tag `casing_eject` per [[spec/animation-system]].

**Visual:** Per-weapon casing sprite; bouncing physics per gravity field per DR-038; persists 8-15s; sound on bounce.

**Gameplay tie-ins:**
- Forensic trail.
- AI hearing.
- Cosmetic flag (replay-deterministic position not required).

### Bullet Hole / Wall Decal

**Trigger:** Projectile impact on solid terrain/walls.

**Visual:** Per-material bullet-hole decal + crack pattern.

**Persistence:** Persistent until terrain mutated.

**Gameplay tie-ins:**
- Forensic.
- Material weakening: many bullet holes cluster → wall integrity reduced.

### Crater (Explosive Impact)

**Trigger:** Explosive projectile detonation.

**Visual:** Per-explosive-yield crater shape on terrain.

**Persistence:** Permanent (terrain mutation per `cf-material` carve).

**Gameplay tie-ins:**
- Tactical: provides cover post-explosion.
- Material affordance: edge of crater has reduced traction.

### Weather Precipitation

Per DR-040 + [[spec/environmental-conditions-model]].

| Weather | Visual |
|---|---|
| Earth rain | Vertical rain streaks; impacts on materials produce splash particles + wet decals |
| Earth snow | Drifting snowflakes; accumulates on terrain (wet decal) |
| Mars dust storm | Horizontal dust streaks at high intensity; visibility reduction; covers windscreens/visors |
| Vulcan acid rain | Yellow-green streaks; damages exposed armor per `cf-material` reaction; spawns acid puddle decals |
| Vulcan ash fall | Slow-falling gray particles; accumulates on terrain (gray decal); reduces visibility |
| Vulcan sulfur fog | Volumetric yellow fog; reduces visibility + breathing-toxic |
| Mimas vacuum | NO precipitation (vacuum world) |
| Europa cryo storm | Ice-crystal precipitation; freezes liquid surfaces + spawns frost decals |
| Solar flare (M7.7) | Saturated red ambient + auroral overlay; no precipitation but radio static per DR-043 |

### Steam (Water+Heat)

**Trigger:** `reaction.steam_formed` per [[spec/atmospherics-and-chemistry-model]].

**Visual:** Rising steam cloud; rises per gravity per DR-038; fades per ventilation.

**Gameplay tie-ins:**
- Visibility blocker.
- Cools surrounding atmosphere.

### Smoke (Combustion)

**Trigger:** `atmospherics.combustion_started`.

**Visual:** Black smoke rising; fades per ventilation; per gravity (rises in positive g; spreads in zero-g).

**Gameplay tie-ins:**
- Visibility blocker.
- Toxic if inhaled (organic origins).
- AI affordance: AI avoids smoke for tactical reasons (visibility) AND health reasons.

### Fire

**Trigger:** `material.ignited` per `cf-material` reaction.

**Visual:** Per-material flame sprite (oil = orange tongues; plasma = blue arc; electrical = white spark).

**Gameplay tie-ins:**
- Spreads per material chemistry.
- Damages actors per [[spec/body-damage-model]] thermal channel.
- Consumes oxygen per [[spec/atmospherics-and-chemistry-model]].

### Footprints

**Trigger:** Animation tag `footstep_*` on soft materials (sand, mud, snow).

**Visual:** Per-foot indentation decal.

**Persistence:** ~1-3 minutes real-time.

**Gameplay tie-ins:**
- Forensic / scout AI tracking.

### Sparks (Hard Impact)

**Trigger:** Hard-velocity metal-on-metal impact.

**Visual:** Spark cluster + bounce per gravity + sound.

**Gameplay tie-ins:**
- Cosmetic flag.
- Visual feedback for armor effectiveness.

### Heat Haze

**Trigger:** Hot surfaces (lava, fire, plasma weapons recently fired, overheated robots).

**Visual:** Distortion shader applied behind hot regions.

**Gameplay tie-ins:**
- Cosmetic flag.
- Visibility hint.

### EMP Static

**Trigger:** EMP weapon discharge; solar flare; damaged robot.

**Visual:** Brief screen static + radio interference per DR-043 + nearby electronics flicker.

**Gameplay tie-ins:**
- Disrupts robot/android per `chassis.power` damage channel.
- Disrupts radio per DR-043.

## Performance Budget

Per [[spec/vfx-and-particles]] particle budget. Decals separate:

| Tier | Active decals | Notes |
|---|---|---|
| Steam Deck | ≤200 active | Cleanup oldest first under pressure |
| Mid-range | ≤500 | |
| High-end | ≤1500 | |

## File Format

```ron
// content/atmospheric_effects/breath_cold.ron
effect: (
    id: "breath_cold",
    category: "atmospheric",
    trigger: { kind: "actor_breathing_cold", threshold_K: 10.0 },
    visual: {
        kind: "particle_loop",
        sprite: "tier3/atmospheric/breath_cloud.atlas.png",
        frames: 6,
        duration_s: 0.6,
        emit_rate_per_s: 1.5,
        anchor: "actor.face",
        faction_tint: true,
    },
    gameplay: {
        ai_hearing: 0.2,  // small noise signature
        ai_visibility: 0.5,  // large visual signature in cold
    },
    cosmetic: false,  // affects gameplay
)
```

## Replay Events

- `atmospheric.breath_emit` (per actor)
- `atmospheric.vent_emit` (per robot)
- `decal.placed` (per type, per cell)
- `decal.faded` / `decal.removed`
- `atmospheric.weather_precipitation_tick` (per scenario second)

## Done-Criteria

- [ ] Breath visible at <0°C ambient + breathable atmosphere; not in vacuum.
- [ ] Robot vent emits per overheat threshold.
- [ ] Blood/oil/coolant decals direction-aligned per projectile vector.
- [ ] Frost forms on cold surfaces; freezes coolant pools; affects movement.
- [ ] Weather precipitation per world; per intensity.
- [ ] Decal persistence + cleanup budget respected.
- [ ] All effects emit replay events.
- [ ] AI affordances feed correctly (vision + hearing + tactical).

## Source Trail

- [[decisions/dr-044-audiovisual-production-pipeline]]
- [[spec/atmospherics-and-chemistry-model]]
- [[spec/origin-reaction-and-resource-model]] for robot heat/coolant/oil
- [[spec/environmental-conditions-model]] for weather
- [[spec/full-collision-physics-plan]] for casing physics
- Project owner verbatim: "we should be able to see human breath in cold weather, blood stains..."
