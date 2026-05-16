# M3B — ONI-Grade Per-Tile Element Model + Heat Transfer Caps + Mass-Based Pressure

## Status

`active`

## Intent

Extend M3's chunked terrain with the **ONI per-tile single-element model**: each pixel/tile carries `element_id + mass_kg + temperature_K` as canonical state, with ONI-grade heat transfer math + thermal conductivity caps + mass-based pressure damage. This is the **per-tile substrate** that M19's room-aggregated atmospherics handshakes with at room/cell boundaries — gives Corefall ONI's readable-per-tile fidelity AND Stationeers' room-as-network performance.

## Player-facing behavior

- Every pixel/tile has visible element + mass + temperature on F-key inspect.
- Outdoor + cave + breach + per-pixel zones use the per-tile model (ONI-grade granularity).
- Sealed rooms continue using M19's room-aggregated atmospherics (Stationeers PV=nRT) for performance.
- Heat moves between adjacent tiles at a bounded rate per ONI's conductivity formula: `transfer = min(cap, k × A × ΔT / mass_scale)`. Cap prevents single-tick thermal flash.
- Mass-based pressure: too much liquid in a tile crushes the tile + any actor inside. Stack a column of water 16 tiles tall = bottom-tile pressure exceeds tile strength → tile breaks.
- Vacuum tiles (mass < 1 g) become **perfect insulators** — no heat transfer through them. ONI parity for vacuum chambers.
- The "one element per cell rule": each tile holds ONE element, NOT a gas mixture (per ONI). When two gases meet, lighter rises + heavier sinks rather than mixing in place. Visually + numerically simpler than Stationeers' molar fractions for outdoor settings.

## Crates / modules touched

| Crate | Status | What changes |
|---|---|---|
| `cf-terrain::tile_element` | NEW | Per-tile (per-pixel) `TileElement { element_id, mass_kg, temperature_k }` state buffer (8 bytes per tile × 256×256 chunk = 512KB per chunk; manageable). |
| `cf-terrain::heat_transfer` | NEW | ONI-grade heat conduction kernel: per-tick neighbor-tile heat exchange with mass scaling + per-element conductivity × area × ΔT cap. |
| `cf-terrain::pressure_damage` | NEW | Mass-based pressure crush: when a tile's stored mass exceeds tile-strength threshold, damage cascades. |
| `cf-material::registry` | READ | Per-element thermal fields (`default_mass_per_tile_kg`, `specific_heat_capacity_j_per_kg_k`, `thermal_conductivity_w_per_m_k`, `high_temp_transition_target`, `low_temp_transition_target`, `melt_temp_k`, `freeze_temp_k`, `density_kg_per_m3`, `state` enum) are **owned by M15C**; M3B reads them at runtime. M3B does not author or mutate the registry schema. |
| `cf-atmos::room_tile_bridge` | CONSUMER | Per-tile substrate that **M19G's canonical bridge** (`cf-atmos::hybrid_strategy` + `room_to_tile` + `tile_to_room` + `boundary_detection`) reads from. M19G owns the handshake code in `cf-atmos`; M3B exposes the per-tile state surface (element + mass + temperature) the bridge consumes. M3B does NOT author its own `room_tile_bridge.rs`. |
| `cf-control` | MODIFY | Per-tick: heat transfer pass + pressure damage pass. The room ↔ tile handshake pass is owned by M19G; M3B does not register its own handshake pass. |
| `cf-replay` | MODIFY | 4 new event schemas. |
| `cf-ui` | MODIFY | F8 toggle: per-tile element + mass + temperature overlay. |
| `cf-mod` | MODIFY | Validate per-element values are within ONI-realistic ranges. |

## Files

