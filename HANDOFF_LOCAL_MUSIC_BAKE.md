# Handoff — Local Music Bake on RTX 5090 (ACE-Step v1.5)

**Audience**: A second AI coding agent who has been hired to handle ONLY this local music-bake job on the product owner's RTX 5090 (32GB VRAM, Blackwell, CUDA 12.8+). You do not have access to the Corefall repo source. You do not need to. Everything you need is in this document, including all 83 music prompts. Your deliverable is a folder of 83 named `.wav` files.

This is a self-contained workpacket. You write the inference scripts. You drive ACE-Step. You hand back 83 WAVs. Done.

---

## TL;DR

1. **Product owner's environment**: RTX 5090 with **32 GB GDDR7 VRAM** (Blackwell), CUDA 12.8+, plenty of system RAM, Linux or Windows-WSL2 host.
2. **Model**: **ACE-Step v1.5 (3.5B parameters)** — Apache 2.0 license, open weights, generates full songs up to ~4 minutes natively, was released **2026-01-28** and is currently the strongest open-weights music model for consumer hardware ("outperforms almost all commercial alternatives" per their paper).
3. **Quality > speed**. Product owner is fine with a 4-8 hour unattended overnight bake. Use highest-quality settings, lots of denoise steps, fp32 weights if the model offers it, otherwise bf16. Do NOT use the half-precision speed mode.
4. **Output**: 83 instrumental loopable stereo WAVs (16-bit PCM 48 kHz or 44.1 kHz — pipeline will resample). 60s–300s per track per Appendix A.
5. **Filename convention**: `<track_id>_<variant>.wav`, lowercase, no spaces. (e.g. `music_world_phobos_calm.wav`).
6. **Hand-off folder**: drop the 83 WAVs into `corefall_ace_step_music_bake_v1/`, zip it, send to the product owner.

---

## Section 1 — Why we're doing this

Corefall is a 2D Cortex-Command-style fortress builder + crawler hybrid. It ships with an adaptive music system (M37A milestone) keyed off `intensity` floats from combat density / mission phase / boss phase. Each scenario state cross-fades between four variants of one base track: `calm` (intensity 0.0-0.3), `buildup` (0.3-0.6), `climax` (0.6-1.0), `debrief` (post-encounter).

The product owner already baked 39 of these tracks via the ElevenLabs Music API and ran out of API credits before finishing. The remaining **83 tracks** need to be filled in. ElevenLabs credits won't reset for ~5 months. Closed-source alternatives (Suno, Udio, AIVA, Lyria) are off the table for licensing + cost + reproducibility reasons. **Local generation on the product owner's RTX 5090 is the path.**

---

## Section 2 — Primary model: ACE-Step v1.5

| | |
|---|---|
| **Authors** | ACE Studio + StepFun |
| **Released** | 2026-01-28 (v1.5); v1 released earlier in 2025 |
| **Parameters** | 3.5B |
| **Architecture** | Diffusion-based music generation with transformer conditioning |
| **License** | **Apache 2.0** (commercial use approved — perfect for a shipping game) |
| **Native duration** | Up to ~4 minutes (240 s) per generation; supports longer with chunking |
| **Output format** | Stereo, 44.1 kHz natively (resample to 48 kHz at post-process) |
| **VRAM (fp16)** | ~8-12 GB (3.5B params + activations) — fits comfortably in 32 GB |
| **VRAM (bf16)** | ~10-14 GB |
| **VRAM (fp32 if exposed)** | ~16-22 GB — try this on the 32 GB card for highest quality |
| **GitHub** | https://github.com/ace-step/ACE-Step + https://github.com/ace-step/ACE-Step-1.5 |
| **HuggingFace primary** | https://huggingface.co/ACE-Step/Ace-Step1.5 |
| **HuggingFace v1 3.5B** | https://huggingface.co/ACE-Step/ACE-Step-v1-3.5B |
| **Paper** | https://huggingface.co/papers/2602.00744 ("Pushing the Boundaries of Open-Source Music Generation") |

### Fallback model: Stable Audio Open 1.0

Use **only** if ACE-Step fails to install / has a regression / produces obviously bad output on a specific prompt.

| | |
|---|---|
| **HuggingFace** | https://huggingface.co/stabilityai/stable-audio-open-1.0 |
| **License** | Stability AI Community License — commercial use allowed but verify the latest terms before shipping |
| **Native duration** | Up to 47 s; needs sliding-window chunking + crossfade for our 60-300 s tracks |
| **Quality** | Strong on textures / ambient / cinematic; weaker on full-arrangement orchestral than ACE-Step |
| **VRAM** | ~6-8 GB fp16; trivial on a 5090 |

### Why NOT MusicGen-large / YuE / DiffRhythm / MAGNeT

- **MusicGen-large** (Meta, 3.3B): non-commercial license per AudioCraft terms — fails the game-shipping bar.
- **YuE 7B** (CUHK): strong but vocal-focused; we want **instrumental only**; YuE struggles to suppress vocals.
- **DiffRhythm 2**: vocal-song focused (text→lyrics→song); not optimized for instrumental game loops.
- **MAGNeT**: faster but lower quality than MusicGen; same non-commercial license.
- **AudioLDM2** (Liu et al.): older 2024 model, surpassed by ACE-Step on every benchmark.

---

## Section 3 — Environment + tooling

Recommended setup on the product owner's RTX 5090 host:

```bash
# Linux or WSL2 host
mkdir -p ~/corefall_local_music && cd ~/corefall_local_music

# Python 3.11+ (3.12 recommended; 3.13 also fine)
python3 -m venv .venv
source .venv/bin/activate

# CUDA 12.8 PyTorch nightly for Blackwell support (5090 needs cu128+)
pip install --pre torch torchvision torchaudio --index-url https://download.pytorch.org/whl/nightly/cu128

# Verify CUDA + 5090 detection
python -c "import torch; print('cuda:', torch.cuda.is_available()); print('dev:', torch.cuda.get_device_name(0)); print('vram:', torch.cuda.get_device_properties(0).total_memory / 1024**3, 'GB')"
# Expected: cuda: True   dev: NVIDIA GeForce RTX 5090   vram: ~31.5 GB

# ACE-Step install (clone + pip install per their README)
git clone https://github.com/ace-step/ACE-Step.git
cd ACE-Step
pip install -e .
# Verify model can be pulled from HF
python -c "from huggingface_hub import snapshot_download; snapshot_download('ACE-Step/Ace-Step1.5')"

# Plus the audio post-processing deps shared with the Corefall pipeline
pip install soundfile numpy scipy librosa pydub
```

