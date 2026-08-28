use std::sync::Arc;

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use tokio::sync::mpsc::UnboundedSender;
use tokio::task;

use crate::history::types::{DUNGEON_NAMESPACE, ENCOUNTER_NAMESPACE};
use crate::history::HistoryStore;
use crate::model::{
    AppEvent, AppState, DungeonPanelLevel, HistoryDeleteAction, HistoryPanelLevel, HistoryView,
};

pub const HISTORY_LIST_OFFSET: u16 = 4;

#[allow(clippy::enum_variant_names)]
pub enum HistoryTask {
    LoadEncounters { date_id: String },
    LoadEncounterDetail { key: Vec<u8> },
    LoadDungeonDays,
    LoadDungeonRuns { date_id: String },
    LoadDungeonRunDetail { key: Vec<u8> },
    LoadDungeonEncounter { key: Vec<u8> },
}

pub fn spawn_load<T, F>(
    store: Arc<HistoryStore>,
    tx: UnboundedSender<AppEvent>,
    load: F,
    map_ok: impl FnOnce(T) -> AppEvent + Send + 'static,
    map_err: impl FnOnce(String) -> AppEvent + Send + 'static,
) where
    T: Send + 'static,
    F: FnOnce(&HistoryStore) -> anyhow::Result<T> + Send + 'static,
{
    tokio::spawn(async move {
        let result = task::spawn_blocking(move || load(&store)).await;
        let event = match result {
            Ok(Ok(value)) => map_ok(value),
            Ok(Err(err)) => map_err(err.to_string()),
            Err(err) => map_err(format!("History load failed: {err}")),
        };
        let _ = tx.send(event);
    });
}

pub fn spawn_initial_history_loads(
    panel: &mut crate::model::HistoryPanel,
    store: Arc<HistoryStore>,
    tx: UnboundedSender<AppEvent>,
) {
    panel.begin_load();
    spawn_load(
        store.clone(),
        tx.clone(),
        |store| store.load_dates(),
        |days| AppEvent::HistoryDatesLoaded { days },
        |message| AppEvent::HistoryError { message },
    );
    panel.begin_load();
    spawn_load(
        store,
        tx,
        |store| store.load_dungeon_days(),
        |days| AppEvent::DungeonDatesLoaded { days },
        |message| AppEvent::HistoryError { message },
    );
}

pub async fn handle_history_mouse(
    mouse: MouseEvent,
    state: &tokio::sync::RwLock<AppState>,
    list_offset: usize,
) {
    let mut s = state.write().await;
    if !s.history.visible || s.history.loading || s.history.confirm.is_some() {
        return;
    }

    match mouse.kind {
        MouseEventKind::ScrollDown => s.history_move_selection(1),
        MouseEventKind::ScrollUp => s.history_move_selection(-1),
        MouseEventKind::Down(MouseButton::Left) => {
            let row = mouse.row.saturating_sub(HISTORY_LIST_OFFSET) as usize;
            let index = row.saturating_add(list_offset);
            match s.history.view {
                HistoryView::Encounters => match s.history.level {
                    HistoryPanelLevel::Dates => {
                        if !s.history.days.is_empty() {
                            let max_index = s.history.days.len().saturating_sub(1);
                            s.history.selected_day = index.min(max_index);
                        }
                        s.history_enter();
                    }
                    HistoryPanelLevel::Encounters => {
                        if let Some(day) = s.history.current_day() {
                            if !day.encounters.is_empty() {
                                let max_index = day.encounters.len().saturating_sub(1);
                                s.history.selected_encounter = index.min(max_index);
                                s.history_enter();
                            }
                        }
                    }
                    HistoryPanelLevel::EncounterDetail => {}
                },
                HistoryView::Dungeons => match s.history.dungeon_level {
                    DungeonPanelLevel::Dates => {
                        if !s.history.dungeon_days.is_empty() {
                            let max_index = s.history.dungeon_days.len().saturating_sub(1);
                            s.history.dungeon_selected_day = index.min(max_index);
                        }
                        s.history_enter();
                    }
                    DungeonPanelLevel::Runs => {
                        if let Some(day) = s.history.current_dungeon_day() {
                            if !day.runs.is_empty() {
                                let max_index = day.runs.len().saturating_sub(1);
                                s.history.dungeon_selected_run = index.min(max_index);
                                s.history_enter();
                            }
                        }
                    }
                    DungeonPanelLevel::RunDetail => {
                        if let Some(run) = s.history.current_dungeon_run() {
                            if let Some(rec) = run.record.as_ref() {
                                if !rec.child_keys.is_empty() {
                                    let max_index = rec.child_keys.len().saturating_sub(1);
                                    s.history.dungeon_selected_child = index.min(max_index);
                                }
                            }
                        }
                    }
                    DungeonPanelLevel::EncounterDetail => {}
                },
            }
        }
        _ => {}
    }
}

