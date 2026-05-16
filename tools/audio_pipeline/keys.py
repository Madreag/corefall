"""Secret loader for cf-audio-pipeline.

Reads provider credentials from `~/.config/cf-audio/*.toml` (preferred — 600
perms enforced) or from environment variables. Never logs the key. Never
returns it embedded in any string the caller might serialize.
"""

from __future__ import annotations

import os
import stat
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Optional


CONFIG_DIR = Path.home() / ".config" / "cf-audio"


@dataclass(frozen=True)
class ProviderKey:
    name: str
    value: str

    def __repr__(self) -> str:
        masked = self.value[:4] + "…" if self.value else ""
        return f"ProviderKey(name={self.name!r}, value=<{masked}{len(self.value)}c>)"


def _read_toml_field(path: Path, field: str) -> Optional[str]:
    if not path.exists():
        return None
    try:
        mode = path.stat().st_mode & 0o777
    except OSError:
        return None
    if mode & (stat.S_IRGRP | stat.S_IROTH | stat.S_IWGRP | stat.S_IWOTH):
        print(
            f"[keys] refusing to read {path} — overly permissive (chmod 600)",
            file=sys.stderr,
        )
        return None
    try:
        import tomllib  # type: ignore[import-not-found]
    except ImportError:
        return None
    try:
        with path.open("rb") as f:
            data = tomllib.load(f)
        val = data.get(field)
        if isinstance(val, str) and val.strip():
            return val.strip()
    except Exception:
        return None
    return None


def load_elevenlabs_key() -> ProviderKey:
    """Order: ELEVENLABS_API_KEY env > ~/.config/cf-audio/elevenlabs.toml(api_key)."""
    env = os.environ.get("ELEVENLABS_API_KEY", "").strip()
    if env:
        return ProviderKey(name="ElevenLabs", value=env)
    val = _read_toml_field(CONFIG_DIR / "elevenlabs.toml", "api_key")
    if val:
        return ProviderKey(name="ElevenLabs", value=val)
    raise RuntimeError(
        "ElevenLabs API key not found. Drop at ~/.config/cf-audio/elevenlabs.toml "
        "(api_key = \"sk_...\", chmod 600) or export ELEVENLABS_API_KEY."
    )


def load_aiva_credentials() -> tuple[Optional[str], Optional[str]]:
    """Returns (email, password). Used by the Playwright capture flow only."""
    email = os.environ.get("AIVA_EMAIL", "").strip() or _read_toml_field(
        CONFIG_DIR / "aiva.toml", "email"
    )
    password = os.environ.get("AIVA_PASSWORD", "").strip() or _read_toml_field(
        CONFIG_DIR / "aiva.toml", "password"
    )
    return email or None, password or None


AIVA_STATE_PATH = CONFIG_DIR / "aiva_state.json"


__all__ = [
    "CONFIG_DIR",
    "AIVA_STATE_PATH",
    "ProviderKey",
    "load_elevenlabs_key",
    "load_aiva_credentials",
]