If ACE-Step's README points to a different installation procedure (their repo evolves quickly), follow that — the above is the May-2026 baseline.

---

## Section 4 — Generation pipeline

The 83 prompts in Appendix A each carry:
- `prompt` — the full musicgen-style text prompt (already crafted for cinematic / orchestral / sci-fi / cyberpunk)
- `seed` — a stable integer seed (use it; this gives the product owner reproducible bakes if they ever need to regen one track)
- `duration_seconds` — 180 to 300 s
- `tempo_bpm` and `key` — feed into the prompt if your ACE-Step version exposes those fields directly

### Per-track loop (pseudocode)

```python
import json, time, random
from pathlib import Path
from acestep import AceStepPipeline   # the canonical ACE-Step inference class

OUT_DIR = Path("corefall_ace_step_music_bake_v1")
OUT_DIR.mkdir(exist_ok=True)
prompts = json.loads(Path("prompts.json").read_text())

# Load model with highest-quality settings (bf16 if fp32 not exposed)
pipe = AceStepPipeline.from_pretrained(
    "ACE-Step/Ace-Step1.5",
    torch_dtype="bf16",          # try "fp32" first if 32 GB VRAM tolerates it
    device="cuda:0",
)

# Generate every prompt, deterministic via seed
for i, p in enumerate(prompts, start=1):
    out = OUT_DIR / f"{p['file_id']}.wav"
    if out.exists() and out.stat().st_size > 100_000:
        print(f"[skip] {p['file_id']} (exists)")
        continue

    print(f"[bake {i}/{len(prompts)}] {p['file_id']}  dur={p['duration_seconds']}s  seed={p['seed']}")
    audio = pipe.generate(
        prompt=p['prompt'],
        duration_seconds=p['duration_seconds'],
        seed=p['seed'],
        force_instrumental=True,       # if exposed; otherwise add "instrumental, no vocals" to prompt
        sample_rate=44100,             # native; pipeline resamples to 48k at post-process
        # Quality knobs — push them to the high end since we have time:
        num_inference_steps=150,       # default is often 50; 150 doubles quality at 3x time cost
        guidance_scale=7.5,            # CFG-style; 7-10 sweet spot
        scheduler="dpm_solver_v2",     # if the build exposes scheduler choice
    )
    # save_audio either writes WAV directly or returns bytes
    pipe.save_audio(audio, str(out), sample_rate=44100, channels=2)
    # Brief settle to let GPU memory free
    time.sleep(2.0)

print(f"done: {len(prompts)} attempted")
```

If ACE-Step's API surface differs from the above, follow their README; the important contract is: **prompt + seed + duration + force-instrumental** in, **stereo WAV** out.

### Extending native cap (only needed if ACE-Step v1.5 caps short of our 240-300 s targets)

If the model maxes out at e.g. 180 s but a track needs 240 s, generate two overlapping segments and crossfade them:

```python
def crossfade_extend(seg_a_samples: np.ndarray, seg_b_samples: np.ndarray, overlap_sec: float = 4.0, sr: int = 44100) -> np.ndarray:
    overlap_n = int(overlap_sec * sr)
    fade = np.linspace(0.0, 1.0, overlap_n).reshape(-1, 1)  # stereo broadcast
    a_tail = seg_a_samples[-overlap_n:] * (1.0 - fade)
    b_head = seg_b_samples[:overlap_n] * fade
    mixed_overlap = a_tail + b_head
    return np.concatenate([seg_a_samples[:-overlap_n], mixed_overlap, seg_b_samples[overlap_n:]], axis=0)
```

Pass slightly different `seed` to the second segment (e.g. `seed + 100`) so the two halves don't repeat identically.

### Post-process pass (REQUIRED before delivery)

Every shipped WAV must pass this cleanup so the downstream Corefall engine can loop it cleanly:

```python
import soundfile as sf, numpy as np
from pathlib import Path

def cleanup(path: Path, target_peak_dbfs: float = -8.0, loop_crossfade_ms: float = 50.0, fade_ms: float = 5.0):
    data, sr = sf.read(str(path), always_2d=True, dtype="float64")

    # 1. trim leading/trailing silence below -60 dBFS
    floor = 10 ** (-60 / 20)
    mono = data.mean(axis=1)
    above = np.where(np.abs(mono) > floor)[0]
    if above.size:
        data = data[int(above[0]):int(above[-1])+1]

    # 2. loop-align (crossfade head+tail so the boundary is seamless)
    fade_n = min(int(loop_crossfade_ms * sr / 1000.0), data.shape[0] // 4)
    if fade_n > 1:
        blend = np.linspace(0.0, 1.0, fade_n).reshape(-1, 1)
        head = data[:fade_n].copy()
        tail = data[-fade_n:].copy()
        cross = tail * (1 - blend) + head * blend
        data[:fade_n]  = cross
        data[-fade_n:] = cross

    # 3. micro fade-in/out (5 ms each side) to suppress click pops
    fn = min(int(fade_ms * sr / 1000.0), data.shape[0] // 2)
    if fn > 1:
        ramp = np.linspace(0.0, 1.0, fn).reshape(-1, 1)
        data[:fn]  *= ramp
        data[-fn:] *= ramp[::-1]

    # 4. normalize peak to -8 dBFS (target for music bus)
    target = 10 ** (target_peak_dbfs / 20)
    peak = float(np.max(np.abs(data)) + 1e-12)
    if peak > 1e-7:
        data = data * (target / peak)
    data = np.clip(data, -1.0, 1.0)

    # 5. write back as 16-bit PCM at native sample rate (pipeline resamples downstream)
    pcm = (data * 32767.0).astype(np.int16)
    sf.write(str(path), pcm, sr, subtype="PCM_16", format="WAV")
```

