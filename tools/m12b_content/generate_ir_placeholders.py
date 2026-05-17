#!/usr/bin/env python3
"""Generate the M12B impulse-response placeholder set.

Per M12B spec § Files:

> 8 IRs — bunker_small_steel / bunker_med_concrete / warehouse_large /
> cave_natural / fabric_lined / glass_lab / open_outdoor / vacuum_null

Each IR is a minimal RIFF/WAVE container with a deterministic
single-tap impulse + a per-IR decay envelope. The cf-app reverb-send
adapter loads these at startup and convolves the wet send with them.

To regenerate:

    python3 tools/m12b_content/generate_ir_placeholders.py
"""

from __future__ import annotations

import math
import struct
import sys
from pathlib import Path

# Per-IR decay-tail profile (length_seconds, decay_per_sample, gain).
IRS = [
    # name, tail_seconds, peak_gain, decay_rate
    ("bunker_small_steel", 0.22, 0.92, 25.0),
    ("bunker_med_concrete", 0.85, 0.85, 5.0),
    ("warehouse_large", 2.1, 0.85, 2.0),
    ("cave_natural", 1.3, 0.55, 3.0),
    ("fabric_lined", 0.20, 0.10, 30.0),
    ("glass_lab", 0.40, 0.70, 10.0),
    ("open_outdoor", 0.01, 0.0, 100.0),
    ("vacuum_null", 0.0, 0.0, 1.0),
]

SAMPLE_RATE = 22_050  # Hz — short, mono, sufficient for late-reflection IRs.
BITS = 32  # f32 PCM (WAVE_FORMAT_IEEE_FLOAT).

OUTPUT_ROOT = (
    Path(__file__).resolve().parents[2] / "game" / "content" / "audio" / "reverb" / "impulse_responses"
)


def render_ir(tail_seconds: float, peak_gain: float, decay_rate: float) -> list[float]:
    """Render a single-tap impulse with exponential decay envelope."""
    if tail_seconds <= 0.0:
        return [0.0]
    n = max(1, int(tail_seconds * SAMPLE_RATE))
    samples = []
    for i in range(n):
        t = i / SAMPLE_RATE
        env = peak_gain * math.exp(-decay_rate * t)
        sign = 1.0 if (i % 2 == 0) else -1.0
        samples.append(env * sign)
    return samples


def write_wave(path: Path, samples: list[float]) -> int:
    """Write a minimal RIFF/WAVE float32 mono PCM file."""
    payload = b"".join(struct.pack("<f", s) for s in samples)
    fmt_chunk = struct.pack(
        "<HHIIHH",
        3,         # WAVE_FORMAT_IEEE_FLOAT
        1,         # mono
        SAMPLE_RATE,
        SAMPLE_RATE * 4,
        4,         # block align
        32,        # bits per sample
    )
    chunks = (
        b"fmt " + struct.pack("<I", len(fmt_chunk)) + fmt_chunk
        + b"data" + struct.pack("<I", len(payload)) + payload
    )
    riff = b"RIFF" + struct.pack("<I", 4 + len(chunks)) + b"WAVE" + chunks
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(riff)
    return len(riff)


def main() -> int:
    for name, tail, gain, decay in IRS:
        samples = render_ir(tail, gain, decay)
        out = OUTPUT_ROOT / f"{name}.wav"
        n = write_wave(out, samples)
        print(f"wrote {out.relative_to(Path.cwd())} ({n} bytes, {len(samples)} samples)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