- `game/crates/cf-terrain/src/tile_element.rs` (NEW)
- `game/crates/cf-terrain/src/heat_transfer.rs` (NEW)
- `game/crates/cf-terrain/src/pressure_damage.rs` (NEW)
- `game/crates/cf-material/src/registry.rs` (READ — schema owned by M15C; M3B consumes the per-element thermal fields without modifying them)
- `game/crates/cf-atmos/src/room_tile_bridge.rs` (OWNED BY M19G — M3B exposes the per-tile state the bridge reads; M19G's `hybrid_strategy.rs` + `room_to_tile.rs` + `tile_to_room.rs` + `boundary_detection.rs` are the canonical files. M3B does not create `room_tile_bridge.rs`.)
- `game/crates/cf-control/src/engine.rs` (MODIFY: per-tick heat transfer + pressure damage passes only; handshake pass owned by M19G)
- `game/crates/cf-ui/src/tile_inspect_overlay.rs` (NEW)
- `game/crates/cf-replay/schemas/event/tile_heat_transferred.json` (NEW)
- `game/crates/cf-replay/schemas/event/tile_pressure_crushed.json` (NEW)
- `game/crates/cf-replay/schemas/event/tile_phase_transitioned.json` (NEW)
- `game/crates/cf-replay/schemas/event/room_tile_handshake.json` (OWNED BY M19G — consumed by M3B; M3B does not author this schema. See M19G `boundary_room_to_tile.json` + `boundary_tile_to_room.json`.)
- `game/content/scenarios/m3b_liquid_pressure_crush.ron` (NEW)
- `game/content/scenarios/m3b_thermal_cascade_demo.ron` (NEW)
- `game/content/scenarios/m3b_vacuum_insulation_test.ron` (NEW)

## The per-element registry (locked launch numbers — ONI parity)

| Element | State | SHC (J/kg·K) | TC (W/m·K) | Default mass/tile (kg) | Melt (K) | Boil/Sublim (K) |
|---|---|---:|---:|---:|---:|---:|
| Oxygen (gas) | Gas | 1.005 | 0.024 | 0.5-2.0 | 54.4 | 90.2 |
| Carbon Dioxide | Gas | 0.846 | 0.0146 | 0.5-2.0 | 216.6 | 194.65 (sublim) |
| Hydrogen | Gas | 14.30 | 0.168 | 0.05-0.2 | 13.99 | 20.27 |
| Polluted Oxygen | Gas | 1.005 | 0.024 | 0.5-2.0 | n/a | 90.2 |
| Steam | Gas | 4.179 | 0.016 | 0.5-2.0 | n/a | 373.15 |
| Water | Liquid | 4.179 | 0.609 | 1000 | 273.15 | 373.15 |
| Polluted Water | Liquid | 4.179 | 0.58 | 1000 | 273.15 | 373.15 |
| Oil (Crude) | Liquid | 1.690 | 0.146 | 870 | 230 | 525 |
| Petroleum | Liquid | 1.760 | 0.16 | 700 | 245 | 540 |
| Magma (Molten Rock) | Liquid | 1.0 | 2.0 | 3000 | n/a | n/a |
| Liquid Methane | Liquid | 3.481 | 0.21 | 426 | 90.7 | 111.7 |
| Liquid Oxygen | Liquid | 1.696 | 0.151 | 1140 | 54.4 | 90.2 |
| Iron (Solid) | Solid | 0.449 | 80.4 | 800 | 1811 | 3134 |
| Concrete | Solid | 0.880 | 1.5 | 2000 | n/a | n/a |
| Dirt | Solid | 1.480 | 0.6 | 800 | n/a | n/a |
| Sandstone | Solid | 0.83 | 0.68 | 1800 | n/a | n/a |
| Ice | Solid | 2.050 | 2.2 | 920 | 273.15 | n/a |
| Insulation (foam) | Solid | 1.0 | 0.05 | 800 | n/a | n/a |
| Vacuum | Vacuum | 0 | 0 | 0 | n/a | n/a |

Worker may add the remaining ~80 ONI elements per the same ONI wiki + Stationeers research. Schema is mod-extensible.

## ONI heat transfer formula (locked)

```
heat_transferred_per_tick_J = min(
    transfer_cap,
    k_eff × (T_a - T_b) × dt_seconds × mass_scaling
)

where:
- k_eff = harmonic mean of the two tiles' thermal_conductivity
- mass_scaling = min(mass_a, mass_b) / 1000.0  (scaled so a 1000 kg tile is baseline)
- transfer_cap = 10 J/tick (default; tunable to prevent thermal flash)
```

Per ONI: heat transfer is bounded to prevent runaway temperature swings even at high conductivity. Vacuum tiles have k=0 → no transfer.

## Acceptance criteria

```gherkin
Scenario: Per-tile element + mass + temperature inspected
  Given a scenario with a water tile next to a vacuum tile
  When the player F-inspects the water tile
  Then HUD shows: element=Water, mass=1000kg, temp=298.15K, state=Liquid
  And the vacuum tile shows: element=Vacuum, mass=0kg, temp=undefined, state=Vacuum

Scenario: Heat flows between adjacent tiles with cap
  Given a 100°C iron tile next to a 0°C iron tile
  When 60 ticks elapse
  Then each tick transfers ~5 J (bounded by cap)
  And temperatures converge slowly (not single-tick flash)
  And tile_heat_transferred fires per tick

Scenario: Vacuum is a perfect insulator
  Given an iron tile at 100°C next to a vacuum tile
  When 600 ticks elapse
  Then iron tile temperature drops < 0.1°C (essentially zero transfer)
  And vacuum tile remains undefined-temp
  And ONI parity confirmed: vacuum thermos design works

Scenario: Liquid pressure crushes tile + actor
  Given a column of 20 water tiles stacked vertically
  When pressure on the bottom tile exceeds threshold (mass × g × column_height > tile_strength)
  Then tile_pressure_crushed fires
  And the bottom tile breaks (terrain mutation)
  And an actor under the column receives crush damage via M14 fall_impulse_chain

Scenario: Phase transition triggers when temp crosses threshold
  Given a water tile at 100°C
  When heat raises its temperature to 373.15K
  Then tile_phase_transitioned fires
  And the water tile becomes Steam (gas)
  And subsequent tick: steam rises (gas density layering per M14B)

Scenario: Room ↔ tile handshake reads M3B per-tile state (handshake owned by M19G)
  Given a sealed room (M19 PV=nRT atmosphere) + adjacent outdoor (M3B per-tile)
  When the door opens
  Then M19G's `boundary_detection` pass detects the aperture
  And M19G fires `boundary_room_to_tile` (canonical handshake event; NOT M3B's `room_tile_handshake`)
  And M19G's aperture-flow formula reads M3B's per-tile element + mass + temperature as the consumer surface
  And on the room side, M19's room atmosphere reduces by the exported mass (M19G writes)
  And on the tile side, M3B's per-tile state buffer is mutated by M19G's `room_to_tile` pass
  And M3B does NOT register its own handshake pass; M19G is the sole writer

Scenario: One element per cell rule preserved
  Given a tile with 1000 kg water
  And an attempt to add 100 kg oil to the same tile
  When the resolver runs
  Then oil cannot occupy the same tile
  And oil routes to the nearest empty tile (vertical preference per density)
  And no mixture forms (ONI rule preserved)

Scenario: Determinism across heat transfer
  Given two engines with same seed + identical thermal scenario
  When 600 ticks elapse with mixed materials
  Then identical per-tile temperatures
  And identical event sequence
  And SaveBlob.checksum identical at tick 600

Scenario: Element registry validation (cf-mod)
  Given a mod author adds a new element with thermal_conductivity = 1e10
  When cf-mod validate runs
  Then validation rejects with warning "TC > 1000 W/m·K unrealistic; use insulator instead"
  And mod author corrects or accepts override
```

## Out of scope

- Per-tile gas mixtures (Stationeers PV=nRT) — that's M19 baseline; M3B is per-tile single-element only.
- Per-tile gas compression (sub-saturated gas at variable pressure) — abstracted via mass-per-tile.
- Per-tile humidity (water vapor as mass within a gas tile) — M19F owns humidity per-room; M3B treats steam as a separate gas tile per element.
- Element table beyond 19 launch elements — modders extend via RON.

## Dependencies

- M3 chunked terrain (done): the underlying terrain buffer + dirty rect system
- M15 active material kernel (active): material reactions consume per-tile element state
- M19 atmospherics-grade kernel (active): the room-aggregated counterpart that handshakes with M3B
- M19G Room ↔ Tile Atmospheric Bridge (active): **canonical owner of the handshake protocol + `hybrid_strategy` / `room_to_tile` / `tile_to_room` / `boundary_detection`**. M3B is a per-tile substrate; M19G is the bridge writer.
- M14B gravity field + wind force (NEW M14B): mass-based liquid flow uses gravity
- M28B base thermal engineering (NEW M28B): per-material thermal conductivity table consumer
- M14 collision physics (done): pressure damage routes through fall_impulse_chain

## Notes for the implementer

- ONI uses kg-scale per-tile mass; Stationeers uses moles. M3B adopts kg-scale (matches ONI + simpler math).
- Heat transfer cap (10 J/tick default) is the critical tunable — controls "fast enough to be visible / slow enough to be readable."
- Vacuum-as-perfect-insulator: hard-coded check; thermal_conductivity = 0 → skip the per-tile pair entirely.
- Phase transition uses M15's existing phase_changes table; M3B reads it.
- Per-tile element checksum must be deterministic — chunk-checksum extension to include per-tile element ID + mass × 1000 (truncated to int for stability).
- The room ↔ tile handshake is THE key bridge between Stationeers room model + ONI tile model. **M19G owns the bridge implementation** (`hybrid_strategy.rs` + `room_to_tile.rs` + `tile_to_room.rs` + `boundary_detection.rs` in `cf-atmos`). M3B is the per-tile substrate the bridge reads from; M3B does NOT author `room_tile_bridge.rs`.
