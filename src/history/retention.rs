use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Local, NaiveDate, TimeZone};
use serde::{Deserialize, Serialize};

use crate::config;

/// Mutually exclusive retention policy for the live history database.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HistoryLimitKind {
    None,
    MaxAgeDays,
    MaxSizeMb,
}

impl HistoryLimitKind {
    pub fn from_config_key(key: &str) -> Self {
        match key {
            "max_age_days" => Self::MaxAgeDays,
            "max_size_mb" => Self::MaxSizeMb,
            _ => Self::None,
        }
    }

    pub fn config_key(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::MaxAgeDays => "max_age_days",
            Self::MaxSizeMb => "max_size_mb",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::MaxAgeDays => "Older than",
            Self::MaxSizeMb => "Max size",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::None => Self::MaxAgeDays,
            Self::MaxAgeDays => Self::MaxSizeMb,
            Self::MaxSizeMb => Self::None,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::None => Self::MaxSizeMb,
            Self::MaxAgeDays => Self::None,
            Self::MaxSizeMb => Self::MaxAgeDays,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryRetentionPolicy {
    pub kind: HistoryLimitKind,
    pub days: u64,
    pub size_mb: u64,
}

impl Default for HistoryRetentionPolicy {
    fn default() -> Self {
        Self {
            kind: HistoryLimitKind::None,
            days: default_history_limit_days(),
            size_mb: default_history_limit_mb(),
        }
    }
}

impl HistoryRetentionPolicy {
    pub fn from_config(cfg: &config::AppConfig) -> Self {
        Self {
            kind: HistoryLimitKind::from_config_key(&cfg.history_limit_kind),
            days: cfg.history_limit_days,
            size_mb: cfg.history_limit_mb,
        }
    }

    pub fn applied_from_config(cfg: &config::AppConfig) -> Self {
        Self {
            kind: HistoryLimitKind::from_config_key(&cfg.history_limit_applied_kind),
            days: cfg.history_limit_applied_days,
            size_mb: cfg.history_limit_applied_mb,
        }
    }

    pub fn write_committed(&self, cfg: &mut config::AppConfig) {
        cfg.history_limit_kind = self.kind.config_key().to_string();
        cfg.history_limit_days = self.days;
        cfg.history_limit_mb = self.size_mb;
    }

    pub fn write_applied(&self, cfg: &mut config::AppConfig) {
        cfg.history_limit_applied_kind = self.kind.config_key().to_string();
        cfg.history_limit_applied_days = self.days;
        cfg.history_limit_applied_mb = self.size_mb;
    }

    pub fn matches_applied(&self, cfg: &config::AppConfig) -> bool {
        cfg.history_limit_kind == self.kind.config_key()
            && cfg.history_limit_days == self.days
            && cfg.history_limit_mb == self.size_mb
            && cfg.history_limit_applied_kind == self.kind.config_key()
            && cfg.history_limit_applied_days == self.days
            && cfg.history_limit_applied_mb == self.size_mb
    }

    pub fn is_applied_in_config(&self, cfg: &config::AppConfig) -> bool {
        self.kind.config_key() == cfg.history_limit_applied_kind
            && self.days == cfg.history_limit_applied_days
            && self.size_mb == cfg.history_limit_applied_mb
    }
}

#[derive(Clone, Debug, Default)]
pub struct RetentionPlan {
    pub encounter_count: usize,
    pub dungeon_count: usize,
    pub oldest_date: Option<String>,
    pub may_rebuild: bool,
}

impl RetentionPlan {
    pub fn is_destructive(&self) -> bool {
        self.encounter_count > 0 || self.dungeon_count > 0 || self.may_rebuild
    }
}

pub fn default_history_limit_days() -> u64 {
    30
}

pub fn default_history_limit_mb() -> u64 {
    256
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn cutoff_ms_for_days(days: u64) -> u64 {
    let days_ms = days.saturating_mul(86_400_000);
    now_ms().saturating_sub(days_ms)
}

pub fn format_oldest_date(ms: u64) -> Option<String> {
    let millis = i64::try_from(ms).ok()?;
    let dt: DateTime<Local> = Local.timestamp_millis_opt(millis).single()?;
    Some(dt.date_naive().format("%Y-%m-%d").to_string())
}

pub fn parse_iso_date(date: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limit_kind_round_trip() {
        for kind in [
            HistoryLimitKind::None,
            HistoryLimitKind::MaxAgeDays,
            HistoryLimitKind::MaxSizeMb,
        ] {
            assert_eq!(
                HistoryLimitKind::from_config_key(kind.config_key()),
                kind
            );
        }
    }
}
