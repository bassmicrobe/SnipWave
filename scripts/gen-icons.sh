#!/usr/bin/env bash
set -euo pipefail

# Generate all app and tray icons from assets/logo/*.svg into assets/icons/.
# Required tools: rsvg-convert (librsvg), iconutil (macOS), magick (ImageMagick).

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

LOGO_DIR="$ROOT_DIR/assets/logo"
OUT_DIR="$ROOT_DIR/assets/icons"

ICON_ACCENT_SVG="$LOGO_DIR/snipwave-icon-accent.svg"
MARK_MONO_BLACK_SVG="$LOGO_DIR/snipwave-mark-mono-black.svg"

for cmd in rsvg-convert iconutil magick; do
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "error: required command not found: $cmd" >&2
    exit 1
  fi
done

for src in "$ICON_ACCENT_SVG" "$MARK_MONO_BLACK_SVG"; do
  if [ ! -f "$src" ]; then
    echo "error: source SVG not found: $src" >&2
    exit 1
  fi
done

mkdir -p "$OUT_DIR/macos" "$OUT_DIR/windows" "$OUT_DIR/tray"

# 1. App PNGs (Linux, window icon uses 256)
for size in 16 32 48 64 128 256 512 1024; do
  rsvg-convert -w "$size" -h "$size" "$ICON_ACCENT_SVG" -o "$OUT_DIR/app-${size}.png"
done

# 2. macOS .icns (824x824 centered on transparent 1024x1024 canvas for macOS icon padding)
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

PADDED_1024="$TMP_DIR/macos-padded-1024.png"
magick "$OUT_DIR/app-1024.png" \
  -resize 824x824 \
  -background none \
  -gravity center \
  -extent 1024x1024 \
  "PNG32:$PADDED_1024"

ICONSET_DIR="$TMP_DIR/icon.iconset"
mkdir -p "$ICONSET_DIR"

magick "$PADDED_1024" -resize 16x16     "PNG32:$ICONSET_DIR/icon_16x16.png"
magick "$PADDED_1024" -resize 32x32     "PNG32:$ICONSET_DIR/icon_16x16@2x.png"
magick "$PADDED_1024" -resize 32x32     "PNG32:$ICONSET_DIR/icon_32x32.png"
magick "$PADDED_1024" -resize 64x64     "PNG32:$ICONSET_DIR/icon_32x32@2x.png"
magick "$PADDED_1024" -resize 128x128   "PNG32:$ICONSET_DIR/icon_128x128.png"
magick "$PADDED_1024" -resize 256x256   "PNG32:$ICONSET_DIR/icon_128x128@2x.png"
magick "$PADDED_1024" -resize 256x256   "PNG32:$ICONSET_DIR/icon_256x256.png"
magick "$PADDED_1024" -resize 512x512   "PNG32:$ICONSET_DIR/icon_256x256@2x.png"
magick "$PADDED_1024" -resize 512x512   "PNG32:$ICONSET_DIR/icon_512x512.png"
cp "$PADDED_1024"                       "$ICONSET_DIR/icon_512x512@2x.png"

iconutil -c icns "$ICONSET_DIR" -o "$OUT_DIR/macos/icon.icns"

# 3. Windows .ico (16, 32, 48, 256)
magick \
  "$OUT_DIR/app-16.png" \
  "$OUT_DIR/app-32.png" \
  "$OUT_DIR/app-48.png" \
  "$OUT_DIR/app-256.png" \
  "$OUT_DIR/windows/icon.ico"

# 4. macOS tray template PNGs (black mark on transparent background)
for size in 22 44; do
  rsvg-convert -w "$size" -h "$size" "$MARK_MONO_BLACK_SVG" -o "$OUT_DIR/tray/tray-template-${size}.png"
done

# 5. Windows / Linux tray PNGs
for size in 32 64; do
  rsvg-convert -w "$size" -h "$size" "$ICON_ACCENT_SVG" -o "$OUT_DIR/tray/tray-${size}.png"
done

echo "Generated icons under $OUT_DIR"
