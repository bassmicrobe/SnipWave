//! T1: macOS で cpal 0.18.2 を使ってシステム音声（ループバック）が取れるかを検証するプローブ。
//!
//! 使い方:
//!   cargo build --example loopback_probe
//!   scripts/macos/probe-app.sh build   # Probe.app を組み立てて ad-hoc 署名する
//!   scripts/macos/probe-app.sh run     # Probe.app を起動してログと probe.wav を残す

#[cfg(not(target_os = "macos"))]
fn main() {
    println!("loopback_probe は macOS 専用です。");
}

#[cfg(target_os = "macos")]
fn main() -> std::process::ExitCode {
    match macos::run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("エラー: {err}");
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use std::f64::consts::TAU;
    use std::fs;
    use std::io::ErrorKind;
    use std::path::{Path, PathBuf};
    use std::process::{Child, Command};
    use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use cpal::{FromSample, Sample, SampleFormat, SizedSample, Stream, StreamConfig};

    const CAPTURE_SECONDS: u64 = 10;
    const BUFFER_CAPACITY_SECONDS: usize = 15;
    const TONE_START_SECOND: u64 = 2;
    const TONE_DURATION_SECONDS: u32 = 3;
    const TONE_FREQ_HZ: f64 = 997.0;
    const TONE_DBFS: f64 = -12.0;
    const TONE_SAMPLE_RATE: u32 = 48_000;

    struct Args {
        output: Option<PathBuf>,
        info_only: bool,
    }

    struct CaptureBuffer {
        samples: Box<[AtomicU32]>,
        max_frames: usize,
        written_frames: AtomicUsize,
        dropped_frames: AtomicU64,
        stream_error: AtomicBool,
    }

    impl CaptureBuffer {
        fn new(max_frames: usize) -> Self {
            let zero_bits = 0.0_f32.to_bits();
            let mut vec = Vec::with_capacity(max_frames.saturating_mul(2));
            for _ in 0..max_frames.saturating_mul(2) {
                vec.push(AtomicU32::new(zero_bits));
            }
            Self {
                samples: vec.into_boxed_slice(),
                max_frames,
                written_frames: AtomicUsize::new(0),
                dropped_frames: AtomicU64::new(0),
                stream_error: AtomicBool::new(false),
            }
        }

        fn read_frame(&self, frame_index: usize) -> (f32, f32) {
            let base = frame_index * 2;
            let left = f32::from_bits(self.samples[base].load(Ordering::Relaxed));
            let right = f32::from_bits(self.samples[base + 1].load(Ordering::Relaxed));
            (left, right)
        }
    }

    struct TempFileGuard(PathBuf);

    impl Drop for TempFileGuard {
        fn drop(&mut self) {
            if let Err(err) = fs::remove_file(&self.0) {
                if err.kind() != ErrorKind::NotFound {
                    eprintln!(
                        "警告: 一時ファイル {} の削除に失敗しました: {err}",
                        self.0.display()
                    );
                }
            }
        }
    }

    struct WindowMetrics {
        frames: usize,
        peak_l: f32,
        peak_r: f32,
        peak_max: f32,
        rms: f64,
        amp_997: f64,
        ratio_997: f64,
    }

    pub fn run() -> Result<(), String> {
        let args = parse_args()?;
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| "既定の出力デバイスが見つかりません".to_string())?;

        let device_name = device.to_string();
        let device_id = device
            .id()
            .map(|id| id.to_string())
            .unwrap_or_else(|e| format!("<取得失敗: {e}>"));
        let supports_input = device.supports_input();
        let supports_output = device.supports_output();

        println!("=== loopback_probe (T1) ===");
        println!("default_output_device: {device_name}");
        println!("device_id: {device_id}");
        println!("supports_input: {supports_input}");
        println!("supports_output: {supports_output}");

        let default_output_cfg = device
            .default_output_config()
            .map_err(|e| format!("default_output_config の取得に失敗しました: {e}"))?;
        println!(
            "default_output_config (before): rate={} Hz, channels={}, sample_format={:?}, buffer_size={:?}",
            default_output_cfg.sample_rate(),
            default_output_cfg.channels(),
            default_output_cfg.sample_format(),
            default_output_cfg.buffer_size()
        );

        let supported_config = if supports_input {
            println!(
                "警告: 既定の出力デバイスが supports_input() == true です。cpal はループバックではなくマイク側を録るため、T1 の判定には出力専用デバイスへ切り替えてください。"
            );
            let default_input_cfg = device
                .default_input_config()
                .map_err(|e| format!("default_input_config の取得に失敗しました: {e}"))?;
            println!(
                "default_input_config: rate={} Hz, channels={}, sample_format={:?}, buffer_size={:?}",
                default_input_cfg.sample_rate(),
                default_input_cfg.channels(),
                default_input_cfg.sample_format(),
                default_input_cfg.buffer_size()
            );
            default_input_cfg
        } else {
            default_output_cfg
        };

        if args.info_only {
            println!("--info-only のため、ストリームを開かずに終了します。");
            return Ok(());
        }

        let output_wav_path = resolve_output_path(args.output)?;
        println!("output_wav_path: {}", output_wav_path.display());

        let sample_rate = supported_config.sample_rate();
        let input_channels = usize::from(supported_config.channels());
        let sample_format = supported_config.sample_format();
        let stream_config: StreamConfig = supported_config.into();

        println!(
            "opening input stream with device default config: rate={} Hz, channels={}, sample_format={:?}",
            stream_config.sample_rate, stream_config.channels, sample_format
        );

        let tone_path =
            std::env::temp_dir().join(format!("snipwave_t1_tone_{}.wav", std::process::id()));
        write_test_tone_wav(&tone_path)?;
        let _tone_guard = TempFileGuard(tone_path.clone());
        println!(
            "test_tone_wav: {} ({} Hz, {} dBFS, {} s)",
            tone_path.display(),
            TONE_FREQ_HZ,
            TONE_DBFS,
            TONE_DURATION_SECONDS
        );

        let capacity_frames = (sample_rate as usize).saturating_mul(BUFFER_CAPACITY_SECONDS);
        let capture = Arc::new(CaptureBuffer::new(capacity_frames));

        let stream = build_capture_stream(
            &device,
            stream_config,
            sample_format,
            input_channels,
            Arc::clone(&capture),
        )?;
        stream
            .play()
            .map_err(|e| format!("ストリームの開始に失敗しました: {e}"))?;

        let start = Instant::now();
        let mut frame_marks = [0usize; (CAPTURE_SECONDS as usize) + 1];
        frame_marks[0] = capture.written_frames.load(Ordering::Acquire);
        let mut prev_frames = frame_marks[0];
        let mut afplay_child: Option<Child> = None;

        for sec in 1..=CAPTURE_SECONDS {
            let target = start + Duration::from_secs(sec);
            let now = Instant::now();
            if let Some(wait) = target.checked_duration_since(now) {
                std::thread::sleep(wait);
            }

            let current_frames = capture.written_frames.load(Ordering::Acquire);
            let delta = current_frames.saturating_sub(prev_frames);
            let (peak_l, peak_r) = peak_range(&capture, prev_frames, current_frames);
            println!(
                "[{sec:>2}s] frames={delta:>6} total_frames={current_frames:>7} peak_L={peak_l:.6} peak_R={peak_r:.6}"
            );

            frame_marks[sec as usize] = current_frames;
            prev_frames = current_frames;

            if sec == TONE_START_SECOND {
                println!(
                    "試験音を再生します: /usr/bin/afplay {}",
                    tone_path.display()
                );
                let child = Command::new("/usr/bin/afplay")
                    .arg(&tone_path)
                    .spawn()
                    .map_err(|e| format!("afplay の起動に失敗しました: {e}"))?;
                afplay_child = Some(child);
            }
        }

        drop(stream);

        if let Some(mut child) = afplay_child {
            let status = child
                .wait()
                .map_err(|e| format!("afplay の終了待ちに失敗しました: {e}"))?;
            println!("afplay_exit_status: {status}");
            if !status.success() {
                return Err(format!("afplay が異常終了しました: {status}"));
            }
        }

        let total_frames = capture.written_frames.load(Ordering::Acquire);
        let dropped_frames = capture.dropped_frames.load(Ordering::Relaxed);
        let stream_error = capture.stream_error.load(Ordering::Acquire);
        println!("total_frames: {total_frames}");
        println!("dropped_frames: {dropped_frames}");
        println!("stream_error: {stream_error}");

        match device.default_output_config() {
            Ok(after_cfg) => {
                println!(
                    "default_output_config (after): rate={} Hz, channels={}, sample_format={:?}, buffer_size={:?}",
                    after_cfg.sample_rate(),
                    after_cfg.channels(),
                    after_cfg.sample_format(),
                    after_cfg.buffer_size()
                );
            }
            Err(e) => {
                println!("default_output_config (after): 取得失敗 ({e})");
            }
        }

        write_probe_wav(&output_wav_path, &capture, total_frames, sample_rate)?;
        println!("wrote_wav: {}", output_wav_path.display());

        let margin_02 = (sample_rate as usize) / 5;
        let margin_05 = (sample_rate as usize) / 2;

        let pre_raw_start = frame_marks[0];
        let pre_raw_end = frame_marks[TONE_START_SECOND as usize].min(total_frames);
        let (pre_start, pre_end) = shrink_window(pre_raw_start, pre_raw_end, margin_02, margin_02);

        let tone_end_sec = (TONE_START_SECOND as usize) + (TONE_DURATION_SECONDS as usize);
        let tone_raw_start = frame_marks[TONE_START_SECOND as usize].min(total_frames);
        let tone_raw_end = frame_marks[tone_end_sec].min(total_frames);
        let (tone_start, tone_end) =
            shrink_window(tone_raw_start, tone_raw_end, margin_05, margin_02);

        let pre_metrics = analyze_window(&capture, pre_start, pre_end, sample_rate, TONE_FREQ_HZ);
        let tone_metrics =
            analyze_window(&capture, tone_start, tone_end, sample_rate, TONE_FREQ_HZ);

        println!(
            "pre_tone_window ({pre_start}..{pre_end}): frames={} peak_L={:.6} peak_R={:.6} peak_max={:.6} rms={:.6} amp_997={:.6} ratio_997={:.6}",
            pre_metrics.frames,
            pre_metrics.peak_l,
            pre_metrics.peak_r,
            pre_metrics.peak_max,
            pre_metrics.rms,
            pre_metrics.amp_997,
            pre_metrics.ratio_997
        );
        println!(
            "tone_window ({tone_start}..{tone_end}): frames={} peak_L={:.6} peak_R={:.6} peak_max={:.6} rms={:.6} amp_997={:.6} ratio_997={:.6}",
            tone_metrics.frames,
            tone_metrics.peak_l,
            tone_metrics.peak_r,
            tone_metrics.peak_max,
            tone_metrics.rms,
            tone_metrics.amp_997,
            tone_metrics.ratio_997
        );
        println!(
            "check: tone_peak_gt_zero={} tone_ratio_ge_0_7={}",
            tone_metrics.peak_max > 0.0,
            tone_metrics.ratio_997 >= 0.7
        );

        Ok(())
    }

    fn parse_args() -> Result<Args, String> {
        let mut output = None;
        let mut info_only = false;
        let mut iter = std::env::args().skip(1);

        while let Some(arg) = iter.next() {
            if arg == "--info-only" {
                info_only = true;
            } else if arg == "--output" {
                let value = iter
                    .next()
                    .ok_or_else(|| "--output の後にパスを指定してください".to_string())?;
                output = Some(PathBuf::from(value));
            } else if let Some(value) = arg.strip_prefix("--output=") {
                output = Some(PathBuf::from(value));
            } else if arg.starts_with("-psn_") {
                // macOS LaunchServices が渡すプロセスシリアル番号は無視する
            } else {
                return Err(format!("不明な引数です: {arg}"));
            }
        }

        Ok(Args { output, info_only })
    }

    fn resolve_output_path(explicit: Option<PathBuf>) -> Result<PathBuf, String> {
        let path = match explicit {
            Some(p) => p,
            None => default_output_path()?,
        };
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                format!(
                    "出力先フォルダ {} の作成に失敗しました: {e}",
                    parent.display()
                )
            })?;
        }
        Ok(path)
    }

    fn default_output_path() -> Result<PathBuf, String> {
        let exe = std::env::current_exe()
            .map_err(|e| format!("実行ファイルパスの取得に失敗しました: {e}"))?;

        for ancestor in exe.ancestors() {
            if ancestor
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext == "app")
            {
                if let Some(parent) = ancestor.parent() {
                    return Ok(parent.join("loopback").join("probe.wav"));
                }
            }
        }

        // target/<profile>/examples/loopback_probe -> target/probe/loopback/probe.wav
        if let Some(target_dir) = exe.parent().and_then(Path::parent).and_then(Path::parent) {
            if target_dir.file_name().and_then(|n| n.to_str()) == Some("target") {
                return Ok(target_dir.join("probe").join("loopback").join("probe.wav"));
            }
        }

        Ok(std::env::temp_dir().join("snipwave_loopback_probe.wav"))
    }

    fn write_test_tone_wav(path: &Path) -> Result<(), String> {
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: TONE_SAMPLE_RATE,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(path, spec)
            .map_err(|e| format!("試験音 WAV {} の作成に失敗しました: {e}", path.display()))?;

        let total_frames = (TONE_SAMPLE_RATE as usize) * (TONE_DURATION_SECONDS as usize);
        let fade_frames = (TONE_SAMPLE_RATE as usize) / 200; // 5 ms
        let amplitude = 10.0_f64.powf(TONE_DBFS / 20.0);

        for i in 0..total_frames {
            let phase = TAU * TONE_FREQ_HZ * (i as f64) / f64::from(TONE_SAMPLE_RATE);
            let env = if i < fade_frames {
                i as f64 / fade_frames as f64
            } else if i + fade_frames >= total_frames {
                (total_frames - 1 - i) as f64 / fade_frames as f64
            } else {
                1.0
            };
            let sample_f64 = phase.sin() * amplitude * env;
            let sample_i16 = (sample_f64 * f64::from(i16::MAX))
                .round()
                .clamp(f64::from(i16::MIN), f64::from(i16::MAX))
                as i16;
            writer
                .write_sample(sample_i16)
                .map_err(|e| format!("試験音サンプルの書き込みに失敗しました: {e}"))?;
            writer
                .write_sample(sample_i16)
                .map_err(|e| format!("試験音サンプルの書き込みに失敗しました: {e}"))?;
        }

        writer
            .finalize()
            .map_err(|e| format!("試験音 WAV の確定に失敗しました: {e}"))?;
        Ok(())
    }

    fn build_capture_stream(
        device: &cpal::Device,
        config: StreamConfig,
        sample_format: SampleFormat,
        input_channels: usize,
        capture: Arc<CaptureBuffer>,
    ) -> Result<Stream, String> {
        match sample_format {
            SampleFormat::F32 => build_typed_stream::<f32>(device, config, input_channels, capture),
            SampleFormat::I16 => build_typed_stream::<i16>(device, config, input_channels, capture),
            SampleFormat::I32 => build_typed_stream::<i32>(device, config, input_channels, capture),
            SampleFormat::U16 => build_typed_stream::<u16>(device, config, input_channels, capture),
            other => Err(format!("未対応のサンプル形式です: {other:?}")),
        }
    }

    fn build_typed_stream<T>(
        device: &cpal::Device,
        config: StreamConfig,
        input_channels: usize,
        capture: Arc<CaptureBuffer>,
    ) -> Result<Stream, String>
    where
        T: SizedSample,
        f32: FromSample<T>,
    {
        let data_capture = Arc::clone(&capture);
        let err_capture = capture;

        device
            .build_input_stream(
                config,
                move |data: &[T], _: &cpal::InputCallbackInfo| {
                    if input_channels == 0 {
                        return;
                    }
                    let mut frame_pos = data_capture.written_frames.load(Ordering::Relaxed);
                    let max_frames = data_capture.max_frames;
                    let mut dropped = 0u64;

                    for frame in data.chunks_exact(input_channels) {
                        let left = f32::from_sample(frame[0]);
                        let right = if input_channels >= 2 {
                            f32::from_sample(frame[1])
                        } else {
                            left
                        };
                        if frame_pos < max_frames {
                            let base = frame_pos * 2;
                            data_capture.samples[base].store(left.to_bits(), Ordering::Relaxed);
                            data_capture.samples[base + 1]
                                .store(right.to_bits(), Ordering::Relaxed);
                            frame_pos += 1;
                        } else {
                            dropped += 1;
                        }
                    }

                    data_capture
                        .written_frames
                        .store(frame_pos, Ordering::Release);
                    if dropped > 0 {
                        data_capture
                            .dropped_frames
                            .fetch_add(dropped, Ordering::Relaxed);
                    }
                },
                move |_err| {
                    err_capture.stream_error.store(true, Ordering::Release);
                },
                None,
            )
            .map_err(|e| format!("入力ストリームの作成に失敗しました: {e}"))
    }

    fn peak_range(capture: &CaptureBuffer, start: usize, end: usize) -> (f32, f32) {
        let mut peak_l = 0.0_f32;
        let mut peak_r = 0.0_f32;
        for frame in start..end {
            let (l, r) = capture.read_frame(frame);
            peak_l = peak_l.max(l.abs());
            peak_r = peak_r.max(r.abs());
        }
        (peak_l, peak_r)
    }

    fn shrink_window(
        start: usize,
        end: usize,
        lead_margin: usize,
        tail_margin: usize,
    ) -> (usize, usize) {
        let shrunk_start = start.saturating_add(lead_margin);
        let shrunk_end = end.saturating_sub(tail_margin);
        if shrunk_start < shrunk_end {
            (shrunk_start, shrunk_end)
        } else {
            (start, end)
        }
    }

    fn analyze_window(
        capture: &CaptureBuffer,
        start: usize,
        end: usize,
        sample_rate: u32,
        target_hz: f64,
    ) -> WindowMetrics {
        let frames = end.saturating_sub(start);
        if frames == 0 || sample_rate == 0 {
            return WindowMetrics {
                frames: 0,
                peak_l: 0.0,
                peak_r: 0.0,
                peak_max: 0.0,
                rms: 0.0,
                amp_997: 0.0,
                ratio_997: 0.0,
            };
        }

        let omega = TAU * target_hz / f64::from(sample_rate);
        let coeff = 2.0 * omega.cos();
        let mut s1 = 0.0_f64;
        let mut s2 = 0.0_f64;
        let mut sum_sq = 0.0_f64;
        let mut peak_l = 0.0_f32;
        let mut peak_r = 0.0_f32;

        for frame in start..end {
            let (l, r) = capture.read_frame(frame);
            peak_l = peak_l.max(l.abs());
            peak_r = peak_r.max(r.abs());
            let mono = (f64::from(l) + f64::from(r)) * 0.5;
            sum_sq += mono * mono;
            let s0 = mono + coeff * s1 - s2;
            s2 = s1;
            s1 = s0;
        }

        let n = frames as f64;
        let rms = (sum_sq / n).sqrt();
        let power = (s1 * s1 + s2 * s2 - coeff * s1 * s2).max(0.0);
        let amp_997 = (2.0 * power.sqrt()) / n;
        let ratio_997 = if rms > 0.0 {
            amp_997 / (2.0_f64.sqrt() * rms)
        } else {
            0.0
        };

        WindowMetrics {
            frames,
            peak_l,
            peak_r,
            peak_max: peak_l.max(peak_r),
            rms,
            amp_997,
            ratio_997,
        }
    }

    fn write_probe_wav(
        path: &Path,
        capture: &CaptureBuffer,
        total_frames: usize,
        sample_rate: u32,
    ) -> Result<(), String> {
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let mut writer = hound::WavWriter::create(path, spec)
            .map_err(|e| format!("probe.wav ({}) の作成に失敗しました: {e}", path.display()))?;

        for frame in 0..total_frames {
            let (l, r) = capture.read_frame(frame);
            writer
                .write_sample(l)
                .map_err(|e| format!("probe.wav への書き込みに失敗しました: {e}"))?;
            writer
                .write_sample(r)
                .map_err(|e| format!("probe.wav への書き込みに失敗しました: {e}"))?;
        }

        writer
            .finalize()
            .map_err(|e| format!("probe.wav の確定に失敗しました: {e}"))?;
        Ok(())
    }
}
