#!/usr/bin/env python3
"""Validate a Slice A prototype run bundle.

Emits either a human-friendly summary (default) or a structured JSON line
(`--json`). Errors carry a canonical rule token from the M4 spec § "12
cross-file rules" plus rule-specific context fields (e.g. ``file``,
``event_id``, ``tick``, ``previous_tick``, ``parent_event_id``).

Use ``--self-test`` to run a built-in fixture suite that covers each rule.
"""

from __future__ import annotations

import argparse
from collections import Counter
import json
from pathlib import Path
from typing import Any, Iterable

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
    "asset_ref",
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

# M4 spec § "12 cross-file rules" — the canonical set of rule tokens. Any rule
# the checker emits must match one of these names if it covers one of the 12
# canonical conditions. Extra rule names below (e.g. ``payload_invalid_type``)
# cover additional shape checks not enumerated by the 12-rule contract but
# preserved from earlier milestones.
CANONICAL_RULES: tuple[str, ...] = (
    "missing_file",
    "schema_version_mismatch",
    "run_id_mismatch",
    "duplicate_event_id",
    "non_monotonic_ticks",
    "parent_event_missing",
    "event_count_mismatch",
    "category_count_mismatch",
    "dropped_total_underflow",
    "evidence_event_missing",
    "missing_notes_heading",
    "expected_outcome_mismatch",
)


def _err(rule: str, message: str, **kwargs: Any) -> dict[str, Any]:
    out: dict[str, Any] = {"rule": rule, "message": message}
    out.update(kwargs)
    return out


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


def _require_fields(
    errors: list[dict[str, Any]],
    label: str,
    data: dict[str, Any],
    fields: Iterable[str],
) -> None:
    for field in fields:
        if field not in data:
            errors.append(_err(
                "missing_field",
                f"{label} missing required field: {field}",
                file=label,
                field=field,
            ))


def _expect_object(errors: list[dict[str, Any]], label: str, value: Any) -> None:
    if not isinstance(value, dict):
        errors.append(_err(
            "payload_invalid_type",
            f"{label} must be a JSON object",
            location=label,
            expected_type="object",
        ))


def _expect_list(errors: list[dict[str, Any]], label: str, value: Any) -> None:
    if not isinstance(value, list):
        errors.append(_err(
            "payload_invalid_type",
            f"{label} must be a JSON array",
            location=label,
            expected_type="array",
        ))


def _count_events(events: list[dict[str, Any]], field: str) -> Counter[str]:
    return Counter(str(event.get(field)) for event in events if event.get(field) is not None)


def _compare_count_map(
    errors: list[dict[str, Any]],
    label: str,
    rule: str,
    actual: Counter[str],
    declared: Any,
) -> None:
    if declared is None:
        return
    if not isinstance(declared, dict):
        errors.append(_err(
            "payload_invalid_type",
            f"{label} must be a JSON object",
            location=label,
            expected_type="object",
        ))
        return

    clean_declared: dict[str, int] = {}
    invalid_value = False
    for key, value in declared.items():
        if not isinstance(value, int):
            errors.append(_err(
                "payload_invalid_type",
                f"{label}.{key} must be an integer",
                location=f"{label}.{key}",
                expected_type="integer",
            ))
            invalid_value = True
            continue
        clean_declared[str(key)] = value

    if invalid_value:
        return

    actual_sorted = dict(sorted(actual.items()))
    declared_sorted = dict(sorted(clean_declared.items()))
    if actual_sorted != declared_sorted:
        errors.append(_err(
            rule,
            f"{label} does not match events: expected {actual_sorted}, got {declared_sorted}",
            location=label,
            expected=actual_sorted,
            actual=declared_sorted,
        ))


