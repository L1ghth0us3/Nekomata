pub const WS_URL_DEFAULT: &str = "ws://127.0.0.1:10501/ws";

mod history_panel;
mod history_settings_panel;
mod settings;
mod state;
mod types;
mod view;

pub use history_panel::{DungeonPanelLevel, HistoryPanel, HistoryPanelLevel, HistoryView};
pub use history_settings_panel::{
    ArchiveBrowser, ConfirmAction, ConfirmDialog, ConfirmFocus, FilenamePrompt,
    HistorySettingsField, HistorySettingsPanel,
};
pub use settings::{AppSettings, LimitBreakMode, SettingsField};
pub use state::{AppSnapshot, AppState};
pub use types::{
    known_jobs, AppEvent, CombatantRow, EncounterSummary, LimitBreakCast, LimitBreakSummary,
};
pub use view::{Decoration, IdleScene, ViewMode};
