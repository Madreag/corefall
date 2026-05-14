#!/usr/bin/env python3
"""M4 § Replay throughput benchmark.

Runs `cargo run -p cf-headless -- replay <bundle> --measure throughput`
against every bundle passed on the command line (or, by default, every
bundle under `prototype_runs/native/`) and reports per-bundle and
aggregate throughput numbers so we can catch perf regressions early.

Output format (JSON to stdout):

    {
      "schema_version": 1,
      "bundles": [
        {
          "bundle": "prototype_runs/native/<id>",
          "result": "ok",
          "replayed_ticks": 18000,
          "throughput_ticks_per_sec": 12345.6,
          "wall_time_ms": 1450.7,
          "peak_memory_mb": 132.4
        },
        ...
      ],
      "aggregate": {
        "bundle_count": N,
        "median_throughput_ticks_per_sec": ...,
        "min_throughput_ticks_per_sec": ...,
        "max_throughput_ticks_per_sec": ...
      }
    }

Non-zero exit when any bundle replay fails (e.g., divergence, missing
files, replay verifier stall).
"""

from __future__ import annotations

import argparse
import json
import os
import statistics
import subprocess
import sys
from pathlib import Path
from typing import Any


REPO_ROOT = Path(__file__).resolve().parent.parent.parent
GAME_DIR = REPO_ROOT / "game"


def find_bundles(roots: list[Path]) -> list[Path]:
    out: list[Path] = []
    for root in roots:
        if not root.exists():
            continue
        if (root / "run_manifest.json").exists():
            out.append(root)
            continue
        for child in sorted(root.iterdir()):
            if child.is_dir() and (child / "run_manifest.json").exists():
                out.append(child)
    return out


def run_one(bundle: Path) -> dict[str, Any]:
    cmd = [
        "cargo",
        "run",
        "--quiet",
        "-p",
        "cf-headless",
        "--",
        "replay",
        str(bundle),
        "--measure",
        "throughput",
    ]
    proc = subprocess.run(
        cmd,
        cwd=GAME_DIR,
        capture_output=True,
        text=True,
        check=False,
        env={**os.environ, "RUST_LOG": "error"},
    )
    if proc.returncode != 0:
        return {
            "bundle": str(bundle.relative_to(REPO_ROOT))
            if bundle.is_relative_to(REPO_ROOT)
            else str(bundle),
            "result": "error",
            "exit_code": proc.returncode,
            "stderr_tail": proc.stderr.strip().splitlines()[-10:],
        }
    # The verifier prints one JSON line on stdout. Tracing output goes to
    # stderr so we ignore it here.
    last_json_line: str | None = None
    for line in proc.stdout.splitlines():
        if line.startswith("{"):
            last_json_line = line
    if not last_json_line:
        return {
            "bundle": str(bundle),
            "result": "error",
            "reason": "no JSON envelope on stdout",
            "stdout_tail": proc.stdout.splitlines()[-10:],
        }
    envelope = json.loads(last_json_line)
    envelope["bundle"] = (
        str(bundle.relative_to(REPO_ROOT))
        if bundle.is_relative_to(REPO_ROOT)
        else str(bundle)
    )
    return envelope


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "bundles",
        nargs="*",
        type=Path,
        help="Bundle directories (default: every bundle under prototype_runs/native/).",
    )
    args = parser.parse_args()

    roots = args.bundles or [REPO_ROOT / "prototype_runs" / "native"]
    bundles = find_bundles([Path(r).resolve() for r in roots])

    if not bundles:
        print(
            json.dumps(
                {
                    "schema_version": 1,
                    "bundles": [],
                    "aggregate": {
                        "bundle_count": 0,
                        "median_throughput_ticks_per_sec": 0.0,
                        "min_throughput_ticks_per_sec": 0.0,
                        "max_throughput_ticks_per_sec": 0.0,
                    },
                    "warning": "no bundles found",
                },
                indent=2,
            )
        )
        return 1

    results: list[dict[str, Any]] = []
    any_error = False
    for bundle in bundles:
        envelope = run_one(bundle)
        if envelope.get("result") != "ok":
            any_error = True
        results.append(envelope)

    throughputs = [
        r["throughput_ticks_per_sec"]
        for r in results
        if r.get("result") == "ok" and "throughput_ticks_per_sec" in r
    ]
    aggregate = {
        "bundle_count": len(results),
        "ok_count": sum(1 for r in results if r.get("result") == "ok"),
        "error_count": sum(1 for r in results if r.get("result") != "ok"),
    }
    if throughputs:
        aggregate["median_throughput_ticks_per_sec"] = statistics.median(throughputs)
        aggregate["min_throughput_ticks_per_sec"] = min(throughputs)
        aggregate["max_throughput_ticks_per_sec"] = max(throughputs)
    else:
        aggregate["median_throughput_ticks_per_sec"] = 0.0
        aggregate["min_throughput_ticks_per_sec"] = 0.0
        aggregate["max_throughput_ticks_per_sec"] = 0.0

    print(
        json.dumps(
            {
                "schema_version": 1,
                "bundles": results,
                "aggregate": aggregate,
            },
            indent=2,
        )
    )
    return 1 if any_error else 0


if __name__ == "__main__":
    raise SystemExit(main())