def validate_bundle_data(
    manifest: dict[str, Any],
    summary: dict[str, Any],
    events: list[dict[str, Any]],
    notes: str,
    *,
    grading: dict[str, Any] | None = None,
) -> list[dict[str, Any]]:
    """Validate pre-parsed bundle data and return structured errors.

    Each error is a dict with at least ``{rule, message}`` plus rule-specific
    context fields (e.g. ``file``, ``line``, ``event_id``). See
    ``CANONICAL_RULES`` for the canonical M4 rule tokens.
    """
    errors: list[dict[str, Any]] = []

    _require_fields(errors, "run_manifest.json", manifest, MANIFEST_REQUIRED)
    _require_fields(errors, "summary.json", summary, SUMMARY_REQUIRED)

    if manifest.get("schema_version") != MANIFEST_VERSION:
        errors.append(_err(
            "schema_version_mismatch",
            f"run_manifest.json schema_version must be {MANIFEST_VERSION}",
            file="run_manifest.json",
            expected=MANIFEST_VERSION,
            actual=manifest.get("schema_version"),
        ))
    if summary.get("schema_version") != SUMMARY_VERSION:
        errors.append(_err(
            "schema_version_mismatch",
            f"summary.json schema_version must be {SUMMARY_VERSION}",
            file="summary.json",
            expected=SUMMARY_VERSION,
            actual=summary.get("schema_version"),
        ))
    if summary.get("manifest_run_id") != manifest.get("run_id"):
        errors.append(_err(
            "run_id_mismatch",
            "summary.json manifest_run_id must equal run_manifest.json run_id",
            file="summary.json",
            expected=manifest.get("run_id"),
            actual=summary.get("manifest_run_id"),
        ))

    _expect_object(errors, "run_manifest.json.build", manifest.get("build"))
    _expect_object(errors, "run_manifest.json.scene", manifest.get("scene"))
    _expect_object(errors, "run_manifest.json.capture_config", manifest.get("capture_config"))
    _expect_list(errors, "run_manifest.json.assumptions_tested", manifest.get("assumptions_tested"))
    _expect_list(errors, "run_manifest.json.linked_specs", manifest.get("linked_specs"))
    _expect_list(errors, "run_manifest.json.expected_tests", manifest.get("expected_tests"))

    _expect_list(errors, "summary.json.tests", summary.get("tests"))
    _expect_object(errors, "summary.json.event_counts", summary.get("event_counts"))
    _expect_object(errors, "summary.json.volume", summary.get("volume"))
    _expect_object(errors, "summary.json.performance", summary.get("performance"))
    _expect_object(errors, "summary.json.artifacts", summary.get("artifacts"))
    _expect_list(errors, "summary.json.blockers", summary.get("blockers"))
    _expect_list(errors, "summary.json.next_actions", summary.get("next_actions"))

    run_id = manifest.get("run_id")
    event_ids: set[str] = set()
    previous_tick: int | float | None = None
    dropped_sum = 0

    for index, event in enumerate(events, start=1):
        label = f"events.jsonl line {index}"

        for field in EVENT_REQUIRED:
            if field not in event:
                errors.append(_err(
                    "missing_field",
                    f"{label} missing required field: {field}",
                    file="events.jsonl",
                    line=index,
                    field=field,
                ))

        # M4: enforce envelope shape lock — reject unknown top-level fields.
        for k in event.keys():
            if k not in EVENT_ENVELOPE_ALLOWED:
                errors.append(_err(
                    "envelope_unknown_field",
                    f"{label} envelope contains unknown field {k!r}; v0.1 envelope is locked",
                    file="events.jsonl",
                    line=index,
                    field=k,
                ))

        if event.get("schema_version") != EVENT_VERSION:
            errors.append(_err(
                "schema_version_mismatch",
                f"{label} schema_version must be {EVENT_VERSION}",
                file="events.jsonl",
                line=index,
                expected=EVENT_VERSION,
                actual=event.get("schema_version"),
            ))
        if event.get("run_id") != run_id:
            errors.append(_err(
                "run_id_mismatch",
                f"{label} run_id must equal manifest run_id",
                file="events.jsonl",
                line=index,
                expected=run_id,
                actual=event.get("run_id"),
            ))

        event_id = event.get("event_id")
        if isinstance(event_id, str):
            if event_id in event_ids:
                errors.append(_err(
                    "duplicate_event_id",
                    f"{label} duplicate event_id: {event_id}",
                    file="events.jsonl",
                    line=index,
                    event_id=event_id,
                ))
            event_ids.add(event_id)
        else:
            errors.append(_err(
                "payload_invalid_type",
                f"{label} event_id must be a string",
                file="events.jsonl",
                line=index,
                location="event_id",
                expected_type="string",
            ))

        tick = event.get("tick")
        if not isinstance(tick, (int, float)):
            errors.append(_err(
                "payload_invalid_type",
                f"{label} tick must be numeric",
                file="events.jsonl",
                line=index,
                location="tick",
                expected_type="number",
            ))
        else:
            if previous_tick is not None and tick < previous_tick:
                errors.append(_err(
                    "non_monotonic_ticks",
                    f"{label} tick is not monotonic",
                    file="events.jsonl",
                    line=index,
                    tick=tick,
                    previous_tick=previous_tick,
                ))
            previous_tick = tick

        sim_time_ms = event.get("sim_time_ms")
        if not isinstance(sim_time_ms, (int, float)):
            errors.append(_err(
                "payload_invalid_type",
                f"{label} sim_time_ms must be numeric",
                file="events.jsonl",
                line=index,
                location="sim_time_ms",
                expected_type="number",
            ))

        if not isinstance(event.get("payload"), dict):
            errors.append(_err(
                "payload_invalid_type",
                f"{label} payload must be a JSON object",
                file="events.jsonl",
                line=index,
                location="payload",
                expected_type="object",
            ))

        dropped_count = event.get("dropped_count", 0)
        if dropped_count is None:
            dropped_count = 0
        if not isinstance(dropped_count, int) or dropped_count < 0:
            errors.append(_err(
                "payload_invalid_type",
                f"{label} dropped_count must be a non-negative integer",
                file="events.jsonl",
                line=index,
                location="dropped_count",
                expected_type="non-negative integer",
            ))
        else:
            dropped_sum += dropped_count

    for index, event in enumerate(events, start=1):
        parent_id = event.get("parent_event_id")
        if not parent_id:
            continue
        if not isinstance(parent_id, str):
            errors.append(_err(
                "payload_invalid_type",
                f"events.jsonl line {index} parent_event_id must be a string",
                file="events.jsonl",
                line=index,
                location="parent_event_id",
                expected_type="string",
            ))
            continue
        if parent_id.startswith("external:"):
            continue
        if parent_id not in event_ids:
            errors.append(_err(
                "parent_event_missing",
                f"events.jsonl line {index} parent_event_id not found: {parent_id}",
                file="events.jsonl",
                line=index,
                parent_event_id=parent_id,
            ))

    event_counts = summary.get("event_counts", {})
    if isinstance(event_counts, dict):
        if event_counts.get("total") != len(events):
            errors.append(_err(
                "event_count_mismatch",
                f"summary.json event_counts.total must equal parsed event count {len(events)}",
                file="summary.json",
                location="event_counts.total",
                expected=len(events),
                actual=event_counts.get("total"),
            ))

        dropped_declared = event_counts.get("dropped_total", 0)
        if not isinstance(dropped_declared, int):
            errors.append(_err(
                "payload_invalid_type",
                "summary.json event_counts.dropped_total must be an integer",
                file="summary.json",
                location="event_counts.dropped_total",
                expected_type="integer",
            ))
        elif dropped_declared < dropped_sum:
            errors.append(_err(
                "dropped_total_underflow",
                "summary.json event_counts.dropped_total must be at least "
                f"the sum of event dropped_count values ({dropped_sum})",
                file="summary.json",
                location="event_counts.dropped_total",
                expected_min=dropped_sum,
                actual=dropped_declared,
            ))

        _compare_count_map(
            errors,
            "summary.json event_counts.by_category",
            "category_count_mismatch",
            _count_events(events, "category"),
            event_counts.get("by_category"),
        )
        _compare_count_map(
            errors,
            "summary.json event_counts.by_type",
            "event_type_count_mismatch",
            _count_events(events, "event_type"),
            event_counts.get("by_type"),
        )

    tests = summary.get("tests", [])
    if isinstance(tests, list):
        for index, test in enumerate(tests, start=1):
            if not isinstance(test, dict):
                errors.append(_err(
                    "payload_invalid_type",
                    f"summary.json tests[{index}] must be a JSON object",
                    file="summary.json",
                    location=f"tests[{index}]",
                    expected_type="object",
                ))
                continue
            test_id = test.get("id")
            if not test_id:
                errors.append(_err(
                    "missing_field",
                    f"summary.json tests[{index}] missing id",
                    file="summary.json",
                    location=f"tests[{index}]",
                    field="id",
                ))
            evidence_ids = test.get("evidence_event_ids", [])
            if not isinstance(evidence_ids, list):
                errors.append(_err(
                    "payload_invalid_type",
                    f"summary.json tests[{index}].evidence_event_ids must be a JSON array",
                    file="summary.json",
                    location=f"tests[{index}].evidence_event_ids",
                    expected_type="array",
                ))
                continue
            for evidence_id in evidence_ids:
                if evidence_id not in event_ids:
                    errors.append(_err(
                        "evidence_event_missing",
                        f"summary.json test {test_id} cites missing evidence event: {evidence_id}",
                        file="summary.json",
                        test_id=test_id,
                        evidence_id=evidence_id,
                    ))

    for heading in NOTE_HEADINGS:
        if heading not in notes:
            errors.append(_err(
                "missing_notes_heading",
                f"notes.md missing heading: {heading}",
                file="notes.md",
                heading=heading,
            ))

    # M3A-005: enforce the run_manifest.expected_outcome contract.
    expected_outcome = manifest.get("expected_outcome", "clean")
    if expected_outcome not in ("clean", "panic", "abort"):
        errors.append(_err(
            "invalid_expected_outcome_value",
            "run_manifest.json expected_outcome must be one of "
            f"('clean', 'panic', 'abort'); found {expected_outcome!r}",
            file="run_manifest.json",
            location="expected_outcome",
            actual=expected_outcome,
        ))
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
                errors.append(_err(
                    "expected_outcome_mismatch",
                    "run_manifest.expected_outcome=clean but events.jsonl has no system.run_finished event",
                    expected="clean",
                    actual="no_run_finished",
                ))
            elif run_finished_count > 1:
                errors.append(_err(
                    "expected_outcome_mismatch",
                    "run_manifest.expected_outcome=clean but events.jsonl has "
                    f"{run_finished_count} system.run_finished events; expected exactly one",
                    expected="clean",
                    actual="multiple_run_finished",
                    run_finished_count=run_finished_count,
                ))
            if panic_count > 0:
                errors.append(_err(
                    "expected_outcome_mismatch",
                    "run_manifest.expected_outcome=clean but events.jsonl contains "
                    f"{panic_count} system.panic event(s); declare expected_outcome=panic",
                    expected="clean",
                    actual="panic",
                    panic_count=panic_count,
                ))
            if error_severity_count > 0:
                errors.append(_err(
                    "expected_outcome_mismatch",
                    "run_manifest.expected_outcome=clean but summary.event_counts.by_severity.error="
                    f"{error_severity_count}; declare expected_outcome=abort",
                    expected="clean",
                    actual="abort",
                    error_severity_count=error_severity_count,
                ))
        elif expected_outcome == "panic":
            if panic_count == 0:
                errors.append(_err(
                    "expected_outcome_mismatch",
                    "run_manifest.expected_outcome=panic but events.jsonl has no system.panic event",
                    expected="panic",
                    actual="no_panic",
                ))
        # `abort` is intentionally permissive: by_severity.error may be > 0,
        # run_finished may or may not exist, but at least one must be present
        # so the bundle isn't silently empty.
        elif expected_outcome == "abort" and run_finished_count == 0 and panic_count == 0:
            errors.append(_err(
                "expected_outcome_mismatch",
                "run_manifest.expected_outcome=abort but events.jsonl has neither "
                "system.run_finished nor system.panic",
                expected="abort",
                actual="empty",
            ))

    # M4 § Acceptance: system.category_baseline + system.run_started shape rules.
    category_baseline = next(
        (
            e for e in events
            if e.get("category") == "system" and e.get("event_type") == "category_baseline"
        ),
        None,
    )
    if category_baseline is None:
        errors.append(_err(
            "missing_category_baseline_event",
            "events.jsonl missing system.category_baseline event (M4 § Event taxonomy)",
            file="events.jsonl",
        ))
    else:
        payload = category_baseline.get("payload") or {}
        categories = payload.get("categories")
        if not isinstance(categories, list) or len(categories) < 36:
            errors.append(_err(
                "category_baseline_invalid",
                "system.category_baseline must declare at least 36 categories "
                f"(M4 § Event taxonomy); found {len(categories) if isinstance(categories, list) else 0}",
                file="events.jsonl",
                expected_min_count=36,
                actual=len(categories) if isinstance(categories, list) else 0,
            ))
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
                errors.append(_err(
                    "category_baseline_invalid",
                    "system.category_baseline missing M9/M13/M17 categories: "
                    f"{sorted(missing)}",
                    file="events.jsonl",
                    missing_categories=sorted(missing),
                ))
            for c in categories:
                if not isinstance(c, dict):
                    continue
                status = c.get("status")
                if status == "active" and "first_event_type" not in c:
                    errors.append(_err(
                        "category_baseline_invalid",
                        f"system.category_baseline.{c.get('name')!r}: "
                        "active category missing first_event_type",
                        file="events.jsonl",
                        category=c.get("name"),
                        missing_field="first_event_type",
                    ))
                if status == "registered" and "ladder_at" not in c:
                    errors.append(_err(
                        "category_baseline_invalid",
                        f"system.category_baseline.{c.get('name')!r}: "
                        "registered category missing ladder_at",
                        file="events.jsonl",
                        category=c.get("name"),
                        missing_field="ladder_at",
                    ))

    run_started = next(
        (
            e for e in events
            if e.get("category") == "system" and e.get("event_type") == "run_started"
        ),
        None,
    )
    if run_started is None:
        errors.append(_err(
            "missing_run_started_event",
            "events.jsonl missing system.run_started event (M4 § Expected outcome + system events)",
            file="events.jsonl",
        ))
    else:
        rs_payload = run_started.get("payload") or {}
        for f in ("protocol_version", "manifest_hash", "build_id", "scenario_id", "seed", "tick_rate_hz"):
            if f not in rs_payload:
                errors.append(_err(
                    "run_started_invalid",
                    f"system.run_started payload missing required field {f!r} (M4 § Expected outcome)",
                    file="events.jsonl",
                    field=f,
                ))

    # LLM-graded test verdict (optional, gated on grading.json being present).
    # When the bundle has a grading.json artifact, the AI agent has produced
    # an LLM-graded verdict per `.claude/skills/corefall-review/SKILL.md`
    # §LLM-Graded Test Verdicts. We validate the shape so a malformed grading
    # file is caught at run-bundle time, not at review time. Bundles WITHOUT
    # grading.json are not flagged here — the gate fires in /corefall-review,
    # not in the per-bundle checker.
    if grading is not None:
        if grading.get("schema_version") != "cf-grading.v1":
            errors.append(_err(
                "invalid_grading",
                f"grading.json schema_version must be 'cf-grading.v1'; "
                f"found {grading.get('schema_version')!r}",
                file="grading.json",
                expected="cf-grading.v1",
                actual=grading.get("schema_version"),
            ))
        for required in ("scenario_id", "agent", "timestamp", "dimensions"):
            if not grading.get(required):
                errors.append(_err(
                    "missing_field",
                    f"grading.json missing required field {required!r}",
                    file="grading.json",
                    field=required,
                ))
        dims = grading.get("dimensions") or []
        if not isinstance(dims, list) or not dims:
            errors.append(_err(
                "invalid_grading",
                "grading.json dimensions must be a non-empty list",
                file="grading.json",
                location="dimensions",
            ))
        else:
            for idx, dim in enumerate(dims):
                dim_label = f"grading.json dimensions[{idx}]"
                if not isinstance(dim, dict):
                    errors.append(_err(
                        "payload_invalid_type",
                        f"{dim_label} must be an object",
                        file="grading.json",
                        location=f"dimensions[{idx}]",
                        expected_type="object",
                    ))
                    continue
                for f in ("id", "criterion", "score", "prose", "verdict"):
                    if f not in dim:
                        errors.append(_err(
                            "missing_field",
                            f"{dim_label} missing field {f!r}",
                            file="grading.json",
                            location=f"dimensions[{idx}]",
                            field=f,
                        ))
                score = dim.get("score")
                if score is not None and not isinstance(score, (int, float)):
                    errors.append(_err(
                        "payload_invalid_type",
                        f"{dim_label} score must be numeric or null",
                        file="grading.json",
                        location=f"dimensions[{idx}].score",
                        expected_type="number_or_null",
                    ))

    return errors