Run cleanup on every WAV in `corefall_ace_step_music_bake_v1/` before zipping the deliverable.

---

## Section 5 — Filename convention (EXACT)

```
music_world_<world_id>_<variant>.wav      # 56 files (14 of the 81 are world)
music_faction_<faction_id>_<variant>.wav  # 12 files
music_storyteller_<story_id>_<variant>.wav # 8 files
music_boss_<boss_id>_<variant>.wav        # 7 files (1 boss already partially done)
```

…where `variant ∈ {calm, buildup, climax, debrief}`. Every file_id is enumerated in Appendix A.

Validation regex: `^music_(world|faction|storyteller|boss)_[a-z_]+_(calm|buildup|climax|debrief)\.wav$`

---

## Section 6 — Final validation before handoff

After running all 83 prompts + the post-process pass, run:

```bash
cd corefall_ace_step_music_bake_v1
total=$(ls *.wav | wc -l)
echo "total files: $total  (expect 83)"

# every file must parse as a real WAV
for f in *.wav; do
  file "$f" | grep -q "WAVE audio" || echo "BAD: $f"
done

# every file must be at least 30 seconds (filter obvious truncation)
for f in *.wav; do
  dur=$(python -c "import soundfile as sf; d=sf.info('$f'); print(d.duration)")
  python -c "import sys; sys.exit(0 if float('$dur') >= 30 else 1)" || echo "SHORT: $f ($dur s)"
done

# every file must be at least 1 MB
for f in *.wav; do
  size=$(stat -c%s "$f" 2>/dev/null || stat -f%z "$f")
  [ "$size" -lt 1000000 ] && echo "TINY: $f ($size B)"
done
```

If anything fails, regenerate that single file with a different `seed` and rerun.

---

## Section 7 — Hand-off

1. ZIP `corefall_ace_step_music_bake_v1/`:
   ```bash
   cd $(dirname corefall_ace_step_music_bake_v1)
   zip -r corefall_ace_step_music_bake_v1.zip corefall_ace_step_music_bake_v1/
   ```
2. Include a one-line per file `bake_report.txt` inside the zip:
   ```
   ACE-Step v1.5 local bake — corefall_ace_step_music_bake_v1
   Completed: <n>/83 tracks
   Failed:    <list of failed file_ids, if any>
   Total wall time: <hh:mm>
   Host: RTX 5090, CUDA 12.8, ACE-Step v1.5 from HuggingFace
   Settings: dtype=bf16, num_inference_steps=150, guidance_scale=7.5, scheduler=dpm_solver_v2
   Notes: <any tracks that needed seed-resampling or special handling>
   ```
3. Send the zip to the product owner via whichever channel they specify at hand-off time.

---

## Section 8 — Constraints (don't violate)

- **Apache 2.0 only**: do not silently substitute a non-commercial model (MusicGen-large, MAGNeT, YuE). If ACE-Step v1.5 fails, use Stable Audio Open 1.0 (Stability AI Community License, commercial OK with attribution). Anything else → escalate, don't substitute.
- **Instrumental only**: every track must be vocal-free. ACE-Step usually obeys `force_instrumental=True` but spot-check 5 random outputs; if any have vocals, reroll with seed+1.
- **No public posting** of generated music without product owner approval.
- **Seed reproducibility**: the seeds in Appendix A are stable — use them. If you have to regenerate a file, document the new seed you chose in `bake_report.txt`.
- **Filename exactly per Appendix A**: do not rename or restructure files.

---

## Section 9 — Pacing + reliability

- ACE-Step on a 5090 typically generates **~1 second of audio in ~0.4-1.2 seconds of wall time** at high quality settings (depending on inference_steps + guidance_scale). For 83 tracks averaging 220 s each, expect **5-12 hours unattended**. Plan an overnight run.
- Save progress incrementally: write a `progress.json` `{"completed": [...], "failed": [...]}` so you can resume after a crash (the loop above implicitly does this via `out.exists()`).
- If the GPU OOMs at any point, drop `num_inference_steps` to 100 and retry just that track; do not lower quality globally.
- Monitor `nvidia-smi -l 5` for the first 10 minutes to confirm VRAM usage stays under ~28 GB; if it creeps higher (memory leak between iterations), restart the script every ~20 tracks.

---

## Appendix A — The 83 tracks (FULL PROMPTS, embed as `prompts.json`)

Schema for `prompts.json`:

```json
[
  {
    "track_id":          "music_world_phobos",
    "variant":           "calm",
    "file_id":           "music_world_phobos_calm",
    "canonical_name":    "Phobos Microgravity Asteroid",
    "duration_seconds":  240,
    "tempo_bpm":         70,
    "key":               "G minor",
    "prompt":            "Phobos microgravity asteroid ambient, eerie silence with floating debris + creaking structural metal + faint synth drone in G minor + sparse chime, 70 BPM, no drums, weightless dread",
    "seed":              140001,
    "group":             "world"
  },
  ...
]
```

The full 83-row JSON is committed in the source repo at `tools/audio_pipeline/HANDOFF_LOCAL_MUSIC_BAKE_prompts.json` (or you can copy them directly from the inline table below).

### A.1 — World ambient (these are the bulk: ~56 files)

(Each `track_id` × 4 variants. Where a track has 1-3 of its variants already baked at Tier 2, only the unfinished variants are listed here.)

Total in this appendix: **83 files** (world 11 + faction 32 + storyteller 20 + boss 20).

### World ambient (11 files)

World-ambient tracks underpin 12 unique world / biome experiences in Corefall. Each base track has 4 variants (calm / buildup / climax / debrief) that cross-fade based on combat intensity. The Tier 2 (clean) bakes already cover Earth, Mars, Moon, Mimas, Europa, Vulcan, Venus, Belt, Deimos, and several variants of Moon/Phobos. The variants still listed below are the ones that need a fresh bake.

#### `music_world_belt` — Belt Asteroid Mining

