use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::Arc;

#[cfg(target_os = "macos")]
pub use super::capture_macos_tap::TapAggregate;
use crate::error::AppError;

pub struct OpenedCapture {
    pub stream: cpal::Stream,
    pub consumer: rtrb::Consumer<f32>,
    pub device_rate: u32,
    pub device_name: String,
    pub device_id: String,
    #[cfg(target_os = "macos")]
    pub tap: Option<TapAggregate>,
}

/// 既定の出力デバイスのループバックを開いて再生（play）まで行う。
/// `dropped_frames` と `stream_error` は、コールバックから書くためのアトミック。
pub fn open_loopback(
    _dropped_frames: Arc<AtomicU64>,
    _stream_error: Arc<AtomicBool>,
) -> Result<OpenedCapture, AppError> {
    Err(AppError::LoopbackUnsupported("未実装".to_string()))
}

/// いまの既定の出力デバイスの ID（作り直しの判断に使う）。
pub fn current_default_output_id() -> Result<String, AppError> {
    Err(AppError::NoOutputDevice)
}
