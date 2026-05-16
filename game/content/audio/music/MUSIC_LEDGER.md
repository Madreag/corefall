# Music Ledger — Corefall game/content/audio/music

**Last updated**: 2026-05-15 (Phoenix MST)  
**Branch state**: 39 Tier 2 baked (37 clean + 2 broken) + 81 Tier 1 procedural placeholders = 120 total music WAVs  
**Source of truth for prompts**: `../sfx/music_tracks_prompts.json` (canonical manifest; this file is the bake-status mirror).

---

## TL;DR

- **37 files** already at Tier 2 (ElevenLabs Music API) — **clean, ship-ready**.
- **2 files** at Tier 2 but **broken bakes** — need re-baking.
- **81 files** are still **Tier 1 procedural numpy synth** — sound like harsh static; need to be replaced.
- **Total needing fresh bake: 83 files** (the 81 never-baked + the 2 broken Tier 2).

Filename convention: `<track_id>_<variant>.wav` where variant ∈ {calm, buildup, climax, debrief}.

---

## Tier 2 clean — 37 files (DO NOT REGENERATE)

These have already been baked via ElevenLabs Music v1 and pass spectral health checks. Leave them alone unless you find a specific quality issue.

| File | Duration | RMS | hi/lo ratio |
|---|---|---|---|
| `music_world_belt_buildup.wav` | 240s | 0.0727 | 0.0470 |
| `music_world_belt_calm.wav` | 240s | 0.0719 | 0.0006 |
| `music_world_belt_climax.wav` | 240s | 0.1044 | 0.0335 |
| `music_world_deimos_calm.wav` | 240s | 0.0878 | 0.0022 |
| `music_world_deimos_climax.wav` | 240s | 0.0991 | 0.0017 |
| `music_world_deimos_debrief.wav` | 240s | 0.0668 | 0.0009 |
| `music_world_earth_buildup.wav` | 240s | 0.0973 | 0.0168 |
| `music_world_earth_calm.wav` | 240s | 0.0717 | 0.0007 |
| `music_world_earth_climax.wav` | 240s | 0.0839 | 0.0359 |
| `music_world_earth_debrief.wav` | 240s | 0.0719 | 0.0033 |
| `music_world_europa_buildup.wav` | 240s | 0.0880 | 0.0005 |
| `music_world_europa_calm.wav` | 240s | 0.0736 | 0.0007 |
| `music_world_europa_climax.wav` | 240s | 0.0782 | 0.0113 |
| `music_world_europa_debrief.wav` | 240s | 0.0685 | 0.0017 |
| `music_world_mars_buildup.wav` | 240s | 0.0797 | 0.0066 |
| `music_world_mars_calm.wav` | 240s | 0.0965 | 0.0062 |
| `music_world_mars_climax.wav` | 240s | 0.0914 | 0.0536 |
| `music_world_mars_debrief.wav` | 240s | 0.0853 | 0.0011 |
| `music_world_mimas_buildup.wav` | 240s | 0.0983 | 0.0086 |
| `music_world_mimas_calm.wav` | 240s | 0.0732 | 0.0017 |
| `music_world_mimas_climax.wav` | 240s | 0.0848 | 0.0159 |
| `music_world_mimas_debrief.wav` | 240s | 0.0547 | 0.0052 |
| `music_world_moon_buildup.wav` | 240s | 0.1181 | 0.0013 |
| `music_world_moon_calm.wav` | 240s | 0.0633 | 0.0050 |
| `music_world_moon_climax.wav` | 240s | 0.0883 | 0.0791 |
| `music_world_moon_debrief.wav` | 240s | 0.0540 | 0.0015 |
| `music_world_phobos_buildup.wav` | 240s | 0.1089 | 0.0622 |
| `music_world_phobos_climax.wav` | 240s | 0.0901 | 0.0288 |
| `music_world_phobos_debrief.wav` | 240s | 0.0692 | 0.0017 |
| `music_world_venus_buildup.wav` | 240s | 0.1098 | 0.0492 |
| `music_world_venus_calm.wav` | 240s | 0.0701 | 0.0022 |
| `music_world_venus_climax.wav` | 240s | 0.0822 | 0.0320 |
| `music_world_venus_debrief.wav` | 240s | 0.0460 | 0.0032 |
| `music_world_vulcan_buildup.wav` | 240s | 0.0932 | 0.0295 |
| `music_world_vulcan_calm.wav` | 240s | 0.0651 | 0.0092 |
| `music_world_vulcan_climax.wav` | 240s | 0.0889 | 0.0125 |
| `music_world_vulcan_debrief.wav` | 240s | 0.0852 | 0.0035 |

---

## Tier 2 broken — 2 files (RE-BAKE)

These were baked via ElevenLabs but ended up near-silent or noisy (probably partial-stream from a rate-limit interruption). Re-bake when credits allow.

| File | RMS | hi/lo ratio | Symptom |
|---|---|---|---|
| `music_world_deimos_buildup.wav` | 0.0941 | 0.1388 | noisy/static |
| `music_world_phobos_calm.wav` | 0.0043 | 1.6040 | near-silent, noisy/static |

---

## Needs bake — 83 files (Tier 1 procedural OR broken Tier 2)

These are the targets for the next round of music generation. The procedural Tier 1 fallback files at these paths sound like static/harsh synth and were never meant to ship.

**Grouped by category** for easier batch generation:

### World ambient (11 files)

