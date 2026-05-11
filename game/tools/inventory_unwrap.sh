#!/usr/bin/env bash
set -euo pipefail
# Scan for .unwrap() on user-controllable inputs (advisory).
REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
echo "=== Scanning for .unwrap() in production code (advisory) ==="
HITS=$(rg -n '\.unwrap\(\)' "$REPO_ROOT/game/crates/" --glob '!*/tests/*' --glob '!*/test_*' --glob '!*/examples/*' --glob '!*/benches/*' 2>/dev/null || true)
if [[ -z "$HITS" ]]; then
    echo "PASS: No .unwrap() found."
    exit 0
else
    COUNT=$(echo "$HITS" | wc -l | tr -d ' ')
    echo "ADVISORY: $COUNT .unwrap() occurrences in production code."
    echo "$HITS"
    exit 0
fi
