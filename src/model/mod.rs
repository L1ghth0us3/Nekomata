pub const WS_URL_DEFAULT: &str = "ws://127.0.0.1:10501/ws";

mod history_panel;
mod settings;
mod state;
mod types;
mod view;

pub use history_panel::{DungeonPanelLevel, HistoryPanel, HistoryPanelLevel, HistoryView};
pub use settings::{
    AppSettings, SettingsField, LIMIT_BREAK_MODE_PANEL, LIMIT_BREAK_MODE_TABLE,
};
pub use state::{AppSnapshot, AppState};
pub use types::{
    known_jobs, AppEvent, CombatantRow, EncounterSummary, LimitBreakCast, LimitBreakHit,
    LimitBreakSummary,
};
pub use view::{Decoration, IdleScene, ViewMode};
