# SnipWave アーキテクチャ設計メモ

## スレッド構成
1. **Audio Capture Thread（OS別ループバック取得）**
   - 48kHz / 2ch / f32 でシステム音声を常時受信。
   - 常に `AudioRingBuffer` へ流し込む（パターンA用）。
   - `is_live_recording == true` の間は、並行して `live_buffer: Vec<f32>` へも追記する（パターンB用）。
2. **UI Thread（eframe / egui）**
   - 60fpsで波形・レベルメーター・ボタンを描画。
   - 「Catch」押下時：`ring_buffer.snapshot(config.buffer_seconds)` を取得して `captured_audio` へセット。
   - 「Live Rec」押下時：`start_live_rec()` を呼び出し、「Stop」押下時に `stop_live_rec()` で録音済み `Vec<f32>` を受け取り `captured_audio` へセット。
   - 「Export WAV」押下時：`captured_audio[start..end]` を `hound` で書き出し。

## 状態遷移（録音モード）
- `Idle`: リングバッファのみ常時稼働。
- `LiveRecording`: リングバッファ稼働 ＋ ライブ録音バッファへ追記中。
- `Editing`: 取り込んだ `captured_audio` を波形表示・トリミング中（裏のリングバッファは稼働継続）。
