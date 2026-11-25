use audioadapter_buffers::direct::InterleavedSlice;
use rubato::{Fft, FixedSync, Resampler};

use crate::audio::{CHANNELS, SAMPLE_RATE};
use crate::error::AppError;

const RESAMPLER_CHUNK_FRAMES: usize = 1024;

struct ToStereoIter<'a, T> {
    input: &'a [T],
    channels: usize,
    frame_index: usize,
    total_frames: usize,
    channel_index: usize,
}

impl<T> Iterator for ToStereoIter<'_, T>
where
    T: cpal::Sample,
    f32: cpal::FromSample<T>,
{
    type Item = f32;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.frame_index >= self.total_frames {
            return None;
        }
        let src_ch = if self.channels == 1 {
            0
        } else {
            self.channel_index
        };
        let sample = self.input[self.frame_index * self.channels + src_ch];
        if self.channel_index == 0 {
            self.channel_index = 1;
        } else {
            self.channel_index = 0;
            self.frame_index += 1;
        }
        Some(<f32 as cpal::Sample>::from_sample(sample))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self
            .total_frames
            .saturating_sub(self.frame_index)
            .saturating_mul(CHANNELS)
            .saturating_sub(self.channel_index);
        (remaining, Some(remaining))
    }
}

/// 任意のチャンネル数のインターリーブを 2ch にする反復子を返す（確保なし。コールバックで使う）。
/// 1ch → 左右に同じ値。2ch 以上 → 先頭の 2 チャンネル（Front L / R）を使い、残りは捨てる。
pub fn to_stereo<T>(input: &[T], channels: usize) -> impl Iterator<Item = f32> + '_
where
    T: cpal::Sample,
    f32: cpal::FromSample<T>,
{
    let total_frames = if channels == 0 {
        0
    } else {
        input.len() / channels
    };
    ToStereoIter {
        input,
        channels,
        frame_index: 0,
        total_frames,
        channel_index: 0,
    }
}

struct ResamplerState {
    resampler: Fft<f32>,
    pending: Vec<f32>,
    output_buf: Vec<f32>,
}

/// デバイスのレート → 48kHz。レートが同じなら素通し。
pub struct StereoResampler {
    state: Option<Box<ResamplerState>>,
}

impl StereoResampler {
    pub fn new(input_rate: u32) -> Result<Self, AppError> {
        if input_rate == 0 {
            return Err(AppError::UnsupportedFormat(
                "サンプルレートが 0 Hz です".to_string(),
            ));
        }
        if input_rate == SAMPLE_RATE {
            return Ok(Self { state: None });
        }

        let resampler = Fft::<f32>::new(
            input_rate as usize,
            SAMPLE_RATE as usize,
            RESAMPLER_CHUNK_FRAMES,
            CHANNELS,
            FixedSync::Input,
        )
        .map_err(|err| {
            AppError::UnsupportedFormat(format!(
                "リサンプラを初期化できません（{input_rate} Hz -> {SAMPLE_RATE} Hz）: {err}"
            ))
        })?;

        let input_frames_max = resampler.input_frames_max();
        let output_frames_max = resampler.output_frames_max();

        Ok(Self {
            state: Some(Box::new(ResamplerState {
                resampler,
                pending: Vec::with_capacity(input_frames_max * CHANNELS * 2),
                output_buf: vec![0.0; output_frames_max * CHANNELS],
            })),
        })
    }

    /// 2ch インターリーブを受け取り、48kHz に変換したぶんを `out` の末尾に足す。
    /// 塊に満たない端数は中に持ち越す。
    pub fn process(&mut self, input: &[f32], out: &mut Vec<f32>) -> Result<(), AppError> {
        if !input.len().is_multiple_of(CHANNELS) {
            return Err(AppError::Internal(format!(
                "リサンプラへの入力サンプル数 {} がチャンネル数 {CHANNELS} の倍数ではありません",
                input.len()
            )));
        }
        if input.is_empty() {
            return Ok(());
        }

        let Some(state) = self.state.as_deref_mut() else {
            out.extend_from_slice(input);
            return Ok(());
        };

        state.pending.extend_from_slice(input);

        let mut consumed_samples = 0;
        loop {
            let needed_frames = state.resampler.input_frames_next();
            let needed_samples = needed_frames * CHANNELS;
            if state.pending.len() - consumed_samples < needed_samples {
                break;
            }

            let in_slice = &state.pending[consumed_samples..consumed_samples + needed_samples];
            let in_adapter =
                InterleavedSlice::new(in_slice, CHANNELS, needed_frames).map_err(|err| {
                    AppError::Internal(format!("リサンプラ入力バッファエラー: {err}"))
                })?;

            let out_frames_cap = state.output_buf.len() / CHANNELS;
            let mut out_adapter =
                InterleavedSlice::new_mut(&mut state.output_buf, CHANNELS, out_frames_cap)
                    .map_err(|err| {
                        AppError::Internal(format!("リサンプラ出力バッファエラー: {err}"))
                    })?;

            let (frames_in, frames_out) = state
                .resampler
                .process_into_buffer(&in_adapter, &mut out_adapter, None)
                .map_err(|err| AppError::Internal(format!("リサンプリング失敗: {err}")))?;

            if frames_in == 0 {
                return Err(AppError::Internal(
                    "リサンプラが入力フレームを消費しませんでした".to_string(),
                ));
            }

            consumed_samples += frames_in * CHANNELS;
            out.extend_from_slice(&state.output_buf[..frames_out * CHANNELS]);
        }

        if consumed_samples > 0 {
            let remaining = state.pending.len() - consumed_samples;
            state.pending.copy_within(consumed_samples.., 0);
            state.pending.truncate(remaining);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::TAU;

    use super::*;

    #[test]
    fn to_stereo_duplicates_mono_and_keeps_first_two_channels() {
        let mono_f32: [f32; 3] = [0.25, -0.5, 0.75];
        let from_mono: Vec<f32> = to_stereo(&mono_f32, 1).collect();
        assert_eq!(from_mono, vec![0.25, 0.25, -0.5, -0.5, 0.75, 0.75]);

        let quad_i16: [i16; 8] = [16_384, 0, 12_345, -12_345, i16::MIN, 8_192, -9_999, 9_999];
        let from_quad: Vec<f32> = to_stereo(&quad_i16, 4).collect();
        assert_eq!(from_quad, vec![0.5, 0.0, -1.0, 0.25]);
    }

    #[test]
    fn resampler_converts_44100_to_48000_length() {
        let input_rate = 44_100_u32;
        let input_frames = input_rate as usize;
        let mut input = Vec::with_capacity(input_frames * CHANNELS);
        for frame in 0..input_frames {
            let phase = TAU * 440.0 * (frame as f32) / (input_rate as f32);
            let sample = phase.sin() * 0.5;
            input.push(sample);
            input.push(-sample);
        }

        let mut resampler = StereoResampler::new(input_rate).unwrap();
        let mut out = Vec::new();
        for chunk in input.chunks(500 * CHANNELS) {
            resampler.process(chunk, &mut out).unwrap();
        }

        assert!(out.len().is_multiple_of(CHANNELS));
        let out_frames = out.len() / CHANNELS;
        let expected_frames = SAMPLE_RATE as usize;
        let tolerance_frames = RESAMPLER_CHUNK_FRAMES * 2;
        assert!(
            out_frames.abs_diff(expected_frames) <= tolerance_frames,
            "出力フレーム数 {out_frames} が期待値 {expected_frames}±{tolerance_frames} の範囲外です"
        );
        assert!(out.iter().any(|&s| s.abs() > 0.1));
    }
}