| File | Dur | BPM | Key | Reason |
|---|---|---|---|---|
| `music_world_belt_debrief.wav` | 240s | 92 | G# minor | Tier 1 procedural (static) |
| `music_world_deimos_buildup.wav` | 240s | 78 | B minor | broken Tier 2 bake |
| `music_world_orbital_buildup.wav` | 240s | 90 | F major | Tier 1 procedural (static) |
| `music_world_orbital_calm.wav` | 240s | 90 | F major | Tier 1 procedural (static) |
| `music_world_orbital_climax.wav` | 240s | 90 | F major | Tier 1 procedural (static) |
| `music_world_orbital_debrief.wav` | 240s | 90 | F major | Tier 1 procedural (static) |
| `music_world_phobos_calm.wav` | 240s | 70 | G minor | broken Tier 2 bake |
| `music_world_sol_zone_buildup.wav` | 240s | 80 | Bb major | Tier 1 procedural (static) |
| `music_world_sol_zone_calm.wav` | 240s | 80 | Bb major | Tier 1 procedural (static) |
| `music_world_sol_zone_climax.wav` | 240s | 80 | Bb major | Tier 1 procedural (static) |
| `music_world_sol_zone_debrief.wav` | 240s | 80 | Bb major | Tier 1 procedural (static) |

### Faction theme (32 files)

| File | Dur | BPM | Key | Reason |
|---|---|---|---|---|
| `music_faction_coalition_buildup.wav` | 180s | 110 | C major | Tier 1 procedural (static) |
| `music_faction_coalition_calm.wav` | 180s | 110 | C major | Tier 1 procedural (static) |
| `music_faction_coalition_climax.wav` | 180s | 110 | C major | Tier 1 procedural (static) |
| `music_faction_coalition_debrief.wav` | 180s | 110 | C major | Tier 1 procedural (static) |
| `music_faction_collective_buildup.wav` | 180s | 95 | G minor | Tier 1 procedural (static) |
| `music_faction_collective_calm.wav` | 180s | 95 | G minor | Tier 1 procedural (static) |
| `music_faction_collective_climax.wav` | 180s | 95 | G minor | Tier 1 procedural (static) |
| `music_faction_collective_debrief.wav` | 180s | 95 | G minor | Tier 1 procedural (static) |
| `music_faction_collegium_buildup.wav` | 180s | 70 | F major | Tier 1 procedural (static) |
| `music_faction_collegium_calm.wav` | 180s | 70 | F major | Tier 1 procedural (static) |
| `music_faction_collegium_climax.wav` | 180s | 70 | F major | Tier 1 procedural (static) |
| `music_faction_collegium_debrief.wav` | 180s | 70 | F major | Tier 1 procedural (static) |
| `music_faction_frontier_buildup.wav` | 180s | 95 | G major | Tier 1 procedural (static) |
| `music_faction_frontier_calm.wav` | 180s | 95 | G major | Tier 1 procedural (static) |
| `music_faction_frontier_climax.wav` | 180s | 95 | G major | Tier 1 procedural (static) |
| `music_faction_frontier_debrief.wav` | 180s | 95 | G major | Tier 1 procedural (static) |
| `music_faction_husks_buildup.wav` | 180s | 80 | B minor | Tier 1 procedural (static) |
| `music_faction_husks_calm.wav` | 180s | 80 | B minor | Tier 1 procedural (static) |
| `music_faction_husks_climax.wav` | 180s | 80 | B minor | Tier 1 procedural (static) |
| `music_faction_husks_debrief.wav` | 180s | 80 | B minor | Tier 1 procedural (static) |
| `music_faction_ronin_buildup.wav` | 180s | 88 | D minor | Tier 1 procedural (static) |
| `music_faction_ronin_calm.wav` | 180s | 88 | D minor | Tier 1 procedural (static) |
| `music_faction_ronin_climax.wav` | 180s | 88 | D minor | Tier 1 procedural (static) |
| `music_faction_ronin_debrief.wav` | 180s | 88 | D minor | Tier 1 procedural (static) |
| `music_faction_starlight_buildup.wav` | 180s | 65 | A major | Tier 1 procedural (static) |
| `music_faction_starlight_calm.wav` | 180s | 65 | A major | Tier 1 procedural (static) |
| `music_faction_starlight_climax.wav` | 180s | 65 | A major | Tier 1 procedural (static) |
| `music_faction_starlight_debrief.wav` | 180s | 65 | A major | Tier 1 procedural (static) |
| `music_faction_synth_buildup.wav` | 180s | 105 | A minor | Tier 1 procedural (static) |
| `music_faction_synth_calm.wav` | 180s | 105 | A minor | Tier 1 procedural (static) |
| `music_faction_synth_climax.wav` | 180s | 105 | A minor | Tier 1 procedural (static) |
| `music_faction_synth_debrief.wav` | 180s | 105 | A minor | Tier 1 procedural (static) |

### Storyteller theme (20 files)

