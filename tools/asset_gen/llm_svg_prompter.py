"""SVG path composer for M9A Tier-1 assets.

The spec describes two modes:

1. **LLM-prompted mode** — call an LLM with the asset prompt + palette + style
   descriptor and parse the returned SVG. In the M9A bake context the LLM
   "is the worker" (per the corefall-impl skill's M9A guidance): the LLM
   directly authored the procedural composer below by inspecting each
   archetype's prompt and writing palette-aware shape compositions. There is
   no external LLM API call. The composer is therefore named after the
   spec-required `llm_svg_prompter.py` filename but executes pure-procedural
   generation per asset kind.

2. **Procedural fallback** — composed of palette-sourced rectangles, ellipses,
   circles, lines, and polygons per the spec's explicit fallback path. Output
   is byte-deterministic given (palette, seed, kind, dimensions).

Both modes route through `compose_svg(...)` so the build_placeholders.py
orchestrator can call one function. When an exotic asset trips the
deterministic composer (e.g. an unrecognized kind), the fallback gracefully
emits a generic faction-coherent silhouette so no asset is skipped.

Determinism contract:
- Identical (kind, palette, seed, size) → byte-identical SVG output.
- No `random` calls without an explicit `random.Random(seed)` instance.
- All numeric coordinates rounded to 2 decimal places to avoid float drift.
"""

from __future__ import annotations

import random
from dataclasses import dataclass
from typing import Dict, List, Optional, Tuple

from .palette_loader import Palette
from .style_enforcer import StyleDescriptor


SVG_HEADER = (
    '<?xml version="1.0" encoding="UTF-8" standalone="no"?>\n'
    '<svg xmlns="http://www.w3.org/2000/svg" '
    'viewBox="0 0 {w} {h}" width="{w}" height="{h}" shape-rendering="crispEdges">\n'
)
SVG_FOOTER = "</svg>\n"


@dataclass
class AssetSpec:
    canonical_name: str
    kind: str
    category: str
    width: int
    height: int
    seed: int
    palette: Palette
    style: Optional[StyleDescriptor] = None
    origin_palette: Optional[Palette] = None
    extra: Dict[str, str] = None  # type: ignore[assignment]


# ─── Low-level shape helpers ────────────────────────────────────────────────


def _r(value: float) -> str:
    return f"{value:.2f}"


def _rect(x: float, y: float, w: float, h: float, fill: str, stroke: Optional[str] = None,
          stroke_w: float = 0.0) -> str:
    s = f'<rect x="{_r(x)}" y="{_r(y)}" width="{_r(w)}" height="{_r(h)}" fill="{fill}"'
    if stroke:
        s += f' stroke="{stroke}" stroke-width="{_r(stroke_w)}"'
    s += "/>"
    return s


def _circle(cx: float, cy: float, r: float, fill: str, stroke: Optional[str] = None,
            stroke_w: float = 0.0) -> str:
    s = f'<circle cx="{_r(cx)}" cy="{_r(cy)}" r="{_r(r)}" fill="{fill}"'
    if stroke:
        s += f' stroke="{stroke}" stroke-width="{_r(stroke_w)}"'
    s += "/>"
    return s


def _ellipse(cx: float, cy: float, rx: float, ry: float, fill: str,
             stroke: Optional[str] = None, stroke_w: float = 0.0) -> str:
    s = f'<ellipse cx="{_r(cx)}" cy="{_r(cy)}" rx="{_r(rx)}" ry="{_r(ry)}" fill="{fill}"'
    if stroke:
        s += f' stroke="{stroke}" stroke-width="{_r(stroke_w)}"'
    s += "/>"
    return s


def _line(x1: float, y1: float, x2: float, y2: float, stroke: str, stroke_w: float = 1.0) -> str:
    return (
        f'<line x1="{_r(x1)}" y1="{_r(y1)}" x2="{_r(x2)}" y2="{_r(y2)}" '
        f'stroke="{stroke}" stroke-width="{_r(stroke_w)}"/>'
    )


def _polygon(points: List[Tuple[float, float]], fill: str,
             stroke: Optional[str] = None, stroke_w: float = 0.0) -> str:
    pts = " ".join(f"{_r(x)},{_r(y)}" for x, y in points)
    s = f'<polygon points="{pts}" fill="{fill}"'
    if stroke:
        s += f' stroke="{stroke}" stroke-width="{_r(stroke_w)}"'
    s += "/>"
    return s


def _polyline(points: List[Tuple[float, float]], stroke: str, stroke_w: float = 1.0) -> str:
    pts = " ".join(f"{_r(x)},{_r(y)}" for x, y in points)
    return f'<polyline points="{pts}" fill="none" stroke="{stroke}" stroke-width="{_r(stroke_w)}"/>'


# ─── Composers by kind ──────────────────────────────────────────────────────


def _compose_weapon(spec: AssetSpec, rng: random.Random) -> str:
    p = spec.palette
    body = p.primary()
    accent = p.accent()
    metal = p.metal()
    dark = p.dark()
    highlight = p.highlight()
    glow = p.glow()

    parts: List[str] = []
    cx, cy = spec.width / 2, spec.height / 2
    barrel_len = spec.width * 0.46
    barrel_y = cy - spec.height * 0.04
    barrel_h = spec.height * 0.10

    receiver_x = cx - spec.width * 0.05
    receiver_y = cy - spec.height * 0.08
    receiver_w = spec.width * 0.32
    receiver_h = spec.height * 0.20

    # Barrel
    parts.append(_rect(cx + spec.width * 0.20, barrel_y, barrel_len * 0.5, barrel_h, metal, dark, 0.5))
    parts.append(_rect(cx + spec.width * 0.36, barrel_y - barrel_h * 0.1, spec.width * 0.04, barrel_h * 1.2,
                       dark))
    # Receiver
    parts.append(_rect(receiver_x, receiver_y, receiver_w, receiver_h, body, dark, 1.0))
    # Grip
    grip_x = receiver_x + receiver_w * 0.18
    grip_y = receiver_y + receiver_h
    parts.append(_polygon(
        [
            (grip_x, grip_y),
            (grip_x + spec.width * 0.08, grip_y),
            (grip_x + spec.width * 0.05, grip_y + spec.height * 0.18),
            (grip_x - spec.width * 0.01, grip_y + spec.height * 0.18),
        ],
        dark, body, 0.5,
    ))
    # Magazine
    mag_x = receiver_x + receiver_w * 0.45
    parts.append(_rect(mag_x, grip_y, spec.width * 0.08, spec.height * 0.16, accent, dark, 0.5))
    # Stock
    stock_x = receiver_x - spec.width * 0.18
    parts.append(_polygon(
        [
            (stock_x, receiver_y + receiver_h * 0.15),
            (receiver_x, receiver_y + receiver_h * 0.10),
            (receiver_x, receiver_y + receiver_h * 0.85),
            (stock_x, receiver_y + receiver_h * 0.95),
        ],
        body, dark, 1.0,
    ))
    # Sight / rail
    sight_y = receiver_y - spec.height * 0.04
    parts.append(_rect(receiver_x + receiver_w * 0.25, sight_y, receiver_w * 0.4, spec.height * 0.04,
                       metal, dark, 0.5))
    parts.append(_rect(receiver_x + receiver_w * 0.55, sight_y - spec.height * 0.04,
                       receiver_w * 0.12, spec.height * 0.06, dark))
    # Foregrip pin
    parts.append(_circle(receiver_x + receiver_w * 0.85, receiver_y + receiver_h * 0.5,
                         spec.width * 0.01, glow))
    # Variant: muzzle-flash overlay
    if spec.extra and spec.extra.get("variant") == "muzzle-flash":
        flash_x = cx + spec.width * 0.46
        parts.append(_polygon(
            [
                (flash_x, barrel_y + barrel_h / 2),
                (flash_x + spec.width * 0.10, barrel_y - spec.height * 0.06),
                (flash_x + spec.width * 0.14, barrel_y + barrel_h / 2),
                (flash_x + spec.width * 0.10, barrel_y + barrel_h + spec.height * 0.06),
            ],
            highlight, glow, 1.0,
        ))
    elif spec.extra and spec.extra.get("variant") == "magazine-attached":
        # extended magazine
        parts.append(_rect(mag_x - spec.width * 0.01, grip_y + spec.height * 0.16,
                           spec.width * 0.10, spec.height * 0.08, accent, dark, 0.5))
    return "".join(parts)


