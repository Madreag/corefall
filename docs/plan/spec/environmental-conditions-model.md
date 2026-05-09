---
type: spec
status: design-intent-post-m1
authority: "Canonical contract for the EnvironmentSignal aggregation layer: per-tick per-actor bundled environmental signals (atmospheric, gravitational, thermal, radiation, photic, EM, weather, water, acoustic) computed once per tick from the underlying kernels and read by every consumer (AI, HUD, accessibility, replay, audio, mission director, server). One source of truth for 'what is the environment doing to this actor at this tick'. Lands at new M5.10 between M5.9 and M6."
ready_when: "EnvironmentSignal struct exists; kernels feed it deterministically; AI / HUD / accessibility / replay / audio all read from the bundle; ENV-A acceptance suite passes; M6.6 promoted to AI Environmental Competence consumes the bundle."
feeds:
  - DR-002
  - DR-003
  - DR-005
  - DR-007
  - DR-008
  - DR-012
  - DR-013
  - DR-014
  - DR-018
  - DR-022
  - DR-024
  - DR-027
  - DR-033
  - DR-034
  - DR-035
  - DR-036
  - DR-037
  - DR-038
  - DR-039
  - DR-040
---

← [[index|vault home]] · [[spec/index|spec section]] · [[spec/celestial-bodies-and-worlds-model|worlds catalog]] · [[spec/atmospherics-and-chemistry-model|atmospherics/chemistry]] · [[spec/gravity-and-ballistics-model|gravity/ballistics]] · [[spec/origin-reaction-and-resource-model|origin reaction/resource]] · [[spec/comms-voice-and-radio-model|comms/voice/radio]] · [[spec/full-collision-physics-plan|full collision plan]] · [[spec/prototype-roadmap|native roadmap]] · [[spec/native-implementation-backlog|native backlog]] · [[decisions/dr-040-environmental-conditions-and-hazards-direction|DR-040]]

# Environmental Conditions Model

> [!summary] What this page is
> The aggregation layer. Atmospherics (DR-037), gravity (DR-038), materials (DR-036), and the new world catalog (DR-039) each produce per-tick signals. Today, an AI agent that wants to know "is this actor in danger right now?" has to query 4-5 systems separately. Tomorrow, AI / HUD / accessibility / replay / audio / mission director all read from **one** `EnvironmentSignal` struct computed once per tick per actor.
>
> The struct is the seam between the kernels (which produce signals) and the consumers (which act on them). New environmental dimensions (weather, day/night, radiation, lighting, EM, acoustic) plug into the same seam.

> [!warning] Authority boundary
> Captured 2026-05-06 as **design intent**. The aggregation contract (which signals exist, who reads/writes them, the per-tick cadence) is a commitment. Specific tuning (signal magnitudes, hazard thresholds, AI affordance scoring) stays open until prototype evidence backs them.

> [!important] Out of scope right now
> M0..M4 stay environment-config-only. M5.5 + M5.6 + M5.7 + M5.9 land the signal-producing kernels. **M5.10 (NEW)** lands this aggregation layer. M6.6 promoted to AI Environmental Competence consumes it. M7.7 (NEW) lands weather + day/night + dynamic events that feed it.

## Why This Page Exists

The vault has signal-producing kernels:

- `cf-atmos` → atmospheric exposure (oxygen partial pressure, toxic gas, combustible mix, breach decompression)
- `cf-physics::gravity` → gravitational exposure (sample at pos)
- `cf-material` → material/contact exposure (acid, electrified water, thermal conduction from lava)
- `cf-control::worlds` → world-level signals (planetary ambient, surface template)

But there's no integration layer. Each consumer (AI doctrine, HUD widget, accessibility caption, replay event recorder, audio mixer, mission objective trigger) currently has to query each kernel independently, in some unspecified order, possibly getting stale views, possibly missing a signal that another system updated mid-tick.

This is the failure mode I want to eliminate forever: **"AI saw atmosphere clear at tick T but missed that the weather kernel started a dust storm at T-1, so it walked the squad into a hypoxia trap"**.

## Principles (locked)

