---
type: spec
status: closed-direction
authority: "Full launch content roster: every weapon, actor, vehicle, base object, faction, mission, world, biome, material, ore, music track, and SFX. AI agents author this content via the 3-tier pipeline; modders extend with parity."
ready_when: "Every roster entry has: (1) Tier 2+ generated asset, (2) functional in-game implementation, (3) AI metadata + role-card, (4) replay events, (5) caption coverage, (6) balance fixture, (7) localized strings."
feeds:
  - DR-006
  - DR-014
  - DR-016
  - DR-019
  - DR-021
  - DR-027
  - DR-031
  - DR-039
  - DR-040
  - DR-041
  - DR-042
  - DR-043
  - DR-044
  - DR-045
  - DR-046
  - DR-047
---

← [[spec/index|spec section]] · [[spec/art-and-asset-pipeline|art pipeline]] · [[spec/equipment-loadout|equipment/loadout]] · [[spec/chassis-armor-mechs-and-origins|chassis/armor/mechs/origins]] · [[spec/celestial-bodies-and-worlds-model|worlds catalog]] · [[spec/game-modes-and-match-grammar|game modes]] · [[spec/comms-voice-and-radio-model|comms]] · [[decisions/dr-045-launch-content-roster|DR-045]]

# Launch Content Roster

> [!summary] What this page is
> The complete roster of content shipping at v1.0. Every weapon, actor, vehicle, base object, faction, mission, world, biome, material, ore, music track, and SFX. Inspired by Cortex Command + CCCP's modding ethos. AI-agent-authored via [[spec/art-and-asset-pipeline]]. Modders use the same pipeline.

> [!important] Functional requirement
> Every entry below is a FULLY WORKING feature integrated in the game. NOT a stat-only entry. NOT an asset-only entry. Each entry has: working sim behavior + AI-readable metadata + replay events + captions + balance fixture + localized strings.

## Weapons (140+ items)

### Firearms — Pistols (12)
| ID | Faction | Damage | Magazine | Notes |
|---|---|---|---|---|
| `pistol_basic` | universal | low | 12 | Trade Star standard sidearm. CCCP-inspired. |
| `pistol_revolver_44` | ronin | medium-high | 6 | Slow reload, high damage. |
| `pistol_machine` | coalition | low | 18 | Burst-fire compact. |
| `pistol_silenced` | trade-star | low | 10 | Suppressed. Stealth. |
| `pistol_smart` | imperatus | medium | 12 | Auto-targeting reticle (limited). |
| `pistol_blaster` | trade-star | medium | 24 | Energy. CCCP precedent. |
| `pistol_plasma` | tek-mart | high (overheat) | 8 | Plasma; charges chassis heat. |
| `pistol_dueling` | ronin | very high | 1 | Single-shot duelist. |
| `pistol_dart_tranq` | trade-star | non-lethal | 6 | Tranq darts. |
| `pistol_emp_compact` | coalition | EMP | 6 | Robot-disabling. |
| `pistol_chemical_injector` | husks | bio-toxin | 4 | Spreads affliction. |
| `pistol_finger_gun` | tek-mart | improvised | 1 | Modder-friendly hilarious version. |

### Firearms — SMGs (8)
| ID | Faction | Notes |
|---|---|---|
| `smg_basic` | universal | CCCP-style SMG. |
| `smg_micro` | trade-star | Compact, dual-wield possible. |
| `smg_vector` | coalition | High-rate. |
| `smg_p90` | imperatus | Top-mounted mag. |
| `smg_silenced` | ronin | Suppressed assault SMG. |
| `smg_blaster` | trade-star | Energy. |
| `smg_chemical` | husks | Spread affliction. |
| `smg_chain_dispenser` | tek-mart | Improvised; ramp-up rate. |

### Firearms — Assault Rifles (10)
| ID | Faction | Notes |
|---|---|---|
| `ar_ak47` | universal | CCCP precedent. Reliable. |
| `ar_m4` | coalition | Modular. Tactical. |
| `ar_g36` | coalition | Heat sink. |
| `ar_galil` | browncoats | Compact. |
| `ar_steyr_aug` | imperatus | Bullpup. |
| `ar_blaster_rifle` | trade-star | Energy. CCCP precedent. |
| `ar_battle_rifle_762` | ronin | High-caliber. |
| `ar_carbine` | trade-star | Mid-range. |
| `ar_pulse_rifle` | imperatus | High-tech burst-pulse. |
| `ar_tek_modular` | tek-mart | Modder template — slot in any barrel + receiver + stock. |

