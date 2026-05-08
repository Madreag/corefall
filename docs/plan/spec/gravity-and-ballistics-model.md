---
type: spec
status: design-intent-post-m1
authority: "Canonical contract for universal gravity field affecting materials, projectiles, actors, equipment items, debris, gases, and liquids. Per-planet gravity values; per-cell override capability for special zones (gravity wells, low-g labs, magnetic boots); ballistic trajectory math integrated with M5.5 collision physics, M5.6 material kernel, and atmospherics density layering. Captured during M1; lands at extended M5.5/M5.6/M5.9. M0 and M1 must remain gravity-config-driven (no hardcoded g)."
ready_when: "Universal gravity reads through one source (cf-physics::gravity); per-planet ambient g loads from scenario manifest; ballistic trajectories deterministic and reproducible; material kernel density layering reads g per-cell; GRAV-A acceptance suite passes."
feeds:
  - DR-002
  - DR-003
  - DR-005
  - DR-007
  - DR-008
  - DR-018
  - DR-021
  - DR-024
  - DR-027
  - DR-033
  - DR-036
  - DR-038
---

← [[index|vault home]] · [[spec/index|spec section]] · [[spec/full-collision-physics-plan|full collision plan]] · [[spec/atmospherics-and-chemistry-model|atmospherics/chemistry]] · [[spec/origin-reaction-and-resource-model|origin reaction/resource]] · [[spec/body-damage-model|body damage]] · [[spec/equipment-loadout|equipment]] · [[spec/prototype-roadmap|native roadmap]] · [[spec/native-implementation-backlog|native backlog]] · [[decisions/dr-007-terrain-material-model|DR-007]] · [[decisions/dr-033-full-collision-physics-direction|DR-033]] · [[decisions/dr-036-systemic-material-simulation-direction|DR-036]] · [[decisions/dr-038-universal-gravity-and-ballistics-direction|DR-038]]

# Gravity And Ballistics Model

> [!summary] What this page is
> Universal gravity is one of the foundation physics pillars of Corefall, alongside full collision ([[spec/full-collision-physics-plan]]), systemic materials ([[decisions/dr-036-systemic-material-simulation-direction]]), and Stationeers-grade atmospherics ([[spec/atmospherics-and-chemistry-model]]). Gravity affects EVERY simulated thing: materials, projectiles, actors, equipment items, debris, gases (density layering), liquids (settling), pieces of broken structures, dropped weapons, leaked coolant, even ejected casings. Per-planet ambient g; per-cell override for special zones; deterministic ballistics integrated with collision and atmospherics.
>
> One source of truth: `cf-physics::gravity::GravityField`. Every other system reads from it. No system computes its own gravity.

> [!warning] Authority boundary
> Captured 2026-05-06 as **design intent**. The model (universal field, per-planet config, per-cell override, ballistic integration) is a commitment. Specific numeric tuning (per-planet g values, per-projectile drag, terminal velocity) stays open until GRAV-A prototype evidence backs them.

> [!important] Out of scope right now
> BP1 is closed and BP2 is active. **Nothing on this page is implemented in M0/M1/M1.5/M2/M3A/M3B/M4A/M4B beyond placeholder `gravity_g` manifest/config fields** (which can stay inert until M5.5 collision lands). Ballistic trajectory math, density-layering coupling, and per-cell overrides land across M5.5 + M5.6 + M5.9.

## Why This Page Exists

The roadmap commits to full collision physics (M5.5) and systemic materials (M5.6+). Both implicitly need gravity. Without locking gravity as a single universal field that every system reads from:

- Different subsystems will compute their own gravity. The actor controller will hardcode `9.8 m/s²`; the projectile system will use a tuned arcade arc; the debris system will use a separate spline. The first time a per-cell low-g zone is added (e.g., a damaged grav generator), three subsystems break in three different ways.
- Per-planet scenarios become a re-tune fest. Mars-scenario players notice their grenades arc the same as Earth-scenario players because grenades don't read the planet g.
- Density layering in materials (oil floats on water; smoke rises) and atmospherics (CO2 sinks; H2 rises) becomes inconsistent with object physics if they use a different gravity source.
- Net code can't snapshot "gravity at this moment in this cell" because it's not a single field.

