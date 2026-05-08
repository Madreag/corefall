---
type: spec
status: design-intent-post-m1
authority: "Canonical contract for Stationeers-grade-or-better gas chemistry, real pressure (PV=nRT), combustion stoichiometry, phase change, pipe networks, room atmospheres, planetary atmospheres, suit/breathing life-support, base atmospherics modules, pressure-flow wind/liquid jets, heat transfer through materials, and ventilation. Captured during M1; lands at extended M7.5 (Base Atmospherics) and M5.9 (Atmospherics-Grade Kernel). M0 and M1 must remain atmospherics-agnostic."
ready_when: "Extended M7.5 + M5.9 land; ATMOS-A acceptance suite passes; pipe networks + room atmospheres + EVA suits + furnace combustion + per-planet ambient + phase change + breach/aperture flow + material heat transfer all run deterministically against the canonical run-bundle schema."
feeds:
  - DR-002
  - DR-003
  - DR-004
  - DR-005
  - DR-007
  - DR-008
  - DR-012
  - DR-013
  - DR-014
  - DR-018
  - DR-021
  - DR-024
  - DR-027
  - DR-029
  - DR-033
  - DR-034
  - DR-035
  - DR-036
  - DR-037
---

← [[index|vault home]] · [[spec/index|spec section]] · [[spec/origin-reaction-and-resource-model|origin reaction/resource model]] · [[spec/full-collision-physics-plan|full collision plan]] · [[spec/body-damage-model|body damage model]] · [[spec/chassis-armor-mechs-and-origins|chassis/armor/mechs/origins]] · [[spec/equipment-loadout|equipment/loadout]] · [[spec/prototype-roadmap|native roadmap]] · [[spec/native-implementation-backlog|native backlog]] · [[decisions/dr-007-terrain-material-model|DR-007]] · [[decisions/dr-036-systemic-material-simulation-direction|DR-036]] · [[decisions/dr-037-stationeers-grade-atmospherics-direction|DR-037]]

# Atmospherics And Chemistry Model