pub fn determine_history_task(state: &mut AppState) -> Option<HistoryTask> {
    if state.history.loading {
        return None;
    }

    let mut task = None;
    let mut blocking = false;

    match state.history.view {
        HistoryView::Encounters => match state.history.level {
            HistoryPanelLevel::Dates => {}
            HistoryPanelLevel::Encounters => {
                if let Some(day) = state.history.current_day() {
                    if !day.encounters_loaded && !day.encounter_ids.is_empty() {
                        task = Some(HistoryTask::LoadEncounters {
                            date_id: day.iso_date.clone(),
                        });
                        blocking = true;
                    }
                }
            }
            HistoryPanelLevel::EncounterDetail => {
                if let Some(enc) = state.history.current_encounter() {
                    if enc.record.is_none() {
                        task = Some(HistoryTask::LoadEncounterDetail {
                            key: enc.key.clone(),
                        });
                        blocking = true;
                    }
                }
            }
        },
        HistoryView::Dungeons => match state.history.dungeon_level {
            DungeonPanelLevel::Dates => {
                if state.history.dungeon_days.is_empty() {
                    task = Some(HistoryTask::LoadDungeonDays);
                    blocking = true;
                }
            }
            DungeonPanelLevel::Runs => {
                if let Some(day) = state.history.current_dungeon_day() {
                    if !day.runs_loaded && !day.run_ids.is_empty() {
                        task = Some(HistoryTask::LoadDungeonRuns {
                            date_id: day.iso_date.clone(),
                        });
                        blocking = true;
                    }
                }
            }
            DungeonPanelLevel::RunDetail => {
                if let Some(run) = state.history.current_dungeon_run() {
                    if run.record.is_none() {
                        task = Some(HistoryTask::LoadDungeonRunDetail {
                            key: run.key.clone(),
                        });
                        blocking = true;
                    }
                }
            }
            DungeonPanelLevel::EncounterDetail => {
                if let Some(run) = state.history.current_dungeon_run() {
                    if let Some(rec) = run.record.as_ref() {
                        let idx = state.history.dungeon_selected_child;
                        if let Some(key) = rec.child_keys.get(idx) {
                            let needs_load = run
                                .child_records
                                .get(idx)
                                .and_then(|entry| entry.as_ref())
                                .is_none();
                            if needs_load {
                                task = Some(HistoryTask::LoadDungeonEncounter { key: key.clone() });
                                blocking = true;
                            }
                        }
                    }
                }
            }
        },
    }

    if blocking {
        state.history_begin_load();
    }

    task
}

