#!/usr/bin/env bash
# M8B § Files / CI — rollback p99 CI gate.
#
# "Fail if p99 resim > 8 ms on reference platform."
#
# Runs the cf-net `rollback_window_p99` integration test which exercises
# the 6-frame resimulate driver under a synthetic workload + asserts
# the p99 wall-clock cost is within the ResimulateBudget::default()
# total_us = 8000 (8 ms).
#
# Exits 0 on green, 1 on budget violation.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "${REPO_ROOT}/game"

echo "M8B rollback p99 gate: running cf-net rollback_window_p99 integration test..."
if ! cargo test --quiet --release -p cf-net --test rollback_window_p99 2>&1 | tail -20; then
    echo "[FAIL] cf-net rollback_window_p99"
    echo "       6-frame resim p99 exceeded the 8 ms budget on this platform."
    echo "       Locked budget lives in cf_net::rollback::resimulate::ResimulateBudget::default()."
    echo "       If the budget bump is intentional, update the constant + this gate together."
    exit 1
fi

echo "M8B rollback p99 gate: PASS"
exit 0
