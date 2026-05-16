"""Tier-1 normal-map baker for the M9A pipeline.

Approximates a per-pixel normal map from an SVG's rendered alpha channel.
Pure Pillow + numpy-free implementation so the bake stays deterministic and
lightweight. The output is intentionally low-fidelity per the spec —
"Tier 1 quality is approximate; M32A bakes proper normal maps from depth."

Algorithm:

1. Render the SVG to a small alpha buffer (256×256 by default).
2. Treat the alpha channel as a height-field `h(x, y) = alpha(x, y) / 255`.
3. Compute screen-space gradient `(dh/dx, dh/dy)` via central differences.
4. Pack normalized `(dx, dy, dz)` into RGB channels: `R = (dx + 1) / 2 * 255`,
   `G = (dy + 1) / 2 * 255`, `B = ((1 - hypot) ** 0.5 + 1) / 2 * 255`.
5. Emit a 4-channel PNG with the alpha set from the input alpha so the
   normal map is masked.
"""

from __future__ import annotations

import io
import math
import os
from pathlib import Path
from typing import Optional

import cairosvg
from PIL import Image


def bake_normal_map(svg_bytes: bytes, output_path: Path, size_px: int = 256,
                    strength: float = 4.0) -> Optional[int]:
    """Bake a Tier-1 normal map from an SVG's alpha channel.

    `strength` scales the gradient magnitude; default 4.0 gives a visible but
    flat normal-map for the typical Corefall side-view sprite. Returns the
    output file size in bytes (or None if the input rendered to fully empty).
    """
    output_path.parent.mkdir(parents=True, exist_ok=True)
    raw = cairosvg.svg2png(
        bytestring=svg_bytes,
        output_width=size_px,
        output_height=size_px,
        background_color=None,
    )
    if raw is None:
        return None
    img = Image.open(io.BytesIO(raw)).convert("RGBA")
    w, h = img.size
    alpha = img.getchannel("A")
    a_bytes = alpha.tobytes()

    def at(x: int, y: int) -> int:
        x = max(0, min(w - 1, x))
        y = max(0, min(h - 1, y))
        return a_bytes[y * w + x]

    out = Image.new("RGBA", (w, h))
    out_pixels = out.load()
    for y in range(h):
        for x in range(w):
            l = at(x - 1, y)
            r = at(x + 1, y)
            u = at(x, y - 1)
            d = at(x, y + 1)
            dx = (r - l) / 255.0 * strength
            dy = (d - u) / 255.0 * strength
            length = math.sqrt(dx * dx + dy * dy + 1.0)
            nx = dx / length
            ny = dy / length
            nz = 1.0 / length
            R = int((nx + 1.0) * 127.5)
            G = int((1.0 - (ny + 1.0) * 0.5) * 255.0)  # flip Y for OpenGL convention
            B = int((nz + 1.0) * 127.5)
            A = at(x, y)
            out_pixels[x, y] = (max(0, min(255, R)), max(0, min(255, G)), max(0, min(255, B)), A)
    staging = output_path.with_suffix(output_path.suffix + ".tmp")
    out.save(staging, format="PNG", optimize=False, compress_level=6)
    os.replace(staging, output_path)
    return output_path.stat().st_size


__all__ = ["bake_normal_map"]
