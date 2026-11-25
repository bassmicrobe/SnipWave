use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use directories::{ProjectDirs, UserDirs};

use crate::audio::{DEFAULT_BUFFER_SECONDS, MAX_BUFFER_SECONDS, MIN_BUFFER_SECONDS};
use crate::error::AppError;
use crate::export::ExportFormat;

const DEFAULT_HOTKEY_CATCH: &str = "Ctrl+Alt+C";
const DEFAULT_HOTKEY_REC: &str = "Ctrl+Alt+R";

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
        default_config_with_output_dir(resolved_default_output_dir().unwrap_or_default())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadedConfig {
    pub config: AppConfig,
    pub warnings: Vec<String>,
}

impl AppConfig {
    pub fn config_path() -> Result<PathBuf, AppError> {
        let project_dirs =
            ProjectDirs::from("com", "snipwave", "SnipWave").ok_or_else(|| AppError::Config {
                path: PathBuf::from("config.toml"),
                message: "設定フォルダの場所を特定できません".to_string(),
            })?;
        Ok(project_dirs.config_dir().join("config.toml"))
    }

    pub fn load() -> Result<LoadedConfig, AppError> {
        let path = Self::config_path()?;
        Self::load_from(&path)
    }

    pub fn load_from(path: &Path) -> Result<LoadedConfig, AppError> {
        let content = match fs::read_to_string(path) {
            Ok(content) => content,
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                let config = default_config_for(path)?;
                return Ok(LoadedConfig {
                    config,
                    warnings: Vec::new(),
                });
            }
            Err(err) => {
                return recover_from_corrupt_file(
                    path,
                    &format!("設定ファイルを読み込めませんでした: {err}"),
                );
            }
        };

        let mut config: AppConfig = match toml::from_str(&content) {
            Ok(config) => config,
            Err(err) => {
                return recover_from_corrupt_file(
                    path,
                    &format!("設定ファイルの形式が正しくありません: {err}"),
                );
            }
        };

        let mut warnings = Vec::new();
        if config.output_dir.as_os_str().is_empty() {
            config.output_dir = default_output_dir(path)?;
            warnings
                .push("出力先フォルダが未設定だったため、既定のフォルダに戻しました".to_string());
        }

        let (config, validation_warnings) = config.validated();
        warnings.extend(validation_warnings);

        Ok(LoadedConfig { config, warnings })
    }

    pub fn save(&self) -> Result<(), AppError> {
        let path = Self::config_path()?;
        self.save_to(&path)
    }

    pub fn save_to(&self, path: &Path) -> Result<(), AppError> {
        let serialized = toml::to_string_pretty(self).map_err(|err| AppError::Config {
            path: path.to_path_buf(),
            message: format!("設定を TOML に変換できませんでした: {err}"),
        })?;

        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(parent).map_err(|err| AppError::Config {
                path: parent.to_path_buf(),
                message: format!("設定フォルダを作成できませんでした: {err}"),
            })?;
        }

        let tmp_path = temp_path_for(path);
        if let Err(err) = write_temp_file(&tmp_path, &serialized) {
            cleanup_temp_file(&tmp_path);
            return Err(AppError::Config {
                path: path.to_path_buf(),
                message: format!("一時ファイルへの書き込みに失敗しました: {err}"),
            });
        }

        if let Err(err) = fs::rename(&tmp_path, path) {
            cleanup_temp_file(&tmp_path);
            return Err(AppError::Config {
                path: path.to_path_buf(),
                message: format!("設定ファイルの置き換えに失敗しました: {err}"),
            });
        }

        Ok(())
    }

    pub fn validated(mut self) -> (AppConfig, Vec<String>) {
        let mut warnings = Vec::new();
        let clamped = self
            .buffer_seconds
            .clamp(MIN_BUFFER_SECONDS, MAX_BUFFER_SECONDS);
        if clamped != self.buffer_seconds {
            warnings.push(format!(
                "遡り秒数（{} 秒）が範囲外のため、{clamped} 秒に調整しました（有効範囲: {MIN_BUFFER_SECONDS}〜{MAX_BUFFER_SECONDS} 秒）",
                self.buffer_seconds
            ));
            self.buffer_seconds = clamped;
        }
        (self, warnings)
    }
}

fn resolved_default_output_dir() -> Option<PathBuf> {
    let user_dirs = UserDirs::new()?;
    Some(
        user_dirs
            .audio_dir()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| user_dirs.home_dir().to_path_buf()),
    )
}

fn default_output_dir(config_path: &Path) -> Result<PathBuf, AppError> {
    resolved_default_output_dir().ok_or_else(|| AppError::Config {
        path: config_path.to_path_buf(),
        message: "既定の出力先フォルダ（ミュージックまたはホームフォルダ）を特定できません"
            .to_string(),
    })
}

