"""One-shot ledger repair: convert any Audio_* entry whose output_path is
relative (`game/content/audio/...`) to its absolute path. Re-hashes only if
needed (it shouldn't — paths point to the same file content).

Use case: I accidentally wrote relative paths in the Tier 2 audio supersede.
`cf-mod ledger verify` was running from `game/` and constructing
`game/game/content/audio/...` which doesn't exist. After this fixer, every
Audio_* row carries an absolute output_path matching existing convention.
"""

from __future__ import annotations

import json
import os
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
LEDGER = REPO_ROOT / "content" / "asset_ledger" / "ledger.jsonl"


def main() -> int:
    if not LEDGER.exists():
        print(f"missing: {LEDGER}", file=sys.stderr)
        return 1
    rows: list[dict] = []
    with LEDGER.open("r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                rows.append(json.loads(line))
            except json.JSONDecodeError:
                continue
    fixed = 0
    for r in rows:
        cat = str(r.get("category", ""))
        if not cat.startswith("Audio_"):
            continue
        p = str(r.get("output_path", ""))
        if not p:
            continue
        if Path(p).is_absolute():
            continue
        absp = (REPO_ROOT / p).resolve()
        r["output_path"] = str(absp)
        fixed += 1
    if fixed == 0:
        print("[fix] nothing to do; all Audio_* rows already absolute.")
        return 0
    rows.sort(key=lambda e: str(e.get("id", "")))
    body = "\n".join(json.dumps(e, separators=(",", ":"), sort_keys=True) for e in rows)
    if body:
        body += "\n"
    staging = LEDGER.with_suffix(LEDGER.suffix + ".tmp")
    staging.write_text(body, encoding="utf-8")
    os.replace(staging, LEDGER)
    print(f"[fix] absolutized {fixed} Audio_* output paths.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