def validate_run(run_dir: Path) -> list[dict[str, Any]]:
    """Validate a run bundle on disk; returns a list of structured error dicts."""
    if not run_dir.exists():
        return [_err(
            "missing_file",
            f"run directory does not exist: {run_dir}",
            file=str(run_dir),
        )]
    if not run_dir.is_dir():
        return [_err(
            "missing_file",
            f"run path is not a directory: {run_dir}",
            file=str(run_dir),
        )]

    errors: list[dict[str, Any]] = []
    for name in REQUIRED_FILES:
        if not (run_dir / name).exists():
            errors.append(_err(
                "missing_file",
                f"missing required file: {name}",
                file=name,
            ))
    if errors:
        return errors

    try:
        manifest = load_json(run_dir / "run_manifest.json")
        summary = load_json(run_dir / "summary.json")
        events = load_events(run_dir / "events.jsonl")
        notes = (run_dir / "notes.md").read_text(errors="ignore")
    except (OSError, TypeError, ValueError, json.JSONDecodeError) as exc:
        return [_err("invalid_json", str(exc), file=str(run_dir))]

    grading: dict[str, Any] | None = None
    grading_path = run_dir / "grading.json"
    if grading_path.is_file():
        try:
            grading = json.loads(grading_path.read_text())
        except json.JSONDecodeError as exc:
            errors.append(_err(
                "invalid_grading",
                f"grading.json is not valid JSON: {exc}",
                file="grading.json",
            ))

    errors.extend(validate_bundle_data(manifest, summary, events, notes, grading=grading))
    return errors


