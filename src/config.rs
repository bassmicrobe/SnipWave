use std::path::{Path, PathBuf};

use crate::audio::DEFAULT_BUFFER_SECONDS;
use crate::error::AppError;
use crate::export::ExportFormat;

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub buffer_seconds: u32,
    pub output_dir: PathBuf,
    pub export_format: ExportFormat,
    pub hotkey_catch: String,
    pub hotkey_rec: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            buffer_seconds: DEFAULT_BUFFER_SECONDS,
            output_dir: PathBuf::new(),
            export_format: ExportFormat::Int24,
            hotkey_catch: "Ctrl+Alt+C".to_string(),
            hotkey_rec: "Ctrl+Alt+R".to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadedConfig {
    pub config: AppConfig,
    pub warnings: Vec<String>,
}

impl AppConfig {
    pub fn config_path() -> Result<PathBuf, AppError> {
        Ok(PathBuf::from("config.toml"))
    }

    pub fn load() -> Result<LoadedConfig, AppError> {
        Ok(LoadedConfig {
            config: Self::default(),
            warnings: Vec::new(),
        })
    }

    pub fn load_from(_path: &Path) -> Result<LoadedConfig, AppError> {
        Ok(LoadedConfig {
            config: Self::default(),
            warnings: Vec::new(),
        })
    }

    pub fn save(&self) -> Result<(), AppError> {
        Ok(())
    }

    pub fn save_to(&self, _path: &Path) -> Result<(), AppError> {
        Ok(())
    }

    pub fn validated(self) -> (AppConfig, Vec<String>) {
        (self, Vec::new())
    }
}
