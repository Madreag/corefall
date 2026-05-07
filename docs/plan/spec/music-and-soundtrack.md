---
type: spec
status: closed-direction
authority: "Music + soundtrack: AI-composed adaptive layers (Suno/Udio cloud + MusicGen/Stable Audio Open local) + diegetic-first mix + adaptive crossfade per EnvironmentSignal/match phase. 30+ launch tracks. FMOD or bevy_kira_audio."
ready_when: "All launch tracks composed + mastered; adaptive layering responds to gameplay events; SFX library covers 400+ events with caption coverage; FMOD/Kira mix passes diegetic-first per DR-020."
feeds:
  - DR-012
  - DR-014
  - DR-019
  - DR-020
  - DR-024
  - DR-031
  - DR-043
  - DR-044
  - DR-046
  - DR-047
---

← [[spec/index|spec section]] · [[spec/audio-identity|audio identity]] · [[spec/art-and-asset-pipeline|art pipeline]] · [[spec/comms-voice-and-radio-model|comms]] · [[decisions/dr-020-audio-identity|DR-020]] · [[decisions/dr-044-audiovisual-production-pipeline|DR-044]]

# Music & Soundtrack

## Approach

**Diegetic-first per DR-020 + adaptive synth-dread layered per match phase + AI-composed via cloud (Suno/Udio) + AI-generated SFX via Stable Audio Open local.**

## Stack

| Component | Detail |
|---|---|
| **Music composer (cloud)** | Suno v5 (currently leads quality per 2026 reviews) OR Udio v2. Both allow commercial use; license review pre-launch. Cost ~$10-25/month. AI-prompted by project-owner; iterations via AI agent. |
| **Music composer (local fallback)** | MusicGen-Medium (Meta, MIT-licensed) OR Stable Audio Open 1.0 (Apache-2.0). Local 32GB VRAM runs both. Lower quality than Suno but no licensing risk. |
| **SFX generator** | Stable Audio Open 1.0 (Apache-2.0); generates up to 47s audio at 44.1 kHz from text prompts. Local. |
| **Voice generator (NPC dialogue)** | ElevenLabs (license review) OR XTTS-v2 / Tortoise (open-source). Skip if license blocks. |
| **Mixer** | FMOD Studio (free under $200K/yr) wrapped via `bevy_fmod` OR pure-Rust `bevy_kira_audio` (Apache-2.0; lower-feature). Preferred: FMOD for adaptive layering + spatial; fallback Kira. |
| **Adaptive system** | `cf-audio-adaptive` crate. Reads `EnvironmentSignal`, match phase, mission director state, combat intensity. Emits crossfade commands to FMOD/Kira. |
| **Spatial audio** | Steam Audio per DR-043 (Apache-2.0). Already integrated for voice/radio; reused for music spatial cues (radio music, base PA system). |

## Track Roster (30+ launch)

### Main theme (1)
- `theme_main` — 90-120s. Heroic + tactical + pulp; modulates between minor + major. Used: title screen, menu, key cinematics.

### World themes (12 — 1 per world)
- `world_earth_ambient` — urban industrial decay. Lo-fi synthwave + sub-bass drone.
- `world_earth_moon_ambient` — vacuum loneliness. Sparse pad + radio static layer.
- `world_mars_ambient` — dust + alien horizon. Eastern-tinged synth.
- `world_phobos_ambient` — vacuum + microgravity + Mars-shine. Industrial echo.
- `world_deimos_ambient` — same family as Phobos.
- `world_mimas_ambient` — orbital station tension. High pad + static.
- `world_europa_ambient` — sub-ice mystery. Bell-like glassy texture.
- `world_vulcan_ambient` — geothermal threat. Distorted bass + thermal pulse.
- `world_venus_ambient` — pressure + heat. Long sustained pad.
- `world_belt_asteroid_ambient` — vacuum mining. Mechanical clank percussion.
- `world_orbital_station_ambient` — corporate sci-fi. Refined synth jazz.
- `world_sol_ambient` — surface-incompatible; cinematic only.

### Combat layers (6)
- `combat_low_intensity` — sparse percussion + pulse. Used: scouting, pre-engagement.
- `combat_mid_intensity` — driving rhythm + bass + melodic motif.
- `combat_high_intensity` — full ensemble + percussion + brass-equivalent + climactic motif.
- `combat_climactic` — peak; used for boss fights, last stands.
- `combat_chase` — fast tempo + stinger; used during pursuit.
- `combat_stalemate` — slow + tense + harmonic dissonance; used during siege/standoff.

### Base / tension layers (4)
- `base_exploration` — calm + curious. Used: between missions in base.
- `base_tension` — pre-incident; ominous undercurrent.
- `base_under_siege` — urgent + driving.
- `base_post_victory` — relief + reflection.

### Menu / UI tracks (4)
- `menu_main` — title screen primary.
- `menu_loadout` — workbench ambient.
- `menu_briefing` — mission briefing build-up.
- `menu_debrief` — mission debrief reflection.

### Mission-specific stings (8)
- `sting_objective_complete` — short fanfare.
- `sting_objective_failed` — descending dirge.
- `sting_breach_imminent` — warning.
- `sting_reinforcements_arriving` — tactical update.
- `sting_command_core_uprooted` — major event.
- `sting_command_core_lost` — critical loss.
- `sting_named_npc_killed` — narrative beat.
- `sting_artifact_recovered` — discovery.