### Firearms — Battle Rifles + DMRs (5)
| ID | Faction | Notes |
|---|---|---|
| `dmr_designated_marksman` | coalition | Single-shot precision. |
| `br_fal_762` | ronin | Heavy battle rifle. |
| `dmr_g3` | imperatus | Long-barrel. |
| `dmr_sks` | browncoats | Cheap, classic. |
| `dmr_blaster_marksman` | trade-star | Energy DMR. |

### Firearms — Sniper Rifles (6)
| ID | Faction | Notes |
|---|---|---|
| `sniper_50bmg` | coalition | Anti-material, can damage chassis modules. |
| `sniper_338` | ronin | Long-range precision. |
| `sniper_railgun` | imperatus | Hyper-velocity. Tunneling capable per DR-038. |
| `sniper_laser` | trade-star | Instant-hit; energy. CCCP precedent. |
| `sniper_emp` | coalition | Anti-robot specialist. |
| `sniper_thermal` | tek-mart | Heat-seeking. |

### Firearms — Shotguns (6)
| ID | Faction | Notes |
|---|---|---|
| `shotgun_pump` | universal | CCCP precedent. |
| `shotgun_blunderbuss` | ronin | CCCP precedent. Wide spread. |
| `shotgun_auto` | coalition | Auto-fire. |
| `shotgun_combat` | browncoats | Heavy slugs. |
| `shotgun_breach` | coalition | Door breach. |
| `shotgun_plasma` | tek-mart | Energy. |

### Firearms — LMG / HMG (4)
| ID | Faction | Notes |
|---|---|---|
| `lmg_m249` | coalition | Suppression LMG. |
| `lmg_pkm` | browncoats | Heavy. |
| `hmg_m134` | coalition | Gatling. CCCP precedent. |
| `hmg_chain_blaster` | trade-star | Energy gatling. |

### Heavy + Explosive (15+)
| ID | Faction | Notes |
|---|---|---|
| `rl_rpg7` | universal | CCCP precedent. |
| `rl_javelin` | coalition | Top-attack. |
| `rl_law` | coalition | Light disposable. |
| `gl_grenade_launcher` | universal | CCCP precedent. |
| `gl_milkor` | coalition | Multi-shot. |
| `flak_cannon` | trade-star | CCCP precedent. Anti-air shrapnel. |
| `mortar_60mm` | coalition | Indirect fire. |
| `gauss_rifle` | imperatus | High-velocity. |
| `railgun_anti_tank` | imperatus | Heavy tunneling. |
| `particle_accelerator` | tek-mart | Wide-arc plasma. |
| `bazooka` | universal | CCCP precedent. |
| `recoilless_rifle` | ronin | High-damage anti-mech. |
| `missile_swarm_launcher` | tek-mart | Multi-target. |
| `auto_grenade_launcher` | coalition | High-rate frag. |
| `fuel_air_explosive_launcher` | imperatus | AOE atmospheric ignition per DR-037. |

### Throwables / Explosives (15)
| ID | Faction | Notes |
|---|---|---|
| `frag_grenade` | universal | CCCP precedent. |
| `smoke_grenade` | universal | Concealment. |
| `flash_grenade` | coalition | Stun + visual. |
| `emp_grenade` | coalition | Robot disable. |
| `incendiary_grenade` | universal | Fire pool. |
| `sticky_grenade` | tek-mart | Adhesive. |
| `remote_charge_c4` | coalition | Detonator. |
| `tripwire_mine` | universal | Proximity. |
| `claymore` | coalition | Directional. |
| `satchel_charge` | universal | High-yield demolition. |
| `nano_grenade` | imperatus | Disassembles target. |
| `plasma_orb` | tek-mart | Lingering plasma cloud. |
| `breach_charge` | coalition | Wall demo. |
| `decoy_beacon` | trade-star | AI distraction. |
| `blue_bomb` | universal | CCCP precedent. Volatile, explodes if shot. |

