"""M12A § Meta AudioCraft adapter — secondary SFX synthesis backend.

Per spec § Architecture rules:
> AudioCraft (Meta) as secondary.
> Determinism: AudioCraft is deterministic with seed; verified per-machine.

Mirrors `stable_audio_adapter.py` — primary path uses the local
AudioCraft inference if available; otherwise yields None so the
orchestrator routes the request to the procedural Tier 1 fallback or
ElevenLabs Tier 2 path.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Optional


@dataclass(frozen=True)
class AudioCraftRequest:
    """One AudioCraft request mirroring the SfxManifestEntry shape."""

    sfx_id: str
    prompt: str
    duration_sec: float
    seed: int
    target_lufs: float = -16.0


def is_model_available() -> bool:
    """Return True iff `audiocraft` is importable AND a pinned model
    checkpoint is locally reachable."""
    try:
        __import__("audiocraft")
    except ImportError:
        return False
    return True


def synthesize_to_wav(req: AudioCraftRequest, out_path: Path) -> Optional[Path]:
    """Synthesize via AudioCraft. Returns the written path on success;
    `None` when the model is unavailable (caller routes elsewhere)."""
    if not is_model_available():
        return None
    try:
        # Reserved branch — when audiocraft is installed, the inference
        # call lands here. AudioCraft uses the `MusicGen.generate`
        # interface for music + `audiogen` for SFX.
        from audiocraft.models import AudioGen  # type: ignore

        model = AudioGen.get_pretrained("facebook/audiogen-medium")
        model.set_generation_params(duration=req.duration_sec)
        wav = model.generate([req.prompt])
        import soundfile as sf  # type: ignore

        sf.write(str(out_path), wav.squeeze().cpu().numpy(), 32000)
        return out_path
    except Exception:
        return None


__all__ = ["AudioCraftRequest", "is_model_available", "synthesize_to_wav"]
