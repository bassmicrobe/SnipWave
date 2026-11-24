use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

pub const SAMPLE_RATE: u32 = 48_000;
pub const CHANNELS: u16 = 2;

#[derive(Clone)]
pub struct AudioRecorderEngine {
    // パターンA用：常時リングバッファ
    ring_buffer: Arc<Mutex<VecDeque<f32>>>,
    max_samples: usize,
    // パターンB用：ライブ録音バッファ
    live_buffer: Arc<Mutex<Vec<f32>>>,
    is_live_recording: Arc<Mutex<bool>>,
}

impl AudioRecorderEngine {
    pub fn new(max_seconds: u32) -> Self {
        let max_samples = (SAMPLE_RATE * CHANNELS as u32 * max_seconds) as usize;
        Self {
            ring_buffer: Arc::new(Mutex::new(VecDeque::with_capacity(max_samples))),
            max_samples,
            live_buffer: Arc::new(Mutex::new(Vec::new())),
            is_live_recording: Arc::new(Mutex::new(false)),
        }
    }

    /// 音声スレッドから常時呼び出される
    pub fn push_samples(&self, samples: &[f32]) {
        if let Ok(mut buf) = self.ring_buffer.lock() {
            for &s in samples {
                if buf.len() >= self.max_samples {
                    buf.pop_front();
                }
                buf.push_back(s);
            }
        }
        if let Ok(is_rec) = self.is_live_recording.lock() {
            if *is_rec {
                if let Ok(mut live) = self.live_buffer.lock() {
                    live.extend_from_slice(samples);
                }
            }
        }
    }

    /// パターンA：末尾から指定秒数分を遡って取得
    pub fn catch_retroactive(&self, seconds: u32) -> Vec<f32> {
        let target_samples = (SAMPLE_RATE * CHANNELS as u32 * seconds) as usize;
        if let Ok(buf) = self.ring_buffer.lock() {
            let len = buf.len();
            let start = len.saturating_sub(target_samples);
            buf.iter().skip(start).copied().collect()
        } else {
            Vec::new()
        }
    }

    /// パターンB：ライブ録音開始
    pub fn start_live_recording(&self) {
        if let Ok(mut live) = self.live_buffer.lock() {
            live.clear();
        }
        if let Ok(mut is_rec) = self.is_live_recording.lock() {
            *is_rec = true;
        }
    }

    /// パターンB：ライブ録音停止＆データ取得
    pub fn stop_live_recording(&self) -> Vec<f32> {
        if let Ok(mut is_rec) = self.is_live_recording.lock() {
            *is_rec = false;
        }
        if let Ok(mut live) = self.live_buffer.lock() {
            std::mem::take(&mut *live)
        } else {
            Vec::new()
        }
    }

    pub fn is_recording(&self) -> bool {
        self.is_live_recording.lock().map(|v| *v).unwrap_or(false)
    }
}
