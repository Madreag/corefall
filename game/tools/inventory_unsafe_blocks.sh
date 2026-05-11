#!/usr/bin/env bash
set -euo pipefail
# Scan for unsafe blocks (AGENTS.md rule: deny unsafe_code).
REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
echo "=== Scanning for unsafe blocks ==="
HITS=$(rg -n 'unsafe\s*\{' "$REPO_ROOT/game/crates/" 2>/dev/null || true)
if [[ -z "$HITS" ]]; then
    echo "PASS: No unsafe blocks found."
    exit 0
else
    COUNT=$(echo "$HITS" | wc -l | tr -d ' ')
    echo "FAIL: $COUNT unsafe blocks found."
    echo "$HITS"
    exit 1
fi
