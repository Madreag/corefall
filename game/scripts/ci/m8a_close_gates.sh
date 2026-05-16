#!/usr/bin/env bash
# M8A closure wrapper — run all 3 CI gates + the determinism lint gate.
#
# Maps to the M8A merge / spec-move-to-done verdict. All four gates
# must exit 0 for the milestone to close.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "${REPO_ROOT}"

GATES=(
    "determinism-lint:game/scripts/ci/m8a_determinism_lint.sh"
    "perf-budget:game/scripts/ci/m8a_perf_gate.sh"
    "cross-os:game/scripts/ci/m8a_cross_os_determinism.sh"
    "m1-m6-backfill:game/scripts/ci/m8a_m1_m6_backfill_gate.sh"
)

FAIL=0

echo "M8A close gates: running 4 gates in order..."

for entry in "${GATES[@]}"; do
    gate_name="${entry%%:*}"
    gate_path="${entry##*:}"

    echo ""
    echo "==> ${gate_name} (${gate_path})"
    if ! bash "${gate_path}"; then
        echo "[FAIL] ${gate_name}"
        FAIL=1
    fi
done

echo ""
if [[ ${FAIL} -ne 0 ]]; then
    echo "M8A close gates: FAIL"
    exit 1
fi

echo "M8A close gates: ALL PASS"
echo "  VAL-M8A-003 (perf budget): OK"
echo "  VAL-M8A-004 (cross-OS): OK"
echo "  VAL-M8A-005 (M1-M6 backfill): OK"
echo "  determinism lint: OK"
exit 0