### Melee (8)
| ID | Faction | Notes |
|---|---|---|
| `melee_combat_knife` | universal | CCCP precedent. |
| `melee_machete` | ronin | Mid-range. |
| `melee_riot_baton` | coalition | Non-lethal. |
| `melee_vibroblade` | imperatus | High damage. |
| `melee_plasma_sword` | tek-mart | Energy. Cuts terrain. |
| `melee_monomolecular_katana` | ronin | Single-strike kill. |
| `melee_stun_rod` | coalition | EMP-on-touch. |
| `melee_chainsaw` | tek-mart | Crowd control. |

### Tools (15)
| ID | Faction | Notes |
|---|---|---|
| `tool_digger_light` | universal | CCCP-style light digger. |
| `tool_digger_medium` | universal | CCCP precedent. |
| `tool_digger_heavy` | universal | CCCP precedent. |
| `tool_breach_charge_handheld` | coalition | Wall breach. |
| `tool_repair_basic` | universal | Repair tool. |
| `tool_repair_advanced` | coalition | Mech-grade. |
| `tool_concrete_sprayer` | universal | CCCP precedent. |
| `tool_foam_constructor` | tek-mart | Quick wall builder. |
| `tool_sample_scanner` | trade-star | Geological scanner per DR-041. |
| `tool_metal_detector` | universal | Salvage finder. |
| `tool_drill_light` | universal | Light mining drill per DR-041. |
| `tool_drill_medium` | universal | Medium mining drill. |
| `tool_drill_heavy` | universal | Heavy mining drill. |
| `tool_drill_vacuum_rated` | trade-star | Vacuum-rated for belt asteroids. |
| `tool_oxygen_analyzer` | universal | Atmospheric scanner per DR-037. |

### Mobility (6)
| ID | Faction | Notes |
|---|---|---|
| `mobility_grapple_hook` | universal | CCCP precedent. |
| `mobility_jetpack_assist` | coalition | Augmented jet. |
| `mobility_rope_tether` | universal | Climbing. |
| `mobility_deployable_ladder` | coalition | Vertical access. |
| `mobility_harpoon_line` | ronin | Long-range hook. |
| `mobility_magnetic_boots` | universal | Per DR-038 universal gravity overrides. |

### Shields (5)
| ID | Faction | Notes |
|---|---|---|
| `shield_riot` | coalition | Bullet-blocker. |
| `shield_combat` | browncoats | Tactical. |
| `shield_deployable_barrier` | coalition | Set-and-forget. |
| `shield_energy` | imperatus | Energy field. |
| `shield_kinetic_dampener` | tek-mart | Reduces impact. |

### Sensors (6)
| ID | Faction | Notes |
|---|---|---|
| `sensor_motion` | universal | CCCP precedent. |
| `sensor_noise_detector` | coalition | Acoustic. |
| `sensor_material_probe` | universal | Per [[spec/atmospherics-and-chemistry-model]]. |
| `sensor_em_scanner` | imperatus | EM threat detection. |
| `sensor_heat_vision` | trade-star | Thermal overlay. |
| `sensor_command_sensor` | universal | Tactical map updater. |

### Medical (8)
| ID | Faction | Notes |
|---|---|---|
| `medical_medikit_basic` | universal | CCCP precedent. |
| `medical_medikit_advanced` | coalition | Advanced. |
| `medical_dart` | trade-star | Ranged heal. |
| `medical_stim` | coalition | Movement boost. |
| `medical_revive_kit` | universal | Per DR-018 rescue. |
| `medical_defibrillator` | universal | Resuscitation. |
| `medical_suture_stapler` | coalition | Quick patch. |
| `medical_pain_blocker` | trade-star | Affliction filter. |

### Repair / Support (6)
| ID | Faction | Notes |
|---|---|---|
| `repair_tool_basic` | universal | Robot repair. |
| `repair_tool_advanced` | coalition | Mech-grade. |
| `repair_spare_module` | universal | Module swap kit. |
| `repair_oil_canister` | universal | Robot fluid refill per DR-040. |
| `repair_emp_proof_patch` | coalition | EMP shielding. |
| `repair_weld_torch` | universal | Structural repair. |

