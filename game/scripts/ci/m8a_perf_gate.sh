#!/usr/bin/env bash
# M8A perf-budget gate.
#
# Runs the four M8A reference benches (m9_firehose, m15_ca_burst,
# m22_pathfinder_load, mp_8player_lan), reads each JSON perf report,
# and asserts every required per-subsystem p99 microsecond key is
# within budget per docs/plan/spec/perf-budget-contract.md.
#
# Maps to VAL-M8A-003 in the mission validation contract.
# Exits 0 on green, 1 on any budget violation.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "${REPO_ROOT}/game"

OUT_DIR=$(mktemp -d -t "m8a_perf_gate_XXXX")
trap 'rm -rf "${OUT_DIR}"' EXIT

FAIL=0
BENCHES=(
    "m9-firehose:m9_firehose"
    "m15-ca-burst:m15_ca_burst"
    "m22-pathfinder-load:m22_pathfinder_load"
    "mp-8player-lan:mp_8player_lan"
)

echo "M8A perf gate: running 4 reference benches..."

for entry in "${BENCHES[@]}"; do
    subcommand="${entry%%:*}"
    bench_id="${entry##*:}"
    perf_path="${OUT_DIR}/${bench_id}.json"

    echo "  -> ${bench_id}: cf-bench ${subcommand} --write-perf-report ${perf_path}"
    if ! cargo run --quiet --release -p cf-bench -- ${subcommand} \
        --ticks 200 \
        --write-perf-report "${perf_path}"; then
        echo "  [FAIL] ${bench_id}: bench run errored"
        FAIL=1
        continue
    fi

    echo "  -> ${bench_id}: assert within budget"
    if ! cargo run --quiet --release -p cf-bench -- perf-assert \
        --input "${perf_path}"; then
        echo "  [FAIL] ${bench_id}: budget violation"
        cat "${perf_path}" || true
        FAIL=1
        continue
    fi

    echo "  [PASS] ${bench_id}"
done

if [[ ${FAIL} -ne 0 ]]; then
    echo "M8A perf gate: FAIL"
    exit 1
fi

echo "M8A perf gate: PASS (VAL-M8A-003 OK)"
exit 0
