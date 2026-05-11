#!/usr/bin/env bash
# build_macos_app.sh — assemble Corefall.app + .dmg from a built cf-app.
#
# Mirrors the macOS packaging step in .github/workflows/release.yml so
# local agents can reproduce the bundle without pushing a tag. Useful
# for the friend-handoff verification step in BP closure notes.
#
# Usage:
#   bash game/tools/release/build_macos_app.sh \
#       --target-dir game/target/release \
#       --output /tmp/Corefall-test.dmg \
#       --version 0.0.0-local
#
# The script:
#   1. Copies cf-app into Corefall.app/Contents/MacOS/Corefall.
#   2. Writes Info.plist from the template (sed-substitutes version).
#   3. Copies content/ + scripts/cfctl/ into Corefall.app/Contents/Resources/.
#   4. Embeds corefall.icns.
#   5. Ad-hoc signs with `codesign --force --deep --sign -`.
#   6. Wraps in a .dmg via hdiutil.

set -euo pipefail

TARGET_DIR=""
OUTPUT_DMG=""
VERSION=""
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")"/../../.. && pwd)"

while [ $# -gt 0 ]; do
  case "$1" in
    --target-dir) TARGET_DIR="$2"; shift 2 ;;
    --output) OUTPUT_DMG="$2"; shift 2 ;;
    --version) VERSION="$2"; shift 2 ;;
    --repo-root) REPO_ROOT="$2"; shift 2 ;;
    *) echo "unknown arg: $1" >&2; exit 2 ;;
  esac
done

if [ -z "${TARGET_DIR}" ] || [ -z "${OUTPUT_DMG}" ] || [ -z "${VERSION}" ]; then
  echo "usage: $0 --target-dir <dir> --output <dmg> --version <semver>" >&2
  exit 2
fi

if [ ! -x "${TARGET_DIR}/cf-app" ]; then
  echo "error: ${TARGET_DIR}/cf-app not found or not executable" >&2
  echo "       run: cargo build --release -p cf-app --manifest-path ${REPO_ROOT}/game/Cargo.toml" >&2
  exit 1
fi

REL_DIR="${REPO_ROOT}/game/tools/release"
WORK="$(mktemp -d -t corefall-app.XXXXXX)"
trap 'rm -rf "${WORK}"' EXIT

APP="${WORK}/Corefall.app"
mkdir -p "${APP}/Contents/MacOS" "${APP}/Contents/Resources"

# 1a. Real cf-app binary lives next to the launcher; bundle CLI helpers
#     (cfctl, cf-e2e, cf-tools-replay-viewer) ride along for power users.
cp "${TARGET_DIR}/cf-app" "${APP}/Contents/MacOS/cf-app"
chmod +x "${APP}/Contents/MacOS/cf-app"
for helper in cfctl cf-e2e cf-tools-replay-viewer; do
  if [ -x "${TARGET_DIR}/${helper}" ]; then
    cp "${TARGET_DIR}/${helper}" "${APP}/Contents/MacOS/${helper}"
  fi
done

# 1b. CFBundleExecutable points at this shell launcher so the bundle
#     `cd`s into Contents/Resources/ (where the bundled `content/` tree
#     lives) before exec'ing cf-app. cf-app's relative scenario lookup
#     (`content/scenarios/<id>.ron`) only works from that cwd; without
#     the launcher Finder would launch with cwd=$HOME and the default
#     scenario load would fail.
cp "${REL_DIR}/Corefall.macos-launcher.sh" "${APP}/Contents/MacOS/Corefall"
chmod +x "${APP}/Contents/MacOS/Corefall"

# 2. Info.plist.
SHORT_VERSION="${VERSION#v}"
SHORT_VERSION="${SHORT_VERSION%%-*}"
sed \
  -e "s|__CFBUNDLE_SHORT_VERSION__|${SHORT_VERSION}|g" \
  -e "s|__CFBUNDLE_VERSION__|${VERSION}|g" \
  "${REL_DIR}/Info.plist.template" > "${APP}/Contents/Info.plist"

# 3. Resources: content + scripts + repo docs.
cp -R "${REPO_ROOT}/game/content" "${APP}/Contents/Resources/content"
mkdir -p "${APP}/Contents/Resources/scripts"
cp -R "${REPO_ROOT}/game/scripts/cfctl" "${APP}/Contents/Resources/scripts/cfctl"
for doc in README.md CHANGELOG.md AGENTS.md LICENSE LICENSE-MIT LICENSE-APACHE; do
  if [ -f "${REPO_ROOT}/${doc}" ]; then
    cp "${REPO_ROOT}/${doc}" "${APP}/Contents/Resources/${doc}"
  fi
done

# 4. Icon.
if [ -f "${REL_DIR}/corefall.icns" ]; then
  cp "${REL_DIR}/corefall.icns" "${APP}/Contents/Resources/corefall.icns"
fi

# 5. Ad-hoc sign. Right-click → Open is the friend's first-run workaround
#    until BP10 lands real Developer ID + notarization.
if command -v codesign >/dev/null 2>&1; then
  codesign --force --deep --sign - "${APP}" 1>/dev/null
  echo "ad-hoc signed: ${APP}"
fi

# 6. .dmg.
if ! command -v hdiutil >/dev/null 2>&1; then
  echo "error: hdiutil not on PATH; can't pack .dmg" >&2
  exit 1
fi

DMG_TMP="${WORK}/Corefall-${VERSION}.dmg"
hdiutil create -volname "Corefall ${VERSION}" \
  -srcfolder "${APP}" \
  -ov -format UDZO \
  "${DMG_TMP}"

mkdir -p "$(dirname "${OUTPUT_DMG}")"
cp "${DMG_TMP}" "${OUTPUT_DMG}"
shasum -a 256 "${OUTPUT_DMG}" > "${OUTPUT_DMG}.sha256"

echo
echo "Corefall.app + .dmg built:"
echo "  bundle: ${APP}"
echo "  dmg:    ${OUTPUT_DMG}"
echo "  sha256: ${OUTPUT_DMG}.sha256"
echo
echo "Friend-handoff test:"
echo "  open '${OUTPUT_DMG}'"
echo "  drag Corefall.app to Applications, double-click → game window."
