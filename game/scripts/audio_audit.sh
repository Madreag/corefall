#!/usr/bin/env bash
# M12A § Audio audit — print per-category SFX counts + verify ledger
# parity. Per spec § Files:
#   game/scripts/audio_audit.sh (NEW)

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
VENV_PY="${REPO_ROOT}/tools/asset_gen/.venv/bin/python"

echo "[audio_audit] === per-category roster ==="
"${VENV_PY}" - <<PY
import json, pathlib
ledger = pathlib.Path("${REPO_ROOT}/content/asset_ledger/ledger.jsonl")
counts = {}
with ledger.open() as f:
    for line in f:
        try:
            e = json.loads(line)
        except Exception:
            continue
        cat = e.get("category", "")
        if cat.startswith("Audio_"):
            counts[cat] = counts.get(cat, 0) + 1
for cat, n in sorted(counts.items()):
    print(f"  {cat:18s} {n}")
print(f"  TOTAL Audio_*: {sum(counts.values())}")
PY

echo "[audio_audit] === on-disk audio file counts ==="
for dir in sfx voice music; do
    wav_count=$(find "${REPO_ROOT}/game/content/audio/${dir}" -maxdepth 1 -name "*.wav" 2>/dev/null | wc -l | tr -d ' ')
    ogg_count=$(find "${REPO_ROOT}/game/content/audio/${dir}" -maxdepth 1 -name "*.ogg" 2>/dev/null | wc -l | tr -d ' ')
    printf "  %-8s wav=%4s ogg=%4s\n" "${dir}:" "${wav_count}" "${ogg_count}"
done

echo "[audio_audit] === pipeline status ==="
"${VENV_PY}" "${REPO_ROOT}/tools/audio_gen/generate_sfx.py" --check

echo "[audio_audit] === ledger verify --strict ==="
(cd "${REPO_ROOT}/game" && cargo run --quiet -p cf-mod -- ledger verify --strict)

echo "[audio_audit] PASS"
