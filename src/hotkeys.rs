use crate::config::AppConfig;
use crate::error::AppError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HotkeyAction {
    Catch,
    ToggleRec,
}

pub struct Hotkeys {
    _private: (),
}

impl Hotkeys {
    /// 2 つを登録する。片方が失敗しても、もう片方は登録する。
    /// 失敗（文字列が読めない、ほかのアプリが使っている、Wayland など）は Vec で返す。
    /// 押された（Pressed）ときだけ `on_action` を呼ぶ。`on_action` の中では送信と request_repaint だけをする。
    pub fn register(
        _config: &AppConfig,
        _on_action: impl Fn(HotkeyAction) + Send + Sync + 'static,
    ) -> (Option<Self>, Vec<AppError>) {
        (None, Vec::new())
    }
}