### Hero antagonist motifs (3)
- `motif_imperatus_legion` — empire's leitmotif. Heavy + brass.
- `motif_husks_corruption` — anomaly leitmotif. Distorted + organic.
- `motif_browncoat_assault` — clone troops leitmotif. Marching + military.

## Adaptive Layering

Per-mission, per-phase, music layers crossfade based on:

| Trigger | Effect |
|---|---|
| `match.started` | World ambient + base exploration overlay |
| `enemy_first_contact` | Combat low-intensity layer crossfade in |
| `intensity_score > 0.5` | Combat mid-intensity |
| `intensity_score > 0.8` | Combat high-intensity |
| `combat_lull` (no contact > 30s) | Crossfade back to ambient |
| `objective_completed` | Sting + return to ambient |
| `objective_failed` | Sting + dirge variant |
| `command_core_status_change` | Sting + tension layer |
| `weather.event_started: solar_flare` | Layer in radio-static texture per DR-043 |
| `actor_low_health` | Heartbeat sub-bass layer for that player |
| `boss_phase_change` | Climactic motif transition |
| `match_victory` | Victory orchestral swell |
| `match_defeat` | Dirge + scroll |

## SFX Library (400+ clips)

### Categories

| Category | Count | Notes |
|---|---|---|
| Weapon fire | 80+ | Per weapon × per state (fire / dry-click / chamber / reload-start / reload-end) |
| Footsteps | 40+ | Per surface material × per chassis type |
| Equipment | 50+ | Buttons, switches, deploy/retract, charge, click |
| Voice (humanoid) | 60+ | Per faction × intent (alert, hurt, dying, signal, taunt) |
| Voice (mech/robot) | 40+ | Mechanical equivalents |
| Environment | 60+ | Wind, water, fire, atmosphere, weather |
| UI | 30+ | Menu navigation, notification, achievement, error |
| Music stings | 8+ | Per mission state |
| Combat | 40+ | Hit confirms, ricochet, explosion, EMP, plasma |

### AI-Generation Pipeline

Per [[spec/art-and-asset-pipeline]] Tier 2:
1. SFX prompt template per category: `[gunfire, AK-47, single shot, indoor reverb, tactical shooter, gameplay-ready]`
2. Stable Audio Open 1.0 generates 5-10 candidates per prompt.
3. AI agent reviews + selects best.
4. Audacity / RX (cleanup) automated via Python.
5. Caption + tag + commit to library.

## Caption Coverage

Per DR-020 + DR-012:

| Audio Type | Caption Required |
|---|---|
| Critical SFX (gunfire, alarm, explosion) | YES |
| Voice / dialogue | YES |
| Music swell (key gameplay event) | YES (mood description: "tense music," "victorious swell") |
| Ambient | NO (atmosphere only) |
| UI tick | NO |

CI gate: `cf-caption-check` validates every critical SFX has caption.

## File Format

```ron
// content/music/world_mars_ambient.ron
track: (
    id: "world_mars_ambient",
    duration_s: 240,
    layers: [
        ( id: "drone_pad", file: "tier3/music/world_mars_ambient_drone.ogg", default_volume: 0.6 ),
        ( id: "percussion", file: "tier3/music/world_mars_ambient_perc.ogg", default_volume: 0.4 ),
        ( id: "melody", file: "tier3/music/world_mars_ambient_melody.ogg", default_volume: 0.3 ),
        ( id: "tension", file: "tier3/music/world_mars_ambient_tension.ogg", default_volume: 0.0 ),
    ],
    license: "Suno v5; project-owner generated; commercial use OK; logged in usage-ledger",
    bpm: 80,
    tempo_synced: true,
)

// content/sfx/weapon_ak47_fire.ron
sfx: (
    id: "weapon_ak47_fire",
    file: "tier3/sfx/weapon_ak47_fire.ogg",
    category: "weapon_fire",
    duration_s: 0.4,
    spatial: true,
    falloff_radius_m: 100,
    caption: "AK-47 gunfire",
    license: "Stable Audio Open 1.0; project-owner generated; Apache-2.0",
)
```

## Done-Criteria

- [ ] All 30+ launch tracks composed + mastered.
- [ ] All 400+ SFX clips generated + tagged + caption-bound.
- [ ] Adaptive layering responds correctly to match phase / EnvironmentSignal.
- [ ] FMOD or Kira mix passes diegetic-first per DR-020.
- [ ] Caption coverage 100% on critical audio.
- [ ] Spatial audio works for voice/radio (Steam Audio integration).
- [ ] usage-ledger covers every track + SFX.
- [ ] CI gate: every track + SFX has license + caption + replay event coverage.

## Source Trail

- [[decisions/dr-020-audio-identity]]
- [[decisions/dr-044-audiovisual-production-pipeline]]
- Stable Audio Open 1.0: https://stability.ai/news-updates/introducing-stable-audio-open
- Suno v5: https://suno.com/
- Udio v2: https://www.udio.com/
- MusicGen: https://github.com/facebookresearch/audiocraft
- bevy_kira_audio: https://crates.io/crates/bevy_kira_audio
- bevy_fmod: https://github.com/Salzian/bevy_fmod