- Base parameters: **240 s** target duration, **92 BPM**, key **G# minor**
- Variants to bake here: debrief

**`music_world_belt_debrief.wav`** (variant: `debrief`, seed: `200004`)

> Belt debrief, slow synth pad in G# minor + drill winding down + distant rock clink + radio sign-off, 70 BPM, hard-won score

#### `music_world_deimos` — Deimos Mining Colony

- Base parameters: **240 s** target duration, **78 BPM**, key **B minor**
- Variants to bake here: buildup

**`music_world_deimos_buildup.wav`** (variant: `buildup`, seed: `150002`) **[Tier 2 bake came out broken; re-bake from scratch]**

> Deimos tension, mining drills accelerating + bass throb in B minor + percussion pulse + alarm warm-up, 92 BPM, ore vein collapse imminent

#### `music_world_orbital` — Orbital Station Interior

- Base parameters: **240 s** target duration, **90 BPM**, key **F major**
- Variants to bake here: calm, buildup, climax, debrief

**`music_world_orbital_calm.wav`** (variant: `calm`, seed: `210001`)

> Orbital station interior ambient, gentle air recycler hum + computer beeps + distant footsteps + synth pad in F major + faint elevator-jazz harmonics, 90 BPM, civilian-station comfort

**`music_world_orbital_buildup.wav`** (variant: `buildup`, seed: `210002`)

> Orbital station tension, klaxon priming + bass throb in F minor + computer alarms + bulkhead clank, 100 BPM, hull breach warning

**`music_world_orbital_climax.wav`** (variant: `climax`, seed: `210003`)

> Orbital station combat, frantic electronic in F minor + driving drums + alarm pulse + bulkhead slam + station-PA shouts, 125 BPM, zero-g station boarding

**`music_world_orbital_debrief.wav`** (variant: `debrief`, seed: `210004`)

> Orbital station debrief, mellow synth pad in F major + air recycler slow + computer chimes + medbay piano, 75 BPM, station recovery

#### `music_world_phobos` — Phobos Microgravity Asteroid

- Base parameters: **240 s** target duration, **70 BPM**, key **G minor**
- Variants to bake here: calm

**`music_world_phobos_calm.wav`** (variant: `calm`, seed: `140001`) **[Tier 2 bake came out broken; re-bake from scratch]**

> Phobos microgravity asteroid ambient, eerie silence with floating debris + creaking structural metal + faint synth drone in G minor + sparse chime, 70 BPM, no drums, weightless dread

#### `music_world_sol_zone` — Sol Zone Stellar Edge

- Base parameters: **240 s** target duration, **80 BPM**, key **Bb major**
- Variants to bake here: calm, buildup, climax, debrief

**`music_world_sol_zone_calm.wav`** (variant: `calm`, seed: `220001`)

> Sol-zone habitat near-star ambient, intense solar wind + crystalline resonance + radiation hum + synth pad in Bb major + cathedral organ + ethereal choir, 80 BPM, sacred stellar awe

**`music_world_sol_zone_buildup.wav`** (variant: `buildup`, seed: `220002`)

> Sol-zone tension, solar flare crescendo + bass swell in Bb major + radiation alarm + brass build, 95 BPM, flare incoming

**`music_world_sol_zone_climax.wav`** (variant: `climax`, seed: `220003`)

> Sol-zone combat, triumphant orchestral in Bb major + full choir + heavy brass + thundering drums + stellar-wind howl, 120 BPM, climactic stellar wrath

**`music_world_sol_zone_debrief.wav`** (variant: `debrief`, seed: `220004`)

> Sol-zone debrief, sustained organ in Bb major + soft choir + receding solar wind + distant chime, 65 BPM, sacred relief

### Faction theme (32 files)

Each of the 8 launch factions has its own theme that plays when faction-aligned missions are active. Variant `calm` underscores narrative + dialog; `buildup` plays during mission rising action; `climax` plays during faction combat; `debrief` plays at mission end (success or failure). Aesthetic notes per faction are inlined into the prompts themselves.

#### `music_faction_coalition` — Coalition Theme

- Base parameters: **180 s** target duration, **110 BPM**, key **C major**
- Variants to bake here: calm, buildup, climax, debrief

**`music_faction_coalition_calm.wav`** (variant: `calm`, seed: `310001`)

> Coalition faction theme calm, militaristic orchestral with strong brass + snare + heroic melody in C major + civilian-radio fanfare + disciplined humanity, 110 BPM, ordered hope

**`music_faction_coalition_buildup.wav`** (variant: `buildup`, seed: `310002`)

> Coalition buildup, brass swell + tactical snare in C major + bass pulse + radio command chatter, 118 BPM, mobilization

**`music_faction_coalition_climax.wav`** (variant: `climax`, seed: `310003`)

> Coalition combat, modern military electronic in E minor + electric guitar + heavy drums + brass + synth + sergeant shouts, 125 BPM, aggressive precision

**`music_faction_coalition_debrief.wav`** (variant: `debrief`, seed: `310004`)

> Coalition debrief, solemn brass in C major + slow snare roll + bugle taps + sustained pad, 70 BPM, honored fallen

#### `music_faction_collective` — Collective Theme

- Base parameters: **180 s** target duration, **95 BPM**, key **G minor**
- Variants to bake here: calm, buildup, climax, debrief

**`music_faction_collective_calm.wav`** (variant: `calm`, seed: `350001`)

> Collective faction theme calm, industrial proletarian electronic with metal-scrap percussion + bass + radio static + worker chants + synth pad in G minor, 95 BPM, gritty solidarity

**`music_faction_collective_buildup.wav`** (variant: `buildup`, seed: `350002`)

> Collective buildup, scrap-percussion accelerating + bass throb in G minor + worker drum + factory whistle, 108 BPM, strike forming

**`music_faction_collective_climax.wav`** (variant: `climax`, seed: `350003`)

> Collective combat, anarchic industrial in D minor + distorted bass + thrashing drums + machinery samples + crowd chants, 130 BPM, brutal direct revolt

**`music_faction_collective_debrief.wav`** (variant: `debrief`, seed: `350004`)

