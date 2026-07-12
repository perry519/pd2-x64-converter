#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

BUNDLE_DIR="$ROOT/target/release/bundle/appimage"
APPDIR="$BUNDLE_DIR/PD2 x64 Converter.AppDir"
APP_VERSION="$(jq -er '.version' package.json)"
OUTPUT="$BUNDLE_DIR/PD2 x64 Converter_${APP_VERSION}_amd64.AppImage"
PLUGIN="${TAURI_APPIMAGE_PLUGIN:-$HOME/.cache/tauri/linuxdeploy-plugin-appimage.AppImage}"
LOG="$(mktemp)"
trap 'rm -f "$LOG"' EXIT

BUILD_SUCCEEDED=false
if WEBKIT_DISABLE_DMABUF_RENDERER=1 pnpm exec tauri build --bundles appimage 2>&1 | tee "$LOG"; then
  BUILD_SUCCEEDED=true
elif ! rg -q "failed to run linuxdeploy|Strip call failed|\\.relr\\.dyn" "$LOG"; then
  exit 1
fi

if [[ ! -d "$APPDIR" ]]; then
  echo "AppDir not found: $APPDIR" >&2
  exit 1
fi

ICON="$ROOT/src-tauri/icons/icon.png"
THEME_ICON="$APPDIR/usr/share/icons/hicolor/128x128/apps/pd2-x64-converter-gui.png"
if [[ ! -f "$ICON" ]]; then
  echo "Source icon not found: $ICON" >&2
  exit 1
fi

if [[ ! -x "$PLUGIN" ]]; then
  echo "Tauri AppImage plugin not found: $PLUGIN" >&2
  exit 1
fi

cp "$ICON" "$THEME_ICON"
cp "$ICON" "$APPDIR/pd2-x64-converter-gui.png"
ln -sfn pd2-x64-converter-gui.png "$APPDIR/.DirIcon"

if [[ "$BUILD_SUCCEEDED" == false ]]; then
  cat >"$APPDIR/AppRun" <<'APPRUN'
#!/usr/bin/env sh
set -eu

APPDIR="${APPDIR:-$(dirname "$(readlink -f "$0")")}"
export LD_LIBRARY_PATH="$APPDIR/usr/lib:$APPDIR/usr/lib64:${LD_LIBRARY_PATH:-}"
export WEBKIT_DISABLE_DMABUF_RENDERER="${WEBKIT_DISABLE_DMABUF_RENDERER:-1}"

exec "$APPDIR/usr/bin/pd2-x64-converter-gui" "$@"
APPRUN
  chmod +x "$APPDIR/AppRun"
fi

rm -f "$APPDIR/usr/bin/xdg-open" "$OUTPUT"

LDAI_OUTPUT="$OUTPUT" \
  LINUXDEPLOY_OUTPUT_APP_NAME="PD2 x64 Converter" \
  LINUXDEPLOY_OUTPUT_VERSION="$APP_VERSION" \
  "$PLUGIN" --appimage-extract-and-run --appdir "$APPDIR"

echo "Built AppImage: $OUTPUT"
