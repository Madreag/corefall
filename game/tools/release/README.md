# Corefall Release Engineering — `game/tools/release/`

This directory holds the static assets + helper scripts the
`.github/workflows/release.yml` workflow uses to package
**double-click-playable** release artifacts per the AGENTS.md
**Double-Click Playability Hard Gate** (search the project AGENTS.md for
`DOUBLE-CLICK PLAYABILITY CONTRACT`).

## Per-platform artifact contract

| Platform | Output | What the friend does |
|---|---|---|
| macOS  arm64 / Intel | `Corefall-<TAG>-macos-<arch>.dmg` containing `Corefall.app` | Double-click `.dmg` → drag `Corefall.app` to Applications → double-click `Corefall.app` → game window opens |
| Windows x86_64       | `Corefall-<TAG>-windows-x86_64.zip` containing `Corefall-<TAG>/Corefall.exe` | Right-click `.zip` → Extract All → double-click `Corefall.exe` → game window opens (SmartScreen "More info → Run anyway" is acceptable through BP9; BP10+ requires Authenticode signing) |
| Linux x86_64         | `Corefall-<TAG>-linux-x86_64.AppImage` | `chmod +x` on first run is the friend's only manual step (most distros' file manager has a "Make executable" toggle in the file's properties) → double-click → game window opens |

Each release additionally publishes the legacy `corefall-<platform>-<TAG>.tar.zst`
/ `corefall-windows-<TAG>.zip` developer archives under the
`corefall-cli-<platform>` artifact name so internal tooling that depends
on the raw archives keeps working.

## Files in this directory

| File | Purpose |
|---|---|
| `Info.plist.template` | macOS `.app` bundle metadata. The release workflow `sed`-substitutes `__CFBUNDLE_SHORT_VERSION__` (e.g., `0.3.0`) and `__CFBUNDLE_VERSION__` (build number) before copying it into `Corefall.app/Contents/Info.plist`. |
| `Corefall.macos-launcher.sh` | macOS `Corefall.app/Contents/MacOS/Corefall` shell launcher. Finder launches the bundle with cwd=$HOME so cf-app's relative `content/scenarios/<id>.ron` lookup would fail; this launcher resolves the bundle's `Contents/Resources/` and `cd`s there before exec'ing the real `cf-app` binary (which lives next to the launcher in `Contents/MacOS/cf-app`). Keeping the launcher as the `CFBundleExecutable` keeps cf-app's behavior unchanged for CI / cfctl / cf-e2e callers. |
| `AppRun.sh` | Linux AppImage launcher. Sets up `LD_LIBRARY_PATH` and `PATH`, `cd`s into the bundled `usr/share/corefall` (so cf-app resolves the bundled `content/` tree relative to its working dir), then `exec`s `usr/bin/cf-app`. |
| `corefall.desktop` | Linux desktop entry the AppImage embeds at the AppDir root so the AppImage runtime (`appimagetool`) can register the app with the host file manager. |
| `corefall.icns` | macOS icon (placeholder — solid orange square + "CF" glyph). Regenerate via `python3 generate_icons.py` on macOS. |
| `corefall.png` | 1024×1024 master PNG for Linux AppImage. Also written at 256×256 (`corefall_256.png`) for `.desktop` icon usage. |
| `generate_icons.py` | One-shot helper to regenerate the icons. Requires `pillow`; uses `iconutil` on macOS to compose the multi-size `.icns`. |
| `icon.iconset/` | Multi-size PNG set for `iconutil -c icns`. Build intermediate; safe to delete and regenerate. |

## Local sanity-check on macOS

```bash
cd /Users/erol/projects/corefall
cargo build --release -p cf-app -p cfctl -p cf-e2e -p cf-tools-replay-viewer --manifest-path game/Cargo.toml
bash game/tools/release/build_macos_app.sh \
  --target-dir game/target/release \
  --output /tmp/Corefall-test.dmg \
  --version 0.0.0-local
open /tmp/Corefall-test.dmg
```

(The `build_macos_app.sh` helper mirrors the workflow's macOS packaging
step so local agents can reproduce the bundle without pushing a tag.)

## Limitations + TODOs

- **Icons are placeholders.** Solid orange square with "CF" glyph; safe
  for prealpha + alpha + beta channels. Replace with the marketing icon
  before the BP10 RC channel transitions.
- **Code signing is ad-hoc.** macOS uses `codesign --force --deep --sign -`
  which produces a Gatekeeper-rejectable signature. Friends must
  right-click `Corefall.app` → Open → Open the first time. Full
  Developer ID Application signing + notarization arrives at BP10
  (Class A authorization required for the cert).
- **Windows binaries are unsigned.** SmartScreen will warn; "More info →
  Run anyway" is the documented friend workaround through BP9.
  Authenticode signing is BP10+ scope.
- **AppImage bundles bevy/wgpu dynamic deps via system libs**. The
  AppImage itself is fully self-contained for the launcher + `cf-app`
  binary, but Vulkan / X11 / Wayland system libraries come from the
  host. Most modern distros (Ubuntu 22.04+, Fedora 38+, Arch current)
  have these by default; the AGENTS.md AppImage requirement is met
  because the friend never types a `sudo apt install` command.
- **`Corefall.exe` on Windows is `cf-app.exe` renamed in-place.** This
  works because `cf-app` defaults to `--scenario m1_actor_range` per
  the AGENTS.md Hard Gate. If a future BP needs a launcher menu / mode
  selector, replace the in-place rename with a real launcher .exe that
  delegates to `cf-app.exe`.
- **No `.msi` installer for Windows yet.** The `.zip` form is
  AGENTS.md-compliant ("either an `.msi` installer OR a `.zip`
  containing `Corefall.exe`"). Adding `.msi` packaging via WiX is
  nice-to-have BP-level work.
- **No `.deb` / `.rpm` packages.** AppImage is the AGENTS.md primary;
  distro-specific packages are explicitly nice-to-haves, not
  requirements.

## Friend-handoff verification checklist

When a BP closes with this release engineering, the implementing agent
must verify the following via the matrix below in the BP closure note
under `## Friend-Handoff Verification`:

```text
Platform        | Artifact                                  | Verified by                | Friend-flow result
macOS aarch64   | Corefall-<TAG>-macos-aarch64.dmg          | <agent + machine + date>  | <prose>
macOS x86_64    | Corefall-<TAG>-macos-x86_64.dmg           | <agent + machine + date>  | <prose>
Windows x86_64  | Corefall-<TAG>-windows-x86_64.zip         | <agent + machine + date>  | <prose>
Linux x86_64    | Corefall-<TAG>-linux-x86_64.AppImage      | <agent + machine + date>  | <prose>
```

If any platform's row says "broken" or "not tested", omit that platform
from the release matrix in the same PR. Do NOT publish a partial
release that pretends to support a platform whose artifact cannot be
opened.
