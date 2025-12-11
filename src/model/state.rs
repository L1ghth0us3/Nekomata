use std::cmp::Ordering;
use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::errors::AppError;

use super::{
    AppEvent, AppSettings, CombatantRow, Decoration, DungeonPanelLevel, EncounterSummary,
    HistoryPanel, HistoryPanelLevel, HistoryView, IdleScene, SettingsField, ViewMode,
};

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct AppSnapshot {
    pub connected: bool,
    pub last_update_ms: u128,
    pub encounter: Option<EncounterSummary>,
    pub rows: Vec<CombatantRow>,
    pub decoration: Decoration,
    pub mode: ViewMode,
    pub is_idle: bool,
    pub idle_scene: IdleScene,
    pub settings: AppSettings,
    pub show_settings: bool,
    pub settings_cursor: SettingsField,
    pub history: HistoryPanel,
    pub show_idle_overlay: bool,
    pub error: Option<AppError>,
    pub dungeon_active_zone: Option<String>,
}

#[derive(Clone, Debug)]
pub struct AppState {
    pub connected: bool,
    pub last_update: Option<Instant>,
    pub last_active: Option<Instant>,
    pub connected_since: Option<Instant>,
    pub encounter: Option<EncounterSummary>,
    pub rows: Vec<CombatantRow>,
    pub decoration: Decoration,
    pub mode: ViewMode,
    pub idle_scene: IdleScene,
    pub settings: AppSettings,
    pub show_settings: bool,
    pub settings_cursor: SettingsField,
    pub history: HistoryPanel,
    pub show_idle_overlay: bool,
    pub error: Option<AppError>,
    pub dungeon_active_zone: Option<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            connected: false,
            last_update: None,
            last_active: None,
            connected_since: None,
            encounter: None,
            rows: Vec::new(),
            decoration: Decoration::default(),
            mode: ViewMode::default(),
            idle_scene: IdleScene::default(),
            settings: AppSettings::default(),
            show_settings: false,
            settings_cursor: SettingsField::default(),
            history: HistoryPanel::default(),
            show_idle_overlay: true,
            error: None,
            dungeon_active_zone: None,
        }
    }
}

