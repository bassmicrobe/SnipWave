#!/usr/bin/env bash
set -euo pipefail

# T1 / T1b 検証用 .app の組み立て・署名・起動・直接実行スクリプト
# 使い方:
#   scripts/macos/probe-app.sh [loopback|tap] [build|run|rebuild|rebuild-run|direct]
#   scripts/macos/probe-app.sh [build|run|rebuild|rebuild-run|direct]   # 省略時は loopback
# 例:
#   scripts/macos/probe-app.sh build              # Probe.app を組み立て・署名（スタンプ固定）
#   scripts/macos/probe-app.sh run                # Probe.app を起動（target/probe/loopback/probe.{log,wav}）
#   scripts/macos/probe-app.sh rebuild            # スタンプを現在時刻にして Probe.app を作り直し・再署名のみ
#   scripts/macos/probe-app.sh rebuild-run        # スタンプを現在時刻にして作り直し・再署名・起動（rebuild.{log,wav}）
#   scripts/macos/probe-app.sh loopback direct    # loopback_probe を端末から直接実行（direct.{log,wav}）
#   scripts/macos/probe-app.sh tap build          # TapProbe.app を組み立て・署名
#   scripts/macos/probe-app.sh tap run            # TapProbe.app を起動（target/probe/tap/probe.{log,wav}）
#   scripts/macos/probe-app.sh tap rebuild        # スタンプを現在時刻にして TapProbe.app を作り直し・再署名のみ
#   scripts/macos/probe-app.sh tap rebuild-run    # スタンプを現在時刻にして TapProbe.app を作り直し・再署名・起動

usage() {
  echo "使い方: $0 [loopback|tap] [build|run|rebuild|rebuild-run|direct]" >&2
  echo "  direct は loopback のみ対応しています。" >&2
  exit 1
}

TARGET="loopback"
MODE="build"

if [ "$#" -gt 2 ]; then
  usage
fi

case "${1:-}" in
  "")
    ;;
  build|run|rebuild|rebuild-run|direct)
    if [ "$#" -gt 1 ]; then
      usage
    fi
    MODE="$1"
    ;;
  loopback|tap)
    TARGET="$1"
    MODE="${2:-build}"
    ;;
  *)
    usage
    ;;
esac

case "$MODE" in
  build|run|rebuild|rebuild-run|direct) ;;
  *)
    usage
    ;;
esac

if [ "$TARGET" = "tap" ] && [ "$MODE" = "direct" ]; then
  echo "エラー: direct モードは loopback のみ対応しています（T1 の確認項目）。" >&2
  exit 1
fi

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
CDHASH_LOG="$OUT_DIR/cdhash.log"

case "$MODE" in
  rebuild|rebuild-run)
    BUILD_STAMP="$(date +%Y%m%dT%H%M%S)-$$"
    export SNIPWAVE_PROBE_BUILD_STAMP="$BUILD_STAMP"
    echo "==> SNIPWAVE_PROBE_BUILD_STAMP=$BUILD_STAMP"
    ;;
  *)
    unset SNIPWAVE_PROBE_BUILD_STAMP
    BUILD_STAMP="default"
    ;;
esac

echo "==> cargo build --example $EXAMPLE_NAME"
cargo build --example "$EXAMPLE_NAME"

BIN_SRC="$REPO_ROOT/target/debug/examples/$EXAMPLE_NAME"
if [ ! -x "$BIN_SRC" ]; then
  echo "エラー: ビルド済みバイナリが見つかりません: $BIN_SRC" >&2
  exit 1
fi

mkdir -p "$OUT_DIR"