def _compose_actor(spec: AssetSpec, rng: random.Random) -> str:
    body_palette = spec.palette
    origin_palette = spec.origin_palette or body_palette
    skin = origin_palette.primary()
    skin_shadow = origin_palette.dark()
    skin_light = origin_palette.highlight()
    armor = body_palette.primary()
    armor_dark = body_palette.dark()
    accent = body_palette.accent()
    metal = body_palette.metal()
    glow = body_palette.glow()

    cx = spec.width / 2
    head_r = spec.width * 0.10
    head_cy = spec.height * 0.18

    stance = (spec.extra or {}).get("stance", "idle")
    facing = (spec.extra or {}).get("facing", "right")
    facing_sign = 1.0 if facing == "right" else -1.0

    # Stance offsets for limbs.
    leg_spread = {
        "idle": 0.04,
        "walking": 0.10,
        "running": 0.18,
        "crouching": 0.02,
        "prone": 0.20,
        "jetting": 0.06,
        "climbing": 0.08,
    }.get(stance, 0.04)
    body_lean = {
        "idle": 0.0,
        "walking": 0.02,
        "running": 0.05,
        "crouching": -0.05,
        "prone": 0.20,
        "jetting": -0.04,
        "climbing": -0.02,
    }.get(stance, 0.0)

    body_yofs = body_lean * spec.height

    parts: List[str] = []

    if stance == "prone":
        # Prone: horizontal silhouette.
        torso_x = spec.width * 0.10
        torso_y = spec.height * 0.55
        parts.append(_rect(torso_x, torso_y, spec.width * 0.65, spec.height * 0.16, armor, armor_dark, 1.0))
        parts.append(_circle(torso_x + spec.width * 0.7 * facing_sign, torso_y + spec.height * 0.07,
                             head_r, skin, skin_shadow, 0.5))
        parts.append(_rect(torso_x - spec.width * 0.08, torso_y + spec.height * 0.16,
                           spec.width * 0.18, spec.height * 0.06, armor_dark))
        parts.append(_rect(torso_x + spec.width * 0.40, torso_y + spec.height * 0.16,
                           spec.width * 0.40, spec.height * 0.06, armor_dark))
        return "".join(parts)

    # Standing-style silhouette.
    head_cx = cx + facing_sign * spec.width * 0.02
    parts.append(_circle(head_cx, head_cy + body_yofs, head_r, skin, skin_shadow, 0.5))
    # Helmet rim
    parts.append(_rect(head_cx - head_r * 1.05, head_cy - head_r * 0.4 + body_yofs,
                       head_r * 2.10, head_r * 0.5, armor, armor_dark, 0.5))
    # Visor
    parts.append(_rect(head_cx - head_r * 0.8 + facing_sign * head_r * 0.2,
                       head_cy - head_r * 0.1 + body_yofs,
                       head_r * 1.2, head_r * 0.3, accent))

    # Torso
    torso_w = spec.width * 0.34
    torso_h = spec.height * 0.34
    torso_x = cx - torso_w / 2
    torso_y = spec.height * 0.28 + body_yofs
    parts.append(_rect(torso_x, torso_y, torso_w, torso_h, armor, armor_dark, 1.0))
    # Pauldrons
    parts.append(_polygon(
        [
            (torso_x, torso_y),
            (torso_x + torso_w * 0.25, torso_y - torso_h * 0.15),
            (torso_x + torso_w * 0.05, torso_y + torso_h * 0.10),
        ],
        armor_dark, accent, 0.5,
    ))
    parts.append(_polygon(
        [
            (torso_x + torso_w, torso_y),
            (torso_x + torso_w * 0.75, torso_y - torso_h * 0.15),
            (torso_x + torso_w * 0.95, torso_y + torso_h * 0.10),
        ],
        armor_dark, accent, 0.5,
    ))
    # Chest emblem
    parts.append(_circle(cx, torso_y + torso_h * 0.45, torso_w * 0.10, accent, glow, 0.5))

    # Arms
    arm_w = spec.width * 0.07
    arm_h = spec.height * 0.30
    arm_y = torso_y + torso_h * 0.05
    parts.append(_rect(torso_x - arm_w * 0.9, arm_y, arm_w, arm_h, armor, armor_dark, 0.5))
    parts.append(_rect(torso_x + torso_w - arm_w * 0.1 + facing_sign * arm_w * 0.2, arm_y,
                       arm_w, arm_h, armor, armor_dark, 0.5))

    # Hands
    parts.append(_circle(torso_x - arm_w * 0.4, arm_y + arm_h, arm_w * 0.5, skin_light))
    parts.append(_circle(torso_x + torso_w + arm_w * 0.4 + facing_sign * arm_w * 0.2,
                         arm_y + arm_h, arm_w * 0.5, skin_light))

    # Legs
    leg_w = spec.width * 0.10
    leg_h = spec.height * 0.30
    leg_y = torso_y + torso_h
    left_x = cx - leg_w * 1.2 - leg_spread * spec.width / 2
    right_x = cx + leg_w * 0.2 + leg_spread * spec.width / 2
    parts.append(_rect(left_x, leg_y, leg_w, leg_h, armor_dark, dark_or(armor_dark, "#000000"), 0.5))
    parts.append(_rect(right_x, leg_y, leg_w, leg_h, armor_dark, dark_or(armor_dark, "#000000"), 0.5))
    # Boots
    parts.append(_rect(left_x - leg_w * 0.1, leg_y + leg_h, leg_w * 1.2, spec.height * 0.04, metal))
    parts.append(_rect(right_x - leg_w * 0.1, leg_y + leg_h, leg_w * 1.2, spec.height * 0.04, metal))

    # Stance-specific decorations
    if stance == "running":
        parts.append(_line(cx - leg_w, leg_y + leg_h * 0.5, cx - leg_w * 2.0, leg_y + leg_h * 0.6,
                           glow, 1.0))
    elif stance == "jetting":
        # Jet flame trail
        parts.append(_polygon(
            [
                (cx, spec.height * 0.94),
                (cx - leg_w * 0.6, spec.height * 0.99),
                (cx, spec.height * 0.96),
                (cx + leg_w * 0.6, spec.height * 0.99),
            ],
            glow, accent, 0.5,
        ))
    elif stance == "climbing":
        parts.append(_line(left_x - leg_w * 0.4, leg_y, left_x - leg_w * 0.8, leg_y - leg_h * 0.4,
                           metal, 1.5))
        parts.append(_line(right_x + leg_w * 1.2, leg_y, right_x + leg_w * 1.6, leg_y - leg_h * 0.4,
                           metal, 1.5))
    return "".join(parts)


def dark_or(color: str, fallback: str) -> str:
    """Tiny helper used inside _compose_actor when the palette doesn't expose `dark`."""
    return color or fallback


