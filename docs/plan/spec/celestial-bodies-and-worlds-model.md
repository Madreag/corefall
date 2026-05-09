---
type: spec
status: design-intent-post-m1
authority: "Canonical contract for the World catalog: planets, moons, asteroids, suns, stations, anomalies. Every subsystem (atmospherics, gravity, terrain, materials, mission director, mining, AI, server, MMO shards) reads world-level data from this single record. Includes simplified Keplerian orbital math (parent + semi-major axis + period + phase + rotation period + tilt), per-tick position + distance + comms-latency lookups. Captured during M1; implementation lands at extended M2 + M5.6 + M5.9 + new M5.10 and finalizes at new M7.7. M0/M1 stay world-config-only."
ready_when: "Canonical World schema exists; 12 launch worlds (Earth, Earth's Moon, Mars, Phobos, Deimos, Mimas, Europa, Vulcan, Venus, Sol, Belt-Asteroid representative, Orbital-Station representative) load deterministically; orbital position + comms latency are byte-identically reproducible across replay; all subsystems (atmospherics ambient + gravity_g + surface terrain template + ore deposits + weather table) read from the World record; WORLD-A and ASTRO-A acceptance suites pass."
feeds:
  - DR-002
  - DR-005
  - DR-007
  - DR-013
  - DR-014
  - DR-016
  - DR-017
  - DR-024
  - DR-027
  - DR-029
  - DR-031
  - DR-033
  - DR-034
  - DR-035
  - DR-036
  - DR-037
  - DR-038
  - DR-039
---

← [[index|vault home]] · [[spec/index|spec section]] · [[spec/atmospherics-and-chemistry-model|atmospherics/chemistry]] · [[spec/gravity-and-ballistics-model|gravity/ballistics]] · [[spec/environmental-conditions-model|environmental conditions]] · [[spec/origin-reaction-and-resource-model|origin reaction/resource]] · [[spec/mining-and-extraction-model|mining/extraction]] · [[spec/setting-and-world-frame|setting/world frame]] · [[spec/prototype-roadmap|native roadmap]] · [[spec/native-implementation-backlog|native backlog]] · [[decisions/dr-016-setting-and-world-frame|DR-016]] · [[decisions/dr-039-celestial-bodies-and-worlds-direction|DR-039]]

# Celestial Bodies And Worlds Model

> [!summary] What this page is
> The canonical `World` schema. Every subsystem that asks "where am I?", "what's the gravity here?", "what's the atmosphere?", "what time of day is it?", "what's the comms latency to my home base?", "what ore is in this dirt?", "what weather can hit me here?" reads from the World record. The launch catalog is 12 representative bodies (10 named + 2 representative classes). Modders add new worlds via data row.
>
> Mars is currently defined in three places independently (atmospherics ambient table, gravity per-planet table, chassis matrix). This page consolidates that into ONE record and the other pages cross-link.

> [!warning] Authority boundary
> Captured 2026-05-06 as **design intent**. The schema (which fields a world declares, how subsystems read them, the orbital math) is a commitment. Specific numeric tuning (per-world ambient values, orbital phase epoch, ore abundance) stays open until prototype evidence backs them.

> [!important] Out of scope right now
> M0 is closed. M1 is the active milestone. **M0/M1/M2/M3/M4 remain world-config-only.** A scenario manifest may carry a `world_id` field as a placeholder; behavior must be identity-no-op until the World loader lands at extended M2 + M5.9 + new M5.10 + new M7.7.

## Why This Page Exists

Today, "Mars" is duplicated across:

- `spec/atmospherics-and-chemistry-model.md` § Planetary Atmospheres (Mars: 2-3 kPa, 95% CO2, ...)
- `spec/gravity-and-ballistics-model.md` § Locked Per-Planet Defaults (Mars: gravity_g=0.378)
- `spec/origin-reaction-and-resource-model.md` § Environment Resistance Matrix (vacuum/oxygen mention)

Adding "asteroid" or "Phobos" or a modder's "PlanetX" today means editing 3+ spec files. Subtle drift is inevitable. This page locks **one record** that all subsystems read from.

## Principles (locked)

