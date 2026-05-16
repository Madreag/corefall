"""Procedural-synthesis primitives shared by sfx + music bakers.

All functions return float32 numpy arrays at SAMPLE_RATE Hz mono. write_wav()
converts to 16-bit PCM with peak normalization. Designed for M12A Tier-1
placeholder baking — no third-party audio models, only numpy + scipy.

Random state is always passed in explicitly so per-entry RNG remains
deterministic across runs.
"""

from __future__ import annotations

import math
from pathlib import Path
from typing import Optional, Sequence

import numpy as np
import soundfile as sf
from scipy import signal as sps

SAMPLE_RATE = 48000


def _n_samples(dur_sec: float) -> int:
    return max(1, int(round(dur_sec * SAMPLE_RATE)))


def t_axis(dur_sec: float) -> np.ndarray:
    n = _n_samples(dur_sec)
    return np.linspace(0.0, dur_sec, n, endpoint=False, dtype=np.float64)


def silence(dur_sec: float) -> np.ndarray:
    return np.zeros(_n_samples(dur_sec), dtype=np.float64)


def envelope_adsr(
    dur_sec: float,
    attack: float = 0.005,
    decay: float = 0.05,
    sustain: float = 0.6,
    release: float = 0.1,
) -> np.ndarray:
    n = _n_samples(dur_sec)
    if n == 0:
        return np.zeros(0)
    a_n = max(1, int(attack * SAMPLE_RATE))
    d_n = max(1, int(decay * SAMPLE_RATE))
    r_n = max(1, int(release * SAMPLE_RATE))
    s_n = max(0, n - a_n - d_n - r_n)
    out = np.zeros(n)
    if a_n > 0:
        out[:a_n] = np.linspace(0.0, 1.0, a_n, endpoint=False)
    if d_n > 0:
        out[a_n:a_n + d_n] = np.linspace(1.0, sustain, d_n, endpoint=False)
    if s_n > 0:
        out[a_n + d_n:a_n + d_n + s_n] = sustain
    if r_n > 0:
        out[a_n + d_n + s_n:a_n + d_n + s_n + r_n] = np.linspace(
            sustain, 0.0, r_n, endpoint=False,
        )
    return out


def envelope_exp_decay(dur_sec: float, tau: float = 0.05) -> np.ndarray:
    t = t_axis(dur_sec)
    return np.exp(-t / max(tau, 1e-6))


def envelope_attack_decay(dur_sec: float, attack: float = 0.005, tau: float = 0.05) -> np.ndarray:
    n = _n_samples(dur_sec)
    out = np.zeros(n)
    a_n = max(1, int(attack * SAMPLE_RATE))
    if a_n > n:
        a_n = n
    out[:a_n] = np.linspace(0.0, 1.0, a_n, endpoint=False)
    if a_n < n:
        decay_t = np.arange(n - a_n) / SAMPLE_RATE
        out[a_n:] = np.exp(-decay_t / max(tau, 1e-6))
    return out


def white_noise(dur_sec: float, amp: float = 1.0, rng: Optional[np.random.RandomState] = None) -> np.ndarray:
    n = _n_samples(dur_sec)
    rng = rng if rng is not None else np.random.RandomState(0)
    return rng.standard_normal(n) * amp


def pink_noise(dur_sec: float, amp: float = 1.0, rng: Optional[np.random.RandomState] = None) -> np.ndarray:
    n = _n_samples(dur_sec)
    rng = rng if rng is not None else np.random.RandomState(0)
    white = rng.standard_normal(n)
    b = [0.049922035, -0.095993537, 0.050612699, -0.004408786]
    a = [1.0, -2.494956002, 2.017265875, -0.522189400]
    out = sps.lfilter(b, a, white)
    peak = float(np.max(np.abs(out)) + 1e-9)
    return (out / peak) * amp


def brown_noise(dur_sec: float, amp: float = 1.0, rng: Optional[np.random.RandomState] = None) -> np.ndarray:
    n = _n_samples(dur_sec)
    rng = rng if rng is not None else np.random.RandomState(0)
    white = rng.standard_normal(n)
    out = np.cumsum(white)
    out = out - np.mean(out)
    peak = float(np.max(np.abs(out)) + 1e-9)
    return (out / peak) * amp


def _butter(cut: float, kind: str, order: int = 4) -> tuple[np.ndarray, np.ndarray]:
    nyq = 0.5 * SAMPLE_RATE
    normalized = max(min(cut / nyq, 0.999), 0.001)
    return sps.butter(order, normalized, btype=kind)


def low_pass(samples: np.ndarray, cutoff: float, order: int = 4) -> np.ndarray:
    b, a = _butter(cutoff, "low", order)
    return sps.filtfilt(b, a, samples)


def high_pass(samples: np.ndarray, cutoff: float, order: int = 4) -> np.ndarray:
    b, a = _butter(cutoff, "high", order)
    return sps.filtfilt(b, a, samples)


