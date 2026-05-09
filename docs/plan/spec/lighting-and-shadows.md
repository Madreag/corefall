---
type: spec
status: closed-direction
authority: "2D dynamic lighting + shadow system: normal-mapped sprites + radial point lights + ambient + light volumes + soft shadows + sky shader. Per-world ambient blending. Per-EnvironmentSignal modulation. AI-driven authoring."
ready_when: "All chassis read normal maps; dynamic lights affect actor + terrain + props; soft shadows render at Steam Deck floor; sky shader produces per-world variants; cinematic comic-noir lighting punches at gameplay events."
feeds:
  - DR-019
  - DR-024
  - DR-028
  - DR-039
  - DR-040
  - DR-044
---

← [[spec/index|spec section]] · [[spec/art-and-asset-pipeline|art pipeline]] · [[spec/visual-direction|visual direction]] · [[spec/atmospheric-effects-and-decals|atmospheric/decals]] · [[spec/celestial-bodies-and-worlds-model|worlds]] · [[decisions/dr-044-audiovisual-production-pipeline|DR-044]]

# Lighting & Shadows

## Approach

**2D dynamic lighting via normal-mapped sprites + radial point lights + ambient lighting + light volumes + soft shadows + procedural sky shader.** Per-world ambient blends from `EnvironmentSignal.day_night` + `weather` + `magnetosphere`. Cinematic comic-noir lighting punches at gameplay events (death, victory, breach).

## Stack

| Component | Detail |
|---|---|
| **Renderer** | `cf-render-2d` (custom wgpu pipelines per DR-024). Normal-map shader + light-volume shader + shadow-mask shader. |
| **Normal-map generation** | Tier 2: Flux.1-dev + ControlNet Depth → automated normal-map bake via `materialize` or `ComfyUI-Normal-Map` custom node. |
| **Per-asset normals** | Authored alongside diffuse sprites in `tier2/<category>/<id>_normal.png`. |
| **Lights** | `cf-lighting` crate. Radial point lights (per-actor flashlights, muzzle flashes, fires, dropship landing lights, base interior, command core glow). |
| **Ambient lighting** | Per-world `World.ambient_light_color` + `solar_distance_au` + day-night cycle modulation. |
| **Light volumes** | Per-base-cell, per-mech-interior, per-suit (sealed actor in vacuum gets internal light from helmet visor). |
| **Soft shadows** | Per-light shadow-cast via screen-space shadow mask; configurable resolution; Steam Deck uses cheap variant. |
| **Sky shader** | Procedural wgpu shader; per-world `World.sky_definition` (gradient + star density + parallax + day/night/weather variants). |
| **Bevy ecosystem reference** | `bevy_lit`, normal-mapped 2D PR #14586, `bevy_light_2d`. |

## Per-World Ambient

Each world has an ambient lighting profile derived from astrography + atmospherics:

| World | Day ambient color | Night ambient | Notes |
|---|---|---|---|
| Earth | Warm yellow-white (3500K → 5500K) | Cool blue (3500K) + moon | Atmospheric scattering = blue sky / orange sunset |
| Earth's Moon | High-contrast harsh white-on-black | Earthshine cool blue | No atmosphere = sharp shadows |
| Mars | Orange-pink dusty light (4500K) | Cool blue | Dust storms reduce visibility |
| Phobos / Deimos | Same as Moon (vacuum, harsh) | Mars-shine | |
| Mimas | Saturn-shine warm + Sun-distant cool | Star-field clarity | Vacuum |
| Europa | Jupiter-shine warm + Sun-distant cool | Same | Vacuum + sub-ice |
| Vulcan | Red-orange thermal glow + lava emission | Lava persistent ambient | Volcanic |
| Venus | Yellow-orange diffuse (heavy atmosphere) | None (always lit by greenhouse) | High pressure scatter |
| Sol | Surface-incompatible | N/A | Reference only |
| Belt asteroid | Same as Moon | Same | Vacuum |
| Orbital station | Per-section configurable | Same | Sealed |

Ambient blends per-frame from `EnvironmentSignal.day_night.phase` + `EnvironmentSignal.weather.intensity` + magnetosphere.

## Sky System

Per-world `World.sky_definition.ron`:

```ron
sky: (
    id: "mars",
    base_gradient: [
        (top: "1a0a05", horizon: "552211", bottom: "884422"),  // dawn
        (top: "443322", horizon: "885533", bottom: "997744"),  // day
        (top: "221100", horizon: "440011", bottom: "330000"),  // dusk
        (top: "000011", horizon: "001122", bottom: "000505"),  // night
    ],
    star_density: 0.6,  // visible at night
    parallax_offset_per_layer: [0.05, 0.15, 0.4, 1.0],
    weather_variants: {
        "dust_storm": "1a0a00",  // dim red-brown
        "clear": (default),
    },
    light_emission: { sun_strength: 0.4, sun_color: "ffaa66" },  // weaker than Earth's
)
```

Runtime sky shader (wgpu):
- Reads spec + `EnvironmentSignal.day_night` + `weather`.
- Blends gradient per phase.
- Adds star field at night (parallax-scrolling).
- Modulates ambient light + sun position.
- Renders behind parallax background layers per [[spec/art-and-asset-pipeline]].

