use crate::audio::{CHANNELS, SAMPLE_RATE};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipSourceKind {
    Catch,
    Live,
}

/// 取り込んだ音声。48kHz / 2ch / f32 のインターリーブ。作ったあとは変えない。
#[derive(Clone, Debug)]
pub struct CapturedClip {
    pub id: u64,
    pub samples: Vec<f32>,
    pub source: ClipSourceKind,
    pub captured_at: chrono::DateTime<chrono::Local>,
}

impl CapturedClip {
    pub fn frames(&self) -> usize {
        self.samples.len() / CHANNELS
    }

    pub fn duration_secs(&self) -> f64 {
        self.frames() as f64 / f64::from(SAMPLE_RATE)
    }
}
