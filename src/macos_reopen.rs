use crate::error::AppError;

/// 自前の NSApplicationDelegate（`applicationShouldHandleReopen:hasVisibleWindows:` だけを持つ）を保持する。
pub struct ReopenDelegate {
    _private: (),
}

/// Dock のアイコンが押されたら `on_reopen` を呼ぶデリゲートを登録する。
pub fn install(_on_reopen: impl Fn() + 'static) -> Result<ReopenDelegate, AppError> {
    Ok(ReopenDelegate { _private: () })
}