1. **One bundle per actor per tick.** `EnvironmentSignal::for_actor(actor, tick)` is computed once at a deterministic point in the tick schedule (after all signal-producing kernels run, before any consumer runs).
2. **All consumers read from the bundle.** AI, HUD, accessibility, replay, audio, mission director, server. No consumer queries individual kernels for environmental data.
3. **Kernels write into the bundle through a typed adapter.** Each kernel exposes a `produce_signal(...)` adapter that writes its slice; the aggregator collects slices into one struct. No consumer reaches into the kernel.
4. **Replay records signal deltas, not full bundles per tick.** Bundle changes are events; static signals are snapshots. Bandwidth-friendly.
5. **Origin gating happens at the consumer, not the producer.** The bundle reports raw environment; origin-specific responses (humans concuss; robots don't) live in [[spec/origin-reaction-and-resource-model]].
6. **Modder-extensible.** New signals (e.g., a future "psionic field" mod) plug into the bundle via a typed extension.
7. **Per-tick performance bounded.** One per-actor bundle computation; SoA over all actors; SIMD-friendly. Sleeping actors skip.

## The EnvironmentSignal Struct

```text
struct EnvironmentSignal {
    // Identity
    actor_id: ActorId,
    tick: u64,
    world: WorldRef,                         // from [[spec/celestial-bodies-and-worlds-model]]
    pos: Vec2,                               // actor world position
    cell_id: CellId,                         // active-region grid cell

    // ATMOSPHERIC EXPOSURE (from cf-atmos kernel via DR-037)
    atmospheric: AtmosphericExposure {
        atm_kind: AtmKind,                    // RoomCell | PipeNetwork | Suit | Lung
        partial_pressure_pa: PerGas<f32>,    // O2, N2, CO2, Volatiles, etc.
        total_pressure_pa: f32,
        temperature_k: f32,
        composition_ratio: PerGas<f32>,
        hazards: Vec<AtmHazard>,             // hypoxic | combustible | toxic | extreme_temp | breach_decomp
        breach_eta_s: Option<f32>,           // Some(t) if a breach is detected and pressure is dropping
    },

    // GRAVITATIONAL EXPOSURE (from cf-physics::gravity via DR-038)
    gravitational: GravityVec {
        direction: Vec2,                      // unit vector
        magnitude_mps2: f32,                  // m/s²
        g_factor: f32,                        // magnitude / 9.81
        source: GravitySource,                // ambient | gravity_generator | grav_well | low_g_lab | magnetic_boots | scripted
    },

    // THERMAL EXPOSURE (from atmospherics + material kernel + radiation)
    thermal: ThermalExposure {
        ambient_k: f32,                       // local atmosphere temperature
        radiation_w_m2: f32,                  // solar + IR radiation flux
        conduction_w: f32,                    // contact-based heat flow (e.g., touching lava)
        wind_chill_modifier_k: f32,
        derived_band: ThermalBand,            // Frigid | Cold | Comfortable | Hot | Extreme
    },

    // RADIATION EXPOSURE (NEW per DR-040)
    radiation: RadiationExposure {
        solar_msvph: f32,                    // solar radiation; varies with world.solar_distance + atmosphere shielding
        cosmic_msvph: f32,                   // cosmic ray background
        reactor_msvph: f32,                  // local reactor / nuclear weapon source contribution
        ambient_msvph: f32,                  // world.surface.radiation_ambient_msvph baseline
        total_msvph: f32,                    // sum
        derived_dose_band: DoseBand,         // Safe | Mild | Hazardous | Critical
    },

    // PHOTIC EXPOSURE (NEW per DR-040)
    photic: PhoticExposure {
        lux: f32,                             // illuminance at actor pos
        spectrum: SpectrumBand,               // Visible | UV | IR | Mixed
        flicker_hz: f32,                      // 0 = steady; >0 = flickering source (solar flare, fire, malfunctioning light)
        derived_visibility_band: VisibilityBand, // Pitch | Dim | Lit | Bright | Glaring
    },

    // EM / MAGNETIC EXPOSURE (NEW per DR-040)
    em: EmExposure {
        magnetic_t: f32,                      // tesla; baseline from world.surface.magnetic_field_microtesla
        em_noise_db: f32,                     // background EM noise (storms, EMP, machinery)
        compass_deviation_rad: f32,           // for navigation
        em_emp_recently: bool,                // last EMP event within X seconds
    },

    // PRESSURE EXPOSURE (atmospheric — but exposed for clarity)
    pressure: PressureExposure {
        ambient_pa: f32,                      // matches atmospheric.total_pressure_pa typically
        suit_pa: Option<f32>,                 // if actor is sealed
        delta_pa_per_s: f32,                  // rate of change (breach detection)
    },

    // WEATHER EXPOSURE (from M7.7 weather kernel)
    weather: WeatherExposure {
        active_event: Option<WeatherEventRef>,
        intensity_0_1: f32,
        wind_mps: Vec2,                       // direction + magnitude; informs atmospherics + ballistics drag
        visibility_m: f32,                    // effective sight range modifier
        precipitation: Option<PrecipKind>,    // dust | rain | snow | acid | ash | meteor_fall
        eta_to_change_s: Option<f32>,
    },

    // WATER EXPOSURE (from cf-material kernel + body damage)
    water: WaterExposure {
        wetness_0_1: f32,
        submerged_depth_m: f32,               // 0 = surface; >0 = under liquid
        liquid_kind: Option<LiquidId>,        // water | acid | coolant | oil | blood
        is_drowning_hazard: bool,
    },

    // ACOUSTIC EXPOSURE (NEW per DR-043 voice/radio)
    acoustic: AcousticExposure {
        ambient_db: f32,                      // background sound level
        propagation_medium: PropagationMedium, // Air | RarefiedAir | Vacuum | LiquidWater | Foam
        reverb_rt60_s: f32,                   // RT60 measure of local reverb
        occlusion_db: f32,                    // attenuation from walls between sources and actor
        derived_can_hear_voice_unaided: bool, // false in vacuum even at point-blank
    },

    // DAY/NIGHT (from M7.7 day/night kernel)
    day_night: DayNightPhase {
        local_solar_time_s: f32,              // [0, day_length_seconds)
        phase: SolarPhase,                    // Night | Dawn | Day | Dusk
        sun_elevation_deg: f32,
    },

    // COMMS LATENCY (from world catalog astrography)
    comms: CommsLatency {
        light_lag_to_command_anchor_s: f32,
        active_radio_links: Vec<RadioLinkRef>, // see [[spec/comms-voice-and-radio-model]]
    },

    // DERIVED HAZARD SUMMARY
    derived_hazards: Vec<HazardClass>,        // hypoxic | combustible | hypothermic | hyperthermic | radiation | suffocation | drowning | concussion | em_disruption | low_visibility
}
```

Sub-types are defined in their producing kernel's spec page; this page is the integration contract.

## Tick Schedule (where the aggregation runs)

```
TICK START
  ↓
[1] cf-physics::gravity        (samples per actor)
[2] cf-atmos                    (kernel; updates rooms/pipes/suits)
[3] cf-material                 (kernel; updates active chunks)
[4] cf-control::worlds          (per-world astrography update; sparse cadence)
[5] cf-control::weather (M7.7)  (weather kernel)
[6] cf-control::day_night (M7.7)(day/night kernel)
[7] cf-comms (M9.5)             (radio/voice kernel; voice propagation, radio LOS, interference)
  ↓
[8] cf-environment::aggregate   ← THIS PAGE; computes EnvironmentSignal per actor
  ↓
[9] cf-actor controller          (reads bundle for movement/aim/etc.)
[10] cf-ai                       (M6 reads bundle for perception; M6.6 reads for environmental doctrine)
[11] cf-equipment                (reads bundle for radio module decisions, etc.)
[12] cf-ui                       (reads bundle for HUD overlays)
[13] cf-replay                   (records bundle deltas)
[14] cf-audio                    (reads bundle for acoustic propagation)
[15] cf-mission                  (reads bundle for objective triggers)
  ↓
TICK END
```

The aggregator is the pivot: producers run before, consumers run after. Determinism preserved through fixed iteration order.

## Hazard Class Taxonomy

`derived_hazards` is the closed-enum list every consumer can match on without looking at every signal field individually:

| Hazard Class | Source signal(s) | Origin gate |
|---|---|---|
| `hypoxic` | atmospheric.partial_pressure_pa[O2] < 16 kPa AND atm_kind != Robot-suit | Humans + Androids organic |
| `combustible_atmosphere` | atmospheric.composition_ratio[Volatiles] >= 5% AND [O2] >= 5% AND temp >= autoignite | Universal |
| `toxic_atmosphere` | atmospheric.composition_ratio[Pollutant] > toxin_threshold | Humans + Androids organic |
| `breach_decomp` | atmospheric.breach_eta_s.is_some() | Humans + Androids; Robots note for navigation |
| `hyperthermic` | thermal.derived_band == Hot OR Extreme on hot side | Humans (heat exhaustion); Androids (per-module overheat); Robots (downclock) |
| `hypothermic` | thermal.derived_band == Cold OR Frigid | Humans + Androids organic (frostbite-flagged); Robots (joint viscosity) |
| `radiation` | radiation.derived_dose_band >= Hazardous | Humans + Androids organic (cumulative dose); Robots (sensor noise + processor faults) |
| `low_visibility` | photic.derived_visibility_band == Pitch OR weather.visibility_m < 50 | All origins (affects perception) |
| `glare` | photic.derived_visibility_band == Glaring (solar flare, etc.) | All origins (affects optics) |
| `em_disruption` | em.em_emp_recently == true OR em_noise_db > threshold | Robots + Androids synthetic; humans note for radio |
| `wind_force` | weather.wind_mps.magnitude() > threshold | All origins; routes through M5.5-008 impulse force |
| `drowning_hazard` | water.is_drowning_hazard == true | Humans + Androids organic |
| `vacuum_no_voice` | acoustic.propagation_medium == Vacuum | All origins (gameplay: must use radio for comms) |
| `comms_blackout` | comms.light_lag_to_command_anchor_s > X OR active_radio_links.iter().all(\|l\| l.signal_quality < threshold) | All origins (gameplay) |
| `gravity_shift` | gravitational.source != Ambient OR magnitude crosses configured band | All origins (movement / ballistics impact) |

The hazard list is what AI doctrine reads for high-level reasoning. The granular fields are what specialized systems read for fine-grained reactions.

## Run-Bundle Event Family Extensions

`environment` event category. Sparse — bundle deltas, not per-tick snapshots.

| Event Type | Fires when | Required Fields |
|---|---|---|
| `environment.signal_changed` | A consumer-relevant slice of the bundle changed beyond a threshold | actor_id, signal_class (atmospheric/gravity/thermal/radiation/photic/em/weather/water/acoustic/comms), old/new summary |
| `environment.hazard_detected` | A new hazard class entered the actor's derived_hazards list | actor_id, hazard_class, parent_event_id (from upstream signal change) |
| `environment.hazard_cleared` | A hazard class left the list | actor_id, hazard_class, parent_event_id |
| `environment.bundle_snapshot` | Sparse periodic full bundle (e.g., once per scenario-second) for debug/replay scrub | actor_id, full bundle hash, key fields |
| `environment.aggregator_perf` | Active actor count, ms per tick, sleeping actor count | tick, n_active, n_sleeping, ms |

## Acceptance Tests (ENV-A)

| Test | Setup | Pass Condition |
|---|---|---|
| ENV-A-01 | Single actor in sealed Earth room. Air = 75% N2 / 25% O2 at 101 kPa, 293 K. | EnvironmentSignal reports atmospheric.O2 partial pressure ≈ 25.3 kPa; thermal.derived_band == Comfortable; gravitational.g_factor ≈ 1.0; derived_hazards is empty. |
| ENV-A-02 | Same actor, breach to vacuum opens. | Within tick: atmospheric.breach_eta_s = Some(t); derived_hazards += [breach_decomp]; environment.hazard_detected fires; AI / HUD react in next tick. |
| ENV-A-03 | Actor on Mars surface, helmet sealed, oxygen tank attached. | Suit atm_kind reports correct partial_pressure values; ambient atmospheric reports 95% CO2 + 2-3 kPa; derived_hazards reflects suit-protected (no hypoxic). |
| ENV-A-04 | Actor enters low-g override region (gravity_g = 0.1). | gravitational.g_factor = 0.1 within tick; derived_hazards += [gravity_shift]; ballistic + actor controller react. |
| ENV-A-05 | Solar flare event fires from M7.7 kernel. | radiation.solar_msvph rises; derived_dose_band escalates; em.em_noise_db rises; derived_hazards += [radiation, em_disruption] for affected actors; comms degradation flag fires. |
| ENV-A-06 | Vacuum scenario, robot vs human + android. | acoustic.propagation_medium == Vacuum for all three; vacuum_no_voice in derived_hazards; consumers must route comms through radio (DR-043). |
| ENV-A-07 | Determinism replay. | Same scenario + same seed + same actor inputs → byte-identical EnvironmentSignal stream across 10000+ ticks. |
| ENV-A-08 | Modder adds a new "psionic_field" signal slice. | Aggregator picks up the extension via typed adapter; replay events include modder slice; consumers can match on it. |
| ENV-A-09 | Active-region budget. | At 50-actor scenario, aggregator runs in <X ms/tick on Steam Deck floor (specific budget set per [[spec/prototype-roadmap#No-Compromise Performance Defaults]]). |
| ENV-A-10 | AI Environmental Competence smoke (M6.6). | AI reads EnvironmentSignal directly; never queries cf-atmos / cf-physics::gravity / cf-material independently. CI grep gate. |

## AI Doctrine Integration (M6.6 promoted to AI Environmental Competence)

Per [[spec/native-implementation-backlog#M6.6 — AI Material Competence]], promoted scope:

| AI Need | EnvironmentSignal slice |
|---|---|
| Stay alive | derived_hazards (every entry has an avoidance plan) |
| Use environment as weapon | hazard tags + reaction-table joins (e.g., "vent O2 into combustible room then ignite") |
| Plan route across worlds | comms latency + per-world catalog data |
| Brief teammates on threats | hazard_detected event chain → squad radio chatter |
| Refuse unsafe order | "won't walk unsealed human across vacuum"; reason label `wrong_origin_for_environment` |
| Calibrate equipment | thermal.ambient_k → adjust suit AC; radiation → swap to radiation-resistant module |
| Time mission by phase | day_night.phase → "attack at dawn"; weather.eta → "wait until storm passes" |

The reason-label enum extends with `environment_*` codes; AI doctrine never invents free-text reasons.

## Modding Contract

- Add a new signal slice: implement `EnvironmentSignalExtension<T>` trait; register via `cf-environment::register_extension(...)` at startup.
- Add a new hazard class: data row in `content/hazards/` with class id, source signal expression, severity threshold, AI affordance tag, HUD chip preset, caption hook.
- Schema validates via `cargo run -p cf-mod -- validate content/hazards/`.

## Performance Posture

- Aggregator runs on the deterministic CPU thread; SoA actors; per-tick budget bounded by active-actor count.
- Sleeping actors (no movement, no signals changed) skip aggregation; their last bundle is reused with a freshness flag.
- Bundle-delta replay events are sparse (threshold-gated); per-second snapshot for debug scrub.
- Determinism: aggregation runs in fixed iteration order across actors; no platform atomics in inner loop.

## Out Of Scope (during M0..M5.9)

- M0/M1/M2/M3: actors don't have an EnvironmentSignal yet; placeholder structure may exist with no producers.
- M4 HUD: reserved fields for hazard chips; render placeholders until M5.10.
- M5.5 / M5.6 / M5.7 / M5.8 / M5.9: signal-producing kernels land; aggregator does NOT exist yet. Per-kernel queries are temporary scaffolding that gets replaced when M5.10 lands.
- M5.10 lands the aggregator; M6 perception starts reading from it; M6.6 promotes to environmental competence; M7.7 lands weather + day/night that feed it; M9.5 lands voice/radio that feeds the acoustic + comms slices.

## Source Trail

- [[spec/celestial-bodies-and-worlds-model]]
- [[spec/atmospherics-and-chemistry-model]]
- [[spec/gravity-and-ballistics-model]]
- [[spec/origin-reaction-and-resource-model]]
- [[spec/comms-voice-and-radio-model]]
- [[spec/full-collision-physics-plan]]
- [[references/prototype-run-bundle-schema]]
- [[decisions/dr-022-ai-humanlike-bar]]
- [[decisions/dr-036-systemic-material-simulation-direction]]
- [[decisions/dr-037-stationeers-grade-atmospherics-direction]]
- [[decisions/dr-038-universal-gravity-and-ballistics-direction]]
- [[decisions/dr-039-celestial-bodies-and-worlds-direction]]
- [[decisions/dr-040-environmental-conditions-and-hazards-direction]]
- [[decisions/dr-043-voice-comms-and-radio-direction]]
- [[research-log/2026-05-06-celestial-bodies-environments-mining-bunker-defence-design-intent]]

## Change Log

- 2026-05-06: Captured during M1 from user-supplied design intent ("environmental conditions affect the player and units; the unit AI needs to handle all this perfectly"). User chose **EnvironmentSignal aggregation layer** as the unification approach. Status: `design-intent-post-m1`. Lands at new M5.10. Origin-resistance matrix in [[spec/origin-reaction-and-resource-model]] is re-rooted to read this bundle.