| File | Dur | BPM | Key | Reason |
|---|---|---|---|---|
| `music_storyteller_cassandra_buildup.wav` | 180s | 95 | C minor | Tier 1 procedural (static) |
| `music_storyteller_cassandra_calm.wav` | 180s | 95 | C minor | Tier 1 procedural (static) |
| `music_storyteller_cassandra_climax.wav` | 180s | 95 | C minor | Tier 1 procedural (static) |
| `music_storyteller_cassandra_debrief.wav` | 180s | 95 | C minor | Tier 1 procedural (static) |
| `music_storyteller_ironman_buildup.wav` | 180s | 88 | G minor | Tier 1 procedural (static) |
| `music_storyteller_ironman_calm.wav` | 180s | 88 | G minor | Tier 1 procedural (static) |
| `music_storyteller_ironman_climax.wav` | 180s | 88 | G minor | Tier 1 procedural (static) |
| `music_storyteller_ironman_debrief.wav` | 180s | 88 | G minor | Tier 1 procedural (static) |
| `music_storyteller_phoebe_buildup.wav` | 180s | 80 | Bb major | Tier 1 procedural (static) |
| `music_storyteller_phoebe_calm.wav` | 180s | 80 | Bb major | Tier 1 procedural (static) |
| `music_storyteller_phoebe_climax.wav` | 180s | 80 | Bb major | Tier 1 procedural (static) |
| `music_storyteller_phoebe_debrief.wav` | 180s | 80 | Bb major | Tier 1 procedural (static) |
| `music_storyteller_randy_buildup.wav` | 180s | 110 | F# minor | Tier 1 procedural (static) |
| `music_storyteller_randy_calm.wav` | 180s | 110 | F# minor | Tier 1 procedural (static) |
| `music_storyteller_randy_climax.wav` | 180s | 110 | F# minor | Tier 1 procedural (static) |
| `music_storyteller_randy_debrief.wav` | 180s | 110 | F# minor | Tier 1 procedural (static) |
| `music_storyteller_sandbox_buildup.wav` | 180s | 75 | D major | Tier 1 procedural (static) |
| `music_storyteller_sandbox_calm.wav` | 180s | 75 | D major | Tier 1 procedural (static) |
| `music_storyteller_sandbox_climax.wav` | 180s | 75 | D major | Tier 1 procedural (static) |
| `music_storyteller_sandbox_debrief.wav` | 180s | 75 | D major | Tier 1 procedural (static) |

### Boss theme (20 files)

| File | Dur | BPM | Key | Reason |
|---|---|---|---|---|
| `music_boss_crimson_tide_buildup.wav` | 240s | 105 | F minor | Tier 1 procedural (static) |
| `music_boss_crimson_tide_calm.wav` | 240s | 105 | F minor | Tier 1 procedural (static) |
| `music_boss_crimson_tide_climax.wav` | 240s | 105 | F minor | Tier 1 procedural (static) |
| `music_boss_crimson_tide_debrief.wav` | 240s | 105 | F minor | Tier 1 procedural (static) |
| `music_boss_eclipse_walker_buildup.wav` | 240s | 102 | C# minor | Tier 1 procedural (static) |
| `music_boss_eclipse_walker_calm.wav` | 240s | 102 | C# minor | Tier 1 procedural (static) |
| `music_boss_eclipse_walker_climax.wav` | 240s | 102 | C# minor | Tier 1 procedural (static) |
| `music_boss_eclipse_walker_debrief.wav` | 240s | 102 | C# minor | Tier 1 procedural (static) |
| `music_boss_frozen_heart_buildup.wav` | 240s | 95 | B minor | Tier 1 procedural (static) |
| `music_boss_frozen_heart_calm.wav` | 240s | 95 | B minor | Tier 1 procedural (static) |
| `music_boss_frozen_heart_climax.wav` | 240s | 95 | B minor | Tier 1 procedural (static) |
| `music_boss_frozen_heart_debrief.wav` | 240s | 95 | B minor | Tier 1 procedural (static) |
| `music_boss_hollow_king_buildup.wav` | 240s | 100 | D minor | Tier 1 procedural (static) |
| `music_boss_hollow_king_calm.wav` | 240s | 100 | D minor | Tier 1 procedural (static) |
| `music_boss_hollow_king_climax.wav` | 240s | 100 | D minor | Tier 1 procedural (static) |
| `music_boss_hollow_king_debrief.wav` | 240s | 100 | D minor | Tier 1 procedural (static) |
| `music_boss_last_star_buildup.wav` | 300s | 110 | A minor | Tier 1 procedural (static) |
| `music_boss_last_star_calm.wav` | 300s | 110 | A minor | Tier 1 procedural (static) |
| `music_boss_last_star_climax.wav` | 300s | 110 | A minor | Tier 1 procedural (static) |
| `music_boss_last_star_debrief.wav` | 300s | 110 | A minor | Tier 1 procedural (static) |

---

## How to regenerate

Four practical paths (pick one):

### Path A — top up ElevenLabs credits (cheapest if your API key still works)

1. Sign in to https://elevenlabs.io/app/subscription and either upgrade the plan or buy a one-time credit pack large enough for ~500k characters (each music compose burns ~5k–10k credits per ~240s track; budget ~80 × 7k ≈ 560k characters for the remaining 83 files).
2. Once credits are restored, run from the repo root:

   ```bash
   cd /Users/erol/projects/corefall
   tools/asset_gen/.venv/bin/python tools/audio_pipeline/eleven_music.py --resume
   ```
3. `--resume` will skip the 39 already-completed files and only re-bake the 81 in `tools/audio_pipeline/_state/eleven_music_progress.json::failed` plus the never-attempted ones. **The 2 broken Tier 2 files (phobos_calm, deimos_buildup) need to be removed from `completed` first** so resume picks them up:

   ```bash
   tools/asset_gen/.venv/bin/python -c "
   import json
   p = json.load(open('tools/audio_pipeline/_state/eleven_music_progress.json'))
   for bad in ['music_world_phobos_calm', 'music_world_deimos_buildup']:
       if bad in p['completed']: p['completed'].remove(bad)
   json.dump(p, open('tools/audio_pipeline/_state/eleven_music_progress.json','w'), indent=2)
   "
   ```