> Collective debrief, slow factory hum in G minor + worker-choir hum + accordion + receding drum, 70 BPM, weary victory

#### `music_faction_collegium` — Collegium Theme

- Base parameters: **180 s** target duration, **70 BPM**, key **F major**
- Variants to bake here: calm, buildup, climax, debrief

**`music_faction_collegium_calm.wav`** (variant: `calm`, seed: `370001`)

> Collegium faction theme calm, monastic-scholarly ambient with Gregorian chant + drone organ + bell + soft strings + synth pad in F major + scriptorium quill, 70 BPM, contemplative knowledge

**`music_faction_collegium_buildup.wav`** (variant: `buildup`, seed: `370002`)

> Collegium buildup, chant rising + bass swell in F major + organ build + tome-slam percussion, 85 BPM, ritual preparation

**`music_faction_collegium_climax.wav`** (variant: `climax`, seed: `370003`)

> Collegium combat, sacred orchestral battle in D minor + male choir + brass + heavy strings + church bells + righteous chant, 110 BPM, archive defense

**`music_faction_collegium_debrief.wav`** (variant: `debrief`, seed: `370004`)

> Collegium debrief, sustained organ in F major + soft choir + library-quiet + soft chime, 60 BPM, sacred preservation

#### `music_faction_frontier` — Frontier Theme

- Base parameters: **180 s** target duration, **95 BPM**, key **G major**
- Variants to bake here: calm, buildup, climax, debrief

**`music_faction_frontier_calm.wav`** (variant: `calm`, seed: `320001`)

> Frontier faction theme calm, frontier folk-electronic with acoustic guitar + harmonica + light synth + steady drum + bottle-percussion in G major, 95 BPM, hardy independent settlers

**`music_faction_frontier_buildup.wav`** (variant: `buildup`, seed: `320002`)

> Frontier buildup, harmonica building + bass strum in G major + tom drums + outlaw whistle, 105 BPM, posse forming

**`music_faction_frontier_climax.wav`** (variant: `climax`, seed: `320003`)

> Frontier combat, western-electronic battle in E minor + electric guitar + heavy drums + bass synth + harmonica + holler shouts, 125 BPM, defiant outlaws

**`music_faction_frontier_debrief.wav`** (variant: `debrief`, seed: `320004`)

> Frontier debrief, slow acoustic guitar in G major + harmonica solo + saloon piano + crickets, 65 BPM, dusty homecoming

#### `music_faction_husks` — Husks Theme

- Base parameters: **180 s** target duration, **80 BPM**, key **B minor**
- Variants to bake here: calm, buildup, climax, debrief

**`music_faction_husks_calm.wav`** (variant: `calm`, seed: `360001`)

> Husks faction theme calm, alien-insectoid ambient with chittering + dissonant synth + drone + skittering percussion in B minor + distorted whisper, 80 BPM, unsettling hive presence

**`music_faction_husks_buildup.wav`** (variant: `buildup`, seed: `360002`)

> Husks buildup, skitter crescendo + bass throb in B minor + insect chitter swarming + dissonant string, 100 BPM, hive convergence

**`music_faction_husks_climax.wav`** (variant: `climax`, seed: `360003`)

> Husks combat, frantic alien insectoid in F minor + skittering percussion + dissonant strings + screaming horns + chaos chant, 145 BPM, overwhelming hive frenzy

**`music_faction_husks_debrief.wav`** (variant: `debrief`, seed: `360004`)

> Husks debrief, eerie drone in B minor + receding chitter + distant queen-call + heartbeat pulse, 60 BPM, alien quiet

#### `music_faction_ronin` — Ronin Theme

- Base parameters: **180 s** target duration, **88 BPM**, key **D minor**
- Variants to bake here: calm, buildup, climax, debrief

**`music_faction_ronin_calm.wav`** (variant: `calm`, seed: `330001`)

> Ronin faction theme calm, lone-wolf neo-noir with koto + electric piano + minimal synth pad in D minor + cyberpunk rain + lonely sax, 88 BPM, wandering blade-for-hire melancholy

**`music_faction_ronin_buildup.wav`** (variant: `buildup`, seed: `330002`)

> Ronin buildup, koto pluck rising + bass pulse in D minor + taiko build + neon-flicker percussion, 100 BPM, duel imminent

**`music_faction_ronin_climax.wav`** (variant: `climax`, seed: `330003`)

> Ronin combat, cyberpunk samurai in D minor + driving taiko drums + electric guitar + koto + screaming synth lead, 130 BPM, blade-and-bullet ballet

**`music_faction_ronin_debrief.wav`** (variant: `debrief`, seed: `330004`)

> Ronin debrief, solo koto in D minor + sustained pad + rain on neon + soft cello, 60 BPM, blood-stained reflection

#### `music_faction_starlight` — Starlight Theme

- Base parameters: **180 s** target duration, **65 BPM**, key **A major**
- Variants to bake here: calm, buildup, climax, debrief

**`music_faction_starlight_calm.wav`** (variant: `calm`, seed: `380001`)

> Starlight faction theme calm, solar-ritual ambient with bell tones + drone synth + cathedral organ + light percussion + synth pad in A major + ethereal choir, 65 BPM, religious science

**`music_faction_starlight_buildup.wav`** (variant: `buildup`, seed: `380002`)

> Starlight buildup, choir building + bass swell in A major + ritual-bell crescendo + sunburst harp, 85 BPM, illumination rite

**`music_faction_starlight_climax.wav`** (variant: `climax`, seed: `380003`)

> Starlight combat, ritualistic orchestral in D minor + full choir + brass + tribal drums + organ + ecstatic chant, 115 BPM, fanatical fervor

**`music_faction_starlight_debrief.wav`** (variant: `debrief`, seed: `380004`)

> Starlight debrief, sustained organ in A major + soft choir + receding bells + harp glissando, 55 BPM, illuminated peace

#### `music_faction_synth` — Synth Theme

- Base parameters: **180 s** target duration, **105 BPM**, key **A minor**
- Variants to bake here: calm, buildup, climax, debrief