## Light Sources

| Source | Light type | Notes |
|---|---|---|
| **Sun** | Directional, infinite | Color + intensity per world + time + weather |
| **Muzzle flash** | Radial point, brief | Per [[spec/vfx-and-particles]]; 80ms; warm color |
| **Explosion** | Radial point, decaying | Hot orange → red → gone; ~0.3-0.6s |
| **Fire** | Radial point, flickering | Animated per-tick; orange |
| **Dropship landing lights** | Spot lights | Ship-mounted; cone projection |
| **Player headlamp** | Spot light | Forward cone; toggleable |
| **Vehicle headlights** | Spot lights | Forward cone |
| **Base interior** | Radial point per fixture | Dim warm OR bright fluorescent depending on faction |
| **Command core glow** | Pulsing radial | Per DR-027; faction-tinted |
| **Hazard alert** | Radial pulse | Red/yellow per affliction state |
| **Plasma weapons** | Radial point + heat haze | Energy wash |
| **Lava** | Radial point per cell | Per `cf-material` |
| **EMP arc** | Radial pulse + decay | Cyan; brief |
| **Comms uplink** | Subtle spot | Antenna emission |

## Shadow Casting

| Element | Casts shadow? | Notes |
|---|---|---|
| Actors (all) | Yes | Procedural shape from sprite outline; soft-shadow projection |
| Vehicles | Yes | |
| Base objects | Yes | Per object collision footprint |
| Terrain | Yes | Per `cf-material` chunk shape |
| Projectiles | No | Cosmetic flag |
| Particles | No | Cosmetic flag |
| Decals | No | Cosmetic |

Shadow rendering:
- Screen-space mask per-light per-frame.
- Soft-shadow blur (1-3 px on Deck floor; 5-9 px on desktop high).
- Color shadows (under colored lights, shadow tints accordingly).
- Cinematic punch: critical hits/death briefly intensify shadow contrast.

## Cinematic Punches

Per gameplay events, per DR-046:

| Event | Lighting effect |
|---|---|
| Critical hit | Brief screen flash + intensified rim light on target |
| Player death | Slow-mo + camera dolly + dramatic spotlight on body + dim ambient |
| Match victory | Sun pierces through clouds (procedural) + golden ambient + comic-page-flip |
| Match defeat | Ambient drains to gray + dim spot on commander/core + scroll-of-failure |
| Bunker breach | Door opens → hot light pours through → silhouettes of actors |
| Reactor breach | Pulsing red emergency lighting + alarm strobe + ambient flicker |
| EMP burst | Brief darkness + electrical static |
| Solar flare (per DR-040 weather) | Saturated red ambient + auroral glow |

## File Format

```ron
// content/lights/dropship_landing.ron
light: (
    id: "dropship_landing_pair",
    kind: "spotlight_pair",
    color: "ffaaaa",
    intensity: 1.2,
    radius_px: 96,
    cone_angle_deg: 35,
    cast_shadow: true,
    flicker_amp: 0.0,
    pulse: false,
)
```

## Performance Budget

| Tier | Active lights | Shadow casters | Notes |
|---|---|---|---|
| Steam Deck (800p/60) | ≤32 dynamic + 1 ambient + 1 sky | ≤16 shadow casters | Soft shadow blur 1-3 px |
| Mid-range (1080p/60) | ≤96 dynamic + 1 ambient + 1 sky | ≤48 shadow casters | Blur 3-5 px |
| High-end (4K/120) | ≤256 dynamic + 1 ambient + 1 sky | ≤128 shadow casters | Blur 5-9 px |

Budget governor:
- Cosmetic lights dropped first under pressure.
- Distance-based culling.
- Reported in `summary.json.perf.lighting_drop_count`.

## AI-Driven Authoring

| Aspect | Tier |
|---|---|
| Per-asset normal map | Tier 2 (Flux.1-dev + ControlNet Depth → automated bake) |
| Per-world sky concept | Tier 2 (Flux.1-dev panoramic; layered) |
| Per-light tuning | Tier 3 (in-engine via `cfctl observe --lights` + project-owner playtest) |
| Cinematic punch tuning | Tier 3 (project-owner playtest) |

## Done-Criteria

- [ ] Every chassis sprite has matching normal-map.
- [ ] Sky shader produces per-world + per-time-of-day + per-weather variants.
- [ ] Soft shadows render at Steam Deck floor.
- [ ] Cinematic punches trigger correctly on gameplay events.
- [ ] Per-world ambient blends correctly across day-night-weather cycles.
- [ ] CI gate: every chassis has normal-map; every world has sky-definition.

## Source Trail

- [[decisions/dr-044-audiovisual-production-pipeline]]
- [[decisions/dr-019-visual-direction]]
- bevy_lit: https://github.com/loopystudios/bevy_light_2d
- Bevy 2D normal map issue: https://github.com/bevyengine/bevy/issues/14586
- WebGL 2D dynamic lighting tutorial: https://www.mattgreer.dev/blog/dynamic-lighting-and-shadows/
- ComfyUI Normal-Map nodes: various community