1. **One record per world.** A `World` declares everything subsystems need: classification, orbital data, gravity, atmosphere ambient, surface terrain template, day length, ore deposits, weather variation table, lore, visual palette.
2. **One source of truth.** No subsystem stores its own copy of world-level data. They query `Worlds::get(world_id)`.
3. **Modder-extensible.** New worlds are data rows in `content/worlds/`. Schema validates via `cargo run -p cf-mod -- validate content/worlds/`.
4. **Deterministic orbital math.** Same scenario time + same World records → same body positions + same distances + same comms-latency values, byte-identical across replay.
5. **Game-friendly approximations.** Simplified Keplerian (parent + semi-major axis + orbital period + phase + rotation period + axial tilt) — not full 6-element orbital elements. Enough to compute position, distance, and light-lag; not enough to predict eclipses to NASA accuracy.
6. **Per-shard catalog.** MMO shards (DR-035) declare which subset of worlds they host; cross-shard travel is a portal, not a seamless transition.
7. **No FTL travel system in launch scope.** Comms latency exists; player travel between worlds is mission-scripted (dropship / deployment) rather than free-roam. A free-roam travel layer is post-launch scope (open).

## World Schema

```text
struct World {
    // Identity
    id: WorldId,                                    // stable; modder-owned namespace
    classification: Classification,                  // Planet | Moon | Asteroid | Sun | Station | Anomaly
    display_name: String,
    parent: Option<WorldId>,                        // Sol for planets; Mars for Phobos; Sol for asteroid belt; null for Sol itself

    // Astrography (simplified Keplerian)
    astro: Astrography {
        semi_major_axis_au: f64,                    // distance from parent in AU
        orbital_period_days: f64,                   // sidereal period
        mean_anomaly_at_epoch_rad: f64,             // phase at game-time T=0 (deterministic; stored per world)
        rotation_period_seconds: f64,               // sidereal day length
        axial_tilt_deg: f64,                        // for solar exposure / day-night band
        epoch_utc_iso: String,                      // anchor moment for the orbital phase (e.g. "2026-01-01T00:00:00Z")
    },

    // Surface
    surface: SurfaceProfile {
        gravity_g: f32,                             // multiplier of Earth standard 9.81 m/s² (0.0 = zero-g; negative = reverse-g cinematic)
        atmosphere_ambient: Option<AtmosphereAmbient>, // null for vacuum bodies
        surface_template: String,                   // terrain generator preset id; resolves at scenario load
        day_length_seconds: f64,                    // SOLAR day (rotation period + orbital correction; for tidally-locked bodies = orbital_period)
        temperature_range_k: (f32, f32),           // diurnal min..max at equator, no latitude / weather modifier
        magnetic_field_microtesla: f32,             // ambient EM; affects compass / sensor noise / EMP radius
        radiation_ambient_msvph: f32,               // baseline radiation in millisieverts/hour at surface
    },

    // Resources
    ore_deposits: Vec<OreDepositEntry>,             // per-ore abundance + depth band; M5.6 / M8.6 mining

    // Weather
    weather: WeatherProfile {
        variation_table: Vec<WeatherEventRef>,      // event ids + frequency + intensity range (per M7.7)
        baseline_wind_mps: (f32, f32),              // diurnal min..max at surface
        baseline_visibility_m: (f32, f32),          // clear..hazy
    },

    // Lore + presentation
    lore: LoreTags {
        tags: Vec<String>,                          // "frontier", "salvage_yards", "old_corp_ruins"
        name_origin: String,                        // canonical / fictional / modder-supplied
    },
    visual_palette: String,                         // "mars_rust", "europa_cyan", "vulcan_ember", ...; resolves to render presets

    // Provenance
    canonical: bool,                                // true for launch set; false for modder
    package_source: PackageRef,                     // mod author + version
}
```

Sub-types:

```text
enum Classification {
    Planet,
    Moon,
    Asteroid,           // small body; sub-categories via lore tags (belt, near-Earth, captured, etc.)
    Sun,                // star; "surface" is the corona for narrative purposes only — non-landable
    Station,            // artificial; orbital or Lagrange-point; can be anywhere; gravity from rotation
    Anomaly,            // wormhole, derelict, drifting hulk; behavior may break the orbital model
}

struct AtmosphereAmbient {
    pressure_kpa: (f32, f32),                       // diurnal min..max
    temperature_k: (f32, f32),                       // diurnal min..max (matches surface.temperature_range_k typically)
    composition: HashMap<GasId, f32>,               // mole fractions; sums to 1.0
}

struct OreDepositEntry {
    ore: OreId,                                     // see [[spec/mining-and-extraction-model]]
    abundance: f32,                                 // 0.0..1.0; relative density at the named depth band
    depth_band: DepthBand,                          // Surface | Subsurface | DeepCrust | Core
    distribution: DistributionShape,                // Uniform | Veined | Pocketed | Streak
}
```

