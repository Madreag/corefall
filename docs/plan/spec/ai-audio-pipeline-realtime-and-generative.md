---
type: spec
status: closed-direction
authority: "AI audio pipeline: 5-tier production (Stable Audio Open SFX, Suno/Udio/ElevenLabs music, MusicGen ambient, XTTS/Coqui/ElevenLabs voice, FMOD/Kira runtime) + real-time procedural audio (impact, doppler, atmospheric absorption, footstep variety, reverb, voice phoneme lip-sync, music adaptive layering). NO humans crafting any audio. Private prototypes are ledger-first; release/sale assets require cleanup or clearance."
ready_when: "All 400+ SFX clips generated + tagged + caption-bound; 30+ music tracks composed + stems exported; 24+ NPCs voice-cloned with 50-100 dialogue lines each; runtime adaptive layering responds; all per DR-053."
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
  - DR-053
  - DR-057
---

← [[spec/index|spec section]] · [[decisions/dr-053-ai-audio-pipeline-realtime-and-generative|DR-053]] · [[spec/audio-identity|audio identity]] · [[spec/music-and-soundtrack|music/soundtrack]] · [[spec/comms-voice-and-radio-model|voice/radio]]

# AI Audio Pipeline — Real-Time + Generative (No Humans)

> [!summary] What this page is
> Complete AI-driven audio production. NO humans crafting. 5 production tiers (Stable Audio Open + Suno/Udio + MusicGen + XTTS/Coqui + FMOD/Kira runtime) + real-time procedural audio (impact, doppler, atmospheric absorption, footstep variety, reverb, lip-sync, music adaptive layering).

## 5-Tier Production Pipeline

### Tier 1 — SFX Library Generation

400+ launch SFX clips: weapons, footsteps, equipment, environment, UI, combat, voice barks.

| Tool | Detail |
|---|---|
| Stable Audio Open 1.0 (Stability AI Community License; local 32GB VRAM) | Per-event prompt template + deterministic seed. 47s max clips at 44.1 kHz. Private prototypes allowed with ledger entry; commercial-use registration/enterprise review required before public sale/release assets ship. |
| Per-event prompt template | E.g., "AK-47 single shot indoor reverb tactical shooter gameplay-ready" |
| Auto-trim/normalize/EQ | librosa + numpy; remove silence + match -16 LUFS + EQ per category |
| Auto-caption | Generated from prompt; caption text bound to clip |
| Spatial flag | true/false; falloff_radius_m |

```python
# tools/audio_pipeline/generate_sfx.py (excerpt)
def generate_sfx(prompt_path, asset_id, seed=None):
    prompt = load_prompt(prompt_path)
    seed = seed or hash_seed(asset_id)
    audio = stable_audio_open.generate(prompt, seed, duration_s=2.0)
    audio = trim_silence(audio)
    audio = normalize_lufs(audio, -16)
    audio = eq_per_category(audio, prompt['category'])
    save_ogg(f'assets/tier3/sfx/{asset_id}.ogg', audio)
    log_to_usage_ledger(asset_id, prompt, seed, model='stable-audio-open-1.0')
    return asset_id
```

### Tier 2 — Hero Music Tracks

Main theme + 12 world themes + 6 combat layers + 4 base-tension + 4 menu/UI + 8 mission stings + 3 antagonist motifs.

| Tool | Detail |
|---|---|
| Suno v5 cloud OR Udio v2 cloud | Per-track prompt + seed. Iterations via AI agent. |
| Stem export | Demucs 4-stem split (drums/bass/melody/other) |
| Per-track license review | Private prototypes log terms at generation time; public sale/release requires cleanup or current clearance |
| Adaptive layer extraction | Per-track 4-6 stems; FMOD parameter automation per stem |

### Tier 3 — Ambient Music + Procedural

Long-loop ambient, world-specific drones, planet atmospherics.

| Tool | Detail |
|---|---|
| MusicGen/AudioCraft local | Prototype-only unless weights are replaced, self-trained, or commercially licensed. AudioCraft code is MIT; released weights are CC-BY-NC 4.0. |
| Per-world ambient | Generated per [[spec/celestial-bodies-and-worlds-model]] world spec |
| Stable Audio Open for stings | Short bursts (3-10s) |

### Tier 4 — Voice / NPC Dialogue

Per-faction commander voice + ~24 named NPCs + 50-100 dialogue lines per NPC.

