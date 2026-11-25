use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

pub const MAX_BUFFER_SECONDS: u32 = 300; // 上限5分

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppConfig {
    pub buffer_seconds: u32,
    pub output_dir: PathBuf,
}

impl Default for AppConfig {
    fn default() -> Self {
        let output_dir = directories::UserDirs::new()
            .and_then(|d| d.audio_dir().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."));

        Self {
            buffer_seconds: 30, // デフォルト30秒
            output_dir,
        }
    }
}

impl AppConfig {
    fn config_path() -> Option<PathBuf> {
        ProjectDirs::from("com", "snipwave", "SnipWave")
            .map(|proj| proj.config_dir().join("config.toml"))
    }

    pub fn load() -> Self {
        if let Some(path) = Self::config_path() {
            if let Ok(content) = fs::read_to_string(path) {
                if let Ok(cfg) = toml::from_str(&content) {
                    return cfg;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) {
        if let Some(path) = Self::config_path() {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if let Ok(toml_str) = toml::to_string_pretty(self) {
                let _ = fs::write(path, toml_str);
            }
        }
    }
}
