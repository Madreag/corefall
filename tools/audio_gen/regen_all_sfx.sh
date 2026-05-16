#!/usr/bin/env bash
# M12A § Build integration — regenerate the full Tier-1 SFX roster from
# scratch.
#
# Per spec § Files:
#   tools/audio_gen/regen_all_sfx.sh (NEW)
#
# Walks the manifest, deletes any stale WAVs, calls `generate_sfx.py --all
# --force`. Used by `cf-mod audio-gen run` + nightly CI smoke.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
VENV_PY="${REPO_ROOT}/tools/asset_gen/.venv/bin/python"
GENERATE_SFX="${REPO_ROOT}/tools/audio_gen/generate_sfx.py"

if [[ ! -x "${VENV_PY}" ]]; then
    echo "regen_all_sfx: venv python not found at ${VENV_PY}" >&2
    echo "Run: python3 -m venv ${REPO_ROOT}/tools/asset_gen/.venv" >&2
    exit 1
fi

if [[ ! -f "${GENERATE_SFX}" ]]; then
    echo "regen_all_sfx: generate_sfx.py missing at ${GENERATE_SFX}" >&2
    exit 1
fi

echo "[regen_all_sfx] running generate_sfx.py --all --force"
"${VENV_PY}" "${GENERATE_SFX}" --all --force

echo "[regen_all_sfx] verifying ledger"
(cd "${REPO_ROOT}/game" && cargo run -p cf-mod -- ledger verify --strict)
echo "[regen_all_sfx] PASS"
