use std::sync::{Arc, RwLock};

use std::path::PathBuf;

use crate::config::{self, AppConfig};
use crate::history::{
    create_backup, delete_archive, list_archives, ArchiveEntry,
    HistoryLimitKind, HistoryRetentionPolicy, HistorySessionHandle, HistoryStore, RetentionPlan,
};
use crate::model::{
    AppState, ConfirmAction, ConfirmDialog, ConfirmFocus, HistorySettingsField,
};

pub struct RetentionState {
    pub policy: HistoryRetentionPolicy,
    pub applied: bool,
}

impl Default for RetentionState {
    fn default() -> Self {
        Self {
            policy: HistoryRetentionPolicy::default(),
            applied: true,
        }
    }
}

pub fn shared_retention_state(cfg: &AppConfig) -> Arc<RwLock<RetentionState>> {
    let policy = HistoryRetentionPolicy::from_config(cfg);
    let applied = policy.is_applied_in_config(cfg);
    Arc::new(RwLock::new(RetentionState { policy, applied }))
}

pub fn sync_retention_state(state: &Arc<RwLock<RetentionState>>, cfg: &AppConfig) {
    if let Ok(mut guard) = state.write() {
        guard.policy = HistoryRetentionPolicy::from_config(cfg);
        guard.applied = guard.policy.is_applied_in_config(cfg);
    }
}

pub fn refresh_archive_count(state: &mut AppState) {
    state.archive_count = list_archives().map(|v| v.len()).unwrap_or(0);
}

pub fn begin_apply_limit(
    panel: &mut crate::model::HistorySettingsPanel,
    plan: RetentionPlan,
    policy: HistoryRetentionPolicy,
) {
    if plan.is_destructive() {
        let mut message = String::new();
        if policy.kind == HistoryLimitKind::MaxAgeDays {
            message.push_str(&format!(
                "This will delete {} encounters and {} dungeon runs older than {} days",
                plan.encounter_count, plan.dungeon_count, policy.days
            ));
            if let Some(oldest) = &plan.oldest_date {
                message.push_str(&format!(" (oldest {oldest})"));
            }
            message.push_str(
                ".\nThis cannot be undone.\nAfter this, new data will also be removed automatically as it ages.\nConsider creating a backup first.",
            );
        } else if policy.kind == HistoryLimitKind::MaxSizeMb {
            message.push_str(&format!(
                "This will delete {} encounters and {} dungeon runs to bring the database under {} MB",
                plan.encounter_count, plan.dungeon_count, policy.size_mb
            ));
            if plan.may_rebuild {
                message.push_str("\nThe database file may be rebuilt.");
            }
            message.push_str(
                "\nThis cannot be undone.\nAfter this, oldest data will keep being removed automatically when over the size cap.\nConsider creating a backup first.",
            );
        }
        panel.confirm = Some(ConfirmDialog {
            message,
            confirm_label: "Apply".to_string(),
            action: ConfirmAction::ApplyRetention { policy },
            focus: ConfirmFocus::Cancel,
        });
    } else {
        panel.committed_limit = policy.clone();
        panel.draft_limit = policy;
    }
}

pub fn dry_run_for_policy(
    store: &HistoryStore,
    policy: &HistoryRetentionPolicy,
) -> anyhow::Result<RetentionPlan> {
    store.dry_run_retention(policy)
}

pub fn commit_retention_policy(cfg: &mut AppConfig, policy: &HistoryRetentionPolicy) {
    policy.write_committed(cfg);
    policy.write_applied(cfg);
}

pub fn build_delete_live_confirm(
    panel: &mut crate::model::HistorySettingsPanel,
    cfg: &AppConfig,
    has_archives: bool,
) {
    let dirty = cfg.history_last_backup_ms.is_none();
    let needs_strong_warning = !has_archives || dirty;
    let message = if needs_strong_warning {
        "This will erase the live history database.\nIt has not been backed up (or has changed since the last backup).\nCreate a backup instead unless you are sure.".to_string()
    } else {
        "Delete all live history?\nArchives are kept.".to_string()
    };
    panel.confirm = Some(ConfirmDialog {
        message,
        confirm_label: if needs_strong_warning {
            "Delete anyway".to_string()
        } else {
            "Delete".to_string()
        },
        action: ConfirmAction::DeleteLiveHistory,
        focus: ConfirmFocus::Cancel,
    });
}

pub fn build_delete_archive_confirm(panel: &mut crate::model::HistorySettingsPanel, name: String) {
    panel.confirm = Some(ConfirmDialog {
        message: format!(
            "Delete archive \"{name}\"?\nThis cannot be undone."
        ),
        confirm_label: "Delete".to_string(),
        action: ConfirmAction::DeleteArchive { name },
        focus: ConfirmFocus::Cancel,
    });
}

pub fn build_discard_draft_confirm(panel: &mut crate::model::HistorySettingsPanel) {
    panel.confirm = Some(ConfirmDialog {
        message: "Discard unapplied limit changes?".to_string(),
        confirm_label: "Discard".to_string(),
        action: ConfirmAction::DiscardDraft,
        focus: ConfirmFocus::Cancel,
    });
}

pub async fn apply_retention_confirmed(
    session: &HistorySessionHandle,
    retention_state: &Arc<RwLock<RetentionState>>,
    cfg: &mut AppConfig,
    policy: HistoryRetentionPolicy,
) -> anyhow::Result<()> {
    if let Some(store) = session.store().await {
        store.apply_retention(&policy)?;
    }
    commit_retention_policy(cfg, &policy);
    sync_retention_state(retention_state, cfg);
    config::save(cfg)?;
    Ok(())
}

pub fn perform_backup(name: &str, cfg: &mut AppConfig) -> anyhow::Result<PathBuf> {
    let dest = create_backup(name)?;
    cfg.history_last_backup_ms = Some(crate::history::retention::now_ms());
    config::save(cfg)?;
    Ok(dest.canonicalize().unwrap_or(dest))
}

pub fn load_archive_entries() -> anyhow::Result<Vec<ArchiveEntry>> {
    list_archives()
}

pub fn remove_archive(name: &str) -> anyhow::Result<()> {
    delete_archive(name)
}

pub fn adjust_draft_limit(
    panel: &mut crate::model::HistorySettingsPanel,
    forward: bool,
    field: HistorySettingsField,
) {
    match field {
        HistorySettingsField::LimitKind => {
            panel.draft_limit.kind = if forward {
                panel.draft_limit.kind.next()
            } else {
                panel.draft_limit.kind.prev()
            };
        }
        HistorySettingsField::LimitValue => match panel.draft_limit.kind {
            HistoryLimitKind::MaxAgeDays => {
                let delta = if forward { 1 } else { -1 };
                let next = panel.draft_limit.days as i64 + delta;
                panel.draft_limit.days = next.max(1) as u64;
            }
            HistoryLimitKind::MaxSizeMb => {
                let delta = if forward { 1 } else { -1 };
                let next = panel.draft_limit.size_mb as i64 + delta;
                panel.draft_limit.size_mb = next.max(1) as u64;
            }
            HistoryLimitKind::None => {}
        },
        _ => {}
    }
}