### Path B — hand off to an AIVA Pro agent (parallel work, no extra ElevenLabs $$)

There is a complete self-contained handoff doc at `/Users/erol/projects/corefall/HANDOFF_AIVA_MUSIC.md`. It contains every prompt from `music_tracks_prompts.json` plus AIVA web-UI automation guidance (Playwright). Hand the doc to a separate agent — they bake the 83 missing tracks and deliver them back as `corefall_aiva_music_bake_v1.zip`. Once you receive the zip, drop the WAVs over the existing files at `game/content/audio/music/<track_id>_<variant>.wav` and re-run the local ledger update:

```bash
# After unzipping the AIVA bake into game/content/audio/music/, run:
tools/asset_gen/.venv/bin/python tools/audio_pipeline/eleven_music.py --tracks <comma-separated-track-ids> # this just refreshes ledger entries
# Better: add a dedicated ingest_aiva.py script (TODO)
```

### Path C — local bake on a CUDA-capable GPU (free, slowest setup)

Run Stable Audio Open or MusicGen-melody locally on the RTX 5090 / 32GB VRAM machine. Both are open-weights with permissive licenses for game audio.

- **Stable Audio Open 1.0** (Stability AI): https://huggingface.co/stabilityai/stable-audio-open-1.0 — text→audio up to 47s, decent quality, free.
- **MusicGen-melody / MusicGen-large** (Meta AudioCraft): https://github.com/facebookresearch/audiocraft — text→music up to 30s natively, can extend via the SDK.
- For 240s game loops we need to splice four 60s segments per track + cross-fade. The `tools/audio_pipeline/` scaffold has the post-processing helpers (`post_process.py::cleanup_wav`) already.

A local bake script doesn't exist yet — when the time comes, add `tools/audio_pipeline/local_music_bake.py` that mirrors the `eleven_music.py` structure but calls a local Stable Audio Open inference loop instead.

### Path D — delete the 83 placeholders and ship silence for those scenarios (lowest-effort fallback)

If shipping silence is preferable to shipping static:

```bash
cd /Users/erol/projects/corefall
# remove the 83 placeholder WAVs and let the engine fall back to silence
tools/asset_gen/.venv/bin/python -c "
import json, os
for r in json.load(open('/tmp/music_ledger_data/needs_bake.json')):
    p = f'game/content/audio/music/{r[\"file_id\"]}.wav'
    if os.path.exists(p): os.unlink(p)
    print('rm', p)
"
# then remove the corresponding ledger rows (script in tools/audio_pipeline/restore_audio_ledger.py)
```

---

## Per-track prompts + parameters (for whatever bake path you choose)

Source of truth: `game/content/sfx/music_tracks_prompts.json`. The full per-variant `musicgen_prompt`, `seed`, `tempo_bpm`, and `key` are inlined below for the 83 files that need baking, so you can copy-paste straight into AIVA's prompt field, ElevenLabs `music.compose(prompt=...)`, or a Stable Audio Open inference call.

Naming: each file is `<track_id>_<variant>.wav`. Variants: `calm` (intensity 0.0-0.3), `buildup` (0.3-0.6), `climax` (0.6-1.0), `debrief` (post-encounter wind-down). The downstream adaptive music engine cross-fades between variants per the in-game intensity float.

### World ambient prompts

#### `music_world_belt` — Belt Asteroid Mining (240s, 92 BPM, key G# minor)

- **music_world_belt_debrief**  
  - seed: `200004`
  - prompt:
    > Belt debrief, slow synth pad in G# minor + drill winding down + distant rock clink + radio sign-off, 70 BPM, hard-won score

#### `music_world_deimos` — Deimos Mining Colony (240s, 78 BPM, key B minor)

- **music_world_deimos_buildup** **(broken bake — needs redo)**  
  - seed: `150002`
  - prompt:
    > Deimos tension, mining drills accelerating + bass throb in B minor + percussion pulse + alarm warm-up, 92 BPM, ore vein collapse imminent

#### `music_world_orbital` — Orbital Station Interior (240s, 90 BPM, key F major)

- **music_world_orbital_calm**  
  - seed: `210001`
  - prompt:
    > Orbital station interior ambient, gentle air recycler hum + computer beeps + distant footsteps + synth pad in F major + faint elevator-jazz harmonics, 90 BPM, civilian-station comfort

- **music_world_orbital_buildup**  
  - seed: `210002`
  - prompt:
    > Orbital station tension, klaxon priming + bass throb in F minor + computer alarms + bulkhead clank, 100 BPM, hull breach warning

- **music_world_orbital_climax**  
  - seed: `210003`
  - prompt:
    > Orbital station combat, frantic electronic in F minor + driving drums + alarm pulse + bulkhead slam + station-PA shouts, 125 BPM, zero-g station boarding

- **music_world_orbital_debrief**  
  - seed: `210004`
  - prompt:
    > Orbital station debrief, mellow synth pad in F major + air recycler slow + computer chimes + medbay piano, 75 BPM, station recovery

#### `music_world_phobos` — Phobos Microgravity Asteroid (240s, 70 BPM, key G minor)

