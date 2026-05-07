---
type: decision
id: DR-053
status: closed-direction
priority: P0
closed_at: 2026-05-06
revisit_trigger: "AI audio quality fails playtest cohort; runtime latency exceeds budget; voice-clone licensing changes; FMOD or bevy_kira_audio path proves inadequate; modder authoring breaks parity."
---

← [[decisions/index|decision records]] · [[dashboards/decision-tracker|decision tracker]] · [[spec/ai-audio-pipeline-realtime-and-generative|AI audio pipeline spec]] · [[spec/music-and-soundtrack|music/soundtrack]] · [[spec/audio-identity|audio identity]] · [[spec/comms-voice-and-radio-model|voice/radio]] · [[decisions/dr-020-audio-identity|DR-020]] · [[decisions/dr-044-audiovisual-production-pipeline|DR-044]]

# DR-053: AI Audio Pipeline — Real-Time + Generative (No Humans Crafting)

> [!success] Status: CLOSED-DIRECTION (project owner committed 2026-05-06)
> Complete AI-driven audio pipeline. **NO humans crafting any audio.** 5 production tiers: (1) Stable Audio Open 1.0 (Apache-2.0 local) for SFX library generation, (2) Suno v5 / Udio v2 cloud for hero music tracks, (3) MusicGen-Medium (Meta MIT) local for ambient music, (4) XTTS-v2 / Coqui TTS local + ElevenLabs cloud for voice, (5) FMOD Studio / bevy_kira_audio runtime adaptive layering + Steam Audio spatial. **Plus runtime generation**: real-time procedural audio for combat impact + atmospheric ambience based on EnvironmentSignal + per-actor voice synthesis with phoneme lip-sync. AI agents produce + tag + caption + license-log everything.

## Decision

### 5-tier production pipeline

| Tier | Use case | Tools | License |
|---|---|---|---|
| **Tier 1 — SFX library generation** | 400+ launch SFX clips: weapons, footsteps, equipment, environment, UI, combat, voice barks. | Stable Audio Open 1.0 (local; 32GB VRAM); per-event prompt template + deterministic seed; auto-trim/normalize/EQ via `librosa` + `numpy`; auto-caption from prompt. | Apache-2.0 (commercial-friendly). |
| **Tier 2 — Hero music tracks** | Main theme + 12 world themes + 6 combat layers + 4 base-tension + 4 menu/UI + 8 mission stings + 3 antagonist motifs. | Suno v5 cloud OR Udio v2 cloud. Per-track prompt + seed. Stem export (drums/bass/melody/tension via Demucs). | Suno commercial-use OK 2026; license review pre-launch. |
| **Tier 3 — Ambient music + procedural** | Long-loop ambient, world-specific drones, planet atmospherics. | MusicGen-Medium (Meta MIT) local. Lower-quality than Suno; no licensing risk. | MIT. |
| **Tier 4 — Voice / NPC dialogue** | Per-faction commander voice + ~24 named NPCs + 50-100 dialogue lines per NPC. | XTTS-v2 / Coqui TTS local (free, ~6s reference clip → cloned voice in 16 languages); ElevenLabs cloud for hero NPCs; Tortoise-TTS local for high-quality offline. | XTTS / Coqui: CPML; ElevenLabs: review TOS pre-launch; Tortoise: Apache-2.0. |
| **Tier 5 — Runtime adaptive + spatial mix** | Per-tick adaptive crossfade per match phase + Steam Audio per DR-043 spatial + FMOD Studio adaptive layering OR bevy_kira_audio + custom Rust mixing. | FMOD Studio (free under $200K/yr revenue) wrapped via `bevy_fmod`, OR bevy_kira_audio (Apache-2.0). | FMOD: free under threshold; Kira: Apache-2.0. |

### Real-time procedural audio (runtime generation)

Beyond pre-baked SFX library, certain audio is generated AT RUNTIME for variety + reactivity:

| Audio Type | Runtime Method |
|---|---|
| **Combat impact (per-projectile per-impulse)** | Per-impulse magnitude → SFX mix: sub-bass (impulse > 8kg·m/s) + mid-range hit (always) + high-frequency click (per material). Per-weapon signature via FMOD parameter automation. |
| **Doppler shift on projectiles** | Steam Audio doppler effect on projectile passage past listener; pitch shift per Δvelocity. |
| **Atmospheric absorption** | Per-`cf-atmos` density: high pressure = brighter audio, vacuum = silent (sealed-helmet exception). Frequency-domain filter per atmospherics state. |
| **Footstep variety** | Per-surface material (sand/concrete/metal/ice/blood/oil/water/grass): footstep SFX selected from 5-10 variants per material + per-chassis-mass. Foot-anchor frame from animation tag. |
| **Ricochet** | Procedural pitch + amplitude per impact angle + material hardness. |
| **Ambient mix per EnvironmentSignal** | Per-tick mix: wind volume scales with `weather.wind_mps`; rain volume per `weather.precipitation`; thermal-shimmer audio per `weather.intensity`; vacuum cuts ambient. |
| **Reverb per room** | Steam Audio raytraced reverb per `cf-atmos` room volume + materials. |
| **Voice phoneme lip-sync** | Generated via XTTS / Coqui per dialogue line; mouth_phoneme animation tags per [[spec/animation-system]]; runtime visualization on actor face. |
| **Music adaptive layering per intensity** | FMOD parameter automation: per-second update of combat_intensity_score → crossfade between drums/bass/melody/tension layers. Real-time mix. |
| **EMP / weapon signature procedural** | Per-EMP discharge: synthesized waveform via `bevy_fundsp` (Rust DSP); not pre-baked; deterministic per seed. |
| **Caption generation** | Per-SFX prompt → LLM-generated short caption (per DR-020 mandatory captions); cached; localized. |

