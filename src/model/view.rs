use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum IdleScene {
    #[default]
    Status,
    TopCritChain,
    AsciiArt,
    TipOfTheDay,
    AchievementTicker,
}

impl IdleScene {
    pub fn label(self) -> &'static str {
        match self {
            IdleScene::Status => "status",
            IdleScene::TopCritChain => "top-crit-chain",
            IdleScene::AsciiArt => "ascii-art",
            IdleScene::TipOfTheDay => "tip",
            IdleScene::AchievementTicker => "achievements",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            IdleScene::Status => "Connection & encounter healthcheck",
            IdleScene::TopCritChain => "Highlights the longest critical damage streak",
            IdleScene::AsciiArt => "Rotating ASCII art showcase",
            IdleScene::TipOfTheDay => "Rotation and encounter tips",
            IdleScene::AchievementTicker => "Recently unlocked achievements",
        }
    }
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum Decoration {
    // No additional decoration; compact one-line rows
    None,
    // Thin role-colored underline on the line below each row (two-line rows)
    #[default]
    Underline,
    // Role-colored background meter behind each row (one-line rows)
    Background,
}

impl Decoration {
    pub fn next(self) -> Self {
        match self {
            Decoration::Underline => Decoration::Background,
            Decoration::Background => Decoration::None,
            Decoration::None => Decoration::Underline,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Decoration::Underline => Decoration::None,
            Decoration::Background => Decoration::Underline,
            Decoration::None => Decoration::Background,
        }
    }

    pub fn row_height(self) -> u16 {
        match self {
            Decoration::Underline => 2,
            Decoration::Background | Decoration::None => 1,
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            Decoration::Underline => "decor:line",
            Decoration::Background => "decor:bg",
            Decoration::None => "decor:none",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Decoration::Underline => "Underline",
            Decoration::Background => "Background",
            Decoration::None => "None",
        }
    }

    pub fn config_key(self) -> &'static str {
        match self {
            Decoration::Underline => "underline",
            Decoration::Background => "background",
            Decoration::None => "none",
        }
    }

    pub fn from_config_key<S: AsRef<str>>(key: S) -> Self {
        match key.as_ref().to_ascii_lowercase().as_str() {
            "background" => Decoration::Background,
            "none" => Decoration::None,
            _ => Decoration::Underline,
        }
    }
}

// High-level view mode of the table
#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum ViewMode {
    #[default]
    Dps,
    Heal,
}

impl ViewMode {
    pub fn next(self) -> Self {
        match self {
            ViewMode::Dps => ViewMode::Heal,
            ViewMode::Heal => ViewMode::Dps,
        }
    }

    pub fn prev(self) -> Self {
        self.next()
    }

    pub fn short_label(self) -> &'static str {
        match self {
            ViewMode::Dps => "mode:DPS",
            ViewMode::Heal => "mode:HEAL",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ViewMode::Dps => "DPS",
            ViewMode::Heal => "HEAL",
        }
    }

    pub fn config_key(self) -> &'static str {
        match self {
            ViewMode::Dps => "dps",
            ViewMode::Heal => "heal",
        }
    }

    pub fn from_config_key<S: AsRef<str>>(key: S) -> Self {
        match key.as_ref().to_ascii_lowercase().as_str() {
            "heal" => ViewMode::Heal,
            _ => ViewMode::Dps,
        }
    }
}