## Orbital Math (Locked Form)

For each world `w` with parent `p`, position at game-time `t` (seconds since epoch) is:

```text
ω = 2π / orbital_period_seconds                      // mean angular velocity
M(t) = mean_anomaly_at_epoch_rad + ω · (t - t_epoch) // mean anomaly at time t
ν(t) ≈ M(t)                                          // simplified: assume circular orbits at launch (e=0)
r = semi_major_axis_au · AU_TO_M                     // constant radius (circular approximation)
position_in_parent_frame(t) = (r · cos(ν(t)), r · sin(ν(t)) · cos(axial_tilt_deg))
```

Position in heliocentric frame is computed by recursing through the parent chain (a moon's heliocentric position = parent_planet's heliocentric position + moon's position-in-parent-frame).

Distance between any two bodies `a` and `b` at time `t`:

```text
d_ab(t) = |position_heliocentric(a, t) - position_heliocentric(b, t)|     // meters
```

Comms latency between any two bodies (one-way; round-trip = 2x):

```text
comms_latency_seconds(a, b, t) = d_ab(t) / SPEED_OF_LIGHT
```

`SPEED_OF_LIGHT = 299_792_458` m/s. `AU_TO_M = 149_597_870_700` m.

Earth↔Mars round trip varies between ~6 minutes (closest approach) and ~44 minutes (opposition through the sun). The kernel computes this on demand; mission scripts query it at scenario load AND at significant transitions (mission start, comms-event triggers).

> [!info] Why circular approximation
> Real Keplerian elliptical math (eccentric anomaly via Kepler's equation, true anomaly conversion, etc.) is solvable but adds compute + per-tick numerical iteration. Launch scope is **circular** orbits with constant `r`; this gives 95% of the "Mars is sometimes far, sometimes near" feel without the iterative solver. A future "elliptical mode" toggle can be added per world via an `eccentricity` field if needed; the schema reserves space.

## Launch World Catalog

Twelve worlds at launch. Two are representative classes (any specific asteroid or station inherits from them via data rows).

| `id` | Classification | Parent | Gravity_g | Atmosphere | Day length | Period (days) | Notes |
|---|---|---|---:|---|---:|---:|---|
| `sol` | Sun | (none) | n/a (28 g, non-landable) | corona only | n/a | (center) | Light source; solar flares; comms anchor for inner-planet latency baseline. |
| `earth` | Planet | `sol` | 1.000 | 101 kPa, 75% N2 / 25% O2, 0-40 °C | 86,400 s | 365.25 | Default habitable. Tutorial/training scenarios. |
| `earth_moon` | Moon | `earth` | 0.166 | vacuum | 2,360,591 s (sidereal) | 27.32 | Long jumps; low fall damage; tidally locked. |
| `mars` | Planet | `sol` | 0.378 | 2-3 kPa, 95% CO2, 220-292 K | 88,775 s | 686.97 | Frontier; salvage yards; corp ruins. Suit required. |
| `phobos` | Moon | `mars` | 0.00057 | vacuum | 27,553 s (tidally locked) | 0.319 | Practically zero-g asteroid-like; mining target. |
| `deimos` | Moon | `mars` | 0.0003 | vacuum | 109,123 s | 1.263 | Even smaller; observation post. |
| `europa` | Moon | `jupiter` (proxy: see note) | 0.134 | 44-47 kPa, 100% N2, 124-133 K | 306,720 s | 3.55 | Cold N2 ambient; cryogenic medium; ice surface. |
| `mimas` | Moon | `saturn` (proxy: see note) | 0.0064 | vacuum | 81,216 s | 0.94 | Microgravity; Death-Star-shaped; close to Saturn. |
| `vulcan` | Planet | `sol` | 0.910 | 24-56 kPa, 53% Volatiles / 26% Pollutant / 21% N2, 400-938 K | 95,000 s | (fictional, ~70d) | Hot oxidizing fictional inner planet; furnace medium; constantly autoignites with O2. |
| `venus` | Planet | `sol` | 0.904 | 239 kPa, 93.1% CO2 / 6.9% N2, 737 K | 10,087,200 s (retrograde) | 224.7 | Crushing pressure; insulated hardsuits only. |
| `belt_asteroid` (representative) | Asteroid | `sol` (≈2.7 AU) | 0.001 | vacuum | varies (rotational) | varies | Mining target class. Specific asteroid worlds inherit + override. |
| `orbital_station` (representative) | Station | (any parent) | varies (centripetal artificial gravity OR zero-g zones) | per-station | per-station | n/a | Artificial. Defenders' bunker scenarios; spinning rings; dock + airlock + module assembly. |

> [!note] Jupiter and Saturn as proxies
> Europa is currently parented to a placeholder `jupiter` World; Mimas to a placeholder `saturn`. Jupiter and Saturn don't appear in the playable launch set (they're gas giants without surfaces) but they exist as parent-only bodies for orbital math. Optional: ship them as `Anomaly` or as gas-giant atmospheric platforms if a scenario calls for it.

## Per-World Data: Worked Example (Mars)

```text
World {
    id: "mars",
    classification: Planet,
    display_name: "Mars",
    parent: Some("sol"),

    astro: Astrography {
        semi_major_axis_au: 1.524,
        orbital_period_days: 686.97,
        mean_anomaly_at_epoch_rad: 0.0,            // canonical; modders override
        rotation_period_seconds: 88_642.66,        // sidereal day
        axial_tilt_deg: 25.19,
        epoch_utc_iso: "2026-01-01T00:00:00Z",
    },

    surface: SurfaceProfile {
        gravity_g: 0.378,
        atmosphere_ambient: Some(AtmosphereAmbient {
            pressure_kpa: (2.0, 3.0),
            temperature_k: (220.0, 292.0),
            composition: { co2: 0.95, n2: 0.03, o2: 0.01, pollutant: 0.01 },
        }),
        surface_template: "mars_oxidized_basalt",
        day_length_seconds: 88_775.0,              // solar day (rotation + orbital correction)
        temperature_range_k: (220.0, 292.0),
        magnetic_field_microtesla: 0.5,            // weak; no global dynamo
        radiation_ambient_msvph: 0.022,            // surface; hardsuit-tolerable
    },

    ore_deposits: [
        OreDepositEntry { ore: "iron",            abundance: 0.40, depth_band: Subsurface, distribution: Veined },
        OreDepositEntry { ore: "silica",          abundance: 0.25, depth_band: Surface,    distribution: Uniform },
        OreDepositEntry { ore: "ice_volatiles",   abundance: 0.05, depth_band: DeepCrust,  distribution: Pocketed },
        OreDepositEntry { ore: "nickel",          abundance: 0.05, depth_band: Subsurface, distribution: Veined },
        OreDepositEntry { ore: "perchlorate",     abundance: 0.10, depth_band: Surface,    distribution: Streak },
    ],

    weather: WeatherProfile {
        variation_table: ["mars_dust_storm", "mars_local_dust_devil", "mars_thermal_inversion"],
        baseline_wind_mps: (2.0, 12.0),
        baseline_visibility_m: (5_000.0, 30_000.0),
    },

    lore: LoreTags {
        tags: ["frontier", "salvage_yards", "old_corp_ruins", "perchlorate_dust_hazard"],
        name_origin: "canonical",
    },
    visual_palette: "mars_rust",
    canonical: true,
    package_source: { mod_id: "core", version: "v1.0.0" },
}
```

## What Reads From The World Record

| Subsystem | What it reads | Source spec |
|---|---|---|
| Atmospherics kernel | `surface.atmosphere_ambient` (composition, pressure range, temperature range) | [[spec/atmospherics-and-chemistry-model]] |
| Gravity field | `surface.gravity_g` | [[spec/gravity-and-ballistics-model]] |
| Material kernel terrain generator | `surface.surface_template` | [[decisions/dr-036-systemic-material-simulation-direction]], M5.6 |
| Mining | `ore_deposits` | [[spec/mining-and-extraction-model]], M8.6 |
| Mission director | `astro.day_length_seconds`, weather variation table, parent chain | [[spec/mission-director-slice-a]] |
| Day/night kernel | `astro.rotation_period_seconds`, `astro.axial_tilt_deg`, `astro.semi_major_axis_au` | [[spec/environmental-conditions-model]], M7.7 |
| Weather kernel | `weather.variation_table`, `weather.baseline_wind_mps`, `weather.baseline_visibility_m` | M7.7 |
| Comms latency | `astro` recursive parent chain | This page; queried by mission director + AI commander adaptation |
| EnvironmentSignal aggregator | All of the above | [[spec/environmental-conditions-model]], M5.10 |
| Server / MMO shards | Subset declared by shard config; per-shard catalog | [[spec/server-app-architecture]], [[spec/persistent-mmo-architecture]] |
| Scenario manifest | `world_id` field references this catalog | [[spec/mission-director-slice-a]] |
| Visual / render | `visual_palette` resolves to render preset | [[spec/visual-direction]] |
| AI doctrine | All of the above (commander adapts per-world) | [[spec/native-implementation-backlog#M6.6 — AI Material Competence]] (promoted to AI Environmental Competence) |

## Run-Bundle Event Family Extensions

Add `world` and `astrography` event categories per [[references/prototype-run-bundle-schema]].

| Event Type | Required Fields | Notes |
|---|---|---|
| `world.loaded` | scenario_id, world_id, world_record_hash, package_source | Fires on scenario load. |
| `world.parent_chain_resolved` | world_id, [parent_chain] | One-shot at scenario load. |
| `astrography.tick` | t, body_positions: { world_id: (x_au, y_au, z_au) }, sparse | Sparse cadence (e.g., once per minute of game time). |
| `astrography.distance_changed` | from_world_id, to_world_id, old_distance_m, new_distance_m, dt_seconds | Triggers when crossing thresholds (closest approach, opposition, etc.). |
| `astrography.comms_latency_changed` | from_world_id, to_world_id, old_latency_s, new_latency_s | Same threshold rule as distance. |
| `astrography.eclipse` | occluder_world_id, occluded_world_id, observer_world_id, t_start, duration_s | Optional; fires when an alignment event matters. |

## Comms Latency: Gameplay Implications

Two flavors:

1. **Player ↔ AI / commander ↔ remote asset.** A player on Earth coordinating with a Mars-deployed squad has a 6-44 minute round-trip light-lag for any synchronous command. This is **gameplay flavor**, not real-time blocking — orders are queued, AI executes locally, debriefs report what happened with timestamps. Mission director can author this as "the player has full control with 0 lag" (campaign-friendly) OR "orders carry a real lag" (immersive).

2. **PvP / MMO cross-body.** Public PvP arenas (DR-005, DR-034, DR-035) on cross-body shards may apply comms latency as a real penalty per [[spec/persistent-mmo-architecture]]. This is OPEN: M12 evidence required before locking. Default is "co-located arena = no comms lag; cross-body raid = lag applies".

The mission director ([[spec/mission-director-slice-a]]) declares the comms-latency policy per scenario:

```text
mission.comms_policy: { earth_anchored | local_authority | full_realtime | scripted_lag_band }
```

## Acceptance Tests (WORLD-A + ASTRO-A)

| Test | Setup | Pass Condition |
|---|---|---|
| WORLD-A-01 | Load `mars.world.ron` via the World loader. | Atmospherics kernel reads `composition` from World; gravity field reads `gravity_g` from World; material kernel reads `surface_template` from World; mining reads `ore_deposits` from World. No subsystem holds a stale duplicate. |
| WORLD-A-02 | Modder authors a custom planet `frostlight.world.ron` with novel ore + composition. | `cf-mod validate content/worlds/frostlight.world.ron --strict` passes; scenario referencing `frostlight` loads; subsystems read modder values without engine-side hardcodes. |
| WORLD-A-03 | Scenario references a missing world id. | Loader returns structured rejection (`world_not_found`) with parent chain; mission director shows a clear failure label. |
| WORLD-A-04 | World has invalid orbital data (period=0). | Schema rejects with diagnostics. |
| ASTRO-A-01 | Compute Earth-Mars distance at game-time t=0 with canonical epoch. | Distance is within Sol-system reasonable bounds (closest approach ≈0.5 AU; opposition ≈2.5 AU). Replay records same value byte-identically across runs. |
| ASTRO-A-02 | Compute comms latency Earth-Mars at game-time t=0 + 100 days. | Latency in plausible range (3-22 minutes one-way). Determinism replay byte-identical. |
| ASTRO-A-03 | Tidal-lock check: rotation_period == orbital_period for Phobos and Earth's Moon. | Day-length kernel reports tidal-locked face. |
| ASTRO-A-04 | Position recursion for Phobos. | Phobos heliocentric position = Mars heliocentric + Phobos-in-Mars-frame; verified at multiple t. |
| ASTRO-A-05 | Determinism replay across mixed-world mission. | 10000 ticks; same seed; byte-identical event stream including all `astrography.*` events. |

## AI Doctrine Implications

Per [[spec/native-implementation-backlog#M6.6 — AI Material Competence]] (promoted to **AI Environmental Competence** under DR-040):

- AI commander adapts doctrine per-world. "Mars dust storm reduces visibility → switch to thermal optics." "Vulcan fuel-rich atmosphere → no muzzle flashes." "Mimas microgravity → grenades have hour-long apex; switch to direct-fire." "Earth's Moon vacuum → close helmets before the patrol."
- AI miner reads `ore_deposits` for prospect selection; AI doctor reads `surface.atmosphere_ambient` to anticipate hypoxia risk; AI scout reads `weather.baseline_visibility_m` to route around dust pockets.
- AI commander memory persists across world transitions (campaign mode): "the player favored stealth on Mars; preserve that posture on Phobos".

## Modding Contract

- Add a new world: data row in `content/worlds/<id>.world.ron` with all required fields. Schema validates on load.
- Required fields enforced: id, classification, parent (or null for Sol), astro, surface, weather (may be empty list), lore (tags may be empty), visual_palette (must resolve), canonical=false, package_source.
- Optional fields: ore_deposits (empty list = no minable ore), magnetic_field_microtesla (default 0), radiation_ambient_msvph (default 0).
- Editor UI ([[spec/native-implementation-backlog#M8 — Scenario Editor And Mod Tools]] M8) provides a world-authoring form + validation + visual preview.

## Performance Posture

- World records load once at scenario-start; held in immutable `Worlds` registry for the run.
- Per-tick orbital position is a closed-form computation; cached per-tick across all queries.
- Comms latency is computed on demand (rare; mission events).
- Determinism: orbital math uses f64; same seed + same epoch + same world record = byte-identical positions.

## Out Of Scope (during M0..M4)

- M0/M1: scenario manifest may declare `world_id` as a placeholder string; loader is no-op until the World loader lands at extended M2.
- M2 (Pixel Terrain And Materials): per-world surface template hook lands; full World loader optional but recommended.
- M3 (Replay): event categories `world` + `astrography` are defined in `prototype-run-bundle-schema.md` but only `world.loaded` fires (one-shot at scenario start).
- M4 (HUD): HUD has reserved fields for "current world" name + `g_factor` + day-night band; they show placeholder values until M5.10.
- Free-roam multi-world travel (player flies a ship between bodies in real time): post-launch; OPEN.
- Real Kepler-elliptical orbits with eccentricity solver: post-launch toggle; OPEN.
- Real-life astronomical N-body perturbations: out of scope forever.

## Source Trail

- [[spec/atmospherics-and-chemistry-model]]
- [[spec/gravity-and-ballistics-model]]
- [[spec/environmental-conditions-model]]
- [[spec/origin-reaction-and-resource-model]]
- [[spec/mining-and-extraction-model]]
- [[spec/mission-director-slice-a]]
- [[spec/setting-and-world-frame]]
- [[spec/full-collision-physics-plan]]
- [[spec/server-app-architecture]]
- [[spec/persistent-mmo-architecture]]
- [[references/prototype-run-bundle-schema]]
- [[decisions/dr-016-setting-and-world-frame]]
- [[decisions/dr-035-persistent-mmo-architecture]]
- [[decisions/dr-036-systemic-material-simulation-direction]]
- [[decisions/dr-037-stationeers-grade-atmospherics-direction]]
- [[decisions/dr-038-universal-gravity-and-ballistics-direction]]
- [[decisions/dr-039-celestial-bodies-and-worlds-direction]]
- [[research-log/2026-05-06-celestial-bodies-environments-mining-bunker-defence-design-intent]]

## Change Log

- 2026-05-06: Captured during M1 from user-supplied design intent ("the engine to support different planets, asteroids, moons, suns, current planet, different gravities based on environment"). User chose **full astrography with orbital math + comms latency** over lighter alternatives. Status: `design-intent-post-m1`. 12-world launch catalog locked. Simplified circular Keplerian orbital math. Lands at extended M2 + new M5.10 + new M7.7. Atmospherics + gravity per-planet tables become cross-links to this page.
