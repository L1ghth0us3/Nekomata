pub(crate) mod dungeon;
pub mod archives;
pub mod loader;
pub mod recorder;
pub mod retention;
pub mod session;
pub mod settings_controller;
pub mod settings_input;
pub mod store;
pub mod types;
pub(crate) mod util;

pub use archives::{
    create_backup, default_backup_name, delete_archive, delete_live_history, format_bytes,
    list_archives, ArchiveEntry,
};
pub use loader::{
    determine_history_task, handle_history_mouse, spawn_history_task, spawn_initial_history_loads,
};
pub use recorder::{spawn_recorder, RecorderHandle};
pub use retention::{
    cutoff_ms_for_days, default_history_limit_days, default_history_limit_mb, format_oldest_date,
    HistoryLimitKind, HistoryRetentionPolicy, RetentionPlan,
};
pub use settings_controller::{
    apply_retention_confirmed, begin_apply_limit, build_delete_archive_confirm,
    build_delete_live_confirm, build_discard_draft_confirm, commit_retention_policy,
    dry_run_for_policy, load_archive_entries, perform_backup, refresh_archive_count,
    remove_archive, shared_retention_state, sync_retention_state, adjust_draft_limit,
    RetentionState,
};
pub use session::{HistorySession, HistorySessionHandle};
pub use settings_input::{
    handle_history_settings_input, open_history_settings_panel, try_close_history_settings,
    HistorySettingsContext,
};
pub use store::HistoryStore;
pub use types::{
    DungeonAggregateRecord, DungeonHistoryDay, DungeonHistoryItem, EncounterRecord, HistoryDay,
    HistoryEncounterItem,
};
