use std::sync::{Arc, RwLock};

use crossterm::event::{KeyCode, KeyEvent};
use tokio::sync::mpsc;

use crate::config::{self, AppConfig};
use crate::history::{
    apply_retention_confirmed, begin_apply_limit, build_delete_archive_confirm,
    build_delete_live_confirm, build_discard_draft_confirm, commit_retention_policy,
    dry_run_for_policy, load_archive_entries, perform_backup, refresh_archive_count,
    remove_archive, spawn_initial_history_loads, sync_retention_state, adjust_draft_limit,
    HistorySessionHandle, HistoryStore, RetentionState,
};
use crate::model::{
    AppEvent, AppState, ConfirmAction, ConfirmFocus, HistorySettingsField,
};

pub struct HistorySettingsContext {
    pub session: HistorySessionHandle,
    pub retention_state: Arc<RwLock<RetentionState>>,
    pub app_cfg: AppConfig,
    pub event_tx: mpsc::UnboundedSender<AppEvent>,
    pub view_store: Option<Arc<HistoryStore>>,
}

pub async fn handle_history_settings_input(
    key: KeyEvent,
    state: &mut AppState,
    ctx: &mut HistorySettingsContext,
) -> bool {
    let panel = &mut state.history_settings;
    if !panel.visible {
        return false;
    }

    if let Some(confirm) = panel.confirm.clone() {
        return handle_confirm_key(key, state, ctx, confirm).await;
    }
    if panel.filename_prompt.is_some() {
        return handle_filename_key(key, state, ctx);
    }
    if panel.archive_browser.is_some() {
        return handle_archive_browser_key(key, state, ctx).await;
    }

    handle_main_panel_key(key, state, ctx).await
}

async fn handle_confirm_key(
    key: KeyEvent,
    state: &mut AppState,
    ctx: &mut HistorySettingsContext,
    confirm: crate::model::ConfirmDialog,
) -> bool {
    let panel = &mut state.history_settings;
    match key.code {
        KeyCode::Left | KeyCode::Right | KeyCode::Tab => {
            panel.confirm = Some(crate::model::ConfirmDialog {
                focus: match panel.confirm.as_ref().map(|c| c.focus) {
                    Some(ConfirmFocus::Cancel) => ConfirmFocus::Confirm,
                    _ => ConfirmFocus::Cancel,
                },
                ..confirm
            });
        }
        KeyCode::Esc => {
            panel.confirm = None;
        }
        KeyCode::Enter => {
            if confirm.focus == ConfirmFocus::Confirm {
                execute_confirm(state, ctx, confirm.action).await;
            } else {
                panel.confirm = None;
            }
        }
        _ => {}
    }
    true
}

fn handle_filename_key(
    key: KeyEvent,
    state: &mut AppState,
    ctx: &mut HistorySettingsContext,
) -> bool {
    let panel = &mut state.history_settings;
    let Some(prompt) = panel.filename_prompt.as_mut() else {
        return true;
    };
    match key.code {
        KeyCode::Esc => {
            panel.filename_prompt = None;
        }
        KeyCode::Enter => {
            let name = prompt.value.clone();
            match perform_backup(&name, &mut ctx.app_cfg) {
                Ok(path) => {
                    panel.filename_prompt = None;
                    panel.status_message = Some(format!(
                        "Backup succeeded: {}",
                        path.display()
                    ));
                    refresh_archive_count(state);
                }
                Err(err) => {
                    panel.filename_prompt = None;
                    panel.status_message =
                        Some(format!("Backup failed: {err}"));
                }
            }
        }
        KeyCode::Backspace => {
            prompt.value.pop();
            prompt.error = None;
        }
        KeyCode::Char(c) => {
            prompt.value.push(c);
            prompt.error = None;
        }
        _ => {}
    }
    true
}

