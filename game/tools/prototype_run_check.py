#!/usr/bin/env python3
"""Validate a Slice A prototype run bundle."""

from __future__ import annotations

import argparse
from collections import Counter
import json
from pathlib import Path
from typing import Any

REQUIRED_FILES = ("run_manifest.json", "events.jsonl", "summary.json", "notes.md")
NOTE_HEADINGS = (
    "## Assumptions Tested",
    "## Good",
    "## Bad",
    "## Meh",
    "## Evidence Links",
    "## Next Actions",
)

MANIFEST_VERSION = "prototype-run-manifest.v0.1"
EVENT_VERSION = "prototype-recorder-event.v0.1"
SUMMARY_VERSION = "prototype-run-summary.v0.1"

MANIFEST_REQUIRED = (
    "schema_version",
    "run_id",
    "prototype_slice",
    "run_mode",
    "build",
    "scene",
    "seed",
    "started_at_utc",
    "duration_target_sec",
    "material_schema_version",
    "config_hash",
    "assumptions_tested",
    "linked_specs",
    "expected_tests",
    "capture_config",
)
EVENT_REQUIRED = (
    "schema_version",
    "run_id",
    "tick",
    "sim_time_ms",
    "event_id",
    "category",
    "event_type",
    "payload",
)

# M4 § Event envelope schema v0.1 is locked. The envelope MUST contain only the
# fields below; additive payload extensions are fine, but any envelope field
# addition requires a schema bump (v0.2).
EVENT_ENVELOPE_ALLOWED = {
    "schema_version",
    "run_id",
    "tick",
    "sim_time_ms",
    "event_id",
    "category",
    "event_type",
    "payload",
    "parent_event_id",
    "actor_id",
    "source_id",
    "team",
    "pos",
    "bbox",
    "dropped_count",
    "cosmetic",
}
SUMMARY_REQUIRED = (
    "schema_version",
    "run_id",
    "manifest_run_id",
    "duration_sec",
    "result",
    "tests",
    "event_counts",
    "volume",
    "performance",
    "artifacts",
    "blockers",
    "next_actions",
)


def load_json(path: Path) -> dict[str, Any]:
    with path.open() as f:
        data = json.load(f)
    if not isinstance(data, dict):
        raise TypeError(f"{path} must contain a JSON object")
    return data


def load_events(path: Path) -> list[dict[str, Any]]:
    events: list[dict[str, Any]] = []
    for line_no, line in enumerate(path.read_text().splitlines(), start=1):
        if not line.strip():
            continue
        try:
            data = json.loads(line)
        except json.JSONDecodeError as exc:
            raise ValueError(f"{path}:{line_no}: invalid JSON: {exc}") from exc
        if not isinstance(data, dict):
            raise TypeError(f"{path}:{line_no}: event must be a JSON object")
        events.append(data)
    return events


def require_fields(errors: list[str], label: str, data: dict[str, Any], fields: tuple[str, ...]) -> None:
    for field in fields:
        if field not in data:
            errors.append(f"{label} missing required field: {field}")


def expect_object(errors: list[str], label: str, value: Any) -> None:
    if not isinstance(value, dict):
        errors.append(f"{label} must be a JSON object")


def expect_list(errors: list[str], label: str, value: Any) -> None:
    if not isinstance(value, list):
        errors.append(f"{label} must be a JSON array")


def count_events(events: list[dict[str, Any]], field: str) -> Counter[str]:
    return Counter(str(event.get(field)) for event in events if event.get(field) is not None)


def compare_count_map(
    errors: list[str],
    label: str,
    actual: Counter[str],
    declared: Any,
) -> None:
    if declared is None:
        return
    if not isinstance(declared, dict):
        errors.append(f"{label} must be a JSON object")
        return

    clean_declared: dict[str, int] = {}
    invalid_value = False
    for key, value in declared.items():
        if not isinstance(value, int):
            errors.append(f"{label}.{key} must be an integer")
            invalid_value = True
            continue
        clean_declared[str(key)] = value

    if invalid_value:
        return

    if dict(sorted(actual.items())) != dict(sorted(clean_declared.items())):
        errors.append(f"{label} does not match events: expected {dict(sorted(actual.items()))}, got {clean_declared}")


