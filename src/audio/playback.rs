use std::num::NonZero;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use rodio::{
    ChannelCount, DeviceSinkBuilder, DeviceSinkError, MixerDeviceSink, SampleRate, Source,
};

use crate::audio::clip::CapturedClip;
use crate::audio::{CHANNELS, SAMPLE_RATE};
use crate::error::AppError;

const CLIP_CHANNELS: ChannelCount = match NonZero::new(CHANNELS as u16) {
    Some(v) => v,
    None => panic!("CHANNELS must be non-zero"),
};

const CLIP_SAMPLE_RATE: SampleRate = match NonZero::new(SAMPLE_RATE) {
    Some(v) => v,
    None => panic!("SAMPLE_RATE must be non-zero"),
};

struct PlaybackShared {
    /// 上位 32 bit が start_frame、下位 32 bit が end_frame。
    /// 1 回のロード・ストアで範囲の整合を保つため 64 bit に詰める。
    range: AtomicU64,
    looping: AtomicBool,
    position_frame: AtomicUsize,
    finished: AtomicBool,
}

impl PlaybackShared {
    fn new(start_frame: usize, end_frame: usize, looping: bool) -> Self {
        Self {
            range: AtomicU64::new(pack_range(start_frame, end_frame)),
            looping: AtomicBool::new(looping),
            position_frame: AtomicUsize::new(start_frame),
            finished: AtomicBool::new(false),
        }
    }

    fn stopped() -> Self {
        Self {
            range: AtomicU64::new(0),
            looping: AtomicBool::new(false),
            position_frame: AtomicUsize::new(0),
            finished: AtomicBool::new(true),
        }
    }

    fn range(&self) -> (usize, usize) {
        unpack_range(self.range.load(Ordering::Acquire))
    }

    fn set_range(&self, start_frame: usize, end_frame: usize) {
        self.range
            .store(pack_range(start_frame, end_frame), Ordering::Release);
    }
}

fn pack_range(start_frame: usize, end_frame: usize) -> u64 {
    let start = u32::try_from(start_frame).unwrap_or(u32::MAX);
    let end = u32::try_from(end_frame).unwrap_or(u32::MAX);
    (u64::from(start) << 32) | u64::from(end)
}

fn unpack_range(packed: u64) -> (usize, usize) {
    let start = (packed >> 32) as usize;
    let end = (packed as u32) as usize;
    (start, end)
}

/// 試聴。選択範囲 [start, end) を鳴らす。範囲とループは再生中でも変えられる。
pub struct Player {
    rodio_player: Option<rodio::Player>,
    sink: Option<MixerDeviceSink>,
    shared: Arc<PlaybackShared>,
}

impl Default for Player {
    fn default() -> Self {
        Self::new()
    }
}

impl Player {
    pub fn new() -> Self {
        Self {
            rodio_player: None,
            sink: None,
            shared: Arc::new(PlaybackShared::stopped()),
        }
    }

    /// 出力を開いて（初回だけ）再生を始める。再生中なら頭から鳴らし直す。
    pub fn play(
        &mut self,
        clip: Arc<CapturedClip>,
        start_frame: usize,
        end_frame: usize,
        looping: bool,
    ) -> Result<(), AppError> {
        let total_frames = clip.frames();
        if start_frame >= end_frame || end_frame > total_frames {
            return Err(AppError::InvalidSelection);
        }

        let sink = match self.sink.as_ref() {
            Some(sink) => sink,
            None => {
                let mut opened =
                    DeviceSinkBuilder::open_default_sink().map_err(|err| match err {
                        DeviceSinkError::NoDevice => AppError::NoOutputDevice,
                        other => AppError::Playback(other.to_string()),
                    })?;
                opened.log_on_drop(false);
                self.sink.insert(opened)
            }
        };

        // 前のソースがあれば即座に打ち切ってから新しいソースを接続する。
        self.shared.finished.store(true, Ordering::Release);
        if let Some(old_player) = self.rodio_player.take() {
            old_player.stop();
        }

        let shared = Arc::new(PlaybackShared::new(start_frame, end_frame, looping));
        let source = ClipSource::new(clip, Arc::clone(&shared));
        let rodio_player = rodio::Player::connect_new(sink.mixer());
        rodio_player.append(source);

        self.shared = shared;
        self.rodio_player = Some(rodio_player);
        Ok(())
    }

