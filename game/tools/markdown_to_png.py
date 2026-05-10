#!/usr/bin/env python3
"""Render a markdown file (or stdin) as a PNG using Pillow.

The M3B replay viewer (`cf-tools-replay-viewer`) emits markdown for every
view / cause-chain / debrief query. The roadmap + backlog evidence targets
("Viewer capture in bundle", "Death/failure recap screenshot") require a
PNG companion to the markdown. Rather than add a heavy GUI dep
(bevy_egui, eframe, etc.) just to satisfy the screenshot evidence, we
render the markdown content as a fixed-width text PNG. The result is
visually faithful to the CLI output and reviewable as a single image.

This mirrors `capture_grid.py`'s pattern (Python + Pillow for image work
that does not need to live in a Rust hot path).

Usage:
    markdown_to_png.py <md_path> [--output PNG] [--width N] [--font-size N]
    cat report.md | markdown_to_png.py - --output report.png

The output PNG is a deterministic function of the input markdown + the
rendering parameters. cf-tools-replay-viewer's deterministic markdown
output combined with this deterministic renderer means the PNG can be
golden-tested offline.

Exit codes:
    0 on success
    1 on file IO / Pillow failure
    2 on invalid arguments
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

try:
    from PIL import Image, ImageDraw, ImageFont
except ImportError:  # pragma: no cover
    print(
        "markdown_to_png.py requires Pillow. Install with: python3 -m pip install Pillow",
        file=sys.stderr,
    )
    sys.exit(1)


DEFAULT_WIDTH = 1100
DEFAULT_FONT_SIZE = 14
LINE_PAD = 4
MARGIN = 24


# Hard-coded fallback font search list. macOS / Linux defaults plus a few
# common monospace TTFs. We pick the first existing font; if none are
# found, fall back to Pillow's bitmap default (which renders fine but is
# tiny). Audit reviewers can override with --font-path.
DEFAULT_FONT_CANDIDATES = (
    # macOS
    "/System/Library/Fonts/Menlo.ttc",
    "/System/Library/Fonts/Monaco.dfont",
    "/Library/Fonts/Andale Mono.ttf",
    "/System/Library/Fonts/SFNSMono.ttf",
    "/Library/Fonts/Arial Unicode.ttf",
    # Linux
    "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    "/usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf",
    # Common
    "/usr/share/fonts/TTF/DejaVuSansMono.ttf",
)


def find_font(font_size: int, override_path: str | None) -> ImageFont.ImageFont:
    if override_path:
        try:
            return ImageFont.truetype(override_path, font_size)
        except (OSError, IOError) as e:
            print(f"font path {override_path!r} not loadable: {e}", file=sys.stderr)
            sys.exit(1)
    for candidate in DEFAULT_FONT_CANDIDATES:
        if Path(candidate).exists():
            try:
                return ImageFont.truetype(candidate, font_size)
            except (OSError, IOError):
                continue
    return ImageFont.load_default()


def render(
    markdown: str,
    output: Path,
    width: int,
    font_size: int,
    font_path: str | None,
) -> None:
    font = find_font(font_size, font_path)
    # Use the bbox of an "X" to estimate line height for monospace.
    try:
        ascent, descent = font.getmetrics()
        line_height = ascent + descent + LINE_PAD
    except AttributeError:
        # ImageFont.load_default() in older Pillow lacks getmetrics().
        line_height = font_size + LINE_PAD

    lines = markdown.splitlines() or [""]
    img_w = width
    img_h = MARGIN * 2 + line_height * len(lines)
    img = Image.new("RGB", (img_w, img_h), color=(20, 22, 26))
    draw = ImageDraw.Draw(img)
    text_color = (220, 225, 230)
    heading_color = (255, 200, 120)
    sub_heading_color = (160, 200, 255)
    bold_color = (255, 240, 240)
    table_color = (180, 230, 200)
    cause_color = (240, 200, 240)

    y = MARGIN
    for raw in lines:
        line = raw.rstrip()
        color = text_color
        if line.startswith("# "):
            color = heading_color
        elif line.startswith("## "):
            color = sub_heading_color
        elif line.startswith("### "):
            color = sub_heading_color
        elif line.startswith("|"):
            color = table_color
        elif line.startswith("→") or line.startswith(" "):
            color = cause_color
        elif line.startswith("- ") or line.startswith("* "):
            color = bold_color
        # Pillow doesn't word-wrap; we draw the line as-is. cf-tools-replay-viewer
        # output is reasonably narrow (most tail rows fit in 1100 px at 14pt).
        draw.text((MARGIN, y), line, font=font, fill=color)
        y += line_height

    output.parent.mkdir(parents=True, exist_ok=True)
    img.save(output, "PNG")


def main() -> int:
    p = argparse.ArgumentParser(
        description="Render a markdown file as a PNG using Pillow."
    )
    p.add_argument(
        "input",
        help="Path to a markdown file, or '-' to read from stdin.",
    )
    p.add_argument("--output", required=True, type=Path)
    p.add_argument("--width", type=int, default=DEFAULT_WIDTH)
    p.add_argument("--font-size", type=int, default=DEFAULT_FONT_SIZE)
    p.add_argument(
        "--font-path",
        default=None,
        help="Override the auto-discovered TTF font path.",
    )
    args = p.parse_args()

    if args.input == "-":
        markdown = sys.stdin.read()
    else:
        try:
            markdown = Path(args.input).read_text()
        except (OSError, UnicodeDecodeError) as e:
            print(f"failed to read {args.input!r}: {e}", file=sys.stderr)
            return 1

    try:
        render(markdown, args.output, args.width, args.font_size, args.font_path)
    except Exception as e:  # pragma: no cover
        print(f"render failed: {e}", file=sys.stderr)
        return 1
    print(f"PNG written to {args.output}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