This page locks the universal field. Other pages cross-link here.

## Principles (locked)

1. **Gravity is a field, not a constant.** Stored as `cf_physics::gravity::GravityField` keyed by world position; defaults to per-planet ambient; can be overridden per-cell or per-region by gameplay (gravity well, low-g lab, ship interior with own grav generator).
2. **One source of truth.** Every other system queries `GravityField::sample(pos)` to get the local `(direction, magnitude)`. No subsystem computes its own gravity.
3. **Affects EVERYTHING.** Materials (oil floats on water; sand settles), gases (CO2 sinks; H2 rises — interacts with atmospherics density layering), projectiles (ballistic arcs), actors (fall damage, jump height, walking gait), equipment items (dropped weapons fall), debris (gibs, casings, ejected magazines), broken structure fragments, liquids in pipes (with appropriate kernel hooks), coolant/oil leaks (pool below the leak source), even visual effects (sparks fall, smoke rises).
4. **Per-planet ambient.** Each scenario manifest declares `gravity_g` (a multiplier of Earth standard, where 1.0g = 9.81 m/s² downward in the screen-y direction for a 2D side-view). Earth = 1.0; Moon = 0.166; Mars = 0.378; Vulcan = (per scenario); zero-g = 0.0; reverse-g = -1.0 (yes, that's allowed for the upside-down ship cinematic).
5. **Per-cell override allowed.** Special zones declare a per-cell or per-region delta or replacement (gravity well: +0.5g toward center point; low-g lab: 0.05g; magnetic boots: artificial 1.0g downward in the actor's local frame regardless of ambient). Per-cell overrides are time-stable until a gameplay event toggles them.
6. **Deterministic.** Same gravity field + same initial conditions = same trajectory, byte-for-byte across replay.
7. **Configurable, not hardcoded.** No `const GRAVITY_MS2: f32 = 9.81` anywhere in production code. Read from `GravityField`. Per [[spec/prototype-roadmap#No-Compromise Performance Defaults]] this is a hard rule.
8. **Net-friendly.** GravityField snapshot is small (per-region overrides + ambient). Server authoritative; clients receive override deltas.

## The Gravity Field

```rust
struct GravityField {
    ambient: GravityVec,                                // per-scenario default
    region_overrides: Vec<(RegionShape, GravityVec)>,   // local zones
    cell_overrides: SparseMap<CellId, GravityVec>,      // per-cell (rare)
}

struct GravityVec {
    direction: Vec2,    // unit vector; (0, -1) = down
    magnitude: f32,     // m/s²; Earth ambient default = 9.81
    source: GravitySource, // ambient | gravity_generator | grav_well | low_g_lab | magnetic_boots | scripted
}

impl GravityField {
    fn sample(&self, pos: Vec2) -> GravityVec { /* layered lookup: cell > region > ambient */ }
    fn ambient_g_factor(&self) -> f32 { self.ambient.magnitude / 9.81 }
}
```

`gravity_g` on the scenario manifest is a scalar multiplier of Earth's 9.81 m/s². Magnitude in the field is computed per `GravityVec`.

## Locked Per-Planet Defaults

| Scenario / World | `gravity_g` | Direction | Notes |
|---|---|---|---|
| Earth-like | 1.000 | (0, -1) | Default. |
| Moon-like | 0.166 | (0, -1) | Long jumps; low fall damage. |
| Mars-like | 0.378 | (0, -1) | Mid-low fall damage. |
| Mercury-like | 0.378 | (0, -1) | Same as Mars. |
| Europa-like | 0.134 | (0, -1) | Cold + low-g; cryogenic vacuum. |
| Mimas-like | 0.0064 | (0, -1) | Effectively microgravity; airlock cycling stays consistent but actors and items drift. |
| Vulcan-like | 0.910 | (0, -1) | Earth-similar; hot oxidizing atmosphere. |
| Venus-like | 0.904 | (0, -1) | Earth-similar; crushing pressure. |
| Zero-g lab | 0.0 | (0, 0) | Items drift; ballistics still take initial velocity. |
| Reverse-g chamber | 1.0 | (0, +1) | Cinematic / puzzle scenarios. |
| Custom planet (modder) | declared | declared | Per-scenario data row. |

Values are consistent with Stationeers' planet roster but reserve the right to tune for game feel during M5.5 GRAV-A acceptance testing.

## What Reads Gravity

Every system that has any physics-y behavior reads from `GravityField::sample(pos)`:

| System | What It Does With g |
|---|---|
| **Actor controller** ([[spec/native-implementation-backlog#M1 — Actor Controller And Sim Core]]) | Walking gait stride length scales with g; jump apex scales with g; fall acceleration = sample(pos).down × magnitude; fall damage threshold per [[spec/origin-reaction-and-resource-model]] depends on local g. |
| **Projectile system** ([[spec/native-implementation-backlog#M1 — Actor Controller And Sim Core]] M1-003 rifle loop, M5.5 collision) | Bullets follow ballistic arc per `pos += v·dt; v += g·dt - drag·v·dt²`; light rounds drop more visibly in low-pressure atmospheres (drag is gas-density-dependent — see Atmospherics Coupling below). |
| **Equipment / dropped items** | Dropped weapon falls at sampled g; resting state uses contact normal with surface; interacts with [[spec/full-collision-physics-plan]] M5.5-007 limb/equipment contacts. |
| **Debris / gibs / casings** | Same kinematics as dropped items; spawn velocity from impulse + g acceleration. |
| **Material kernel** ([[spec/native-implementation-backlog#M5.6 — Material Kernel]]) | Density layering: liquids settle by density × local g; gases stratify by density (CO2 sinks faster in higher g; H2 rises). Sand/pebble flow uses g for free-fall component. |
| **Atmospherics density layering** ([[spec/atmospherics-and-chemistry-model]]) | When sealed-room atmosphere has multiple gases at significantly different molar masses, kernel applies per-tick partial-pressure stratification proportional to local g. CO2 (44 g/mol) sinks; H2 (2 g/mol) rises; O2 (16 g/mol) middle. |
| **Body damage / fall damage** ([[spec/body-damage-model]]) | Fall damage threshold = base_threshold × local_g_factor; origin gates per [[spec/origin-reaction-and-resource-model]] still apply. |
| **Liquid pipes / tanks** | Gravity-driven flow when an active or passive vent connects pipes to ambient; pumps work against g. |
| **Visual effects** | Sparks, ember particles, leaked coolant droplets — all read g for consistency with the rest of the sim. |
| **AI doctrine** ([[spec/native-implementation-backlog#M6 — AI Core And Trust Harness]]) | AI plans jumps/falls/grenade arcs against the sampled g; avoids walking into low-g zones with heavy equipment unless mission requires. |
| **Net code** ([[spec/native-implementation-backlog#M11 — Online Co-op (Self-Hosted Dedicated Servers)]]) | Server authoritative on `GravityField`; clients receive ambient + override deltas; ballistics deterministic across server + clients with same g. |

## Ballistic Trajectory Math

For a projectile with mass `m`, initial velocity `v_0`, and current position `p`:

```text
F_gravity      = m · g(p)              # vector, sampled from field
F_drag         = -0.5 · ρ_local · v · |v| · C_d · A   # gas-density-dependent
F_collision    = collision impulses from M5.5 contact solver

a              = (F_gravity + F_drag + F_collision) / m
v              = v + a · dt
p              = p + v · dt
```

Where `ρ_local` is local atmosphere mass density, computed from atmospherics: `ρ = sum(n_g · M_g) / V` (kg/L; convert to kg/m³).

In vacuum (Mars exterior, Moon, etc.), `ρ_local ≈ 0`, so drag is near-zero — projectiles fly farther and follow purer parabolic arcs.

In dense atmospheres (Venus 239 kPa CO2), drag is significant — heavy projectiles tunnel through; light projectiles tumble or stop. This is gameplay flavor: a sniper rifle on Venus has very different range than on Mars.

`C_d × A` (drag coefficient × cross-sectional area) is per-projectile data; tunable per item.

## Atmospherics Coupling

Atmospheric density layering reads g per cell:

- **Liquid layering** (per [[spec/atmospherics-and-chemistry-model]] phase change): when a liquid co-exists with a gas, the kernel sorts by liquid density. Oil floats on water under positive g; sinks under reverse g.
- **Gas stratification** in sealed rooms: per-tick partial-pressure adjustment proportional to (g_factor × molar_mass_difference / mean_molar_mass × dt). At 1g, CO2 sinks toward floor; H2 rises toward ceiling. At 0g, no stratification (uniform mix). At reverse g, the stratification flips. The kernel emits `atmospherics.gas_stratified` events with per-cell partial-pressure deltas so the HUD and replay can visualize the gas layers.
- **Wind from pressure differentials** still operates regardless of g — wind is from ΔP, not from g — but in low-g zones the wind force on actors and items is more relative to inertia than to weight, which feels different to the player.

## Per-Cell Override Examples (Gameplay)

Locked-form behaviors that ride on the per-cell override system. Every override is data-driven and emits a replay event when activated/deactivated.

| Override Class | Mechanism | Example Use |
|---|---|---|
| **Gravity Generator** (base module) | Sets a region inside the base to ambient + 1g down regardless of planet. | Habitable Mars base; player walks at 1g indoors despite 0.378g outside. |
| **Gravity Well** (anomaly / special weapon) | Per-cell vector pointing toward a center point at scaled magnitude. | Singularity grenade pulls debris and actors toward the impact point. |
| **Low-g Lab** (modular base section) | Per-region 0.1g down. | Construction / assembly / weightless storage. |
| **Magnetic Boots** (item) | Per-actor override: 1g toward the surface normal in the actor's frame, regardless of ambient. | EVA on a rotating asteroid; actor sticks to the rotating surface. |
| **Damaged Gravity Generator** | Region override toggles intermittently between configured g and 0g. | Mission consequence; player must navigate flickering gravity. |
| **Reverse-g Chamber** (puzzle / cinematic) | Per-region (0, +1) at configured magnitude. | One-off rooms; not a launch promise. |

Active overrides are first-class state and replicate via net code.

## Replay / Run Evidence Events

Add `gravity` and `ballistics` event categories. All payloads include `parent_event_id` chains where applicable.

| Event Type | Required Fields |
|---|---|
| `gravity.field_changed` | region_id, old_vec, new_vec, source, parent_event_id |
| `gravity.override_activated` | region_or_cell_id, override_class, vec, parent_event_id |
| `gravity.override_deactivated` | region_or_cell_id, parent_event_id |
| `gravity.entity_entered_region` | entity_id, region_id, vec, parent_event_id |
| `gravity.entity_exited_region` | entity_id, region_id, parent_event_id |
| `ballistics.projectile_launched` | projectile_id, owner_id, weapon_id, p_0, v_0, mass, drag_coef, parent_event_id |
| `ballistics.projectile_step` | projectile_id, p, v, a (sparse — only every N ticks) |
| `ballistics.projectile_terminated` | projectile_id, reason (impact / fuse_expired / despawn / fragmented), parent_event_id |
| `ballistics.terminal_velocity_reached` | entity_id, v_terminal, ρ_local, source |
| `ballistics.fall_damage_threshold_crossed` | actor_id, fall_distance_m, impact_v_mps, local_g, threshold, parent_event_id |
| `atmospherics.gas_stratified` | atm_id, gas, top_pp, bottom_pp, layer_height_m, g_factor (cross-system; lives in atmospherics category but driven by gravity) |

## Acceptance Tests (GRAV-A)

| Test | Setup | Pass Condition |
|---|---|---|
| GRAV-A-01 | Drop test: actor falls from 10 m on Earth scenario (g=1.0) | Acceleration ≈ 9.81 m/s² down; impact velocity ≈ √(2·g·h) ≈ 14 m/s; fall damage threshold per [[spec/origin-reaction-and-resource-model]] applies. |
| GRAV-A-02 | Same drop on Mars scenario (g=0.378) | Acceleration ≈ 3.71 m/s² down; impact velocity scales accordingly; fall damage reduced per origin gate × local_g_factor. |
| GRAV-A-03 | Same drop on Moon (g=0.166) | As above; verify replay determinism: same seed + same actor input = byte-identical event stream. |
| GRAV-A-04 | Projectile arc test on Earth + on Mars | Same launch v_0 on both worlds → Mars projectile travels further in horizontal due to lower drop rate; `ballistics.projectile_step` events show proportional vertical acceleration. |
| GRAV-A-05 | Vacuum projectile range test (Moon, ρ=0) vs Earth (ρ=1.225 kg/m³) | Vacuum projectile travels farther (no drag); Earth projectile decelerates. |
| GRAV-A-06 | Low-g override region inside Earth scenario | Actor walks from 1g region → 0.1g region → 1g region; replay logs `gravity.entity_entered_region` and `_exited_region`; jump height triples in low-g region. |
| GRAV-A-07 | Magnetic boots on rotating asteroid | Actor's local "down" follows surface normal; replay shows per-actor override applied each tick. |
| GRAV-A-08 | Liquid layering in tank under 1g vs 0g | At 1g: oil floats on water (visible split). At 0g: oil and water remain mixed; no settling. Both deterministic across replay. |
| GRAV-A-09 | Gas stratification in sealed room | CO2 partial pressure higher at floor; H2 higher at ceiling at 1g. At 0g: uniform. Verifies atmospherics × gravity coupling. |
| GRAV-A-10 | Determinism replay test | Run a complex scenario with mixed gravity regions for 10000 ticks; replay produces byte-identical event stream and final atmosphere/projectile/actor state. |

## Modding Contract

- Add a new gravity override class: data row in `content/gravity_overrides/` with shape, vec, source, activation conditions, replay event class.
- Add a new planet ambient: scenario manifest field `gravity_g` per [[spec/atmospherics-and-chemistry-model#Planetary Atmospheres]].
- Add a new projectile drag profile: data row in `content/projectiles/` with `drag_coef`, `cross_section_m2`, mass, terminal velocity hint.
- Schema validates via `cargo run -p cf-mod -- validate content/gravity_overrides/`.

## Performance Posture

- `GravityField::sample(pos)` is hot-path; layered lookup must be cache-friendly. SoA storage; SIMD-friendly per-cell array.
- Per-projectile integration runs at fixed-tick alongside collision (M5.5 contact solver).
- Stratification kernel runs at sleeping-aware cadence — only sealed atmospheres with multi-gas mixes and significant ΔM run the per-tick stratification step.
- Determinism: integration order is fixed; no platform-specific atomics in the inner loop.

## Milestone Routing

- M5.5 introduces collision and impulse routing hooks that can sample local gravity.
- M5.6 material kernel uses gravity for density layering and settling.
- M5.9 atmospherics uses gravity for gas/liquid stratification and pressure-flow interaction.
- M5.10 environment aggregation exposes gravity slices to AI/HUD/replay through `EnvironmentSignal`.

## Out Of Scope (during M0..M4)

- M0/M1: scenario manifest carries `gravity_g` field but engine ignores it (placeholder).
- M2: terrain materials don't read g yet; M5.6 owns the integration.
- M3: replay records the placeholder field but `gravity.*` events don't fire yet.
- M4: HUD shows local g_factor in the advanced panel only; no widget on default HUD.

## Source Trail

- [[spec/origin-reaction-and-resource-model]]
- [[spec/atmospherics-and-chemistry-model]]
- [[spec/full-collision-physics-plan]]
- [[spec/body-damage-model]]
- [[spec/equipment-loadout]]
- [[references/prototype-run-bundle-schema]]
- [[decisions/dr-007-terrain-material-model]]
- [[decisions/dr-033-full-collision-physics-direction]]
- [[decisions/dr-036-systemic-material-simulation-direction]]
- [[decisions/dr-038-universal-gravity-and-ballistics-direction]]
- [[research-log/2026-05-06-origin-reaction-and-resource-design-intent]]
- [[research-log/2026-05-06-atmospherics-and-chemistry-stationeers-research]]

## Change Log

- 2026-05-06: Captured during M1 from user-supplied design intent ("gravity should be a thing that affects materials, bullets, entities, everything really"). Status: `design-intent-post-m1`. Universal gravity field; per-planet defaults locked; per-cell override schema defined; integration with M5.5 collision + M5.6 material kernel + M5.9 atmospherics density layering + ballistics math + replay event family extension.
