use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("設定を読み書きできません（{path}）: {message}")]
    Config { path: PathBuf, message: String },
    #[error("出力デバイスが見つかりません")]
    NoOutputDevice,
    #[error("この環境ではシステム音声を取り込めません: {0}")]
    LoopbackUnsupported(String),
    #[error("音声の形式に対応していません: {0}")]
    UnsupportedFormat(String),
    #[error("音声の取得でエラーが起きました: {0}")]
    Capture(String),
    #[error("まだ取り込める音声がありません")]
    NoAudioYet,
    #[error("選択範囲が正しくありません")]
    InvalidSelection,
    #[error("書き出せません（{path}）: {message}")]
    Export { path: PathBuf, message: String },
    #[error("再生できません: {0}")]
    Playback(String),
    #[error("ホットキー「{key}」を登録できません: {message}")]
    Hotkey { key: String, message: String },
    #[error("トレイを作れません: {0}")]
    Tray(String),
    #[error("内部エラー: {0}")]
    Internal(String),
}