**`music_faction_synth_calm.wav`** (variant: `calm`, seed: `340001`)

> Synth faction theme calm, robotic drone-collective ambient, synthetic monotone choir + arpeggiated sequencer + cold synth pad in A minor + circuit-glitch percussion, 105 BPM, machine consensus

**`music_faction_synth_buildup.wav`** (variant: `buildup`, seed: `340002`)

> Synth buildup, sequencer accelerating + bass pulse in A minor + glitch percussion + hive-mind ping, 115 BPM, swarm coalescing

**`music_faction_synth_climax.wav`** (variant: `climax`, seed: `340003`)

> Synth combat, frantic electronic in A minor + arpeggiated sequencer at full speed + heavy drums + distorted bass + robotic vocals, 140 BPM, mechanized overwhelm

**`music_faction_synth_debrief.wav`** (variant: `debrief`, seed: `340004`)

> Synth debrief, slow arpeggio in A minor + synthetic pad + cooling-fan whir + bell tone, 70 BPM, machine satisfaction

### Storyteller theme (20 files)

Five storyteller directors (Cassandra Classic / Phoebe Chillax / Randy Random / Ironman / Sandbox) drive the rogue-like event pacing. Their themes underscore narrative beats: `calm` for ordinary play, `buildup` for an incoming director event, `climax` for the event resolution, `debrief` for the post-event reflection. The director's personality dictates the music vibe (e.g. Randy is chaotic, Ironman is grim, Phoebe is mellow).

#### `music_storyteller_cassandra` — Cassandra Classic Theme

- Base parameters: **180 s** target duration, **95 BPM**, key **C minor**
- Variants to bake here: calm, buildup, climax, debrief

**`music_storyteller_cassandra_calm.wav`** (variant: `calm`, seed: `410001`)

> Cassandra Classic narrative theme calm, balanced cinematic synth pad in C minor + soft strings + measured piano + steady heartbeat percussion, 95 BPM, fair storyteller pacing

**`music_storyteller_cassandra_buildup.wav`** (variant: `buildup`, seed: `410002`)

> Cassandra Classic buildup, strings swell + bass pulse in C minor + tension percussion + ascending piano motif, 105 BPM, the story turns

**`music_storyteller_cassandra_climax.wav`** (variant: `climax`, seed: `410003`)

> Cassandra Classic event climax, orchestral in C minor + heavy strings + brass + percussion + leitmotif return, 120 BPM, dramatic incident

**`music_storyteller_cassandra_debrief.wav`** (variant: `debrief`, seed: `410004`)

> Cassandra Classic debrief, slow piano outro in C minor + soft strings + lone violin, 65 BPM, balanced reflection

#### `music_storyteller_ironman` — Ironman Theme

- Base parameters: **180 s** target duration, **88 BPM**, key **G minor**
- Variants to bake here: calm, buildup, climax, debrief

**`music_storyteller_ironman_calm.wav`** (variant: `calm`, seed: `440001`)

> Ironman narrative theme calm, grim permadeath ambient, low synth drone in G minor + sparse cello + military snare + ticking clock + heartbeat pulse, 88 BPM, no second chances

**`music_storyteller_ironman_buildup.wav`** (variant: `buildup`, seed: `440002`)

> Ironman buildup, low strings rising + bass throb in G minor + snare roll + funeral-bell tease, 100 BPM, irreversible threat

**`music_storyteller_ironman_climax.wav`** (variant: `climax`, seed: `440003`)

> Ironman event climax, dark orchestral in G minor + heavy strings + funeral brass + drum-roll + ominous choir, 115 BPM, life-or-death stakes

**`music_storyteller_ironman_debrief.wav`** (variant: `debrief`, seed: `440004`)

> Ironman debrief, solo cello in G minor + sustained drone + funeral bell + sparse piano, 55 BPM, permadeath aftermath

#### `music_storyteller_phoebe` — Phoebe Chillax Theme

- Base parameters: **180 s** target duration, **80 BPM**, key **Bb major**
- Variants to bake here: calm, buildup, climax, debrief

**`music_storyteller_phoebe_calm.wav`** (variant: `calm`, seed: `420001`)

> Phoebe Chillax narrative theme calm, mellow lofi synth pad in Bb major + soft piano + jazz brush percussion + sparse warm bass, 80 BPM, player-friendly mellow

**`music_storyteller_phoebe_buildup.wav`** (variant: `buildup`, seed: `420002`)

> Phoebe Chillax buildup, warm strings swelling + soft bass in Bb major + light percussion + gentle vibraphone, 90 BPM, light tension

**`music_storyteller_phoebe_climax.wav`** (variant: `climax`, seed: `420003`)

> Phoebe Chillax event climax, light orchestral in Bb major + warm brass + brush drums + piano melody + uplifting choir, 105 BPM, generous challenge

**`music_storyteller_phoebe_debrief.wav`** (variant: `debrief`, seed: `420004`)

> Phoebe Chillax debrief, mellow piano outro in Bb major + soft strings + smiling vibraphone, 60 BPM, gentle wind-down

#### `music_storyteller_randy` — Randy Random Theme

- Base parameters: **180 s** target duration, **110 BPM**, key **F# minor**
- Variants to bake here: calm, buildup, climax, debrief

**`music_storyteller_randy_calm.wav`** (variant: `calm`, seed: `430001`)

> Randy Random narrative theme calm, chaotic-unpredictable synth pad in F# minor + glitch percussion + erratic piano stabs + random pitch sweeps, 110 BPM, anything goes

**`music_storyteller_randy_buildup.wav`** (variant: `buildup`, seed: `430002`)

> Randy Random buildup, escalating chaos in F# minor + accelerating drums + dissonant strings + alarm sweeps, 125 BPM, unpredictable cascade

**`music_storyteller_randy_climax.wav`** (variant: `climax`, seed: `430003`)

> Randy Random event climax, frantic electronic in F# minor + double-time drums + dissonant brass + screaming synth + chaos percussion, 145 BPM, total mayhem

**`music_storyteller_randy_debrief.wav`** (variant: `debrief`, seed: `430004`)

