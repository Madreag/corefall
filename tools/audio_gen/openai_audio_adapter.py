"""M12A § OpenAI Audio adapter — fallback SFX backend for special cases.

Per spec § Architecture rules:
> OpenAI Audio / ElevenLabs as fallback for special cases (voice grunts
> that need character).

OpenAI Audio doesn't ship a free SFX synthesis surface today; this
adapter ships a stub that returns None unless an explicit
`OPENAI_API_KEY` is configured at `~/.config/cf-audio/openai.toml`.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Optional


@dataclass(frozen=True)
class OpenAIAudioRequest:
    """One OpenAI Audio request."""

    sfx_id: str
    prompt: str
    duration_sec: float


def is_configured() -> bool:
    """Return True iff `~/.config/cf-audio/openai.toml` exists with a
    `api_key = "..."` line. Falls back to `OPENAI_API_KEY` env var."""
    import os

    config = Path.home() / ".config" / "cf-audio" / "openai.toml"
    return config.exists() or bool(os.environ.get("OPENAI_API_KEY"))


def synthesize_to_wav(req: OpenAIAudioRequest, out_path: Path) -> Optional[Path]:
    """Stub — OpenAI does not currently expose a free SFX synthesis
    surface. The adapter is preserved per the spec § Files contract;
    when OpenAI ships a `audio.synthesis` endpoint, the inference
    path lands here. Today this returns None so the orchestrator
    routes to the procedural Tier 1 fallback.
    """
    _ = req
    _ = out_path
    return None


__all__ = ["OpenAIAudioRequest", "is_configured", "synthesize_to_wav"]
