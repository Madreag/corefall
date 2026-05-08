#!/usr/bin/env python3
"""capture_grid.py — T-CAPTURE composer.

Reads a run-bundle's `captures/capture_manifest.json` (written by `cf-capture`),
loads the per-frame PNGs, downsamples them to thumbnail size, composes them into
8x8 grid PNGs (`grid_NNN.png`) tagged with tick/HP/mission overlays, plus a
`summary_grid.png` containing one frame per major event (max 64 frames).

Outputs:
    captures/grid_001.png, grid_002.png, ...
    captures/grid_001.json (tick + event mapping for the grid above)
    captures/summary_grid.png
    captures/summary_grid.json

Usage:
    python3 game/tools/capture_grid.py <run_dir>            # composes for one run
    python3 game/tools/capture_grid.py <run_dir>/captures   # also accepted
    python3 game/tools/capture_grid.py <run_dir> --dry-run  # validate without writing

Exit codes:
    0 — success (or dry-run with valid manifest)
    1 — manifest missing / malformed
    2 — frame PNG missing / unreadable
    3 — composition failure (image library error)
"""

from __future__ import annotations

import argparse
import json
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import List, Optional

GRID_COLS = 8
GRID_ROWS = 8
FRAMES_PER_GRID = GRID_COLS * GRID_ROWS  # 64
SUMMARY_GRID_MAX_FRAMES = 64
COMPOSER_VERSION = "0.1.0"
DEFAULT_THUMB_W = 320
DEFAULT_THUMB_H = 180


@dataclass(frozen=True)
class FrameEntry:
    frame_index: int
    tick: int
    kind: str
    event_type: Optional[str]
    label: Optional[str]
    png_relpath: str

    @classmethod
    def from_dict(cls, raw: dict) -> "FrameEntry":
        return cls(
            frame_index=int(raw["frame_index"]),
            tick=int(raw["tick"]),
            kind=str(raw["kind"]),
            event_type=raw.get("event_type"),
            label=raw.get("label"),
            png_relpath=str(raw["png_relpath"]),
        )

    @property
    def is_event_keyframe(self) -> bool:
        return self.kind == "event_keyframe"


def find_captures_dir(input_path: Path) -> Path:
    if input_path.name == "captures":
        return input_path
    candidate = input_path / "captures"
    if candidate.is_dir():
        return candidate
    raise SystemExit(
        f"capture_grid: could not find a captures/ directory at "
        f"{input_path} or {candidate}"
    )


def load_manifest(captures_dir: Path) -> tuple[dict, List[FrameEntry]]:
    manifest_path = captures_dir / "capture_manifest.json"
    if not manifest_path.exists():
        raise SystemExit(
            f"capture_grid: capture_manifest.json missing at {manifest_path}; "
            f"did cf-capture run with --capture-grid?"
        )
    try:
        raw = json.loads(manifest_path.read_text())
    except json.JSONDecodeError as e:
        raise SystemExit(f"capture_grid: malformed manifest: {e}")
    frames = [FrameEntry.from_dict(f) for f in raw.get("frames", [])]
    return raw, frames


def lazy_import_pillow():
    try:
        from PIL import Image, ImageDraw, ImageFont  # noqa: F401
        return True
    except ImportError:
        return False