| Tool | Detail |
|---|---|
| XTTS-v2 / Coqui TTS local | Free; 16 languages; 6s reference clip → cloned voice |
| ElevenLabs cloud | Hero NPCs; subscription available but not exclusive. Private prototypes log terms at generation time; public sale/release requires cleanup or current clearance. |
| Tortoise-TTS local (Apache-2.0) | High-quality offline; slower |
| Per-NPC voice model | Reference 6s clip stored in `content/audio/voice_models/<npc_id>/` |
| Phoneme lip-sync output | Animation tags (`mouth_phoneme_a/e/i/o/u`) per [[spec/animation-system]] |

```python
# tools/audio_pipeline/generate_voice.py (excerpt)
def generate_voice(npc_id, dialogue_text, lang='en'):
    voice_model = load_voice_model(npc_id)  # 6s reference
    audio = xtts.generate(
        text=dialogue_text,
        speaker_wav=voice_model,
        language=lang,
    )
    phonemes = extract_phonemes(audio, dialogue_text)
    save_ogg(f'assets/tier3/voice_dialogue/{npc_id}/{hash(dialogue_text)}.ogg', audio)
    save_phoneme_track(f'.../{hash(dialogue_text)}.phonemes', phonemes)
    log_to_usage_ledger(...)
```

### Tier 5 — Runtime Adaptive + Spatial Mix

Per-tick adaptive crossfade per match phase + Steam Audio per DR-043 spatial + FMOD adaptive layering OR bevy_kira_audio.

| Tool | Detail |
|---|---|
| FMOD Studio (free <$200K/yr) wrapped via `bevy_fmod` | Adaptive parameter automation; per-event signature |
| bevy_kira_audio (Apache-2.0) | Pure-Rust fallback; lower-feature |
| Steam Audio (Apache-2.0) | Spatial 3D + occlusion + reverb per [[spec/comms-voice-and-radio-model]] |

## Real-Time Procedural Audio

### Combat impact (per-projectile per-impulse)

```rust
fn impact_audio(impulse: f32, weapon: &Weapon, surface: &Surface) -> Audio {
    let sub_bass_volume = (impulse - 8.0).max(0.0).clamp(0.0, 1.0);
    let mid_volume = (impulse / 20.0).clamp(0.5, 1.0);
    let high_freq_click = surface.material_hardness;
    
    Audio::layered(vec![
        SubBassThump { volume: sub_bass_volume, frequency: 60.0 },
        MidImpact { volume: mid_volume, signature: weapon.signature },
        HighClick { volume: high_freq_click, sample: surface.click_sample },
    ])
}
```

### Doppler shift on projectiles

Per Steam Audio doppler:
```
delta_velocity_to_listener = (v_projectile - v_listener) · direction_unit
pitch_shift_factor = 1.0 - delta_velocity / sound_speed
audio.pitch *= pitch_shift_factor
```

### Atmospheric absorption

Per `cf-atmos`:
```
audio_high_freq_attenuation = atmos.density / atmos.reference_density
audio.eq_high.gain -= 6.0 * (1.0 - audio_high_freq_attenuation)  // dB attenuation
if vacuum: audio.volume = 0.0  // unless sealed-helmet
```

### Footstep variety

Per surface material (sand/concrete/metal/ice/blood/oil/water/grass): 5-10 variants per material × per-chassis-mass.

```rust
fn footstep_audio(actor: &Actor, surface_material: &Material) -> Audio {
    let variant_idx = rng.gen_range(0..5);
    let sample = footstep_sample(surface_material.id, variant_idx);
    let volume = actor.mass.scaled_volume();
    let pitch = 1.0 + actor.mass.pitch_offset();
    Audio::oneshot(sample, volume, pitch)
}
```

### Ricochet

Procedural: pitch + amplitude per impact angle + material hardness.

### Ambient mix per EnvironmentSignal

```rust
fn ambient_mix(env: &EnvironmentSignal) -> AmbientMix {
    AmbientMix {
        wind: env.weather.wind_mps / 50.0,  // normalize 0-1
        rain: env.weather.precipitation,
        thermal_shimmer: env.weather.intensity,
        atmosphere_density: env.atmosphere_kPa / 100.0,
        vacuum_cut: env.atmosphere_kPa < 0.1,
    }
}
```

### Reverb per room

Steam Audio raytraced reverb per `cf-atmos` room volume + materials.

### Voice phoneme lip-sync

