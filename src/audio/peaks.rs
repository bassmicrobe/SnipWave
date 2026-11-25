use crate::audio::clip::CapturedClip;

/// 波形用。256 フレームごとの最小・最大（左右の平均）を、取り込み時に 1 回だけ作る。
pub struct PeakCache {
    _private: (),
}

impl PeakCache {
    pub const BLOCK_FRAMES: usize = 256;

    pub fn build(_clip: &CapturedClip) -> Self {
        Self { _private: () }
    }

    /// クリップ全体を `width_px` 本の列に分け、列ごとの (最小, 最大) を返す。
    /// 1 列が BLOCK_FRAMES より短いときは、クリップのサンプルを直接見る。
    pub fn columns(&self, _clip: &CapturedClip, _width_px: usize) -> Vec<(f32, f32)> {
        Vec::new()
    }
}