- **music_world_phobos_calm** **(broken bake — needs redo)**  
  - seed: `140001`
  - prompt:
    > Phobos microgravity asteroid ambient, eerie silence with floating debris + creaking structural metal + faint synth drone in G minor + sparse chime, 70 BPM, no drums, weightless dread

#### `music_world_sol_zone` — Sol Zone Stellar Edge (240s, 80 BPM, key Bb major)

- **music_world_sol_zone_calm**  
  - seed: `220001`
  - prompt:
    > Sol-zone habitat near-star ambient, intense solar wind + crystalline resonance + radiation hum + synth pad in Bb major + cathedral organ + ethereal choir, 80 BPM, sacred stellar awe

- **music_world_sol_zone_buildup**  
  - seed: `220002`
  - prompt:
    > Sol-zone tension, solar flare crescendo + bass swell in Bb major + radiation alarm + brass build, 95 BPM, flare incoming

- **music_world_sol_zone_climax**  
  - seed: `220003`
  - prompt:
    > Sol-zone combat, triumphant orchestral in Bb major + full choir + heavy brass + thundering drums + stellar-wind howl, 120 BPM, climactic stellar wrath

- **music_world_sol_zone_debrief**  
  - seed: `220004`
  - prompt:
    > Sol-zone debrief, sustained organ in Bb major + soft choir + receding solar wind + distant chime, 65 BPM, sacred relief

### Faction theme prompts

#### `music_faction_coalition` — Coalition Theme (180s, 110 BPM, key C major)

- **music_faction_coalition_calm**  
  - seed: `310001`
  - prompt:
    > Coalition faction theme calm, militaristic orchestral with strong brass + snare + heroic melody in C major + civilian-radio fanfare + disciplined humanity, 110 BPM, ordered hope

- **music_faction_coalition_buildup**  
  - seed: `310002`
  - prompt:
    > Coalition buildup, brass swell + tactical snare in C major + bass pulse + radio command chatter, 118 BPM, mobilization

- **music_faction_coalition_climax**  
  - seed: `310003`
  - prompt:
    > Coalition combat, modern military electronic in E minor + electric guitar + heavy drums + brass + synth + sergeant shouts, 125 BPM, aggressive precision

- **music_faction_coalition_debrief**  
  - seed: `310004`
  - prompt:
    > Coalition debrief, solemn brass in C major + slow snare roll + bugle taps + sustained pad, 70 BPM, honored fallen

#### `music_faction_collective` — Collective Theme (180s, 95 BPM, key G minor)

- **music_faction_collective_calm**  
  - seed: `350001`
  - prompt:
    > Collective faction theme calm, industrial proletarian electronic with metal-scrap percussion + bass + radio static + worker chants + synth pad in G minor, 95 BPM, gritty solidarity

- **music_faction_collective_buildup**  
  - seed: `350002`
  - prompt:
    > Collective buildup, scrap-percussion accelerating + bass throb in G minor + worker drum + factory whistle, 108 BPM, strike forming

- **music_faction_collective_climax**  
  - seed: `350003`
  - prompt:
    > Collective combat, anarchic industrial in D minor + distorted bass + thrashing drums + machinery samples + crowd chants, 130 BPM, brutal direct revolt

- **music_faction_collective_debrief**  
  - seed: `350004`
  - prompt:
    > Collective debrief, slow factory hum in G minor + worker-choir hum + accordion + receding drum, 70 BPM, weary victory

#### `music_faction_collegium` — Collegium Theme (180s, 70 BPM, key F major)

- **music_faction_collegium_calm**  
  - seed: `370001`
  - prompt:
    > Collegium faction theme calm, monastic-scholarly ambient with Gregorian chant + drone organ + bell + soft strings + synth pad in F major + scriptorium quill, 70 BPM, contemplative knowledge

- **music_faction_collegium_buildup**  
  - seed: `370002`
  - prompt:
    > Collegium buildup, chant rising + bass swell in F major + organ build + tome-slam percussion, 85 BPM, ritual preparation

- **music_faction_collegium_climax**  
  - seed: `370003`
  - prompt:
    > Collegium combat, sacred orchestral battle in D minor + male choir + brass + heavy strings + church bells + righteous chant, 110 BPM, archive defense

- **music_faction_collegium_debrief**  
  - seed: `370004`
  - prompt:
    > Collegium debrief, sustained organ in F major + soft choir + library-quiet + soft chime, 60 BPM, sacred preservation

#### `music_faction_frontier` — Frontier Theme (180s, 95 BPM, key G major)

- **music_faction_frontier_calm**  
  - seed: `320001`
  - prompt:
    > Frontier faction theme calm, frontier folk-electronic with acoustic guitar + harmonica + light synth + steady drum + bottle-percussion in G major, 95 BPM, hardy independent settlers

- **music_faction_frontier_buildup**  
  - seed: `320002`
  - prompt:
    > Frontier buildup, harmonica building + bass strum in G major + tom drums + outlaw whistle, 105 BPM, posse forming

- **music_faction_frontier_climax**  
  - seed: `320003`
  - prompt:
    > Frontier combat, western-electronic battle in E minor + electric guitar + heavy drums + bass synth + harmonica + holler shouts, 125 BPM, defiant outlaws

- **music_faction_frontier_debrief**  
  - seed: `320004`
  - prompt:
    > Frontier debrief, slow acoustic guitar in G major + harmonica solo + saloon piano + crickets, 65 BPM, dusty homecoming

#### `music_faction_husks` — Husks Theme (180s, 80 BPM, key B minor)