> Randy Random debrief, surreal pad in F# minor + erratic glitch fading + soft chime + breath of relief, 75 BPM, chaos receding

#### `music_storyteller_sandbox` — Sandbox Theme

- Base parameters: **180 s** target duration, **75 BPM**, key **D major**
- Variants to bake here: calm, buildup, climax, debrief

**`music_storyteller_sandbox_calm.wav`** (variant: `calm`, seed: `450001`)

> Sandbox narrative theme calm, pure-exploration ambient with airy synth pad in D major + acoustic guitar + bird-call samples + minimal percussion + harp glissando, 75 BPM, no-pressure curiosity

**`music_storyteller_sandbox_buildup.wav`** (variant: `buildup`, seed: `450002`)

> Sandbox buildup, soft strings swelling + warm bass in D major + light percussion + discovery harp, 85 BPM, mild surprise

**`music_storyteller_sandbox_climax.wav`** (variant: `climax`, seed: `450003`)

> Sandbox climax, exploration orchestral in D major + soaring strings + brass crescendo + uplifting drum + choir of wonder, 105 BPM, major discovery

**`music_storyteller_sandbox_debrief.wav`** (variant: `debrief`, seed: `450004`)

> Sandbox debrief, acoustic guitar outro in D major + soft strings + harp + gentle wind, 60 BPM, contented exploration

### Boss theme (20 files)

Five endgame bosses (The Hollow King, The Frozen Heart, The Crimson Tide, The Eclipse Walker, The Last Star). Each has multi-phase combat — `calm` is the arena entry, `buildup` is phase 1, `climax` covers the combat phases, `debrief` is the defeat cadence. Tracks are 240-300 s native because boss fights are long.

#### `music_boss_crimson_tide` — The Crimson Tide Theme

- Base parameters: **240 s** target duration, **105 BPM**, key **F minor**
- Boss has 4 combat phases
- Tied to world: `mars`
- Variants to bake here: calm, buildup, climax, debrief

**`music_boss_crimson_tide_calm.wav`** (variant: `calm`, seed: `530001`)

> Crimson Tide boss arena entry, dust-storm orchestral with rust-grit percussion + heavy brass + low synth pad in F minor + Bedouin choir + sand-walker chant, 105 BPM, sandstorm-titan looms

**`music_boss_crimson_tide_buildup.wav`** (variant: `buildup`, seed: `530002`)

> Crimson Tide phase 1/2 buildup, sandstorm crescendo + bass throb in F minor + driving tribal drum + windswept brass + war chant, 120 BPM, swarm tide rising

**`music_boss_crimson_tide_climax.wav`** (variant: `climax`, seed: `530003`)

> Crimson Tide phase 3/4 combat, furious orchestral in F minor + full tribal drums + heavy brass + dust-storm howl + war choir + creature roars + crumbling-arena rumble, 135 BPM, four-phase sand-titan war

**`music_boss_crimson_tide_debrief.wav`** (variant: `debrief`, seed: `530004`)

> Crimson Tide defeat, settling-dust orchestral in F minor + receding tribal drum + lone reed flute + sparse cello + wind sigh, 70 BPM, sand-buried lament

#### `music_boss_eclipse_walker` — The Eclipse Walker Theme

- Base parameters: **240 s** target duration, **102 BPM**, key **C# minor**
- Boss has 3 combat phases
- Tied to world: `mimas`
- Variants to bake here: calm, buildup, climax, debrief

**`music_boss_eclipse_walker_calm.wav`** (variant: `calm`, seed: `540001`)

> Eclipse Walker boss arena entry, microgravity-eerie orchestral with floating synth pad in C# minor + ethereal choir + gravity-warp synth + slow heartbeat + reverb cello, 102 BPM, weightless cyborg presence

**`music_boss_eclipse_walker_buildup.wav`** (variant: `buildup`, seed: `540002`)

> Eclipse Walker phase 1 buildup, cyborg-precision crescendo + bass pulse in C# minor + tight drum + glitch percussion + ascending choir, 118 BPM, gravity inversion incoming

**`music_boss_eclipse_walker_climax.wav`** (variant: `climax`, seed: `540003`)

> Eclipse Walker phase 2/3 combat, frantic electronic-orchestral in C# minor + full drum + brass + cyborg-vocals + gravity-warp synth lead + agile percussion, 132 BPM, microgravity duel

**`music_boss_eclipse_walker_debrief.wav`** (variant: `debrief`, seed: `540004`)

> Eclipse Walker defeat, drifting synth pad in C# minor + receding choir + soft chime + cooling-cyborg whine, 65 BPM, weightless lament

#### `music_boss_frozen_heart` — The Frozen Heart Theme

- Base parameters: **240 s** target duration, **95 BPM**, key **B minor**
- Boss has 3 combat phases
- Tied to world: `europa`
- Variants to bake here: calm, buildup, climax, debrief

**`music_boss_frozen_heart_calm.wav`** (variant: `calm`, seed: `520001`)

> Frozen Heart boss arena entry, glacial dread orchestral with ice-crystal chimes + low choir + cold synth pad in B minor + creature heartbeat + sonar ping, 95 BPM, deep-cold confrontation

**`music_boss_frozen_heart_buildup.wav`** (variant: `buildup`, seed: `520002`)

> Frozen Heart phase 1 buildup, ice-crystal crescendo + cryogenic synth swell in B minor + driving drum + whisper choir + cold-snap percussion, 110 BPM, supercooled awakening

**`music_boss_frozen_heart_climax.wav`** (variant: `climax`, seed: `520003`)

> Frozen Heart phase 2/3 combat, frigid orchestral in B minor + full strings + ice-chime + thundering drum + creature roar + screaming brass + supercooled-core whine, 125 BPM, cryogenic meltdown war

**`music_boss_frozen_heart_debrief.wav`** (variant: `debrief`, seed: `520004`)

> Frozen Heart defeat, mournful strings in B minor + descending chime + sparse cello + ice shatter + soft choir, 65 BPM, heart-of-ice lament

#### `music_boss_hollow_king` — The Hollow King Theme

