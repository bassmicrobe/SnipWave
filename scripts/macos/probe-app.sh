#!/usr/bin/env bash
set -euo pipefail

# T1 / T1b 検証用 .app の組み立て・署名・起動スクリプト
# 使い方:
#   scripts/macos/probe-app.sh [loopback|tap] [build|run]
#   scripts/macos/probe-app.sh [build|run]        # 省略時は loopback
# 例:
#   scripts/macos/probe-app.sh build              # Probe.app (loopback_probe) を組み立て・署名
#   scripts/macos/probe-app.sh run                # Probe.app を起動（target/probe/loopback/ に出力）
#   scripts/macos/probe-app.sh tap build          # TapProbe.app (tap_probe) を組み立て・署名
#   scripts/macos/probe-app.sh tap run            # TapProbe.app を起動（target/probe/tap/ に出力）

TARGET="loopback"
MODE="build"

case "${1:-}" in
  "")
    ;;
  build|run)
    MODE="$1"
    ;;
  loopback|tap)
    TARGET="$1"
    MODE="${2:-build}"
    ;;
  *)
    echo "使い方: $0 [loopback|tap] [build|run]" >&2
    exit 1
    ;;
esac

case "$MODE" in
  build|run) ;;
  *)
    echo "使い方: $0 [loopback|tap] [build|run]" >&2
    exit 1
    ;;
esac

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$REPO_ROOT"

if [ "$TARGET" = "loopback" ]; then
  EXAMPLE_NAME="loopback_probe"
  APP_NAME="Probe"
  BUNDLE_ID="com.snipwave.Probe"
else
  EXAMPLE_NAME="tap_probe"
  APP_NAME="TapProbe"
  BUNDLE_ID="com.snipwave.TapProbe"
fi

PROBE_ROOT="$REPO_ROOT/target/probe"
APP_DIR="$PROBE_ROOT/${APP_NAME}.app"
CONTENTS_DIR="$APP_DIR/Contents"
MACOS_DIR="$CONTENTS_DIR/MacOS"
INFO_PLIST="$CONTENTS_DIR/Info.plist"
INFO_EXT_PLIST="$REPO_ROOT/packaging/macos/Info.ext.plist"

OUT_DIR="$PROBE_ROOT/$TARGET"
WAV_PATH="$OUT_DIR/probe.wav"
STDOUT_LOG="$OUT_DIR/stdout.log"
STDERR_LOG="$OUT_DIR/stderr.log"
COMBINED_LOG="$OUT_DIR/probe.log"
AUDIO_BEFORE="$OUT_DIR/audio_before.txt"
AUDIO_AFTER="$OUT_DIR/audio_after.txt"

if [ ! -f "$INFO_EXT_PLIST" ]; then
  echo "エラー: $INFO_EXT_PLIST が見つかりません" >&2
  exit 1
fi

echo "==> cargo build --example $EXAMPLE_NAME"
cargo build --example "$EXAMPLE_NAME"

BIN_SRC="$REPO_ROOT/target/debug/examples/$EXAMPLE_NAME"
if [ ! -x "$BIN_SRC" ]; then
  echo "エラー: ビルド済みバイナリが見つかりません: $BIN_SRC" >&2
  exit 1
fi

echo "==> ${APP_NAME}.app を組み立てます: $APP_DIR"
rm -rf "$APP_DIR"
mkdir -p "$MACOS_DIR" "$OUT_DIR"
cp "$BIN_SRC" "$MACOS_DIR/$EXAMPLE_NAME"
chmod +x "$MACOS_DIR/$EXAMPLE_NAME"

INFO_EXT_CONTENT="$(cat "$INFO_EXT_PLIST")"
cat > "$INFO_PLIST" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key>
  <string>ja</string>
  <key>CFBundleExecutable</key>
  <string>${EXAMPLE_NAME}</string>
  <key>CFBundleIdentifier</key>
  <string>${BUNDLE_ID}</string>
  <key>CFBundleInfoDictionaryVersion</key>
  <string>6.0</string>
  <key>CFBundleName</key>
  <string>${APP_NAME}</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>0.1.0</string>
  <key>CFBundleVersion</key>
  <string>1</string>
  <key>LSMinimumSystemVersion</key>
  <string>14.6</string>
${INFO_EXT_CONTENT}
</dict>
</plist>
EOF

plutil -lint "$INFO_PLIST"

echo "==> ad-hoc 署名します: $APP_DIR"
codesign --force --sign - "$APP_DIR"

echo "==> 完了: $APP_DIR"

if [ "$MODE" = "build" ]; then
  echo "起動するには次を実行してください:"
  echo "  scripts/macos/probe-app.sh $TARGET run"
  exit 0
fi

echo "==> 起動前の出力デバイス情報を記録します（サンプルレート・チャンネル数。ビット数は Audio MIDI 設定で確認してください）"
system_profiler SPAudioDataType > "$AUDIO_BEFORE"

rm -f "$STDOUT_LOG" "$STDERR_LOG" "$COMBINED_LOG" "$WAV_PATH"

echo "==> ${APP_NAME}.app を起動します（約 10 秒かかります）"
open -W -n --stdout "$STDOUT_LOG" --stderr "$STDERR_LOG" "$APP_DIR" --args --output "$WAV_PATH"

echo "==> 起動後の出力デバイス情報を記録します"
system_profiler SPAudioDataType > "$AUDIO_AFTER"

{
  echo "=== [1] system_profiler SPAudioDataType (before) ==="
  cat "$AUDIO_BEFORE"
  echo ""
  echo "=== [2] stdout ($STDOUT_LOG) ==="
  if [ -f "$STDOUT_LOG" ]; then cat "$STDOUT_LOG"; fi
  echo ""
  echo "=== [3] stderr ($STDERR_LOG) ==="
  if [ -f "$STDERR_LOG" ]; then cat "$STDERR_LOG"; fi
  echo ""
  echo "=== [4] system_profiler SPAudioDataType (after) ==="
  cat "$AUDIO_AFTER"
  echo ""
  echo "=== [5] diff (before vs after) ==="
  if diff -u "$AUDIO_BEFORE" "$AUDIO_AFTER"; then
    echo "(差分なし)"
  fi
} > "$COMBINED_LOG"

echo "==> 実行完了"
echo "ログ: $COMBINED_LOG"
echo "WAV:  $WAV_PATH"
echo "--- stdout ---"
if [ -f "$STDOUT_LOG" ]; then cat "$STDOUT_LOG"; fi
if [ -s "$STDERR_LOG" ]; then
  echo "--- stderr ---"
  cat "$STDERR_LOG"
fi
