# M9C — Static Fortifications + Defensive Structures

## Status

`active`

## Intent

**M9C is the static-fortifications roster milestone** — ships the authored defensive-structure content that Cortex Command players expect: MG nest module + ammo box + sandbag wall (3 height tiers) + spotter scope + tripod-mounted variant; watchtower (3 height tiers) + spotlight + observation post; minefield system (4 mine kinds + minesweeper tool + bomb-disposal robot); barbed wire + razor wire + electrified fence (per-actor cut/pass/damage); anti-tank ditch + dragon's teeth + tank trap; camo netting; bunker firing slit. Each fortification is a first-class authored asset with per-instance state, M14 damage routing, M22 AI doctrine consumption, and modder-overridable RON definitions.

**Why this milestone exists:** M9B authored the trench (the hole in the ground). M9C authors what goes IN, ON, and AROUND the trench — the wall the player shoots from behind, the MG that locks the corridor, the wire that slows the assault, the mines that punish careless advance, the tower that sees the enemy a screen away, the tank trap that stops the IFV cold. Cortex Command's bunker grammar (`Door`, `BunkerEntrance`, `MGNest`, `Watchtower`) is the reference; M9C ports that grammar to Corefall's chassis/damage/AI stack.

M9C promise: **"a defended position has the right authored toolset — MGs behind sandbags inside a fire-step trench, with a spotter in a watchtower, mines forward of the wire, dragon's teeth blocking vehicle approaches — and every piece has real HP, real cover state, real per-actor interaction (cut wire, sweep mine, dazzle spotlight), real AI consumers."**

## Player-facing behavior

### MG nest module + ammo box + tripod variant

| Asset | Footprint | HP | Provides | Cost | Build time |
|---|---|---|---|---|---|
| `mg_nest_static` | 4×2 (4 px tall) | 800 | Mounted MG with 360° base traverse, 90° firing arc, full cover when crewed | 60 sandbags + 40 iron + 1 heavy_mg_7_62mm | 60s |
| `ammo_box_mg` | 2×1 | 200 | 800-round belt cache; auto-feeds adjacent MG nest | 4 iron + 200 ammo cost | 8s |
| `mg_tripod_portable` | 1×1 (deploys to 3×2) | 400 | Squad-portable MG; 4s deploy + 4s pack-up; M16A stamina-cost movement | 20 iron + 1 heavy_mg_7_62mm | 30s |
| `spotter_scope` | 1×1 | 100 | Adds +50% target acquisition range to adjacent crewed MG / sniper position | 2 glass + 4 iron + 1 electronics | 12s |
| `bunker_firing_slit` | 2×1 (in concrete wall) | 1600 | Concrete-embedded slit; defender inside has Full cover; incoming rounds blocked except via slit (8 px aperture) | 40 concrete + 1 heavy_mg / sniper | 90s (pre-built in template) |

- **MG nest crewing**: actor uses `act.player.crew_fortification { id }` → enters the nest, gains Full cover, weapon control rebound to MG (different than personal weapon). Exit via `act.player.uncrew_fortification`.
- **Ammo box auto-feed**: an adjacent `ammo_box_mg` feeds the MG nest until empty; player swaps in a fresh box (1 inventory slot, 50 kg) to reload.
- **Tripod portable** is the **squad-portable** counterpart: M22 squad doctrine lets one actor carry + deploy the tripod, then crew it; classic Cortex Command "set up the MG to lock the corridor" play.
- **Spotter scope** is a passive force-multiplier: M22 target_selection consults adjacent spotter scope's "spotter_bonus" field. AI snipers/MGs in cover with a paired spotter actor get +50% effective acquisition range and -25% target re-acquisition time (M22 AI-OBS-A-01 doctrine).
- **Bunker firing slit** is pre-built into M28F bunker templates; HP 1600 vs HP 800 for a standalone MG nest. A penetrating round (HEAT, M14C) through the slit reaches the crewed actor; rounds that hit the surrounding concrete bounce harmlessly off the parapet.

### Sandbag wall (3 height tiers)

| Tier | Height | HP | Cover (Standing) | Cover (Crouched) | Cost | Build time |
|---|---|---|---|---|---|---|
| `sandbag_low` | 4 px | 200 | None | Partial | 4 sandbags | 4s |
| `sandbag_mid` | 8 px | 400 | Partial | Full | 8 sandbags | 8s |
| `sandbag_high` | 12 px (chest-high standing) | 600 | Full | Full | 12 sandbags | 12s |

- A `sandbag` is a 1-slot inventory item; M30 mining produces sand → M32 crafting fills sandbags (10 sandbags per sand-bag-pile). Crafted in field by `entrenching_tool` + sand + burlap (or just sand for emergency fill).
- Sandbag walls **degrade per-pixel** (M14): hit a high wall with sustained MG and the top row erodes first, downgrading the tier (high → mid → low → gone) over time. `sandbag_eroded` event fires per tier-drop.
- **Repair**: `act.player.repair_fortification { id }` consumes sandbags from inventory equal to the missing HP.
- **Tactical role**: sandbag walls are the **modular cover** primitive — built fast, repaired fast, eroded by sustained fire. The trade-off vs concrete wall is repairability + footprint flexibility.