> [!summary] What this page is
> The canonical contract for **Stationeers-grade as the minimum bar, then beyond it**: gas chemistry, real pressure simulation (PV=nRT), combustion stoichiometry, phase change, pipe networks, room atmospheres, planetary atmospheres, suit life-support, base atmospherics modules, doors and weapon-created holes as pressure barriers/apertures, vents/valves/regulators/filters, breach detection, gas/liquid pressure jets, wind from pressure differentials, and heat transfer through materials. Every simulation surface — actors, equipment, chassis modules, base modules, terrain materials, weather, weapons, fire — reads from this model. There is one atmospherics/thermal simulation; everyone subscribes.
>
> The grammar mirrors Stationeers (the minimum acceptable feel target; 29+ source citations in [[references/sources#stationeers-atmospherics-research|sources ledger]]) but the project is allowed to go beyond it with more gases, more elements, more materials, richer heat transfer, and tighter combat/base coupling when prototypes prove readability, performance, replay determinism, and fun.

> [!warning] Authority boundary
> Captured 2026-05-06 as **design intent**. The model (which equations apply, which gases ship at launch, which events fire, which devices interface with which networks) is a commitment. Specific numeric tuning (per-planet ambient values, per-device flow rates, per-actor breathing rates) stays open until M7.5 / M5.9 prototype evidence backs them.

> [!important] Out of scope right now
> M0 is closed. M1 is the active milestone. **Nothing on this page is implemented in M0/M1/M2/M3/M4.** The first implementation surface is the extended M7.5 (Base Atmospherics, currently shallow) and a new proposed **M5.9 — Atmospherics-Grade Kernel** that lands the gas registry, PV=nRT engine, pipe network topology, room detection, and combustion. Earlier milestones may carry placeholder fields (e.g., a `room_id` field on actors) only if they're identity-no-op until the kernel lands.

## Why This Page Exists

DR-036 already commits to systemic material simulation. The existing M7.5 task card commits to "base atmospherics". But neither nails down:

- That gas pressure follows the ideal gas law (PV=nRT) per atmosphere unit, not a fudged arcade approximation.
- That gases have molar quantities with per-gas specific heat, latent heat, autoignition temperature, condensation/freezing thresholds, and molar mass.
- That combustion is a stoichiometric reaction with deterministic energy/byproduct outputs (Volatiles + O2 → CO2 + Pollutant + heat; H2 + O2 → Steam + heat; etc.).
- That doors/windows/walls are pressure barriers with rupture thresholds.
- That pipe networks are first-class atmospheres separate from room atmospheres, connected via valves/pumps/regulators/filtration.
- That suits and helmets are tiny atmospheres with breathing input + waste output + filter routing — not "O2 timer".
- That planetary atmospheres are infinite reservoirs with per-planet composition, temperature, pressure.
- That fire/explosions/breaches/wind all emerge from pressure differentials and reaction rates, not scripted effects.

This page locks all of that. Other pages cross-link here instead of restating it.

## Principles (locked)

1. **One simulation, many subscribers.** Every system that reads "is there oxygen here" / "what's the pressure" / "is it on fire" / "what's the temperature" reads from the atmospherics kernel. No parallel atmosphere models in actor / weapon / fire / weather code.
2. **Real ideal gas law, all the time.** Every atmosphere unit (room cell, pipe network, suit interior, canister, lung) tracks `n` (moles per gas type), `V` (volume in liters), `T` (temperature in Kelvin), and computes `P = nRT/V` on demand. R = 8314.46 J/(kmol·K) per Stationeers convention (note: kilomole, not mole, because the engine works in kPa·L = J·1000). Source: [[references/sources#stationeers-pressure-volume-quantity-temperature|Stationeers wiki Pressure/Volume/Quantity/Temperature]].
3. **Combustion is stoichiometry, not a flag.** Each combustion reaction has a fixed mole equation, energy release in joules per mole, autoignition temperature, and minimum ratio thresholds for ignition. Per-tick reaction rate is a deterministic function of temperature and present moles. Source: [[references/sources#stationeers-volatiles|Volatiles]], [[references/sources#stationeers-hydrogen|Hydrogen]], [[references/sources#stationeers-furnace-temperature-and-pressure-math|Furnace math]].
4. **Phase change is gradual, not instant.** Gases ↔ liquids ↔ solids transition over time per phase diagram. Liquids always co-exist with a gas (any gas can be the pressurant). Latent heat is consumed on evaporation and released on condensation. Source: [[references/sources#stationeers-phase-change-guide|Phase Change guide]].
5. **Pipe networks are first-class atmospheres.** A connected pipe segment graph is one atmosphere. Pumps, valves, regulators, filtration units, condensation chambers split networks. Each piece of network state (volume, total moles, partial moles per gas, temperature) is one record. Source: [[references/sources#stationeers-atmosphere|Atmosphere § Pipe Networks]].
6. **Rooms are first-class atmospheres.** A connected sealed-volume graph is one atmosphere. Walls/floors/ceilings are barriers (sealed); doors/windows are barriers with state (open/closed/breached). Vacuum dissipates atmosphere within "a few large grid atmospheres of distance" — exact rule per kernel.
7. **No invisible chemistry.** Every reaction emits replay events. Every phase change emits replay events. Every breach emits replay events. The run-bundle reproduces the atmosphere state from event stream.
8. **Determinism > visual fidelity for sim core.** Kernel runs CPU-deterministic. Per-cell shader hints are presentation; the source of truth is the kernel state.
9. **Modder-extensible.** New gases, new reactions, new device types are data-driven. Schema covers the common case; Lua/script escape hatches allowed for affliction logic / device behavior.
10. **Openings are physical apertures.** Door openings, vents, cracked windows, bullet holes, shaped-charge cuts, blast breaches, pipe ruptures, suit punctures, and terrain cracks create aperture records with area, edge material, normal, source event, and open/closed/breached state. Gas/liquid flow reads those apertures; a hole is not just a decal.
11. **Heat moves through matter.** Solids, liquids, gases, equipment, armor, weapons, pipes, tanks, doors, and base modules exchange heat through material conductivity/insulation, moving fluids, phase change, combustion, electrical load, collision/friction, and bounded ambient/radiation exchange.
12. **Temperature is gameplay.** Players must be able to cool, heat, insulate, vent, pump, flood, radiator-loop, phase-change, and power-throttle systems. Hot/cold state affects pressure, suit survival, batteries, weapons, armor, pumps, sensors, doors, bases, AI doctrine, and mission objectives.
13. **Stationeers-grade is a floor, not a cap.** The launch kernel may start with a curated registry, but the architecture must support more elements, more material phases, more reaction families, and more thermal devices after M8.5 material-lab evidence.
14. **Performance must scale.** Atmosphere/thermal hot paths are active-region, cache-friendly, multicore-ready, and benchmarked. GPU acceleration is welcome for visualization and future compute, but replay-deterministic CPU truth remains the acceptance path until a GPU compute path proves parity.

## Atmosphere Unit Model

Every atmosphere unit (room cell, pipe network, suit, canister, lung, furnace, base module) is the same record:

```text
struct Atmosphere {
    id: AtmId,                         // stable id
    kind: RoomCell | PipeNetwork | Suit | Canister | Lung | DeviceInternal,
    volume_l: f32,                     // liters
    moles: PerGas<f32>,                // moles of each registered gas
    moles_liquid: PerGas<f32>,         // moles of each registered gas in liquid phase
    moles_solid: PerGas<f32>,          // moles of each registered gas in solid phase
    temperature_k: f32,                // kelvin
    insulation_class: Insulator,       // none | partial | sealed | radiator | superinsulator
    pressure_differential_max_pa: f32, // rupture threshold
    thermal_mass_j_per_k: f32,         // heat capacity of contained matter + structure
    material_shell: MaterialId,        // wall/container/pipe/suit material for conductivity
    parent: Option<AtmId>,             // for nested atmospheres (suit-in-room)
    flags: AtmFlags,                   // sealed, vented, on_fire, breached, etc.
}
```

`pressure = sum(moles[g]) * R * T / V` (computed, never stored).

Liquids contribute to volume (40 L/kmol typical) but produce no gas pressure of their own; they evaporate when ambient gas pressure drops below their per-temperature vapor pressure.

Solids contribute negligible volume; they don't produce vapor pressure (per Stationeers footnote — actual sublimation is invisible until macroscopic ice forms).

## Gas Registry (Launch Set)

The launch gas registry mirrors Stationeers' base 7 + the elemental subset. Mod authors can add more via the data schema in [[spec/modding-model]].

| Gas | Symbol | Molar Mass (g/mol) | Specific Heat Capacity (J/mol·K) | Latent Heat (J/mol) | Min Condensation Pressure (kPa @ K) | Max Liquid Temp (kPa @ K) | Freeze Point (K) | Liquid Density (L/kmol) | Combustion Role |
|---|---|---:|---:|---:|---|---|---:|---:|---|
| Oxygen (O2) | `o2` | 16 (game value) | **21.1** | 800 | 6.3 @ 56.4 | 6000 @ 162.2 | 56.4 | 30 | **Oxidizer** (pure O2 + volatiles autoignites at 573.15 K / 300 °C) |
| Nitrogen (N2) | `n2` | 14 (game value) | **20.6** | — | — | — | — | — | Inert filler (air composition); cryogenic coolant |
| Carbon Dioxide (CO2) | `co2` | 44 | **28.2** | 600 | 517 @ 217.82 | 6000 @ 265 | 217.82 | 40 | Inert; combustion byproduct; plant feed; coolant (high specific heat) |
| Volatiles / Methane (CH4) | `volatiles` | 16 | **20.4** | 1000 | 6.3 @ 81.6 | 6000 @ 195 | 81.6 | 40 | **Combustible** with O2/N2O/Ozone |
| Pollutant (X) | `pollutant` | 28 | **24.8** | 2000 | 1800 @ 173.32 | 6000 @ 425 | 173.32 | 40 | Toxic to humans/plants; combustion byproduct of Volatiles+O2; coolant (high latent heat) |
| Hydrogen (H2) | `h2` | 2 | **20.4** | 200 | 6.3 @ 15.18 | 6000 @ 70.06 | 15.18 | 28 | **Combustible** with O2/N2O/Ozone (cleanest fuel — H2+O2 → Steam) |
| Nitrous Oxide (N2O) | `n2o` | 44 | **23** | — | — | — | — | — | **Oxidizer** (lower autoignition with volatiles: 50 °C); rocket fuel component |
| Water / Steam (H2O) | `water` | 18 | **72** (game value; very high — both liquid and gas in one record) | — | — | — | — | — | Inert; combustion byproduct of H2+O2; high-capacity coolant |
| Ozone (O3) | `ozone` | 48 (game value) | — | — | — | — | — | — | **Oxidizer** (autoignition with H2/Volatiles: 150 °C); rocketry; tracer |
| Helium (He) | `helium` | 4 | — | — | — | — | — | — | Inert; cryogenic |
| Liquid mixtures: Polluted Water, Alcohol, Silanol, Liquid Sodium Chloride, Hydrochloric Acid, Hydrazine | various | — | — | — | — | — | — | — | Liquid-only initially; Hydrazine = combustible; HCl = corrosive |

Game-vs-real mass note: Stationeers uses gameplay-friendly molar masses (e.g. Volatiles/Methane both at 16 g/mol). We inherit those values at launch for tuning fidelity with established player intuition; we may move to real molar masses (CH4 = 16.04, N2 = 28.01) post-launch if balance and rocketry math demand it.

## Ideal Gas Law — Locked Form

```text
P · V = n · R · T

where:
    P  = pressure                      [Pa]   (= kPa × 1000)
    V  = atmosphere volume             [L]    (= m³ × 1000)
    n  = total moles of gas (sum across registered gases) [mol]
    R  = ideal gas constant            [L·Pa / (mol·K)]
       = 8314.46  (Stationeers convention; equivalent to 8.31446 J/(mol·K) when V is in m³)
    T  = temperature                   [K]
```

Per-gas partial pressure: `P_g = n_g · R · T / V`.

Per-gas mole fraction (ratio): `x_g = n_g / n_total`.

When two atmospheres become connected (door opens, valve opens, breach occurs), the kernel mixes them by:

1. Total `n` is summed across both.
2. Total `V` is summed (sealed-room + pipe-network = combined atmosphere if a vent connects them).
3. Temperature is mass-weighted by specific heat: `T_mixed = Σ (n_g · cp_g · T_g) / Σ (n_g · cp_g)`.

Mixing is **gradual** when atmospheres are connected via a flow-rate-limited interface (vent, regulator, valve at flow setting). Mixing is **immediate** when atmospheres are directly connected (open door, broken wall) — but the kernel still writes per-tick partial mixing events, so replay can scrub through the equalization curve.

## Combustion (Stoichiometry + Energy)

Locked combustion table for the launch gas set. Reaction rate is a deterministic function of temperature and partial pressures; the combusting gas mixes consume a fraction of the limiting ingredient per tick.

| Reaction | Stoichiometry | Energy (kJ/mol of reaction) | Autoignition T | Min Ratio Required |
|---|---|---:|---|---|
| Volatiles + Oxygen | `2 V + 1 O2 → 6 CO2 + 3 X (Pollutant)` | **572** | 573.15 K (300 °C) | ≥ 5% O2 AND ≥ 5% Volatiles |
| Volatiles + Nitrous Oxide | `1 V + 1 N2O → 2 CO2 + 2 N2` | **572** | 323.15 K (50 °C) | ≥ 5% N2O AND ≥ 5% Volatiles |
| Volatiles + Ozone | `3 V + 2 O3 → 6 CO2 + 3 X + 1 Steam` | **1716** | 423.15 K (150 °C) | ≥ 5% O3 AND ≥ 5% Volatiles |
| Hydrogen + Oxygen | `2 H2 + 1 O2 → 3 Steam` | **612** (perfect fuel: 593.107 J/mol O2 effective at 95% efficiency = 563,452 J/mol) | 573.15 K (300 °C) | ≥ 5% O2 AND ≥ 5% H2 AND ≥ 10 kPa total pressure |
| Hydrogen + Nitrous Oxide | `1 H2 + 1 N2O → 1 Steam + 1 N2` | **612** | 323.15 K (50 °C) | ≥ 5% N2O AND ≥ 5% H2 |
| Hydrogen + Ozone | `3 H2 + 1 O3 → 4 Steam` | **1836** | 423.15 K (150 °C) | ≥ 5% O3 AND ≥ 5% H2 |

Source: [[references/sources#stationeers-volatiles|Volatiles]], [[references/sources#stationeers-hydrogen|Hydrogen]], [[references/sources#stationeers-furnace-temperature-and-pressure-math|Furnace temperature and pressure math]].

**Reaction rate (per tick):**

```text
rate_o2(T) = clamp01(1 / (0.002 * T^1.6 + 0.05)) / 5
rate_n2o(T) = clamp01(1 / (0.0025 * T^1.01 + 0.05)) / 5
```

Default cap: 0.2 (20% of limiting ingredient per tick) below ~211 °C for O2; ~107 °C for N2O. Above those bands, rate decreases logarithmically. Combustor / Gas Fuel Generator devices boost rate to 0.9 (90% per tick).

**Combustion efficiency:** 95% of limiting ingredient is consumed per ignition event in the furnace; the residue prevents perpetual lossless cycling.

**Energy → temperature → pressure flow per tick:**

```text
ΔE_combusted        = moles_consumed * energy_per_mol
ΔT                  = ΔE_combusted / Σ(n_g * cp_g)        # specific heat of the mix
P_after             = (n_after * R * T_after) / V         # PV=nRT recompute
```

Pressure spikes from combustion can rupture pipes/canisters/walls per their `pressure_differential_max_pa`. Rupture is a structural event with replay payload.

## Phase Change

Gases ↔ liquids ↔ solids gradually. Source: [[references/sources#stationeers-phase-change-guide|Phase Change guide]].

```text
For each gas g in atmosphere:
    P_g = n_g * R * T / V                     # current partial pressure
    P_vapor_g = vapor_pressure(g, T)          # phase diagram lookup

    if T < freeze_point(g) AND P_g < min_condensation_pressure(g):
        # gas → solid directly (sublimation reverse); risky (can break pipes)
        fraction_to_solid = small_per_tick_rate
        n_solid_g += n_g * fraction_to_solid
        n_g       -= n_g * fraction_to_solid

    elif T > max_liquid_temperature(g):
        # too hot for any liquid → all evaporates
        evaporate(g)

    else:
        # liquid/gas equilibrium band
        if P_g > P_vapor_g:
            # condense gas into liquid
            condensed = (P_g - P_vapor_g) * V / (R * T) * condensation_rate
            n_liquid_g += condensed
            n_g        -= condensed
            T          += condensed * latent_heat(g) / Σ(n_g * cp_g)

        elif n_liquid_g > 0 AND P_g < P_vapor_g:
            # evaporate liquid into gas
            evaporated = (P_vapor_g - P_g) * V / (R * T) * evaporation_rate
            n_g        += evaporated
            n_liquid_g -= evaporated
            T          -= evaporated * latent_heat(g) / Σ(n_g * cp_g)
```

Per Stationeers: condensation increases temperature; evaporation decreases temperature; freezing changes neither (the energy goes into structural lattice). Phase-change loops can drive heating/cooling cycles.

## Pipe Networks

A pipe network is one atmosphere shared across all connected pipe segments (default 100 L per segment per Stationeers). Pumps, valves, regulators, filtration units, condensation/evaporation chambers split the network into separate atmospheres. Source: [[references/sources#stationeers-atmosphere|Atmosphere § Pipe Networks]], [[references/sources#stationeers-pipe-volume-pump|Pipe Volume Pump]].

| Device | Owns | What It Does |
|---|---|---|
| Pipe Segment | volume contribution | Adds 100 L (default) to its network. Junctions don't add volume. |
| Active Vent | room ↔ pipe boundary | Moves gas in or out (`Outward` / `Inward` mode); enforces both `PressureExternal` and `PressureInternal` thresholds. Power: 100 W. Max 10 kPa/tick into 8000 L grid. |
| Passive Vent | room ↔ pipe boundary | Equalizes passively; no power; no flow control. |
| Pressure Regulator | pipe-pipe | Targets a specific output pressure; flows gas only when output is below target. |
| Back Pressure Regulator | pipe-pipe | Targets a specific input pressure; releases gas only when input exceeds target (the dump-valve role). |
| Volume Pump | pipe-pipe | Moves a fixed volume of gas per tick (0-10 L/tick, dial); flow is rate-based, not pressure-based. |
| Turbo Pump | pipe-pipe | High-flow variant for industrial transfer. |
| Valve | pipe-pipe | Manual on/off (and one-way variants). |
| Filtration Unit | pipe-pipe-pipe | Splits 1 input into 2 outputs by gas filter type; the matching gas goes to "filtered out" pipe. |
| Condensation Chamber | gas → liquid | Phase-change device with controlled cooling. |
| Evaporation Chamber | liquid → gas | Phase-change device with controlled heating. |
| Purge Valve | liquid pipe → gas pipe (gas only) | Removes pressurant gas from liquid pipe. |
| Pressurant Valve | gas pipe → liquid pipe (gas only) | Adds pressurant gas to liquid pipe. |
| Condensation Valve | gas pipe → liquid pipe (liquid only) | Phase-change valve. |
| Expansion Valve | liquid pipe → gas pipe (liquid only) | Phase-change valve. |
| Tank | bulk storage | Up to 10 MPa portable (game value); per-material structural limit. |

**Pipe damage thresholds:**

- Gas pipes: rupture if frozen-solid contents > 0.05 mol/L OR pressure differential > 600 atm (60.795 MPa) OR liquid stress > 100% (where stress % = 5000 × liquid_L / network_volume_L).
- Liquid pipes: rupture if pressure differential > 60 atm (6.079 MPa) OR frozen contents > 0.05 mol/L. Liquid pipes can over-pressurize from added liquid even though liquid produces no pressure (volume is shared with the pressurant gas).
- Pipes inside double-welded frames are immune to rupture (Stationeers exploit; we should NOT replicate this loophole; we may keep the double-frame visual but charge a small overhead for the protection so it doesn't trivialize design).

## Rooms And Walls

A **room** is a connected sealed-volume graph. Walls / floors / ceilings are barriers. Doors / windows / hatches / airlocks are barriers with state.

- Each room cell (large grid: e.g., 2×2×2 m or chosen per [[spec/full-collision-physics-plan]]) holds one atmosphere unit. Adjacent atmosphere units exchange contents and heat via the kernel diffusion step.
- A "sealed room" is a room cell whose every face is either a sealed barrier or an adjacent sealed room cell. Sealing makes the union of those cells a single combined atmosphere update — the kernel collapses adjacent sealed cells into a meta-atmosphere for performance, but per-cell partial pressures are still queryable for the HUD.
- Open boundaries (vacuum, planetary atmosphere) cause atmosphere dissipation within a few large-grid distances per tick.
- Walls / windows / pipe segments / containers each carry a `pressure_differential_max_pa` field. Exceeding it causes structural rupture, dumping internal atmosphere to neighbors and breaking the structure.
- Weapon/projectile/explosive damage can create partial holes before full rupture. A bullet hole is an aperture with small area; a shaped charge cut is a larger aperture; a blown wall section is a full connection. The source event stays in the replay chain so the player can inspect "this room depressurized because round X punched hole Y."

## Doors And Airlocks

Doors are pressure barriers with state machine: `closed_sealed` / `closed_unsealed` / `cycling_open` / `open` / `cycling_close` / `breached`.

| Door Type | Closed Behavior | Open Behavior |
|---|---|---|
| Hatch (manual) | sealed | direct connection between two atmospheres |
| Powered Door | sealed when powered + closed; unsealed if power-off | direct connection |
| Emergency Door | auto-close on detected pressure differential; sealed | direct connection |
| Airlock chamber | per Stationeers Airlock guide: 1×1×1 chamber, two doors, two active vents (one for each side's atmosphere), one logic console — both inner and outer doors locked unless cycle complete | passable when cycle matches target side |

Airlock cycle: vent the chamber to target side, wait for pressure equalization (e.g., > 101 kPa for inhabited side; < some threshold for vacuum side), then unlock target door. Fully scriptable via base IC chips for now, but a canonical Airlock Controller object should ship with the same logic baked in for non-tinkerers.

Breach detection: if a sealed-room atmosphere loses > X% pressure per tick OR a structure with `pressure_differential_max_pa` ruptures, kernel emits `room_breach` event.

## Flow, Wind, Liquid Jets, And Breach Holes

When two atmospheres or liquid volumes are connected and have different pressures or fluid heads, matter flows from high potential to low potential. Per Stationeers:

> Flux of gases between open atmospheric systems is indicated by the particles travelling from higher-pressure to lower pressure regions. The difference in pressure accelerates that movement, causing loose objects and player to get pulled and flung about by the flux.

- Flow rate is proportional to ΔP and to aperture/interface area (open door = large, passive vent = small, bullet hole = tiny, shaped-charge cut = medium, broken wall = large). Extreme ΔP uses a bounded choked-flow cap so the sim stays stable and readable.
- Every aperture has: `aperture_id`, `from_atm_id`, `to_atm_id`, `area_m2`, `normal`, `edge_material`, `source_event_id`, `open_fraction`, `flow_limit`, `liquid_limit`, `seal_state`, `damage_stage`.
- Doors/hatches set aperture area by state. Weapon hits create apertures based on projectile energy, material hardness, exit wound, and local damage stage. Repairs reduce aperture area over time rather than snapping to sealed unless the tool explicitly patches it.
- Liquids flow too. Water, fuel, acid, coolant, blood/vomit, and molten liquids have density, viscosity, temperature, contamination, and phase state. A high-pressure tank or flooded room can jet liquid through a hole, spray actors, push loose objects, cool/heat surfaces, short electronics, spread acid/fuel, or flood lower rooms by gravity.
- Mixed gas/liquid expulsion is valid: a breached hot tank can vent steam plus boiling liquid; a punctured coolant line sprays cold liquid plus vapor; a pressure door opening can shove mist/smoke/debris.
- Loose physics objects (dropped equipment, debris, gibs) take an impulse force proportional to local ΔP. Hooks into [[spec/full-collision-physics-plan]] M5.5-008 impulse-to-damage routing.
- Actors take a wind force on their hull; below threshold = walking is harder; above threshold = pulled toward the lower-pressure side; at extreme = ragdoll → vacuum (the "blown out the airlock" cinematic). Hooks into [[spec/origin-reaction-and-resource-model#Origin Reaction Matrix|origin reaction/resource model]] for fall-damage and force-feedback events.
- Direction interacts with gravity: heavy gases sink, light gases rise — see [[spec/gravity-and-ballistics-model]].

## Heat Transfer And Thermal Engineering

Temperature is one of the primary systemic axes, equal with pressure, material, and power. It is not a visual-only status.

| Heat Route | Contract |
|---|---|
| Conduction | Adjacent solids/structures exchange heat through material thermal conductivity, thickness, contact area, insulation, and damage stage. Metal doors conduct heat faster than ceramic/insulated panels. Armor, weapons, pipes, tanks, and base walls all participate. |
| Fluid advection/convection | Moving gas/liquid carries heat. A hot gas leak warms a room; cold coolant spray chills armor; a vent loop can intentionally move heat from one chamber to another. |
| Phase change | Evaporation consumes latent heat; condensation/freezing releases heat. Cooling can be done by expansion/phase-change chambers; heating can happen through compression/combustion. |
| Combustion/electrical load | Fire, reactors, batteries, motors, pumps, shields, turrets, and overclocked modules produce heat according to load and efficiency. Damage can increase waste heat. |
| Collision/friction | High-energy impacts, bullet strikes, grinding, and moving machinery can add localized heat when relevant and bounded. |
| Ambient/radiation | Simplified bounded ambient/radiative exchange lets radiators, vacuum exposure, hot planets, and cold planets matter without requiring full CFD or ray-traced thermal simulation. |

Player techniques that must be valid:

- Install heaters/coolers, wall heat exchangers, radiators, heat sinks, powered pumps, fans/vents, thermal shutters, insulated panels, and emergency dump valves.
- Build coolant loops using high-heat-capacity gases/liquids, including CO2/Pollutant/Water-style coolants and future material-lab coolants.
- Use phase-change chambers, expansion valves, condensation/evaporation chambers, and exterior radiators to move heat.
- Vent hot atmosphere to outside, flood hot machinery, isolate rooms with doors/airlocks, reroute power, or deliberately depressurize a burning compartment.
- Overheat enemy rooms/equipment, freeze pipes, cool reactors, warm suits, preheat fuel/combustion chambers, and manage battery/weapon/mech thermal limits.

## Suit / Helmet / Lung Life-Support

Each player and AI actor has internal atmospheres for helmet, suit, and lungs. Source: [[references/sources#stationeers-eva-suit|EVA Suit]], [[references/sources#stationeers-hardsuit|Hardsuit]].

| Atmosphere | Volume | Function |
|---|---|---|
| Lung | (small, per origin) | Inhale partial pressure of breathable gas (humans: O2; android organic side: O2; android variants: depends; robot: n/a). Exhale partial pressure of waste gas (humans/androids: CO2). |
| Helmet | small | Connected to lung when helmet open (= room atmosphere); sealed against ambient when closed. |
| Suit | 10 L (Stationeers EVA) | Connected to helmet when sealed; receives input from gas tank canister; routes waste through filter canisters into waste tank canister. |

**Slots (per Stationeers EVA Suit, with origin gates):**

| Slot | Item | Origin Gate |
|---|---|---|
| Air Tank | Canister with breathable gas mix | Humans + androids organic side. Robots: rejected with `wrong_origin_for_equipment` per [[spec/origin-reaction-and-resource-model#Helmet + Oxygen Tank — Equipment Contract]]. |
| Waste Tank | Empty canister; collects filtered waste | Humans + androids; robots n/a. |
| Life Support | Battery cell powering suit life-support pump | All origins (robots use it for thermal/control). |
| Filter Slot 1..3 (EVA) / 1..4 (Hardsuit) | Per-gas filter | Humans + androids select filters per breathable mix; e.g., air (75% N2 / 25% O2) requires N2 filter + CO2 filter. |
| Processor (Hardsuit only) | IC10 logic chip | Optional; consumes 5 W from suit battery. |

**Breathing math (humans, normal difficulty per Stationeers):**

```text
inhaled_o2_mol_per_tick     = 0.0048 * BreathingEfficiency * BreathingRate
exhaled_co2_mol_per_tick    = 0.5 * inhaled_o2_mol_per_tick   # humans exhale 50% of inhaled as CO2

BreathingEfficiency = AtmosphericEfficiency * DamageEfficiency
AtmosphericEfficiency = clamp(partial_pressure_inhaled_gas / 16 kPa, 0, 1.5)
DamageEfficiency      = 1.0 - lung_damage_fraction

BreathingRate per difficulty: { Creative: 0, Easy: 1, Normal: 2, Stationeer: 4 }
```

At normal difficulty + perfect efficiency: 1.728 mol O2 / minute consumed; 0.864 mol CO2 / minute exhaled.

**Hypoxia threshold:** below 16 kPa inhaled-gas partial pressure → AtmosphericEfficiency drops below 1; below 12 kPa → "OXYGEN LOW" warning; below 5 kPa → "OXYGEN CRITICAL" warning + lung damage. Per [[spec/origin-reaction-and-resource-model#Affliction Layer Extensions]] this drives the `hypoxia` affliction.

**Pressure tolerance (humans):**

- Comfortable: 50-100 kPa (0.5-1 atm Earth equivalent).
- Tolerable: 11-250 kPa.
- Survivable: 11-300 kPa.
- Outside survivable band: barotrauma damage (we DO model this; Stationeers disregards it).

**Temperature tolerance (humans):**

- Comfortable: 10-29 °C.
- Tolerable: 0-39 °C.
- Survivable: -10 to 49 °C.
- Outside survivable band: hypothermia/heat-exhaustion afflictions tick.

**Filter behavior:** A filter with type `CO2` redirects all CO2 from the suit atmosphere to the waste-tank canister at up to 4052 kPa (Stationeers cap). Beyond cap, filter chokes and CO2 accumulates in suit → hypoxia warning even if O2 is fine. Filter types match the gas registry one-to-one (CO2 filter, N2 filter, Pollutant filter, Volatiles filter, etc.).

**Helmet flush function:** manually purges suit atmosphere to the surrounding room (one-shot dump). Useful when filters are saturated or wrong-gas accumulated.

## Planetary Atmospheres

Each scenario / world has an ambient atmosphere — an infinite reservoir with locked composition, temperature, pressure. Source: [[references/sources#stationeers-atmosphere|Atmosphere § Specific Planetary Atmospheres]].

| World Archetype | Pressure | Temperature Range | Composition (mole fractions) | Notes |
|---|---|---|---|---|
| Earth-like | 101 kPa | 0-40 °C (273-313 K) | 75% N2, 25% O2 (game-canonical "Air") | Default habitable. |
| Mars-like (cold thin) | 2-3 kPa | -53 to 19 °C (220-292 K) | 95% CO2, 3% N2, 1% O2, 1% Pollutant | Vacuum-equivalent for breathing; usable for plants once pressurized. |
| Europa-like (cold N2) | 44-47 kPa | -149 to -140 °C (124-133 K) | 100% N2 | Excellent passive cryogenic medium. |
| Moon / Mimas (vacuum) | 0 kPa | n/a | none | Pure vacuum; suits required. |
| Vulcan-like (hot oxidizing) | 24-56 kPa | 127-665 °C (400-938 K) | 21% N2, 26% Pollutant, 53% Volatiles | Combustible at any spark; furnace medium. |
| Venus-like (hot dense CO2) | 239 kPa | 464 °C (737 K) | 93.1% CO2, 6.9% N2 | Crushing pressure; insulated suits only. |

Per-world `gravity_g` and per-world `wind_velocity_mps` come from [[spec/gravity-and-ballistics-model]] not this page.

## Hazardous-Composition Detection

Suit/helmet/HUD warns the player when the inhaled atmosphere is unsafe. Origin-gated per [[spec/origin-reaction-and-resource-model]]:

| Condition | Threshold | Affliction / Warning |
|---|---|---|
| O2 partial pressure < 16 kPa (humans/androids) | Hypoxia (yellow) | `affliction.hypoxia_warning` |
| O2 partial pressure < 12 kPa | Hypoxia (red) | `affliction.hypoxia_critical` |
| O2 partial pressure < 5 kPa | Hypoxia + lung damage tick | `affliction.hypoxia_damage` |
| Pollutant > 0.1 mol per atmosphere unit | Toxin warning | `affliction.poisoned_warning` |
| Pollutant > toxic threshold | Lung damage tick | `affliction.poisoned_damage` |
| Volatiles > 5% AND O2 > 5% AND T > 280 K | Pre-ignition hazard | `hazard.combustible_atmosphere` |
| Active combustion in atmosphere | Fire | `hazard.atmosphere_on_fire` |
| Breach detected | Decompression | `hazard.atmospheric_breach` |
| Temperature outside tolerance band | Thermal damage tick | `affliction.heat_exhaustion` / `affliction.frostbite` |

## Actor Interactions

Every actor reads ambient atmosphere from its current room cell (or its sealed suit atmosphere). The atmospherics kernel exposes a per-actor query API:

```rust
fn ambient_for(actor: ActorId) -> AtmosphereView;
fn breathe(actor: ActorId, dt_seconds: f32);  // inhale + exhale per tick
```

Per [[spec/origin-reaction-and-resource-model]]:

- Humans: breathe O2 → emit CO2; consume `caloric_energy`; vulnerable to hypoxia / toxin / temp.
- Androids (organic side): breathe O2 → emit CO2; consume `caloric_energy`; vulnerable to hypoxia / toxin / temp; battery side persists when organic side is impaired.
- Robots: do NOT breathe. Their `power` resource accumulator does NOT consume oxygen. Vacuum is a non-issue for robots. Heat ambient drives the involuntary downclock branch.
- Zrilian (modder example, per Stationeers): breathe Volatiles → emit N2O. Different reaction rates and exhalation ratios.

## Base / Station Modules

Each base module that affects atmosphere is a device with one or more atmosphere connections (room-side or pipe-side). Locked launch set:

| Module Class | Function |
|---|---|
| Oxygen Generator | Electrolyzes Ice (Oxite) → O2 + dump byproducts; pumps O2 to output pipe. |
| Filtration | Splits a gas mix on input pipe into per-gas output pipes (one filter type per filtration unit). |
| Wall Cooler / Wall Heater | Active heat-exchange with attached pipe network; CO2 / Pollutant / Water are good coolants per their high specific heat or latent heat. |
| Air Conditioner | Sets pipe-network temperature toward target by exchanging heat with another atmosphere. |
| Pipe Radiator | Passive heat exchange between pipe network and ambient. |
| Heat Exchanger / Radiator Loop | Moves heat between two pipe networks, a room and pipe network, or pipe network and exterior ambient; supports coolant-loop gameplay. |
| Insulated Panel / Thermal Door | Reduces conduction and slows fire/heat spread; damage degrades insulation and can create thermal leaks before full breach. |
| Emergency Vent / Dump Valve | Rapidly vents hot, toxic, or over-pressurized gas/liquid to another atmosphere or exterior reservoir; creates wind/liquid-jet forces if ΔP is high. |
| Furnace | Combustion chamber for smelting; consumes Fuel (H2+O2) at controlled ratio and temperature. |
| Gas Generator | Burns combustible gas → produces electricity + waste heat + CO2/Steam. |
| Gas Fuel Generator (Combustor) | High-rate combustion (90%/tick); higher power output; more violent. |
| Stirling Engine | Closed thermodynamic cycle with H2 / Helium working fluid. |
| Atmosphere Analyzer / Pipe Analyzer | HUD readout of per-gas mole fractions, total moles, pressure, temperature. |
| Suit Storage | Recharges/refills suit canisters; routes per-gas filters via pipe network. |
| Hydroponic Tray / Station | Plants consume CO2, emit O2 (humans-friendly photosynthesis loop). |

All modules use the same atmosphere unit struct; differ only in how they read/write/transform.

## Event Family Extensions

The canonical run-bundle schema ([[references/prototype-run-bundle-schema]]) adds `atmospherics` as a category. All payloads include `parent_event_id` chains so replay can trace causality.

| Event Type | Required Fields |
|---|---|
| `atmospherics.kernel_tick` | tick_id, atmospheres_active, atmospheres_sleeping, perf_us |
| `atmospherics.atmosphere_created` | atm_id, kind, volume_l, parent_atm_id |
| `atmospherics.atmosphere_destroyed` | atm_id, reason (room_unsealed / device_removed / merged_with) |
| `atmospherics.atmosphere_merged` | from_atm_id, to_atm_id, reason |
| `atmospherics.flow` | from_atm_id, to_atm_id, gas, moles_transferred, dt_ticks |
| `atmospherics.aperture_created` | aperture_id, from_atm_id, to_atm_id, area_m2, edge_material, source_event_id, source_kind (door_open / bullet_hole / blast_breach / pipe_rupture / suit_puncture) |
| `atmospherics.aperture_changed` | aperture_id, old_area_m2, new_area_m2, reason (damage / repair / door_motion / seal_failure) |
| `atmospherics.liquid_flow` | from_volume_id, to_volume_id, liquid, liters_transferred, pressure_pa, temperature_k, contamination_tags |
| `atmospherics.liquid_jet_force_applied` | target_id, liquid, impulse_n_s, pressure_pa, source_aperture_id |
| `atmospherics.partial_pressure_changed` | atm_id, gas, old_pp, new_pp |
| `atmospherics.temperature_changed` | atm_id, old_t, new_t, source (conduction / convection / advection / phase_change / combustion / radiation / friction / electrical_load / coolant_loop) |
| `atmospherics.thermal_transfer` | from_id, to_id, joules, route (conduction / fluid / phase_change / radiator / heat_exchanger), material_id, source_event_id |
| `atmospherics.thermal_device_tick` | device_id, mode (heat / cool / exchange / radiator / vent), joules_moved, power_w, coolant_id |
| `atmospherics.phase_change` | atm_id, gas, from_phase, to_phase, moles, parent_event_id |
| `atmospherics.combustion_started` | atm_id, fuel, oxidizer, autoignition_t_reached, parent_event_id |
| `atmospherics.combustion_consumed` | atm_id, reaction_id, moles_consumed, energy_released_j, byproducts, dt_ticks |
| `atmospherics.combustion_stopped` | atm_id, reason (fuel_depleted / oxidizer_depleted / temp_below_threshold / extinguished_by) |
| `atmospherics.room_breach` | room_atm_id, breach_position, area, parent_structure_event_id, peak_dp_pa |
| `atmospherics.structure_rupture` | structure_id, kind (pipe / canister / wall / window), pressure_pa, threshold_pa, parent_event_id |
| `atmospherics.suit_breach` | actor_id, suit_part (helmet / suit / canister), parent_hit_event_id |
| `atmospherics.suit_filter_choked` | actor_id, filter_slot, gas, waste_tank_pressure |
| `atmospherics.breath_inhaled` / `_exhaled` | actor_id, gas, moles, partial_pressure |
| `atmospherics.hazardous_atmosphere_detected` | atm_id, hazard (combustible / toxic / hypoxic / extreme_temp), parent_event_id |
| `atmospherics.wind_force_applied` | target_id (actor / item / debris), force_n, dp_pa |

## Acceptance Tests (ATMOS-A)

Run alongside (not replacing) BODY-A / CHASSIS-A / COLL-A / MAT-* / AI-H / ORIGIN-A.

| Test | Setup | Pass Condition |
|---|---|---|
| ATMOS-A-01 | Sealed 8000 L room with 10 mol O2 + 30 mol N2 at 293 K | `P` = (40 × 8314.46 × 293) / 8000 ≈ 12.18 kPa; HUD shows ratios 25% O2 / 75% N2; `total_moles = 40`. |
| ATMOS-A-02 | Two sealed rooms, A at 100 kPa, B at 0 kPa, vent opened between them | Pressures equalize over time per kernel diffusion step; replay events trace partial pressure decline curve in A and rise in B; `T_mixed` = mass-weighted average. |
| ATMOS-A-03 | Sealed canister with 1 atm O2; heat to 600 K via radiator | `P_after = P_before · T_after / T_before` per Gay-Lussac; replay records temperature_changed event chain. |
| ATMOS-A-04 | Sealed room with 5% Volatiles + 20% O2 + 75% N2 at 280 K; ignition source raises temp to 575 K | Combustion starts at 573.15 K; replay shows `combustion_started` → repeated `combustion_consumed` with stoichiometry 2 V + 1 O2 → 6 CO2 + 3 X; energy release matches `moles_consumed × 286 kJ/mol`; pressure spikes per ideal gas law; if pressure_differential exceeds wall threshold, `room_breach` fires. |
| ATMOS-A-05 | Pipe network with 3 pipe segments (300 L); regulator targets 200 kPa output; input is 400 kPa O2 reservoir | Output pipe stabilizes at 200 kPa; flow stops; replay shows flow events tapering. |
| ATMOS-A-06 | Filtration unit: input pipe with 50% O2 + 50% CO2; CO2 filter type | Input pipe loses CO2 to filtered-out pipe; clean pipe receives O2 only; mole conservation across all three pipes. |
| ATMOS-A-07 | Mars-like ambient (95% CO2, 3 kPa); player exits sealed room into ambient | Player loses sealed room atmosphere via room_breach OR via airlock cycle; if helmet is open, AtmosphericEfficiency drops to 0 (no O2 partial pressure); hypoxia affliction stacks. |
| ATMOS-A-08 | EVA suit with 100% O2 canister + 3x CO2 filters; player breathes for 10 simulated minutes | Canister O2 mol drops linearly per breathing math; waste tank CO2 mol rises linearly per exhalation math; suit pressure stays at setpoint via internal pump. |
| ATMOS-A-09 | EVA suit with Air mix (25% O2 + 75% N2) canister + ONLY CO2 filters (no N2 filter) | Over time, N2 accumulates in suit atmosphere → suit O2 partial pressure drops → hypoxia warning even though canister still has O2. Verifies the filter-mismatch failure mode. |
| ATMOS-A-10 | Helmet flush function | One-shot dump of suit atmosphere into surrounding room; suit then refills from canister to setpoint. |
| ATMOS-A-11 | Cold pipe radiator on Europa with O2 at 6 MPa | O2 condenses into liquid at -111 °C (162 K); replay shows phase_change events; passive liquid drain removes liquid before pipe ruptures. |
| ATMOS-A-12 | Wind force scenario: sealed room at 200 kPa, valve to vacuum opens, dropped item nearby | Item gets accelerated toward vacuum boundary; replay shows wind_force_applied events with force scaling proportional to ΔP and item mass. |
| ATMOS-A-13 | Plant in hydroponic tray in CO2-rich room | CO2 mol drops over time; O2 mol rises; replay shows the photosynthesis exchange events. |
| ATMOS-A-14 | Furnace combustion math validation | Add 1 mol O2 + 2 mol H2 + ignite at 300 K; expected T_after = (300×61.9 + 563452) / 234.515 = ~2480 K; expected P_after = 2.9 × P_before × (T_after/T_before). Run-bundle records exact match within float tolerance. |
| ATMOS-A-15 | Determinism replay | Same seed, same scenario, same actor inputs → identical atmospheric event stream byte-for-byte. |
| ATMOS-A-16 | Bullet-hole depressurization | Rifle round punches a 2 cm² aperture between a 200 kPa room and vacuum; flow curve follows aperture area and ΔP; actor/item wind impulse is recorded; patch tool reduces area to zero and pressure stabilizes. |
| ATMOS-A-17 | Liquid jet / flooding | Pressurized water tank is punctured into a room; liquid jet applies impulse to a loose item, wets/cools the struck surface, then floods lower cells by gravity; replay records liquid_flow + liquid_jet_force_applied. |
| ATMOS-A-18 | Material heat transfer | Hot metal wall adjacent to cold insulated room, uninsulated metal room, and active coolant loop: uninsulated room warms fastest, insulated room slowest, coolant loop removes heat while consuming power; all energy deltas reconcile within tolerance. |
| ATMOS-A-19 | Player thermal techniques | Overheated base module can be recovered by two valid methods (radiator loop OR emergency vent/flood). Both restore function but leave different risks: heat dumped outside vs pressure/liquid hazard. |

## Modding Contract

- Add a new gas: data row in `content/gases/` with molar mass, specific heat, latent heat, condensation/freeze pressures + temperatures, autoignition table per partner gas, mole fraction of which atmospheres it belongs to by default.
- Add a new combustion reaction: data row in `content/reactions/` with reactants + stoichiometry + products + energy + autoignition T + min ratio thresholds + reaction rate function.
- Add a new device class: AGENTS.md per-crate contract; declare which atmosphere kinds it connects (room-side / pipe-side / suit-slot / canister-slot); declare per-tick read/write hooks.
- Add a new planet ambient: data row in `content/worlds/` with composition, pressure, temperature range, gravity_g, wind_velocity_mps, weather variation table.
- Schema validates via `cargo run -p cf-mod -- validate content/gases/` etc.

## Performance Posture

- Active-region scheduling per [[spec/native-implementation-backlog#M5.6 — Material Kernel]] — only atmospheres whose partial pressures, temperatures, or contents change run the full kernel step. Sleeping atmospheres are checksummed and skipped.
- Sealed adjacent room cells collapse into a meta-atmosphere for the kernel pass; partial-pressure HUD queries break them apart on demand.
- Combustion and phase change run on dirty atmospheres only (any reaction = dirty).
- Per-tick kernel budget per the No-Compromise Performance Defaults section in [[spec/prototype-roadmap]]: 60 Hz default; 120 Hz path validated; 128 Hz candidate.
- Determinism: kernel runs on the deterministic CPU thread with fixed iteration order; no platform-specific atomics in the inner loop.

## Out Of Scope (during M0..M4)

- No atmosphere kernel during M0-M3.
- No room detection during M0-M3.
- No suit life-support modeling during M0-M4 (M4 HUD shows placeholder bars).
- No combustion during M0-M4 (the placeholder `--debug-inject-panic-at-tick` in M0 is unrelated).
- No pipe networks in M5 (M5 owns chassis, equipment, damage; the pipe-network crate `cf-atmos` lands in M5.9).
- No actor breathing in M5.5-M5.7 (M5.5 owns collision, M5.6 owns material kernel, M5.7 owns hazard package; the breathing math lands in M5.8 origin resource pass + M5.9 atmospherics-grade kernel).

## Source Trail

- [[references/sources]] — see "Stationeers atmospherics research (29+ sources)" section
- [[spec/origin-reaction-and-resource-model]]
- [[spec/full-collision-physics-plan]]
- [[spec/body-damage-model]]
- [[spec/chassis-armor-mechs-and-origins]]
- [[spec/equipment-loadout]]
- [[references/prototype-run-bundle-schema]]
- [[decisions/dr-007-terrain-material-model]]
- [[decisions/dr-036-systemic-material-simulation-direction]]
- [[decisions/dr-037-stationeers-grade-atmospherics-direction]]
- [[research-log/2026-05-06-atmospherics-and-chemistry-stationeers-research]]

## Change Log

- 2026-05-06: Captured during M1 from user-supplied design intent ("real chemistry and pressures and wind — just like in stationeers"). Status: `design-intent-post-m1`. 13 wiki scrapes locked: Atmosphere page, PV=nRT page, Volatiles, Oxygen, Carbon Dioxide, Pollutant, Hydrogen, Phase Change guide, Furnace temperature/pressure math, EVA Suit, Hardsuit, Active Vent, Air. Key locked numbers: R = 8314.46; combustion stoichiometry; specific heat capacities (O2=21.1, N2=20.6, CO2=28.2, Volatiles=20.4, Pollutant=24.8, N2O=23, H2O=72, H2=20.4); breathing rate 0.0048 mol/tick × BreathingRate × BreathingEfficiency; min inhaled gas partial pressure 16 kPa; suit pressure tolerance 11-300 kPa; suit max filter pressure 4052 kPa.
