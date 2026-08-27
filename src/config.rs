use std::env;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const CONFIG_DIR_ENV: &str = "NEKOMATA_CONFIG_DIR";
const CONFIG_DIR_NAME: &str = "nekomata";
const CONFIG_FILE_NAME: &str = "nekomata.config";
const HISTORY_ARCHIVES_DIR: &str = "archives";

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
    #[serde(default = "default_theme_id")]
    pub theme_id: String,
    #[serde(default = "default_role_theme_enabled")]
    pub role_theme_enabled: bool,
    /// 0=Off, 1=PanelAlways, 2=TableRow
    /// Kept as `Option` to allow migrating from older configs that had `show_limit_break`.
    #[serde(default)]
    pub limit_break_mode: Option<u8>,
    /// Legacy boolean config key. If present and `limit_break_mode` is missing, it maps to:
    /// - `false` => Off (0)
    /// - `true` => PanelAlways (1)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_limit_break: Option<bool>,
    #[serde(default = "default_history_enabled")]
    pub history_enabled: bool,
    #[serde(default = "default_history_limit_kind")]
    pub history_limit_kind: String,
    #[serde(default = "default_history_limit_days")]
    pub history_limit_days: u64,
    #[serde(default = "default_history_limit_mb")]
    pub history_limit_mb: u64,
    #[serde(default = "default_history_limit_kind")]
    pub history_limit_applied_kind: String,
    #[serde(default = "default_history_limit_days")]
    pub history_limit_applied_days: u64,
    #[serde(default = "default_history_limit_mb")]
    pub history_limit_applied_mb: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub history_last_backup_ms: Option<u64>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            idle_seconds: default_idle_seconds(),
            default_decoration: default_decoration(),
            default_mode: default_mode(),
            dungeon_mode_enabled: default_dungeon_mode_enabled(),
            theme_id: default_theme_id(),
            role_theme_enabled: default_role_theme_enabled(),
            limit_break_mode: Some(default_limit_break_mode()),
            show_limit_break: None,
            history_enabled: default_history_enabled(),
            history_limit_kind: default_history_limit_kind(),
            history_limit_days: default_history_limit_days(),
            history_limit_mb: default_history_limit_mb(),
            history_limit_applied_kind: default_history_limit_kind(),
            history_limit_applied_days: default_history_limit_days(),
            history_limit_applied_mb: default_history_limit_mb(),
            history_last_backup_ms: None,
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

fn default_theme_id() -> String {
    String::new()
}

fn default_role_theme_enabled() -> bool {
    true
}

fn default_limit_break_mode() -> u8 {
    1
}

fn default_history_enabled() -> bool {
    true
}

fn default_history_limit_kind() -> String {
    "none".to_string()
}

fn default_history_limit_days() -> u64 {
    30
}

fn default_history_limit_mb() -> u64 {
    256
}

pub fn load() -> Result<AppConfig> {
    let path = config_path();
    match fs::read(&path) {
        Ok(bytes) => {
            let cfg: AppConfig = serde_json::from_slice(&bytes)
                .with_context(|| format!("Failed to parse config at {}", path.display()))?;
            Ok(cfg)
        }
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(AppConfig::default()),
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
    history_dir().join("encounters.sled")
}

pub fn history_archives_dir() -> PathBuf {
    history_dir().join(HISTORY_ARCHIVES_DIR)
}

pub fn history_archive_path(name: &str) -> PathBuf {
    history_archives_dir().join(name)
}

/// Sanitize a user-provided archive folder name.
pub fn sanitize_archive_name(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == "." || trimmed == ".." {
        return None;
    }
    if trimmed.contains('/') || trimmed.contains('\\') {
        return None;
    }
    if !trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
    {
        return None;
    }
    Some(trimmed.to_string())
}
