# SnipWave

PC で再生されているシステム音声（ループバック音声）を「遡りキャプチャ（Catch）」または「ライブ録音（Rec / Stop）」で取り込み、波形で範囲を選んで WAV（48kHz / 2ch、24-bit int または 32-bit float）に書き出す常駐型デスクトップアプリです。

macOS だけ実機での動作を確認しています。Windows と Linux は CI でのビルド確認のみです。

## 必要な環境

- **Rust ツールチェーン**: `1.93.0`（`rust-toolchain.toml` で固定、`rustfmt`・`clippy` を含む）
- **対応 OS**:
  - **macOS 14.6 以上**: Core Audio Process Tap を使用（システム音声取得には `.app` バンドル経由での起動と「システムオーディオ録音」の許可が必要）
  - **Windows 10 / 11**: WASAPI Loopback を使用
  - **Linux**: PipeWire または PulseAudio が必要（ALSA のみではシステム音声を取り込めません。Wayland 環境ではホットキーが使えない場合があり、ボタンとトレイから操作します）
- **macOS `.app` バンドル作成ツール（任意）**: `cargo-bundle 0.12.0`

```sh
cargo install cargo-bundle --version 0.12.0 --locked
```

### Linux で必要なシステムパッケージ

Ubuntu 24.04 での導入例です。`tray-icon` は `ksni`、`rfd` は xdg-portal を使う構成のため GTK（`libgtk-3-dev`）は不要です。

```sh
sudo apt-get update
sudo apt-get install -y --no-install-recommends \
  pkg-config libclang-dev \
  libasound2-dev libpipewire-0.3-dev libpulse-dev \
  libxkbcommon-dev libwayland-dev libgl1-mesa-dev \
  libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev
```

| パッケージ | 用途 |
|---|---|
| `libasound2-dev` | `cpal`（PipeWire / PulseAudio 利用時も必要） |
| `libpipewire-0.3-dev`、`libclang-dev` | `cpal` の `pipewire`（`bindgen` が `libclang` を使用） |
| `libpulse-dev` | `cpal` の `pulseaudio` |
| `libxkbcommon-dev`、`libwayland-dev`、`libgl1-mesa-dev`、`libxcb-render0-dev`、`libxcb-shape0-dev`、`libxcb-xfixes0-dev` | `eframe`（`winit` / `glow`） |
| `pkg-config` | 上記ライブラリの検出 |

## ビルド・実行手順

開発ビルド:

```sh
cargo build --locked
```

起動（画面確認用）:

```sh
cargo run --locked
```

リリースビルド:

```sh
cargo build --release --locked
```

フォーマット・lint・テスト（CI と同じコマンド）:

```sh
cargo fmt --all --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
```

## 免責事項

本ツールは個人の音楽制作・学習・検証用途を想定しています。YouTube、Netflix等の外部サービスから取得した音声の利用については、各サービスの利用規約および著作権法を遵守してください。

---

## macOS 音声取得プローブ（検証用）

### T1: `loopback_probe`（cpal によるシステム音声取得の確認）

ビルドと `.app` の組み立て・ad-hoc 署名のみ（スタンプ固定、署名時の `CDHash=` は `target/probe/loopback/cdhash.log` に追記される）:

```sh
scripts/macos/probe-app.sh build
```

組み立て・署名して起動（ログは `target/probe/loopback/probe.log`、録音は `target/probe/loopback/probe.wav` に保存される。再実行してもスタンプは変わらない）:

```sh
scripts/macos/probe-app.sh run
```

バイナリの中身（`SNIPWAVE_PROBE_BUILD_STAMP`）を現在時刻に変えて作り直し・再署名のみ（`CDHash=` の変化を確認する用。起動しない）:

```sh
scripts/macos/probe-app.sh rebuild
```

バイナリの中身を変えて作り直し・再署名して起動（R3「再署名後に許可が残るか」の確認用。ログは `target/probe/loopback/rebuild.log`、録音は `target/probe/loopback/rebuild.wav` に保存され、`probe.log` / `probe.wav` を上書きしない）:

```sh
scripts/macos/probe-app.sh rebuild-run
```

ターミナルからバイナリを直接実行（`target/debug/examples/loopback_probe` を実行。ログと前後の `system_profiler` は `target/probe/loopback/direct.log`、録音は `target/probe/loopback/direct.wav` に保存される）:

```sh
scripts/macos/probe-app.sh loopback direct
```

デバイスの情報だけを確認する場合（ストリームを開かない）:

```sh
cargo build --example loopback_probe
target/debug/examples/loopback_probe --info-only
```

### T1b: `tap_probe`（自前の Process Tap ＋集約デバイスの確認）

ビルドと `.app` の組み立て・ad-hoc 署名のみ（署名時の `CDHash=` は `target/probe/tap/cdhash.log` に追記される）:

```sh
scripts/macos/probe-app.sh tap build
```

組み立て・署名して起動（ログは `target/probe/tap/probe.log`、録音は `target/probe/tap/probe.wav` に保存される）:

```sh
scripts/macos/probe-app.sh tap run
```

ビルドスタンプを変えて作り直し・再署名のみ / 再署名して起動（`target/probe/tap/rebuild.{log,wav}`）:

```sh
scripts/macos/probe-app.sh tap rebuild
scripts/macos/probe-app.sh tap rebuild-run
```

デバイスの情報だけを確認する場合（Tap とストリームを作らない）:

```sh
cargo build --example tap_probe
target/debug/examples/tap_probe --info-only
```