def _compose_vehicle(spec: AssetSpec, rng: random.Random) -> str:
    p = spec.palette
    body = p.primary()
    dark = p.dark()
    accent = p.accent()
    metal = p.metal()
    glow = p.glow()
    variant = (spec.extra or {}).get("variant", "side")

    parts: List[str] = []
    hull_y = spec.height * 0.40
    hull_h = spec.height * 0.32
    parts.append(_rect(spec.width * 0.08, hull_y, spec.width * 0.84, hull_h, body, dark, 1.0))
    # Slanted front armor
    parts.append(_polygon(
        [
            (spec.width * 0.92, hull_y),
            (spec.width * 0.98, hull_y + hull_h * 0.4),
            (spec.width * 0.92, hull_y + hull_h),
        ],
        metal, dark, 0.5,
    ))
    # Viewport
    parts.append(_rect(spec.width * 0.62, hull_y + hull_h * 0.18, spec.width * 0.18, hull_h * 0.32,
                       accent, dark, 0.5))
    # Wheels / treads
    for i in range(4):
        wx = spec.width * (0.16 + 0.20 * i)
        parts.append(_circle(wx, spec.height * 0.78, spec.height * 0.07, dark, metal, 0.5))
        parts.append(_circle(wx, spec.height * 0.78, spec.height * 0.04, metal))
    # Top hatch
    parts.append(_rect(spec.width * 0.32, hull_y - spec.height * 0.08, spec.width * 0.18, spec.height * 0.08,
                       body, dark, 1.0))
    if variant == "boarding":
        # ramp open
        parts.append(_polygon(
            [
                (spec.width * 0.08, hull_y + hull_h),
                (spec.width * 0.08, hull_y + hull_h + spec.height * 0.06),
                (spec.width * 0.32, hull_y + hull_h),
            ],
            metal, dark, 0.5,
        ))
    elif variant == "boarded":
        # door closed + indicator
        parts.append(_circle(spec.width * 0.18, hull_y + hull_h * 0.5, spec.width * 0.02, glow))
    return "".join(parts)


def _compose_chassis(spec: AssetSpec, rng: random.Random) -> str:
    p = spec.palette
    body = p.primary()
    dark = p.dark()
    metal = p.metal()
    accent = p.accent()
    parts: List[str] = []
    canonical = spec.canonical_name
    cx = spec.width / 2

    if "bipedal" in canonical:
        parts.append(_circle(cx, spec.height * 0.22, spec.width * 0.10, body, dark, 1.0))
        parts.append(_rect(cx - spec.width * 0.17, spec.height * 0.32, spec.width * 0.34, spec.height * 0.34,
                           body, dark, 1.0))
        parts.append(_rect(cx - spec.width * 0.18, spec.height * 0.66, spec.width * 0.12, spec.height * 0.28,
                           dark, metal, 0.5))
        parts.append(_rect(cx + spec.width * 0.06, spec.height * 0.66, spec.width * 0.12, spec.height * 0.28,
                           dark, metal, 0.5))
    elif "quadruped" in canonical:
        parts.append(_rect(spec.width * 0.10, spec.height * 0.35, spec.width * 0.78, spec.height * 0.28,
                           body, dark, 1.0))
        for i in range(4):
            wx = spec.width * (0.16 + 0.22 * i)
            parts.append(_rect(wx, spec.height * 0.63, spec.width * 0.06, spec.height * 0.30, dark, metal, 0.5))
        parts.append(_polygon(
            [
                (spec.width * 0.88, spec.height * 0.35),
                (spec.width * 0.96, spec.height * 0.4),
                (spec.width * 0.88, spec.height * 0.5),
            ],
            body, dark, 0.5,
        ))
    elif "treaded" in canonical:
        parts.append(_rect(spec.width * 0.10, spec.height * 0.35, spec.width * 0.80, spec.height * 0.32,
                           body, dark, 1.0))
        parts.append(_rect(spec.width * 0.10, spec.height * 0.70, spec.width * 0.80, spec.height * 0.18,
                           dark, metal, 0.5))
        for i in range(7):
            wx = spec.width * (0.12 + 0.11 * i)
            parts.append(_circle(wx, spec.height * 0.78, spec.height * 0.04, metal))
    elif "hovering" in canonical:
        parts.append(_polygon(
            [
                (spec.width * 0.16, spec.height * 0.45),
                (spec.width * 0.84, spec.height * 0.45),
                (spec.width * 0.94, spec.height * 0.55),
                (spec.width * 0.84, spec.height * 0.65),
                (spec.width * 0.16, spec.height * 0.65),
                (spec.width * 0.06, spec.height * 0.55),
            ],
            body, dark, 1.0,
        ))
        parts.append(_rect(spec.width * 0.20, spec.height * 0.80, spec.width * 0.60, spec.height * 0.04,
                           accent))
    elif "wheeled" in canonical:
        parts.append(_rect(spec.width * 0.10, spec.height * 0.35, spec.width * 0.80, spec.height * 0.32,
                           body, dark, 1.0))
        for i in range(4):
            wx = spec.width * (0.16 + 0.22 * i)
            parts.append(_circle(wx, spec.height * 0.78, spec.height * 0.07, dark, metal, 0.5))
            parts.append(_circle(wx, spec.height * 0.78, spec.height * 0.04, metal))
    else:
        # Fallback
        parts.append(_rect(spec.width * 0.10, spec.height * 0.30, spec.width * 0.80, spec.height * 0.50,
                           body, dark, 1.0))

    weight_class = (spec.extra or {}).get("weight_class", "medium")
    pip_count = {"light": 1, "medium": 2, "heavy": 3, "super_heavy": 4}.get(weight_class, 2)
    for i in range(pip_count):
        parts.append(_circle(spec.width * 0.08 + i * spec.width * 0.04, spec.height * 0.10,
                             spec.width * 0.012, accent))
    return "".join(parts)


def _compose_base_module(spec: AssetSpec, rng: random.Random) -> str:
    p = spec.palette
    body = p.primary()
    dark = p.dark()
    accent = p.accent()
    metal = p.metal()
    glow = p.glow()
    parts: List[str] = []
    canonical = spec.canonical_name
    cx = spec.width / 2

    base_w = spec.width * 0.70
    base_h = spec.height * 0.62
    base_x = (spec.width - base_w) / 2
    base_y = spec.height * 0.20
    parts.append(_rect(base_x, base_y, base_w, base_h, body, dark, 1.0))

    if "turret" in canonical:
        parts.append(_circle(cx, base_y + base_h * 0.3, base_w * 0.20, metal, dark, 0.5))
        parts.append(_rect(cx, base_y + base_h * 0.25, base_w * 0.60, base_h * 0.08, metal, dark, 0.5))
    elif "wall" in canonical or "gate" in canonical:
        for i in range(3):
            parts.append(_line(base_x, base_y + base_h * (0.25 + i * 0.25),
                               base_x + base_w, base_y + base_h * (0.25 + i * 0.25), dark, 0.5))
    elif "generator" in canonical or "reactor" in canonical or "battery" in canonical or "capacitor" in canonical:
        parts.append(_circle(cx, base_y + base_h * 0.5, base_h * 0.22, accent, glow, 1.0))
        parts.append(_circle(cx, base_y + base_h * 0.5, base_h * 0.12, glow))
    elif "antenna" in canonical or "radar" in canonical or "sensor" in canonical:
        parts.append(_line(cx, base_y + base_h, cx, base_y - base_h * 0.4, metal, 1.5))
        parts.append(_circle(cx, base_y - base_h * 0.4, base_w * 0.15, metal))
    elif "pump" in canonical or "valve" in canonical:
        parts.append(_circle(cx, base_y + base_h * 0.5, base_h * 0.30, metal, dark, 1.0))
        parts.append(_line(cx, base_y + base_h * 0.2, cx, base_y + base_h * 0.8, dark, 1.0))
        parts.append(_line(cx - base_h * 0.3, base_y + base_h * 0.5,
                           cx + base_h * 0.3, base_y + base_h * 0.5, dark, 1.0))
    elif "pipe" in canonical:
        parts.append(_rect(base_x - base_w * 0.05, base_y + base_h * 0.35,
                           base_w * 1.10, base_h * 0.30, metal, dark, 1.0))
    elif "silo" in canonical or "tank" in canonical:
        parts.append(_polygon(
            [
                (base_x, base_y + base_h * 0.20),
                (base_x + base_w / 2, base_y),
                (base_x + base_w, base_y + base_h * 0.20),
            ],
            metal, dark, 0.5,
        ))
    elif "beacon" in canonical or "spotlight" in canonical:
        parts.append(_circle(cx, base_y + base_h * 0.20, base_w * 0.15, glow, accent, 1.0))
        parts.append(_polygon(
            [
                (cx - base_w * 0.20, base_y + base_h * 0.20),
                (cx + base_w * 0.20, base_y + base_h * 0.20),
                (cx, base_y - base_h * 0.20),
            ],
            glow, accent, 0.5,
        ))
    elif "pad" in canonical or "platform" in canonical or "dock" in canonical:
        parts.append(_rect(base_x, base_y + base_h * 0.85, base_w, base_h * 0.15, dark, accent, 0.5))
    elif "console" in canonical or "terminal" in canonical:
        parts.append(_rect(base_x + base_w * 0.10, base_y + base_h * 0.10,
                           base_w * 0.80, base_h * 0.40, glow, dark, 0.5))
    elif "med" in canonical:
        parts.append(_rect(cx - base_w * 0.04, base_y + base_h * 0.20, base_w * 0.08, base_h * 0.30,
                           "#ffffff", dark, 0.5))
        parts.append(_rect(cx - base_w * 0.16, base_y + base_h * 0.31, base_w * 0.32, base_h * 0.08,
                           "#ffffff", dark, 0.5))
    elif "objective" in canonical or "extraction" in canonical or "lift" in canonical or "emplacement" in canonical:
        parts.append(_polygon(
            [
                (cx, base_y - base_h * 0.10),
                (cx - base_w * 0.10, base_y + base_h * 0.10),
                (cx + base_w * 0.10, base_y + base_h * 0.10),
            ],
            accent, dark, 0.5,
        ))
    else:
        # Industrial / habitat generic
        parts.append(_rect(base_x + base_w * 0.10, base_y + base_h * 0.20,
                           base_w * 0.80, base_h * 0.40, metal, dark, 0.5))

    # Module state pip in top-right
    state = (spec.extra or {}).get("module_state", "nominal")
    state_color = {
        "nominal": "#5bd078",
        "degraded": "#dab438",
        "warning": "#e87826",
        "failed": "#c93030",
    }.get(state, body)
    parts.append(_circle(base_x + base_w - spec.width * 0.04, base_y + spec.height * 0.04,
                         spec.width * 0.025, state_color, dark, 0.5))
    return "".join(parts)


