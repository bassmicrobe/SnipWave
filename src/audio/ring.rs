use crate::audio::CHANNELS;

/// 300 秒ぶんを起動時に 1 回だけ確保する循環バッファ。確保し直さない。
pub struct RingBuffer {
    data: Vec<f32>,
    _write_frame: usize,
    filled_frames: usize,
}

impl RingBuffer {
    pub fn new(capacity_frames: usize) -> Self {
        Self {
            data: vec![0.0; capacity_frames.saturating_mul(CHANNELS)],
            _write_frame: 0,
            filled_frames: 0,
        }
    }

    pub fn capacity_frames(&self) -> usize {
        self.data.len() / CHANNELS
    }

    pub fn filled_frames(&self) -> usize {
        self.filled_frames
    }

    /// インターリーブのサンプルを足す。古いものから上書きする。長さは CHANNELS の倍数。
    pub fn push_interleaved(&mut self, _samples: &[f32]) {}

    /// 末尾から `frames` ぶんを、古い順に `out` へ写す。持っている量が少なければその量だけ。
    /// 写したフレーム数を返す。`out` は空にしてから書く。
    pub fn copy_latest(&self, _frames: usize, out: &mut Vec<f32>) -> usize {
        out.clear();
        0
    }
}