def _self_test_cases() -> list[tuple[str, set[str], dict, dict, list, str]]:
    """Build the inline fixture suite. Returns
    ``[(name, expected_rules, manifest, summary, events, notes), ...]``."""
    base_run_id = "selftest_run_id_aaaa"

    def good_manifest() -> dict[str, Any]:
        return {
            "schema_version": MANIFEST_VERSION,
            "run_id": base_run_id,
            "prototype_slice": "A",
            "run_mode": "headless",
            "build": {},
            "scene": {},
            "seed": 42,
            "started_at_utc": "2026-01-01T00:00:00Z",
            "duration_target_sec": 1.0,
            "material_schema_version": "v1",
            "config_hash": "hash",
            "assumptions_tested": [],
            "linked_specs": [],
            "expected_tests": [],
            "capture_config": {},
            "expected_outcome": "clean",
        }

    def good_event(**overrides: Any) -> dict[str, Any]:
        ev: dict[str, Any] = {
            "schema_version": EVENT_VERSION,
            "run_id": base_run_id,
            "tick": 0,
            "sim_time_ms": 0,
            "event_id": "evt_0",
            "category": "system",
            "event_type": "noop",
            "payload": {},
        }
        ev.update(overrides)
        return ev

    def baseline_event() -> dict[str, Any]:
        categories = [
            {"name": n, "status": "registered", "ladder_at": "M99"}
            for n in (
                "input", "control", "mind", "collision", "server", "anti_cheat",
                "mmo", "material", "reaction", "atmospherics", "affliction",
                "hazard", "shield", "thermal", "environment", "armor", "module",
                "resource", "internal", "concussion", "fluid", "origin", "combat",
                "body", "terrain", "ai", "logistics", "mission", "system",
                "snapshot", "determinism", "ux", "accessibility", "performance",
                "equipment", "chassis", "actor", "ability",
            )
        ]
        return good_event(
            event_id="evt_baseline",
            event_type="category_baseline",
            payload={"schema_version": 1, "categories": categories},
        )

    def run_started_event() -> dict[str, Any]:
        return good_event(
            event_id="evt_run_started",
            event_type="run_started",
            payload={
                "protocol_version": 1,
                "manifest_hash": "hash",
                "build_id": "build",
                "scenario_id": "test",
                "seed": 42,
                "tick_rate_hz": 60,
            },
        )

    def run_finished_event() -> dict[str, Any]:
        return good_event(
            event_id="evt_run_finished",
            event_type="run_finished",
            payload={"outcome": "clean", "ticks_run": 60, "wall_seconds": 1.0},
            tick=60,
            sim_time_ms=1000,
        )

    def good_events() -> list[dict[str, Any]]:
        return [run_started_event(), baseline_event(), run_finished_event()]

    def good_notes() -> str:
        return "\n".join(NOTE_HEADINGS) + "\n"

    def good_summary(events: list[dict[str, Any]]) -> dict[str, Any]:
        by_category: dict[str, int] = {}
        by_type: dict[str, int] = {}
        for e in events:
            by_category[e["category"]] = by_category.get(e["category"], 0) + 1
            by_type[e["event_type"]] = by_type.get(e["event_type"], 0) + 1
        return {
            "schema_version": SUMMARY_VERSION,
            "run_id": base_run_id,
            "manifest_run_id": base_run_id,
            "duration_sec": 1.0,
            "result": "ok",
            "tests": [],
            "event_counts": {
                "total": len(events),
                "by_category": by_category,
                "by_type": by_type,
                "dropped_total": 0,
            },
            "volume": {},
            "performance": {},
            "artifacts": {},
            "blockers": [],
            "next_actions": [],
        }

    cases: list[tuple[str, set[str], dict, dict, list, str]] = []

    events = good_events()
    cases.append(("good_bundle", set(), good_manifest(), good_summary(events), events, good_notes()))

    events_a = good_events()
    cases.append((
        "schema_version_mismatch",
        {"schema_version_mismatch"},
        {**good_manifest(), "schema_version": "bogus"},
        good_summary(events_a),
        events_a,
        good_notes(),
    ))

    events_b = good_events()
    sum_b = good_summary(events_b)
    sum_b["manifest_run_id"] = "different_run_id"
    cases.append((
        "run_id_mismatch",
        {"run_id_mismatch"},
        good_manifest(),
        sum_b,
        events_b,
        good_notes(),
    ))

    events_c = good_events()
    events_c[1]["event_id"] = events_c[0]["event_id"]
    cases.append((
        "duplicate_event_id",
        {"duplicate_event_id"},
        good_manifest(),
        good_summary(events_c),
        events_c,
        good_notes(),
    ))

    events_d = good_events()
    events_d[0]["tick"] = 100
    events_d[1]["tick"] = 50
    cases.append((
        "non_monotonic_ticks",
        {"non_monotonic_ticks"},
        good_manifest(),
        good_summary(events_d),
        events_d,
        good_notes(),
    ))

    events_e = good_events()
    events_e.append(good_event(
        event_id="evt_orphan",
        parent_event_id="evt_nonexistent",
        tick=70,
        sim_time_ms=1100,
        event_type="orphan_event",
    ))
    cases.append((
        "parent_event_missing",
        {"parent_event_missing"},
        good_manifest(),
        good_summary(events_e),
        events_e,
        good_notes(),
    ))

    events_f = good_events()
    sum_f = good_summary(events_f)
    sum_f["event_counts"]["total"] = 999
    cases.append((
        "event_count_mismatch",
        {"event_count_mismatch"},
        good_manifest(),
        sum_f,
        events_f,
        good_notes(),
    ))

    events_g = good_events()
    sum_g = good_summary(events_g)
    sum_g["event_counts"]["by_category"] = {"bogus_category": 99}
    cases.append((
        "category_count_mismatch",
        {"category_count_mismatch"},
        good_manifest(),
        sum_g,
        events_g,
        good_notes(),
    ))

    events_h = good_events()
    events_h[0]["dropped_count"] = 10
    sum_h = good_summary(events_h)
    sum_h["event_counts"]["dropped_total"] = 1
    cases.append((
        "dropped_total_underflow",
        {"dropped_total_underflow"},
        good_manifest(),
        sum_h,
        events_h,
        good_notes(),
    ))

    events_i = good_events()
    sum_i = good_summary(events_i)
    sum_i["tests"] = [{"id": "test_x", "evidence_event_ids": ["evt_does_not_exist"]}]
    cases.append((
        "evidence_event_missing",
        {"evidence_event_missing"},
        good_manifest(),
        sum_i,
        events_i,
        good_notes(),
    ))

    events_j = good_events()
    cases.append((
        "missing_notes_heading",
        {"missing_notes_heading"},
        good_manifest(),
        good_summary(events_j),
        events_j,
        good_notes().replace("## Good\n", ""),
    ))

    events_k = good_events()
    events_k.append(good_event(
        event_id="evt_panic",
        category="system",
        event_type="panic",
        tick=70,
        sim_time_ms=1200,
    ))
    cases.append((
        "expected_outcome_mismatch",
        {"expected_outcome_mismatch"},
        good_manifest(),
        good_summary(events_k),
        events_k,
        good_notes(),
    ))

    events_l = good_events()
    events_l[0]["extra_envelope_field"] = "nope"
    cases.append((
        "envelope_unknown_field",
        {"envelope_unknown_field"},
        good_manifest(),
        good_summary(events_l),
        events_l,
        good_notes(),
    ))

    return cases


