#!/usr/bin/env bash
set -euo pipefail

# check_status_surfaces.sh — Verify the 4 status surfaces are consistent.
# Per AGENTS.md Status-Surface Update Contract:
#   README + checklist + roadmap + CHANGELOG must agree.
#
# Usage: bash game/tools/check_status_surfaces.sh [bp<N>]
#   bp<N> is optional; defaults to checking all BPs.

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
README="$REPO_ROOT/README.md"
CHECKLIST="$REPO_ROOT/docs/plan/spec/feature-completion-checklist.md"
ROADMAP="$REPO_ROOT/docs/plan/spec/prototype-roadmap.md"
CHANGELOG="$REPO_ROOT/CHANGELOG.md"

BP_FILTER="${1:-}"
ERRORS=0

check_file_exists() {
    local f="$1"
    if [[ ! -f "$f" ]]; then
        echo "FAIL: Missing file: $f"
        ERRORS=$((ERRORS + 1))
        return 1
    fi
    return 0
}

echo "=== Status-Surface Update Contract Check ==="
echo "Repo root: $REPO_ROOT"
echo ""

check_file_exists "$README" || true
check_file_exists "$CHECKLIST" || true
check_file_exists "$ROADMAP" || true
check_file_exists "$CHANGELOG" || true

if [[ -n "$BP_FILTER" ]]; then
    BP_UPPER="$(echo "$BP_FILTER" | tr '[:lower:]' '[:upper:]')"
    echo "Filtering for: $BP_UPPER"
    echo ""

    # Check README mentions the BP
    if ! grep -qi "$BP_UPPER" "$README" 2>/dev/null; then
        echo "FAIL: README.md does not mention $BP_UPPER"
        ERRORS=$((ERRORS + 1))
    else
        echo "PASS: README.md mentions $BP_UPPER"
    fi

    # Check checklist has BP row
    if ! grep -qi "$BP_FILTER" "$CHECKLIST" 2>/dev/null; then
        echo "FAIL: feature-completion-checklist.md has no $BP_FILTER row"
        ERRORS=$((ERRORS + 1))
    else
        echo "PASS: feature-completion-checklist.md has $BP_FILTER row"
    fi

    # Check roadmap mentions BP
    if ! grep -qi "$BP_UPPER" "$ROADMAP" 2>/dev/null; then
        echo "FAIL: prototype-roadmap.md does not mention $BP_UPPER"
        ERRORS=$((ERRORS + 1))
    else
        echo "PASS: prototype-roadmap.md mentions $BP_UPPER"
    fi

    # Check CHANGELOG has BP section
    if ! grep -qi "$BP_UPPER\|$BP_FILTER" "$CHANGELOG" 2>/dev/null; then
        echo "FAIL: CHANGELOG.md has no $BP_UPPER section"
        ERRORS=$((ERRORS + 1))
    else
        echo "PASS: CHANGELOG.md mentions $BP_UPPER"
    fi
fi

# Cross-surface consistency: check that "Closed" claims in README match checklist
echo ""
echo "--- Closed-claim cross-check ---"
for bp_num in 0 1 2 3; do
    bp_label="BP${bp_num}"
    readme_closed=$(grep -ci "${bp_label}.*Closed\|${bp_label}.*✅" "$README" 2>/dev/null || echo 0)
    checklist_closed=$(grep -ci "\[x\].*\`${bp_label}\`" "$CHECKLIST" 2>/dev/null || echo 0)
    if [[ "$readme_closed" -gt 0 && "$checklist_closed" -eq 0 ]]; then
        echo "WARN: README claims $bp_label closed but checklist has no [x] $bp_label row"
        ERRORS=$((ERRORS + 1))
    elif [[ "$readme_closed" -gt 0 ]]; then
        echo "PASS: $bp_label closed in README and has checklist evidence"
    else
        echo "INFO: $bp_label not claimed closed in README"
    fi
done

echo ""
if [[ "$ERRORS" -gt 0 ]]; then
    echo "STATUS_SURFACE_CHECK: FAIL ($ERRORS issues)"
    exit 1
else
    echo "STATUS_SURFACE_CHECK: PASS"
    exit 0
fi