- **music_faction_husks_calm**  
  - seed: `360001`
  - prompt:
    > Husks faction theme calm, alien-insectoid ambient with chittering + dissonant synth + drone + skittering percussion in B minor + distorted whisper, 80 BPM, unsettling hive presence

- **music_faction_husks_buildup**  
  - seed: `360002`
  - prompt:
    > Husks buildup, skitter crescendo + bass throb in B minor + insect chitter swarming + dissonant string, 100 BPM, hive convergence

- **music_faction_husks_climax**  
  - seed: `360003`
  - prompt:
    > Husks combat, frantic alien insectoid in F minor + skittering percussion + dissonant strings + screaming horns + chaos chant, 145 BPM, overwhelming hive frenzy

- **music_faction_husks_debrief**  
  - seed: `360004`
  - prompt:
    > Husks debrief, eerie drone in B minor + receding chitter + distant queen-call + heartbeat pulse, 60 BPM, alien quiet

#### `music_faction_ronin` — Ronin Theme (180s, 88 BPM, key D minor)

- **music_faction_ronin_calm**  
  - seed: `330001`
  - prompt:
    > Ronin faction theme calm, lone-wolf neo-noir with koto + electric piano + minimal synth pad in D minor + cyberpunk rain + lonely sax, 88 BPM, wandering blade-for-hire melancholy

- **music_faction_ronin_buildup**  
  - seed: `330002`
  - prompt:
    > Ronin buildup, koto pluck rising + bass pulse in D minor + taiko build + neon-flicker percussion, 100 BPM, duel imminent

- **music_faction_ronin_climax**  
  - seed: `330003`
  - prompt:
    > Ronin combat, cyberpunk samurai in D minor + driving taiko drums + electric guitar + koto + screaming synth lead, 130 BPM, blade-and-bullet ballet

- **music_faction_ronin_debrief**  
  - seed: `330004`
  - prompt:
    > Ronin debrief, solo koto in D minor + sustained pad + rain on neon + soft cello, 60 BPM, blood-stained reflection

#### `music_faction_starlight` — Starlight Theme (180s, 65 BPM, key A major)

- **music_faction_starlight_calm**  
  - seed: `380001`
  - prompt:
    > Starlight faction theme calm, solar-ritual ambient with bell tones + drone synth + cathedral organ + light percussion + synth pad in A major + ethereal choir, 65 BPM, religious science

- **music_faction_starlight_buildup**  
  - seed: `380002`
  - prompt:
    > Starlight buildup, choir building + bass swell in A major + ritual-bell crescendo + sunburst harp, 85 BPM, illumination rite

- **music_faction_starlight_climax**  
  - seed: `380003`
  - prompt:
    > Starlight combat, ritualistic orchestral in D minor + full choir + brass + tribal drums + organ + ecstatic chant, 115 BPM, fanatical fervor

- **music_faction_starlight_debrief**  
  - seed: `380004`
  - prompt:
    > Starlight debrief, sustained organ in A major + soft choir + receding bells + harp glissando, 55 BPM, illuminated peace

#### `music_faction_synth` — Synth Theme (180s, 105 BPM, key A minor)

- **music_faction_synth_calm**  
  - seed: `340001`
  - prompt:
    > Synth faction theme calm, robotic drone-collective ambient, synthetic monotone choir + arpeggiated sequencer + cold synth pad in A minor + circuit-glitch percussion, 105 BPM, machine consensus

- **music_faction_synth_buildup**  
  - seed: `340002`
  - prompt:
    > Synth buildup, sequencer accelerating + bass pulse in A minor + glitch percussion + hive-mind ping, 115 BPM, swarm coalescing

- **music_faction_synth_climax**  
  - seed: `340003`
  - prompt:
    > Synth combat, frantic electronic in A minor + arpeggiated sequencer at full speed + heavy drums + distorted bass + robotic vocals, 140 BPM, mechanized overwhelm

- **music_faction_synth_debrief**  
  - seed: `340004`
  - prompt:
    > Synth debrief, slow arpeggio in A minor + synthetic pad + cooling-fan whir + bell tone, 70 BPM, machine satisfaction

### Storyteller theme prompts

#### `music_storyteller_cassandra` — Cassandra Classic Theme (180s, 95 BPM, key C minor)

- **music_storyteller_cassandra_calm**  
  - seed: `410001`
  - prompt:
    > Cassandra Classic narrative theme calm, balanced cinematic synth pad in C minor + soft strings + measured piano + steady heartbeat percussion, 95 BPM, fair storyteller pacing

- **music_storyteller_cassandra_buildup**  
  - seed: `410002`
  - prompt:
    > Cassandra Classic buildup, strings swell + bass pulse in C minor + tension percussion + ascending piano motif, 105 BPM, the story turns

- **music_storyteller_cassandra_climax**  
  - seed: `410003`
  - prompt:
    > Cassandra Classic event climax, orchestral in C minor + heavy strings + brass + percussion + leitmotif return, 120 BPM, dramatic incident

- **music_storyteller_cassandra_debrief**  
  - seed: `410004`
  - prompt:
    > Cassandra Classic debrief, slow piano outro in C minor + soft strings + lone violin, 65 BPM, balanced reflection

#### `music_storyteller_ironman` — Ironman Theme (180s, 88 BPM, key G minor)

- **music_storyteller_ironman_calm**  
  - seed: `440001`
  - prompt:
    > Ironman narrative theme calm, grim permadeath ambient, low synth drone in G minor + sparse cello + military snare + ticking clock + heartbeat pulse, 88 BPM, no second chances

