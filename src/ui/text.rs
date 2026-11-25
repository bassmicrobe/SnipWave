//! 画面に出す文言（言語を切り替えるときはここを直す）。

pub const APP_TITLE: &str = "SnipWave";

// ヘッダー
pub const STATUS_STARTING: &str = "取得を始めています";
pub const STATUS_RUNNING: &str = "取得中";
pub const STATUS_FAILED: &str = "取得できません";
pub const SETTINGS_BUTTON: &str = "設定";
pub const CHANNEL_LEFT: &str = "L";
pub const CHANNEL_RIGHT: &str = "R";

// 操作バー
pub const CATCH: &str = "Catch";
pub const REC: &str = "Rec";
pub const STOP: &str = "Stop";
pub const BUFFER_SECONDS_LABEL: &str = "遡り秒数";
pub const SECONDS_UNIT: &str = "秒";
pub const HOTKEY_UNREGISTERED: &str = "（未登録）";

// 波形・待機表示
pub const IDLE_PLACEHOLDER: &str = "Catch で直前の音を取り込むか、Rec で録音を始めます";

// 編集バー
pub const START_LABEL: &str = "Start";
pub const END_LABEL: &str = "End";
pub const LENGTH_LABEL: &str = "長さ";
pub const PREVIEW_PLAY: &str = "試聴";
pub const PREVIEW_STOP: &str = "停止";
pub const LOOP_LABEL: &str = "ループ";
pub const FORMAT_LABEL: &str = "形式";
pub const FORMAT_INT24: &str = "24-bit";
pub const FORMAT_FLOAT32: &str = "32-bit float";
pub const EXPORT_BUTTON: &str = "書き出し";
pub const EXPORTING_BUTTON: &str = "書き出し中…";
pub const CLOSE_CLIP_BUTTON: &str = "閉じる";

// 設定パネル
pub const SETTINGS_TITLE: &str = "設定";
pub const OUTPUT_DIR_LABEL: &str = "出力先";
pub const CHANGE_BUTTON: &str = "変更…";
pub const HOTKEYS_LABEL: &str = "ホットキー";
pub const HOTKEYS_HINT: &str = "変えるには config.toml を書き換えて再起動します";
pub const CONFIG_PATH_LABEL: &str = "設定ファイル";
pub const CAPTURE_INFO_LABEL: &str = "取得の情報";
pub const DEVICE_NAME_LABEL: &str = "デバイス";
pub const DEVICE_RATE_LABEL: &str = "サンプルレート";
pub const DROPPED_FRAMES_LABEL: &str = "取りこぼし";
pub const DISCLAIMER_LABEL: &str = "免責事項";
pub const DISCLAIMER: &str = "本ツールは個人の音楽制作・学習・検証用途を想定しています。YouTube、Netflix等の外部サービスから取得した音声の利用については、各サービスの利用規約および著作権法を遵守してください。";
pub const VERSION_LABEL: &str = "バージョン";

// トレイメニュー
pub const TRAY_SHOW_WINDOW: &str = "ウィンドウを表示";
pub const TRAY_CATCH: &str = "Catch";
pub const TRAY_REC_START: &str = "録音を開始";
pub const TRAY_REC_STOP: &str = "録音を停止";
pub const TRAY_QUIT: &str = "終了";

// 状態表示・通知
pub const HISTORY_BUTTON: &str = "履歴";
pub const SAVED_PREFIX: &str = "保存しました";
pub const MSG_CATCH_WHILE_RECORDING: &str = "録音中は Catch できません";
pub const MSG_SILENT_MACOS: &str = "音声が届いていません。音が鳴っているのにこの表示が消えないときは、システム設定で SnipWave のシステムオーディオ録音を許可してください";
pub const MSG_WAYLAND_HOTKEY_NOTICE: &str =
    "Wayland ではホットキーが効かないことがあります。ボタンとトレイで操作してください";
pub const MSG_WAYLAND_NO_HIDE_NOTICE: &str =
    "Wayland 環境ではウィンドウを隠せないため、閉じると終了します";
pub const MSG_NO_TRAY_NOTICE: &str = "トレイを作成できなかったため、ウィンドウを閉じると終了します";
pub const MSG_LIVE_AUTO_STOPPED: &str = "10 分に達したので録音を止めました";
pub const MSG_DEVICE_CHANGED_PREFIX: &str = "出力デバイスが変わりました";
pub const MSG_DROPPED_FRAMES_PREFIX: &str = "音声を取りこぼしました";
pub const MSG_CLIPPED_SAMPLES_NOTICE: &str = "音量が 0 dBFS を超えたサンプルを切り詰めました";

// テーマ名（確認用・切り替え用）
pub const THEME_SYSTEM: &str = "システム";
pub const THEME_DARK: &str = "ダーク";
pub const THEME_LIGHT: &str = "ライト";
