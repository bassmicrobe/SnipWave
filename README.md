# SnipWave

## macOS 音声取得プローブ（検証用）

### T1: `loopback_probe`（cpal によるシステム音声取得の確認）

ビルドと `.app` の組み立て・ad-hoc 署名のみ:

```sh
scripts/macos/probe-app.sh build
```

組み立て・署名して起動（ログと `probe.wav` は `target/probe/loopback/` に保存される）:

```sh
scripts/macos/probe-app.sh run
```

バイナリのみビルドして直接実行する場合:

```sh
cargo build --example loopback_probe
target/debug/examples/loopback_probe
```

デバイスの情報だけを確認する場合（ストリームを開かない）:

```sh
target/debug/examples/loopback_probe --info-only
```

### T1b: `tap_probe`（自前の Process Tap ＋集約デバイスの確認）

ビルドと `.app` の組み立て・ad-hoc 署名のみ:

```sh
scripts/macos/probe-app.sh tap build
```

組み立て・署名して起動（ログと `probe.wav` は `target/probe/tap/` に保存される）:

```sh
scripts/macos/probe-app.sh tap run
```

デバイスの情報だけを確認する場合（Tap とストリームを作らない）:

```sh
cargo build --example tap_probe
target/debug/examples/tap_probe --info-only
```