### Comms (10) — per [[spec/comms-voice-and-radio-model]]
| ID | Origin | Notes |
|---|---|---|
| `comms_helmet_voice` | human-only | Human voice pickup. |
| `comms_throat_mic` | human-only | Stealth. |
| `comms_bone_conductor` | human-only | Underwater/sealed. |
| `comms_handheld_vhf` | universal | Squad-level. |
| `comms_backpack_vhf` | universal | Long range. |
| `comms_hf_transceiver` | universal | Skywave HF. |
| `comms_satellite_uplink` | universal | Cross-planet. |
| `comms_jammer` | coalition | Active jam. |
| `comms_encryption_module` | universal | Crypto. |
| `comms_antenna_yagi` | universal | Directional. |

## Actors (44+ items)

### Human chassis (8 + power armor variants)

| ID | Tier | Notes |
|---|---|---|
| `human_light_infantry` | base | CCCP precedent. Quick. Bleeds. |
| `human_heavy_infantry` | base | Bullet sponge. |
| `human_scout` | base | Ronin-style. Fast. |
| `human_sniper` | base | Long-range. |
| `human_assault` | base | Frontline. |
| `human_medic` | base | Medical role. |
| `human_engineer` | base | Build/repair. |
| `human_demolitions` | base | Explosives spec. |

| ID | Tier | Notes |
|---|---|---|
| `pa_light` | tier-1 | Light power armor. Coalition default. |
| `pa_medium` | tier-2 | Medium power armor. Browncoat default. |
| `pa_heavy` | tier-3 | Heavy "Spartan" armor. Carries HMG comfortably. |
| `pa_jump` | special | Jump-rocket variant; high mobility. |

### Android chassis (4)

| ID | Notes |
|---|---|
| `android_civilian` | Basic android; modular slots. |
| `android_military` | Combat-tuned android. |
| `android_engineer` | Repair-tool integrated. |
| `android_infiltrator` | Stealth+silenced; per DR-022 humanlike threshold. |

### Robot chassis (5)

| ID | Notes |
|---|---|
| `robot_combat` | CCCP-style combat robot. |
| `robot_scout_drone` | Light drone, recon. |
| `robot_security_drone` | Static patrol. |
| `robot_repair_drone` | CCCP medic drone analog. |
| `robot_anti_air_drone` | AA missile carrier. |

### Mech chassis (5)

| ID | Notes |
|---|---|
| `mech_light_bipedal` | Per DR-021 light mech. |
| `mech_medium_quadruped` | CCCP Dreadnought-style 4-leg. |
| `mech_heavy_walker` | Heavy bipedal. |
| `mech_siege_artillery` | Slow, heavy guns. |
| `mech_aerial_gunship` | Hover platform; weapon-stable. |

### Civilian / NPC (6)

| ID | Notes |
|---|---|
| `npc_hostage` | Rescue mission target. |
| `npc_scientist` | Research mission. |
| `npc_engineer` | Civilian engineer. |
| `npc_salvager` | Salvage mission. |
| `npc_broker` | Trade hub NPC. |
| `npc_prisoner` | POW. |

### Anomaly / Husk (6)

| ID | Notes |
|---|---|
| `anomaly_husk_thin` | Fast attacker. |
| `anomaly_husk_medium` | CCCP-zombie-medium-style. |
| `anomaly_husk_fat` | Heavy crusher. |
| `anomaly_skeleton` | CCCP precedent. |
| `anomaly_mutant_charger` | Tank. |
| `anomaly_alien_husk` | Hybrid biological-mechanical. |

### Turret / Static (6)

| ID | Notes |
|---|---|
| `turret_mg_small` | CCCP precedent. |
| `turret_autocannon` | Anti-vehicle. |
| `turret_missile` | Anti-air. |
| `turret_laser` | Energy. |
| `turret_sentry_drone_pad` | Spawns scout drones. |
| `turret_aa_emplacement` | Heavy AA. |

## Vehicles / Dropcraft (18)

### Dropcraft (12)

| ID | Notes |
|---|---|
| `craft_light_dropship` | CCCP precedent. 1-2 slot. |
| `craft_heavy_dropship` | CCCP precedent. 4-6 slot. |
| `craft_attack_dropship` | Armed. |
| `craft_rocket_capsule` | CCCP precedent. Single use. |
| `craft_drop_pod_single` | 1-actor. |
| `craft_supply_pod` | Equipment delivery. |
| `craft_troop_transport` | 8-actor. |
| `craft_gunship` | Heavy weapons platform. |
| `craft_salvage_rig` | Salvage missions. |
| `craft_mining_rig` | Mining missions per DR-041. |
| `craft_scout_drone_carrier` | Scout drone bay. |
| `craft_evac_shuttle` | Extraction vehicle. |