    pub fn stop(&mut self) {
        self.shared.finished.store(true, Ordering::Release);
        if let Some(player) = self.rodio_player.take() {
            player.stop();
        }
        self.sink = None;
    }

    pub fn set_range(&self, start_frame: usize, end_frame: usize) {
        if start_frame >= end_frame {
            return;
        }
        self.shared.set_range(start_frame, end_frame);
    }

    pub fn set_looping(&self, looping: bool) {
        self.shared.looping.store(looping, Ordering::Release);
    }

    pub fn is_playing(&self) -> bool {
        !self.shared.finished.load(Ordering::Acquire)
            && self.rodio_player.as_ref().is_some_and(|p| !p.empty())
    }

    pub fn position_frame(&self) -> usize {
        self.shared.position_frame.load(Ordering::Relaxed)
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        self.stop();
    }
}

/// rodio::Source の実装。start / end / looping / position / finished はアトミック。
/// 位置が範囲の外に出たら start に戻る（ループなしで end に達したら終わる）。
struct ClipSource {
    clip: Arc<CapturedClip>,
    shared: Arc<PlaybackShared>,
    current_frame: usize,
    next_channel: usize,
}

impl ClipSource {
    fn new(clip: Arc<CapturedClip>, shared: Arc<PlaybackShared>) -> Self {
        let (start_frame, _) = shared.range();
        Self {
            clip,
            shared,
            current_frame: start_frame,
            next_channel: 0,
        }
    }
}

impl Iterator for ClipSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next_channel == 0 {
            if self.shared.finished.load(Ordering::Acquire) {
                return None;
            }

            let total_frames = self.clip.frames();
            let (raw_start, raw_end) = self.shared.range();
            let end = raw_end.min(total_frames);
            let start = raw_start.min(end);
            if start >= end {
                self.shared.finished.store(true, Ordering::Release);
                return None;
            }

            if self.current_frame < start {
                self.current_frame = start;
            } else if self.current_frame >= end {
                if self.shared.looping.load(Ordering::Acquire) {
                    self.current_frame = start;
                } else {
                    self.shared.position_frame.store(end, Ordering::Relaxed);
                    self.shared.finished.store(true, Ordering::Release);
                    return None;
                }
            }

            self.shared
                .position_frame
                .store(self.current_frame, Ordering::Relaxed);
        }

        let sample_index = self.current_frame * CHANNELS + self.next_channel;
        let sample = self.clip.samples.get(sample_index).copied();

        self.next_channel += 1;
        if self.next_channel >= CHANNELS {
            self.next_channel = 0;
            self.current_frame += 1;
        }

        sample
    }
}

impl Source for ClipSource {
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> ChannelCount {
        CLIP_CHANNELS
    }

    fn sample_rate(&self) -> SampleRate {
        CLIP_SAMPLE_RATE
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::clip::ClipSourceKind;

    #[test]
    fn clip_source_loops_from_range_end_back_to_range_start() {
        let clip = Arc::new(CapturedClip {
            id: 1,
            samples: vec![
                0.0, 0.5, // frame 0
                1.0, 1.5, // frame 1 (range start)
                2.0, 2.5, // frame 2
                3.0, 3.5, // frame 3 (range end - 1)
                4.0, 4.5, // frame 4
            ],
            source: ClipSourceKind::Catch,
            captured_at: chrono::Local::now(),
        });

        let shared = Arc::new(PlaybackShared::new(1, 4, true));
        let source = ClipSource::new(clip, Arc::clone(&shared));

        assert_eq!(source.channels().get(), CHANNELS as u16);
        assert_eq!(source.sample_rate().get(), SAMPLE_RATE);

        let got: Vec<f32> = source.take(12).collect();
        assert_eq!(
            got,
            vec![
                1.0, 1.5, 2.0, 2.5, 3.0, 3.5, // 1 周目
                1.0, 1.5, 2.0, 2.5, 3.0, 3.5, // 2 周目
            ]
        );
        assert!(!shared.finished.load(Ordering::Acquire));
    }
}