impl AppState {
    pub fn apply(&mut self, evt: AppEvent) {
        match evt {
            AppEvent::Connected => {
                self.connected = true;
                let now = Instant::now();
                self.last_update = Some(now);
                self.last_active = None;
                self.connected_since = Some(now);
            }
            AppEvent::Disconnected => {
                self.connected = false;
                self.last_update = None;
                self.last_active = None;
                self.connected_since = None;
            }
            AppEvent::CombatData { encounter, rows } => {
                let now = Instant::now();
                self.encounter = Some(encounter);
                self.rows = rows;
                self.resort_rows();
                self.last_update = Some(now);
                self.idle_scene = IdleScene::Status;
                if self
                    .encounter
                    .as_ref()
                    .map(|enc| enc.is_active)
                    .unwrap_or(false)
                {
                    self.last_active = Some(now);
                }
            }
            AppEvent::HistoryDatesLoaded { days } => {
                self.history.loading = false;
                self.history.error = None;
                self.history.days = days;
                if self.history.selected_day >= self.history.days.len() {
                    self.history.selected_day = 0;
                }
                if let Some(day) = self.history.current_day() {
                    if day.encounters.is_empty() {
                        self.history.selected_encounter = 0;
                    } else if self.history.selected_encounter >= day.encounters.len() {
                        self.history.selected_encounter = day.encounters.len() - 1;
                    }
                }
            }
            AppEvent::HistoryEncountersLoaded {
                date_id,
                encounters,
            } => {
                if let Some(day) = self.history.find_day_mut(&date_id) {
                    day.encounters = encounters;
                    day.encounters_loaded = true;
                    let new_len = day.encounters.len();
                    if self.history.selected_encounter >= new_len
                        && self.history.level == HistoryPanelLevel::Encounters
                    {
                        self.history.selected_encounter = new_len.saturating_sub(1);
                    }
                }
                self.history.loading = false;
            }
            AppEvent::HistoryEncounterLoaded { key, record } => {
                if let Some(item) = self.history.find_encounter_mut(&key) {
                    item.record = Some(record);
                }
                self.history.loading = false;
            }
            AppEvent::DungeonDatesLoaded { days } => {
                self.history.dungeon_days = days;
                if self.history.dungeon_selected_day >= self.history.dungeon_days.len() {
                    self.history.dungeon_selected_day = 0;
                }
                self.history.dungeon_selected_run = 0;
                self.history.dungeon_selected_child = 0;
                self.history.loading = false;
            }
            AppEvent::DungeonRunsLoaded { date_id, runs } => {
                if let Some(day) = self.history.find_dungeon_day_mut(&date_id) {
                    day.runs = runs;
                    day.runs_loaded = true;
                    let len = day.runs.len();
                    if self.history.dungeon_selected_run >= len {
                        self.history.dungeon_selected_run = len.saturating_sub(1);
                    }
                }
                self.history.loading = false;
            }
            AppEvent::DungeonRunLoaded { key, record } => {
                if let Some(run) = self.history.find_dungeon_run_mut(&key) {
                    let child_count = record.child_keys.len();
                    run.record = Some(record);
                    run.child_records = vec![None; child_count];
                }
                self.history.loading = false;
            }
            AppEvent::DungeonEncounterLoaded { key, record } => {
                'outer: for day in &mut self.history.dungeon_days {
                    for run in &mut day.runs {
                        if let Some(rec) = run.record.as_ref() {
                            if let Some(idx) = rec
                                .child_keys
                                .iter()
                                .position(|child_key| child_key.as_slice() == key.as_slice())
                            {
                                if run.child_records.len() < rec.child_keys.len() {
                                    run.child_records.resize(rec.child_keys.len(), None);
                                }
                                run.child_records[idx] = Some(record);
                                break 'outer;
                            }
                        }
                    }
                }
                self.history.loading = false;
            }
            AppEvent::DungeonSessionUpdate { active_zone } => {
                self.dungeon_active_zone = active_zone;
            }
            AppEvent::HistoryError { message } => {
                self.history.loading = false;
                self.history.error = Some(message);
            }
            AppEvent::SystemError { error } => {
                self.error = Some(error);
            }
        }
    }

    pub fn clone_snapshot(&self) -> AppSnapshot {
        let now = Instant::now();
        let last_update_ms = self
            .last_update
            .map(|instant| now.saturating_duration_since(instant).as_millis())
            .unwrap_or(0);
        AppSnapshot {
            connected: self.connected,
            last_update_ms,
            encounter: self.encounter.clone(),
            rows: self.rows.clone(),
            decoration: self.decoration,
            mode: self.mode,
            is_idle: self.is_idle_at(now),
            idle_scene: self.idle_scene,
            settings: self.settings.clone(),
            show_settings: self.show_settings,
            settings_cursor: self.settings_cursor,
            history: self.history.clone(),
            show_idle_overlay: self.show_idle_overlay,
            error: self.error.clone(),
            dungeon_active_zone: self.dungeon_active_zone.clone(),
        }
    }

    pub fn resort_rows(&mut self) {
        match self.mode {
            ViewMode::Dps => {
                self.rows.sort_by(|a, b| {
                    b.encdps
                        .partial_cmp(&a.encdps)
                        .unwrap_or(Ordering::Equal)
                        .then_with(|| a.name.cmp(&b.name))
                });
            }
            ViewMode::Heal => {
                self.rows.sort_by(|a, b| {
                    b.enchps
                        .partial_cmp(&a.enchps)
                        .unwrap_or(Ordering::Equal)
                        .then_with(|| a.name.cmp(&b.name))
                });
            }
        }
    }
}