### Watchtower (3 height tiers) + spotlight + observation post

| Asset | Height | Footprint | HP | Provides | Cost | Build time |
|---|---|---|---|---|---|---|
| `watchtower_t1` | 16 px | 3×3 base | 600 | +1 LOS tile; standing-cover at top platform; vision over low cover | 40 wood + 10 iron | 90s |
| `watchtower_t2` | 32 px | 4×4 base | 1200 | +3 LOS tiles; cover against ground fire; rooftop mount slot (MG / sniper) | 80 iron + 20 steel + 8 wood | 150s |
| `watchtower_t3` | 48 px | 5×5 base | 2400 | +6 LOS tiles; multi-actor platform; integrated radio repeater | 200 steel + 40 concrete + 20 electronics + 1 radio_relay | 240s |
| `spotlight` | 1×1 (mounted on watchtower) | 100 | Cone-of-light 24-tile range; reveals concealed actors in cone | 4 iron + 2 glass + 1 electronics | 12s |
| `observation_post` | 4×3 | 400 | Hardened spotter station; full cover; +75% acquisition bonus to faction-wide artillery (M44D SPG) | 40 concrete + 4 glass + 2 electronics | 60s |
| `radio_repeater` | 1×1 | 200 | Extends squad-radio range by 100 tiles; req for off-screen squad coordination (M44B) | 8 electronics + 1 antenna | 20s |

- **Spotter role**: actor in a watchtower / observation_post / spotter_scope station marks targets — M22 AI-OBS-A-01 doctrine triggers: spotter actor emits `spotter.target_marked { target_id, target_pos }` per LOS-confirmed target; squad members with relevant weapons receive +50% acquisition rate on the marked target.
- **Spotlight**: nighttime gameplay (M31 day/night cycle when shipped); cone-of-light reveals concealed/prone enemies. Actor in cone receives "illuminated" status (M16A affliction surface placeholder). Spotlight can be destroyed (HP 100) or temporarily dazzled by a flashbang (M15D).
- **Radio repeater** extends the radio module's range (M44D radio is hard-killable component): a forward observation_post with repeater enables long-range squad coordination + artillery call.
- **Height tradeoff**: taller tower = more visible to enemies + bigger collateral when destroyed. T3 tower destruction (M14F lateral collapse) drops 48 px of debris over a 5-tile radius.

### Minefield system (4 mine kinds + minesweeper + bomb disposal)

| Mine kind | Trigger | Yield | Detection | Cost |
|---|---|---|---|---|
| `mine_proximity` | Actor within 1.5 tiles, any side | 80 J HE blast (1.5-tile radius) | Detected by `minesweeper` within 3 tiles | 2 iron + 1 explosive + 1 trigger |
| `mine_pressure` | Actor stance:Standing/Crouched directly over | 120 J HE blast (1-tile radius); cripples legs | Hard to detect (requires `minesweeper` directly adjacent) | 2 iron + 2 explosive + 1 trigger |
| `tripwire_mine` | Actor crosses linked tripwire | 60 J HE + alerts faction | Visible tripwire (1-tile line of sight); mine itself hidden | 2 iron + 1 explosive + 1 trigger + 1 wire |
| `ied_chain` | Manual remote detonator OR proximity OR pressure | 200-400 J HE (configurable yield per IED); chains: detonating one IED daisy-fires all linked IEDs within 4 tiles | Cabled IEDs visible if `wire_visible: true`; hidden IEDs detected by minesweeper at 2 tiles | 4 iron + 4 explosive + 1 trigger + 1 wire (per node) |

- **Minefield template**: a `MineField` is a declarative cluster of mines + spacing + density; `content/mine_fields/<id>.minefield.ron`. Defenders place via `act.player.deploy_minefield_template { id, origin }` consuming pooled mine inventory.
- **Minesweeper tool** (T1 handheld): held in equipment slot; emits 3-tile radius detection ping every 2s; revealed mines render with yellow marker visible only to the sweeping faction; `minesweeper_detected` event fires.
- **Bomb disposal robot** (T2 deployable; M44C IFV-grade chassis): remote-controlled tracked robot, slow (40 px/s), survives a single mine blast (HP 1200 with reactive armor). Disarms mines on contact (`act.player.disarm_mine { mine_id }`).
- **Manual disarm**: actor crouched adjacent to a detected mine for 6 seconds disarms it; failure (interrupt / damage / movement) re-arms; `mine_disarmed` / `mine_disarm_failed` events fire.
- **Pressure vs proximity distinction**: a vehicle (M44D tank) crossing proximity mines triggers them but tank armor absorbs most damage; pressure mines designed for anti-personnel — armor track may absorb but per-component leg/track damage is rerouted through M44C.
- **IED chain**: characteristic Cortex Command "engineer rigs the whole entryway" play — player rigs 5 IEDs in series, retreats, detonates remotely when enemy enters kill zone. M14C cookoff + M14 fragmentation routes through existing damage stack.

