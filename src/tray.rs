use crate::error::AppError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrayAction {
    ShowWindow,
    Catch,
    ToggleRec,
    Quit,
}

pub struct Tray {
    _private: (),
}

impl Tray {
    /// メニュー: 「ウィンドウを表示」「Catch（直前 N 秒）」「Rec 開始 / 停止」「終了」。
    /// アイコンの左クリックは ShowWindow。
    pub fn create(
        _on_action: impl Fn(TrayAction) + Send + Sync + 'static,
    ) -> Result<Self, AppError> {
        Err(AppError::Tray("未実装".to_string()))
    }

    pub fn set_recording(&self, _recording: bool) -> Result<(), AppError> {
        Ok(())
    }

    pub fn set_buffer_seconds(&self, _seconds: u32) -> Result<(), AppError> {
        Ok(())
    }
}
