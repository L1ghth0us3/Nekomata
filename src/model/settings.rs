use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::config::AppConfig;
use crate::history::HistoryRetentionPolicy;

use super::{Decoration, ViewMode};

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum LimitBreakMode {
    #[default]
    Off = 0,
    Panel = 1,
    TableRow = 2,
}

impl LimitBreakMode {
    pub fn from_u8(value: u8) -> Self {
        match value {
            1 => Self::Panel,
            2 => Self::TableRow,
            _ => Self::Off,
        }
    }

    pub fn as_u8(self) -> u8 {
        self as u8
    }

    pub fn next(self) -> Self {
        match self {
            Self::Off => Self::Panel,
            Self::Panel => Self::TableRow,
            Self::TableRow => Self::Off,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Off => Self::TableRow,
            Self::Panel => Self::Off,
            Self::TableRow => Self::Panel,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Panel => "PanelAlways",
            Self::TableRow => "TableRow",
        }
    }
}

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
    HistorySettings,
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
            SettingsField::RoleTheme => SettingsField::HistorySettings,
            SettingsField::HistorySettings => SettingsField::IdleTimeout,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            SettingsField::IdleTimeout => SettingsField::HistorySettings,
            SettingsField::DefaultDecoration => SettingsField::IdleTimeout,
            SettingsField::DefaultMode => SettingsField::DefaultDecoration,
            SettingsField::DungeonMode => SettingsField::DefaultMode,
            SettingsField::LimitBreakMode => SettingsField::DungeonMode,
            SettingsField::Theme => SettingsField::LimitBreakMode,
            SettingsField::RoleTheme => SettingsField::Theme,
            SettingsField::HistorySettings => SettingsField::RoleTheme,
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
    pub limit_break_mode: LimitBreakMode,
    pub history_enabled: bool,
    pub history_limit: HistoryRetentionPolicy,
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
            limit_break_mode: LimitBreakMode::Panel,
            history_enabled: true,
            history_limit: HistoryRetentionPolicy::default(),
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

    pub fn committed_retention(&self) -> HistoryRetentionPolicy {
        self.history_limit.clone()
    }
}

impl From<AppConfig> for AppSettings {
    fn from(value: AppConfig) -> Self {
        let limit_break_mode = value.limit_break_mode.or_else(|| {
            value.show_limit_break.map(|b| {
                if b {
                    LimitBreakMode::Panel.as_u8()
                } else {
                    LimitBreakMode::Off.as_u8()
                }
            })
        });
        let limit_break_mode =
            LimitBreakMode::from_u8(limit_break_mode.unwrap_or(LimitBreakMode::Panel.as_u8()));

        Self {
            idle_seconds: value.idle_seconds,
            default_decoration: Decoration::from_config_key(&value.default_decoration),
            default_mode: ViewMode::from_config_key(&value.default_mode),
            dungeon_mode_enabled: value.dungeon_mode_enabled,
            theme_id: value.theme_id.clone(),
            role_theme_enabled: value.role_theme_enabled,
            limit_break_mode,
            history_enabled: value.history_enabled,
            history_limit: HistoryRetentionPolicy::from_config(&value),
        }
    }
}

impl From<AppSettings> for AppConfig {
    fn from(value: AppSettings) -> Self {
        let mut cfg = AppConfig {
            idle_seconds: value.idle_seconds,
            default_decoration: value.default_decoration.config_key().to_string(),
            default_mode: value.default_mode.config_key().to_string(),
            dungeon_mode_enabled: value.dungeon_mode_enabled,
            theme_id: value.theme_id,
            role_theme_enabled: value.role_theme_enabled,
            limit_break_mode: Some(value.limit_break_mode.as_u8()),
            show_limit_break: None,
            history_enabled: value.history_enabled,
            ..Default::default()
        };
        value.history_limit.write_committed(&mut cfg);
        cfg
    }
}
