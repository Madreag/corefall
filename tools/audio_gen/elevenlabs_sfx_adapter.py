"""M12A § ElevenLabs Sound Effects adapter — fallback SFX backend.

Per spec § Architecture rules:
> OpenAI Audio / ElevenLabs as fallback for special cases (voice grunts
> that need character).

This adapter is a thin wrapper around `tools/audio_pipeline/eleven_sfx.py`
which already implements the production Tier 2 ElevenLabs SFX bake +
ledger supersede. The spec-canonical entry point reuses the proven
implementation under the `tools/audio_gen/` namespace so the M12A
orchestrator can dispatch via `from audio_gen.elevenlabs_sfx_adapter
import synthesize_to_wav`.
"""

from __future__ import annotations

import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Optional

_HERE = Path(__file__).resolve().parent
_REPO_ROOT = _HERE.parents[1]
sys.path.insert(0, str(_REPO_ROOT / "tools" / "audio_pipeline"))


@dataclass(frozen=True)
class ElevenLabsSfxRequest:
    """One ElevenLabs SFX request."""

    sfx_id: str
    prompt: str
    duration_sec: float
    loops: bool = False
    prompt_influence: float = 0.45


def is_configured() -> bool:
    """Return True iff the ElevenLabs API key is reachable."""
    config = Path.home() / ".config" / "cf-audio" / "elevenlabs.toml"
    return config.exists()


def synthesize_to_wav(req: ElevenLabsSfxRequest, out_path: Path) -> Optional[Path]:
    """Call `client.text_to_sound_effects.convert` via the existing
    `tools/audio_pipeline/eleven_sfx.py` pipeline. Returns the WAV path
    on success; `None` when the API key is missing or the call fails.
    """
    if not is_configured():
        return None
    try:
        from elevenlabs.client import ElevenLabs  # type: ignore
        from keys import load_elevenlabs_key  # type: ignore
        from post_process import cleanup_wav  # type: ignore
    except ImportError:
        return None

    key = load_elevenlabs_key()
    client = ElevenLabs(api_key=key.value)
    pcm_path = out_path.with_suffix(".pcm")

    try:
        stream = client.text_to_sound_effects.convert(
            text=req.prompt,
            duration_seconds=float(req.duration_sec),
            loop=bool(req.loops),
            prompt_influence=req.prompt_influence,
            model_id="eleven_text_to_sound_v2",
            output_format="pcm_48000",
        )
    except Exception:
        return None

    try:
        out_path.parent.mkdir(parents=True, exist_ok=True)
        written = 0
        with pcm_path.open("wb") as f:
            for chunk in stream:
                if chunk:
                    f.write(chunk)
                    written += len(chunk)
        if written < 4096:
            pcm_path.unlink(missing_ok=True)
            return None
        import numpy as np  # type: ignore
        import soundfile as sf  # type: ignore

        raw = pcm_path.read_bytes()
        data = np.frombuffer(raw, dtype="<i2")
        sf.write(str(out_path), data, 48000, subtype="PCM_16", format="WAV")
        pcm_path.unlink(missing_ok=True)
        cleanup_wav(out_path, category="sfx", loop=bool(req.loops), skip_trim=False)
        return out_path
    except Exception:
        try:
            pcm_path.unlink(missing_ok=True)
            if out_path.exists() and out_path.stat().st_size < 8192:
                out_path.unlink(missing_ok=True)
        except OSError:
            pass
        return None


__all__ = [
    "ElevenLabsSfxRequest",
    "is_configured",
    "synthesize_to_wav",
]