fn default_config_with_output_dir(output_dir: PathBuf) -> AppConfig {
    AppConfig {
        buffer_seconds: DEFAULT_BUFFER_SECONDS,
        output_dir,
        export_format: ExportFormat::Int24,
        hotkey_catch: DEFAULT_HOTKEY_CATCH.to_string(),
        hotkey_rec: DEFAULT_HOTKEY_REC.to_string(),
    }
}

fn default_config_for(config_path: &Path) -> Result<AppConfig, AppError> {
    Ok(default_config_with_output_dir(default_output_dir(
        config_path,
    )?))
}

fn bad_backup_path(path: &Path) -> PathBuf {
    let mut os = path.as_os_str().to_os_string();
    os.push(".bad");
    PathBuf::from(os)
}

fn recover_from_corrupt_file(path: &Path, reason: &str) -> Result<LoadedConfig, AppError> {
    let bad_path = bad_backup_path(path);
    let warning = match fs::rename(path, &bad_path) {
        Ok(()) => {
            log::warn!(
                "壊れた設定ファイルを {} へ退避し、既定値で起動します: {reason}",
                bad_path.display()
            );
            format!(
                "設定ファイルを読み込めなかったため、{} に退避して既定値で起動しました（{reason}）",
                bad_path.display()
            )
        }
        Err(rename_err) => {
            log::warn!(
                "壊れた設定ファイル {} の退避に失敗しました（{rename_err}）。既定値で起動します: {reason}",
                path.display()
            );
            format!(
                "設定ファイルを読み込めず、{} への退避も失敗したため既定値で起動しました（読み込み: {reason} / 退避: {rename_err}）",
                bad_path.display()
            )
        }
    };
    let config = default_config_for(path)?;
    Ok(LoadedConfig {
        config,
        warnings: vec![warning],
    })
}

fn temp_path_for(path: &Path) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let mut os = path.as_os_str().to_os_string();
    os.push(format!(".tmp.{pid}.{seq}"));
    PathBuf::from(os)
}

fn write_temp_file(tmp_path: &Path, content: &str) -> io::Result<()> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(tmp_path)?;
    file.write_all(content.as_bytes())?;
    file.sync_all()?;
    Ok(())
}

fn cleanup_temp_file(tmp_path: &Path) {
    match fs::remove_file(tmp_path) {
        Ok(()) => {}
        Err(err) if err.kind() == io::ErrorKind::NotFound => {}
        Err(err) => {
            log::warn!(
                "設定の一時ファイル {} を削除できませんでした: {err}",
                tmp_path.display()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_to_and_load_from_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("nested").join("config.toml");
        let expected = AppConfig {
            buffer_seconds: 120,
            output_dir: dir.path().join("exports"),
            export_format: ExportFormat::Float32,
            hotkey_catch: "Ctrl+Shift+C".to_string(),
            hotkey_rec: String::new(),
        };

        expected.save_to(&config_path).unwrap();
        let loaded = AppConfig::load_from(&config_path).unwrap();

        assert_eq!(loaded.config, expected);
        assert!(loaded.warnings.is_empty());
    }

    #[test]
    fn validated_clamps_out_of_range_buffer_seconds_with_warning() {
        let too_small = AppConfig {
            buffer_seconds: 0,
            output_dir: PathBuf::from("exports"),
            ..AppConfig::default()
        };
        let (clamped_low, warnings_low) = too_small.validated();
        assert_eq!(clamped_low.buffer_seconds, MIN_BUFFER_SECONDS);
        assert_eq!(warnings_low.len(), 1);

        let too_large = AppConfig {
            buffer_seconds: MAX_BUFFER_SECONDS + 100,
            output_dir: PathBuf::from("exports"),
            ..AppConfig::default()
        };
        let (clamped_high, warnings_high) = too_large.validated();
        assert_eq!(clamped_high.buffer_seconds, MAX_BUFFER_SECONDS);
        assert_eq!(warnings_high.len(), 1);

        let valid = AppConfig {
            buffer_seconds: DEFAULT_BUFFER_SECONDS,
            output_dir: PathBuf::from("exports"),
            ..AppConfig::default()
        };
        let (unchanged, warnings_valid) = valid.validated();
        assert_eq!(unchanged.buffer_seconds, DEFAULT_BUFFER_SECONDS);
        assert!(warnings_valid.is_empty());
    }

    #[test]
    fn load_from_backs_up_corrupt_file_and_returns_defaults_with_warning() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        let broken_content = "buffer_seconds = \"not_a_number\"\n[broken";
        fs::write(&config_path, broken_content).unwrap();

        let loaded = AppConfig::load_from(&config_path).unwrap();
        let bad_path = dir.path().join("config.toml.bad");

        assert!(!config_path.exists());
        assert!(bad_path.exists());
        assert_eq!(fs::read_to_string(&bad_path).unwrap(), broken_content);
        assert_eq!(loaded.config, AppConfig::default());
        assert!(!loaded.warnings.is_empty());
    }
}
