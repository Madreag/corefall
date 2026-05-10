#!/usr/bin/env python3
"""Find run bundles that prove the current source state.

BP closeout evidence can be dirty while an audit branch is still in flight, but
`commit_sha: <HEAD>-dirty` is too coarse: every dirty iteration shares that
string. This helper rejects stale dirty bundles unless their run-manifest
worktree fingerprint matches the checkout fingerprint captured by the closure
loop.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


def read_json(path: Path) -> dict[str, Any] | None:
    try:
        with path.open("r", encoding="utf-8") as f:
            data = json.load(f)
        return data if isinstance(data, dict) else None
    except (OSError, json.JSONDecodeError):
        return None


def manifest_for(bundle: Path) -> dict[str, Any] | None:
    return read_json(bundle / "run_manifest.json")


def current_code_match(manifest: dict[str, Any], head_sha12: str, current_dirty: bool, current_fingerprint: str) -> bool:
    build = manifest.get("build", {})
    if not isinstance(build, dict):
        return False
    if current_dirty:
        return bool(build.get("worktree_dirty")) and build.get("worktree_fingerprint") == current_fingerprint
    commit = str(build.get("commit_sha", ""))
    return not bool(build.get("worktree_dirty")) and commit.replace("-dirty", "")[:12] == head_sha12


def contract_key(manifest: dict[str, Any]) -> tuple[Any, ...]:
    scene = manifest.get("scene", {})
    if not isinstance(scene, dict):
        scene = {}
    return (
        scene.get("id"),
        manifest.get("config_hash"),
        manifest.get("tick_rate_hz"),
        manifest.get("settings"),
        manifest.get("capture_config"),
        manifest.get("expected_tests"),
    )


def scene_id(manifest: dict[str, Any]) -> str:
    scene = manifest.get("scene", {})
    if isinstance(scene, dict):
        return str(scene.get("id", ""))
    return ""


def iter_bundles(root: Path) -> list[Path]:
    bundles = [p for p in root.glob("m*_*") if p.is_dir() and (p / "run_manifest.json").is_file()]
    return sorted(bundles, key=lambda p: (p / "run_manifest.json").stat().st_mtime, reverse=True)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("find", nargs="?")
    parser.add_argument("--root", required=True, type=Path)
    parser.add_argument("--head-sha12", required=True)
    parser.add_argument("--current-dirty", choices=["true", "false"], required=True)
    parser.add_argument("--current-fingerprint", default="")
    parser.add_argument("--scenario", default="")
    parser.add_argument("--reference", type=Path)
    args = parser.parse_args()

    current_dirty = args.current_dirty == "true"
    if current_dirty and not args.current_fingerprint:
        return 3

    reference_key = None
    if args.reference:
        ref_manifest = manifest_for(args.reference)
        if ref_manifest is None:
            return 2
        reference_key = contract_key(ref_manifest)

    for bundle in iter_bundles(args.root):
        manifest = manifest_for(bundle)
        if manifest is None:
            continue
        if args.scenario and scene_id(manifest) != args.scenario:
            continue
        if reference_key is not None and contract_key(manifest) != reference_key:
            continue
        if not current_code_match(manifest, args.head_sha12, current_dirty, args.current_fingerprint):
            continue
        if not (bundle / "grading.json").is_file():
            continue
        print(bundle)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
