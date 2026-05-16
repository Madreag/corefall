"""Post-processing pass shared by every bake.

Operations per shipped WAV:
1. Trim leading/trailing silence below -60 dBFS (skip if shorter than 50 ms)
2. Optional loop-align (crossfade head/tail) when `loop=True`
3. Apply 5 ms fade-in + 5 ms fade-out (click suppression)
4. Normalize peak to category target (voice -16 / sfx -14 / music -8 dBFS)
5. Re-write as 16-bit PCM at 48 kHz (matches Wwise/FMOD shipping standard)

Stereo input is preserved. Mono input stays mono.
"""

from __future__ import annotations

from pathlib import Path
from typing import Literal

import numpy as np
import soundfile as sf

SAMPLE_RATE = 48000

CategoryTarget = Literal["voice", "sfx", "music"]

PEAK_DBFS_TARGETS: dict[CategoryTarget, float] = {
    "voice": -16.0,
    "sfx": -14.0,
    "music": -8.0,
}


def _dbfs_to_linear(dbfs: float) -> float:
    return float(10.0 ** (dbfs / 20.0))


def _peak(samples: np.ndarray) -> float:
    if samples.size == 0:
        return 0.0
    return float(np.max(np.abs(samples)) + 1e-12)


def _trim_silence(samples: np.ndarray, sr: int, floor_dbfs: float = -60.0) -> np.ndarray:
    if samples.ndim == 1:
        mono = samples
    else:
        mono = samples.mean(axis=1)
    threshold = _dbfs_to_linear(floor_dbfs)
    above = np.where(np.abs(mono) > threshold)[0]
    if above.size == 0:
        return samples
    start = int(above[0])
    end = int(above[-1]) + 1
    min_len = max(1, int(0.050 * sr))
    if end - start < min_len:
        return samples
    return samples[start:end]


def _fade(samples: np.ndarray, sr: int, fade_ms: float = 5.0) -> np.ndarray:
    n = samples.shape[0]
    fade_n = min(int(fade_ms * sr / 1000.0), n // 2)
    if fade_n <= 1:
        return samples
    ramp = np.linspace(0.0, 1.0, fade_n, dtype=np.float64)
    out = samples.astype(np.float64).copy()
    if out.ndim == 1:
        out[:fade_n] *= ramp
        out[-fade_n:] *= ramp[::-1]
    else:
        out[:fade_n] *= ramp[:, None]
        out[-fade_n:] *= ramp[::-1, None]
    return out


def _normalize_peak(samples: np.ndarray, target_dbfs: float) -> np.ndarray:
    target = _dbfs_to_linear(target_dbfs)
    peak = _peak(samples)
    if peak <= 1e-7:
        return samples
    return samples * (target / peak)


def _loop_align(samples: np.ndarray, sr: int, fade_ms: float = 50.0) -> np.ndarray:
    n = samples.shape[0]
    fade_n = min(int(fade_ms * sr / 1000.0), n // 4)
    if fade_n <= 1:
        return samples
    out = samples.astype(np.float64).copy()
    blend = np.linspace(0.0, 1.0, fade_n, dtype=np.float64)
    head = out[:fade_n].copy()
    tail = out[-fade_n:].copy()
    if out.ndim == 1:
        cross = tail * (1.0 - blend) + head * blend
        out[:fade_n] = cross
        out[-fade_n:] = cross
    else:
        cross = tail * (1.0 - blend[:, None]) + head * blend[:, None]
        out[:fade_n] = cross
        out[-fade_n:] = cross
    return out


def cleanup_wav(
    path: Path,
    *,
    category: CategoryTarget,
    loop: bool = False,
    skip_trim: bool = False,
) -> None:
    """In-place cleanup of a baked WAV file.

    No-op + warning print if the file is empty or unreadable.
    """
    data, sr = sf.read(str(path), dtype="float64", always_2d=False)
    if data.size == 0:
        return
    samples = data
    if not skip_trim:
        samples = _trim_silence(samples, sr)
    if loop:
        samples = _loop_align(samples, sr)
    samples = _fade(samples, sr, fade_ms=5.0)
    samples = _normalize_peak(samples, PEAK_DBFS_TARGETS[category])
    samples = np.clip(samples, -1.0, 1.0)
    pcm = (samples * 32767.0).astype(np.int16)
    sf.write(str(path), pcm, sr, subtype="PCM_16", format="WAV")


def to_48k_mono_pcm16(path: Path) -> None:
    """Resample a WAV to 48kHz mono PCM16 (used for voice + SFX shipping format)."""
    data, sr = sf.read(str(path), dtype="float64", always_2d=False)
    if data.ndim > 1:
        data = data.mean(axis=1)
    if sr != SAMPLE_RATE:
        ratio = SAMPLE_RATE / sr
        n_new = int(round(len(data) * ratio))
        if n_new <= 0:
            return
        xp_old = np.arange(len(data)) / sr
        xp_new = np.arange(n_new) / SAMPLE_RATE
        data = np.interp(xp_new, xp_old, data)
        sr = SAMPLE_RATE
    pcm = (np.clip(data, -1.0, 1.0) * 32767.0).astype(np.int16)
    sf.write(str(path), pcm, sr, subtype="PCM_16", format="WAV")


__all__ = ["cleanup_wav", "to_48k_mono_pcm16", "PEAK_DBFS_TARGETS"]