- **music_storyteller_ironman_buildup**  
  - seed: `440002`
  - prompt:
    > Ironman buildup, low strings rising + bass throb in G minor + snare roll + funeral-bell tease, 100 BPM, irreversible threat

- **music_storyteller_ironman_climax**  
  - seed: `440003`
  - prompt:
    > Ironman event climax, dark orchestral in G minor + heavy strings + funeral brass + drum-roll + ominous choir, 115 BPM, life-or-death stakes

- **music_storyteller_ironman_debrief**  
  - seed: `440004`
  - prompt:
    > Ironman debrief, solo cello in G minor + sustained drone + funeral bell + sparse piano, 55 BPM, permadeath aftermath

#### `music_storyteller_phoebe` — Phoebe Chillax Theme (180s, 80 BPM, key Bb major)

- **music_storyteller_phoebe_calm**  
  - seed: `420001`
  - prompt:
    > Phoebe Chillax narrative theme calm, mellow lofi synth pad in Bb major + soft piano + jazz brush percussion + sparse warm bass, 80 BPM, player-friendly mellow

- **music_storyteller_phoebe_buildup**  
  - seed: `420002`
  - prompt:
    > Phoebe Chillax buildup, warm strings swelling + soft bass in Bb major + light percussion + gentle vibraphone, 90 BPM, light tension

- **music_storyteller_phoebe_climax**  
  - seed: `420003`
  - prompt:
    > Phoebe Chillax event climax, light orchestral in Bb major + warm brass + brush drums + piano melody + uplifting choir, 105 BPM, generous challenge

- **music_storyteller_phoebe_debrief**  
  - seed: `420004`
  - prompt:
    > Phoebe Chillax debrief, mellow piano outro in Bb major + soft strings + smiling vibraphone, 60 BPM, gentle wind-down

#### `music_storyteller_randy` — Randy Random Theme (180s, 110 BPM, key F# minor)

- **music_storyteller_randy_calm**  
  - seed: `430001`
  - prompt:
    > Randy Random narrative theme calm, chaotic-unpredictable synth pad in F# minor + glitch percussion + erratic piano stabs + random pitch sweeps, 110 BPM, anything goes

- **music_storyteller_randy_buildup**  
  - seed: `430002`
  - prompt:
    > Randy Random buildup, escalating chaos in F# minor + accelerating drums + dissonant strings + alarm sweeps, 125 BPM, unpredictable cascade

- **music_storyteller_randy_climax**  
  - seed: `430003`
  - prompt:
    > Randy Random event climax, frantic electronic in F# minor + double-time drums + dissonant brass + screaming synth + chaos percussion, 145 BPM, total mayhem

- **music_storyteller_randy_debrief**  
  - seed: `430004`
  - prompt:
    > Randy Random debrief, surreal pad in F# minor + erratic glitch fading + soft chime + breath of relief, 75 BPM, chaos receding

#### `music_storyteller_sandbox` — Sandbox Theme (180s, 75 BPM, key D major)

- **music_storyteller_sandbox_calm**  
  - seed: `450001`
  - prompt:
    > Sandbox narrative theme calm, pure-exploration ambient with airy synth pad in D major + acoustic guitar + bird-call samples + minimal percussion + harp glissando, 75 BPM, no-pressure curiosity

- **music_storyteller_sandbox_buildup**  
  - seed: `450002`
  - prompt:
    > Sandbox buildup, soft strings swelling + warm bass in D major + light percussion + discovery harp, 85 BPM, mild surprise

- **music_storyteller_sandbox_climax**  
  - seed: `450003`
  - prompt:
    > Sandbox climax, exploration orchestral in D major + soaring strings + brass crescendo + uplifting drum + choir of wonder, 105 BPM, major discovery

- **music_storyteller_sandbox_debrief**  
  - seed: `450004`
  - prompt:
    > Sandbox debrief, acoustic guitar outro in D major + soft strings + harp + gentle wind, 60 BPM, contented exploration

### Boss theme prompts

#### `music_boss_crimson_tide` — The Crimson Tide Theme (240s, 105 BPM, key F minor)

- **music_boss_crimson_tide_calm**  
  - seed: `530001`
  - prompt:
    > Crimson Tide boss arena entry, dust-storm orchestral with rust-grit percussion + heavy brass + low synth pad in F minor + Bedouin choir + sand-walker chant, 105 BPM, sandstorm-titan looms

- **music_boss_crimson_tide_buildup**  
  - seed: `530002`
  - prompt:
    > Crimson Tide phase 1/2 buildup, sandstorm crescendo + bass throb in F minor + driving tribal drum + windswept brass + war chant, 120 BPM, swarm tide rising

- **music_boss_crimson_tide_climax**  
  - seed: `530003`
  - prompt:
    > Crimson Tide phase 3/4 combat, furious orchestral in F minor + full tribal drums + heavy brass + dust-storm howl + war choir + creature roars + crumbling-arena rumble, 135 BPM, four-phase sand-titan war

- **music_boss_crimson_tide_debrief**  
  - seed: `530004`
  - prompt:
    > Crimson Tide defeat, settling-dust orchestral in F minor + receding tribal drum + lone reed flute + sparse cello + wind sigh, 70 BPM, sand-buried lament

#### `music_boss_eclipse_walker` — The Eclipse Walker Theme (240s, 102 BPM, key C# minor)