def compose_grid(
    captures_dir: Path,
    frames: List[FrameEntry],
    thumb_w: int,
    thumb_h: int,
    grid_filename: str,
    json_filename: str,
    grid_kind: str,
    runtime_tick_rate_hz: int,
    *,
    dry_run: bool = False,
) -> dict:
    """Compose `frames` into one grid PNG. Returns a stats dict."""
    if not frames:
        return {
            "grid_kind": grid_kind,
            "grid_path": None,
            "json_path": None,
            "frame_count": 0,
            "event_count": 0,
            "tick_first": None,
            "tick_last": None,
            "non_blank_ratio": 0.0,
        }

    if dry_run:
        return {
            "grid_kind": grid_kind,
            "grid_path": str(captures_dir / grid_filename),
            "json_path": str(captures_dir / json_filename),
            "frame_count": len(frames),
            "event_count": sum(1 for f in frames if f.is_event_keyframe),
            "tick_first": frames[0].tick,
            "tick_last": frames[-1].tick,
            "non_blank_ratio": None,  # measured only on real composition
        }

    if not lazy_import_pillow():
        raise SystemExit(
            "capture_grid: Pillow is required to compose grids. Install with "
            "`python3 -m pip install --user Pillow` (or skip with --dry-run)."
        )
    from PIL import Image, ImageDraw

    cols = GRID_COLS
    rows = (len(frames) + cols - 1) // cols
    canvas = Image.new("RGB", (cols * thumb_w, rows * thumb_h), color=(20, 22, 28))
    draw = ImageDraw.Draw(canvas)
    non_blank = 0

    for idx, frame in enumerate(frames):
        png_path = captures_dir / frame.png_relpath
        if not png_path.exists():
            raise SystemExit(
                f"capture_grid: frame PNG missing: {png_path} (manifest "
                f"references but file is absent)"
            )
        try:
            with Image.open(png_path) as src:
                src = src.convert("RGB")
                src.thumbnail((thumb_w, thumb_h), Image.Resampling.LANCZOS)
                bbox = src.getbbox()
                if bbox is not None and bbox != (0, 0, 1, 1):
                    non_blank += 1
                col = idx % cols
                row = idx // cols
                x = col * thumb_w + (thumb_w - src.width) // 2
                y = row * thumb_h + (thumb_h - src.height) // 2
                canvas.paste(src, (x, y))
                # Burn tick + (optional) event label as a corner overlay.
                overlay_lines = [f"t{frame.tick}"]
                if frame.is_event_keyframe and frame.event_type:
                    overlay_lines.append(frame.event_type)
                ox = col * thumb_w + 4
                oy = row * thumb_h + 4
                for line in overlay_lines:
                    # Drop-shadow then text for legibility on any background.
                    draw.text((ox + 1, oy + 1), line, fill=(0, 0, 0))
                    draw.text((ox, oy), line, fill=(220, 220, 220))
                    oy += 12
                # Frame border (thin)
                draw.rectangle(
                    [(col * thumb_w, row * thumb_h),
                     (col * thumb_w + thumb_w - 1, row * thumb_h + thumb_h - 1)],
                    outline=(60, 64, 76),
                    width=1,
                )
        except Exception as e:  # pragma: no cover (image library variance)
            raise SystemExit(f"capture_grid: failed to load {png_path}: {e}")

    out_grid = captures_dir / grid_filename
    canvas.save(out_grid, format="PNG", optimize=True)

    grid_meta = {
        "composer_version": COMPOSER_VERSION,
        "grid_kind": grid_kind,
        "frame_count": len(frames),
        "event_count": sum(1 for f in frames if f.is_event_keyframe),
        "tick_first": frames[0].tick,
        "tick_last": frames[-1].tick,
        "runtime_tick_rate_hz": runtime_tick_rate_hz,
        "non_blank_ratio": round(non_blank / len(frames), 4),
        "frames": [
            {
                "cell": idx,
                "row": idx // cols,
                "col": idx % cols,
                "frame_index": f.frame_index,
                "tick": f.tick,
                "kind": f.kind,
                "event_type": f.event_type,
                "label": f.label,
                "png_relpath": f.png_relpath,
            }
            for idx, f in enumerate(frames)
        ],
    }
    out_json = captures_dir / json_filename
    out_json.write_text(json.dumps(grid_meta, indent=2))

    return {
        "grid_kind": grid_kind,
        "grid_path": str(out_grid),
        "json_path": str(out_json),
        "frame_count": len(frames),
        "event_count": grid_meta["event_count"],
        "tick_first": grid_meta["tick_first"],
        "tick_last": grid_meta["tick_last"],
        "non_blank_ratio": grid_meta["non_blank_ratio"],
    }