impl AppState {
    pub fn is_idle_at(&self, now: Instant) -> bool {
        if !self.connected {
            return false;
        }
        let Some(threshold) = self.settings.idle_duration() else {
            return false;
        };
        if self
            .encounter
            .as_ref()
            .map(|enc| enc.is_active)
            .unwrap_or(false)
        {
            return false;
        }
        if let Some(active) = self.last_active {
            if now.saturating_duration_since(active) >= threshold {
                return true;
            }
            return false;
        }
        if let Some(since) = self.connected_since {
            return now.saturating_duration_since(since) >= threshold;
        }
        false
    }

    pub fn apply_settings(&mut self, settings: AppSettings) {
        self.settings = settings;
        self.sync_current_with_defaults();
    }

    pub fn adjust_idle_seconds(&mut self, delta: i64) -> bool {
        let current = self.settings.idle_seconds;
        let raw = current as i64 + delta;
        let adjusted = if raw < 0 { 0 } else { raw as u64 };
        if adjusted != current {
            self.settings.idle_seconds = adjusted;
            true
        } else {
            false
        }
    }

    pub fn adjust_selected_setting(&mut self, forward: bool) -> bool {
        match self.settings_cursor {
            SettingsField::IdleTimeout => self.adjust_idle_seconds(if forward { 1 } else { -1 }),
            SettingsField::DefaultDecoration => {
                let changed = self.cycle_default_decoration(forward);
                if changed {
                    self.sync_current_with_defaults();
                }
                changed
            }
            SettingsField::DefaultMode => {
                let changed = self.cycle_default_mode(forward);
                if changed {
                    self.sync_current_with_defaults();
                }
                changed
            }
            SettingsField::DungeonMode => {
                let before = self.settings.dungeon_mode_enabled;
                let after = if forward { !before } else { !before };
                if after != before {
                    self.settings.dungeon_mode_enabled = after;
                    true
                } else {
                    false
                }
            } // Placeholder for future settings fields
        }
    }

    pub fn next_setting(&mut self) {
        self.settings_cursor = self.settings_cursor.next();
    }

    pub fn prev_setting(&mut self) {
        self.settings_cursor = self.settings_cursor.prev();
    }

    fn cycle_default_decoration(&mut self, forward: bool) -> bool {
        let current = self.settings.default_decoration;
        let next = if forward {
            current.next()
        } else {
            current.prev()
        };
        if next != current {
            self.settings.default_decoration = next;
            true
        } else {
            false
        }
    }

    fn cycle_default_mode(&mut self, forward: bool) -> bool {
        let current = self.settings.default_mode;
        let next = if forward {
            current.next()
        } else {
            current.prev()
        };
        if next != current {
            self.settings.default_mode = next;
            true
        } else {
            false
        }
    }

    fn sync_current_with_defaults(&mut self) {
        self.decoration = self.settings.default_decoration;
        self.mode = self.settings.default_mode;
        self.resort_rows();
    }

    pub fn toggle_history(&mut self) -> bool {
        if self.history.visible {
            self.history.visible = false;
            self.history.reset();
            false
        } else {
            self.history.visible = true;
            self.history.loading = true;
            self.history.error = None;
            self.history.level = HistoryPanelLevel::Dates;
            self.history.dungeon_level = DungeonPanelLevel::Dates;
            self.history.selected_day = 0;
            self.history.selected_encounter = 0;
            self.history.dungeon_selected_day = 0;
            self.history.dungeon_selected_run = 0;
            self.history.dungeon_selected_child = 0;
            self.history.detail_mode = self.mode;
            self.history.dungeon_detail_mode = self.mode;
            true
        }
    }

    pub fn history_set_loading(&mut self) {
        self.history.loading = true;
        self.history.error = None;
    }