- Base parameters: **240 s** target duration, **100 BPM**, key **D minor**
- Boss has 3 combat phases
- Tied to world: `earth`
- Variants to bake here: calm, buildup, climax, debrief

**`music_boss_hollow_king_calm.wav`** (variant: `calm`, seed: `510001`)

> Hollow King boss arena entry, ominous building orchestral with menacing brass + tribal drums + low choir + bass + slow heartbeat in D minor, 100 BPM, confrontation looming

**`music_boss_hollow_king_buildup.wav`** (variant: `buildup`, seed: `510002`)

> Hollow King phase 1 buildup, strings crescendo + brass build + tribal drums escalating in D minor + lava-crackle percussion + king's-voice chant, 115 BPM, flame king awakens

**`music_boss_hollow_king_climax.wav`** (variant: `climax`, seed: `510003`)

> Hollow King phase 2/3 combat, triumphant epic in D minor + full orchestra + battle choir + powerful brass + thundering drums + leitmotif + pyroclastic roar, 130 BPM, climactic flame king war

**`music_boss_hollow_king_debrief.wav`** (variant: `debrief`, seed: `510004`)

> Hollow King defeat, somber orchestral cadence in D minor + descending strings + low brass + funeral choir + cooling-lava crackle, 70 BPM, fallen king lament

#### `music_boss_last_star` — The Last Star Theme

- Base parameters: **300 s** target duration, **110 BPM**, key **A minor**
- Boss has 5 combat phases
- Tied to world: `vulcan`
- Variants to bake here: calm, buildup, climax, debrief

**`music_boss_last_star_calm.wav`** (variant: `calm`, seed: `550001`)

> Last Star superboss arena entry, stellar-cathedral orchestral with cathedral organ + full choir + low brass + slow heartbeat + synth pad in A minor + cosmic-wind howl, 110 BPM, end-of-campaign confrontation

**`music_boss_last_star_buildup.wav`** (variant: `buildup`, seed: `550002`)

> Last Star phase 1/2 buildup, choir crescendo + organ build + bass throb in A minor + ascending strings + ritual percussion + stellar-flare hiss, 125 BPM, sol-zone-titan awakens

**`music_boss_last_star_climax.wav`** (variant: `climax`, seed: `550003`)

> Last Star phase 3/4/5 combat, climactic epic orchestral in A minor + full choir + powerful brass + thundering drums + leitmotif return + screaming synth lead + cosmic-roar samples + stellar-wrath howl, 140 BPM, end-game superboss war

**`music_boss_last_star_debrief.wav`** (variant: `debrief`, seed: `550004`)

> Last Star defeat, triumphant resolved orchestral in A major + ascending choir + warm brass + sustained organ + soft drum + dawn-chime + receding stellar wind, 90 BPM, campaign-ending triumph

---

## Appendix B — Machine-readable `prompts.json`

A ready-to-`json.load()` version of all 83 entries lives at:

```
tools/audio_pipeline/HANDOFF_LOCAL_MUSIC_BAKE_prompts.json
```

(committed to the Corefall repo alongside this doc). Each entry has fields: `track_id`, `variant`, `file_id`, `canonical_name`, `duration_seconds`, `tempo_bpm`, `key`, `prompt`, `seed`, `group`.

---

## Appendix C — Quick reference: 4 variant prompts per track

ACE-Step v1.5 (and most modern music models) responds to **detailed prose prompts** more than tag clouds. Each prompt in Appendix A already specifies: scene/context + instrumentation + harmonic key + tempo + emotional descriptor. Keep them intact when you feed them into the model — the descriptive specificity is doing real work.

If ACE-Step lets you provide *both* a long prompt AND a tag string, also pass these tag distillations:

| Variant | Tag cloud |
|---|---|
| `calm` | `instrumental, ambient, low-tension, sparse percussion, minimal arrangement, no vocals, loopable, cinematic` |
| `buildup` | `instrumental, rising tension, pulsing low end, rhythmic underlay, lead motif emerging, no vocals, loopable` |
| `climax` | `instrumental, full arrangement, driving drums, heavy bass, lead synth or brass, harmonic peak, no vocals, loopable` |
| `debrief` | `instrumental, reflective, sparse percussion, sustained pad, melodic recap, slow tempo, no vocals, loopable` |

---

## Appendix D — Common failure modes + recovery

| Symptom | Cause | Recovery |
|---|---|---|
| Vocals leak into a track | `force_instrumental=True` not honored | Append "strictly instrumental, no vocals, no singing, no choir without lyrics" to the prompt + reroll with seed+1 |
| Track is shorter than `duration_seconds` | ACE-Step soft-capped on this prompt | Generate twice and crossfade-extend (see §4) OR raise `num_inference_steps` and retry |
| Track sounds noisy / muddy | `num_inference_steps` too low | Bump from 100 → 150 → 200; quality plateaus around 200 |
| Track is repetitive / loops too obviously | guidance_scale too high | Drop from 7.5 → 5.0 and retry |
| CUDA OOM | activation memory spike on long prompts | Switch to `bf16` instead of `fp32`; drop `num_inference_steps` to 100 |
| Track ends abruptly mid-phrase | model didn't see a natural cadence | Allow 5-10 s overgenerate, then trim to nearest cadence in post-processing |
| Mac/AMD host doesn't run | ACE-Step says it supports those, but verify your CUDA path | Stick to the 5090/CUDA host; do NOT try this on the Mac M-series laptop |

---

## Appendix E — Contact

If you hit a blocker that requires a decision (e.g., "ACE-Step v1.6 came out and the API changed", "a specific prompt produces garbage no matter what", "GPU thermal throttle is causing failures"), do NOT improvise large changes. Save your `progress.json`, screenshot the failure, and ping the product owner. They will decide within 2 hours of waking time (Phoenix MST).

For routine quality tweaks (per-track seed reroll, prompt micro-edits, scheduler swap), you have full authority over the bake script. Note any deviations in `bake_report.txt`.

---

**Last updated**: 2026-05-15 (Phoenix MST). If you receive this doc later than 2026-08-15, ping the product owner — ACE-Step / model landscape may have drifted significantly.

— End handoff
