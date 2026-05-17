# M6C — Equipment Catalog Buildout (68 Launch SKUs)

## Status

`done`

## Intent

Author the **68 missing equipment SKUs** across firearms (12) / melee (8) / throwables (10) / heavy weapons (8) / medical (12) / survival (8) / sensors (5) / personal protective equipment (15). M6 baseline ships 6 weapons + 4 grenades + basic tools; M6C closes the gap to no-compromise Cortex-Command-plus-Tarkov SKU depth.

## Canonical ownership

Owns the equipment SKU registry under the M6B `ItemSpec` schema. Each SKU is a `content/equipment/<category>/<id>.ron` file with full ItemSpec + per-category metadata.

## Player-facing behavior

### Firearms (12 new beyond M6 baseline)

- `revolver_357` — slow + heavy hit; manual reload
- `submachine_gun_9mm` — rapid fire; close range; large magazine
- `assault_rifle_t2` — standard mid-tier
- `sniper_rifle_t2` — long range + high damage
- `shotgun_pump` — spread + close range devastating
- `heavy_machine_gun_50cal` — crew-served; vehicle-mountable
- `battle_rifle_762` — full-auto + heavier round
- `carbine_compact` — short barrel + carbine
- `dmr_762` — designated marksman; semi-auto precision
- `lmg_belt_fed` — sustained fire
- `anti_materiel_rifle_127` — anti-vehicle precision (HEAT/APFSDS compatible per M14C)
- `squad_automatic_saw` — bipod + sustained suppress

### Melee (8 new)

- `dagger_combat` — fast + light
- `katana` — long blade + high damage
- `sledgehammer` — heavy 2-hand; blunt + structural breach
- `spear` — reach + thrust
- `bayonet` — rifle attachment
- `axe_hatchet` — split wood + melee
- `stun_baton` — non-lethal + electric jolt
- `pickaxe_combat_variant` — melee + mining (T1 hybrid)

### Throwables (10 new)

- `he_grenade` — high explosive (M14C consumer; large radius)
- `acid_grenade` — chemical splash (M15 acid material spawn)
- `pipe_bomb` — improvised; craftable
- `molotov_cocktail` — fire spread (M15 fire material)
- `proximity_mine` — auto-trigger on hostile approach
- `pressure_mine` — trigger on actor weight
- `tripwire_mine` — trigger on cross
- `c4_charge` — manual detonation + remote
- `incendiary_grenade` — fire + smoke
- `bouncing_betty` — anti-personnel + air-burst

### Heavy weapons (8 new)

- `rpg_launcher_heat` — M14C HEAT round
- `tank_autocannon_m14c` — M14C APFSDS round
- `mortar_60mm` — indirect fire + crew-served
- `recoilless_rifle` — anti-armor + back-blast
- `atgm_javelin` — fire-and-forget + top-attack HEAT
- `flamethrower` — sustained fire spray (M15 fire)
- `plasma_cannon_m48` — exotic; long range
- `gauss_rifle_anti_materiel` — electromagnetic + high damage

### Medical (12 new beyond M6 medkit)

Per M14H consumer: field_bandage, trauma_pack, tourniquet, sutures, splint, surgery_kit, defibrillator, cpr_compressions, transfusion_bag, iv_fluids, oxygen_therapy, medical_scanner_t1.

### Survival (8 new)

- `water_bottle_1l`
- `food_ration_mre`
- `sleeping_bag`
- `tent_2_person`
- `cooking_pot`
- `lighter_zippo`
- `compass_magnetic`
- `binoculars_8x`

### Sensors (5 new beyond M29B's existing list)

- `radio_direction_finder`
- `sound_detector_passive`
- `heat_camera_handheld`
- `radar_compact_t2`
- `geological_surveyor_m30d`

### PPE (15 new)

- `helmet_light_kevlar`
- `helmet_medium_steel`
- `helmet_heavy_titanium`
- `armor_kevlar_light` (-20% kinetic / 4 kg)
- `armor_ceramic_medium` (-40% kinetic + -20% thermal / 7 kg)
- `armor_steel_heavy` (-60% kinetic + -10% mobility / 12 kg)
- `armor_heavy_plate` (-80% kinetic + -30% mobility / 18 kg)
- `combat_gloves_light` (+5% grip / 0.2 kg)
- `combat_gloves_heavy` (-10% cold afflictions / 0.4 kg)
- `tactical_boots` (+5% walk / 1.2 kg)
- `knee_pads` (-20% prone scrape / 0.4 kg)
- `elbow_pads` (-20% lean scrape / 0.3 kg)
- `hardsuit_full` (sealed; -30% mobility / 18 kg)
- `eva_suit` (vacuum-rated; -20% mobility / 14 kg)
- `radiation_suit` (-90% radiation tick / 8 kg)
- `hazmat_suit` (-90% chemical contact / 6 kg)
- `insulated_suit` (-40% cold / -20% heat / 5 kg)
- `armor_modular_plate_carrier` (accepts 4 plate inserts; 5 kg base + plate)

