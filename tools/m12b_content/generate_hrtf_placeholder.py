#!/usr/bin/env python3
"""Generate the M12B placeholder HRIR binary table.

Per M12B spec § HRTF table format:

> `game/content/audio/hrtf/mit_kemar_subset.bin` — fixed-layout binary,
> 32 azimuth × 8 elevation × 2 ears × 128 samples × 4 bytes (f32) ≈
> 256 KB on disk after compression.

This script generates a deterministic placeholder table that the
cf-audio `HrirTable::from_bytes` loader accepts. Each (az, el, ear)
bucket carries a single-tap impulse at sample 0 (value 1.0) — the
HRIR convolution adapter sees a pass-through.

To regenerate (re-run after spec changes):

    python3 tools/m12b_content/generate_hrtf_placeholder.py

The production MIT KEMAR subset will replace this binary; the layout
is identical so the loader doesn't need to change.
"""

from __future__ import annotations

import struct
import sys
from pathlib import Path

AZIMUTH_BUCKETS = 32
ELEVATION_BUCKETS = 8
EARS = 2
SAMPLES = 128
TOTAL_F32 = AZIMUTH_BUCKETS * ELEVATION_BUCKETS * EARS * SAMPLES  # 65 536
TOTAL_BYTES = TOTAL_F32 * 4  # 262 144

OUTPUT = Path(__file__).resolve().parents[2] / "game" / "content" / "audio" / "hrtf" / "mit_kemar_subset.bin"


def main() -> int:
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)

    buf = bytearray(TOTAL_BYTES)
    for az in range(AZIMUTH_BUCKETS):
        for el in range(ELEVATION_BUCKETS):
            for ear in range(EARS):
                # Layout: ((az * EL + el) * EARS + ear) * SAMPLES.
                offset = ((az * ELEVATION_BUCKETS + el) * EARS + ear) * SAMPLES * 4
                # Single-tap impulse at sample 0 = 1.0.
                struct.pack_into("<f", buf, offset, 1.0)

    OUTPUT.write_bytes(bytes(buf))
    print(f"wrote {OUTPUT.relative_to(Path.cwd())} ({TOTAL_BYTES} bytes)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
