use std::sync::Arc;

use crate::audio::clip::CapturedClip;
use crate::error::AppError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CaptureStatus {
    Starting,
    Running {
        device_name: String,
        device_rate: u32,
    },
    Failed {
        message: String,
    },
}

#[derive(Debug)]
pub enum EngineEvent {
    CaptureStarted {
        device_name: String,
        device_rate: u32,
    },
    CaptureFailed(AppError),
    LiveAutoStopped,
    FramesDropped(u64),
    SilentSinceStart,
}

pub struct AudioEngine {
    _private: (),
}

impl AudioEngine {
    /// 300 秒ぶんを確保し、snipwave-audio スレッドを始める。
    /// `wake` は、イベントを送ったあとに呼ぶ（UI の `ctx.request_repaint()` を渡す）。
    pub fn start(_wake: Arc<dyn Fn() + Send + Sync>) -> Result<Self, AppError> {
        Ok(Self { _private: () })
    }

    /// 直前 `seconds` 秒を取り込む。持っている量が少なければその量。0 フレームなら AppError::NoAudioYet。
    pub fn snapshot_last(&self, _seconds: u32) -> Result<CapturedClip, AppError> {
        Err(AppError::NoAudioYet)
    }

    pub fn start_live(&self) -> Result<(), AppError> {
        Err(AppError::Internal("未実装".to_string()))
    }

    pub fn stop_live(&self) -> Result<CapturedClip, AppError> {
        Err(AppError::NoAudioYet)
    }

    pub fn is_live(&self) -> bool {
        false
    }

    pub fn live_frames(&self) -> u64 {
        0
    }

    pub fn status(&self) -> CaptureStatus {
        CaptureStatus::Starting
    }

    pub fn peak_levels(&self) -> [f32; 2] {
        [0.0, 0.0]
    }

    pub fn dropped_frames(&self) -> u64 {
        0
    }

    pub fn drain_events(&self) -> Vec<EngineEvent> {
        Vec::new()
    }

    pub fn shutdown(self) {}
}
