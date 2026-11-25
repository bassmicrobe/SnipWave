use crate::audio::clip::CapturedClip;
use crate::audio::peaks::PeakCache;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Selection {
    pub start_frame: usize,
    pub end_frame: usize,
}

#[derive(Default)]
pub struct WaveformView {
    _private: (),
}

impl WaveformView {
    /// 描いて、操作があれば新しい選択を返す。
    pub fn show(
        &mut self,
        _ui: &mut egui::Ui,
        _clip: &CapturedClip,
        _peaks: &PeakCache,
        _selection: Selection,
        _playhead_frame: Option<usize>,
    ) -> Option<Selection> {
        None
    }
}

/// 画面の x 座標 → フレーム番号（純粋な関数。テストの対象）
pub fn x_to_frame(_x: f32, _rect_left: f32, _rect_width: f32, _total_frames: usize) -> usize {
    0
}

pub fn selection_from_drag(
    anchor_frame: usize,
    current_frame: usize,
    _total_frames: usize,
) -> Selection {
    Selection {
        start_frame: anchor_frame.min(current_frame),
        end_frame: anchor_frame.max(current_frame),
    }
}
