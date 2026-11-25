pub mod capture;
#[cfg(target_os = "macos")]
mod capture_macos_tap;
pub mod clip;
pub mod convert;
pub mod engine;
pub mod peaks;
pub mod playback;
pub mod ring;

pub const SAMPLE_RATE: u32 = 48_000;
pub const CHANNELS: usize = 2;
pub const MIN_BUFFER_SECONDS: u32 = 5;
pub const MAX_BUFFER_SECONDS: u32 = 300;
pub const DEFAULT_BUFFER_SECONDS: u32 = 30;
pub const RING_CAPACITY_FRAMES: usize = (SAMPLE_RATE * MAX_BUFFER_SECONDS) as usize;
pub const LIVE_MAX_SECONDS: u32 = 600;
pub const HANDOFF_SECONDS: u32 = 2;
pub const MIN_SELECTION_FRAMES: usize = 480;
