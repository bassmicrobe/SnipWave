use std::sync::Arc;

use crate::audio::clip::CapturedClip;
use crate::error::AppError;

/// 試聴。選択範囲 [start, end) を鳴らす。範囲とループは再生中でも変えられる。
#[derive(Default)]
pub struct Player {
    _private: (),
}

impl Player {
    pub fn new() -> Self {
        Self::default()
    }

    /// 出力を開いて（初回だけ）再生を始める。再生中なら頭から鳴らし直す。
    pub fn play(
        &mut self,
        _clip: Arc<CapturedClip>,
        _start_frame: usize,
        _end_frame: usize,
        _looping: bool,
    ) -> Result<(), AppError> {
        Err(AppError::Playback("未実装".to_string()))
    }

    pub fn stop(&mut self) {}

    pub fn set_range(&self, _start_frame: usize, _end_frame: usize) {}

    pub fn set_looping(&self, _looping: bool) {}

    pub fn is_playing(&self) -> bool {
        false
    }

    pub fn position_frame(&self) -> usize {
        0
    }
}