### Ground vehicles (6)

| ID | Notes |
|---|---|
| `vehicle_apc` | Armored personnel carrier. |
| `vehicle_scout_buggy` | Fast recon. |
| `vehicle_mining_hauler` | Mining-pack mover. |
| `vehicle_cargo_flatbed` | Salvage hauler. |
| `vehicle_mobile_command` | Forward command core. |
| `vehicle_recon_walker` | Light walker; bridge to mechs. |

## Base Objects (60+)

### Command + Power (12)
| ID | Notes |
|---|---|
| `base_command_core_rooted` | Per DR-027. |
| `base_command_core_uprooted_avatar` | Per DR-015. |
| `base_brain_case_mount` | CCCP precedent. |
| `base_power_core_main` | Power generation. |
| `base_power_core_aux` | Backup. |
| `base_power_node` | Distribution. |
| `base_power_cable` | Pipe variants. |
| `base_solar_panel` | Renewable. |
| `base_battery_bank` | Storage. |
| `base_capacitor_bank` | Burst power. |
| `base_emergency_generator` | Backup. |
| `base_command_console` | UI access. |

### Defense (10)
| ID | Notes |
|---|---|
| `base_shield_generator_small` | Per DR-027. |
| `base_shield_generator_large` | |
| `base_turret_mg` | |
| `base_turret_autocannon` | |
| `base_turret_missile` | |
| `base_turret_laser` | |
| `base_automated_turret_control` | |
| `base_alarm_system` | |
| `base_blast_door` | Heavy seal. |
| `base_force_field_door` | Energy seal. |

### Atmospherics (10) — per DR-037
| ID | Notes |
|---|---|
| `base_oxygen_generator` | Per [[spec/atmospherics-and-chemistry-model]]. |
| `base_atmosphere_pump` | |
| `base_atmosphere_vent` | |
| `base_atmosphere_filter` | |
| `base_pipe_segment_gas` | |
| `base_pipe_segment_liquid` | |
| `base_pipe_segment_coolant` | |
| `base_pressure_regulator` | |
| `base_pressure_valve` | |
| `base_condenser_chamber` | |

### Doors (5)
| ID | Notes |
|---|---|
| `base_sealed_door_small` | |
| `base_sealed_door_medium` | |
| `base_sealed_door_large` | |
| `base_airlock_assembly` | Per DR-037 cycle state machine. |
| `base_decontamination_chamber` | Affliction strip. |

### Storage / Logistics (8)
| ID | Notes |
|---|---|
| `base_cargo_storage` | |
| `base_ammo_storage` | |
| `base_weapon_rack` | |
| `base_medical_bay` | |
| `base_repair_pad` | Per DR-027. |
| `base_drone_bay` | |
| `base_dropship_pad` | |
| `base_hangar_door` | Wide opening. |

### Mining / Industry (6) — per DR-041
| ID | Notes |
|---|---|
| `base_refinery` | Per [[spec/mining-and-extraction-model]]. |
| `base_smelter` | |
| `base_foundry` | |
| `base_ore_cargo_bay` | |
| `base_fuel_canister_rack` | |
| `base_autoclave` | Sterilization. |

### Sensors / Comms (5) — per DR-043
| ID | Notes |
|---|---|
| `base_sensor_station` | |
| `base_scanner_array` | |
| `base_comm_relay` | Per [[spec/comms-voice-and-radio-model]]. |
| `base_satellite_uplink_station` | |
| `base_jammer_station` | |

### Gravity / Special (4) — per DR-038
| ID | Notes |
|---|---|
| `base_gravity_generator` | |
| `base_gravity_well_projector` | |
| `base_magnetic_plating` | |
| `base_zero_g_lab` | |

## Factions (8 launch factions)

Per [[spec/setting-and-world-frame]] + DR-016:

| Faction | Tone | Signature gear | Doctrine flavor | Visual register |
|---|---|---|---|---|
| **Trade Star** | Corporate mercenary; cold pragmatism | `pistol_blaster`, `ar_blaster_rifle`, `flak_cannon`, light dropships | Buy-rented loyalty; salvage-focused; clean energy weapons | Chrome + glass; corporate logos; muted tan-blue |
| **Coalition** | Military; tactical professionalism | `ar_m4`, `dmr_designated_marksman`, `lmg_m249`, jetpack power armor | Doctrine: pincer + suppress; medic + engineer support | Forest-green / khaki; numbered patches; clean lines |
| **Browncoats** | Heavy clone troopers; super soldier discipline | `ar_galil`, `shotgun_combat`, heavy power armor | Frontline grinder; CCCP precedent | Brown + red; weathered armor; uniform faceplates |
| **Ronin** | Frontier specialists / mercenary loners | `melee_monomolecular_katana`, `pistol_revolver_44`, `dmr_338`, infiltrator gear | Stealth + duelist + signature kill; no formation | Black + crimson highlights; individualized; named operatives only |
| **Tek-Mart** | Tech-bro frontier rats | `pistol_plasma`, `tool_foam_constructor`, modular weapons, jury-rigged mechs | Improvised + modular + chaotic; modder analog faction | Mismatched colors; visible wires; "rebuilt by hobby" aesthetic |
| **Imperatus** | Oppressive empire | `gauss_rifle`, `ar_pulse_rifle`, `gauss_rifle`, EM weapons, autonomous drones | Order + hierarchy + autocrat; legion formations | Imperial gold + black; rigid geometry; uniform |
| **Free Hold** | Frontier independents / scrappers | `pistol_basic`, `ar_ak47`, `shotgun_pump`, salvage-built gear | Asymmetric defense; bunker-first; CCCP Ronin/Coalition mix | Earth tones; patched cloth; community-painted insignia |
| **The Husks** | Post-anomaly biological-mechanical hybrids | `pistol_chemical_injector`, `melee_chainsaw`, swarming mutants | Antagonist faction; swarms + biotoxin; corruption | Sickly green + flesh-pink; oozing armor; mismatched limbs |

## Missions (30+ launch)

### Onboarding (3) — per DR-023
- `mission_first_contract` — main onboarding mission. 12-15 min.
- `mission_lab_intro` — lab launcher tutorial.
- `mission_workshop_intro` — modder onboarding.

### Modular labs (8) — per DR-023
- `lab_movement_aim`, `lab_terrain_materials`, `lab_loadout_delivery`, `lab_squad_orders_ai`, `lab_command_core_base`, `lab_avatar_mode`, `lab_chassis_damage`, `lab_replay_debrief`

### Anchor campaign (6)
- `mission_breach_contract` — DR-004 first playable. Earth.
- `mission_mars_dust_recovery` — Mars salvage with dust storm.
- `mission_vulcan_extraction` — Vulcan thermal hazard rescue.
- `mission_belt_asteroid_robot_mining` — robot-only mining.
- `mission_europa_ice_crisis` — cryo + ice geyser.
- `mission_husk_outbreak_finale` — campaign endgame.

### Procedural contract templates (8)
- `template_breach_assault`, `template_extraction_rescue`, `template_defend_position`, `template_recon_signal`, `template_assassinate_target`, `template_mining_run`, `template_atmosphere_repair`, `template_anomaly_containment`

### Bunker Defence flagship maps (4)
- `bd_earth_industrial_complex`, `bd_mars_research_outpost`, `bd_vulcan_geothermal_station`, `bd_orbital_station`

### PvP arena maps (3)
- `arena_wreckage_field`, `arena_shattered_dome`, `arena_caves_of_mimas`

### Coop-vs-AI (2)
- `coop_swarm_defense`, `coop_husk_mine_clear`

### Modder-template scenarios (6)
- `modder_template_chassis_test`, `modder_template_weapon_lab`, `modder_template_atmosphere_test`, `modder_template_ai_doctrine_test`, `modder_template_pvp_arena`, `modder_template_campaign_chapter`

## Worlds (12) — per [[spec/celestial-bodies-and-worlds-model]]

