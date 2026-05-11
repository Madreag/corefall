#!/usr/bin/env bash
set -euo pipefail
# Scan for println! in production code (AGENTS.md rule: no println in production).
REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
echo "=== Scanning for println! in production code ==="
HITS=$(rg -n 'println!' "$REPO_ROOT/game/crates/" --glob '!*/tests/*' --glob '!*/test_*' --glob '!*/examples/*' --glob '!*/benches/*' 2>/dev/null || true)
if [[ -z "$HITS" ]]; then
    echo "PASS: No println! found in production code."
    exit 0
else
    echo "$HITS"
    COUNT=$(echo "$HITS" | wc -l | tr -d ' ')
    echo "FAIL: $COUNT println! occurrences in production code."
    exit 1
fi