if [ "$MODE" = "direct" ]; then
  WAV_PATH="$OUT_DIR/direct.wav"
  STDOUT_LOG="$OUT_DIR/direct_stdout.log"
  STDERR_LOG="$OUT_DIR/direct_stderr.log"
  COMBINED_LOG="$OUT_DIR/direct.log"
  AUDIO_BEFORE="$OUT_DIR/direct_audio_before.txt"
  AUDIO_AFTER="$OUT_DIR/direct_audio_after.txt"

  echo "==> 起動前の出力デバイス情報を記録します（サンプルレート・チャンネル数。ビット数は Audio MIDI 設定で確認してください）"
  system_profiler SPAudioDataType > "$AUDIO_BEFORE"

  rm -f "$STDOUT_LOG" "$STDERR_LOG" "$COMBINED_LOG" "$WAV_PATH"

  echo "==> 直接実行します（約 10 秒かかります）: $BIN_SRC --output $WAV_PATH"
  PROBE_EXIT=0
  "$BIN_SRC" --output "$WAV_PATH" > "$STDOUT_LOG" 2> "$STDERR_LOG" || PROBE_EXIT=$?

  echo "==> 起動後の出力デバイス情報を記録します"
  system_profiler SPAudioDataType > "$AUDIO_AFTER"

  {
    echo "=== [0] execution ==="
    echo "mode: direct"
    echo "command: $BIN_SRC --output $WAV_PATH"
    echo "build_stamp: $BUILD_STAMP"
    echo "exit_code: $PROBE_EXIT"
    echo ""
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

  echo "==> 実行完了 (exit_code=$PROBE_EXIT)"
  echo "ログ: $COMBINED_LOG"
  echo "WAV:  $WAV_PATH"
  echo "--- stdout ---"
  if [ -f "$STDOUT_LOG" ]; then cat "$STDOUT_LOG"; fi
  if [ -s "$STDERR_LOG" ]; then
    echo "--- stderr ---"
    cat "$STDERR_LOG"
  fi
  exit "$PROBE_EXIT"
fi

if [ ! -f "$INFO_EXT_PLIST" ]; then
  echo "エラー: $INFO_EXT_PLIST が見つかりません" >&2
  exit 1
fi

echo "==> ${APP_NAME}.app を組み立てます: $APP_DIR"
rm -rf "$APP_DIR"
mkdir -p "$MACOS_DIR"
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

CODESIGN_INFO="$(codesign -dvvv "$APP_DIR" 2>&1)"
CDHASH_LINE="$(printf '%s\n' "$CODESIGN_INFO" | grep '^CDHash=' || true)"
if [ -z "$CDHASH_LINE" ]; then
  echo "エラー: codesign -dvvv から CDHash= の行を取得できませんでした" >&2
  printf '%s\n' "$CODESIGN_INFO" >&2
  exit 1
fi

SIGNED_AT="$(date '+%Y-%m-%dT%H:%M:%S%z')"
printf '[%s] mode=%s build_stamp=%s %s\n' "$SIGNED_AT" "$MODE" "$BUILD_STAMP" "$CDHASH_LINE" >> "$CDHASH_LOG"
echo "==> $CDHASH_LINE (build_stamp=$BUILD_STAMP)"
echo "==> 署名履歴: $CDHASH_LOG"
echo "==> 完了: $APP_DIR"

if [ "$MODE" = "build" ] || [ "$MODE" = "rebuild" ]; then
  echo "起動するには次を実行してください:"
  echo "  scripts/macos/probe-app.sh $TARGET run"
  echo "  scripts/macos/probe-app.sh $TARGET rebuild-run  # バイナリを変えて再署名・起動"
  exit 0
fi

if [ "$MODE" = "rebuild-run" ]; then
  WAV_PATH="$OUT_DIR/rebuild.wav"
  STDOUT_LOG="$OUT_DIR/rebuild_stdout.log"
  STDERR_LOG="$OUT_DIR/rebuild_stderr.log"
  COMBINED_LOG="$OUT_DIR/rebuild.log"
  AUDIO_BEFORE="$OUT_DIR/rebuild_audio_before.txt"
  AUDIO_AFTER="$OUT_DIR/rebuild_audio_after.txt"
else
  WAV_PATH="$OUT_DIR/probe.wav"
  STDOUT_LOG="$OUT_DIR/stdout.log"
  STDERR_LOG="$OUT_DIR/stderr.log"
  COMBINED_LOG="$OUT_DIR/probe.log"
  AUDIO_BEFORE="$OUT_DIR/audio_before.txt"
  AUDIO_AFTER="$OUT_DIR/audio_after.txt"
fi

echo "==> 起動前の出力デバイス情報を記録します（サンプルレート・チャンネル数。ビット数は Audio MIDI 設定で確認してください）"
system_profiler SPAudioDataType > "$AUDIO_BEFORE"

rm -f "$STDOUT_LOG" "$STDERR_LOG" "$COMBINED_LOG" "$WAV_PATH"

echo "==> ${APP_NAME}.app を起動します（約 10 秒かかります）"
PROBE_EXIT=0
open -W -n --stdout "$STDOUT_LOG" --stderr "$STDERR_LOG" "$APP_DIR" --args --output "$WAV_PATH" || PROBE_EXIT=$?

echo "==> 起動後の出力デバイス情報を記録します"
system_profiler SPAudioDataType > "$AUDIO_AFTER"

{
  echo "=== [0] codesign ==="
  echo "mode: $MODE"
  echo "build_stamp: $BUILD_STAMP"
  echo "$CDHASH_LINE"
  echo "open_exit_code: $PROBE_EXIT"
  echo ""
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

echo "==> 実行完了 (open_exit_code=$PROBE_EXIT)"
echo "ログ: $COMBINED_LOG"
echo "WAV:  $WAV_PATH"
echo "--- stdout ---"
if [ -f "$STDOUT_LOG" ]; then cat "$STDOUT_LOG"; fi
if [ -s "$STDERR_LOG" ]; then
  echo "--- stderr ---"
  cat "$STDERR_LOG"
fi
exit "$PROBE_EXIT"