async fn handle_archive_browser_key(
    key: KeyEvent,
    state: &mut AppState,
    ctx: &mut HistorySettingsContext,
) -> bool {
    let panel = &mut state.history_settings;
    let Some(browser) = panel.archive_browser.as_mut() else {
        return true;
    };
    match key.code {
        KeyCode::Esc => {
            panel.archive_browser = None;
        }
        KeyCode::Up => {
            if browser.selected > 0 {
                browser.selected -= 1;
            }
        }
        KeyCode::Down => {
            if browser.selected + 1 < browser.entries.len() {
                browser.selected += 1;
            }
        }
        KeyCode::Char('d') | KeyCode::Char('D') => {
            if let Some(entry) = browser.entries.get(browser.selected) {
                let name = entry.name.clone();
                build_delete_archive_confirm(panel, name);
            }
        }
        KeyCode::Enter => {
            if let Some(entry) = browser.entries.get(browser.selected).cloned() {
                open_archive_view(state, ctx, entry.name, entry.path).await;
            }
        }
        _ => {}
    }
    true
}

async fn handle_main_panel_key(
    key: KeyEvent,
    state: &mut AppState,
    ctx: &mut HistorySettingsContext,
) -> bool {
    let panel = &mut state.history_settings;
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            if panel.has_draft_changes() {
                build_discard_draft_confirm(panel);
            } else {
                panel.close();
            }
        }
        KeyCode::Up => {
            let kind = panel.draft_limit.kind;
            panel.cursor = panel.cursor.prev(kind);
        }
        KeyCode::Down => {
            let kind = panel.draft_limit.kind;
            panel.cursor = panel.cursor.next(kind);
        }
        KeyCode::Left | KeyCode::Right => {
            let forward = matches!(key.code, KeyCode::Right);
            let cursor = panel.cursor;
            if cursor == HistorySettingsField::Recording {
                let enabled = !state.settings.history_enabled;
                state.settings.history_enabled = enabled;
                ctx.app_cfg.history_enabled = enabled;
                let _ = config::save(&ctx.app_cfg);
                if enabled {
                    if let Err(err) = ctx.session.enable().await {
                        panel.status_message = Some(format!("Failed to enable history: {err}"));
                        state.settings.history_enabled = false;
                        ctx.app_cfg.history_enabled = false;
                        let _ = config::save(&ctx.app_cfg);
                    } else if panel.committed_limit.is_applied_in_config(&ctx.app_cfg) {
                        let _ = ctx
                            .session
                            .apply_retention_if_applied(&panel.committed_limit, &ctx.app_cfg)
                            .await;
                    }
                } else {
                    let _ = ctx.session.disable().await;
                    if state.history.visible && state.history.viewing_archive.is_none() {
                        state.history.visible = false;
                        state.history.reset();
                    }
                }
                panel.live_db_size = ctx.session.live_db_size_bytes().await;
            } else if matches!(
                cursor,
                HistorySettingsField::LimitKind | HistorySettingsField::LimitValue
            ) {
                adjust_draft_limit(panel, forward, cursor);
            }
        }
        KeyCode::Enter => match panel.cursor {
            HistorySettingsField::LimitKind | HistorySettingsField::LimitValue => {
                if panel.has_draft_changes() {
                    try_begin_apply_limit(state, ctx).await;
                }
            }
            HistorySettingsField::CreateBackup => {
                panel.open_filename_prompt();
            }
            HistorySettingsField::BrowseArchives => {
                match load_archive_entries() {
                    Ok(entries) => panel.open_archive_browser(entries),
                    Err(err) => panel.status_message = Some(err.to_string()),
                }
            }
            HistorySettingsField::DeleteCurrent => {
                build_delete_live_confirm(panel, &ctx.app_cfg, state.archive_count > 0);
            }
            _ => {}
        },
        _ => {}
    }
    true
}

async fn try_begin_apply_limit(state: &mut AppState, ctx: &mut HistorySettingsContext) {
    let policy = state.history_settings.draft_limit.clone();
    if let Some(store) = ctx.session.store().await {
        match dry_run_for_policy(&store, &policy) {
            Ok(plan) => {
                if plan.is_destructive() {
                    begin_apply_limit(&mut state.history_settings, plan, policy);
                } else {
                    commit_retention_policy(&mut ctx.app_cfg, &policy);
                    sync_retention_state(&ctx.retention_state, &ctx.app_cfg);
                    let _ = config::save(&ctx.app_cfg);
                    state.history_settings.committed_limit = policy.clone();
                    state.history_settings.draft_limit = policy.clone();
                    state.settings.history_limit = policy.clone();
                    state.history_settings.status_message = Some("Limit applied.".to_string());
                }
            }
            Err(err) => {
                state.history_settings.status_message = Some(err.to_string());
            }
        }
    } else if policy.kind == crate::history::HistoryLimitKind::None {
        commit_retention_policy(&mut ctx.app_cfg, &policy);
        sync_retention_state(&ctx.retention_state, &ctx.app_cfg);
        let _ = config::save(&ctx.app_cfg);
        state.history_settings.committed_limit = policy.clone();
        state.history_settings.draft_limit = policy;
        state.history_settings.status_message = Some("Limit applied.".to_string());
    } else {
        state.history_settings.status_message =
            Some("Enable history recording to apply this limit.".to_string());
    }
}

