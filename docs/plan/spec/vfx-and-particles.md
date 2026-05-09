---
type: spec
status: closed-direction
authority: "VFX + particle system: muzzle flashes, casings, smoke, blood, breath, sparks, weather, decals, explosions. Cosmetic flag for replay determinism. Budget governor for perf. AI-driven authoring + tuning."
ready_when: "All atmospheric effects render at 800p/60 Steam Deck floor with full launch roster active; cosmetic VFX excluded from determinism replay; flashy + punchy juice across all gameplay events."
feeds:
  - DR-002
  - DR-014
  - DR-019
  - DR-024
  - DR-028
  - DR-033
  - DR-036
  - DR-037
  - DR-038
  - DR-040
  - DR-044
---

← [[spec/index|spec section]] · [[spec/art-and-asset-pipeline|art pipeline]] · [[spec/animation-system|animation system]] · [[spec/lighting-and-shadows|lighting/shadows]] · [[spec/atmospheric-effects-and-decals|atmospheric effects/decals]] · [[decisions/dr-044-audiovisual-production-pipeline|DR-044]]

# VFX & Particles

## Architecture

**Hybrid: GPU-instanced sprite particles for cosmetic VFX (high count, cheap) + CPU-deterministic particles for gameplay-critical VFX (replay-recorded).**

| System | Purpose | Replay determinism |
|---|---|---|
| `cf-vfx-cosmetic` | Visual flair: smoke wisps, screen flash, dust trails, sparkles, decorative debris. | NOT replay-deterministic. Cosmetic flag. |
| `cf-vfx-gameplay` | Cause-chain VFX: muzzle flash anchor, casing eject, blood splatter direction, oil drips, decal placement. | Replay-deterministic. Events fire. |
| `cf-decal` | Persistent terrain decals: blood, oil, scorch, frost, footprints. Per-`cf-material` chunk. | Replay-deterministic. |

## Particle Categories

### Combat (cause-chain VFX)

| Effect | Trigger | Visual |
|---|---|---|
| **Muzzle flash** | `weapon_fired` event | Per-weapon flash signature (laser ≠ kinetic ≠ plasma); 2-frame sprite + dynamic light emission |
| **Casing eject** | Animation tag `casing_eject` | Per-weapon casing sprite; bouncing physics; persists 8-15s; sound on bounce |
| **Tracer round** | `projectile_spawned` event (visible-tracer flag) | Streak per-projectile path; brightness fade per range |
| **Bullet impact dust** | `projectile_hit_terrain` event | Material-typed dust puff (sand=tan, rock=gray, metal=spark+ricochet sound) |
| **Blood splatter** | `wound_added` event (humans, androids) | Direction-aligned per projectile vector; spawns persistent decal (see below) |
| **Oil/coolant burst** | `wound_added` event (robots) | Direction-aligned spray; spawns oil decal |
| **Limb separation** | Animation tag `limb_detach` | Detached limb sprite + blood/oil burst + bone fragments |
| **Explosion** | `projectile_terminated` (explosive) | Multi-stage: flash → fireball → smoke + debris + scorch decal |
| **Plasma discharge** | Plasma weapon fire | Energy arc + ionization shimmer + secondary plasma cloud |
| **EMP arc** | EMP weapon fire | Cyan zigzag pattern; shorts nearby electronics (gameplay) + screen static (cosmetic) |
| **Laser beam** | Laser weapon fire | Continuous beam sprite + heat haze + glow |
| **Railgun trail** | Railgun fire | High-velocity sonic boom + ionization streak |

### Atmospheric (per [[spec/atmospheric-effects-and-decals]])

| Effect | Trigger | Visual |
|---|---|---|
| **Breath (cold weather)** | Per-actor when `actor.body_temp - air_temp > threshold` | 6-frame breath cloud loop; faction-tinted; visible only in cold + breathable atmosphere |
| **Robot vent (overheat)** | Per-robot when `chassis.heat > threshold` | Steam plume from vent ports; intensity scales with heat |
| **Dust trail (movement)** | Animation tag `footstep_*` | Per-actor footstep dust; intensity per movement speed + ground material |
| **Jet flame** | Jetpack thrust active | Per-jetpack flame sprite + heat distortion + light emission |
| **Smoke trail (projectile)** | Per-projectile flag | Streak smoke fade per range |
| **Weather precipitation** | `weather.event_started` per DR-040 | Rain/snow/dust/ash/acid particle field; per-cell density |
| **Sparks (impact)** | Hard-impact metal-on-metal | Spark cluster + bounce + sound |
| **Steam (water+heat)** | `reaction.steam_formed` per [[spec/atmospherics-and-chemistry-model]] | Rising steam cloud |
| **Smoke (combustion)** | `atmospherics.combustion_started` | Black smoke rising; fades per ventilation |
| **Fire** | `material.ignited` | Per-material flame: oil = orange, plasma = blue, electrical = white |

### UI / Juice