def validate_run(run_dir: Path) -> list[str]:
    errors: list[str] = []

    if not run_dir.exists():
        return [f"run directory does not exist: {run_dir}"]
    if not run_dir.is_dir():
        return [f"run path is not a directory: {run_dir}"]

    for name in REQUIRED_FILES:
        if not (run_dir / name).exists():
            errors.append(f"missing required file: {name}")
    if errors:
        return errors

    try:
        manifest = load_json(run_dir / "run_manifest.json")
        summary = load_json(run_dir / "summary.json")
        events = load_events(run_dir / "events.jsonl")
        notes = (run_dir / "notes.md").read_text(errors="ignore")
    except (OSError, TypeError, ValueError, json.JSONDecodeError) as exc:
        return [str(exc)]

    require_fields(errors, "run_manifest.json", manifest, MANIFEST_REQUIRED)
    require_fields(errors, "summary.json", summary, SUMMARY_REQUIRED)

    if manifest.get("schema_version") != MANIFEST_VERSION:
        errors.append(f"run_manifest.json schema_version must be {MANIFEST_VERSION}")
    if summary.get("schema_version") != SUMMARY_VERSION:
        errors.append(f"summary.json schema_version must be {SUMMARY_VERSION}")
    if summary.get("manifest_run_id") != manifest.get("run_id"):
        errors.append("summary.json manifest_run_id must equal run_manifest.json run_id")

    expect_object(errors, "run_manifest.json.build", manifest.get("build"))
    expect_object(errors, "run_manifest.json.scene", manifest.get("scene"))
    expect_object(errors, "run_manifest.json.capture_config", manifest.get("capture_config"))
    expect_list(errors, "run_manifest.json.assumptions_tested", manifest.get("assumptions_tested"))
    expect_list(errors, "run_manifest.json.linked_specs", manifest.get("linked_specs"))
    expect_list(errors, "run_manifest.json.expected_tests", manifest.get("expected_tests"))

    expect_list(errors, "summary.json.tests", summary.get("tests"))
    expect_object(errors, "summary.json.event_counts", summary.get("event_counts"))
    expect_object(errors, "summary.json.volume", summary.get("volume"))
    expect_object(errors, "summary.json.performance", summary.get("performance"))
    expect_object(errors, "summary.json.artifacts", summary.get("artifacts"))
    expect_list(errors, "summary.json.blockers", summary.get("blockers"))
    expect_list(errors, "summary.json.next_actions", summary.get("next_actions"))

    run_id = manifest.get("run_id")
    event_ids: set[str] = set()
    previous_tick: int | float | None = None
    dropped_sum = 0

    for index, event in enumerate(events, start=1):
        label = f"events.jsonl line {index}"
        require_fields(errors, label, event, EVENT_REQUIRED)

        # M4: enforce envelope shape lock — reject unknown top-level fields.
        for k in event.keys():
            if k not in EVENT_ENVELOPE_ALLOWED:
                errors.append(
                    f"{label} envelope contains unknown field {k!r}; v0.1 envelope is locked"
                )

        if event.get("schema_version") != EVENT_VERSION:
            errors.append(f"{label} schema_version must be {EVENT_VERSION}")
        if event.get("run_id") != run_id:
            errors.append(f"{label} run_id must equal manifest run_id")

        event_id = event.get("event_id")
        if isinstance(event_id, str):
            if event_id in event_ids:
                errors.append(f"{label} duplicate event_id: {event_id}")
            event_ids.add(event_id)
        else:
            errors.append(f"{label} event_id must be a string")

        tick = event.get("tick")
        if not isinstance(tick, (int, float)):
            errors.append(f"{label} tick must be numeric")
        else:
            if previous_tick is not None and tick < previous_tick:
                errors.append(f"{label} tick is not monotonic")
            previous_tick = tick

        sim_time_ms = event.get("sim_time_ms")
        if not isinstance(sim_time_ms, (int, float)):
            errors.append(f"{label} sim_time_ms must be numeric")

        if not isinstance(event.get("payload"), dict):
            errors.append(f"{label} payload must be a JSON object")

        dropped_count = event.get("dropped_count", 0)
        if dropped_count is None:
            dropped_count = 0
        if not isinstance(dropped_count, int) or dropped_count < 0:
            errors.append(f"{label} dropped_count must be a non-negative integer")
        else:
            dropped_sum += dropped_count

    for index, event in enumerate(events, start=1):
        parent_id = event.get("parent_event_id")
        if not parent_id:
            continue
        if not isinstance(parent_id, str):
            errors.append(f"events.jsonl line {index} parent_event_id must be a string")
            continue
        if parent_id.startswith("external:"):
            continue
        if parent_id not in event_ids:
            errors.append(f"events.jsonl line {index} parent_event_id not found: {parent_id}")

    event_counts = summary.get("event_counts", {})
    if isinstance(event_counts, dict):
        if event_counts.get("total") != len(events):
            errors.append(f"summary.json event_counts.total must equal parsed event count {len(events)}")

        dropped_declared = event_counts.get("dropped_total", 0)
        if not isinstance(dropped_declared, int):
            errors.append("summary.json event_counts.dropped_total must be an integer")
        elif dropped_declared < dropped_sum:
            errors.append(
                "summary.json event_counts.dropped_total must be at least "
                f"the sum of event dropped_count values ({dropped_sum})"
            )

        compare_count_map(errors, "summary.json event_counts.by_category", count_events(events, "category"), event_counts.get("by_category"))
        compare_count_map(errors, "summary.json event_counts.by_type", count_events(events, "event_type"), event_counts.get("by_type"))

    tests = summary.get("tests", [])
    if isinstance(tests, list):
        for index, test in enumerate(tests, start=1):
            if not isinstance(test, dict):
                errors.append(f"summary.json tests[{index}] must be a JSON object")
                continue
            test_id = test.get("id")
            if not test_id:
                errors.append(f"summary.json tests[{index}] missing id")
            evidence_ids = test.get("evidence_event_ids", [])
            if not isinstance(evidence_ids, list):
                errors.append(f"summary.json tests[{index}].evidence_event_ids must be a JSON array")
                continue
            for evidence_id in evidence_ids:
                if evidence_id not in event_ids:
                    errors.append(f"summary.json test {test_id} cites missing evidence event: {evidence_id}")

    for heading in NOTE_HEADINGS:
        if heading not in notes:
            errors.append(f"notes.md missing heading: {heading}")

    # M3A-005: enforce the run_manifest.expected_outcome contract.
    expected_outcome = manifest.get("expected_outcome", "clean")
    if expected_outcome not in ("clean", "panic", "abort"):
        errors.append(
            "run_manifest.json expected_outcome must be one of "
            f"('clean', 'panic', 'abort'); found {expected_outcome!r}"
        )
    else:
        run_finished_count = sum(
            1 for e in events if e.get("category") == "system" and e.get("event_type") == "run_finished"
        )
        panic_count = sum(
            1 for e in events if e.get("category") == "system" and e.get("event_type") == "panic"
        )
        error_severity_count = 0
        if isinstance(event_counts, dict):
            by_severity = event_counts.get("by_severity") or {}
            if isinstance(by_severity, dict):
                error_severity_count = int(by_severity.get("error") or 0)
        if expected_outcome == "clean":
            # Devin BUG_pr-review-job 0001 (yellow): the documented Clean
            # contract in cf_replay::ExpectedOutcome says "MUST contain
            # exactly one system.run_finished event". A double-emit (e.g.
            # accidental record_run_finished + write_run_bundle finals) is
            # silent unless the checker enforces the upper bound too.
            if run_finished_count == 0:
                errors.append(
                    "run_manifest.expected_outcome=clean but events.jsonl has no system.run_finished event"
                )
            elif run_finished_count > 1:
                errors.append(
                    "run_manifest.expected_outcome=clean but events.jsonl has "
                    f"{run_finished_count} system.run_finished events; expected exactly one"
                )
            if panic_count > 0:
                errors.append(
                    "run_manifest.expected_outcome=clean but events.jsonl contains "
                    f"{panic_count} system.panic event(s); declare expected_outcome=panic"
                )
            if error_severity_count > 0:
                errors.append(
                    "run_manifest.expected_outcome=clean but summary.event_counts.by_severity.error="
                    f"{error_severity_count}; declare expected_outcome=abort"
                )
        elif expected_outcome == "panic":
            if panic_count == 0:
                errors.append(
                    "run_manifest.expected_outcome=panic but events.jsonl has no system.panic event"
                )
        # `abort` is intentionally permissive: by_severity.error may be > 0,
        # run_finished may or may not exist, but at least one must be present
        # so the bundle isn't silently empty.
        elif expected_outcome == "abort" and run_finished_count == 0 and panic_count == 0:
            errors.append(
                "run_manifest.expected_outcome=abort but events.jsonl has neither "
                "system.run_finished nor system.panic"
            )

    # M4 § Acceptance: system.category_baseline + system.run_started shape rules.
    category_baseline = next(
        (
            e for e in events
            if e.get("category") == "system" and e.get("event_type") == "category_baseline"
        ),
        None,
    )
    if category_baseline is None:
        errors.append(
            "events.jsonl missing system.category_baseline event (M4 § Event taxonomy)"
        )
    else:
        payload = category_baseline.get("payload") or {}
        categories = payload.get("categories")
        if not isinstance(categories, list) or len(categories) < 36:
            errors.append(
                "system.category_baseline must declare at least 36 categories "
                f"(M4 § Event taxonomy); found {len(categories) if isinstance(categories, list) else 0}"
            )
        else:
            required_new = {
                "hazard",
                "shield",
                "thermal",
                "environment",
                "armor",
                "internal",
                "concussion",
                "fluid",
                "origin",
                "module",
                "resource",
                "ability",
            }
            seen = {c.get("name") for c in categories if isinstance(c, dict)}
            missing = required_new - seen
            if missing:
                errors.append(
                    "system.category_baseline missing M9/M13/M17 categories: "
                    f"{sorted(missing)}"
                )
            for c in categories:
                if not isinstance(c, dict):
                    continue
                status = c.get("status")
                if status == "active" and "first_event_type" not in c:
                    errors.append(
                        f"system.category_baseline.{c.get('name')!r}: "
                        "active category missing first_event_type"
                    )
                if status == "registered" and "ladder_at" not in c:
                    errors.append(
                        f"system.category_baseline.{c.get('name')!r}: "
                        "registered category missing ladder_at"
                    )

    run_started = next(
        (
            e for e in events
            if e.get("category") == "system" and e.get("event_type") == "run_started"
        ),
        None,
    )
    if run_started is None:
        errors.append(
            "events.jsonl missing system.run_started event (M4 § Expected outcome + system events)"
        )
    else:
        rs_payload = run_started.get("payload") or {}
        for f in ("protocol_version", "manifest_hash", "build_id", "scenario_id", "seed", "tick_rate_hz"):
            if f not in rs_payload:
                errors.append(
                    f"system.run_started payload missing required field {f!r} (M4 § Expected outcome)"
                )

    # LLM-graded test verdict (optional, gated on grading.json being present).
    # When the bundle has a grading.json artifact, the AI agent has produced
    # an LLM-graded verdict per `.claude/skills/corefall-review/SKILL.md`
    # §LLM-Graded Test Verdicts. We validate the shape so a malformed grading
    # file is caught at run-bundle time, not at review time. Bundles WITHOUT
    # grading.json are not flagged here — the gate fires in /corefall-review,
    # not in the per-bundle checker.
    grading_path = run_dir / "grading.json"
    if grading_path.is_file():
        try:
            grading = json.loads(grading_path.read_text())
        except json.JSONDecodeError as exc:
            errors.append(f"grading.json is not valid JSON: {exc}")
        else:
            if grading.get("schema_version") != "cf-grading.v1":
                errors.append(
                    f"grading.json schema_version must be 'cf-grading.v1'; "
                    f"found {grading.get('schema_version')!r}"
                )
            for required in ("scenario_id", "agent", "timestamp", "dimensions"):
                if not grading.get(required):
                    errors.append(f"grading.json missing required field {required!r}")
            dims = grading.get("dimensions") or []
            if not isinstance(dims, list) or not dims:
                errors.append("grading.json dimensions must be a non-empty list")
            else:
                for idx, dim in enumerate(dims):
                    label = f"grading.json dimensions[{idx}]"
                    if not isinstance(dim, dict):
                        errors.append(f"{label} must be an object")
                        continue
                    for f in ("id", "criterion", "score", "prose", "verdict"):
                        if f not in dim:
                            errors.append(f"{label} missing field {f!r}")
                    score = dim.get("score")
                    if score is not None and not isinstance(score, (int, float)):
                        errors.append(f"{label} score must be numeric or null")

    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("run_dir", type=Path, help="Directory containing run_manifest.json, events.jsonl, summary.json, and notes.md")
    args = parser.parse_args()

    errors = validate_run(args.run_dir)
    if errors:
        print(f"run_dir {args.run_dir}")
        print(f"errors {len(errors)}")
        for error in errors:
            print(f"- {error}")
        return 1

    manifest = load_json(args.run_dir / "run_manifest.json")
    summary = load_json(args.run_dir / "summary.json")
    events = load_events(args.run_dir / "events.jsonl")
    print(f"run_id {manifest.get('run_id')}")
    print(f"events {len(events)}")
    print(f"tests {len(summary.get('tests', []))}")
    print("errors 0")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