### Hardware floor

- 32GB VRAM (RTX 4090-class) per project owner.
- Tier 1 (Stable Audio Open) runs locally.
- Tier 4 (XTTS-v2) runs locally; ~3GB VRAM minimum.
- Tier 5 runtime uses CPU-bound DSP; no VRAM dependency.
- Modders can run Tier 1-4 with ≥12GB VRAM (lower quality but functional).

### File structure

```
content/audio/
├── prompts/                     # AI generation prompts (deterministic seeds)
│   ├── sfx/
│   │   ├── weapon_ak47_fire.prompt
│   │   ├── footstep_sand_human.prompt
│   │   └── ...
│   ├── music/
│   │   └── ...
│   ├── voice/
│   │   ├── coalition_marshal_male.prompt
│   │   └── ...
├── stems/                       # Music stems (drums/bass/melody/tension/percussion/ambient)
│   ├── world_mars_ambient/
│   │   ├── drums.ogg
│   │   ├── bass.ogg
│   │   ├── melody.ogg
│   │   ├── tension.ogg
│   │   └── ...
├── voice_models/                # Per-NPC voice clone reference samples
│   ├── coalition_marshal/
│   │   └── reference_6sec.wav
│   └── ...
├── voice_dialogue/              # Generated dialogue lines
│   ├── coalition_marshal_male/
│   │   ├── briefing_001.ogg
│   │   ├── briefing_002.ogg
│   │   └── ...
└── manifest.json                # All generated audio + provenance + license
```

### AI orchestrator

`tools/audio_pipeline/` (Python orchestrator; talks to Stable Audio + MusicGen + XTTS + FMOD/Kira):

```bash
$ cf-audio-pipeline regen --tier 1 --asset weapon_ak47_fire
[INFO] Loading prompt: content/audio/prompts/sfx/weapon_ak47_fire.prompt
[INFO] Generating with Stable Audio Open 1.0 (seed: 12345)
[INFO] Output: assets/tier3/sfx/weapon_ak47_fire.ogg
[INFO] Auto-tagging: weapon, kinetic, AK-47, gunshot
[INFO] Caption generated: "AK-47 gunfire"
[INFO] License logged: usage-ledger.md
[INFO] Spatial flag: true; falloff_radius_m: 100
[INFO] Done in 4.2s
```

### Modder parity

Modders use the SAME tools:

```bash
$ cf-audio-pipeline regen --mod my-faction-mod --tier 1
[INFO] Discovered 12 audio prompts in mod
[INFO] Regenerating...
[INFO] Done in 47s
```

### Localization

| Aspect | Detail |
|---|---|
| Voice translation | XTTS-v2 supports 16 languages out of the box; AI translation per [[spec/localization-plan]] then re-synthesize per locale. |
| Per-locale voice models | Tier-A languages get full voice sets; Tier-B languages use cross-lingual XTTS clone of base reference. |
| Captions per locale | Per [[spec/localization-plan]] Project Fluent. |

### Performance budget

| Tier | Target |
|---|---|
| Steam Deck floor | Adaptive mix at 60Hz; ≤32 simultaneous voices; ≤8 spatial channels (Steam Audio). |
| Mid-range | ≤96 simultaneous voices; ≤32 spatial channels. |
| High-end | ≤256 simultaneous voices; ≤128 spatial channels. |

Voice budget governor:
- Drop oldest non-critical voice first under pressure.
- Spatial channel cap before raw voice cap.
- Reported in `summary.json.perf.audio_voice_drop_count`.

### Caption coverage (per DR-020)

100% on critical audio. CI gate `cf-caption-check` validates.

### Replay / determinism

| Aspect | Detail |
|---|---|
| Pre-baked SFX | Triggered by replay events; deterministic order; cosmetic flag for non-gameplay-critical. |
| Music | Cosmetic flag; not in determinism island. |
| Voice / dialogue | Triggered by mission director events; deterministic per event chain. |
| Runtime procedural | Cosmetic flag (e.g., footstep variety; doppler shift). Cause-chain to deterministic events (e.g., footstep_left animation tag). |

