#!/usr/bin/env python3
"""Report direct dependency drift and transitive duplicate crates.

This is intentionally repo-local and advisory by default. Corefall keeps exact
pins for compatibility-sensitive crates such as Bevy, but many Rust crates
legitimately pull duplicate transitive versions while upstream ecosystems move.
The report makes those facts visible in CI without treating every upstream
duplicate as a failure.
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
import tomllib
from dataclasses import dataclass
from pathlib import Path
from typing import Any


SEMVER_RE = re.compile(r"(\d+)\.(\d+)\.(\d+)")


@dataclass(frozen=True)
class DependencyRow:
    name: str
    requirement: str
    latest: str
    status: str
    note: str


def run_command(args: list[str], cwd: Path) -> tuple[int, str, str]:
    proc = subprocess.run(
        args,
        cwd=cwd,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    return proc.returncode, proc.stdout, proc.stderr


def load_workspace_dependencies(cargo_toml: Path) -> dict[str, Any]:
    data = tomllib.loads(cargo_toml.read_text(encoding="utf-8"))
    return data.get("workspace", {}).get("dependencies", {})


def dependency_requirement(spec: Any) -> str | None:
    if isinstance(spec, str):
        return spec
    if isinstance(spec, dict):
        version = spec.get("version")
        if isinstance(version, str):
            return version
    return None


def exact_semver(requirement: str) -> tuple[int, int, int] | None:
    match = SEMVER_RE.search(requirement)
    if not match:
        return None
    return tuple(int(part) for part in match.groups())


def latest_registry_version(name: str, workspace_root: Path) -> str | None:
    code, stdout, _stderr = run_command(["cargo", "search", name, "--limit", "5"], workspace_root)
    if code != 0:
        return None
    pattern = re.compile(rf"^{re.escape(name)}\s*=\s*\"([^\"]+)\"", re.MULTILINE)
    match = pattern.search(stdout)
    if not match:
        return None
    return match.group(1)


def build_dependency_rows(
    deps: dict[str, Any],
    workspace_root: Path,
) -> tuple[list[DependencyRow], list[str]]:
    rows: list[DependencyRow] = []
    warnings: list[str] = []

    for name in sorted(deps):
        requirement = dependency_requirement(deps[name])
        if requirement is None:
            rows.append(
                DependencyRow(
                    name=name,
                    requirement="path/git/workspace",
                    latest="n/a",
                    status="skipped",
                    note="not a registry version requirement",
                )
            )
            continue

        latest = latest_registry_version(name, workspace_root)
        if latest is None:
            rows.append(
                DependencyRow(
                    name=name,
                    requirement=requirement,
                    latest="unresolved",
                    status="warning",
                    note="cargo search could not resolve latest version",
                )
            )
            warnings.append(f"{name}: cargo search could not resolve latest version")
            continue

        requested_semver = exact_semver(requirement)
        latest_semver = exact_semver(latest)
        if requested_semver is None or latest_semver is None:
            rows.append(
                DependencyRow(
                    name=name,
                    requirement=requirement,
                    latest=latest,
                    status="range",
                    note="floating range; review intentionally broad pins manually",
                )
            )
            continue

        if requested_semver < latest_semver:
            status = "behind"
            note = "direct dependency has a newer registry release"
        elif requested_semver == latest_semver:
            status = "current"
            note = "matches latest registry release"
        else:
            status = "ahead"
            note = "requirement is newer than cargo search result"

        rows.append(
            DependencyRow(
                name=name,
                requirement=requirement,
                latest=latest,
                status=status,
                note=note,
            )
        )

    return rows, warnings


def duplicate_tree(workspace_root: Path) -> tuple[str, str | None]:
    code, stdout, stderr = run_command(["cargo", "tree", "-d"], workspace_root)
    text = stdout.strip()
    if code != 0:
        return "", stderr.strip() or "cargo tree -d failed"
    if not text:
        return "No duplicate transitive crate versions reported by cargo tree -d.", None
    return text, None


def trim_lines(text: str, line_limit: int) -> tuple[str, int]:
    lines = text.splitlines()
    if line_limit <= 0 or len(lines) <= line_limit:
        return text, 0
    omitted = len(lines) - line_limit
    return "\n".join(lines[:line_limit]), omitted


def render_markdown(
    rows: list[DependencyRow],
    duplicates: str,
    duplicate_error: str | None,
    duplicate_line_limit: int,
) -> str:
    behind = [row for row in rows if row.status == "behind"]
    warnings = [row for row in rows if row.status == "warning"]

    lines = [
        "# Dependency Drift Report",
        "",
        f"- Direct registry dependencies checked: {len(rows)}",
        f"- Behind latest: {len(behind)}",
        f"- Unresolved latest lookup warnings: {len(warnings)}",
        "",
        "## Direct Workspace Dependencies",
        "",
        "| Crate | Requirement | Latest | Status | Note |",
        "|---|---:|---:|---|---|",
    ]
    for row in rows:
        lines.append(
            f"| `{row.name}` | `{row.requirement}` | `{row.latest}` | {row.status} | {row.note} |"
        )

    lines.extend(
        [
            "",
            "## Transitive Duplicate Versions",
            "",
            "Duplicate transitive versions are review signals, not automatic failures. They are expected while upstream crates migrate at different speeds, especially around Bevy/wgpu/windows ecosystems.",
            "",
        ]
    )
    if duplicate_error:
        lines.extend(["```text", duplicate_error, "```"])
    else:
        duplicate_sample, omitted = trim_lines(duplicates, duplicate_line_limit)
        lines.extend(["```text", duplicate_sample, "```"])
        if omitted:
            lines.extend(
                [
                    "",
                    f"_Duplicate tree truncated after {duplicate_line_limit} lines; {omitted} more lines omitted. Run with `--duplicate-line-limit 0` for the full tree._",
                ]
            )

    return "\n".join(lines) + "\n"


def render_text(
    rows: list[DependencyRow],
    duplicates: str,
    duplicate_error: str | None,
    duplicate_line_limit: int,
) -> str:
    output = [f"{row.name}: {row.requirement} -> {row.latest} [{row.status}] {row.note}" for row in rows]
    output.append("")
    output.append("duplicate transitive versions:")
    if duplicate_error:
        output.append(duplicate_error)
    else:
        duplicate_sample, omitted = trim_lines(duplicates, duplicate_line_limit)
        output.append(duplicate_sample)
        if omitted:
            output.append(
                f"Duplicate tree truncated after {duplicate_line_limit} lines; {omitted} more lines omitted. "
                "Run with --duplicate-line-limit 0 for the full tree."
            )
    return "\n".join(output) + "\n"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--workspace-root",
        default=".",
        type=Path,
        help="Path to the Cargo workspace root, usually game/ or . from game/.",
    )
    parser.add_argument(
        "--format",
        choices=["markdown", "text"],
        default="text",
        help="Output format.",
    )
    parser.add_argument(
        "--deny-outdated",
        action="store_true",
        help="Exit non-zero when any direct dependency is behind latest.",
    )
    parser.add_argument(
        "--duplicate-line-limit",
        type=int,
        default=80,
        help="Maximum cargo tree -d lines to print; use 0 for the full duplicate tree.",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    workspace_root = args.workspace_root.resolve()
    cargo_toml = workspace_root / "Cargo.toml"
    if not cargo_toml.exists():
        print(f"ERROR: missing Cargo.toml at {cargo_toml}", file=sys.stderr)
        return 2

    deps = load_workspace_dependencies(cargo_toml)
    rows, warnings = build_dependency_rows(deps, workspace_root)
    duplicates, duplicate_error = duplicate_tree(workspace_root)

    if args.format == "markdown":
        print(render_markdown(rows, duplicates, duplicate_error, args.duplicate_line_limit), end="")
    else:
        print(render_text(rows, duplicates, duplicate_error, args.duplicate_line_limit), end="")

    if warnings:
        print("WARNING: some latest-version lookups failed; report is partial.", file=sys.stderr)

    if args.deny_outdated and any(row.status == "behind" for row in rows):
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