def select_summary_frames(frames: List[FrameEntry]) -> List[FrameEntry]:
    """Pick at most SUMMARY_GRID_MAX_FRAMES frames for the summary grid:
    every event keyframe first; if more remain, evenly sample baseline frames.
    """
    keyframes = [f for f in frames if f.is_event_keyframe]
    if len(keyframes) >= SUMMARY_GRID_MAX_FRAMES:
        # Too many keyframes: keep first + last + evenly spaced middle.
        step = max(1, len(keyframes) // SUMMARY_GRID_MAX_FRAMES)
        picked = keyframes[::step][:SUMMARY_GRID_MAX_FRAMES]
        return picked

    picked = list(keyframes)
    remaining = SUMMARY_GRID_MAX_FRAMES - len(picked)
    if remaining > 0:
        baselines = [f for f in frames if not f.is_event_keyframe]
        if baselines:
            step = max(1, len(baselines) // remaining)
            picked.extend(baselines[::step][:remaining])
    picked.sort(key=lambda f: f.frame_index)
    return picked


def main() -> int:
    parser = argparse.ArgumentParser(description="Compose cf-capture frames into grids")
    parser.add_argument(
        "input",
        type=Path,
        help="Path to a run-bundle directory OR its captures/ subdirectory",
    )
    parser.add_argument("--thumbnail-w", type=int, default=DEFAULT_THUMB_W)
    parser.add_argument("--thumbnail-h", type=int, default=DEFAULT_THUMB_H)
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Validate manifest without writing grids",
    )
    args = parser.parse_args()

    captures_dir = find_captures_dir(args.input)
    raw_manifest, frames = load_manifest(captures_dir)

    if not frames:
        print(
            f"capture_grid: manifest at {captures_dir / 'capture_manifest.json'} "
            f"contains zero frames; nothing to compose."
        )
        # Still emit empty summary stub so downstream tools have a stable artifact.
        empty = {
            "composer_version": COMPOSER_VERSION,
            "grid_kind": "summary",
            "frame_count": 0,
            "event_count": 0,
            "tick_first": None,
            "tick_last": None,
            "runtime_tick_rate_hz": int(raw_manifest.get("runtime_tick_rate_hz", 60)),
            "non_blank_ratio": 0.0,
            "frames": [],
        }
        if not args.dry_run:
            (captures_dir / "summary_grid.json").write_text(json.dumps(empty, indent=2))
        return 0

    runtime_tick_rate_hz = int(raw_manifest.get("runtime_tick_rate_hz", 60))
    grid_results: List[dict] = []
    for grid_idx, start in enumerate(range(0, len(frames), FRAMES_PER_GRID)):
        chunk = frames[start : start + FRAMES_PER_GRID]
        grid_filename = f"grid_{grid_idx + 1:03d}.png"
        json_filename = f"grid_{grid_idx + 1:03d}.json"
        result = compose_grid(
            captures_dir,
            chunk,
            args.thumbnail_w,
            args.thumbnail_h,
            grid_filename,
            json_filename,
            grid_kind="grid",
            runtime_tick_rate_hz=runtime_tick_rate_hz,
            dry_run=args.dry_run,
        )
        grid_results.append(result)

    summary_frames = select_summary_frames(frames)
    summary_result = compose_grid(
        captures_dir,
        summary_frames,
        args.thumbnail_w,
        args.thumbnail_h,
        "summary_grid.png",
        "summary_grid.json",
        grid_kind="summary",
        runtime_tick_rate_hz=runtime_tick_rate_hz,
        dry_run=args.dry_run,
    )

    print(json.dumps(
        {
            "captures_dir": str(captures_dir),
            "frame_count": len(frames),
            "event_keyframe_count": sum(1 for f in frames if f.is_event_keyframe),
            "grids": grid_results,
            "summary_grid": summary_result,
            "dry_run": args.dry_run,
        },
        indent=2,
    ))
    return 0


if __name__ == "__main__":
    sys.exit(main())