## What This Locks In

| Spec Area | Implication |
|---|---|
| `cf-audio-pipeline` | Python orchestrator + Stable Audio + MusicGen + XTTS adapters. |
| `cf-audio-runtime` | Bevy runtime mixer + FMOD/Kira adapter + Steam Audio integration. |
| `cf-audio-procedural` | Real-time DSP for impact + doppler + atmospheric absorption + footstep variety. |
| `cf-audio-adaptive` | FMOD parameter automation OR Kira crossfade for music layering. |
| `content/audio/` | All audio under one roof; prompts + stems + voice models + dialogue + manifest. |
| `references/usage-ledger.md` | Every generated audio asset logged: prompt + seed + model + license + regenerable Y. |
| Modders | Run same `cf-audio-pipeline` for their mods. |

## What This Does NOT Lock

| Non-Commitment | Why |
|---|---|
| Suno vs Udio for music | Open. Default Suno v5; pivot to Udio if licensing changes. |
| ElevenLabs vs Coqui for hero NPCs | Open. ElevenLabs higher quality; Coqui free. Default ElevenLabs if license clears, Coqui fallback. |
| FMOD vs Kira for runtime | Open. Default FMOD if revenue model fits; Kira if revenue threshold approached. |
| Real-time AI music generation (post-launch) | Open. Default pre-baked stems with adaptive crossfade; could add real-time ML music post-launch. |
| Cross-platform voice synthesis on Switch | Open. Switch console eval; XTTS may not run on Switch CPU; pre-bake all voices for Switch. |

## Why This Direction

| Driver | Detail |
|---|---|
| AI-augmented solo dev | Per DR-026; humans crafting audio = unsustainable. AI pipeline produces 400+ SFX + 30+ tracks + ~2400 voice lines (24 NPCs × 100 lines) without human bottleneck. |
| 32GB VRAM available | Per project owner; Stable Audio Open + XTTS + MusicGen all run locally. No cloud lock-in for SFX/voice. |
| License safety | All open-weight models commercially usable; logged in usage-ledger; verified pre-launch. |
| Determinism + regenerability | Same prompt + seed + model = same audio. Reproducible; modders can extend. |
| Real-time variety | Pre-baked SFX get repetitive; runtime procedural variety (footsteps, doppler, atmospheric absorption) keeps audio fresh. |
| Tactical readability per DR-020 | Diegetic-first; caption coverage 100%; spatial audio per Steam Audio + DR-043. |

## Why Not The Alternatives

| Alternative | Why Rejected |
|---|---|
| Hire a sound designer | AI-augmented solo dev; multi-year stall risk; doesn't scale. |
| Hire voice actors | Cost-prohibitive; AI voice clone ~free post-training; modder parity. |
| Cloud-only audio (no local) | License + cost + latency + offline-play risk. Local-first matches DR-013 backend posture. |
| Pure Wwise vs FMOD | Wwise also viable; FMOD has better Rust ecosystem; revisit if needed. |
| Pure pre-baked (no runtime procedural) | Loses tactical variety; impact + doppler + atmospheric absorption are core feel. |

## Evidence Trail

- Project owner verbatim (2026-05-06): "Figure out the best way to implement an AI audio pipeline for the game - nothing will be crafted by humans."
- Stable Audio Open 1.0 (Apache-2.0): https://stability.ai/news-updates/introducing-stable-audio-open
- Stable Audio 2.5 (cloud): https://stability.ai/stable-audio
- Suno v5 commercial: https://suno.com/ (review TOS pre-launch)
- Udio v2: https://www.udio.com/
- MusicGen (Meta MIT): https://github.com/facebookresearch/audiocraft
- Coqui XTTS-v2: https://huggingface.co/coqui/XTTS-v2 (16 languages; 6s voice clone)
- Tortoise-TTS (Apache-2.0): https://github.com/neonbjb/tortoise-tts
- ElevenLabs: https://elevenlabs.io/ (license review)
- FMOD Studio: https://www.fmod.com/ (free <$200K/yr)
- bevy_kira_audio: https://crates.io/crates/bevy_kira_audio
- bevy_fundsp: Rust DSP library
- Steam Audio: https://valvesoftware.github.io/steam-audio/ (Apache-2.0)
- AudioCraft Demucs (stem separation): https://github.com/facebookresearch/audiocraft
- Captured in [[research-log/2026-05-06-third-pass-audit-followup]] (TBD).

## Revisit Trigger

- AI audio quality fails playtest cohort.
- Runtime latency exceeds budget.
- Voice-clone licensing changes.
- FMOD or bevy_kira_audio path proves inadequate.
- Modder authoring breaks parity.
- Per-platform deployment breaks (e.g., Switch can't run XTTS).