    pub fn history_move_selection(&mut self, delta: i32) {
        if !self.history.visible || self.history.loading {
            return;
        }
        match self.history.view {
            HistoryView::Encounters => match self.history.level {
                HistoryPanelLevel::Dates => {
                    if self.history.days.is_empty() {
                        return;
                    }
                    let len = self.history.days.len() as i32;
                    let current = self.history.selected_day as i32;
                    let mut next = current + delta;
                    if next < 0 {
                        next = 0;
                    } else if next >= len {
                        next = len - 1;
                    }
                    self.history.selected_day = next as usize;
                    if let Some(day) = self.history.current_day() {
                        if day.encounters.is_empty() {
                            self.history.selected_encounter = 0;
                        } else if self.history.selected_encounter >= day.encounters.len() {
                            self.history.selected_encounter = day.encounters.len() - 1;
                        }
                    }
                }
                HistoryPanelLevel::Encounters | HistoryPanelLevel::EncounterDetail => {
                    if let Some(day) = self.history.current_day() {
                        if day.encounters.is_empty() {
                            return;
                        }
                        let len = day.encounters.len() as i32;
                        let current = self.history.selected_encounter as i32;
                        let mut next = current + delta;
                        if next < 0 {
                            next = 0;
                        } else if next >= len {
                            next = len - 1;
                        }
                        self.history.selected_encounter = next as usize;
                    }
                }
            },
            HistoryView::Dungeons => match self.history.dungeon_level {
                DungeonPanelLevel::Dates => {
                    if self.history.dungeon_days.is_empty() {
                        return;
                    }
                    let len = self.history.dungeon_days.len() as i32;
                    let current = self.history.dungeon_selected_day as i32;
                    let mut next = current + delta;
                    if next < 0 {
                        next = 0;
                    } else if next >= len {
                        next = len - 1;
                    }
                    self.history.dungeon_selected_day = next as usize;
                    if let Some(day) = self.history.current_dungeon_day() {
                        if day.runs.is_empty() {
                            self.history.dungeon_selected_run = 0;
                        } else if self.history.dungeon_selected_run >= day.runs.len() {
                            self.history.dungeon_selected_run = day.runs.len() - 1;
                        }
                        self.history.dungeon_selected_child = 0;
                    }
                }
                DungeonPanelLevel::Runs => {
                    if let Some(day) = self.history.current_dungeon_day() {
                        if day.runs.is_empty() {
                            return;
                        }
                        let len = day.runs.len() as i32;
                        let current = self.history.dungeon_selected_run as i32;
                        let mut next = current + delta;
                        if next < 0 {
                            next = 0;
                        } else if next >= len {
                            next = len - 1;
                        }
                        self.history.dungeon_selected_run = next as usize;
                        self.history.dungeon_selected_child = 0;
                    }
                }
                DungeonPanelLevel::RunDetail => {
                    if let Some(run) = self.history.current_dungeon_run() {
                        let child_len = run
                            .record
                            .as_ref()
                            .map(|rec| rec.child_keys.len())
                            .unwrap_or(run.child_records.len());
                        if child_len == 0 {
                            return;
                        }
                        let len = child_len as i32;
                        let current = self.history.dungeon_selected_child as i32;
                        let mut next = current + delta;
                        if next < 0 {
                            next = 0;
                        } else if next >= len {
                            next = len - 1;
                        }
                        self.history.dungeon_selected_child = next as usize;
                    }
                }
                DungeonPanelLevel::EncounterDetail => {
                    if let Some(run) = self.history.current_dungeon_run() {
                        let child_len = run
                            .record
                            .as_ref()
                            .map(|rec| rec.child_keys.len())
                            .unwrap_or(run.child_records.len());
                        if child_len == 0 {
                            return;
                        }
                        let len = child_len as i32;
                        let current = self.history.dungeon_selected_child as i32;
                        let mut next = current + delta;
                        if next < 0 {
                            next = 0;
                        } else if next >= len {
                            next = len - 1;
                        }
                        self.history.dungeon_selected_child = next as usize;
                    }
                }
            },
        }
    }

    pub fn history_toggle_mode(&mut self) {
        if !self.history.visible || self.history.loading {
            return;
        }
        match self.history.view {
            HistoryView::Encounters => {
                if self.history.level == HistoryPanelLevel::EncounterDetail {
                    self.history.detail_mode = self.history.detail_mode.next();
                }
            }
            HistoryView::Dungeons => match self.history.dungeon_level {
                DungeonPanelLevel::RunDetail => {
                    self.history.dungeon_detail_mode = self.history.dungeon_detail_mode.next();
                }
                DungeonPanelLevel::EncounterDetail => {
                    self.history.detail_mode = self.history.detail_mode.next();
                }
                _ => {}
            },
        }
    }