### Barbed wire + razor wire + electrified fence

| Asset | HP | Per-actor effect on cross | Per-actor effect on cut | Damage | Cost |
|---|---|---|---|---|---|
| `barbed_wire` | 200 | Speed -75% while crossing; "snagged" 0.5s; possible torn-clothing visual | Cut with wire_cutters (M30B tool); 3-second hold | 1 dmg/tick while crossing | 4 iron + 1 wire |
| `razor_wire` | 300 | Speed -90%; "snagged" 1.0s; visible bleed effect | Cut with wire_cutters; 4-second hold; cutter takes 1 HP damage | 4 dmg/tick while crossing; M16A laceration affliction | 6 iron + 1 wire (concertina coil) |
| `electrified_fence` | 400 | If powered (M29 grid): instant 80 J shock damage to non-insulated actor; cannot cross while powered | Cut while powered = electrocution; must depower first | 80 dmg shock OR 1 dmg/tick if depowered (acts as barbed_wire) | 6 iron + 2 wire + 1 insulator + power coupling |
| `concertina_roll` | 250 | Speed -85%; 0.75s snag; covers 4 tiles | Cut wire_cutters; 4-second hold per coil section | 3 dmg/tick | 12 iron + 3 wire |

- **Per-actor cut/pass/damage** is the model: each wire instance maintains state per actor (crossing / snagged / cut), drives speed/damage/affliction per the table.
- **Wire cutters** is the dedicated tool (M30B T1 tool): equipped + held [E] adjacent to wire for the cut-time-seconds; emits `wire_cut` event with wire_id. Without cutters, an actor can FORCE through (Speed -98% + 8 dmg/tick + likely affliction).
- **Electrified fence** consumes M29 power grid (default 240V); a fence node draws 1 kW continuous. Power loss → fence acts as barbed_wire (still damaging on cross but no shock). Player can shut off feeder breaker (M29 grid) before cutting; `fence_depowered` event signals safe-to-cut.
- **Vehicle interaction**: light vehicles cross wire (taking minor track damage); heavy vehicles (tanks) crush wire (destroying it on contact, no damage to vehicle). `wire_crushed_by_vehicle` event fires.

### Anti-tank ditch + dragon's teeth + tank trap

