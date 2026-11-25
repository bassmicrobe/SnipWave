use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::audio::clip::CapturedClip;
use crate::audio::{CHANNELS, SAMPLE_RATE};
use crate::error::AppError;

const I24_MAX_SCALE: f64 = 8_388_607.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Int24,
    Float32,
}

#[derive(Clone, Debug)]
pub struct ExportRequest {
    pub clip: Arc<CapturedClip>,
    pub start_frame: usize,
    pub end_frame: usize,
    pub format: ExportFormat,
    pub output_dir: PathBuf,
    pub now: chrono::DateTime<chrono::Local>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportOutcome {
    pub path: PathBuf,
    pub frames: usize,
    pub clipped_samples: usize,
}

/// 書き出す。出力先が無ければ作る。`<名前>.part` に書いてから名前を変える（途中で失敗したら .part を消す）。
pub fn export_wav(req: &ExportRequest) -> Result<ExportOutcome, AppError> {
    let total_frames = req.clip.frames();
    if req.start_frame >= req.end_frame || req.end_frame > total_frames {
        return Err(AppError::InvalidSelection);
    }

    let frames = req.end_frame - req.start_frame;
    let slice = &req.clip.samples[req.start_frame * CHANNELS..req.end_frame * CHANNELS];

    fs::create_dir_all(&req.output_dir).map_err(|err| AppError::Export {
        path: req.output_dir.clone(),
        message: err.to_string(),
    })?;

    let output_path = unique_output_path(&req.output_dir, req.now);
    let part_path = part_path_for(&output_path);

    let clipped_samples = match write_wav_file(&part_path, slice, req.format) {
        Ok(clipped) => clipped,
        Err(err) => {
            cleanup_part_file(&part_path);
            return Err(AppError::Export {
                path: output_path,
                message: err.to_string(),
            });
        }
    };

    if let Err(err) = fs::rename(&part_path, &output_path) {
        cleanup_part_file(&part_path);
        return Err(AppError::Export {
            path: output_path,
            message: err.to_string(),
        });
    }

    Ok(ExportOutcome {
        path: output_path,
        frames,
        clipped_samples,
    })
}

/// `snip_YYYYMMDD_HHMMSS.wav`。同じ名前があれば `snip_YYYYMMDD_HHMMSS_2.wav`、`_3` … とする。
pub fn unique_output_path(dir: &Path, now: chrono::DateTime<chrono::Local>) -> PathBuf {
    let stem = format!("snip_{}", now.format("%Y%m%d_%H%M%S"));
    let first = dir.join(format!("{stem}.wav"));
    if !first.exists() {
        return first;
    }

    let mut suffix: u64 = 2;
    loop {
        let candidate = dir.join(format!("{stem}_{suffix}.wav"));
        if !candidate.exists() {
            return candidate;
        }
        suffix += 1;
    }
}

/// f32 → 24-bit。[-1.0, 1.0] に切り詰めてから 8_388_607 を掛けて四捨五入する。ディザはかけない。
pub fn f32_to_i24(sample: f32) -> i32 {
    if sample.is_nan() {
        return 0;
    }
    let clamped = f64::from(sample.clamp(-1.0, 1.0));
    (clamped * I24_MAX_SCALE).round() as i32
}

fn wav_spec(format: ExportFormat) -> hound::WavSpec {
    let (bits_per_sample, sample_format) = match format {
        ExportFormat::Int24 => (24, hound::SampleFormat::Int),
        ExportFormat::Float32 => (32, hound::SampleFormat::Float),
    };
    hound::WavSpec {
        channels: CHANNELS as u16,
        sample_rate: SAMPLE_RATE,
        bits_per_sample,
        sample_format,
    }
}

fn part_path_for(path: &Path) -> PathBuf {
    let mut os = path.as_os_str().to_os_string();
    os.push(".part");
    PathBuf::from(os)
}

fn write_wav_file(path: &Path, samples: &[f32], format: ExportFormat) -> hound::Result<usize> {
    let spec = wav_spec(format);
    let file = fs::File::create(path)?;
    let buf_writer = io::BufWriter::new(file);
    let mut writer = hound::WavWriter::new(buf_writer, spec)?;

    let mut clipped_samples = 0_usize;
    match format {
        ExportFormat::Int24 => {
            for &sample in samples {
                if !(-1.0..=1.0).contains(&sample) {
                    clipped_samples += 1;
                }
                writer.write_sample(f32_to_i24(sample))?;
            }
        }
        ExportFormat::Float32 => {
            for &sample in samples {
                writer.write_sample(sample)?;
            }
        }
    }

    writer.finalize()?;
    Ok(clipped_samples)
}

fn cleanup_part_file(part_path: &Path) {
    match fs::remove_file(part_path) {
        Ok(()) => {}
        Err(err) if err.kind() == io::ErrorKind::NotFound => {}
        Err(err) => {
            log::warn!(
                "一時ファイルを削除できませんでした（{}）: {err}",
                part_path.display()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::clip::ClipSourceKind;
    use chrono::{Local, TimeZone};

    fn fixed_local_time() -> chrono::DateTime<Local> {
        Local.with_ymd_and_hms(2026, 11, 25, 14, 30, 5).unwrap()
    }

    #[test]
    fn export_wav_writes_selected_range_float32() {
        let dir = tempfile::tempdir().unwrap();
        let now = fixed_local_time();
        let samples = vec![
            -1.0, -0.75, // frame 0
            -0.5, -0.25, // frame 1
            0.0, 0.125, // frame 2
            0.25, 0.5, // frame 3
            0.75, 1.0, // frame 4
            0.2, -0.2, // frame 5
        ];
        let clip = Arc::new(CapturedClip {
            id: 1,
            samples: samples.clone(),
            source: ClipSourceKind::Catch,
            captured_at: now,
        });

        let start_frame = 1;
        let end_frame = 5;
        let req = ExportRequest {
            clip,
            start_frame,
            end_frame,
            format: ExportFormat::Float32,
            output_dir: dir.path().to_path_buf(),
            now,
        };

        let outcome = export_wav(&req).unwrap();
        assert_eq!(outcome.frames, end_frame - start_frame);
        assert_eq!(outcome.clipped_samples, 0);
        assert_eq!(outcome.path, dir.path().join("snip_20261125_143005.wav"));
        assert!(!part_path_for(&outcome.path).exists());

        let mut reader = hound::WavReader::open(&outcome.path).unwrap();
        assert_eq!(
            reader.spec(),
            hound::WavSpec {
                channels: CHANNELS as u16,
                sample_rate: SAMPLE_RATE,
                bits_per_sample: 32,
                sample_format: hound::SampleFormat::Float,
            }
        );
        assert_eq!(reader.duration(), (end_frame - start_frame) as u32);

        let read_samples: Vec<f32> = reader
            .samples::<f32>()
            .collect::<hound::Result<Vec<f32>>>()
            .unwrap();
        assert_eq!(
            read_samples,
            &samples[start_frame * CHANNELS..end_frame * CHANNELS]
        );
    }

    #[test]
    fn f32_to_i24_clamps_and_scales() {
        assert_eq!(f32_to_i24(0.0), 0);
        assert_eq!(f32_to_i24(1.0), 8_388_607);
        assert_eq!(f32_to_i24(-1.0), -8_388_607);
        assert_eq!(f32_to_i24(1.5), 8_388_607);
        assert_eq!(f32_to_i24(-2.0), -8_388_607);
        assert_eq!(f32_to_i24(f32::NAN), 0);
    }

    #[test]
    fn unique_output_path_formats_and_avoids_collision() {
        let dir = tempfile::tempdir().unwrap();
        let now = fixed_local_time();

        let first = unique_output_path(dir.path(), now);
        assert_eq!(first, dir.path().join("snip_20261125_143005.wav"));

        fs::write(&first, b"").unwrap();
        let second = unique_output_path(dir.path(), now);
        assert_eq!(second, dir.path().join("snip_20261125_143005_2.wav"));

        fs::write(&second, b"").unwrap();
        let third = unique_output_path(dir.path(), now);
        assert_eq!(third, dir.path().join("snip_20261125_143005_3.wav"));
    }
}
