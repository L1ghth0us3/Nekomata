use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::config::AppConfig;

use super::{Decoration, ViewMode};

pub const LIMIT_BREAK_MODE_OFF: u8 = 0;
pub const LIMIT_BREAK_MODE_PANEL: u8 = 1;
pub const LIMIT_BREAK_MODE_TABLE: u8 = 2;

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum SettingsField {
    #[default]
    IdleTimeout,
    DefaultDecoration,
    DefaultMode,
    DungeonMode,
    LimitBreakMode,
    Theme,
    RoleTheme,
}

impl SettingsField {
    pub fn next(self) -> Self {
        match self {
            SettingsField::IdleTimeout => SettingsField::DefaultDecoration,
            SettingsField::DefaultDecoration => SettingsField::DefaultMode,
            SettingsField::DefaultMode => SettingsField::DungeonMode,
            SettingsField::DungeonMode => SettingsField::LimitBreakMode,
            SettingsField::LimitBreakMode => SettingsField::Theme,
            SettingsField::Theme => SettingsField::RoleTheme,
            SettingsField::RoleTheme => SettingsField::IdleTimeout,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            SettingsField::IdleTimeout => SettingsField::RoleTheme,
            SettingsField::DefaultDecoration => SettingsField::IdleTimeout,
            SettingsField::DefaultMode => SettingsField::DefaultDecoration,
            SettingsField::DungeonMode => SettingsField::DefaultMode,
            SettingsField::LimitBreakMode => SettingsField::DungeonMode,
            SettingsField::Theme => SettingsField::LimitBreakMode,
            SettingsField::RoleTheme => SettingsField::Theme,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppSettings {
    pub idle_seconds: u64,
    pub default_decoration: Decoration,
    pub default_mode: ViewMode,
    pub dungeon_mode_enabled: bool,
    pub theme_id: String,
    pub role_theme_enabled: bool,
    /// 0=Off, 1=PanelAlways, 2=TableRow
    pub limit_break_mode: u8,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            idle_seconds: 5,
            default_decoration: Decoration::Underline,
            default_mode: ViewMode::Dps,
            dungeon_mode_enabled: true,
            theme_id: String::new(),
            role_theme_enabled: true,
            limit_break_mode: LIMIT_BREAK_MODE_PANEL,
        }
    }
}

impl AppSettings {
    pub fn idle_duration(&self) -> Option<Duration> {
        if self.idle_seconds == 0 {
            None
        } else {
            Some(Duration::from_secs(self.idle_seconds))
        }
    }
}

impl From<AppConfig> for AppSettings {
    fn from(value: AppConfig) -> Self {
        let limit_break_mode = value.limit_break_mode.or_else(|| {
            value
                .show_limit_break
                .map(|b| if b { LIMIT_BREAK_MODE_PANEL } else { LIMIT_BREAK_MODE_OFF })
        });
        let limit_break_mode = limit_break_mode.unwrap_or(LIMIT_BREAK_MODE_PANEL);

        Self {
            idle_seconds: value.idle_seconds,
            default_decoration: Decoration::from_config_key(&value.default_decoration),
            default_mode: ViewMode::from_config_key(&value.default_mode),
            dungeon_mode_enabled: value.dungeon_mode_enabled,
            theme_id: value.theme_id,
            role_theme_enabled: value.role_theme_enabled,
            limit_break_mode,
        }
    }
}

impl From<AppSettings> for AppConfig {
    fn from(value: AppSettings) -> Self {
        AppConfig {
            idle_seconds: value.idle_seconds,
            default_decoration: value.default_decoration.config_key().to_string(),
            default_mode: value.default_mode.config_key().to_string(),
            dungeon_mode_enabled: value.dungeon_mode_enabled,
            theme_id: value.theme_id,
            role_theme_enabled: value.role_theme_enabled,
            limit_break_mode: Some(value.limit_break_mode),
            show_limit_break: None,
        }
    }
}