pub fn spawn_history_task(
    task: HistoryTask,
    store: Arc<HistoryStore>,
    tx: UnboundedSender<AppEvent>,
) {
    match task {
        HistoryTask::LoadEncounters { date_id } => {
            let date_for_load = date_id.clone();
            spawn_load(
                store,
                tx,
                move |store| store.load_encounter_summaries(&date_for_load),
                move |encounters| AppEvent::HistoryEncountersLoaded {
                    date_id,
                    encounters,
                },
                |message| AppEvent::HistoryError { message },
            )
        }
        HistoryTask::LoadEncounterDetail { key } => {
            let key_for_load = key.clone();
            spawn_load(
                store,
                tx,
                move |store| store.load_encounter_record(&key_for_load),
                move |record| AppEvent::HistoryEncounterLoaded {
                    key,
                    record: Arc::new(record),
                },
                |message| AppEvent::HistoryError { message },
            )
        }
        HistoryTask::LoadDungeonDays => spawn_load(
            store,
            tx,
            |store| store.load_dungeon_days(),
            |days| AppEvent::DungeonDatesLoaded { days },
            |message| AppEvent::HistoryError { message },
        ),
        HistoryTask::LoadDungeonRuns { date_id } => {
            let date_for_load = date_id.clone();
            spawn_load(
                store,
                tx,
                move |store| store.load_dungeon_summaries(&date_for_load),
                move |runs| AppEvent::DungeonRunsLoaded { date_id, runs },
                |message| AppEvent::HistoryError { message },
            )
        }
        HistoryTask::LoadDungeonRunDetail { key } => {
            let tx_run = tx.clone();
            let store_clone = store.clone();
            tokio::spawn(async move {
                let key_for_block = key.clone();
                let store_for_load = store_clone.clone();
                let result = task::spawn_blocking(move || {
                    store_for_load.load_dungeon_record(&key_for_block)
                })
                .await;
                match result {
                    Ok(Ok(record)) => {
                        let child_keys = record.child_keys.clone();
                        let _ = tx_run.send(AppEvent::DungeonRunLoaded {
                            key: key.clone(),
                            record: record.clone(),
                        });
                        for child_key in child_keys {
                            spawn_load(
                                store_clone.clone(),
                                tx_run.clone(),
                                {
                                    let child_key = child_key.clone();
                                    move |store| store.load_encounter_record(&child_key)
                                },
                                {
                                    let child_key = child_key.clone();
                                    move |record| AppEvent::DungeonEncounterLoaded {
                                        key: child_key,
                                        record: Arc::new(record),
                                    }
                                },
                                |message| AppEvent::HistoryError { message },
                            );
                        }
                    }
                    Ok(Err(err)) => {
                        let _ = tx_run.send(AppEvent::HistoryError {
                            message: format!("Failed to load dungeon run: {err}"),
                        });
                    }
                    Err(err) => {
                        let _ = tx_run.send(AppEvent::HistoryError {
                            message: format!("History load failed: {err}"),
                        });
                    }
                }
            });
        }
        HistoryTask::LoadDungeonEncounter { key } => {
            let key_for_load = key.clone();
            spawn_load(
                store,
                tx,
                move |store| store.load_encounter_record(&key_for_load),
                move |record| AppEvent::DungeonEncounterLoaded {
                    key,
                    record: Arc::new(record),
                },
                |message| AppEvent::HistoryError { message },
            )
        }
    }
}

pub fn spawn_history_delete(
    store: Arc<HistoryStore>,
    tx: UnboundedSender<AppEvent>,
    action: HistoryDeleteAction,
) {
    tokio::spawn(async move {
        let action_for_event = action.clone();
        let result = task::spawn_blocking(move || execute_history_delete(&store, &action)).await;
        let event = match result {
            Ok(Ok(deleted_encounter_keys)) => AppEvent::HistoryDeleted {
                action: action_for_event,
                deleted_encounter_keys,
            },
            Ok(Err(err)) => AppEvent::HistoryError {
                message: err.to_string(),
            },
            Err(err) => AppEvent::HistoryError {
                message: format!("History delete failed: {err}"),
            },
        };
        let _ = tx.send(event);
    });
}

fn execute_history_delete(
    store: &HistoryStore,
    action: &HistoryDeleteAction,
) -> anyhow::Result<Vec<Vec<u8>>> {
    match action {
        HistoryDeleteAction::Encounter { key, .. } => {
            store.remove_entry(key)?;
            store.flush()?;
            Ok(Vec::new())
        }
        HistoryDeleteAction::EncounterDate { date_id } => {
            store.remove_date(date_id, ENCOUNTER_NAMESPACE)?;
            store.flush()?;
            Ok(Vec::new())
        }
        HistoryDeleteAction::DungeonRun {
            key, with_children, ..
        } => {
            let deleted = store.remove_dungeon_run(key, *with_children)?;
            store.flush()?;
            Ok(deleted)
        }
        HistoryDeleteAction::DungeonDate { date_id } => {
            store.remove_date(date_id, DUNGEON_NAMESPACE)?;
            store.flush()?;
            Ok(Vec::new())
        }
    }
}
