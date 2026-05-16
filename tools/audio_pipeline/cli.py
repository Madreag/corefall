"""cf-audio-pipeline orchestrator.

Usage:
    python cli.py status
    python cli.py bake-all                  # design → voice → sfx → music
    python cli.py bake-all --dry-run
    python cli.py voice-design [--dry-run]
    python cli.py voice-lines  [--dry-run] [--filter chatter|npc|...]
    python cli.py sfx          [--dry-run] [--filter weapon|ambient|...]
    python cli.py music        [--dry-run] [--filter world|faction|...]
"""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
PY = sys.executable


def _run(args: list[str]) -> int:
    print(f"\n=== cli.py running: {' '.join(args)} ===")
    return subprocess.call([PY, *args], cwd=str(_HERE))


def cmd_status(_args) -> int:
    REPO_ROOT = _HERE.parents[1]
    sfx_dir = REPO_ROOT / "game" / "content" / "audio" / "sfx"
    voice_dir = REPO_ROOT / "game" / "content" / "audio" / "voice"
    music_dir = REPO_ROOT / "game" / "content" / "audio" / "music"
    ledger = REPO_ROOT / "content" / "asset_ledger" / "ledger.jsonl"
    registry = _HERE / "voice_synthesis" / "per_npc_voice_registry.toml"
    state = _HERE / "_state"

    def _count(p: Path, ext: str) -> int:
        if not p.exists():
            return 0
        return sum(1 for _ in p.glob(f"*{ext}"))

    print(f"sfx wavs   : {_count(sfx_dir, '.wav')}")
    print(f"voice wavs : {_count(voice_dir, '.wav')}")
    print(f"music wavs : {_count(music_dir, '.wav')}")
    if ledger.exists():
        lines = ledger.read_text(encoding="utf-8").splitlines()
        cats: dict[str, int] = {}
        for line in lines:
            try:
                e = __import__("json").loads(line)
                cats[e["category"]] = cats.get(e["category"], 0) + 1
            except Exception:
                continue
        print(f"ledger     : {len(lines)} entries")
        for cat in sorted(cats):
            if cat.startswith("Audio_"):
                print(f"  {cat:<14s} {cats[cat]}")
    print(f"registry   : {'present' if registry.exists() else 'absent'}")
    if state.exists():
        for f in state.glob("*.json"):
            try:
                p = __import__("json").loads(f.read_text(encoding="utf-8"))
                print(f"state/{f.name}: completed={len(p.get('completed', []))} failed={len(p.get('failed', []))}")
            except Exception:
                pass
    return 0


def cmd_voice_design(args) -> int:
    extra = ["--dry-run"] if args.dry_run else []
    return _run(["eleven_voice_design.py", *extra, *args.passthrough])


def cmd_voice_lines(args) -> int:
    extra = ["--dry-run"] if args.dry_run else []
    if args.filter:
        extra += ["--filter", args.filter]
    return _run(["eleven_voice_lines.py", *extra, *args.passthrough])


def cmd_sfx(args) -> int:
    extra = ["--dry-run"] if args.dry_run else []
    if args.filter:
        extra += ["--filter", args.filter]
    return _run(["eleven_sfx.py", *extra, *args.passthrough])


def cmd_music(args) -> int:
    extra = ["--dry-run"] if args.dry_run else []
    if args.filter:
        extra += ["--filter", args.filter]
    return _run(["eleven_music.py", *extra, *args.passthrough])


def cmd_bake_all(args) -> int:
    extra = ["--dry-run"] if args.dry_run else []
    steps = [
        (["eleven_voice_design.py", *extra], "voice-design"),
        (["eleven_voice_lines.py", *extra], "voice-lines"),
        (["eleven_sfx.py", *extra], "sfx"),
        (["eleven_music.py", *extra], "music"),
    ]
    for argv, label in steps:
        rc = _run(argv)
        if rc != 0:
            print(f"\n[cli] step `{label}` failed (rc={rc}); halting bake-all")
            return rc
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest="cmd", required=True)

    sub.add_parser("status").set_defaults(func=cmd_status)

    p = sub.add_parser("voice-design")
    p.add_argument("--dry-run", action="store_true")
    p.add_argument("passthrough", nargs=argparse.REMAINDER)
    p.set_defaults(func=cmd_voice_design)

    p = sub.add_parser("voice-lines")
    p.add_argument("--dry-run", action="store_true")
    p.add_argument("--filter", default=None)
    p.add_argument("passthrough", nargs=argparse.REMAINDER)
    p.set_defaults(func=cmd_voice_lines)

    p = sub.add_parser("sfx")
    p.add_argument("--dry-run", action="store_true")
    p.add_argument("--filter", default=None)
    p.add_argument("passthrough", nargs=argparse.REMAINDER)
    p.set_defaults(func=cmd_sfx)

    p = sub.add_parser("music")
    p.add_argument("--dry-run", action="store_true")
    p.add_argument("--filter", default=None)
    p.add_argument("passthrough", nargs=argparse.REMAINDER)
    p.set_defaults(func=cmd_music)

    p = sub.add_parser("bake-all")
    p.add_argument("--dry-run", action="store_true")
    p.set_defaults(func=cmd_bake_all)

    args = ap.parse_args()
    return int(args.func(args) or 0)


if __name__ == "__main__":
    raise SystemExit(main())
