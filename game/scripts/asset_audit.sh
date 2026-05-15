#!/usr/bin/env bash
# M9A: nightly asset-pipeline audit. Verifies every M9A-baked asset matches
# its ledger row and that the placeholder tree on disk is in sync with the
# manifests.
#
# Usage:
#   game/scripts/asset_audit.sh            # full strict verify (CI form)
#   game/scripts/asset_audit.sh --json     # emit JSON report on stdout
#   game/scripts/asset_audit.sh --counts   # quick file-count summary only

set -euo pipefail

# Resolve repo root from this script's location regardless of cwd.
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
REPO_ROOT="$( cd "$SCRIPT_DIR/.." >/dev/null 2>&1 && pwd )"  # game/
REPO_ROOT="$( cd "$REPO_ROOT/.." >/dev/null 2>&1 && pwd )"   # /Users/.../corefall

JSON_MODE=0
COUNTS_ONLY=0
for arg in "$@"; do
  case "$arg" in
    --json) JSON_MODE=1 ;;
    --counts) COUNTS_ONLY=1 ;;
    -h|--help) sed -n '1,12p' "$0"; exit 0 ;;
    *) echo "[asset_audit] unknown arg: $arg" >&2; exit 2 ;;
  esac
done

PLACEHOLDER_ROOT="$REPO_ROOT/content/assets/placeholders"
LEDGER="$REPO_ROOT/content/asset_ledger/ledger.jsonl"

if [[ ! -d "$PLACEHOLDER_ROOT" ]]; then
  echo "[asset_audit] FAIL: placeholder tree missing at $PLACEHOLDER_ROOT" >&2
  exit 3
fi
if [[ ! -f "$LEDGER" ]]; then
  echo "[asset_audit] FAIL: ledger missing at $LEDGER" >&2
  exit 3
fi

SVG_COUNT=$(find "$PLACEHOLDER_ROOT" -type f -name '*.svg' | wc -l | tr -d ' ')
PNG_COUNT=$(find "$PLACEHOLDER_ROOT" -type f -name '*.png' | wc -l | tr -d ' ')
LEDGER_COUNT=$(wc -l < "$LEDGER" | tr -d ' ')

if [[ "$COUNTS_ONLY" == "1" ]]; then
  echo "svg=$SVG_COUNT png=$PNG_COUNT ledger_lines=$LEDGER_COUNT"
  exit 0
fi

# Full strict verify via cf-mod (must run from game/ for the ledger path).
cd "$REPO_ROOT/game"
if [[ "$JSON_MODE" == "1" ]]; then
  cargo run --release -p cf-mod -- --strict --json ledger verify --strict-status --all
else
  cargo run --release -p cf-mod -- --strict ledger verify --strict-status --all
fi

echo "[asset_audit] counts: svg=$SVG_COUNT png=$PNG_COUNT ledger_lines=$LEDGER_COUNT"
