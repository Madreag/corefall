"""M12 § CCCP-style intro slideshow — narration voice bake.

Bakes the single ~67 second narration WAV for the M12 intro slideshow via
ElevenLabs `eleven_v3` (highest expressiveness). Uses the existing
`cassandra_narrator_balanced_female` voice from the per-NPC registry — a
warm authoritative middle-aged female narrator (Helldivers-2-Eagle-1
meets calm BBC documentary, American accent).

Per M12 spec § CCCP-style intro slideshow:
- Voice-over WAV baked via ElevenLabs `eleven_v3`
- Uses existing storyteller voice `cassandra_narrator_balanced_female`
- ~$0.50 of credits at our scale (~470 chars; we have ~4,200 remaining)

Output: `game/content/audio/voice/voice_intro_narration_corefall_universe_arc.wav`
Ledger: inserted as `Audio_Voice` / `Tier2_Audio_Production` via
`ledger_supersede::add_new_entries`.

Run from repo root:

    tools/asset_gen/.venv/bin/python tools/audio_pipeline/eleven_intro_narration.py
"""

from __future__ import annotations

import argparse
import sys
import tomllib
from pathlib import Path

from elevenlabs.client import ElevenLabs
from elevenlabs.types.voice_settings import VoiceSettings

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))

from keys import load_elevenlabs_key  # noqa: E402
from ledger_supersede import SupersedeRecord, add_new_entries  # noqa: E402
from post_process import cleanup_wav  # noqa: E402

REPO_ROOT = _HERE.parents[1]
OUT_DIR = REPO_ROOT / "game" / "content" / "audio" / "voice"
REGISTRY_PATH = _HERE / "voice_synthesis" / "per_npc_voice_registry.toml"

PIPELINE = "M12_eleven_intro_narration_v1"
TOOL = "tools/audio_pipeline/eleven_intro_narration.py"
MODEL = "eleven_v3"
MODEL_VERSION = "v3"

# Canonical narration matching `cf-ui::slideshow::INTRO_NARRATIVE`. The text
# is concatenated into a single ~67 second narration run; each subtitle
# matches one slide in the 8-slide cf-ui::slideshow timeline. Audio tags
# (e.g. [pause] / [breathy]) shape the eleven_v3 delivery without changing
# the visible text. cf-ui::slideshow renders the visible subtitle from
# its INTRO_NARRATIVE table, not from this WAV.
INTRO_NARRATION_TEXT = (
    "At the end of the 22nd century, Earth's old empires collapsed. "
    "[pause] But the survivors learned to leave their bodies behind. "
    "With brains preserved in steel and silicon, "
    "humanity scattered to twelve worlds across the Sol system. "
    "Each world is a frontier. Each frontier breeds factions. "
    "Coalition. Frontier. Ronin. Synth. "
    "Collective. Husks. Collegium. Starlight. "
    "[pause] The bunkers run deep. The atmospheres are real. "
    "The bodies bleed, leak, and burn. "
    "[pause] You will now join the frontier. Your command core is waiting."
)

VOICE_INTERNAL_ID = "cassandra_narrator_balanced_female"
CANONICAL_NAME = "voice_intro_narration_corefall_universe_arc"

VOICE_SETTINGS = VoiceSettings(
    stability=0.6,
    similarity_boost=0.85,
    style=0.30,
    use_speaker_boost=True,
)


def _resolve_voice_id() -> str:
    if not REGISTRY_PATH.exists():
        raise FileNotFoundError(f"voice registry missing: {REGISTRY_PATH}")
    with REGISTRY_PATH.open("rb") as f:
        registry = tomllib.load(f)
    entry = registry.get(VOICE_INTERNAL_ID)
    if not entry:
        raise KeyError(f"voice {VOICE_INTERNAL_ID!r} not in registry")
    return str(entry["elevenlabs_voice_id"])


def _stream_to_pcm(byte_iter, dest: Path) -> int:
    dest.parent.mkdir(parents=True, exist_ok=True)
    written = 0
    with dest.open("wb") as f:
        for chunk in byte_iter:
            if not chunk:
                continue
            f.write(chunk)
            written += len(chunk)
    return written


def _wrap_pcm_to_wav(pcm_path: Path, wav_path: Path, *, sample_rate: int) -> None:
    import numpy as np
    import soundfile as sf

    raw = pcm_path.read_bytes()
    if not raw:
        raise ValueError("empty PCM stream")
    data = np.frombuffer(raw, dtype="<i2")
    sf.write(str(wav_path), data, sample_rate, subtype="PCM_16", format="WAV")


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--dry-run", action="store_true")
    args = ap.parse_args()

    text_chars = len(INTRO_NARRATION_TEXT)
    print(f"[intro-narration] {text_chars} chars of text")
    print(f"[intro-narration] voice = {VOICE_INTERNAL_ID}")
    print(f"[intro-narration] model = {MODEL}")

    if args.dry_run:
        print(f"[intro-narration] DRY-RUN: would call ElevenLabs convert + write {OUT_DIR / (CANONICAL_NAME + '.wav')}")
        return 0

    voice_id = _resolve_voice_id()
    key = load_elevenlabs_key()
    client = ElevenLabs(api_key=key.value)
    print(f"[intro-narration] client ready ({key!r})")

    out_path = OUT_DIR / f"{CANONICAL_NAME}.wav"
    pcm_path = out_path.with_suffix(".pcm")
    try:
        stream = client.text_to_speech.convert(
            voice_id=voice_id,
            text=INTRO_NARRATION_TEXT,
            model_id=MODEL,
            voice_settings=VOICE_SETTINGS,
            output_format="pcm_48000",
        )
    except Exception as exc:
        print(f"[intro-narration] FAIL request: {exc}")
        return 2

    try:
        size = _stream_to_pcm(stream, pcm_path)
        if size < 8192:
            print(f"[intro-narration] FAIL — short PCM ({size}B)")
            pcm_path.unlink(missing_ok=True)
            return 3
        _wrap_pcm_to_wav(pcm_path, out_path, sample_rate=48000)
        pcm_path.unlink(missing_ok=True)
        cleanup_wav(out_path, category="voice", loop=False, skip_trim=False)
    except Exception as exc:
        print(f"[intro-narration] FAIL stream/wrap: {exc}")
        try:
            pcm_path.unlink(missing_ok=True)
            if out_path.exists() and out_path.stat().st_size < 8192:
                out_path.unlink(missing_ok=True)
        except OSError:
            pass
        return 4

    print(f"[intro-narration] wrote {out_path} ({out_path.stat().st_size} B)")

    record = SupersedeRecord(
        category="Audio_Voice",
        kind="voice_narration",
        canonical_name=CANONICAL_NAME,
        output_path=out_path.resolve(),
        new_pipeline=PIPELINE,
        new_tool=TOOL,
        new_model=MODEL,
        new_model_version=MODEL_VERSION,
        new_workflow="m12_intro_slideshow::narration",
        prompt=INTRO_NARRATION_TEXT,
        seed=abs(hash(CANONICAL_NAME)) & 0x7FFFFFFF,
        old_tier="Tier1_LLM_Audio",
        new_tier="Tier2_Audio_Production",
    )
    total = add_new_entries([record])
    print(f"[intro-narration] ledger total now {total}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
