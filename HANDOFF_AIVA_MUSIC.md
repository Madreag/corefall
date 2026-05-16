# Handoff — AIVA Music Bake (120 tracks) — Playwright Browser Automation

**Audience**: A second AI coding agent (let's call you "Agent B") who has been hired to handle ONLY this music-bake job. You do not have access to the Corefall repo. You do not need to. Everything you need is in this document, including all 120 music prompts. Your deliverable is a folder of 120 named `.wav` files that I will integrate downstream.

This is a self-contained workpacket. You write the Playwright automation. You drive the AIVA web UI. You hand back 120 WAV files. Done.

---

## TL;DR

1. **The product owner has a paid AIVA Pro account** (€33-49/mo billed annually, full copyright ownership on every output). AIVA Pro does NOT include public API access — that's why we're going browser-automation.
2. **Generate 120 unique music tracks** via AIVA's web UI (`https://www.aiva.ai/` then click into the Studio / Composition app). Each track is one of:
   - 12 world ambient themes × 4 variants (calm / buildup / climax / debrief) = **48 tracks**
   - 8 faction themes × 4 variants = **32 tracks**
   - 5 storyteller themes × 4 variants = **20 tracks**
   - 5 boss themes × 4 variants = **20 tracks**
3. **Output format**: 16-bit PCM 48 kHz stereo WAV, 60-240 sec per track (most ~180 sec, boss themes up to 240 sec, "Last Star" up to 300 sec). AIVA's WAV export defaults to 44.1 kHz / 16-bit stereo — that's acceptable, the downstream pipeline will resample.
4. **Filename convention**: `<track_id>_<variant>.wav` (see Section 6, exact list embedded).
5. **Hand-off folder**: drop the 120 WAVs into a single folder named `corefall_aiva_music_bake_v1/`. Zip it. Done.
6. **Pacing**: moderate — 30-60s random jitter between generations. Goal is ~2-4 hour unattended overnight run.
7. **Detection-risk fallback**: if AIVA flags the automation, halt and hand back what you have. Do NOT re-attempt with the same account. The product owner accepts partial delivery.

---

## Section 1 — Why we're doing this

The product is a 2D game called **Corefall** (Cortex-Command-style fortress builder + crawler hybrid). It needs an adaptive music system (M37A milestone) with one base track per scenario state × four intensity variants. The downstream game engine (Bevy 0.18) cross-fades variants based on a `0.0-1.0` intensity float driven by combat density, mission phase, boss phase, etc.

**Why AIVA**: the product owner subscribed to AIVA Pro on 5/15/2026 after research suggested it had the best cinematic / orchestral output for game soundtracks plus full copyright ownership on outputs. **AIVA Pro does NOT include public API access** (that's enterprise-tier only, behind a sales contact), so we're driving the web UI programmatically with Playwright to get bulk throughput out of a paid subscription. The product owner has authorized this approach regardless of AIVA TOS specifics. Don't email AIVA, don't appeal, just do the bake.

If AIVA's anti-bot detection trips: stop, log what you completed, hand back what you have. The product owner has a fallback (ElevenLabs Music API) running in parallel.

---

## Section 2 — Your environment + tooling

You'll need:

```
- Python 3.11+ (3.14 confirmed working)
- playwright (pip install playwright)
- playwright chromium (python -m playwright install chromium)
- requests (only if AIVA's web app makes direct XHR calls you can shortcut)
- A modern macOS / Linux machine. Chromium will run headed in the first session, headless in the bake.
```

Recommended layout:

```
aiva_bake/
├── bake.py                    # main Playwright orchestrator
├── capture_session.py         # one-shot: open headed browser, user logs in, save state.json
├── prompts.json               # the 120 prompts (paste from this doc, see Appendix A)
├── progress.json              # per-track status (pending / in_flight / done / failed) — resumable
├── state.json                 # storage state (cookies + localStorage) — created by capture_session.py
├── downloads/                 # AIVA's download landing zone (Playwright sets this)
└── corefall_aiva_music_bake_v1/   # final renamed WAVs you'll hand back
```

You can store everything outside any repo. The product owner does not need your code, only the final WAV folder.

---

## Section 3 — AIVA web UI workflow (mapped from their official help docs)

I scraped the AIVA help center (`https://aiva.crisp.help/`) so you don't have to.

**Login flow**:
1. `https://www.aiva.ai/` → click "Log in" (top right).
2. Manual login with the product owner's credentials. They will run `capture_session.py` themselves the first time. Your script then uses the saved `state.json`.

**Create-track flow** (per AIVA docs, "Creating a track" section):
1. From the AIVA Studio dashboard, click the green **"Create Track"** button (top).
2. AIVA offers four sub-flows:
   - **From a Style** ← USE THIS for every Corefall track.
   - From a Chord Progression
   - From an influence
   - Presets (legacy)
3. Select **"From a Style"**.
4. AIVA shows a Style picker with categories. Select a style that semantically matches the track's `musicgen_prompt` (genre + mood). Examples mapping (see Section 4 for the full picker strategy):
   - "cinematic", "modern cinematic" → world/boss themes  
   - "epic", "epic orchestra" → boss themes  
   - "ambient", "electronic ambient" → calm variants  
   - "rock", "metal" → frontier / collective combat variants  
   - "synthwave", "cyberpunk" → ronin / synth tracks  
   - "classical", "religious", "choral" → starlight / collegium  
   - "world fusion", "tribal" → husks / mimas tracks
5. After style is selected, AIVA prompts for **composition parameters**:
   - Composition workflow name (use the `id` field, e.g. `music_world_earth_calm`)
   - Composition duration (set per Appendix A, in seconds)
   - Number of compositions (set to **1** per request — we want deterministic output per prompt)
   - Optional Key Signature (use the `key` field if AIVA's style allows custom; otherwise let it default)
   - Optional Tempo (BPM) (use the `tempo_bpm` field if exposed)
6. Click **Generate** (or "Create" — wording varies per UI version).
7. AIVA queues the job. Wait for the track to appear in the user's track list / library. The "Generating…" / spinner state lasts 30-90 seconds for most tracks; up to 3 min for 240-300 sec compositions.
8. Once done, hover the track row → click the three-dot menu (or right-click) → **Download** → choose **WAV** (NOT MP3 — we want lossless).
9. AIVA downloads to whatever you configured in Playwright's `accept_downloads=True` context.

**File output**: AIVA names files like `<workflow-name>_<timestamp>.wav`. Rename to `<track_id>_<variant>.wav` immediately (see Section 6 for the exact map).

**Important UI gotchas**:
- AIVA's UI is React-based with heavily-mangled CSS class names. **Use role-based + text-based Playwright selectors** (e.g. `page.get_by_role("button", name="Create Track")`, `page.get_by_text("From a Style")`) — they're much more stable than `.css-1a2b3c` class chains.
- AIVA shows a tutorial / onboarding modal on first login. Dismiss it once in the headed capture session; the state.json will then remember the dismissal.
- AIVA Pro has 300 downloads/month. 120 tracks fits with 180 left over for retries. You should aim for ≤2 retries per track to stay within budget.
- AIVA throttles aggressive use. Stick to the moderate pacing (30-60s random jitter between submissions). If you see HTTP 429 or a "rate limit" / "queue full" modal, sleep 5 min and resume.

---

## Section 4 — Style picker strategy (one of the trickiest parts)

AIVA's Style library has 250+ predefined styles plus user-custom Styles you can create with the "Style Designer". You will NOT have time to train 30+ custom styles for this bake. Instead, **map each track to the closest pre-existing AIVA Style** using the table below. The intent is to pick a Style whose default character is close to what we need, then rely on AIVA's per-track parameter customization (key, tempo, duration) to nudge it.

**Recommended Style picks** (verify these exact names in AIVA's Style library at bake-time; substitute the nearest match if a name has changed):

| Corefall track group | Primary AIVA Style | Fallback | Notes |
|---|---|---|---|
| World: earth, mars, moon, phobos, deimos | "Modern Cinematic" | "Cinematic" | Grit + tension; default tempo ~90 BPM |
| World: mimas, europa | "Ambient Cinematic" | "Underwater / Aquatic" if available | Alien water vibe |
| World: vulcan | "Epic Cinematic" | "Action Cinematic" | Forge / lava |
| World: venus | "Modern Cinematic" | "Dark Cinematic" | Acid atmosphere |
| World: belt, orbital | "Modern Cinematic" | "Sci-Fi" | Industrial spacescape |
| World: sol_zone | "Religious / Choral Cinematic" | "Epic Cinematic" | Stellar awe |
| Faction: coalition | "Modern Military" | "Modern Cinematic" | Brass + snare |
| Faction: frontier | "Country" | "Western" | Acoustic guitar + harmonica |
| Faction: ronin | "Cyberpunk" | "Synthwave" | Cyberpunk samurai |
| Faction: synth | "Synthwave" | "Electronic" | Robotic arpeggio |
| Faction: collective | "Industrial" | "Modern Cinematic" | Scrap percussion |
| Faction: husks | "Tribal" | "World Fusion" | Alien chitter |
| Faction: collegium | "Religious / Choral" | "Classical" | Gregorian chant |
| Faction: starlight | "Religious / Choral" | "Epic Cinematic" | Solar ritual |
| Storyteller: cassandra | "Modern Cinematic" | "Cinematic" | Balanced |
| Storyteller: phoebe | "Lo-Fi" | "Jazz" | Mellow |
| Storyteller: randy | "Electronic" | "Glitch" | Chaotic |
| Storyteller: ironman | "Dark Cinematic" | "Modern Cinematic" | Grim |
| Storyteller: sandbox | "Acoustic" | "Folk" | Exploration |
| Boss: hollow_king | "Epic Cinematic" | "Action Cinematic" | Flame king |
| Boss: frozen_heart | "Dark Cinematic" | "Epic Cinematic" | Glacial |
| Boss: crimson_tide | "Epic Cinematic" | "Modern Cinematic" | Dust storm |
| Boss: eclipse_walker | "Cyberpunk" | "Synthwave" | Cyborg + gravity |
| Boss: last_star | "Religious / Choral Cinematic" | "Epic Cinematic" | End-of-campaign superboss |

**Per-variant nudge** (apply on top of the Style pick):

- **calm**: set duration short side of the track's `duration_seconds`, tempo at base BPM, no aggressive percussion.
- **buildup**: same Style, +10-15 BPM, ask for "tension percussion / rising bass" in the prompt text field if AIVA exposes one.
- **climax**: same Style, +20-30 BPM, full arrangement.
- **debrief**: same Style, -20-30 BPM, ask for "reflective / sparse / descending" in the prompt text field.

If AIVA exposes a free-text prompt field (some Style flows have a "Text-to-Harmony" or descriptor textbox), paste a **short summary** of the `musicgen_prompt` (≤200 chars). Don't paste the whole thing — AIVA's prompt parser is shallow.

---

## Section 5 — Playwright skeleton code (starter)

This is enough to bootstrap you. Adapt freely.

### `capture_session.py` (one-shot, headed, manual login)

```python
"""Run this ONCE. Headed browser. Product owner logs in. Saves state.json."""
import json, os, stat
from pathlib import Path
from playwright.sync_api import sync_playwright

STATE = Path("state.json")
AIVA_HOME = "https://www.aiva.ai/"

print("[capture] launching headed Chromium...")
with sync_playwright() as p:
    browser = p.chromium.launch(headless=False)
    ctx = browser.new_context(viewport={"width": 1440, "height": 900})
    page = ctx.new_page()
    page.goto(AIVA_HOME, wait_until="domcontentloaded", timeout=45_000)
    print("[capture] Log in to AIVA in the browser window.")
    print("[capture] Once you reach the AIVA dashboard, press Enter here.")
    input()
    state = ctx.storage_state()
    STATE.write_text(json.dumps(state, indent=2))
    try:
        os.chmod(STATE, 0o600)
    except OSError:
        pass
    print(f"[capture] saved {STATE} ({STATE.stat().st_size}B)")
    ctx.close()
    browser.close()
```

### `bake.py` (the main worker — adapt the selectors as you discover them)

```python
"""AIVA music bake — loop 120 prompts, generate one WAV each.

Selectors here are placeholders based on AIVA's help docs as of 2025-09-24.
Run with --recon to inspect the page structure before committing the loop.
Run with --first-only to bake just one track end-to-end and verify.
"""
import argparse, json, random, re, time
from datetime import datetime
from pathlib import Path
from playwright.sync_api import sync_playwright, expect, TimeoutError as PWTimeout

ROOT = Path(__file__).parent
PROMPTS = json.loads((ROOT / "prompts.json").read_text())
PROGRESS = ROOT / "progress.json"
STATE = ROOT / "state.json"
DL_DIR = ROOT / "downloads"
OUT_DIR = ROOT / "corefall_aiva_music_bake_v1"
DL_DIR.mkdir(exist_ok=True)
OUT_DIR.mkdir(exist_ok=True)

# ---- selector hints (tune in --recon mode) ----
SEL_CREATE_TRACK = "button:has-text('Create Track')"
SEL_FROM_STYLE = "text=From a Style"
SEL_STYLE_FILTER = "[placeholder='Search styles']"  # or text="Filters"
SEL_DURATION_INPUT = "input[type='number']"          # may need more specificity
SEL_GENERATE = "button:has-text('Generate')"
# completion: poll for a track row in the library with matching name; OR look for
# a "Download" affordance to appear.
SEL_TRACK_ROW = ".track-row, [data-testid='track-row']"
SEL_TRACK_MENU = "button[aria-label*='options']"
SEL_DOWNLOAD = "text=Download"
SEL_WAV = "text=WAV"

def load_progress() -> dict:
    if PROGRESS.exists():
        return json.loads(PROGRESS.read_text())
    return {"completed": [], "failed": []}

def save_progress(p: dict) -> None:
    PROGRESS.write_text(json.dumps(p, indent=2))

def safe_name(s: str) -> str:
    return re.sub(r"[^a-z0-9_]+", "_", s.lower())

def bake_one(page, entry: dict) -> Path | None:
    track_id = entry["track_id"]
    variant = entry["variant"]
    style = entry["aiva_style"]
    duration_sec = int(entry["duration_seconds"])
    prompt_summary = entry["prompt_summary"]
    name = f"{track_id}_{variant}"

    print(f"[bake] {name}  style={style!r}  dur={duration_sec}s")

    # 1. Click Create Track
    page.click(SEL_CREATE_TRACK, timeout=30_000)
    page.wait_for_load_state("networkidle", timeout=20_000)

    # 2. From a Style
    page.click(SEL_FROM_STYLE, timeout=20_000)
    page.wait_for_load_state("networkidle", timeout=20_000)

    # 3. Pick the style (search or scroll)
    try:
        page.fill(SEL_STYLE_FILTER, style)
        page.wait_for_timeout(800)
    except Exception:
        pass  # filter may not exist; user can navigate manually
    # Click the first style result whose text matches
    page.click(f"text={style}", timeout=20_000)
    page.wait_for_load_state("networkidle", timeout=10_000)

    # 4. Set duration (this is the trickiest field — wrap in try)
    try:
        dur_field = page.locator("input[type='number']").first
        dur_field.fill(str(duration_sec))
    except Exception as exc:
        print(f"[bake] duration field skipped ({exc})")

    # 5. Optional: fill prompt textbox if present
    try:
        prompt_box = page.locator("textarea").first
        prompt_box.fill(prompt_summary[:200])
    except Exception:
        pass

    # 6. Generate
    page.click(SEL_GENERATE, timeout=20_000)

    # 7. Wait for track to appear. AIVA emits a toast or adds a row.
    deadline = time.time() + 240  # 4 min hard cap
    while time.time() < deadline:
        rows = page.locator(SEL_TRACK_ROW)
        if rows.count() > 0 and rows.first.is_visible():
            # heuristic: top row is the most recent
            break
        time.sleep(2.0)
    else:
        print(f"[bake] timeout waiting for {name}")
        return None

    # 8. Open the track menu + download WAV
    top = page.locator(SEL_TRACK_ROW).first
    top.locator(SEL_TRACK_MENU).first.click()
    page.click(SEL_DOWNLOAD, timeout=10_000)
    with page.expect_download(timeout=120_000) as dl_info:
        page.click(SEL_WAV, timeout=10_000)
    download = dl_info.value
    dest = OUT_DIR / f"{name}.wav"
    download.save_as(str(dest))
    print(f"[bake] saved {dest} ({dest.stat().st_size}B)")
    return dest

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--recon", action="store_true",
                    help="Open dashboard and stop; print page structure.")
    ap.add_argument("--first-only", action="store_true",
                    help="Bake just the first pending track.")
    ap.add_argument("--start-from", type=str, default=None,
                    help="track_id_variant to skip-to (e.g. music_world_mars_buildup)")
    ap.add_argument("--min-jitter", type=float, default=30.0)
    ap.add_argument("--max-jitter", type=float, default=60.0)
    args = ap.parse_args()

    if not STATE.exists():
        raise SystemExit(f"Missing {STATE}. Run capture_session.py first.")
    state = json.loads(STATE.read_text())
    prog = load_progress()
    completed = set(prog["completed"])

    with sync_playwright() as p:
        browser = p.chromium.launch(headless=False if args.recon else True)
        ctx = browser.new_context(
            storage_state=state,
            accept_downloads=True,
            viewport={"width": 1440, "height": 900},
        )
        page = ctx.new_page()
        page.goto("https://www.aiva.ai/", wait_until="domcontentloaded")
        page.wait_for_load_state("networkidle", timeout=30_000)

        if args.recon:
            print("=== RECON ===")
            print("title:", page.title())
            print("url:", page.url)
            print("buttons visible:")
            for b in page.get_by_role("button").all():
                try:
                    print("  -", b.inner_text(timeout=1000))
                except Exception:
                    pass
            input("Press Enter to close recon...")
            return

        skip_until = args.start_from
        for entry in PROMPTS:
            name = f"{entry['track_id']}_{entry['variant']}"
            if skip_until and name != skip_until:
                continue
            skip_until = None
            if name in completed:
                continue
            try:
                out = bake_one(page, entry)
                if out:
                    prog["completed"].append(name)
                else:
                    prog["failed"].append(name)
                save_progress(prog)
            except Exception as exc:
                print(f"[bake] FAILED {name}: {exc}")
                prog["failed"].append(name)
                save_progress(prog)
                # screenshot for diagnostics
                shot = ROOT / f"fail_{name}.png"
                try:
                    page.screenshot(path=str(shot), full_page=True)
                except Exception:
                    pass
            if args.first_only:
                break
            # jitter
            j = random.uniform(args.min_jitter, args.max_jitter)
            print(f"[bake] sleeping {j:.1f}s...")
            time.sleep(j)
        ctx.close()
        browser.close()

if __name__ == "__main__":
    main()
```

### Operating sequence

```bash
# 1. Capture session (interactive, headed). User logs in.
python capture_session.py

# 2. Sanity check the UI with --recon (prints all visible buttons).
python bake.py --recon

# 3. Bake the first track end-to-end. Verify the WAV.
python bake.py --first-only

# 4. Full bake (resumable; progress.json checkpointed per track).
python bake.py

# 5. If a crash: bake.py reads progress.json and skips completed tracks.
python bake.py --start-from music_faction_ronin_climax
```

---

## Section 6 — Filename convention (EXACT)

Each output must be named **`<track_id>_<variant>.wav`**, lowercase, no spaces. Examples:

```
music_world_earth_calm.wav
music_world_earth_buildup.wav
music_world_earth_climax.wav
music_world_earth_debrief.wav
music_world_mars_calm.wav
...
music_faction_coalition_calm.wav
music_faction_coalition_buildup.wav
...
music_storyteller_cassandra_climax.wav
...
music_boss_last_star_debrief.wav
```

The full 120 names are deterministic from `<track_id>` × `{calm, buildup, climax, debrief}`. See Appendix A for all 30 `track_id`s.

---

## Section 7 — Final validation before handoff

Before zipping `corefall_aiva_music_bake_v1/` and handing it back, validate:

1. **File count**: exactly 120 `.wav` files.
2. **Naming**: all match `^music_(world|faction|storyteller|boss)_[a-z_]+_(calm|buildup|climax|debrief)\.wav$`.
3. **Format**: every file is a real WAV. A quick check:
   ```bash
   for f in corefall_aiva_music_bake_v1/*.wav; do
     file "$f" | grep -q "WAVE audio" || echo "BAD: $f"
   done
   ```
4. **Min duration**: every file ≥ 30 seconds (filter obvious truncation):
   ```bash
   for f in corefall_aiva_music_bake_v1/*.wav; do
     dur=$(ffprobe -v error -show_entries format=duration -of csv=p=0 "$f" 2>/dev/null)
     awk "BEGIN { exit !($dur < 30) }" && echo "SHORT: $f ($dur s)"
   done
   ```
   (If `ffprobe` isn't installed: `brew install ffmpeg` or `apt install ffmpeg`.)
5. **Size sanity**: every file ≥ 100 KB (typical 60-180s WAV is 10-30 MB).

If any track is missing, mark it in `progress.json` as `failed` and retry. If after 2 retries it still fails, ship without it — the product owner has fallback coverage.

---

## Section 8 — Hand-off

Once the bake is complete:

1. ZIP the folder:
   ```bash
   cd <parent dir>
   zip -r corefall_aiva_music_bake_v1.zip corefall_aiva_music_bake_v1/
   ```
2. Share the ZIP with the product owner via their preferred channel (will be communicated to you separately — likely Dropbox / Google Drive / direct file transfer).
3. Include a short summary text file (`bake_report.txt`) in the ZIP:
   ```
   AIVA music bake — corefall_aiva_music_bake_v1
   Completed: <n>/120 tracks
   Failed:    <list of failed track_ids, if any>
   Total time: <hh:mm>
   AIVA account: <product owner's email, REDACT before sharing>
   Notes: <any UI quirks you noticed, e.g. "AIVA UI updated 2026-05-15, had to adjust Style picker selector">
   ```

The product owner will then run a downstream ingest pipeline that hashes each WAV, applies post-processing (trim / fade / loop-align / normalize to -8 dBFS), and updates an asset ledger. You don't need to do any of that.

---

## Section 9 — Constraints (don't violate these)

- **No automation of AIVA's billing / subscription pages.** Stick to the composition flow.
- **No public posting of generated music** without product owner approval.
- **No sharing of `state.json`** with anyone — that file contains an active session cookie. Treat it like a password. The product owner runs the headed capture and provides `state.json` to your runtime; you do NOT request or commit it.
- **No bypass of AIVA's "Pro" gating** — we want WAV format which is Pro-tier only; that's why the product owner paid. If a track tries to download as MP3 because the UI fell back, that's a bug — flag it.
- **No second AIVA account / no proxy rotation.** One account, one machine, moderate pacing. If detected, halt + report.

---

## Section 10 — Pacing + detection considerations

- AIVA's web app is a normal React SPA. They likely use Cloudflare bot management on the API edge. Default Playwright + chromium-bidi will pass most checks; we don't expect to need stealth plugins.
- Random jitter (30-60s) between submissions mimics a designer iterating manually. Don't go below 15s.
- If AIVA's dashboard suddenly shows a CAPTCHA: halt the bake. Save progress.json. Hand back what you have. The product owner will decide whether to manually complete via the UI or pivot to ElevenLabs Music (the parallel fallback already in flight).
- AIVA's queue can get slow at peak hours (9 AM - 5 PM EST). Schedule the bake overnight Phoenix time (= midnight - 8 AM in user time zone MST).
- The product owner is OK with the bake taking 4-8 hours. They are NOT OK with it taking days. If your per-track latency exceeds 5 min × 120 = 10 hours, escalate.

---

## Appendix A — The 120 tracks (FULL PROMPTS, embed in `prompts.json`)

Each row below is one entry. Convert to JSON array (see schema after the table). The `prompt_summary` field is what to paste into AIVA's prompt textbox if one is available (truncated to ≤200 chars). The `musicgen_prompt` field is the full prompt (use it to inform Style selection + parameter tuning).

**Schema for `prompts.json` (Python tuple / JSON array of dicts)**:

```json
[
  {
    "track_id":       "music_world_earth",
    "variant":        "calm",
    "duration_seconds": 240,
    "tempo_bpm":      82,
    "key":            "D minor",
    "aiva_style":     "Modern Cinematic",
    "prompt_summary": "Earth post-collapse wasteland, slow synth pad in D minor + distant wind + sparse piano motif, no drums, contemplative",
    "musicgen_prompt": "Earth post-collapse wasteland ambient, ruined-city melancholy, slow synth pad in D minor + distant wind + sparse piano motif + faint radio interference, 82 BPM, no drums, cinematic sci-fi grimdark, contemplative",
    "seed":           110001
  },
  ...
]
```

### A.1 — World ambient (12 tracks × 4 variants = 48)

Per track: `<track_id>` is the row anchor; `<duration_s>` / `<tempo_bpm>` / `<key>` are constant across the 4 variants; `<aiva_style>` is the recommended pick from Section 4.

For each track, embed the 4 variants from the JSON below. The full data follows.

**Track `music_world_earth`** (Earth Wastelands) — duration 240s, tempo 82 BPM, key D minor, AIVA style "Modern Cinematic"
- **calm**:   prompt `"Earth post-collapse wasteland ambient, ruined-city melancholy, slow synth pad in D minor + distant wind + sparse piano motif + faint radio interference, 82 BPM, no drums, cinematic sci-fi grimdark, contemplative"`, seed 110001
- **buildup**: `"Earth wasteland tension building, same synth pad in D minor + low ticking percussion + bass swell + distant industrial drone rising, 82 BPM, threat approaching"`, seed 110002
- **climax**: `"Earth wasteland combat, aggressive industrial electronic in D minor + distorted bass + driving drums + ominous brass stabs + faction radio chatter, 110 BPM, urgent oppressive"`, seed 110003
- **debrief**: `"Earth wasteland post-combat, sparse piano motif in D minor + sustained pad + light rain + breath of relief, 70 BPM, reflective melancholic"`, seed 110004

**Track `music_world_mars`** (Mars Dust Plains) — 240s, 80 BPM, C minor, "Modern Cinematic"
- **calm**: `"Mars rust-desert ambient, vast empty alien wasteland, low-fi synth pad in C minor + lonely synth lead + sparse percussion + dust storm wind whisper, 80 BPM, 4/4, cinematic sci-fi grimdark"`, seed 120001
- **buildup**: `"Mars dust plains tension, same synth pad in C minor + rising bassline + tension percussion + heat shimmer + distant siren, 95 BPM, dust storm approaching"`, seed 120002
- **climax**: `"Mars combat, full arrangement in C minor + heavy drums + synth lead + harmonic tension peak + sandstorm howl, 115 BPM, frenetic survival"`, seed 120003
- **debrief**: `"Mars dust plains debrief, reflective piano outro in C minor + soft synth pad + receding wind, 60 BPM, hollow desolation"`, seed 120004

**Track `music_world_moon`** (Moon Vacuum Surface) — 240s, 72 BPM, F# minor, "Modern Cinematic"
- **calm**: `"Moon vacuum lunar ambient, deep silence with helmet breathing + isolated synth pad in F# minor + faint suit-pump pulse + radio static fragments, 72 BPM, no drums, claustrophobic vacuum"`, seed 130001
- **buildup**: `"Moon surface tension, oxygen depletion alarm rhythm + bass drone in F# minor + pulsing pad + distant explosion echoes, 85 BPM, suit damage incoming"`, seed 130002
- **climax**: `"Moon vacuum combat, intense electronic in F# minor + sharp synth stabs + tight drums + alarm sweeps + EVA gunfire echo, 105 BPM, frantic vacuum survival"`, seed 130003
- **debrief**: `"Moon surface debrief, slow synth pad in F# minor + suit breathing slowing + distant earth-shine + lonely chime, 55 BPM, isolated victory"`, seed 130004

**Track `music_world_phobos`** (Phobos Microgravity Asteroid) — 240s, 70 BPM, G minor, "Modern Cinematic"
- **calm**: `"Phobos microgravity asteroid ambient, eerie silence with floating debris + creaking structural metal + faint synth drone in G minor + sparse chime, 70 BPM, no drums, weightless dread"`, seed 140001
- **buildup**: `"Phobos tension, debris cascade + bass pulse in G minor + tension percussion + structural groan, 88 BPM, gravity warning"`, seed 140002
- **climax**: `"Phobos combat, frantic electronic in G minor + tight drums + tumbling debris sample + sharp synth + alarms, 110 BPM, weightless combat chaos"`, seed 140003
- **debrief**: `"Phobos debrief, drifting synth pad in G minor + slow chime + structural settle + distant Mars-rise, 50 BPM, weightless peace"`, seed 140004

**Track `music_world_deimos`** (Deimos Mining Colony) — 240s, 78 BPM, B minor, "Modern Cinematic"
- **calm**: `"Deimos mining colony ambient, distant industrial machinery hum + metallic clanks + synth pad in B minor + radio chatter fragments + air recycler drone, 78 BPM, working-class space station"`, seed 150001
- **buildup**: `"Deimos tension, mining drills accelerating + bass throb in B minor + percussion pulse + alarm warm-up, 92 BPM, ore vein collapse imminent"`, seed 150002
- **climax**: `"Deimos combat, industrial electronic in B minor + heavy machinery samples + driving drums + distorted bass + worker shouts, 115 BPM, colony riot"`, seed 150003
- **debrief**: `"Deimos debrief, slow synth pad in B minor + drill winding down + tool clatter receding + worker whistle, 65 BPM, end-of-shift weariness"`, seed 150004

**Track `music_world_mimas`** (Mimas Methane Sea) — 240s, 68 BPM, Eb minor, "Ambient Cinematic"
- **calm**: `"Mimas methane sea ambient, gentle liquid lapping + bubble synth pad in Eb minor + ethereal flute + distant rumble + atmospheric whistle, 68 BPM, no drums, alien aquatic dread"`, seed 160001
- **buildup**: `"Mimas methane sea tension, deep rumble crescendo + bass pulse in Eb minor + tribal drums + sea-creature call, 85 BPM, something rising"`, seed 160002
- **climax**: `"Mimas combat, flowing exotic in Eb minor + tribal drums + watery sound design + chant + bass synth + creature roar, 108 BPM, alien hostile depths"`, seed 160003
- **debrief**: `"Mimas debrief, drifting synth pad in Eb minor + soft bubbles + flute solo + receding waves, 55 BPM, alien beauty"`, seed 160004

**Track `music_world_europa`** (Europa Ice Cavern Ocean) — 240s, 72 BPM, A minor, "Ambient Cinematic"
- **calm**: `"Europa subsurface ice cavern ambient, dripping water + ice crystal crackle + deep underwater hum + bioluminescent creature calls + synth pad in A minor + ethereal chimes, 72 BPM, no drums, deep-ocean alien"`, seed 170001
- **buildup**: `"Europa tension, sonar ping rising + bass throb in A minor + creature-vocal swell + ice crack percussion, 88 BPM, leviathan approaches"`, seed 170002
- **climax**: `"Europa combat, aquatic orchestral in A minor + heavy drums + brass stabs + creature roar + bubble percussion, 110 BPM, deep-sea hunt"`, seed 170003
- **debrief**: `"Europa debrief, ethereal synth pad in A minor + distant whale-song + ice settle + cathedral choir, 55 BPM, alien-ocean awe"`, seed 170004

**Track `music_world_vulcan`** (Vulcan Magma Forge) — 240s, 86 BPM, E minor, "Epic Cinematic"
- **calm**: `"Vulcan magma chamber ambient, bubbling lava + heat shimmer + rock-fall percussion + synth pad in E minor + steam vent hiss + low industrial drum, 86 BPM, infernal forge"`, seed 180001
- **buildup**: `"Vulcan tension, lava surge crescendo + bass throb in E minor + driving percussion + alarm tone, 100 BPM, eruption imminent"`, seed 180002
- **climax**: `"Vulcan combat, blistering industrial in E minor + thunder drums + brass + screaming lead synth + erupting magma roar, 130 BPM, hellfire combat"`, seed 180003
- **debrief**: `"Vulcan debrief, smoldering synth pad in E minor + cooling lava crackle + sparse hammer + reverent organ, 65 BPM, forge-tested survival"`, seed 180004

**Track `music_world_venus`** (Venus Acid Cloud Sea) — 240s, 76 BPM, C# minor, "Modern Cinematic" / Dark
- **calm**: `"Venus high-pressure atmospheric ambient, howling wind + acid rain patter + distant lightning rumble + pressure groan + synth pad in C# minor + ominous brass, 76 BPM, hostile-atmosphere dread"`, seed 190001
- **buildup**: `"Venus tension, pressure squeal rising + bass throb in C# minor + tension percussion + acid sizzle, 92 BPM, hull breach incoming"`, seed 190002
- **climax**: `"Venus combat, atmospheric electronic in C# minor + heavy drums + acid-rain sample + brass + alarms + crackling lightning, 115 BPM, corrosive battlefield"`, seed 190003
- **debrief**: `"Venus debrief, slow synth pad in C# minor + receding storm + dripping acid + lonely chime, 60 BPM, corrosive aftermath"`, seed 190004

**Track `music_world_belt`** (Belt Asteroid Mining) — 240s, 92 BPM, G# minor, "Sci-Fi" / "Modern Cinematic"
- **calm**: `"Asteroid belt mining colony ambient, distant rock impacts + ore conveyor + radio chatter + spacesuit breathing + synth pad in G# minor + clanking metal percussion, 92 BPM, industrial frontier"`, seed 200001
- **buildup**: `"Belt tension, rock-impact crescendo + bass throb in G# minor + pirate radio bleed + drill spin, 105 BPM, pirate raid forming"`, seed 200002
- **climax**: `"Belt combat, anarchic industrial in G# minor + distorted bass + thrashing drums + ore-crusher samples + worker chants, 130 BPM, brutal direct mining war"`, seed 200003
- **debrief**: `"Belt debrief, slow synth pad in G# minor + drill winding down + distant rock clink + radio sign-off, 70 BPM, hard-won score"`, seed 200004

**Track `music_world_orbital`** (Orbital Station Interior) — 240s, 90 BPM, F major, "Sci-Fi" / "Modern Cinematic"
- **calm**: `"Orbital station interior ambient, gentle air recycler hum + computer beeps + distant footsteps + synth pad in F major + faint elevator-jazz harmonics, 90 BPM, civilian-station comfort"`, seed 210001
- **buildup**: `"Orbital station tension, klaxon priming + bass throb in F minor + computer alarms + bulkhead clank, 100 BPM, hull breach warning"`, seed 210002
- **climax**: `"Orbital station combat, frantic electronic in F minor + driving drums + alarm pulse + bulkhead slam + station-PA shouts, 125 BPM, zero-g station boarding"`, seed 210003
- **debrief**: `"Orbital station debrief, mellow synth pad in F major + air recycler slow + computer chimes + medbay piano, 75 BPM, station recovery"`, seed 210004

**Track `music_world_sol_zone`** (Sol Zone Stellar Edge) — 240s, 80 BPM, Bb major, "Religious / Choral Cinematic"
- **calm**: `"Sol-zone habitat near-star ambient, intense solar wind + crystalline resonance + radiation hum + synth pad in Bb major + cathedral organ + ethereal choir, 80 BPM, sacred stellar awe"`, seed 220001
- **buildup**: `"Sol-zone tension, solar flare crescendo + bass swell in Bb major + radiation alarm + brass build, 95 BPM, flare incoming"`, seed 220002
- **climax**: `"Sol-zone combat, triumphant orchestral in Bb major + full choir + heavy brass + thundering drums + stellar-wind howl, 120 BPM, climactic stellar wrath"`, seed 220003
- **debrief**: `"Sol-zone debrief, sustained organ in Bb major + soft choir + receding solar wind + distant chime, 65 BPM, sacred relief"`, seed 220004

### A.2 — Faction themes (8 tracks × 4 variants = 32)

**Track `music_faction_coalition`** — 180s, 110 BPM, C major, "Modern Military"
- **calm**: `"Coalition faction theme calm, militaristic orchestral with strong brass + snare + heroic melody in C major + civilian-radio fanfare + disciplined humanity, 110 BPM, ordered hope"`, seed 310001
- **buildup**: `"Coalition buildup, brass swell + tactical snare in C major + bass pulse + radio command chatter, 118 BPM, mobilization"`, seed 310002
- **climax**: `"Coalition combat, modern military electronic in E minor + electric guitar + heavy drums + brass + synth + sergeant shouts, 125 BPM, aggressive precision"`, seed 310003
- **debrief**: `"Coalition debrief, solemn brass in C major + slow snare roll + bugle taps + sustained pad, 70 BPM, honored fallen"`, seed 310004

**Track `music_faction_frontier`** — 180s, 95 BPM, G major, "Country" / "Western"
- **calm**: `"Frontier faction theme calm, frontier folk-electronic with acoustic guitar + harmonica + light synth + steady drum + bottle-percussion in G major, 95 BPM, hardy independent settlers"`, seed 320001
- **buildup**: `"Frontier buildup, harmonica building + bass strum in G major + tom drums + outlaw whistle, 105 BPM, posse forming"`, seed 320002
- **climax**: `"Frontier combat, western-electronic battle in E minor + electric guitar + heavy drums + bass synth + harmonica + holler shouts, 125 BPM, defiant outlaws"`, seed 320003
- **debrief**: `"Frontier debrief, slow acoustic guitar in G major + harmonica solo + saloon piano + crickets, 65 BPM, dusty homecoming"`, seed 320004

**Track `music_faction_ronin`** — 180s, 88 BPM, D minor, "Cyberpunk"
- **calm**: `"Ronin faction theme calm, lone-wolf neo-noir with koto + electric piano + minimal synth pad in D minor + cyberpunk rain + lonely sax, 88 BPM, wandering blade-for-hire melancholy"`, seed 330001
- **buildup**: `"Ronin buildup, koto pluck rising + bass pulse in D minor + taiko build + neon-flicker percussion, 100 BPM, duel imminent"`, seed 330002
- **climax**: `"Ronin combat, cyberpunk samurai in D minor + driving taiko drums + electric guitar + koto + screaming synth lead, 130 BPM, blade-and-bullet ballet"`, seed 330003
- **debrief**: `"Ronin debrief, solo koto in D minor + sustained pad + rain on neon + soft cello, 60 BPM, blood-stained reflection"`, seed 330004

**Track `music_faction_synth`** — 180s, 105 BPM, A minor, "Synthwave"
- **calm**: `"Synth faction theme calm, robotic drone-collective ambient, synthetic monotone choir + arpeggiated sequencer + cold synth pad in A minor + circuit-glitch percussion, 105 BPM, machine consensus"`, seed 340001
- **buildup**: `"Synth buildup, sequencer accelerating + bass pulse in A minor + glitch percussion + hive-mind ping, 115 BPM, swarm coalescing"`, seed 340002
- **climax**: `"Synth combat, frantic electronic in A minor + arpeggiated sequencer at full speed + heavy drums + distorted bass + robotic vocals, 140 BPM, mechanized overwhelm"`, seed 340003
- **debrief**: `"Synth debrief, slow arpeggio in A minor + synthetic pad + cooling-fan whir + bell tone, 70 BPM, machine satisfaction"`, seed 340004

**Track `music_faction_collective`** — 180s, 95 BPM, G minor, "Industrial"
- **calm**: `"Collective faction theme calm, industrial proletarian electronic with metal-scrap percussion + bass + radio static + worker chants + synth pad in G minor, 95 BPM, gritty solidarity"`, seed 350001
- **buildup**: `"Collective buildup, scrap-percussion accelerating + bass throb in G minor + worker drum + factory whistle, 108 BPM, strike forming"`, seed 350002
- **climax**: `"Collective combat, anarchic industrial in D minor + distorted bass + thrashing drums + machinery samples + crowd chants, 130 BPM, brutal direct revolt"`, seed 350003
- **debrief**: `"Collective debrief, slow factory hum in G minor + worker-choir hum + accordion + receding drum, 70 BPM, weary victory"`, seed 350004

**Track `music_faction_husks`** — 180s, 80 BPM, B minor, "Tribal" / "World Fusion"
- **calm**: `"Husks faction theme calm, alien-insectoid ambient with chittering + dissonant synth + drone + skittering percussion in B minor + distorted whisper, 80 BPM, unsettling hive presence"`, seed 360001
- **buildup**: `"Husks buildup, skitter crescendo + bass throb in B minor + insect chitter swarming + dissonant string, 100 BPM, hive convergence"`, seed 360002
- **climax**: `"Husks combat, frantic alien insectoid in F minor + skittering percussion + dissonant strings + screaming horns + chaos chant, 145 BPM, overwhelming hive frenzy"`, seed 360003
- **debrief**: `"Husks debrief, eerie drone in B minor + receding chitter + distant queen-call + heartbeat pulse, 60 BPM, alien quiet"`, seed 360004

**Track `music_faction_collegium`** — 180s, 70 BPM, F major, "Religious / Choral"
- **calm**: `"Collegium faction theme calm, monastic-scholarly ambient with Gregorian chant + drone organ + bell + soft strings + synth pad in F major + scriptorium quill, 70 BPM, contemplative knowledge"`, seed 370001
- **buildup**: `"Collegium buildup, chant rising + bass swell in F major + organ build + tome-slam percussion, 85 BPM, ritual preparation"`, seed 370002
- **climax**: `"Collegium combat, sacred orchestral battle in D minor + male choir + brass + heavy strings + church bells + righteous chant, 110 BPM, archive defense"`, seed 370003
- **debrief**: `"Collegium debrief, sustained organ in F major + soft choir + library-quiet + soft chime, 60 BPM, sacred preservation"`, seed 370004

**Track `music_faction_starlight`** — 180s, 65 BPM, A major, "Religious / Choral"
- **calm**: `"Starlight faction theme calm, solar-ritual ambient with bell tones + drone synth + cathedral organ + light percussion + synth pad in A major + ethereal choir, 65 BPM, religious science"`, seed 380001
- **buildup**: `"Starlight buildup, choir building + bass swell in A major + ritual-bell crescendo + sunburst harp, 85 BPM, illumination rite"`, seed 380002
- **climax**: `"Starlight combat, ritualistic orchestral in D minor + full choir + brass + tribal drums + organ + ecstatic chant, 115 BPM, fanatical fervor"`, seed 380003
- **debrief**: `"Starlight debrief, sustained organ in A major + soft choir + receding bells + harp glissando, 55 BPM, illuminated peace"`, seed 380004

### A.3 — Storyteller themes (5 tracks × 4 variants = 20)

**Track `music_storyteller_cassandra`** (Cassandra Classic) — 180s, 95 BPM, C minor, "Modern Cinematic"
- **calm**: `"Cassandra Classic narrative theme calm, balanced cinematic synth pad in C minor + soft strings + measured piano + steady heartbeat percussion, 95 BPM, fair storyteller pacing"`, seed 410001
- **buildup**: `"Cassandra Classic buildup, strings swell + bass pulse in C minor + tension percussion + ascending piano motif, 105 BPM, the story turns"`, seed 410002
- **climax**: `"Cassandra Classic event climax, orchestral in C minor + heavy strings + brass + percussion + leitmotif return, 120 BPM, dramatic incident"`, seed 410003
- **debrief**: `"Cassandra Classic debrief, slow piano outro in C minor + soft strings + lone violin, 65 BPM, balanced reflection"`, seed 410004

**Track `music_storyteller_phoebe`** (Phoebe Chillax) — 180s, 80 BPM, Bb major, "Lo-Fi" / "Jazz"
- **calm**: `"Phoebe Chillax narrative theme calm, mellow lofi synth pad in Bb major + soft piano + jazz brush percussion + sparse warm bass, 80 BPM, player-friendly mellow"`, seed 420001
- **buildup**: `"Phoebe Chillax buildup, warm strings swelling + soft bass in Bb major + light percussion + gentle vibraphone, 90 BPM, light tension"`, seed 420002
- **climax**: `"Phoebe Chillax event climax, light orchestral in Bb major + warm brass + brush drums + piano melody + uplifting choir, 105 BPM, generous challenge"`, seed 420003
- **debrief**: `"Phoebe Chillax debrief, mellow piano outro in Bb major + soft strings + smiling vibraphone, 60 BPM, gentle wind-down"`, seed 420004

**Track `music_storyteller_randy`** (Randy Random) — 180s, 110 BPM, F# minor, "Electronic" / "Glitch"
- **calm**: `"Randy Random narrative theme calm, chaotic-unpredictable synth pad in F# minor + glitch percussion + erratic piano stabs + random pitch sweeps, 110 BPM, anything goes"`, seed 430001
- **buildup**: `"Randy Random buildup, escalating chaos in F# minor + accelerating drums + dissonant strings + alarm sweeps, 125 BPM, unpredictable cascade"`, seed 430002
- **climax**: `"Randy Random event climax, frantic electronic in F# minor + double-time drums + dissonant brass + screaming synth + chaos percussion, 145 BPM, total mayhem"`, seed 430003
- **debrief**: `"Randy Random debrief, surreal pad in F# minor + erratic glitch fading + soft chime + breath of relief, 75 BPM, chaos receding"`, seed 430004

**Track `music_storyteller_ironman`** (Ironman) — 180s, 88 BPM, G minor, "Dark Cinematic"
- **calm**: `"Ironman narrative theme calm, grim permadeath ambient, low synth drone in G minor + sparse cello + military snare + ticking clock + heartbeat pulse, 88 BPM, no second chances"`, seed 440001
- **buildup**: `"Ironman buildup, low strings rising + bass throb in G minor + snare roll + funeral-bell tease, 100 BPM, irreversible threat"`, seed 440002
- **climax**: `"Ironman event climax, dark orchestral in G minor + heavy strings + funeral brass + drum-roll + ominous choir, 115 BPM, life-or-death stakes"`, seed 440003
- **debrief**: `"Ironman debrief, solo cello in G minor + sustained drone + funeral bell + sparse piano, 55 BPM, permadeath aftermath"`, seed 440004

**Track `music_storyteller_sandbox`** (Sandbox) — 180s, 75 BPM, D major, "Acoustic" / "Folk"
- **calm**: `"Sandbox narrative theme calm, pure-exploration ambient with airy synth pad in D major + acoustic guitar + bird-call samples + minimal percussion + harp glissando, 75 BPM, no-pressure curiosity"`, seed 450001
- **buildup**: `"Sandbox buildup, soft strings swelling + warm bass in D major + light percussion + discovery harp, 85 BPM, mild surprise"`, seed 450002
- **climax**: `"Sandbox climax, exploration orchestral in D major + soaring strings + brass crescendo + uplifting drum + choir of wonder, 105 BPM, major discovery"`, seed 450003
- **debrief**: `"Sandbox debrief, acoustic guitar outro in D major + soft strings + harp + gentle wind, 60 BPM, contented exploration"`, seed 450004

### A.4 — Boss themes (5 tracks × 4 variants = 20)

**Track `music_boss_hollow_king`** (The Hollow King, world earth, 3 phases) — 240s, 100 BPM, D minor, "Epic Cinematic"
- **calm**: `"Hollow King boss arena entry, ominous building orchestral with menacing brass + tribal drums + low choir + bass + slow heartbeat in D minor, 100 BPM, confrontation looming"`, seed 510001
- **buildup**: `"Hollow King phase 1 buildup, strings crescendo + brass build + tribal drums escalating in D minor + lava-crackle percussion + king's-voice chant, 115 BPM, flame king awakens"`, seed 510002
- **climax**: `"Hollow King phase 2/3 combat, triumphant epic in D minor + full orchestra + battle choir + powerful brass + thundering drums + leitmotif + pyroclastic roar, 130 BPM, climactic flame king war"`, seed 510003
- **debrief**: `"Hollow King defeat, somber orchestral cadence in D minor + descending strings + low brass + funeral choir + cooling-lava crackle, 70 BPM, fallen king lament"`, seed 510004

**Track `music_boss_frozen_heart`** (The Frozen Heart, world europa, 3 phases) — 240s, 95 BPM, B minor, "Dark Cinematic"
- **calm**: `"Frozen Heart boss arena entry, glacial dread orchestral with ice-crystal chimes + low choir + cold synth pad in B minor + creature heartbeat + sonar ping, 95 BPM, deep-cold confrontation"`, seed 520001
- **buildup**: `"Frozen Heart phase 1 buildup, ice-crystal crescendo + cryogenic synth swell in B minor + driving drum + whisper choir + cold-snap percussion, 110 BPM, supercooled awakening"`, seed 520002
- **climax**: `"Frozen Heart phase 2/3 combat, frigid orchestral in B minor + full strings + ice-chime + thundering drum + creature roar + screaming brass + supercooled-core whine, 125 BPM, cryogenic meltdown war"`, seed 520003
- **debrief**: `"Frozen Heart defeat, mournful strings in B minor + descending chime + sparse cello + ice shatter + soft choir, 65 BPM, heart-of-ice lament"`, seed 520004

**Track `music_boss_crimson_tide`** (The Crimson Tide, world mars, 4 phases) — 240s, 105 BPM, F minor, "Epic Cinematic"
- **calm**: `"Crimson Tide boss arena entry, dust-storm orchestral with rust-grit percussion + heavy brass + low synth pad in F minor + Bedouin choir + sand-walker chant, 105 BPM, sandstorm-titan looms"`, seed 530001
- **buildup**: `"Crimson Tide phase 1/2 buildup, sandstorm crescendo + bass throb in F minor + driving tribal drum + windswept brass + war chant, 120 BPM, swarm tide rising"`, seed 530002
- **climax**: `"Crimson Tide phase 3/4 combat, furious orchestral in F minor + full tribal drums + heavy brass + dust-storm howl + war choir + creature roars + crumbling-arena rumble, 135 BPM, four-phase sand-titan war"`, seed 530003
- **debrief**: `"Crimson Tide defeat, settling-dust orchestral in F minor + receding tribal drum + lone reed flute + sparse cello + wind sigh, 70 BPM, sand-buried lament"`, seed 530004

**Track `music_boss_eclipse_walker`** (The Eclipse Walker, world mimas, 3 phases) — 240s, 102 BPM, C# minor, "Cyberpunk"
- **calm**: `"Eclipse Walker boss arena entry, microgravity-eerie orchestral with floating synth pad in C# minor + ethereal choir + gravity-warp synth + slow heartbeat + reverb cello, 102 BPM, weightless cyborg presence"`, seed 540001
- **buildup**: `"Eclipse Walker phase 1 buildup, cyborg-precision crescendo + bass pulse in C# minor + tight drum + glitch percussion + ascending choir, 118 BPM, gravity inversion incoming"`, seed 540002
- **climax**: `"Eclipse Walker phase 2/3 combat, frantic electronic-orchestral in C# minor + full drum + brass + cyborg-vocals + gravity-warp synth lead + agile percussion, 132 BPM, microgravity duel"`, seed 540003
- **debrief**: `"Eclipse Walker defeat, drifting synth pad in C# minor + receding choir + soft chime + cooling-cyborg whine, 65 BPM, weightless lament"`, seed 540004

**Track `music_boss_last_star`** (The Last Star, world vulcan, 5 phases) — **300s**, 110 BPM, A minor, "Religious / Choral Cinematic"
- **calm**: `"Last Star superboss arena entry, stellar-cathedral orchestral with cathedral organ + full choir + low brass + slow heartbeat + synth pad in A minor + cosmic-wind howl, 110 BPM, end-of-campaign confrontation"`, seed 550001
- **buildup**: `"Last Star phase 1/2 buildup, choir crescendo + organ build + bass throb in A minor + ascending strings + ritual percussion + stellar-flare hiss, 125 BPM, sol-zone-titan awakens"`, seed 550002
- **climax**: `"Last Star phase 3/4/5 combat, climactic epic orchestral in A minor + full choir + powerful brass + thundering drums + leitmotif return + screaming synth lead + cosmic-roar samples + stellar-wrath howl, 140 BPM, end-game superboss war"`, seed 550003
- **debrief**: `"Last Star defeat, triumphant resolved orchestral in A major + ascending choir + warm brass + sustained organ + soft drum + dawn-chime + receding stellar wind, 90 BPM, campaign-ending triumph"`, seed 550004

---

## Appendix B — `prompts.json` builder snippet

If you want a faster way to materialize Appendix A into JSON without retyping, here's a Python snippet that builds it from the 30 base track stanzas above:

```python
# Paste each track's 4 variants into ROWS as (track_id, variant, dur, bpm, key, style, summary, full, seed)
import json
ROWS = [
    # ----- WORLDS (48) -----
    ("music_world_earth", "calm", 240, 82, "D minor", "Modern Cinematic",
     "Earth post-collapse wasteland, slow synth pad in D minor + distant wind + sparse piano motif, no drums, contemplative",
     "Earth post-collapse wasteland ambient, ruined-city melancholy, slow synth pad in D minor + distant wind + sparse piano motif + faint radio interference, 82 BPM, no drums, cinematic sci-fi grimdark, contemplative", 110001),
    # ... (add the 119 others from Appendix A) ...
]
SCHEMA = ["track_id","variant","duration_seconds","tempo_bpm","key","aiva_style","prompt_summary","musicgen_prompt","seed"]
data = [dict(zip(SCHEMA, r)) for r in ROWS]
open("prompts.json","w").write(json.dumps(data, indent=2))
```

You don't NEED to do this if you'd rather iterate through the 4 variants inline — but a `prompts.json` is required for the bake skeleton above to run as-is.

---

## Appendix C — Common failure modes + recovery

| Symptom | Cause | Recovery |
|---|---|---|
| AIVA returns 401 / redirects to login | Session expired | Re-run capture_session.py |
| AIVA returns 429 / "queue full" modal | Rate limited | sleep 5 min, resume from progress.json |
| AIVA returns 402 "out of downloads" | Hit 300/month cap | Halt; product owner adds credits or waits for monthly reset |
| Generated track is < 30s | Generation got stuck mid-render | Delete + retry (rare) |
| AIVA UI shows a tutorial overlay | First-login state leaked | dismiss in capture_session.py; re-save state.json |
| AIVA Style picker returns no match | Style name changed | use `--recon` mode to inspect; pick nearest match by hand for that one track + update Section 4 table |
| Playwright timeout on `wait_for_load_state` | AIVA is slow today | bump timeouts to 60_000ms (already at 20-30k); add exponential backoff |
| WAV downloaded but corrupt | Race condition | redownload via the track's menu; second attempt usually works |
| Chromium crashes | Memory leak after ~200 generations | restart browser every 50 tracks; the skeleton above doesn't do this — add a `if i % 50 == 0: ctx.close(); browser.close(); restart` loop |

---

## Appendix D — Contact

If you hit a blocker that requires a decision (e.g., "AIVA changed the Studio app URL", "Style names refactored", "CAPTCHA appeared"), do NOT improvise large changes. Save `progress.json`, screenshot the page, and ping the product owner via the channel set up at handoff time. They will decide within 2 hours of waking time (Phoenix MST).

For routine selector tweaks, just iterate with `--recon` and adjust the selectors in `bake.py`. You have full authority over that file.

---

**Last updated**: 5/15/2026 (initial issue, Phoenix MST). If you receive this doc later than 5/22/2026, ping the product owner — AIVA's UI may have drifted.

— End handoff