| Effect | Trigger | Visual |
|---|---|---|
| **Hit confirm** | Damage dealt | Brief screen flash + crosshair pulse + bass thump |
| **Critical hit** | High-damage event | Slow-mo 80ms + chromatic aberration + screen flash |
| **Death cam zoom** | Player death | Camera dolly + slow-mo + replay handoff |
| **Achievement unlock** | Per DR-046 | Comic-panel pop-in + cheer sting + collection update |
| **Match start** | Mission begin | Dropship cinematic + camera drift + LZ flash |
| **Match victory** | Mission win | Comic-page-flip + slow-mo + adaptive music swell + confetti VFX |
| **Match defeat** | Mission lose | Scroll-of-failure transition + adaptive music dirge |
| **Pickup glow** | Lootable on ground | Soft glow halo + faction-color tint |
| **Reload progress** | Reload animation | Bar fills + chamber-snap sound at end |
| **Healing tick** | Medical apply | Heart pulse + green particle drift |
| **Damage flash** | Damage taken | Red vignette + screen shake (magnitude scaled) + heartbeat sub-bass |
| **Status effect indicators** | Affliction applied | Floating icon over actor + caption per DR-020 |

## Persistent Decals

Per [[spec/atmospheric-effects-and-decals]]:

| Decal | Persistence | Cleanup |
|---|---|---|
| Blood splatter | Real-time minutes (configurable; default 5min) | Faded then removed; perf budget governor drops oldest first under load |
| Oil pool | Persistent until burned/cleaned | Reactive: ignites + spreads per `cf-material` |
| Coolant pool | Real-time minutes; freezes if ambient temp low (per DR-049) | Frozen variant slips actors |
| Scorch mark | Persistent until terrain mutated | Per `cf-material` chunk |
| Footprints | Real-time seconds (1-3 min) | Faded |
| Frost patch | Active per ambient temp | Melts when temp rises |
| Crater | Permanent terrain mutation | Per terrain modification system |
| Dust pile (blast) | Settles in ~5s | Then removed |
| Bullet hole (wall) | Persistent until terrain mutated | Per `cf-material` |
| Charcoal/burn (post-fire) | Persistent until cleaned | Per `cf-material` |

## Performance Budget

| Tier | Particle count | Decal count | Notes |
|---|---|---|---|
| **Steam Deck (800p/60)** | ≤1500 active particles | ≤200 active decals | Cosmetic flag drops oldest first |
| **Mid-range desktop (1080p/60)** | ≤4000 active particles | ≤500 active decals | |
| **High-end desktop (4K/120)** | ≤10000 active particles | ≤1500 active decals | |

Budget governor:
- Cosmetic-flag particles dropped first.
- Oldest decals fade then unloaded.
- Critical gameplay VFX never dropped.
- Reported in `summary.json.perf.vfx_drop_count`.

## AI-Driven Authoring

Per [[spec/art-and-asset-pipeline]]:

- Tier 1: procedural primitives (rectangles, circles).
- Tier 2: SDXL+LoRA generated particle textures (Civitai pixel-art LoRAs); ComfyUI Sprite Sheet Generator for animated VFX.
- Tier 3: Aseprite cleanup + per-effect tuning in-engine via `cfctl observe --vfx --tune`.

## File Format

```ron
// content/vfx/muzzle_flash_kinetic.ron
vfx: (
    id: "muzzle_flash_kinetic",
    category: "combat",
    cosmetic: false,
    sprite: "tier3/vfx/muzzle_flash_kinetic.atlas.png",
    frames: 2,
    duration_s: 0.05,
    light_emission: { color: "ffaa55", intensity: 0.8, radius_px: 24 },
    sound: "weapon_fire_kinetic_<weapon_id>",
    spawns_caption: false,  // muzzle flash is cosmetic detail; no caption
)

// content/vfx/blood_splatter.ron
vfx: (
    id: "blood_splatter",
    category: "combat",
    cosmetic: false,
    sprite: "tier3/vfx/blood_splatter.atlas.png",
    frames: 6,
    duration_s: 0.3,
    decal_on_complete: "decal_blood_persistent",
    direction_align: true,
    sound: "wound_impact_organic",
    spawns_caption: true,  // wound impact is gameplay-critical
)
```

## Replay Events

Per [[references/prototype-run-bundle-schema]]:

- `vfx.cosmetic_spawned` (cosmetic flag = true)
- `vfx.gameplay_spawned` (cosmetic flag = false; replay-deterministic)
- `decal.placed`
- `decal.removed`

## Done-Criteria

- [ ] All combat + atmospheric + UI VFX implemented.
- [ ] Steam Deck floor passes 800p/60 with full launch roster active.
- [ ] Cosmetic flag separates determinism vs visual.
- [ ] Decal persistence + cleanup works.
- [ ] AI-driven generation pipeline produces production-quality VFX.
- [ ] Flashy + punchy juice rules per DR-046 implemented across all gameplay events.

## Source Trail

- [[decisions/dr-044-audiovisual-production-pipeline]]
- [[decisions/dr-046-player-facing-surfaces-direction]] for juice rules
- [[spec/atmospheric-effects-and-decals]] for breath/blood/etc. specifics
