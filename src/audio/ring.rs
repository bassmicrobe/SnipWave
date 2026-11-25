use crate::audio::CHANNELS;

/// 300 秒ぶんを起動時に 1 回だけ確保する循環バッファ。確保し直さない。
pub struct RingBuffer {
    data: Vec<f32>,
    write_frame: usize,
    filled_frames: usize,
}

impl RingBuffer {
    pub fn new(capacity_frames: usize) -> Self {
        Self {
            data: vec![0.0; capacity_frames.saturating_mul(CHANNELS)],
            write_frame: 0,
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
    pub fn push_interleaved(&mut self, samples: &[f32]) {
        let capacity = self.capacity_frames();
        let input_frames = samples.len() / CHANNELS;
        if capacity == 0 || input_frames == 0 {
            return;
        }
        let samples = &samples[..input_frames * CHANNELS];

        if input_frames >= capacity {
            let tail_start = (input_frames - capacity) * CHANNELS;
            self.data.copy_from_slice(&samples[tail_start..]);
            self.write_frame = 0;
            self.filled_frames = capacity;
            return;
        }

        let first_frames = input_frames.min(capacity - self.write_frame);
        let first_samples = first_frames * CHANNELS;
        let write_sample = self.write_frame * CHANNELS;
        self.data[write_sample..write_sample + first_samples]
            .copy_from_slice(&samples[..first_samples]);

        let second_frames = input_frames - first_frames;
        if second_frames > 0 {
            let second_samples = second_frames * CHANNELS;
            self.data[..second_samples]
                .copy_from_slice(&samples[first_samples..first_samples + second_samples]);
        }

        self.write_frame = (self.write_frame + input_frames) % capacity;
        self.filled_frames = (self.filled_frames + input_frames).min(capacity);
    }

    /// 末尾から `frames` ぶんを、古い順に `out` へ写す。持っている量が少なければその量だけ。
    /// 写したフレーム数を返す。`out` は空にしてから書く。
    pub fn copy_latest(&self, frames: usize, out: &mut Vec<f32>) -> usize {
        out.clear();
        let count = frames.min(self.filled_frames);
        if count == 0 {
            return 0;
        }

        let capacity = self.capacity_frames();
        let start_frame = if self.write_frame >= count {
            self.write_frame - count
        } else {
            capacity - (count - self.write_frame)
        };

        let first_frames = count.min(capacity - start_frame);
        let first_samples = first_frames * CHANNELS;
        let start_sample = start_frame * CHANNELS;
        let second_samples = (count - first_frames) * CHANNELS;

        out.reserve(count * CHANNELS);
        out.extend_from_slice(&self.data[start_sample..start_sample + first_samples]);
        if second_samples > 0 {
            out.extend_from_slice(&self.data[..second_samples]);
        }

        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_overwrites_oldest_and_copy_latest_preserves_order() {
        let mut ring = RingBuffer::new(4);
        ring.push_interleaved(&[1.0, -1.0, 2.0, -2.0, 3.0, -3.0]);
        ring.push_interleaved(&[4.0, -4.0, 5.0, -5.0, 6.0, -6.0]);

        assert_eq!(ring.capacity_frames(), 4);
        assert_eq!(ring.filled_frames(), 4);

        let mut out = vec![99.0; 2];
        let copied = ring.copy_latest(4, &mut out);
        assert_eq!(copied, 4);
        assert_eq!(out, vec![3.0, -3.0, 4.0, -4.0, 5.0, -5.0, 6.0, -6.0]);
    }

    #[test]
    fn copy_latest_returns_available_without_zero_padding() {
        let mut ring = RingBuffer::new(10);
        ring.push_interleaved(&[0.1, 0.2, 0.3, 0.4, 0.5, 0.6]);

        let mut out = vec![42.0; 4];
        let copied = ring.copy_latest(10, &mut out);
        assert_eq!(copied, 3);
        assert_eq!(out.len(), 3 * CHANNELS);
        assert_eq!(out, vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6]);
    }
}
