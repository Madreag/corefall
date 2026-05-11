#!/usr/bin/env bash
set -euo pipefail
# Scan for thread_rng() in sim crates (AGENTS.md rule: no thread_rng in sim).
REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
SIM_CRATES="cf-sim-core cf-physics cf-material cf-ai cf-terrain cf-atmos"
echo "=== Scanning for thread_rng() in sim crates ==="
FAIL=0
for crate in $SIM_CRATES; do
    CRATE_DIR="$REPO_ROOT/game/crates/$crate"
    if [[ -d "$CRATE_DIR" ]]; then
        HITS=$(rg -n 'thread_rng' "$CRATE_DIR/src/" 2>/dev/null || true)
        if [[ -n "$HITS" ]]; then
            echo "FAIL: $crate has thread_rng():"
            echo "$HITS"
            FAIL=1
        fi
    fi
done
if [[ "$FAIL" -eq 0 ]]; then
    echo "PASS: No thread_rng() in sim crates."
    exit 0
else
    exit 1
fi