def _compose_ui_icon(spec: AssetSpec, rng: random.Random) -> str:
    p = spec.palette
    body = p.primary()
    accent = p.accent()
    dark = p.dark()
    highlight = p.highlight()
    metal = p.metal()
    glow = p.glow()
    name = spec.canonical_name
    cx, cy = spec.width / 2, spec.height / 2

    parts: List[str] = []
    # Frame
    parts.append(_rect(spec.width * 0.06, spec.height * 0.06, spec.width * 0.88, spec.height * 0.88,
                       dark, body, max(1.0, spec.width * 0.02)))
    parts.append(_rect(spec.width * 0.10, spec.height * 0.10, spec.width * 0.80, spec.height * 0.80,
                       body, accent, max(1.0, spec.width * 0.015)))

    # Inner glyph: bias by name
    if "slot_primary" in name or "weapon_rifle" in name:
        parts.append(_rect(spec.width * 0.18, cy - spec.height * 0.05, spec.width * 0.64, spec.height * 0.10,
                           metal, dark, 0.5))
        parts.append(_rect(spec.width * 0.20, cy - spec.height * 0.08, spec.width * 0.20, spec.height * 0.06,
                           accent))
        parts.append(_rect(spec.width * 0.42, cy + spec.height * 0.05, spec.width * 0.08, spec.height * 0.10,
                           metal))
    elif "slot_secondary" in name or "weapon_pistol" in name:
        parts.append(_rect(spec.width * 0.32, cy - spec.height * 0.08, spec.width * 0.40, spec.height * 0.10,
                           metal, dark, 0.5))
        parts.append(_rect(spec.width * 0.48, cy + spec.height * 0.02, spec.width * 0.08, spec.height * 0.12,
                           dark))
    elif "weapon_smg" in name:
        parts.append(_rect(spec.width * 0.22, cy - spec.height * 0.04, spec.width * 0.50, spec.height * 0.08,
                           metal, dark, 0.5))
        parts.append(_rect(spec.width * 0.40, cy + spec.height * 0.04, spec.width * 0.06, spec.height * 0.10,
                           accent))
    elif "weapon_shotgun" in name:
        parts.append(_rect(spec.width * 0.18, cy - spec.height * 0.06, spec.width * 0.66, spec.height * 0.12,
                           metal, dark, 0.5))
    elif "weapon_sniper" in name:
        parts.append(_rect(spec.width * 0.12, cy - spec.height * 0.03, spec.width * 0.76, spec.height * 0.06,
                           metal, dark, 0.5))
        parts.append(_circle(spec.width * 0.74, cy, spec.height * 0.08, accent, dark, 0.5))
    elif "weapon_gl" in name or "weapon_grenade" in name:
        parts.append(_circle(cx, cy, spec.height * 0.18, metal, dark, 0.5))
        parts.append(_rect(cx - spec.width * 0.04, cy - spec.height * 0.24, spec.width * 0.08, spec.height * 0.08,
                           dark))
    elif "weapon_melee" in name or "slot_melee" in name:
        parts.append(_polygon(
            [
                (cx - spec.width * 0.04, cy - spec.height * 0.20),
                (cx + spec.width * 0.04, cy - spec.height * 0.20),
                (cx + spec.width * 0.02, cy + spec.height * 0.20),
                (cx - spec.width * 0.02, cy + spec.height * 0.20),
            ],
            metal, dark, 0.5,
        ))
    elif "weapon_tool" in name or "slot_tool" in name:
        parts.append(_circle(cx, cy, spec.height * 0.16, metal, dark, 0.5))
        parts.append(_rect(cx - spec.width * 0.02, cy, spec.width * 0.04, spec.height * 0.18, dark))
    elif "weapon_heavy" in name:
        parts.append(_rect(spec.width * 0.14, cy - spec.height * 0.08, spec.width * 0.72, spec.height * 0.16,
                           metal, dark, 0.5))
    elif "slot_helmet" in name:
        parts.append(_polygon(
            [
                (cx - spec.width * 0.20, cy + spec.height * 0.08),
                (cx, cy - spec.height * 0.22),
                (cx + spec.width * 0.20, cy + spec.height * 0.08),
            ],
            metal, dark, 0.5,
        ))
    elif "slot_torso" in name:
        parts.append(_rect(cx - spec.width * 0.18, cy - spec.height * 0.18, spec.width * 0.36, spec.height * 0.36,
                           metal, dark, 0.5))
    elif "slot_legs" in name:
        parts.append(_rect(cx - spec.width * 0.16, cy - spec.height * 0.20, spec.width * 0.10, spec.height * 0.40,
                           metal, dark, 0.5))
        parts.append(_rect(cx + spec.width * 0.06, cy - spec.height * 0.20, spec.width * 0.10, spec.height * 0.40,
                           metal, dark, 0.5))
    elif "slot_backpack" in name:
        parts.append(_rect(cx - spec.width * 0.18, cy - spec.height * 0.16, spec.width * 0.36, spec.height * 0.32,
                           accent, dark, 0.5))
    elif "status_health" in name:
        parts.append(_polygon(
            [
                (cx - spec.width * 0.18, cy - spec.height * 0.05),
                (cx, cy - spec.height * 0.20),
                (cx + spec.width * 0.18, cy - spec.height * 0.05),
                (cx, cy + spec.height * 0.20),
            ],
            "#c93030", dark, 0.5,
        ))
    elif "status_stamina" in name:
        parts.append(_polygon(
            [
                (cx - spec.width * 0.12, cy + spec.height * 0.18),
                (cx, cy - spec.height * 0.18),
                (cx + spec.width * 0.06, cy),
                (cx + spec.width * 0.14, cy + spec.height * 0.18),
            ],
            "#dab438", dark, 0.5,
        ))
    elif "status_armor" in name:
        parts.append(_polygon(
            [
                (cx - spec.width * 0.18, cy - spec.height * 0.10),
                (cx, cy - spec.height * 0.20),
                (cx + spec.width * 0.18, cy - spec.height * 0.10),
                (cx + spec.width * 0.10, cy + spec.height * 0.18),
                (cx - spec.width * 0.10, cy + spec.height * 0.18),
            ],
            metal, dark, 0.5,
        ))
    elif "status_oxygen" in name:
        parts.append(_polygon(
            [
                (cx, cy - spec.height * 0.20),
                (cx - spec.width * 0.14, cy + spec.height * 0.12),
                (cx + spec.width * 0.14, cy + spec.height * 0.12),
            ],
            "#8ac6ff", dark, 0.5,
        ))
    elif "status_temperature" in name:
        parts.append(_rect(cx - spec.width * 0.04, cy - spec.height * 0.20, spec.width * 0.08, spec.height * 0.30,
                           metal, dark, 0.5))
        parts.append(_circle(cx, cy + spec.height * 0.10, spec.height * 0.08, accent))
    elif "status_radiation" in name:
        for i in range(3):
            ang = (i * 120) - 90
            import math
            x = cx + spec.width * 0.18 * math.cos(math.radians(ang))
            y = cy + spec.height * 0.18 * math.sin(math.radians(ang))
            parts.append(_circle(x, y, spec.width * 0.06, accent))
        parts.append(_circle(cx, cy, spec.width * 0.05, dark))
    elif "status_bleed" in name:
        parts.append(_polygon(
            [
                (cx, cy - spec.height * 0.18),
                (cx - spec.width * 0.10, cy + spec.height * 0.12),
                (cx + spec.width * 0.10, cy + spec.height * 0.12),
            ],
            "#c93030", dark, 0.5,
        ))
    elif "status_stun" in name:
        parts.append(_polygon(
            [
                (cx - spec.width * 0.05, cy - spec.height * 0.18),
                (cx + spec.width * 0.10, cy - spec.height * 0.04),
                (cx, cy + spec.height * 0.00),
                (cx + spec.width * 0.10, cy + spec.height * 0.18),
                (cx - spec.width * 0.10, cy + spec.height * 0.02),
                (cx, cy - spec.height * 0.02),
            ],
            glow, dark, 0.5,
        ))
    elif "action_use" in name or "action_pickup" in name:
        parts.append(_polygon(
            [
                (cx - spec.width * 0.12, cy + spec.height * 0.12),
                (cx, cy - spec.height * 0.18),
                (cx + spec.width * 0.12, cy + spec.height * 0.12),
            ],
            accent, dark, 0.5,
        ))
    elif "action_open" in name or "action_close" in name:
        parts.append(_rect(cx - spec.width * 0.18, cy - spec.height * 0.20,
                           spec.width * 0.36, spec.height * 0.40, metal, dark, 0.5))
        parts.append(_line(cx, cy - spec.height * 0.20, cx, cy + spec.height * 0.20, dark, 1.0))
    elif "action_repair" in name:
        parts.append(_rect(cx - spec.width * 0.04, cy - spec.height * 0.20,
                           spec.width * 0.08, spec.height * 0.40, metal, dark, 0.5))
        parts.append(_circle(cx, cy, spec.width * 0.08, accent))
    elif "action_hack" in name:
        parts.append(_rect(cx - spec.width * 0.20, cy - spec.height * 0.16,
                           spec.width * 0.40, spec.height * 0.32, dark, accent, 0.5))
        parts.append(_rect(cx - spec.width * 0.06, cy + spec.height * 0.05,
                           spec.width * 0.04, spec.height * 0.10, accent))
    elif "action_revive" in name:
        parts.append(_polygon(
            [
                (cx - spec.width * 0.04, cy - spec.height * 0.18),
                (cx + spec.width * 0.04, cy - spec.height * 0.18),
                (cx + spec.width * 0.04, cy - spec.height * 0.04),
                (cx + spec.width * 0.18, cy - spec.height * 0.04),
                (cx + spec.width * 0.18, cy + spec.height * 0.04),
                (cx + spec.width * 0.04, cy + spec.height * 0.04),
                (cx + spec.width * 0.04, cy + spec.height * 0.18),
                (cx - spec.width * 0.04, cy + spec.height * 0.18),
                (cx - spec.width * 0.04, cy + spec.height * 0.04),
                (cx - spec.width * 0.18, cy + spec.height * 0.04),
                (cx - spec.width * 0.18, cy - spec.height * 0.04),
                (cx - spec.width * 0.04, cy - spec.height * 0.04),
            ],
            "#5bd078", dark, 0.5,
        ))
    elif "action_board" in name:
        parts.append(_rect(spec.width * 0.40, cy - spec.height * 0.18,
                           spec.width * 0.40, spec.height * 0.36, metal, dark, 0.5))
        parts.append(_polygon(
            [
                (spec.width * 0.20, cy),
                (spec.width * 0.36, cy - spec.height * 0.08),
                (spec.width * 0.36, cy + spec.height * 0.08),
            ],
            accent, dark, 0.5,
        ))
    elif "action_drop" in name:
        parts.append(_rect(cx - spec.width * 0.10, cy - spec.height * 0.20,
                           spec.width * 0.20, spec.height * 0.20, metal, dark, 0.5))
        parts.append(_polygon(
            [
                (cx, cy + spec.height * 0.20),
                (cx - spec.width * 0.10, cy + spec.height * 0.04),
                (cx + spec.width * 0.10, cy + spec.height * 0.04),
            ],
            accent, dark, 0.5,
        ))
    elif "action_signal" in name:
        parts.append(_line(cx, cy + spec.height * 0.18, cx, cy - spec.height * 0.18, metal, 1.5))
        for i in range(3):
            r = spec.width * (0.06 + 0.06 * i)
            parts.append(_circle(cx, cy - spec.height * 0.18, r, "none", accent, 1.0))
    elif "acca_" in name:
        # Generic ACC-A glyph
        parts.append(_circle(cx, cy, spec.width * 0.16, accent, dark, 0.5))
        parts.append(_rect(cx - spec.width * 0.02, cy - spec.height * 0.08, spec.width * 0.04, spec.height * 0.16,
                           highlight))
    elif "material_" in name:
        parts.append(_rect(cx - spec.width * 0.18, cy - spec.height * 0.10,
                           spec.width * 0.36, spec.height * 0.20, metal, dark, 0.5))
    elif "hud_compass" in name:
        parts.append(_circle(cx, cy, spec.width * 0.22, dark, metal, 1.0))
        parts.append(_polygon(
            [
                (cx, cy - spec.height * 0.18),
                (cx - spec.width * 0.04, cy),
                (cx + spec.width * 0.04, cy),
            ],
            accent, dark, 0.5,
        ))
        parts.append(_polygon(
            [
                (cx, cy + spec.height * 0.18),
                (cx - spec.width * 0.04, cy),
                (cx + spec.width * 0.04, cy),
            ],
            highlight, dark, 0.5,
        ))
    elif "hud_objective_complete" in name:
        parts.append(_polyline(
            [
                (cx - spec.width * 0.18, cy),
                (cx - spec.width * 0.04, cy + spec.height * 0.14),
                (cx + spec.width * 0.18, cy - spec.height * 0.14),
            ],
            "#5bd078", stroke_w=max(2.0, spec.width * 0.05),
        ))
    elif "hud_objective" in name:
        parts.append(_polygon(
            [
                (cx - spec.width * 0.18, cy + spec.height * 0.04),
                (cx, cy - spec.height * 0.20),
                (cx + spec.width * 0.18, cy + spec.height * 0.04),
                (cx + spec.width * 0.10, cy + spec.height * 0.20),
                (cx - spec.width * 0.10, cy + spec.height * 0.20),
            ],
            accent, dark, 0.5,
        ))
    elif "hud_waypoint" in name:
        parts.append(_polygon(
            [
                (cx - spec.width * 0.04, cy + spec.height * 0.18),
                (cx - spec.width * 0.04, cy - spec.height * 0.18),
                (cx + spec.width * 0.18, cy - spec.height * 0.10),
                (cx - spec.width * 0.04, cy - spec.height * 0.04),
            ],
            accent, dark, 0.5,
        ))
    elif "hud_extraction" in name:
        parts.append(_rect(cx - spec.width * 0.16, cy - spec.height * 0.04, spec.width * 0.32, spec.height * 0.08,
                           metal, dark, 0.5))
        parts.append(_rect(cx + spec.width * 0.10, cy - spec.height * 0.10, spec.width * 0.10, spec.height * 0.20,
                           dark))
    elif "hud_alert" in name:
        bars = {"low": 1, "med": 2, "high": 3}
        n = 1
        for k, v in bars.items():
            if k in name:
                n = v
                break
        for i in range(3):
            color = accent if i < n else metal
            parts.append(_rect(cx - spec.width * 0.18 + i * spec.width * 0.13,
                               cy + spec.height * 0.04 - i * spec.height * 0.04,
                               spec.width * 0.10, spec.height * 0.14, color, dark, 0.5))
    else:
        # Generic
        parts.append(_circle(cx, cy, spec.width * 0.16, accent, dark, 0.5))
        parts.append(_rect(cx - spec.width * 0.04, cy - spec.height * 0.04, spec.width * 0.08, spec.height * 0.08,
                           highlight))
    return "".join(parts)