def _run_self_test() -> tuple[int, int]:
    passes = 0
    failures = 0
    for name, expected_rules, manifest, summary, events, notes in _self_test_cases():
        errs = validate_bundle_data(manifest, summary, events, notes)
        actual_rules = {e["rule"] for e in errs}
        if expected_rules == set():
            ok = errs == []
        else:
            ok = expected_rules.issubset(actual_rules)
        status = "PASS" if ok else "FAIL"
        print(
            f"{status} {name} "
            f"expected={sorted(expected_rules)} got={sorted(actual_rules)}"
        )
        if ok:
            passes += 1
        else:
            failures += 1
            for e in errs:
                print(f"    - {e}")

    missing_path = Path("/tmp/__prototype_run_check_selftest_does_not_exist__")
    errs = validate_run(missing_path)
    actual_rules = {e["rule"] for e in errs}
    ok = actual_rules == {"missing_file"}
    status = "PASS" if ok else "FAIL"
    print(
        f"{status} missing_file_via_validate_run "
        f"expected=['missing_file'] got={sorted(actual_rules)}"
    )
    if ok:
        passes += 1
    else:
        failures += 1

    canonical_covered = {
        name for name, expected, *_ in _self_test_cases() for name in expected
    }
    canonical_covered.add("missing_file")
    missing_canonical = set(CANONICAL_RULES) - canonical_covered
    if missing_canonical:
        print(f"FAIL canonical_rule_coverage missing={sorted(missing_canonical)}")
        failures += 1
    else:
        print(f"PASS canonical_rule_coverage all 12 canonical rules covered")
        passes += 1

    return passes, failures


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "run_dir",
        nargs="?",
        type=Path,
        help="Directory containing run_manifest.json, events.jsonl, summary.json, and notes.md",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Emit a single structured JSON line instead of the human-readable summary",
    )
    parser.add_argument(
        "--self-test",
        action="store_true",
        help="Run built-in fixture suite covering each canonical rule and exit",
    )
    args = parser.parse_args()

    if args.self_test:
        passes, failures = _run_self_test()
        print(f"\nself-test: passes={passes} failures={failures}")
        return 0 if failures == 0 else 1

    if args.run_dir is None:
        parser.error("run_dir is required (or use --self-test)")

    errors = validate_run(args.run_dir)

    if args.json:
        out: dict[str, Any] = {
            "run_dir": str(args.run_dir),
            "errors": errors,
        }
        if not errors:
            try:
                manifest = load_json(args.run_dir / "run_manifest.json")
                summary = load_json(args.run_dir / "summary.json")
                events = load_events(args.run_dir / "events.jsonl")
                out["run_id"] = manifest.get("run_id")
                out["events"] = len(events)
                out["tests"] = len(summary.get("tests", []))
            except (OSError, TypeError, ValueError, json.JSONDecodeError):
                pass
        print(json.dumps(out))
        return 1 if errors else 0

    if errors:
        print(f"run_dir {args.run_dir}")
        print(f"errors {len(errors)}")
        for error in errors:
            print(f"- {error['message']}")
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
