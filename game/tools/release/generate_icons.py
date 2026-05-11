#!/usr/bin/env python3
"""generate_icons.py — produce the placeholder Corefall icons.

Generates:
- corefall.png (1024x1024 master, also exported at 256x256 for Linux .desktop).
- icon.iconset/ (multi-size PNG set for `iconutil -c icns`).
- corefall.icns (only when run on macOS where `iconutil` exists).

The icon is a deliberately-simple solid orange square with a white "CF"
glyph. It exists to satisfy the Hard Gate requirement that the macOS
.app bundle ships an icon resource. A polished icon arrives in a future
BP that owns marketing assets.
"""

from __future__ import annotations

import shutil
import subprocess
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

HERE = Path(__file__).resolve().parent

ORANGE = (235, 110, 42)
WHITE = (255, 255, 255)
GLYPH = "CF"
SIZES = [16, 32, 64, 128, 256, 512, 1024]


def _font(size: int) -> ImageFont.FreeTypeFont:
    candidates = [
        "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
        "/System/Library/Fonts/Supplemental/Arial.ttf",
        "/System/Library/Fonts/Helvetica.ttc",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf",
        "/Library/Fonts/Arial Bold.ttf",
    ]
    for path in candidates:
        if Path(path).exists():
            try:
                return ImageFont.truetype(path, int(size * 0.5))
            except Exception:
                continue
    return ImageFont.load_default()


def make_square(size: int) -> Image.Image:
    img = Image.new("RGBA", (size, size), ORANGE + (255,))
    draw = ImageDraw.Draw(img)
    border = max(2, size // 32)
    draw.rectangle(
        [border, border, size - border - 1, size - border - 1],
        outline=WHITE + (255,),
        width=max(1, size // 96),
    )
    font = _font(size)
    bbox = draw.textbbox((0, 0), GLYPH, font=font)
    glyph_w = bbox[2] - bbox[0]
    glyph_h = bbox[3] - bbox[1]
    x = (size - glyph_w) // 2 - bbox[0]
    y = (size - glyph_h) // 2 - bbox[1]
    draw.text((x, y), GLYPH, fill=WHITE + (255,), font=font)
    return img


def main() -> int:
    iconset = HERE / "icon.iconset"
    if iconset.exists():
        shutil.rmtree(iconset)
    iconset.mkdir(parents=True)

    master = make_square(1024)
    master.save(HERE / "corefall.png", format="PNG")
    make_square(256).save(HERE / "corefall_256.png", format="PNG")

    naming = {
        16: ("icon_16x16.png", "icon_16x16@2x.png"),
        32: ("icon_32x32.png", "icon_32x32@2x.png"),
        128: ("icon_128x128.png", "icon_128x128@2x.png"),
        256: ("icon_256x256.png", "icon_256x256@2x.png"),
        512: ("icon_512x512.png", "icon_512x512@2x.png"),
    }
    written = {}
    for size in SIZES:
        png = make_square(size)
        written[size] = png

    pairs = [
        (16, written[16], "icon_16x16.png"),
        (32, written[32], "icon_16x16@2x.png"),
        (32, written[32], "icon_32x32.png"),
        (64, written[64], "icon_32x32@2x.png"),
        (128, written[128], "icon_128x128.png"),
        (256, written[256], "icon_128x128@2x.png"),
        (256, written[256], "icon_256x256.png"),
        (512, written[512], "icon_256x256@2x.png"),
        (512, written[512], "icon_512x512.png"),
        (1024, written[1024], "icon_512x512@2x.png"),
    ]
    for _size, img, name in pairs:
        img.save(iconset / name, format="PNG")

    icns_path = HERE / "corefall.icns"
    if shutil.which("iconutil"):
        subprocess.run(
            ["iconutil", "-c", "icns", str(iconset), "-o", str(icns_path)],
            check=True,
        )
        print(f"wrote {icns_path}")
    else:
        print("iconutil not on PATH; skipped .icns generation. Run on macOS to refresh.")

    print(f"wrote {HERE / 'corefall.png'}")
    print(f"wrote {HERE / 'corefall_256.png'}")
    print(f"wrote iconset at {iconset}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
