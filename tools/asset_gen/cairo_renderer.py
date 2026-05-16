"""SVG → PNG renderer for the M9A Tier-1 pipeline.

Uses cairosvg (vector → PNG raster) and Pillow (post-processing). Deterministic
given identical input SVG bytes + identical output dimensions. Pinned versions
are listed in `tools/asset_gen/requirements.txt`.

The renderer never writes a partial file — it stages to a tempfile and renames
on success — so a Ctrl-C mid-bake leaves a coherent on-disk state for the
build_placeholders.py resume path.
"""

from __future__ import annotations

import io
import os
from pathlib import Path
from typing import Iterable, List, Tuple

import cairosvg
from PIL import Image


def render_to_png(svg_bytes: bytes, output_path: Path, size_px: int) -> int:
    """Render the SVG bytes to a PNG at `output_path` at `size_px` × `size_px`.

    Returns the number of bytes written. Output path's parent is created if
    missing. Uses a staged tempfile-then-rename so partial writes are
    impossible.
    """
    if size_px <= 0:
        raise ValueError(f"size_px must be > 0, got {size_px}")
    output_path.parent.mkdir(parents=True, exist_ok=True)
    raw = cairosvg.svg2png(
        bytestring=svg_bytes,
        output_width=size_px,
        output_height=size_px,
        background_color=None,
    )
    if raw is None:
        raise RuntimeError("cairosvg returned None for non-empty SVG input")
    # Re-encode through Pillow to normalize PNG metadata (PIL writes a
    # deterministic chunk ordering; cairosvg's libpng output can include
    # tIME / iDOT chunks that drift across machines).
    img = Image.open(io.BytesIO(raw))
    if img.mode != "RGBA":
        img = img.convert("RGBA")
    staging = output_path.with_suffix(output_path.suffix + ".tmp")
    img.save(staging, format="PNG", optimize=False, compress_level=6)
    os.replace(staging, output_path)
    return output_path.stat().st_size


def render_many(svg_bytes: bytes, output_dir: Path, base_name: str,
                sizes: Iterable[int]) -> List[Tuple[Path, int, int]]:
    """Render multiple PNG sizes for a single SVG.

    Each output is written as `<base_name>_<size>.png`. Returns a list of
    (path, size_px, size_bytes) tuples in the order the sizes were emitted.
    """
    results: List[Tuple[Path, int, int]] = []
    for size in sizes:
        out_path = output_dir / f"{base_name}_{size}.png"
        n = render_to_png(svg_bytes, out_path, size)
        results.append((out_path, size, n))
    return results


__all__ = ["render_to_png", "render_many"]