| ID | Type | Notes |
|---|---|---|
| `world_sol` | star | Reference; surface-incompatible. |
| `world_earth` | terrestrial | Standard atmosphere + 1g. |
| `world_earth_moon` | moon | Vacuum + low-g. |
| `world_mars` | terrestrial | Thin atmosphere + low-g + dust storms. |
| `world_phobos` | small moon | Vacuum + microgravity. |
| `world_deimos` | small moon | Vacuum + microgravity. |
| `world_mimas` | moon | Vacuum + ice + microgravity. |
| `world_europa` | moon | Sub-ice ocean + cryo storms. |
| `world_vulcan` | terrestrial | Volcanic + flammable atmosphere + thermal storms. |
| `world_venus` | terrestrial | Crushing pressure + high heat. |
| `world_belt_asteroid_a1` | asteroid | Vacuum + microgravity + ore-rich. |
| `world_orbital_station_a1` | station | Sealed station + variable gravity. |

Each world has 3-5 biome variants (urban-ruin, desert, ice-cave, vacuum-surface, volcanic, jungle, sealed-corridor, alien-coral) ≈ 50 biome variants total.

## Materials (17 launch + 10+ expansion)

Per DR-036 + [[comparables/noita-grade-material-simulation-research]]: Air, dirt/sand, rock/concrete, metal, wood/organic, water, steam/mist, smoke, fire/heat, oil/fuel, acid, toxic sludge, toxic gas, lava, blood/vomit, electricity, pebble/debris.

Expansion (lab-gated): slime, brine, coolant, cryo, fuel vapor, foam, nanogel, alchemic precursor, biological variants.

## Ores (12) — per DR-041

iron, copper, silica, ice_volatiles, ice_oxite, ice_water, nickel, cobalt, gold, uranium, perchlorate, platinum_group.

## Reactions (20+) — per [[spec/atmospherics-and-chemistry-model]]

water+fire→steam, oil+spark→fire-on-oil, lava+water→rock+steam, acid+metal→corrosion, electricity+water→shock, volatiles+O2→combustion, H2+O2→combustion, etc.

## Music Tracks (30+) — per [[spec/music-and-soundtrack]]

- Main theme (1)
- World themes (12 × 1) — Earth ambient, Mars dust, Vulcan thermal, Europa cryo, Mimas vacuum, etc.
- Combat layers (6) — low / mid / high intensity per faction
- Base-tension layers (4) — exploration / pre-combat / siege / siege-boss
- Menu / UI tracks (4)
- Mission-specific stings (8)
- Hero antagonist motifs (3) — Imperatus / Husks / Browncoat heavies

## SFX Library (400+ clips)

Per `cf-audio` registry. Caption-bound per DR-020. Organized by category: weapons, footsteps, equipment, voice, environment, UI, music-stings.

## Authoring Pipeline

Every roster entry follows [[spec/art-and-asset-pipeline]] 3-tier pipeline:
1. AI agent generates Tier 1 SVG placeholder (M0..M2).
2. AI agent generates Tier 2 ComfyUI sprite/animation/VFX/audio (M2..M5).
3. AI agent finalizes Tier 3 cleanup + variants (M5+); project-owner review is approval only.

Schema: `content/<category>/<id>.ron` per [[decisions/dr-006-modding-data-model]] schema-first.

## Validation Gates (CI)

| Gate | Checks |
|---|---|
| `cf-mod validate content/ --strict` | Every entry has all required fields. |
| `cf-asset-pipeline check-coverage` | Every entry has Tier 2+ assets. |
| `cf-asset-pipeline check-localized` | Every player-visible string is keyed in en.ftl. |
| `cf-balance-fixture-check` | Every entry has BALANCE-A row in fixtures. |
| `cf-replay-event-check` | Every entry emits expected events. |
| `cf-caption-check` | Every audible cue has caption. |
| `cf-ai-metadata-check` | Every weapon/chassis/equipment has AI role-card metadata. |

## Source Trail

- [[decisions/dr-045-launch-content-roster]] — DR scoping.
- [[spec/art-and-asset-pipeline]] — pipeline that produces these.
- [[spec/equipment-loadout]] + [[references/equipment-corpus-cccp]] — CCCP equipment ground truth.
- Cortex Command Wiki: https://datarealmscortexcommand.fandom.com/wiki/List_of_Weapons + List_of_Actors
- CCCP repo: `Cortex-Command-Community-Project/Data/`