Per dialogue line: extract phonemes via Praat or auto-lip-sync model; emit `mouth_phoneme_a/e/i/o/u` animation tags per frame; runtime visualization on actor face mesh.

### Music adaptive layering per intensity

```rust
fn adaptive_music_mix(combat_intensity: f32) -> StemMix {
    let drums = (combat_intensity * 1.5).clamp(0.0, 1.0);
    let bass = (combat_intensity * 1.2).clamp(0.0, 1.0);
    let melody = (combat_intensity * 0.8).clamp(0.0, 1.0);
    let tension = (combat_intensity - 0.4).clamp(0.0, 1.0);
    StemMix { drums, bass, melody, tension }
}
```

FMOD parameter automation: per-second update of `combat_intensity_score` → crossfade between stems.

### EMP / weapon signature procedural

Per-EMP discharge: synthesized waveform via `bevy_fundsp` (Rust DSP); not pre-baked; deterministic per seed.

### Caption generation

Per-SFX prompt → LLM-generated short caption; cached; localized per [[spec/localization-plan]].

## File Structure

```
content/audio/
├── prompts/
│   ├── sfx/
│   ├── music/
│   └── voice/
├── stems/
│   └── world_mars_ambient/
├── voice_models/
│   └── coalition_marshal/
│       └── reference_6sec.wav
├── voice_dialogue/
│   └── coalition_marshal_male/
└── manifest.json
```

## AI Orchestrator (`tools/audio_pipeline/`)

Python orchestrator. Talks to Stable Audio + MusicGen + XTTS + FMOD/Kira.

```bash
$ cf-audio-pipeline regen --tier 1 --asset weapon_ak47_fire
$ cf-audio-pipeline regen --tier 4 --npc coalition_marshal --dialogue all
$ cf-audio-pipeline regen --mod my-faction-mod --tier 1
```

## Modder Parity

Same tools available to modders.

## Performance Budget

Per DR-054.

| Tier | Voices | Spatial channels |
|---|---|---|
| Steam Deck | ≤32 simultaneous | ≤8 channels |
| Mid-range | ≤96 | ≤32 |
| High-end | ≤256 | ≤128 |

Voice budget governor: drop oldest non-critical first.

## Caption Coverage

100% on critical audio per DR-020. CI gate `cf-caption-check`.

## Replay / Determinism

- Pre-baked SFX: triggered by replay events; deterministic order; cosmetic flag for non-gameplay-critical.
- Music: cosmetic flag.
- Voice / dialogue: triggered by mission director events; deterministic per event chain.
- Runtime procedural: cosmetic flag.

## Done-Criteria

- [ ] All 400+ SFX clips generated + tagged + caption-bound.
- [ ] 30+ music tracks composed + stems exported.
- [ ] 24+ NPCs voice-cloned + 50-100 dialogue lines each.
- [ ] Runtime adaptive layering responds to gameplay.
- [ ] All audio caption-coverage 100% on critical.
- [ ] usage-ledger covers every generated asset.
- [ ] `cf-asset-ledger check --mode private` passes for retained private prototype audio; `--mode release` is reserved for public sale/release cleanup.
- [ ] FMOD or Kira mix passes diegetic-first per DR-020.
- [ ] Modder parity verified.

## Source Trail

- [[decisions/dr-053-ai-audio-pipeline-realtime-and-generative]]
- [[decisions/dr-057-optional-gacha-battle-pass-and-private-prototype-license-posture]]
- Stable Audio Open overview: https://stability.ai/news-updates/introducing-stable-audio-open
- Stable Audio Open model card/license: https://huggingface.co/stabilityai/stable-audio-open-1.0
- Suno terms: https://suno.com/terms
- Udio terms: https://www.udio.com/terms-of-service
- MusicGen / AudioCraft code + weight-license split: https://github.com/facebookresearch/audiocraft
- Coqui XTTS-v2: https://huggingface.co/coqui/XTTS-v2
- Tortoise-TTS: https://github.com/neonbjb/tortoise-tts
- ElevenLabs Music terms: https://elevenlabs.io/eleven-music-v1-terms
- FMOD Studio: https://www.fmod.com/
- bevy_kira_audio: https://crates.io/crates/bevy_kira_audio
- bevy_fundsp: Rust DSP library
- Steam Audio: https://valvesoftware.github.io/steam-audio/
- AudioCraft Demucs: https://github.com/facebookresearch/audiocraft
