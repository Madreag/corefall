#!/usr/bin/env bash
# M9A: full clean-checkout re-bake of every Tier-1 SVG placeholder.
#
# Usage:
#   game/scripts/regen_all_assets.sh             # full bake (parallel)
#   game/scripts/regen_all_assets.sh --serial    # serial bake (debug mode)
#   game/scripts/regen_all_assets.sh --category WeaponSprite
#
# The pipeline is deterministic; same palettes + manifests produce
# byte-identical output on any host. CI runs this script to validate
# regen-and-verify on every PR that touches `tools/asset_gen/` or
# `content/asset_ledger/`.

set -euo pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
REPO_ROOT="$( cd "$SCRIPT_DIR/../.." >/dev/null 2>&1 && pwd )"

VENV_PY="$REPO_ROOT/tools/asset_gen/.venv/bin/python"
if [[ ! -x "$VENV_PY" ]]; then
  echo "[regen_all_assets] venv missing at $VENV_PY" >&2
  echo "  run: cd $REPO_ROOT && python3 -m venv tools/asset_gen/.venv \\" >&2
  echo "    && tools/asset_gen/.venv/bin/pip install cairosvg Pillow blake3" >&2
  exit 1
fi

PARALLEL="8"
CATEGORY=""
for arg in "$@"; do
  case "$arg" in
    --serial) PARALLEL="0" ;;
    --parallel=*) PARALLEL="${arg#--parallel=}" ;;
    --category=*) CATEGORY="${arg#--category=}" ;;
    --category) shift; CATEGORY="${1:-}" ;;
    -h|--help) sed -n '1,14p' "$0"; exit 0 ;;
    *) ;;
  esac
done

cd "$REPO_ROOT"
ARGS=(--all --parallel "$PARALLEL")
if [[ -n "$CATEGORY" ]]; then
  ARGS=(--category "$CATEGORY" --parallel "$PARALLEL")
fi

echo "[regen_all_assets] invoking build_placeholders.py ${ARGS[*]}"
"$VENV_PY" tools/asset_gen/build_placeholders.py "${ARGS[@]}"

echo "[regen_all_assets] strict verifying ledger..."
cd "$REPO_ROOT/game"
cargo run --release -p cf-mod -- --strict ledger verify --strict-status --all
echo "[regen_all_assets] done."