def band_filter(samples: np.ndarray, f_low: float, f_high: float, order: int = 4) -> np.ndarray:
    nyq = 0.5 * SAMPLE_RATE
    low = max(min(f_low / nyq, 0.999), 0.001)
    high = max(min(f_high / nyq, 0.999), 0.001)
    if high <= low:
        high = min(low + 0.01, 0.999)
    b, a = sps.butter(order, [low, high], btype="band")
    return sps.filtfilt(b, a, samples)


def sine(dur_sec: float, freq: float, amp: float = 1.0, phase: float = 0.0) -> np.ndarray:
    t = t_axis(dur_sec)
    return np.sin(2.0 * np.pi * freq * t + phase) * amp


def square(dur_sec: float, freq: float, amp: float = 1.0) -> np.ndarray:
    t = t_axis(dur_sec)
    return np.sign(np.sin(2.0 * np.pi * freq * t)) * amp


def saw(dur_sec: float, freq: float, amp: float = 1.0) -> np.ndarray:
    t = t_axis(dur_sec)
    return sps.sawtooth(2.0 * np.pi * freq * t) * amp


def fm_synth(dur_sec: float, carrier_hz: float, mod_hz: float, mod_index: float, amp: float = 1.0) -> np.ndarray:
    t = t_axis(dur_sec)
    return np.sin(2.0 * np.pi * carrier_hz * t + mod_index * np.sin(2.0 * np.pi * mod_hz * t)) * amp


def chirp(dur_sec: float, f_start: float, f_end: float, amp: float = 1.0, method: str = "linear") -> np.ndarray:
    t = t_axis(dur_sec)
    return sps.chirp(t, f0=f_start, f1=f_end, t1=dur_sec, method=method) * amp


def transient_click(dur_sec: float = 0.005, amp: float = 1.0) -> np.ndarray:
    n = _n_samples(dur_sec)
    out = np.zeros(n)
    if n > 0:
        out[0] = amp
        if n >= 2:
            out[1] = -amp * 0.7
        if n >= 3:
            out[2] = amp * 0.3
    return out


def burst_noise(dur_sec: float, color: str = "white", amp: float = 1.0, rng: Optional[np.random.RandomState] = None) -> np.ndarray:
    if color == "white":
        return white_noise(dur_sec, amp, rng)
    if color == "pink":
        return pink_noise(dur_sec, amp, rng)
    if color == "brown":
        return brown_noise(dur_sec, amp, rng)
    return white_noise(dur_sec, amp, rng)


def pitch_envelope(dur_sec: float, f_start: float, f_end: float, curve: str = "linear", amp: float = 1.0) -> np.ndarray:
    t = t_axis(dur_sec)
    if curve == "exp" and f_start > 0 and f_end > 0:
        freqs = f_start * np.power(f_end / f_start, t / max(dur_sec, 1e-6))
    else:
        freqs = f_start + (f_end - f_start) * (t / max(dur_sec, 1e-6))
    phase = 2.0 * np.pi * np.cumsum(freqs) / SAMPLE_RATE
    return np.sin(phase) * amp


def mix(*tracks: np.ndarray, normalize: bool = True) -> np.ndarray:
    if not tracks:
        return np.zeros(0)
    max_len = max(len(t) for t in tracks)
    out = np.zeros(max_len)
    for tr in tracks:
        if len(tr) == 0:
            continue
        out[:len(tr)] += tr
    if normalize:
        peak = float(np.max(np.abs(out)) + 1e-9)
        if peak > 1.0:
            out = out / peak
    return out


def overlay_at(base: np.ndarray, layer: np.ndarray, start_sample: int) -> np.ndarray:
    if start_sample < 0:
        start_sample = 0
    end = start_sample + len(layer)
    if end > len(base):
        base = np.concatenate([base, np.zeros(end - len(base))])
    base[start_sample:end] += layer
    return base


def reverb_simple(samples: np.ndarray, decay: float = 0.4, density: int = 12, rng: Optional[np.random.RandomState] = None) -> np.ndarray:
    rng = rng if rng is not None else np.random.RandomState(0)
    out = samples.astype(np.float64).copy()
    n = len(samples)
    for _ in range(density):
        delay_ms = float(rng.uniform(5.0, 80.0))
        delay = int(delay_ms * SAMPLE_RATE / 1000.0)
        gain = decay * float(rng.uniform(0.4, 0.95)) * math.exp(-delay_ms / 80.0)
        if delay < n:
            shifted = np.zeros(n)
            shifted[delay:] = samples[:n - delay] * gain
            out = out + shifted
    peak = float(np.max(np.abs(out)) + 1e-9)
    if peak > 1.0:
        out = out / peak
    return out


def normalize_peak(samples: np.ndarray, peak_dbfs: float = -12.0) -> np.ndarray:
    target = 10.0 ** (peak_dbfs / 20.0)
    peak = float(np.max(np.abs(samples)) + 1e-9)
    if peak <= 1e-7:
        return samples
    return samples * (target / peak)