- **music_boss_eclipse_walker_calm**  
  - seed: `540001`
  - prompt:
    > Eclipse Walker boss arena entry, microgravity-eerie orchestral with floating synth pad in C# minor + ethereal choir + gravity-warp synth + slow heartbeat + reverb cello, 102 BPM, weightless cyborg presence

- **music_boss_eclipse_walker_buildup**  
  - seed: `540002`
  - prompt:
    > Eclipse Walker phase 1 buildup, cyborg-precision crescendo + bass pulse in C# minor + tight drum + glitch percussion + ascending choir, 118 BPM, gravity inversion incoming

- **music_boss_eclipse_walker_climax**  
  - seed: `540003`
  - prompt:
    > Eclipse Walker phase 2/3 combat, frantic electronic-orchestral in C# minor + full drum + brass + cyborg-vocals + gravity-warp synth lead + agile percussion, 132 BPM, microgravity duel

- **music_boss_eclipse_walker_debrief**  
  - seed: `540004`
  - prompt:
    > Eclipse Walker defeat, drifting synth pad in C# minor + receding choir + soft chime + cooling-cyborg whine, 65 BPM, weightless lament

#### `music_boss_frozen_heart` — The Frozen Heart Theme (240s, 95 BPM, key B minor)

- **music_boss_frozen_heart_calm**  
  - seed: `520001`
  - prompt:
    > Frozen Heart boss arena entry, glacial dread orchestral with ice-crystal chimes + low choir + cold synth pad in B minor + creature heartbeat + sonar ping, 95 BPM, deep-cold confrontation

- **music_boss_frozen_heart_buildup**  
  - seed: `520002`
  - prompt:
    > Frozen Heart phase 1 buildup, ice-crystal crescendo + cryogenic synth swell in B minor + driving drum + whisper choir + cold-snap percussion, 110 BPM, supercooled awakening

- **music_boss_frozen_heart_climax**  
  - seed: `520003`
  - prompt:
    > Frozen Heart phase 2/3 combat, frigid orchestral in B minor + full strings + ice-chime + thundering drum + creature roar + screaming brass + supercooled-core whine, 125 BPM, cryogenic meltdown war

- **music_boss_frozen_heart_debrief**  
  - seed: `520004`
  - prompt:
    > Frozen Heart defeat, mournful strings in B minor + descending chime + sparse cello + ice shatter + soft choir, 65 BPM, heart-of-ice lament

#### `music_boss_hollow_king` — The Hollow King Theme (240s, 100 BPM, key D minor)

- **music_boss_hollow_king_calm**  
  - seed: `510001`
  - prompt:
    > Hollow King boss arena entry, ominous building orchestral with menacing brass + tribal drums + low choir + bass + slow heartbeat in D minor, 100 BPM, confrontation looming

- **music_boss_hollow_king_buildup**  
  - seed: `510002`
  - prompt:
    > Hollow King phase 1 buildup, strings crescendo + brass build + tribal drums escalating in D minor + lava-crackle percussion + king's-voice chant, 115 BPM, flame king awakens

- **music_boss_hollow_king_climax**  
  - seed: `510003`
  - prompt:
    > Hollow King phase 2/3 combat, triumphant epic in D minor + full orchestra + battle choir + powerful brass + thundering drums + leitmotif + pyroclastic roar, 130 BPM, climactic flame king war

- **music_boss_hollow_king_debrief**  
  - seed: `510004`
  - prompt:
    > Hollow King defeat, somber orchestral cadence in D minor + descending strings + low brass + funeral choir + cooling-lava crackle, 70 BPM, fallen king lament

#### `music_boss_last_star` — The Last Star Theme (300s, 110 BPM, key A minor)

- **music_boss_last_star_calm**  
  - seed: `550001`
  - prompt:
    > Last Star superboss arena entry, stellar-cathedral orchestral with cathedral organ + full choir + low brass + slow heartbeat + synth pad in A minor + cosmic-wind howl, 110 BPM, end-of-campaign confrontation

- **music_boss_last_star_buildup**  
  - seed: `550002`
  - prompt:
    > Last Star phase 1/2 buildup, choir crescendo + organ build + bass throb in A minor + ascending strings + ritual percussion + stellar-flare hiss, 125 BPM, sol-zone-titan awakens

- **music_boss_last_star_climax**  
  - seed: `550003`
  - prompt:
    > Last Star phase 3/4/5 combat, climactic epic orchestral in A minor + full choir + powerful brass + thundering drums + leitmotif return + screaming synth lead + cosmic-roar samples + stellar-wrath howl, 140 BPM, end-game superboss war

- **music_boss_last_star_debrief**  
  - seed: `550004`
  - prompt:
    > Last Star defeat, triumphant resolved orchestral in A major + ascending choir + warm brass + sustained organ + soft drum + dawn-chime + receding stellar wind, 90 BPM, campaign-ending triumph

---

## Append-only ingest log

When the missing 83 files are eventually baked, append a one-line entry here so future readers can audit who/when/how:

```
# format: YYYY-MM-DD <agent> <path> <files_count>  <notes>
# example: 2026-05-15 droid eleven_music_v1 39/120  initial Tier 2 bake; rate limit + credit cap stopped at 39
```

Entries:

```
2026-05-15 droid eleven_music_v1 39/120  initial Tier 2 bake; rate limit + credit cap stopped at 39; 1 broken (phobos_calm); seed for resume captured in tools/audio_pipeline/_state/eleven_music_progress.json
```