async fn execute_confirm(
    state: &mut AppState,
    ctx: &mut HistorySettingsContext,
    action: ConfirmAction,
) {
    let panel = &mut state.history_settings;
    panel.confirm = None;
    match action {
        ConfirmAction::ApplyRetention { policy } => {
            match apply_retention_confirmed(
                &ctx.session,
                &ctx.retention_state,
                &mut ctx.app_cfg,
                policy.clone(),
            )
            .await
            {
                Ok(()) => {
                    panel.committed_limit = policy.clone();
                    panel.draft_limit = policy.clone();
                    state.settings.history_limit = policy;
                    panel.status_message = Some("Limit applied.".to_string());
                    panel.live_db_size = ctx.session.live_db_size_bytes().await;
                    if state.history.visible && state.history.viewing_archive.is_none() {
                        state.history.reset();
                        if let Some(store) = ctx.session.store().await {
                            spawn_initial_history_loads(
                                &mut state.history,
                                store,
                                ctx.event_tx.clone(),
                            );
                        }
                    }
                }
                Err(err) => panel.status_message = Some(err.to_string()),
            }
        }
        ConfirmAction::DiscardDraft => {
            panel.draft_limit = panel.committed_limit.clone();
            panel.close();
        }
        ConfirmAction::DeleteArchive { name } => {
            match remove_archive(&name) {
                Ok(()) => {
                    refresh_archive_count(state);
                    if let Some(browser) = state.history_settings.archive_browser.as_mut() {
                        browser.entries.retain(|e| e.name != name);
                        if browser.selected >= browser.entries.len() {
                            browser.selected = browser.entries.len().saturating_sub(1);
                        }
                        browser.error = None;
                    }
                    state.history_settings.status_message =
                        Some(format!("Deleted archive {name}"));
                }
                Err(err) => {
                    if let Some(browser) = state.history_settings.archive_browser.as_mut() {
                        browser.error = Some(err.to_string());
                    }
                }
            }
        }
        ConfirmAction::DeleteLiveHistory => {
            let reopen = state.settings.history_enabled;
            match ctx.session.delete_live_and_reopen(reopen).await {
                Ok(()) => {
                    panel.live_db_size = ctx.session.live_db_size_bytes().await;
                    panel.status_message = Some("Live history deleted.".to_string());
                    if state.history.visible {
                        state.history.visible = false;
                        state.history.reset();
                    }
                }
                Err(err) => panel.status_message = Some(err.to_string()),
            }
        }
    }
}

async fn open_archive_view(
    state: &mut AppState,
    ctx: &mut HistorySettingsContext,
    name: String,
    path: std::path::PathBuf,
) {
    match HistoryStore::open(&path) {
        Ok(store) => {
            ctx.view_store = Some(Arc::new(store));
            state.history_settings.close();
            state.show_settings = false;
            state.history.visible = true;
            state.history.viewing_archive = Some(name);
            state.history.reset();
            if let Some(store) = ctx.view_store.clone() {
                spawn_initial_history_loads(&mut state.history, store, ctx.event_tx.clone());
            }
        }
        Err(err) => {
            state.history_settings.status_message = Some(err.to_string());
        }
    }
}

pub fn open_history_settings_panel(state: &mut AppState, live_size: Option<u64>) {
    state
        .history_settings
        .open(state.settings.committed_retention(), live_size);
}

pub async fn try_close_history_settings(state: &mut AppState) -> bool {
    if !state.history_settings.visible {
        return false;
    }
    if state.history_settings.has_draft_changes() {
        build_discard_draft_confirm(&mut state.history_settings);
        return true;
    }
    state.history_settings.close();
    true
}
