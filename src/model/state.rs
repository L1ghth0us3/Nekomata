use std::cmp::Ordering;
use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::errors::AppError;
use crate::theme;

use super::{
    AppEvent, AppSettings, CombatantRow, Decoration, DungeonPanelLevel, EncounterSummary,
    HistoryPanel, HistoryPanelLevel, HistorySettingsPanel, IdleScene, LimitBreakSummary,
    SettingsField, ViewMode,
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
    #[serde(default, skip)]
    pub history_settings: HistorySettingsPanel,
    #[serde(default)]
    pub archive_count: usize,
    pub show_idle_overlay: bool,
    pub error: Option<AppError>,
    pub dungeon_active_zone: Option<String>,
    pub lb_summary: Option<LimitBreakSummary>,
}

#[derive(Clone, Debug)]
pub struct AppState {
    pub connected: bool,
    pub last_update: Option<Instant>,
    pub last_active: Option<Instant>,
    pub connected_since: Option<Instant>,
    pub disconnected_since: Option<Instant>,
    pub encounter: Option<EncounterSummary>,
    pub rows: Vec<CombatantRow>,
    pub decoration: Decoration,
    pub mode: ViewMode,
    pub idle_scene: IdleScene,
    pub settings: AppSettings,
    pub show_settings: bool,
    pub settings_cursor: SettingsField,
    pub history: HistoryPanel,
    pub history_settings: HistorySettingsPanel,
    pub archive_count: usize,
    pub show_idle_overlay: bool,
    pub error: Option<AppError>,
    pub dungeon_active_zone: Option<String>,
    pub lb_summary: Option<LimitBreakSummary>,
    pub history_list_state: ratatui::widgets::ListState,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            connected: false,
            last_update: None,
            last_active: None,
            connected_since: None,
            disconnected_since: None,
            encounter: None,
            rows: Vec::new(),
            decoration: Decoration::default(),
            mode: ViewMode::default(),
            idle_scene: IdleScene::default(),
            settings: AppSettings::default(),
            show_settings: false,
            settings_cursor: SettingsField::default(),
            history: HistoryPanel::default(),
            history_settings: HistorySettingsPanel::default(),
            archive_count: 0,
            show_idle_overlay: true,
            error: None,
            dungeon_active_zone: None,
            lb_summary: None,
            history_list_state: ratatui::widgets::ListState::default(),
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
                self.disconnected_since = None;
            }
            AppEvent::Disconnected => {
                self.connected = false;
                let now = Instant::now();
                self.last_update = None;
                self.last_active = None;
                // Reset disconnected_since if we were previously connected (to restart idle timer)
                // Otherwise preserve it if already set (preserves initial startup time)
                let was_connected = self.connected_since.is_some();
                self.connected_since = None;
                if was_connected {
                    // We were connected, so reset the idle timer
                    self.disconnected_since = Some(now);
                } else if self.disconnected_since.is_none() {
                    // We were never connected and don't have a timestamp yet, set it now
                    self.disconnected_since = Some(now);
                }
                // Otherwise, keep the existing disconnected_since (preserves startup time)
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
            AppEvent::LimitBreakUpdate { summary } => {
                self.lb_summary = summary;
            }
            AppEvent::HistoryDatesLoaded { days } => {
                self.history.finish_load();
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
                self.history.finish_load();
            }
            AppEvent::HistoryEncounterLoaded { key, record } => {
                if let Some(item) = self.history.find_encounter_mut(&key) {
                    item.record = Some(record);
                }
                self.history.finish_load();
            }
            AppEvent::DungeonDatesLoaded { days } => {
                self.history.dungeon_days = days;
                if self.history.dungeon_selected_day >= self.history.dungeon_days.len() {
                    self.history.dungeon_selected_day = 0;
                }
                self.history.dungeon_selected_run = 0;
                self.history.dungeon_selected_child = 0;
                self.history.finish_load();
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
                self.history.finish_load();
            }
            AppEvent::DungeonRunLoaded { key, record } => {
                if let Some(run) = self.history.find_dungeon_run_mut(&key) {
                    let child_count = record.child_keys.len();
                    run.record = Some(record);
                    run.child_records = vec![None; child_count];
                }
                self.history.finish_load();
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
                self.history.finish_load();
            }
            AppEvent::DungeonSessionUpdate { active_zone } => {
                self.dungeon_active_zone = active_zone;
            }
            AppEvent::HistoryError { message } => {
                self.history.finish_load();
                self.history.error = Some(message);
            }
            AppEvent::HistoryDeleted {
                action,
                deleted_encounter_keys,
            } => {
                self.history.finish_load();
                self.history.error = None;
                self.history.apply_delete(&action, &deleted_encounter_keys);
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
            history_settings: self.history_settings.clone(),
            archive_count: self.archive_count,
            show_idle_overlay: self.show_idle_overlay,
            error: self.error.clone(),
            dungeon_active_zone: self.dungeon_active_zone.clone(),
            lb_summary: self.lb_summary.clone(),
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
        let Some(threshold) = self.settings.idle_duration() else {
            return false;
        };

        if !self.connected {
            // When disconnected, check if we've been disconnected long enough
            if let Some(disconnected) = self.disconnected_since {
                return now.saturating_duration_since(disconnected) >= threshold;
            }
            // If we don't have a disconnected timestamp yet, we're not idle
            return false;
        }

        // When connected, check for active encounters
        if self
            .encounter
            .as_ref()
            .map(|enc| enc.is_active)
            .unwrap_or(false)
        {
            return false;
        }

        // Check time since last active encounter
        if let Some(active) = self.last_active {
            if now.saturating_duration_since(active) >= threshold {
                return true;
            }
            return false;
        }

        // Check time since connection
        if let Some(since) = self.connected_since {
            return now.saturating_duration_since(since) >= threshold;
        }

        false
    }

    pub fn apply_settings(&mut self, settings: AppSettings) {
        self.settings = settings;
        // Apply theme selection and role theme toggle.
        let effective_id = theme::apply_theme_by_id(&self.settings.theme_id);
        self.settings.theme_id = effective_id;
        theme::set_role_theme_enabled(self.settings.role_theme_enabled);
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
                self.settings.dungeon_mode_enabled = !self.settings.dungeon_mode_enabled;
                true
            }
            SettingsField::LimitBreakMode => {
                let before = self.settings.limit_break_mode;
                self.settings.limit_break_mode = if forward {
                    before.next()
                } else {
                    before.prev()
                };
                before != self.settings.limit_break_mode
            }
            SettingsField::Theme => {
                // Cycle through themes from the registry.
                let registry = theme::theme_registry();
                let descriptors = registry.descriptors();
                if descriptors.is_empty() {
                    return false;
                }
                let len = descriptors.len();
                let current_id = &self.settings.theme_id;
                let current_index = descriptors
                    .iter()
                    .position(|d| d.id.eq_ignore_ascii_case(current_id))
                    .unwrap_or(0);
                let next_index = if forward {
                    (current_index + 1) % len
                } else if current_index == 0 {
                    len.saturating_sub(1)
                } else {
                    current_index.saturating_sub(1)
                };
                drop(registry);
                let next_id = descriptors[next_index].id.clone();
                if next_id.eq_ignore_ascii_case(current_id) {
                    false
                } else {
                    let effective_id = theme::apply_theme_by_id(&next_id);
                    self.settings.theme_id = effective_id;
                    true
                }
            }
            SettingsField::RoleTheme => {
                let before = self.settings.role_theme_enabled;
                let after = !before;
                if after != before {
                    self.settings.role_theme_enabled = after;
                    theme::set_role_theme_enabled(after);
                    true
                } else {
                    false
                }
            }
            SettingsField::HistorySettings => false,
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
        if !self.settings.history_enabled && self.history.viewing_archive.is_none() {
            self.error = Some(crate::errors::AppError::new(
                crate::errors::AppErrorKind::History,
                "History is disabled".to_string(),
            ));
            return false;
        }
        if self.history.visible {
            self.history.visible = false;
            self.history.reset();
            false
        } else {
            self.history.visible = true;
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

    pub fn history_begin_load(&mut self) {
        self.history.begin_load();
    }

    pub fn history_move_selection(&mut self, delta: i32) {
        self.history.move_selection(delta);
    }

    pub fn history_toggle_mode(&mut self) {
        self.history.toggle_mode();
    }

    pub fn history_toggle_view(&mut self) {
        self.history.toggle_view();
    }

    pub fn history_enter(&mut self) {
        self.history.enter();
    }

    pub fn history_back(&mut self) {
        self.history.back();
    }

    pub fn history_begin_delete(&mut self) -> bool {
        self.history.begin_delete_confirm()
    }

    pub fn history_confirm_cycle(&mut self, delta: i32) {
        if let Some(confirm) = self.history.confirm.as_mut() {
            confirm.cycle_focus(delta);
        }
    }

    pub fn history_cancel_confirm(&mut self) {
        self.history.cancel_confirm();
    }

    pub fn history_take_confirm_action(&mut self) -> Option<crate::model::HistoryDeleteAction> {
        self.history.take_confirm_action()
    }
}
