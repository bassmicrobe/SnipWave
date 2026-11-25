use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::audio::clip::CapturedClip;
use crate::error::AppError;

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
    Err(AppError::Export {
        path: req.output_dir.clone(),
        message: "未実装".to_string(),
    })
}

/// `snip_YYYYMMDD_HHMMSS.wav`。同じ名前があれば `snip_YYYYMMDD_HHMMSS_2.wav`、`_3` … とする。
pub fn unique_output_path(dir: &Path, now: chrono::DateTime<chrono::Local>) -> PathBuf {
    dir.join(format!("snip_{}.wav", now.format("%Y%m%d_%H%M%S")))
}

/// f32 → 24-bit。[-1.0, 1.0] に切り詰めてから 8_388_607 を掛けて四捨五入する。ディザはかけない。
pub fn f32_to_i24(_sample: f32) -> i32 {
    0
}