## Crates / modules touched

| Crate | Status | What |
|---|---|---|
| `cf-equipment::weapon::{revolver, carbine, battle_rifle, dmr, lmg, hmg, anti_materiel, saw}` | NEW | 12 firearm SKUs |
| `cf-equipment::melee::{dagger, katana, sledge, spear, bayonet, axe}` | NEW | 8 melee SKUs |
| `cf-equipment::grenade::{he, acid, pipe_bomb, molotov, prox_mine, pressure_mine, tripwire}` | NEW | 10 throwables |
| `cf-equipment::heavy::{mortar, recoilless, atgm, flamethrower, plasma_cannon}` | NEW | 8 heavy weapons |
| `cf-equipment::medical::*` | NEW | 12 medical SKUs (M14H consumers) |
| `cf-equipment::survival::*` | NEW | 8 survival SKUs |
| `cf-equipment::sensor::*` | NEW | 5 sensor SKUs |
| `cf-equipment::ppe::*` | NEW | 15 PPE SKUs |
| `cf-actor::body_armor_slot` | NEW | Per-actor helmet + body armor + gloves + boots + knee/elbow slots (separate from M13 chassis armor) |
| `cf-mod` | MODIFY | Validate per-category folder |
| `cf-replay` | MODIFY | 5 new event schemas (per-category usage) |

## Files

- `game/content/equipment/firearms/*.ron` (12 NEW)
- `game/content/equipment/melee/*.ron` (8 NEW)
- `game/content/equipment/grenades/*.ron` (10 NEW)
- `game/content/equipment/heavy/*.ron` (8 NEW)
- `game/content/equipment/medical/*.ron` (12 NEW)
- `game/content/equipment/survival/*.ron` (8 NEW)
- `game/content/equipment/sensors/*.ron` (5 NEW)
- `game/content/equipment/ppe/*.ron` (15 NEW)
- ~68 RON files total

## Acceptance criteria

```gherkin
Scenario: M6C-1 All 78 SKUs registered with M6B schema
  Given content/equipment/items/manifest.ron
  Then 78 new SKUs declared with mass + grid + bulk
  And cf-mod validate passes

Scenario: M6C-2 Body armor slot separate from chassis armor
  Given infantry actor wearing armor_kevlar_light
  When hit by rifle round (kinetic 50)
  Then damage reduced by 20%
  And body_armor.degraded fires on durability tick

Scenario: M6C-3 ATGM Javelin top-attack lock
  Given player aims atgm_javelin at tank chassis
  When lock_acquired fires after 3s
  And player releases:
    Then projectile arcs to top of target
    And HEAT-tandem penetrates top armor

Scenario: M6C-4 Flamethrower fuel canister coupling
  Given flamethrower in primary + fuel_canister in tank_utility
  When player fires:
    Then fuel drains from canister
    And fire material spawns per M15

Scenario: M6C-5 Defibrillator revives downed actor (M14H consumer)
  Given actor in Downed state
  When ally uses defibrillator within 30s window:
    Then actor.revived fires
    And HP restored to 25%

Scenario: M6C-6 EVA suit + helmet seal vacuum
  Given player wearing eva_suit + sealed helmet in vacuum
  Then O2 supply from tank slot maintains breathing
  And no decompression damage

Scenario: M6C-7 Proximity mine triggers
  Given player places proximity_mine
  When hostile enters 4-tile radius:
    Then mine.detonated fires

Scenario: M6C-8 Mortar crew-served
  Given mortar_60mm requires 2 actors (gunner + loader)
  When solo actor attempts to fire:
    Then "Crew required" warning
  When second actor assists:
    Then mortar.crewed fires
```

## Out of scope

- Per-SKU sprite art (M9A asset pipeline).
- Per-SKU sound effects (M12A audio pipeline).
- Per-SKU mod content (M33 modding workbench).

## Dependencies

- M6 (done) — base weapon class system
- M6B (active) — item schema canonicalization
- M14C (active) — HEAT/APFSDS rounds for ATGM
- M14H (proposed) — medical SKU consumers
- M15 (active) — acid + fire materials for grenades / molotov
- M16 (active) — afflictions for medical SKU mapping
- M32 (active) — T0-T4 craftability for each SKU

## Notes for the implementer

- All SKUs use M6B ItemSpec schema as the single source of truth.
- Mass values calibrated for M14A mass aggregation realism.
- Per-category folder structure enables modders to add SKUs in any category.