def _compose_material(spec: AssetSpec, rng: random.Random) -> str:
    # spec.extra carries 'base_color' (the integrity-banded hex). Pipeline
    # passes the material id via canonical_name.
    base = (spec.extra or {}).get("base_color", spec.palette.primary())
    band = (spec.extra or {}).get("integrity_band", "pristine")
    mode = (spec.extra or {}).get("overlay_mode")
    name = spec.canonical_name
    parts: List[str] = []

    parts.append(_rect(0, 0, spec.width, spec.height, base))
    # Add per-material texture flavor
    if "dirt" in name or "sand" in name or "mud" in name:
        for _ in range(40):
            x = rng.uniform(0, spec.width)
            y = rng.uniform(0, spec.height)
            r = rng.uniform(0.5, 1.4)
            parts.append(_circle(x, y, r, _darken_hex(base, 0.2)))
    elif "concrete" in name or "loose_fill" in name:
        for _ in range(18):
            x = rng.uniform(0, spec.width)
            y = rng.uniform(0, spec.height)
            r = rng.uniform(0.6, 2.0)
            parts.append(_rect(x, y, r * 2, r * 2, _darken_hex(base, 0.15)))
    elif "metal" in name or "alloy" in name:
        for i in range(0, spec.height, max(1, spec.height // 8)):
            parts.append(_line(0, i, spec.width, i, _lighten_hex(base, 0.05), 0.3))
    elif "hazard" in name:
        for i in range(0, spec.width + spec.height, max(1, spec.width // 6)):
            parts.append(_polyline(
                [(i, 0), (i - spec.height, spec.height)],
                _darken_hex(base, 0.2), max(1.0, spec.width / 24),
            ))
    elif "lava" in name:
        for _ in range(8):
            x = rng.uniform(0, spec.width)
            y = rng.uniform(0, spec.height)
            parts.append(_polyline(
                [(x, y), (x + rng.uniform(-spec.width / 4, spec.width / 4),
                          y + rng.uniform(-spec.height / 4, spec.height / 4))],
                _lighten_hex(base, 0.30), 1.5,
            ))
    elif "ice" in name or "glass" in name:
        for _ in range(8):
            x1 = rng.uniform(0, spec.width)
            x2 = x1 + rng.uniform(-spec.width / 3, spec.width / 3)
            y1 = rng.uniform(0, spec.height)
            y2 = y1 + rng.uniform(-spec.height / 3, spec.height / 3)
            parts.append(_line(x1, y1, x2, y2, _lighten_hex(base, 0.2), 0.5))
    elif "wood" in name:
        for i in range(0, spec.height, max(2, spec.height // 6)):
            parts.append(_line(0, i + rng.randint(-1, 1), spec.width, i + rng.randint(-1, 1),
                               _darken_hex(base, 0.2), 0.3))
    elif "snow" in name:
        for _ in range(80):
            x = rng.uniform(0, spec.width)
            y = rng.uniform(0, spec.height)
            parts.append(_circle(x, y, 0.5, _lighten_hex(base, 0.15)))
    elif "circuit" in name:
        for i in range(0, spec.height, max(2, spec.height // 8)):
            parts.append(_line(0, i, spec.width, i, _lighten_hex(base, 0.1), 0.4))
            parts.append(_line(i, 0, i, spec.height, _lighten_hex(base, 0.1), 0.4))
    elif "biomatter" in name:
        for _ in range(14):
            cx = rng.uniform(0, spec.width)
            cy = rng.uniform(0, spec.height)
            parts.append(_ellipse(cx, cy, rng.uniform(2, 6), rng.uniform(1, 3),
                                  _lighten_hex(base, 0.15)))
    # Integrity-band cracks
    if band in ("cracked", "critical", "destroyed"):
        count = {"cracked": 3, "critical": 6, "destroyed": 10}[band]
        for _ in range(count):
            x1 = rng.uniform(0, spec.width)
            y1 = rng.uniform(0, spec.height)
            x2 = x1 + rng.uniform(-spec.width / 3, spec.width / 3)
            y2 = y1 + rng.uniform(-spec.height / 3, spec.height / 3)
            parts.append(_line(x1, y1, x2, y2, _darken_hex(base, 0.35), 1.0))
    # Overlay mode tint (used by the 5-mode M3 overlay).
    if mode is not None:
        tint = {
            "integrity": "#5bd078",
            "pathability": "#3a8cff",
            "mobility": "#dab438",
            "hazard": "#c93030",
            "build_repair": "#8a78ff",
        }.get(mode, "#888888")
        parts.append(_rect(0, 0, spec.width, spec.height, tint))
    return "".join(parts)


def _compose_particle(spec: AssetSpec, rng: random.Random) -> str:
    p = spec.palette
    body = p.primary()
    accent = p.accent()
    highlight = p.highlight()
    glow = p.glow()
    name = spec.canonical_name
    phase = (spec.extra or {}).get("phase", "spawn")
    cx, cy = spec.width / 2, spec.height / 2

    # Phase scales the particle's footprint.
    scale = {"spawn": 0.25, "mid": 0.55, "late": 0.80, "dissipate": 1.0}.get(phase, 0.5)
    parts: List[str] = []

    if "spark" in name:
        # Spark fan
        for i in range(8):
            ang = (i / 8.0) * 360.0
            import math
            r = spec.width * 0.35 * scale
            x = cx + r * math.cos(math.radians(ang))
            y = cy + r * math.sin(math.radians(ang))
            parts.append(_line(cx, cy, x, y, accent if i % 2 == 0 else highlight, 1.2))
    elif "smoke" in name or "dust" in name:
        for _ in range(12):
            x = cx + rng.uniform(-spec.width / 4, spec.width / 4) * scale
            y = cy + rng.uniform(-spec.height / 4, spec.height / 4) * scale
            r = spec.width * 0.08 * scale * rng.uniform(0.6, 1.6)
            parts.append(_circle(x, y, r, body))
    elif "ember" in name:
        for _ in range(6):
            x = cx + rng.uniform(-spec.width / 4, spec.width / 4) * scale
            y = cy + rng.uniform(-spec.height / 4, spec.height / 4) * scale
            r = spec.width * 0.04 * scale
            parts.append(_circle(x, y, r, accent))
    elif "impact" in name or "debris" in name:
        for _ in range(8):
            x = cx + rng.uniform(-spec.width / 3, spec.width / 3) * scale
            y = cy + rng.uniform(-spec.height / 3, spec.height / 3) * scale
            parts.append(_polygon(
                [(x, y),
                 (x + 1.5 * scale * spec.width / 32, y + 1.5 * scale * spec.height / 32),
                 (x + 1.5 * scale * spec.width / 32, y - 0.5 * scale * spec.height / 32)],
                body, accent, 0.5,
            ))
    elif "fluid" in name:
        for _ in range(8):
            x = cx + rng.uniform(-spec.width / 3, spec.width / 3) * scale
            y = cy + rng.uniform(-spec.height / 3, spec.height / 3) * scale
            r = spec.width * 0.05 * scale * rng.uniform(0.6, 1.4)
            parts.append(_circle(x, y, r, accent))
    else:
        # Generic glow orb
        parts.append(_circle(cx, cy, spec.width * 0.30 * scale, glow))
        parts.append(_circle(cx, cy, spec.width * 0.18 * scale, highlight))
        parts.append(_circle(cx, cy, spec.width * 0.10 * scale, accent))
    return "".join(parts)


def _compose_terrain_tile(spec: AssetSpec, rng: random.Random) -> str:
    base = (spec.extra or {}).get("base_color", spec.palette.primary())
    variant = (spec.extra or {}).get("variant", "a")
    seed_offset = sum(ord(c) for c in variant)
    rng2 = random.Random(spec.seed + seed_offset)
    parts: List[str] = [_rect(0, 0, spec.width, spec.height, base)]
    name = spec.canonical_name
    if "dirt" in name or "sand" in name or "mud" in name or "loose" in name:
        for _ in range(60):
            x = rng2.uniform(0, spec.width)
            y = rng2.uniform(0, spec.height)
            r = rng2.uniform(0.5, 1.6)
            parts.append(_circle(x, y, r, _darken_hex(base, 0.2)))
    elif "concrete" in name or "anchor" in name:
        for i in range(0, spec.width, max(1, spec.width // 4)):
            parts.append(_line(i, 0, i, spec.height, _darken_hex(base, 0.15), 0.4))
            parts.append(_line(0, i, spec.width, i, _darken_hex(base, 0.15), 0.4))
    elif "metal" in name or "alloy" in name:
        for i in range(0, spec.height, max(1, spec.height // 6)):
            parts.append(_line(0, i, spec.width, i, _lighten_hex(base, 0.06), 0.5))
    elif "hazard" in name:
        for i in range(-spec.height, spec.width, max(2, spec.width // 5)):
            parts.append(_polyline(
                [(i, 0), (i + spec.height, spec.height)],
                _darken_hex(base, 0.25), max(1.0, spec.width / 16),
            ))
    elif "lava" in name:
        for _ in range(12):
            x = rng2.uniform(0, spec.width)
            y = rng2.uniform(0, spec.height)
            parts.append(_circle(x, y, rng2.uniform(1, 3), _lighten_hex(base, 0.25)))
    elif "ice" in name or "glass" in name:
        for _ in range(10):
            x1 = rng2.uniform(0, spec.width)
            x2 = x1 + rng2.uniform(-spec.width / 3, spec.width / 3)
            y1 = rng2.uniform(0, spec.height)
            y2 = y1 + rng2.uniform(-spec.height / 3, spec.height / 3)
            parts.append(_line(x1, y1, x2, y2, _lighten_hex(base, 0.20), 0.5))
    elif "wood" in name:
        for i in range(0, spec.height, max(2, spec.height // 5)):
            parts.append(_line(0, i, spec.width, i, _darken_hex(base, 0.20), 0.5))
    elif "snow" in name:
        for _ in range(100):
            x = rng2.uniform(0, spec.width)
            y = rng2.uniform(0, spec.height)
            parts.append(_circle(x, y, 0.6, _lighten_hex(base, 0.15)))
    elif "circuit" in name:
        for i in range(0, spec.height, max(2, spec.height // 7)):
            parts.append(_line(0, i, spec.width, i, _lighten_hex(base, 0.10), 0.4))
            parts.append(_line(i, 0, i, spec.height, _lighten_hex(base, 0.10), 0.4))
    elif "repair" in name:
        for _ in range(16):
            x = rng2.uniform(0, spec.width)
            y = rng2.uniform(0, spec.height)
            parts.append(_circle(x, y, rng2.uniform(1, 4), _lighten_hex(base, 0.18)))
    elif "biomatter" in name:
        for _ in range(12):
            cx = rng2.uniform(0, spec.width)
            cy = rng2.uniform(0, spec.height)
            parts.append(_ellipse(cx, cy, rng2.uniform(2, 5), rng2.uniform(1, 3),
                                  _lighten_hex(base, 0.15)))
    return "".join(parts)


def _compose_cosmetic_stub(spec: AssetSpec, rng: random.Random) -> str:
    p = spec.palette
    body = p.primary()
    accent = p.accent()
    dark = p.dark()
    highlight = p.highlight()
    parts: List[str] = []
    name = spec.canonical_name
    cx, cy = spec.width / 2, spec.height / 2

    parts.append(_rect(spec.width * 0.10, spec.height * 0.10, spec.width * 0.80, spec.height * 0.80,
                       body, dark, max(0.5, spec.width * 0.01)))
    if "helmet" in name or "decal" in name or "emblem" in name:
        parts.append(_circle(cx, cy, spec.width * 0.22, accent, dark, 1.0))
        parts.append(_circle(cx, cy, spec.width * 0.10, highlight))
    elif "pauldron" in name or "stripe" in name or "band" in name or "trim" in name:
        parts.append(_rect(spec.width * 0.14, spec.height * 0.45, spec.width * 0.72, spec.height * 0.10,
                           accent, dark, 0.5))
    elif "banner" in name:
        parts.append(_polygon(
            [
                (spec.width * 0.30, spec.height * 0.15),
                (spec.width * 0.70, spec.height * 0.15),
                (spec.width * 0.65, spec.height * 0.85),
                (cx, spec.height * 0.75),
                (spec.width * 0.35, spec.height * 0.85),
            ],
            accent, dark, 1.0,
        ))
    elif "skin" in name:
        for i in range(4):
            parts.append(_rect(spec.width * 0.14, spec.height * (0.18 + 0.18 * i),
                               spec.width * 0.72, spec.height * 0.08,
                               accent if i % 2 == 0 else highlight, dark, 0.5))
    return "".join(parts)


def _compose_emblem(spec: AssetSpec, rng: random.Random) -> str:
    p = spec.palette
    body = p.primary()
    accent = p.accent()
    dark = p.dark()
    highlight = p.highlight()
    glow = p.glow()
    name = spec.canonical_name
    cx, cy = spec.width / 2, spec.height / 2
    simple = "_simple" in name

    parts: List[str] = []
    # Backdrop circle
    parts.append(_circle(cx, cy, min(spec.width, spec.height) * 0.46, dark, accent,
                         max(1.0, spec.width * 0.02)))
    if not simple:
        parts.append(_circle(cx, cy, min(spec.width, spec.height) * 0.38, body, dark, 1.0))

    # Faction-symbolic inner mark
    if "hostile_corp" in name:
        # Hex-and-spike
        import math
        pts = []
        for i in range(6):
            ang = math.radians(60 * i - 30)
            r = spec.width * 0.22
            pts.append((cx + r * math.cos(ang), cy + r * math.sin(ang)))
        parts.append(_polygon(pts, accent, dark, 1.0))
        parts.append(_rect(cx - spec.width * 0.02, cy - spec.height * 0.10,
                           spec.width * 0.04, spec.height * 0.20, highlight))
    elif "allied_resistance" in name:
        # Crossed wrench-and-rifle
        parts.append(_polyline(
            [(cx - spec.width * 0.18, cy - spec.height * 0.18),
             (cx + spec.width * 0.18, cy + spec.height * 0.18)],
            accent, max(2.0, spec.width * 0.04),
        ))
        parts.append(_polyline(
            [(cx - spec.width * 0.18, cy + spec.height * 0.18),
             (cx + spec.width * 0.18, cy - spec.height * 0.18)],
            accent, max(2.0, spec.width * 0.04),
        ))
    elif "marauder" in name:
        # Asymmetric tribal mark
        parts.append(_polygon(
            [(cx, cy - spec.height * 0.20),
             (cx + spec.width * 0.16, cy),
             (cx + spec.width * 0.06, cy + spec.height * 0.20),
             (cx - spec.width * 0.12, cy + spec.height * 0.06)],
            accent, dark, 0.5,
        ))
    elif "religious_order" in name:
        # Vertical with horizontal bar
        parts.append(_rect(cx - spec.width * 0.03, cy - spec.height * 0.22,
                           spec.width * 0.06, spec.height * 0.44, accent, dark, 0.5))
        parts.append(_rect(cx - spec.width * 0.18, cy - spec.height * 0.04,
                           spec.width * 0.36, spec.height * 0.08, accent, dark, 0.5))
    elif "scientist_order" in name:
        # Atom
        parts.append(_circle(cx, cy, spec.width * 0.05, glow))
        parts.append(_ellipse(cx, cy, spec.width * 0.22, spec.height * 0.10, "none",
                              accent, max(1.0, spec.width * 0.02)))
        parts.append(_ellipse(cx, cy, spec.width * 0.10, spec.height * 0.22, "none",
                              accent, max(1.0, spec.width * 0.02)))
    elif "mercenary_guild" in name:
        # Shield-and-bar
        parts.append(_polygon(
            [(cx - spec.width * 0.16, cy - spec.height * 0.18),
             (cx + spec.width * 0.16, cy - spec.height * 0.18),
             (cx + spec.width * 0.10, cy + spec.height * 0.20),
             (cx - spec.width * 0.10, cy + spec.height * 0.20)],
            accent, dark, 0.5,
        ))
        parts.append(_rect(cx - spec.width * 0.16, cy - spec.height * 0.06,
                           spec.width * 0.32, spec.height * 0.04, highlight))
    elif "pirates" in name:
        # Skull silhouette
        parts.append(_circle(cx, cy - spec.height * 0.04, spec.width * 0.16, highlight, dark, 0.5))
        parts.append(_circle(cx - spec.width * 0.06, cy - spec.height * 0.02,
                             spec.width * 0.03, dark))
        parts.append(_circle(cx + spec.width * 0.06, cy - spec.height * 0.02,
                             spec.width * 0.03, dark))
        parts.append(_rect(cx - spec.width * 0.04, cy + spec.height * 0.06,
                           spec.width * 0.08, spec.height * 0.04, dark))
    elif "drone_collective" in name:
        # Faceted hex
        import math
        pts = []
        for i in range(6):
            ang = math.radians(60 * i)
            r = spec.width * 0.20
            pts.append((cx + r * math.cos(ang), cy + r * math.sin(ang)))
        parts.append(_polygon(pts, accent, dark, 1.0))
        parts.append(_circle(cx, cy, spec.width * 0.04, glow))
    else:
        parts.append(_circle(cx, cy, spec.width * 0.08, accent))
    return "".join(parts)


def _compose_overlay(spec: AssetSpec, rng: random.Random) -> str:
    p = spec.palette
    accent = p.accent()
    dark = p.dark()
    highlight = p.highlight()
    name = spec.canonical_name
    parts: List[str] = []

    # Frame
    parts.append(_rect(0, 0, spec.width, spec.height, "none", accent, max(2.0, spec.width * 0.01)))
    parts.append(_rect(spec.width * 0.005, spec.height * 0.005,
                       spec.width * 0.99, spec.height * 0.99,
                       "none", highlight, max(1.0, spec.width * 0.005)))
    if "logo_watermark_corner" in name:
        parts.append(_rect(spec.width * 0.02, spec.height * 0.02, spec.width * 0.10, spec.height * 0.06,
                           accent, dark, 1.0))
    elif "seed_watermark_topright" in name:
        parts.append(_rect(spec.width * 0.88, spec.height * 0.02, spec.width * 0.10, spec.height * 0.06,
                           highlight, dark, 1.0))
    elif "tick_watermark_bottom" in name:
        parts.append(_rect(spec.width * 0.45, spec.height * 0.92, spec.width * 0.10, spec.height * 0.06,
                           accent, dark, 1.0))
    elif "version_watermark_bottomleft" in name:
        parts.append(_rect(spec.width * 0.02, spec.height * 0.92, spec.width * 0.14, spec.height * 0.06,
                           highlight, dark, 1.0))
    elif "bp" in name or "m6" in name or "m7" in name or "m8" in name or "m9" in name or "m10" in name or "m11" in name:
        parts.append(_rect(spec.width * 0.02, spec.height * 0.02, spec.width * 0.20, spec.height * 0.08,
                           accent, dark, 1.0))
        parts.append(_rect(spec.width * 0.78, spec.height * 0.90, spec.width * 0.20, spec.height * 0.08,
                           highlight, dark, 1.0))
    else:
        parts.append(_rect(spec.width * 0.40, spec.height * 0.05, spec.width * 0.20, spec.height * 0.06,
                           accent, dark, 1.0))
    return "".join(parts)


# ─── Color utility ──────────────────────────────────────────────────────────


def _hex_to_rgb(hx: str) -> Tuple[int, int, int]:
    s = hx.lstrip("#")
    if len(s) == 3:
        s = "".join(c * 2 for c in s)
    return (int(s[0:2], 16), int(s[2:4], 16), int(s[4:6], 16))


def _rgb_to_hex(r: int, g: int, b: int) -> str:
    return f"#{r:02x}{g:02x}{b:02x}"


def _darken_hex(hx: str, factor: float) -> str:
    r, g, b = _hex_to_rgb(hx)
    f = max(0.0, 1.0 - factor)
    return _rgb_to_hex(int(r * f), int(g * f), int(b * f))


def _lighten_hex(hx: str, factor: float) -> str:
    r, g, b = _hex_to_rgb(hx)
    return _rgb_to_hex(
        min(255, int(r + (255 - r) * factor)),
        min(255, int(g + (255 - g) * factor)),
        min(255, int(b + (255 - b) * factor)),
    )


# ─── Dispatch ───────────────────────────────────────────────────────────────


_COMPOSERS = {
    "WeaponSprite": _compose_weapon,
    "ActorSprite": _compose_actor,
    "VehicleSprite": _compose_vehicle,
    "ChassisSprite": _compose_chassis,
    "BaseModuleSprite": _compose_base_module,
    "UiIcon": _compose_ui_icon,
    "MaterialSwatch": _compose_material,
    "Particle": _compose_particle,
    "TerrainTile": _compose_terrain_tile,
    "Cosmetic": _compose_cosmetic_stub,
    "FactionEmblem": _compose_emblem,
    "CaptureGridOverlay": _compose_overlay,
}


def compose_svg(spec: AssetSpec) -> str:
    """Compose an SVG string for the given asset spec.

    Routes by `spec.category`. Pre-pads the header + footer; the composer
    returns only the inner <g>-body.
    """
    rng = random.Random(int(spec.seed) & 0xFFFFFFFFFFFFFFFF)
    composer = _COMPOSERS.get(spec.category)
    if composer is None:
        # Procedural fallback: faction-coherent rectangle stack.
        p = spec.palette
        body = p.primary()
        accent = p.accent()
        dark = p.dark()
        inner = "".join([
            _rect(spec.width * 0.10, spec.height * 0.10, spec.width * 0.80, spec.height * 0.80,
                  body, dark, 1.0),
            _rect(spec.width * 0.30, spec.height * 0.30, spec.width * 0.40, spec.height * 0.40,
                  accent, dark, 0.5),
        ])
    else:
        inner = composer(spec, rng)
    return SVG_HEADER.format(w=spec.width, h=spec.height) + inner + SVG_FOOTER


__all__ = ["AssetSpec", "compose_svg"]