def fade_in_out(samples: np.ndarray, fade_ms: float = 5.0) -> np.ndarray:
    n = len(samples)
    if n == 0:
        return samples
    fade_n = min(int(fade_ms * SAMPLE_RATE / 1000.0), n // 2)
    if fade_n <= 0:
        return samples
    out = samples.copy()
    fade_in = np.linspace(0.0, 1.0, fade_n)
    out[:fade_n] *= fade_in
    out[-fade_n:] *= fade_in[::-1]
    return out


def loop_align(samples: np.ndarray, fade_ms: float = 50.0) -> np.ndarray:
    n = len(samples)
    if n == 0:
        return samples
    fade_n = min(int(fade_ms * SAMPLE_RATE / 1000.0), n // 4)
    if fade_n <= 0:
        return samples
    head = samples[:fade_n].copy()
    tail = samples[-fade_n:].copy()
    blend = np.linspace(0.0, 1.0, fade_n)
    cross = tail * (1.0 - blend) + head * blend
    out = samples.copy()
    out[:fade_n] = cross
    out[-fade_n:] = cross
    return out


def amplitude_lfo(samples: np.ndarray, rate_hz: float, depth: float = 0.3) -> np.ndarray:
    n = len(samples)
    t = np.arange(n) / SAMPLE_RATE
    lfo = 1.0 - depth + depth * np.cos(2.0 * np.pi * rate_hz * t)
    return samples * lfo


def crackle_pattern(dur_sec: float, density_per_sec: float = 10.0, peak: float = 0.6, rng: Optional[np.random.RandomState] = None) -> np.ndarray:
    n = _n_samples(dur_sec)
    rng = rng if rng is not None else np.random.RandomState(0)
    out = np.zeros(n)
    expected = max(1, int(dur_sec * density_per_sec))
    positions = rng.randint(0, n, size=expected)
    for p in positions:
        amp = float(rng.uniform(0.3, 1.0)) * peak
        out[p] += amp
        if p + 1 < n:
            out[p + 1] -= amp * 0.6
    return out


def random_transients(dur_sec: float, count: int, min_amp: float = 0.4, max_amp: float = 0.9, rng: Optional[np.random.RandomState] = None) -> np.ndarray:
    n = _n_samples(dur_sec)
    rng = rng if rng is not None else np.random.RandomState(0)
    out = np.zeros(n)
    if count <= 0 or n <= 0:
        return out
    positions = rng.randint(0, max(1, n - 4), size=count)
    for p in positions:
        amp = float(rng.uniform(min_amp, max_amp))
        out[p] += amp
        if p + 1 < n:
            out[p + 1] -= amp * 0.7
    return out


def write_wav(path: Path, samples: np.ndarray) -> int:
    samples = np.clip(samples, -1.0, 1.0)
    pcm = (samples * 32767.0).astype(np.int16)
    sf.write(str(path), pcm, SAMPLE_RATE, subtype="PCM_16", format="WAV")
    return path.stat().st_size


def ensure_duration(samples: np.ndarray, dur_sec: float) -> np.ndarray:
    target = _n_samples(dur_sec)
    if len(samples) == target:
        return samples
    if len(samples) > target:
        return samples[:target]
    return np.concatenate([samples, np.zeros(target - len(samples))])


def voice_formant(
    dur_sec: float,
    f0: float,
    formants: Sequence[float],
    vibrato_hz: float = 5.0,
    vibrato_depth_hz: float = 4.0,
    rng: Optional[np.random.RandomState] = None,
) -> np.ndarray:
    """Build a vocoded-ish formant tone (no actual speech synthesis)."""
    rng = rng if rng is not None else np.random.RandomState(0)
    t = t_axis(dur_sec)
    vibrato = vibrato_depth_hz * np.sin(2.0 * np.pi * vibrato_hz * t)
    freqs = f0 + vibrato
    phase = 2.0 * np.pi * np.cumsum(freqs) / SAMPLE_RATE
    base = np.sin(phase) * 0.3
    for fm in formants:
        ratio = max(fm / max(f0, 1.0), 1.0)
        base = base + np.sin(phase * ratio) * 0.25 / ratio
    noise = white_noise(dur_sec, 0.05, rng)
    out = base + noise
    out = band_filter(out, max(80.0, f0 * 0.5), min(20000.0, formants[-1] * 1.5))
    return out


__all__ = [
    "SAMPLE_RATE",
    "amplitude_lfo",
    "band_filter",
    "brown_noise",
    "burst_noise",
    "chirp",
    "crackle_pattern",
    "ensure_duration",
    "envelope_adsr",
    "envelope_attack_decay",
    "envelope_exp_decay",
    "fade_in_out",
    "fm_synth",
    "high_pass",
    "loop_align",
    "low_pass",
    "mix",
    "normalize_peak",
    "overlay_at",
    "pink_noise",
    "pitch_envelope",
    "random_transients",
    "reverb_simple",
    "saw",
    "silence",
    "sine",
    "square",
    "t_axis",
    "transient_click",
    "voice_formant",
    "white_noise",
    "write_wav",
]
