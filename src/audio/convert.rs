use crate::error::AppError;

/// 任意のチャンネル数のインターリーブを 2ch にする反復子を返す（確保なし。コールバックで使う）。
/// 1ch → 左右に同じ値。2ch 以上 → 先頭の 2 チャンネル（Front L / R）を使い、残りは捨てる。
pub fn to_stereo<T>(_input: &[T], _channels: usize) -> impl Iterator<Item = f32> + '_
where
    T: cpal::Sample,
    f32: cpal::FromSample<T>,
{
    std::iter::empty()
}

/// デバイスのレート → 48kHz。レートが同じなら素通し。
pub struct StereoResampler {
    _private: (),
}

impl StereoResampler {
    pub fn new(_input_rate: u32) -> Result<Self, AppError> {
        Ok(Self { _private: () })
    }

    /// 2ch インターリーブを受け取り、48kHz に変換したぶんを `out` の末尾に足す。
    /// 塊に満たない端数は中に持ち越す。
    pub fn process(&mut self, _input: &[f32], _out: &mut Vec<f32>) -> Result<(), AppError> {
        Ok(())
    }
}