| Asset | Footprint | HP | Vehicle effect | Infantry effect | Cost |
|---|---|---|---|---|---|
| `anti_tank_ditch` | 8×4 (8 px wide × 4 px deep) | n/a (terrain carve) | Light vehicle: 30% stuck chance; Heavy: 80% stuck chance (1-2 ticks to escape via per-component traction); SPG/howitzer: impassable | Infantry: 25% slip, partial cover when in ditch | 0 (carved by engineer's tool) |
| `dragons_teeth` | 1×1 each (placed in 3-row staggered pattern) | 1200 per tooth (concrete pyramid) | Light: full stop on contact; Heavy: stops + per-component suspension damage | Infantry: walks through gaps freely | 40 concrete per tooth |
| `tank_trap_x` | 2×2 (welded I-beam X) | 800 | Light vehicle: stops + per-component damage; Heavy: damages then can plow through (~5s) | Infantry: walks through gaps; cover from one direction | 30 steel + 8 weld |
| `bollard_concrete` | 1×1 | 600 | Stops light vehicle / soft-stops heavy; non-block to infantry | None | 20 concrete |

- **Anti-tank ditch** is a deeper authored carve than a trench — 8×4, mandates `engineering_tool` (T2 from M30B) and 60s to dig. Different cross-section from M9B trench (wider, no cover state); ditch-pass-state determines vehicle stuck-on-edge behavior.
- **Dragon's teeth** is the iconic WWII anti-tank obstacle: 3-row staggered concrete pyramids, infantry walks through, vehicles physically blocked. M14 per-component vehicle damage routes through tooth contact.
- **Tank trap X** is the welded-I-beam X (Czech hedgehog reference) — cheaper than dragon's teeth but destructible.
- **Combined defense**: ditch + dragon's teeth + electrified fence layered = classic Cortex Command "vehicle cannot pass without engineer + infantry support" gameplay.

### Camo netting

- `camo_netting`: a 4×4 overlay placed on any structure / vehicle / actor cluster; gives `concealed: true` status against M22 visual detection at > 8 tiles range (only matters against AI/observers; not against spotlight or thermal scope).
- HP 100; flammable (M15D fire propagates fast).
- Player + AI doctrine: spotters with thermal scope (M44D ARV) bypass camo netting; spotlights bypass it; ground-level visual detection at < 8 tiles bypasses it.

## Crates / modules touched

| Crate | Status | What changes |
|---|---|---|
| `cf-fortification` | NEW | Static-fortification kernel: per-instance HP + crew slots + per-faction permissions + repair API + asset registry. |
| `cf-fortification::mg_nest` | NEW | MG nest crewing logic, ammo-box auto-feed, tripod-deploy state machine. |
| `cf-fortification::watchtower` | NEW | Watchtower tier kernel, spotter role, spotlight cone, observation_post, radio_repeater. |
| `cf-fortification::minefield` | NEW | Mine instance state machine, trigger evaluation (proximity/pressure/tripwire/IED chain), detection masking. |
| `cf-fortification::wire` | NEW | Wire kernel: per-actor crossing state, cut-with-tool, electrified-power coupling. |
| `cf-fortification::anti_tank` | NEW | Anti-tank ditch carve, dragon's teeth + tank trap collision + per-vehicle damage routing. |
| `cf-fortification::camo` | NEW | Camo netting concealment overlay; bypass-rule consumers (thermal, spotlight, proximity). |
| `cf-equipment::tools` | MODIFY | Add `minesweeper`, `wire_cutters`, `engineering_tool`, `entrenching_tool` (shared with M9B). |
| `cf-equipment::deployables` | NEW | Bomb-disposal robot deployable item with M44C chassis grammar. |
| `cf-control` | MODIFY | New cfctl: `crew_fortification`, `uncrew_fortification`, `deploy_minefield_template`, `disarm_mine`, `cut_wire`, `repair_fortification`, `deploy_mg_tripod`, `mark_spotter_target`, `power_fence`, `unpower_fence`. |
| `cf-actor::stance` | MODIFY | New stance `Crewing { fortification_id }` with cover-state = Full + alternate input map. |
| `cf-ai` | MODIFY | Doctrines: AI-MG-A-02 (crew nearest empty MG), AI-OBS-A-01 (spotter mark), AI-ENG-A-03 (lay mines / cut wire / disarm), AI-AT-A-04 (vehicle approach + anti-tank ditch evade). |
| `cf-ui::fortification_hud` | NEW | Per-fortification HP bar, ammo-box state, spotlight cone preview, minefield-warning banner. |
| `cf-replay` | MODIFY | 16 new event schemas. |
| `cf-mission` | MODIFY | 10 new scenarios registered. |
| `cf-mod` | MODIFY | Validate `content/fortifications/*.ron` + `content/mine_fields/*.minefield.ron`. |
| `cf-render-2d` | MODIFY | Per-fortification sprite layers, spotlight cone shader, wire visuals (barbed/razor/electric/concertina), camo netting overlay. |
| `cf-audio` | MODIFY | New cues: `mg_nest_burst`, `mine_arming_beep`, `wire_snag_pain`, `electrified_shock_zap`, `spotlight_relay_click`, `tripwire_snap`. |

## Files

- `game/crates/cf-fortification/Cargo.toml` (NEW)
- `game/crates/cf-fortification/src/lib.rs` (NEW)
- `game/crates/cf-fortification/src/mg_nest.rs` (NEW)
- `game/crates/cf-fortification/src/watchtower.rs` (NEW)
- `game/crates/cf-fortification/src/minefield.rs` (NEW)
- `game/crates/cf-fortification/src/wire.rs` (NEW)
- `game/crates/cf-fortification/src/anti_tank.rs` (NEW)
- `game/crates/cf-fortification/src/camo.rs` (NEW)
- `game/crates/cf-fortification/src/sandbag.rs` (NEW)
- `game/crates/cf-equipment/src/tools.rs` (MODIFY: minesweeper, wire_cutters, engineering_tool)
- `game/crates/cf-equipment/src/deployables/bomb_disposal_robot.rs` (NEW)
- `game/crates/cf-control/src/server.rs` (MODIFY: 10 new cfctl methods)
- `game/crates/cf-control/src/schemas.rs` (MODIFY: param + observe structs)
- `game/crates/cf-actor/src/stance.rs` (MODIFY: Crewing stance + alternate input map)
- `game/crates/cf-ai/src/mg_doctrine.rs` (NEW)
- `game/crates/cf-ai/src/observer_doctrine.rs` (NEW)
- `game/crates/cf-ai/src/engineer_doctrine.rs` (NEW)
- `game/crates/cf-ai/src/anti_tank_doctrine.rs` (NEW)
- `game/crates/cf-ui/src/fortification_hud.rs` (NEW)
- `game/crates/cf-render-2d/src/fortification_layers.rs` (NEW)
- `game/crates/cf-render-2d/src/spotlight_cone.rs` (NEW)
- `game/crates/cf-render-2d/src/wire_visuals.rs` (NEW)
- `game/crates/cf-audio/src/registry.rs` (MODIFY: fortification audio family)
- `game/crates/cf-mission/src/m9c_scenarios.rs` (NEW)
- `game/crates/cf-replay/schemas/event/mg_nest_crewed.json` (NEW)
- `game/crates/cf-replay/schemas/event/mg_nest_uncrewed.json` (NEW)
- `game/crates/cf-replay/schemas/event/mg_nest_fired_burst.json` (NEW)
- `game/crates/cf-replay/schemas/event/mg_tripod_deployed.json` (NEW)
- `game/crates/cf-replay/schemas/event/ammo_box_depleted.json` (NEW)
- `game/crates/cf-replay/schemas/event/sandbag_eroded.json` (NEW)
- `game/crates/cf-replay/schemas/event/watchtower_destroyed.json` (NEW)
- `game/crates/cf-replay/schemas/event/spotlight_dazzled.json` (NEW)
- `game/crates/cf-replay/schemas/event/spotter_target_marked.json` (NEW)
- `game/crates/cf-replay/schemas/event/mine_armed.json` (NEW)
- `game/crates/cf-replay/schemas/event/mine_triggered.json` (NEW)
- `game/crates/cf-replay/schemas/event/mine_disarmed.json` (NEW)
- `game/crates/cf-replay/schemas/event/minesweeper_detected.json` (NEW)
- `game/crates/cf-replay/schemas/event/wire_cut.json` (NEW)
- `game/crates/cf-replay/schemas/event/wire_crushed_by_vehicle.json` (NEW)
- `game/crates/cf-replay/schemas/event/fence_shocked_actor.json` (NEW)
- `game/content/fortifications/mg_nest_static.ron` (NEW)
- `game/content/fortifications/ammo_box_mg.ron` (NEW)
- `game/content/fortifications/mg_tripod_portable.ron` (NEW)
- `game/content/fortifications/spotter_scope.ron` (NEW)
- `game/content/fortifications/bunker_firing_slit.ron` (NEW)
- `game/content/fortifications/sandbag_low.ron` (NEW)
- `game/content/fortifications/sandbag_mid.ron` (NEW)
- `game/content/fortifications/sandbag_high.ron` (NEW)
- `game/content/fortifications/watchtower_t1.ron` (NEW)
- `game/content/fortifications/watchtower_t2.ron` (NEW)
- `game/content/fortifications/watchtower_t3.ron` (NEW)
- `game/content/fortifications/spotlight.ron` (NEW)
- `game/content/fortifications/observation_post.ron` (NEW)
- `game/content/fortifications/radio_repeater.ron` (NEW)
- `game/content/fortifications/barbed_wire.ron` (NEW)
- `game/content/fortifications/razor_wire.ron` (NEW)
- `game/content/fortifications/electrified_fence.ron` (NEW)
- `game/content/fortifications/concertina_roll.ron` (NEW)
- `game/content/fortifications/anti_tank_ditch.ron` (NEW)
- `game/content/fortifications/dragons_teeth.ron` (NEW)
- `game/content/fortifications/tank_trap_x.ron` (NEW)
- `game/content/fortifications/bollard_concrete.ron` (NEW)
- `game/content/fortifications/camo_netting.ron` (NEW)
- `game/content/mine_fields/proximity_belt_dense.minefield.ron` (NEW)
- `game/content/mine_fields/pressure_corridor.minefield.ron` (NEW)
- `game/content/mine_fields/tripwire_perimeter.minefield.ron` (NEW)
- `game/content/mine_fields/ied_chain_killzone.minefield.ron` (NEW)
- `game/content/scenarios/m9c_mg_nest_crewed_defense.ron` (NEW)
- `game/content/scenarios/m9c_sandbag_erosion.ron` (NEW)
- `game/content/scenarios/m9c_watchtower_spotter_chain.ron` (NEW)
- `game/content/scenarios/m9c_minefield_clearance_drill.ron` (NEW)
- `game/content/scenarios/m9c_ied_chain_killzone.ron` (NEW)
- `game/content/scenarios/m9c_wire_breach_assault.ron` (NEW)
- `game/content/scenarios/m9c_electrified_fence_depower.ron` (NEW)
- `game/content/scenarios/m9c_anti_tank_layered_defense.ron` (NEW)
- `game/content/scenarios/m9c_camo_netting_concealment.ron` (NEW)
- `game/content/scenarios/m9c_full_strongpoint.ron` (NEW)
- `game/Cargo.toml` (MODIFY: register cf-fortification)

## Acceptance criteria

```gherkin
Scenario: Player crews an MG nest and gains Full cover
  Given a built `mg_nest_static` (HP 800, loaded ammo_box_mg with 800 rounds)
  And a player adjacent to the nest's crew-entry tile
  When the player issues act.player.crew_fortification { id: nest_id }
  Then mg_nest_crewed event fires with actor_id + nest_id
  And the actor's stance becomes Crewing { nest_id }
  And cover_state observation reports Full
  And the player's primary fire input now controls the MG (90° firing arc + 360° base traverse)
  And the personal weapon is suspended (visible weapon icon switches to MG)

Scenario: Ammo box auto-feeds MG nest until empty
  Given a crewed MG nest with ammo_box_mg (800 rounds) adjacent
  When the player fires 200 sustained rounds
  Then mg_nest_fired_burst events fire (one per burst window)
  And ammo_box rounds_remaining decrements correctly
  When rounds_remaining reaches 0
  Then ammo_box_depleted event fires
  And the MG nest cannot fire until the player swaps in a fresh ammo_box

Scenario: Sandbag wall erodes from `high` → `mid` → `low` under sustained MG fire
  Given a `sandbag_high` wall (HP 600) facing an enemy MG nest at 12 tiles range
  When the enemy MG fires 200 rounds × 6 J at the wall
  Then per-pixel material erodes (M14)
  And the top row of sandbags is destroyed first
  And when HP drops below 400, sandbag_eroded event fires with from=high to=mid
  And the wall's cover_state (Standing) downgrades from Full to Partial
  When HP drops below 200, sandbag_eroded fires again with from=mid to=low
  And when HP reaches 0 the wall is destroyed entirely

Scenario: Tripod-mounted MG deploys + crews + packs up
  Given a player carrying a packed mg_tripod_portable (50 kg in inventory)
  When the player issues act.player.deploy_mg_tripod { pos: P }
  Then mg_tripod_deployed event fires after 4s
  And the tripod becomes a placed fortification at P (HP 400, 90° firing arc)
  When the player crews + fires 60 rounds
  And then issues act.player.uncrew_fortification + act.player.pack_mg_tripod
  Then the tripod returns to inventory after 4s
  And ammo_remaining is preserved through pack/unpack cycle

Scenario: Watchtower destruction triggers lateral collapse
  Given a built `watchtower_t3` (HP 2400, 48 px tall)
  When the player fires HEAT rounds (M14C) at the tower base until HP=0
  Then watchtower_destroyed event fires
  And M14F lateral wall collapse triggers a 5-tile-radius debris drop
  And actors within the radius take fall_impulse damage
  And the radio_repeater module is destroyed (M44D radio-as-component grammar)
  And faction squad-radio range drops by 100 tiles immediately

Scenario: Spotter in watchtower marks target; squad MG receives acquisition bonus
  Given a friendly spotter actor in a `watchtower_t2` with LOS to an enemy
  And a friendly MG nest 16 tiles away with LOS-blocked-by-terrain to the same enemy
  When AI-OBS-A-01 doctrine evaluates the spotter
  Then spotter_target_marked event fires with target_id + target_pos
  And the MG nest's target_acquisition_rate increases by 50%
  And the MG nest's UI shows the marked target with a yellow chevron
  When the spotter is killed or LOS breaks for > 3s
  Then the mark expires and the bonus drops

Scenario: Spotlight reveals concealed actor in cone
  Given a watchtower_t2 with a working spotlight + 1 kW grid power
  And a concealed enemy actor under `camo_netting` 16 tiles away
  When the spotlight cone sweeps over the enemy
  Then the enemy actor's visibility flag flips to "illuminated"
  And M22 visual detection registers the actor regardless of camo netting
  And ui shows a yellow target marker on the illuminated actor
  When a flashbang (M15D) detonates near the spotlight
  Then spotlight_dazzled event fires
  And the spotlight is offline for 12 seconds (visibility returns)

Scenario: Minesweeper detects proximity mine; player disarms it
  Given a player holding `minesweeper` tool
  And a hidden `mine_proximity` 2 tiles away
  When the minesweeper's 2s detection ping runs
  Then minesweeper_detected event fires with mine_id
  And the mine's render flag flips to visible (yellow marker, faction-local)
  When the player crouches adjacent + holds [E] for 6 seconds
  Then mine_disarmed event fires
  And the mine is removed from the world (no detonation)
  And the player gains 1 explosive component (recovered from disarmed mine)

Scenario: IED chain daisy-fires on remote detonation
  Given 5 IEDs placed in a 4-tile-spacing line, all wired to a remote detonator
  When the player presses the detonator
  Then mine_triggered events fire in cascade (IED1 → IED2 → IED3 → IED4 → IED5) within 0.5s
  And each blast applies HE damage per its yield
  And cookoff cascade is deterministic across two engines with same seed

Scenario: Tripwire mine triggers on actor crossing line of sight
  Given a tripwire_mine with tripwire line visible across a corridor
  When an enemy actor walks across the tripwire
  Then mine_triggered event fires with trigger_kind=tripwire
  And the enemy takes 60 J HE damage in 1-tile radius
  And the defending faction receives a `alarm.tripwire_triggered` audio cue

Scenario: Actor crossing barbed wire is slowed + bleeds
  Given a `barbed_wire` segment between player and goal
  When the player walks into the wire (no wire_cutters held)
  Then actor speed drops to 25%
  And actor "snagged" status applies for 0.5s
  And 1 dmg/tick fires while crossing
  And the player can FORCE through (continued movement at 2% speed) OR retreat
  When the player switches to wire_cutters + holds [E] for 3s
  Then wire_cut event fires
  And the wire is removed; player passes freely

Scenario: Electrified fence shocks actor; depowering enables safe cut
  Given an `electrified_fence` powered from a 240V M29 grid node
  When an actor walks into the fence
  Then fence_shocked_actor event fires with actor_id + fence_id
  And the actor takes 80 J shock damage; M16A electrocution status applied
  And the actor is repelled (knockback 4 tiles)
  When the player toggles the M29 breaker to unpowered
  Then the fence's powered=false; subsequent contact applies barbed_wire 1 dmg/tick only
  And wire_cutters can cut the fence in 4s without shock

Scenario: Anti-tank ditch + dragon's teeth + electrified fence layered defense
  Given m9c_anti_tank_layered_defense scenario: 3-row dragon's teeth behind a 8-px AT ditch behind an electrified fence
  And an enemy light tank (M44D, hull HP 8000) approaches
  When the tank enters the fence
  Then fence_shocked_actor fires for the driver (crew_member_killed possible if non-insulated)
  And the tank crushes the fence (wire_crushed_by_vehicle fires)
  When the tank reaches the AT ditch
  Then 30% stuck chance triggers per-tick check; on stuck, escape requires 2-tick traction recovery
  When the tank reaches dragon's teeth
  Then the tank's chassis takes per-component suspension damage on each tooth contact
  And the tank is fully stopped after 2 teeth (M44C suspension HP exhausted)

Scenario: Bomb-disposal robot survives a single mine blast
  Given a deployed `bomb_disposal_robot` (HP 1200, reactive armor)
  And a `mine_pressure` directly ahead
  When the robot drives over the mine
  Then mine_triggered fires
  And the robot takes 120 J HE → reactive armor absorbs 80% → robot survives with HP ~960
  And the robot can continue to the next mine
  When the robot reaches a detected mine in its 1-tile arc
  Then act.player.disarm_mine routes through the robot's mechanical arm (4s disarm)
  And mine_disarmed event fires; robot continues

Scenario: AI doctrine crews nearest empty MG nest
  Given m9c_mg_nest_crewed_defense scenario: 3 empty MG nests + 4 AI defenders
  When the scenario starts
  Then AI-MG-A-02 doctrine has 3 AI move to + crew the 3 nests
  And the 4th AI takes overwatch position
  And AI ammo-feed swap is automatic when ammo_box_depleted fires
  And AI uncrew + retreat when nest HP < 200

Scenario: Camo netting concealment broken by thermal scope (ARV)
  Given a player + squad under `camo_netting` at 16 tiles range from an enemy ARV (M44D thermal scope)
  When the ARV's M22 visual detection runs
  Then the camo concealment is bypassed (thermal sees through)
  And the ARV target_acquisition fires on the squad regardless of netting
  Given a separate enemy infantry actor at 16 tiles (no thermal)
  Then the infantry's visual detection is blocked by the netting (concealment holds)

Scenario: Determinism across full M9C strongpoint scenario
  Given two engines running m9c_full_strongpoint with same world_seed
  And the scenario includes: 2 MG nests + 1 watchtower_t2 + 6 sandbag walls + 1 minefield + barbed wire perimeter + 1 dragon's teeth row
  When 3600 ticks elapse
  Then identical event sequence for all 16 M9C event kinds
  And identical SaveBlob.checksum at tick 3600
  And both engines render identical fortification states at tick 3600
```

## Out of scope

- Bunker walls / sealed concrete rooms / bunker hatches — M28F drop-in bunker pre-fab + M14E structural integrity already own; M9C only ships the `bunker_firing_slit` as the M9C-specific overlap.
- Naval mines (sea-floor) + maritime fortification — M44 amphibious + future.
- Air-defense (anti-air gun + SAM + radar) — future M44E/M9D.
- Razor wire snagging on parachute drop — M44 parachute deployment surface; M9C wire snagging is ground-actor only.
- Faction-specific fortification skin overrides — M9C ships canonical assets; modders can override via `cf-mod` per-asset RON; faction-specific skins are a post-M9C art pass.
- Spawnable enemy from minefield-triggered alarm (the alarm summons defenders) — M9C fires the alarm event; squad spawn is M22 AI doctrine + mission director's responsibility.
- Day/night spotlight gameplay integration (full nighttime stealth meta) — M31 day/night cycle owns; M9C spotlight is forward-compatible (cone-of-light works even in day).
- Per-faction electrified-fence voltage classes (e.g., 480V industrial) — M29 power grid placeholder; M9C ships single 240V default. Mod-exposed voltage_class field future-compat.
- Per-mine fragmentation pattern customization (shaped charge mines, EFP IEDs) — M14C ammo physics already supports HEAT/EFP; M9C ships HE-only mine kinds; advanced kinds via modders.
- Bomb-disposal robot path-planning AI — M9C ships the robot as remote-controlled deployable; AI autonomous disposal future post-M22.

## Dependencies

- M9 Reactor Defense + 5-tier terrain HP (done): per-pixel damage substrate
- M9B Trench Networks (active): MG nests + watchtowers + sandbag walls sit inside / adjacent to trench segments
- M14 collision + impulse routing (active): wire snag, mine blast, dragon's-teeth contact
- M14C ammo physics + HEAT/APFSDS/ERA (active): tank-vs-dragon's-teeth penetration
- M14E per-pixel structural integrity (active): bunker firing slit + watchtower base integrity
- M14F lateral wall collapse (active): watchtower destruction cascade
- M14J cookoff (active): IED chain detonation cascade
- M15 projectiles (active): MG nest firing
- M15D fire/smoke/flashbang (active): spotlight dazzle, camo flammability
- M16A afflictions (active): laceration (razor wire), electrocution (electrified fence), illuminated status
- M19 atmospheric (active): wire flammability + camo flammability
- M22 AI pathfinding (active): wire / mine / ditch avoidance + AI doctrine consumers
- M22B AI doctrine framework (active): AI-MG-A-02, AI-OBS-A-01, AI-ENG-A-03, AI-AT-A-04
- M28A base build mode UX (active): build palette for all M9C fortifications
- M28F blueprints + zones + pre-fabs (active): pre-fab modules embed M9C fortifications (e.g., observation_post inside bunker_t2)
- M29 power kernel (active): electrified fence + spotlight + radio_repeater grid coupling
- M30B mining tool tier ladder (active): wire_cutters, minesweeper, engineering_tool, entrenching_tool
- M30C cave-in physics (active): bunker firing slit pre-placed in M43 procgen ruins
- M31 weather (active): rainfall affects mine-trigger reliability, lightning may discharge electrified fence
- M32 crafting (active): sandbag + ammo box + mine + wire crafting recipes
- M44B convoy + squad-radio (active): radio_repeater extends squad-radio
- M44C per-component vehicle damage (active): dragon's-teeth + tank-trap + AT-ditch consumers
- M44D combat ground vehicles (active): tank-vs-fortification interaction (the central CC parity)
- DR-031 factions (active): per-fortification faction permissions
- DR-008 utility-AI target selection (active): spotter mark + MG target prioritization

## Notes for the implementer

- **Crewing semantics** (the hardest part): a crewed fortification has a 1:1 actor→fortification binding. The actor's stance becomes `Crewing { fortification_id }`. Movement inputs are suspended; firing inputs are rebound to the fortification's mounted weapon. Use M44D crew-slot grammar — same pattern, fortification-side instead of vehicle-side. Uncrew via cfctl OR via actor-death OR via fortification-destruction.
- **Per-actor wire state**: don't store wire crossing state on the wire (one wire, many actors); store it on the actor's `crossing: Option<wire_id>`. Avoid the O(n×m) interaction-matrix trap.
- **Mine detection masking**: hidden mines are not invisible in the render — they're behind a per-faction "detected" flag. The minesweeper flips the flag for the sweeping faction; the mine's enemy faction never sees the marker. Reuse M22 visibility / faction-detection plumbing.
- **IED chain** detonation order is `BFS from trigger origin` over the wire-link graph; cascade fires 100ms apart for the visual chain. Deterministic seed = trigger event.
- **Spotter mark** has a TTL (3s after LOS break) + a per-target unique mark (multiple spotters can stack-mark for + bonus). Don't let marks pile up infinitely; cap at 1 per target.
- **Electrified fence** consumes power; if a fence's power coupling is destroyed (M14 hit on the coupling component), fire `fence_depowered` and the fence acts as barbed_wire from then on. Repair the coupling to re-energize.
- **Anti-tank ditch** stuck-chance is rolled deterministically off `(actor_id, ditch_id, world_seed)`. Multiple attempts to escape are independent rolls (no save-scumming bias).
- **Dragon's teeth** + **tank trap** vehicle damage routes through `M44C per-component damage` — the tank's suspension/track takes the hit, not the hull armor. A tank can plow over teeth if suspension is fresh; a damaged tank may bog down.
- **Camo netting** concealment is a soft check, not a hard block — M22 visual detection rolls `(distance, netting_present, observer_thermal_capable)`. Bypass rules: thermal scope (ARV), spotlight cone, distance < 8 tiles, motion-while-firing.
- **Watchtower destruction** uses M14F lateral wall collapse — falling debris causes M14 fall_impulse damage to anything in the radius. T3 tower at 48 px is the biggest collapse in M9C; use the same animation as M14F.
- **AI-MG-A-02** = "crew nearest empty MG within 8 tiles when threat detected within 24 tiles." Burst-and-duck behavior reuses M9B AI-TRENCH-A-01 pattern (crewed MG = effectively in fire-step position).
- **AI-OBS-A-01** = "if I see a target my squad cannot, broadcast a mark with TTL 3s." Doctrine output is the `spotter_target_marked` event; consumers handle the bonus.
- **AI-ENG-A-03** = "if I have wire/mines + a defensive perimeter is being built, lay wire/mines forward of the fortification line; if a friendly defensive line has a breach, repair (re-mine, restring wire); if an enemy minefield is in my squad's path, disarm with minesweeper."
- **AI-AT-A-04** = "if I'm driving a vehicle approaching an AT ditch / dragon's teeth, evaluate the obstacle: detour > 30 tiles unless suspension HP > 70% (then plow); never plow if dragon's teeth (always detour)."
- All authored content (fortifications / minefield templates / scenarios) lives under `content/fortifications/` + `content/mine_fields/`. Validate via cf-mod; reject unknown enums up-front; emit warning event for missing dependencies (e.g., if M9B not shipped and a fortification references a trench segment).
- The 16 event schemas are the M9C event surface; once locked, do not add new event kinds without bumping schema version (M5 surface compatibility).
