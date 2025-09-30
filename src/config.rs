use std::env;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const CONFIG_DIR_ENV: &str = "NEKOMATA_CONFIG_DIR";
const LEGACY_CONFIG_DIR_ENV: &str = "IINACT_TUI_CONFIG_DIR";
const CONFIG_DIR_NAME: &str = "nekomata";
const LEGACY_CONFIG_DIR_NAME: &str = "iinact-tui";
const CONFIG_FILE_NAME: &str = "nekomata.config";
const LEGACY_CONFIG_FILE_NAME: &str = "iinact-tui.config";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_idle_seconds")]
    pub idle_seconds: u64,
    #[serde(default = "default_decoration")]
    pub default_decoration: String,
    #[serde(default = "default_mode")]
    pub default_mode: String,
    #[serde(default = "default_dungeon_mode_enabled")]
    pub dungeon_mode_enabled: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            idle_seconds: default_idle_seconds(),
            default_decoration: default_decoration(),
            default_mode: default_mode(),
            dungeon_mode_enabled: default_dungeon_mode_enabled(),
        }
    }
}

fn default_idle_seconds() -> u64 {
    5
}

fn default_decoration() -> String {
    "underline".to_string()
}

fn default_mode() -> String {
    "dps".to_string()
}

fn default_dungeon_mode_enabled() -> bool {
    true
}

pub fn load() -> Result<AppConfig> {
    let path = config_path();
    match fs::read(&path) {
        Ok(bytes) => {
            let cfg: AppConfig = serde_json::from_slice(&bytes)
                .with_context(|| format!("Failed to parse config at {}", path.display()))?;
            Ok(cfg)
        }
        Err(err) if err.kind() == ErrorKind::NotFound => {
            if let Some(legacy_path) = legacy_config_path() {
                match fs::read(&legacy_path) {
                    Ok(bytes) => {
                        eprintln!(
                            "Loaded legacy config from {}. It will be rewritten to the new location on save.",
                            legacy_path.display()
                        );
                        let cfg: AppConfig = serde_json::from_slice(&bytes).with_context(|| {
                            format!("Failed to parse legacy config at {}", legacy_path.display())
                        })?;
                        Ok(cfg)
                    }
                    Err(err) => Err(err).with_context(|| {
                        format!(
                            "Failed to read legacy config at {} while falling back",
                            legacy_path.display()
                        )
                    }),
                }
            } else {
                Ok(AppConfig::default())
            }
        }
        Err(err) => {
            Err(err).with_context(|| format!("Failed to read config at {}", path.display()))
        }
    }
}

pub fn save(cfg: &AppConfig) -> Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Unable to create config directory {}", parent.display()))?;
    }
    let data = serde_json::to_vec_pretty(cfg)?;
    fs::write(&path, data)
        .with_context(|| format!("Failed to write config to {}", path.display()))?;
    Ok(())
}

pub fn config_path() -> PathBuf {
    config_dir().join(CONFIG_FILE_NAME)
}

pub fn config_dir() -> PathBuf {
    if let Some(path) = env::var_os(CONFIG_DIR_ENV) {
        PathBuf::from(path)
    } else if let Some(path) = env::var_os(LEGACY_CONFIG_DIR_ENV) {
        PathBuf::from(path)
    } else if let Some(path) = env::var_os("XDG_CONFIG_HOME") {
        PathBuf::from(path).join(CONFIG_DIR_NAME)
    } else if let Some(home) = env::var_os("HOME") {
        Path::new(&home).join(".config").join(CONFIG_DIR_NAME)
    } else if let Some(appdata) = env::var_os("APPDATA") {
        PathBuf::from(appdata).join(CONFIG_DIR_NAME)
    } else {
        PathBuf::from(".")
    }
}

pub fn history_dir() -> PathBuf {
    config_dir().join("history")
}

pub fn history_db_path() -> PathBuf {
    let primary = history_dir().join("encounters.sled");
    if primary.exists() {
        return primary;
    }

    if let Some(legacy_dir) = legacy_config_dir() {
        let legacy = legacy_dir.join("history").join("encounters.sled");
        if legacy.exists() {
            return legacy;
        }
    }

    primary
}

fn legacy_config_dir() -> Option<PathBuf> {
    if let Some(path) = env::var_os(LEGACY_CONFIG_DIR_ENV) {
        Some(PathBuf::from(path))
    } else if let Some(path) = env::var_os("XDG_CONFIG_HOME") {
        Some(PathBuf::from(path).join(LEGACY_CONFIG_DIR_NAME))
    } else if let Some(home) = env::var_os("HOME") {
        Some(
            Path::new(&home)
                .join(".config")
                .join(LEGACY_CONFIG_DIR_NAME),
        )
    } else if let Some(appdata) = env::var_os("APPDATA") {
        Some(PathBuf::from(appdata).join(LEGACY_CONFIG_DIR_NAME))
    } else {
        None
    }
}

fn legacy_config_path() -> Option<PathBuf> {
    let path = legacy_config_dir()?.join(LEGACY_CONFIG_FILE_NAME);
    if path.exists() {
        Some(path)
    } else {
        None
    }
}