    pub fn history_toggle_view(&mut self) {
        if !self.history.visible {
            return;
        }
        self.history.loading = false;
        match self.history.view {
            HistoryView::Encounters => {
                self.history.view = HistoryView::Dungeons;
                self.history.dungeon_level = DungeonPanelLevel::Dates;
                self.history.error = None;
            }
            HistoryView::Dungeons => {
                self.history.view = HistoryView::Encounters;
                self.history.level = HistoryPanelLevel::Dates;
                self.history.error = None;
            }
        }
    }

    pub fn history_enter(&mut self) {
        if !self.history.visible || self.history.loading {
            return;
        }
        match self.history.view {
            HistoryView::Encounters => match self.history.level {
                HistoryPanelLevel::Dates => {
                    if let Some(day) = self.history.current_day() {
                        if day.encounters_loaded {
                            if !day.encounters.is_empty() {
                                self.history.level = HistoryPanelLevel::Encounters;
                                self.history.selected_encounter = 0;
                            }
                        } else if !day.encounter_ids.is_empty() {
                            self.history.level = HistoryPanelLevel::Encounters;
                            self.history.selected_encounter = 0;
                        }
                    }
                }
                HistoryPanelLevel::Encounters => {
                    if self.history.current_encounter().is_some() {
                        self.history.level = HistoryPanelLevel::EncounterDetail;
                    }
                }
                HistoryPanelLevel::EncounterDetail => {}
            },
            HistoryView::Dungeons => match self.history.dungeon_level {
                DungeonPanelLevel::Dates => {
                    if let Some(day) = self.history.current_dungeon_day() {
                        if day.runs_loaded {
                            if !day.runs.is_empty() {
                                self.history.dungeon_level = DungeonPanelLevel::Runs;
                                self.history.dungeon_selected_run = 0;
                            }
                        } else if !day.run_ids.is_empty() {
                            self.history.dungeon_level = DungeonPanelLevel::Runs;
                            self.history.dungeon_selected_run = 0;
                        }
                    }
                }
                DungeonPanelLevel::Runs => {
                    if self.history.current_dungeon_run().is_some() {
                        self.history.dungeon_level = DungeonPanelLevel::RunDetail;
                        self.history.dungeon_selected_child = 0;
                    }
                }
                DungeonPanelLevel::RunDetail => {
                    if let Some(run) = self.history.current_dungeon_run() {
                        if let Some(record) = run.record.as_ref() {
                            if !record.child_keys.is_empty() {
                                self.history.dungeon_level = DungeonPanelLevel::EncounterDetail;
                                self.history.dungeon_selected_child = 0;
                            }
                        }
                    }
                }
                DungeonPanelLevel::EncounterDetail => {}
            },
        }
    }

    pub fn history_back(&mut self) {
        if !self.history.visible {
            return;
        }
        match self.history.view {
            HistoryView::Encounters => match self.history.level {
                HistoryPanelLevel::EncounterDetail => {
                    self.history.level = HistoryPanelLevel::Encounters;
                }
                HistoryPanelLevel::Encounters => {
                    self.history.level = HistoryPanelLevel::Dates;
                    self.history.selected_encounter = 0;
                }
                HistoryPanelLevel::Dates => {}
            },
            HistoryView::Dungeons => match self.history.dungeon_level {
                DungeonPanelLevel::EncounterDetail => {
                    self.history.dungeon_level = DungeonPanelLevel::RunDetail;
                }
                DungeonPanelLevel::RunDetail => {
                    self.history.dungeon_level = DungeonPanelLevel::Runs;
                    self.history.dungeon_selected_child = 0;
                }
                DungeonPanelLevel::Runs => {
                    self.history.dungeon_level = DungeonPanelLevel::Dates;
                    self.history.dungeon_selected_run = 0;
                }
                DungeonPanelLevel::Dates => {}
            },
        }
    }
}
