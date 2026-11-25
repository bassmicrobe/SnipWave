use crate::audio::clip::CapturedClip;
use crate::audio::CHANNELS;

/// 波形用。256 フレームごとの最小・最大（左右の平均）を、取り込み時に 1 回だけ作る。
pub struct PeakCache {
    clip_id: u64,
    blocks: Vec<(f32, f32)>,
}

impl PeakCache {
    pub const BLOCK_FRAMES: usize = 256;

    pub fn build(clip: &CapturedClip) -> Self {
        let total_frames = clip.frames();
        let mut blocks = Vec::with_capacity(total_frames.div_ceil(Self::BLOCK_FRAMES));
        let mut start = 0;
        while start < total_frames {
            let end = (start + Self::BLOCK_FRAMES).min(total_frames);
            blocks.push(scan_frame_range(&clip.samples, start, end));
            start = end;
        }

        Self {
            clip_id: clip.id,
            blocks,
        }
    }

    /// クリップ全体を `width_px` 本の列に分け、列ごとの (最小, 最大) を返す。
    /// 1 列が BLOCK_FRAMES より短いときは、クリップのサンプルを直接見る。
    pub fn columns(&self, clip: &CapturedClip, width_px: usize) -> Vec<(f32, f32)> {
        let total_frames = clip.frames();
        if width_px == 0 || total_frames == 0 {
            return Vec::new();
        }

        let mut cols = Vec::with_capacity(width_px);
        let total_u128 = total_frames as u128;
        let width_u128 = width_px as u128;

        for x in 0..width_px {
            let start = ((x as u128 * total_u128) / width_u128) as usize;
            let raw_end = (((x + 1) as u128 * total_u128) / width_u128) as usize;
            let end = raw_end.clamp(start + 1, total_frames);

            let peak = if self.clip_id == clip.id && end - start >= Self::BLOCK_FRAMES {
                self.range_peak(&clip.samples, start, end)
            } else {
                scan_frame_range(&clip.samples, start, end)
            };
            cols.push(peak);
        }

        cols
    }

    fn range_peak(&self, samples: &[f32], start: usize, end: usize) -> (f32, f32) {
        let first_full_block = start.div_ceil(Self::BLOCK_FRAMES);
        let last_full_block = (end / Self::BLOCK_FRAMES).min(self.blocks.len());
        if first_full_block > last_full_block {
            return scan_frame_range(samples, start, end);
        }

        let mut acc: Option<(f32, f32)> = None;

        let left_end = first_full_block * Self::BLOCK_FRAMES;
        if left_end > start {
            acc = Some(merge_peak(acc, scan_frame_range(samples, start, left_end)));
        }

        for &block in &self.blocks[first_full_block..last_full_block] {
            acc = Some(merge_peak(acc, block));
        }

        let right_start = last_full_block * Self::BLOCK_FRAMES;
        if end > right_start {
            acc = Some(merge_peak(acc, scan_frame_range(samples, right_start, end)));
        }

        acc.unwrap_or((0.0, 0.0))
    }
}

fn scan_frame_range(samples: &[f32], start_frame: usize, end_frame: usize) -> (f32, f32) {
    if start_frame >= end_frame {
        return (0.0, 0.0);
    }
    let slice = &samples[start_frame * CHANNELS..end_frame * CHANNELS];
    let mut chunks = slice.chunks_exact(CHANNELS);
    let Some(first_frame) = chunks.next() else {
        return (0.0, 0.0);
    };
    let first = 0.5 * (first_frame[0] + first_frame[1]);
    let mut min_val = first;
    let mut max_val = first;
    for frame in chunks {
        let v = 0.5 * (frame[0] + frame[1]);
        min_val = min_val.min(v);
        max_val = max_val.max(v);
    }
    (min_val, max_val)
}

#[inline]
fn merge_peak(acc: Option<(f32, f32)>, next: (f32, f32)) -> (f32, f32) {
    match acc {
        Some((cur_min, cur_max)) => (cur_min.min(next.0), cur_max.max(next.1)),
        None => next,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::clip::ClipSourceKind;

    #[test]
    fn columns_match_original_sample_min_max() {
        let total_frames = 1500;
        let mut samples = Vec::with_capacity(total_frames * CHANNELS);
        for frame in 0..total_frames {
            let t = frame as f32 / total_frames as f32;
            let left = (t * 23.0).sin() * 0.6;
            let right = (t * 17.0).cos() * 0.4;
            samples.push(left);
            samples.push(right);
        }
        // ブロック境界付近やブロック内に特徴的なピークを置く
        samples[10 * CHANNELS] = 0.95;
        samples[10 * CHANNELS + 1] = 0.95;
        samples[255 * CHANNELS] = -0.9;
        samples[255 * CHANNELS + 1] = -0.9;
        samples[256 * CHANNELS] = 0.98;
        samples[256 * CHANNELS + 1] = 0.98;
        samples[900 * CHANNELS] = -0.92;
        samples[900 * CHANNELS + 1] = -0.92;

        let clip = CapturedClip {
            id: 1,
            samples,
            source: ClipSourceKind::Catch,
            captured_at: chrono::Local::now(),
        };
        let cache = PeakCache::build(&clip);

        // 1 列が BLOCK_FRAMES 以上（キャッシュ使用）と未満（直接参照）の両方で一致を確認する
        for width_px in [2_usize, 10_usize] {
            let cols = cache.columns(&clip, width_px);
            assert_eq!(cols.len(), width_px);
            for (x, &(col_min, col_max)) in cols.iter().enumerate() {
                let start = (x * total_frames) / width_px;
                let end = ((x + 1) * total_frames) / width_px;
                let mut expected_min = f32::INFINITY;
                let mut expected_max = f32::NEG_INFINITY;
                for frame in start..end {
                    let mono =
                        0.5 * (clip.samples[frame * CHANNELS] + clip.samples[frame * CHANNELS + 1]);
                    expected_min = expected_min.min(mono);
                    expected_max = expected_max.max(mono);
                }
                assert!(
                    (col_min - expected_min).abs() < 1e-6,
                    "width={width_px}, col={x}: min {col_min} != {expected_min}"
                );
                assert!(
                    (col_max - expected_max).abs() < 1e-6,
                    "width={width_px}, col={x}: max {col_max} != {expected_max}"
                );
            }
        }
    }
}
