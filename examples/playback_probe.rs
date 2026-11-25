//! T8: 試聴（`audio::playback::Player`）のループ再生・停止・末尾での自動停止を確かめるプローブ。
//!
//! 出力デバイスだけを使い、入力ストリームは開かない。
//!
//! 使い方:
//!   cargo run --example playback_probe

use std::f64::consts::TAU;
use std::process::ExitCode;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use snipwave::audio::clip::{CapturedClip, ClipSourceKind};
use snipwave::audio::playback::Player;
use snipwave::audio::{CHANNELS, SAMPLE_RATE};
use snipwave::error::AppError;

const CLIP_SECONDS: usize = 5;
const TONE_FREQ_HZ: f64 = 440.0;
const TONE_DBFS: f64 = -12.0;
const RANGE_START_SECS: usize = 1;
const RANGE_END_SECS: usize = 2;
const LOOP_PLAY_SECONDS: usize = 4;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("エラー: {err}");
            ExitCode::FAILURE
        }
    }
}

fn build_sine_clip() -> Arc<CapturedClip> {
    let total_frames = SAMPLE_RATE as usize * CLIP_SECONDS;
    let amplitude = 10.0_f64.powf(TONE_DBFS / 20.0);
    let mut samples = Vec::with_capacity(total_frames * CHANNELS);

    for frame in 0..total_frames {
        let t = frame as f64 / f64::from(SAMPLE_RATE);
        let value = (amplitude * (TAU * TONE_FREQ_HZ * t).sin()) as f32;
        for _ in 0..CHANNELS {
            samples.push(value);
        }
    }

    Arc::new(CapturedClip {
        id: 1,
        samples,
        source: ClipSourceKind::Catch,
        captured_at: chrono::Local::now(),
    })
}

fn frame_to_secs(frame: usize) -> f64 {
    frame as f64 / f64::from(SAMPLE_RATE)
}

fn run() -> Result<(), AppError> {
    let clip = build_sine_clip();
    let start_frame = SAMPLE_RATE as usize * RANGE_START_SECS;
    let end_frame = SAMPLE_RATE as usize * RANGE_END_SECS;

    println!(
        "クリップ作成: {SAMPLE_RATE} Hz / {CHANNELS} ch / {:.1} 秒 (440 Hz, {TONE_DBFS:.1} dBFS)",
        clip.duration_secs(),
    );
    println!(
        "再生範囲: {:.3}〜{:.3} 秒 ({start_frame}..{end_frame} フレーム)",
        frame_to_secs(start_frame),
        frame_to_secs(end_frame),
    );

    let mut player = Player::new();

    println!("\n--- 1. ループありで {LOOP_PLAY_SECONDS} 秒再生して停止 ---");
    player.play(Arc::clone(&clip), start_frame, end_frame, true)?;
    println!(
        "t=0s: is_playing={}, position_frame={} ({:.3}s)",
        player.is_playing(),
        player.position_frame(),
        frame_to_secs(player.position_frame())
    );

    for sec in 1..=LOOP_PLAY_SECONDS {
        thread::sleep(Duration::from_secs(1));
        let pos = player.position_frame();
        println!(
            "t={sec}s: is_playing={}, position_frame={pos} ({:.3}s)",
            player.is_playing(),
            frame_to_secs(pos)
        );
    }

    player.stop();
    println!(
        "stop() 後: is_playing={}, position_frame={} ({:.3}s)",
        player.is_playing(),
        player.position_frame(),
        frame_to_secs(player.position_frame())
    );

    thread::sleep(Duration::from_millis(500));

    println!("\n--- 2. ループなしで 1 回再生して自動停止を確認 ---");
    let started = Instant::now();
    player.play(Arc::clone(&clip), start_frame, end_frame, false)?;
    println!(
        "t=0.00s: is_playing={}, position_frame={} ({:.3}s)",
        player.is_playing(),
        player.position_frame(),
        frame_to_secs(player.position_frame())
    );

    let timeout = Duration::from_secs(3);
    while player.is_playing() && started.elapsed() < timeout {
        thread::sleep(Duration::from_millis(200));
        let pos = player.position_frame();
        println!(
            "t={:.2}s: is_playing={}, position_frame={pos} ({:.3}s)",
            started.elapsed().as_secs_f64(),
            player.is_playing(),
            frame_to_secs(pos)
        );
    }

    if player.is_playing() {
        player.stop();
        return Err(AppError::Playback(
            "ループなし再生が制限時間内に自動停止しませんでした".to_string(),
        ));
    }

    // 出力バッファの残りが鳴り切るまで少し待ってから出力を閉じる。
    thread::sleep(Duration::from_millis(150));
    player.stop();
    println!(
        "自動停止確認後 stop(): is_playing={}, position_frame={} ({:.3}s)",
        player.is_playing(),
        player.position_frame(),
        frame_to_secs(player.position_frame())
    );

    Ok(())
}
