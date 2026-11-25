# SnipWave

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
