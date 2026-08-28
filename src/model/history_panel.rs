use serde::{Deserialize, Serialize};

use crate::history::{DungeonHistoryDay, DungeonHistoryItem, HistoryDay, HistoryEncounterItem};

use super::ViewMode;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HistoryDeleteAction {
    Encounter {
        key: Vec<u8>,
        date_id: String,
    },
    EncounterDate {
        date_id: String,
    },
    DungeonRun {
        key: Vec<u8>,
        date_id: String,
        with_children: bool,
    },
    DungeonDate {
        date_id: String,
    },
}

#[derive(Clone, Debug)]
pub struct HistoryConfirm {
    pub message: String,
    pub options: Vec<(String, Option<HistoryDeleteAction>)>,
    pub focus: usize,
}

impl HistoryConfirm {
    pub fn cycle_focus(&mut self, delta: i32) {
        let len = self.options.len();
        if len == 0 {
            return;
        }
        let next = (self.focus as i32 + delta).rem_euclid(len as i32);
        self.focus = next as usize;
    }

    pub fn focused_action(&self) -> Option<&HistoryDeleteAction> {
        self.options.get(self.focus)?.1.as_ref()
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum HistoryPanelLevel {
    #[default]
    Dates,
    Encounters,
    EncounterDetail,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum HistoryView {
    #[default]
    Encounters,
    Dungeons,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum DungeonPanelLevel {
    #[default]
    Dates,
    Runs,
    RunDetail,
    EncounterDetail,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HistoryPanel {
    pub visible: bool,
    pub loading: bool,
    pub level: HistoryPanelLevel,
    #[serde(default)]
    pub view: HistoryView,
    pub days: Vec<HistoryDay>,
    pub selected_day: usize,
    pub selected_encounter: usize,
    #[serde(default)]
    pub dungeon_days: Vec<DungeonHistoryDay>,
    #[serde(default)]
    pub dungeon_level: DungeonPanelLevel,
    #[serde(default)]
    pub dungeon_selected_day: usize,
    #[serde(default)]
    pub dungeon_selected_run: usize,
    #[serde(default)]
    pub dungeon_selected_child: usize,
    pub error: Option<String>,
    #[serde(default, skip)]
    pub pending_loads: u32,
    #[serde(default, skip)]
    pub list_scroll_offset: usize,
    #[serde(default)]
    pub detail_mode: ViewMode,
    #[serde(default)]
    pub dungeon_detail_mode: ViewMode,
    #[serde(default, skip)]
    pub viewing_archive: Option<String>,
    #[serde(default, skip)]
    pub confirm: Option<HistoryConfirm>,
}

impl Default for HistoryPanel {
    fn default() -> Self {
        Self {
            visible: false,
            loading: false,
            level: HistoryPanelLevel::Dates,
            view: HistoryView::Encounters,
            days: Vec::new(),
            selected_day: 0,
            selected_encounter: 0,
            dungeon_days: Vec::new(),
            dungeon_level: DungeonPanelLevel::Dates,
            dungeon_selected_day: 0,
            dungeon_selected_run: 0,
            dungeon_selected_child: 0,
            error: None,
            pending_loads: 0,
            list_scroll_offset: 0,
            detail_mode: ViewMode::Dps,
            dungeon_detail_mode: ViewMode::Dps,
            viewing_archive: None,
            confirm: None,
        }
    }
}

impl HistoryPanel {
    pub fn reset(&mut self) {
        self.loading = false;
        self.pending_loads = 0;
        self.list_scroll_offset = 0;
        self.level = HistoryPanelLevel::Dates;
        self.dungeon_level = DungeonPanelLevel::Dates;
        self.selected_day = 0;
        self.selected_encounter = 0;
        self.dungeon_selected_day = 0;
        self.dungeon_selected_run = 0;
        self.dungeon_selected_child = 0;
        self.error = None;
        self.detail_mode = ViewMode::Dps;
        self.dungeon_detail_mode = ViewMode::Dps;
        self.viewing_archive = None;
        self.confirm = None;
        for day in &mut self.days {
            day.encounters.clear();
            day.encounters_loaded = false;
        }
        for day in &mut self.dungeon_days {
            day.runs.clear();
            day.runs_loaded = false;
        }
    }

    pub fn current_day(&self) -> Option<&HistoryDay> {
        self.days.get(self.selected_day)
    }

    pub fn current_encounter(&self) -> Option<&HistoryEncounterItem> {
        self.current_day()
            .and_then(|day| day.encounters.get(self.selected_encounter))
    }

    pub fn find_day_mut(&mut self, date_id: &str) -> Option<&mut HistoryDay> {
        self.days.iter_mut().find(|day| day.iso_date == date_id)
    }

    pub fn find_encounter_mut(&mut self, key: &[u8]) -> Option<&mut HistoryEncounterItem> {
        for day in &mut self.days {
            if let Some(item) = day.encounters.iter_mut().find(|item| item.key == key) {
                return Some(item);
            }
        }
        None
    }

    pub fn current_dungeon_day(&self) -> Option<&DungeonHistoryDay> {
        self.dungeon_days.get(self.dungeon_selected_day)
    }

    pub fn current_dungeon_run(&self) -> Option<&DungeonHistoryItem> {
        self.current_dungeon_day()
            .and_then(|day| day.runs.get(self.dungeon_selected_run))
    }

    pub fn find_dungeon_day_mut(&mut self, date_id: &str) -> Option<&mut DungeonHistoryDay> {
        self.dungeon_days
            .iter_mut()
            .find(|day| day.iso_date == date_id)
    }

    pub fn find_dungeon_run_mut(&mut self, key: &[u8]) -> Option<&mut DungeonHistoryItem> {
        for day in &mut self.dungeon_days {
            if let Some(run) = day.runs.iter_mut().find(|run| run.key == key) {
                return Some(run);
            }
        }
        None
    }

    pub fn begin_load(&mut self) {
        self.pending_loads = self.pending_loads.saturating_add(1);
        self.loading = true;
        self.error = None;
    }

    pub fn finish_load(&mut self) {
        self.pending_loads = self.pending_loads.saturating_sub(1);
        self.loading = self.pending_loads > 0;
    }

    pub fn move_selection(&mut self, delta: i32) {
        if !self.visible || self.loading || self.confirm.is_some() {
            return;
        }
        match self.view {
            HistoryView::Encounters => match self.level {
                HistoryPanelLevel::Dates => {
                    if self.days.is_empty() {
                        return;
                    }
                    let len = self.days.len() as i32;
                    let current = self.selected_day as i32;
                    let next = (current + delta).clamp(0, len - 1);
                    self.selected_day = next as usize;
                    if let Some(day) = self.current_day() {
                        if day.encounters.is_empty() {
                            self.selected_encounter = 0;
                        } else if self.selected_encounter >= day.encounters.len() {
                            self.selected_encounter = day.encounters.len() - 1;
                        }
                    }
                }
                HistoryPanelLevel::Encounters | HistoryPanelLevel::EncounterDetail => {
                    if let Some(day) = self.current_day() {
                        if day.encounters.is_empty() {
                            return;
                        }
                        let len = day.encounters.len() as i32;
                        let current = self.selected_encounter as i32;
                        let next = (current + delta).clamp(0, len - 1);
                        self.selected_encounter = next as usize;
                    }
                }
            },
            HistoryView::Dungeons => match self.dungeon_level {
                DungeonPanelLevel::Dates => {
                    if self.dungeon_days.is_empty() {
                        return;
                    }
                    let len = self.dungeon_days.len() as i32;
                    let current = self.dungeon_selected_day as i32;
                    let next = (current + delta).clamp(0, len - 1);
                    self.dungeon_selected_day = next as usize;
                    if let Some(day) = self.current_dungeon_day() {
                        if day.runs.is_empty() {
                            self.dungeon_selected_run = 0;
                        } else if self.dungeon_selected_run >= day.runs.len() {
                            self.dungeon_selected_run = day.runs.len() - 1;
                        }
                        self.dungeon_selected_child = 0;
                    }
                }
                DungeonPanelLevel::Runs => {
                    if let Some(day) = self.current_dungeon_day() {
                        if day.runs.is_empty() {
                            return;
                        }
                        let len = day.runs.len() as i32;
                        let current = self.dungeon_selected_run as i32;
                        let next = (current + delta).clamp(0, len - 1);
                        self.dungeon_selected_run = next as usize;
                        self.dungeon_selected_child = 0;
                    }
                }
                DungeonPanelLevel::RunDetail | DungeonPanelLevel::EncounterDetail => {
                    if let Some(run) = self.current_dungeon_run() {
                        let child_len = run
                            .record
                            .as_ref()
                            .map(|rec| rec.child_keys.len())
                            .unwrap_or(run.child_records.len());
                        if child_len == 0 {
                            return;
                        }
                        let len = child_len as i32;
                        let current = self.dungeon_selected_child as i32;
                        let next = (current + delta).clamp(0, len - 1);
                        self.dungeon_selected_child = next as usize;
                    }
                }
            },
        }
    }

    pub fn toggle_mode(&mut self) {
        if !self.visible || self.loading || self.confirm.is_some() {
            return;
        }
        match self.view {
            HistoryView::Encounters => {
                if self.level == HistoryPanelLevel::EncounterDetail {
                    self.detail_mode = self.detail_mode.next();
                }
            }
            HistoryView::Dungeons => match self.dungeon_level {
                DungeonPanelLevel::RunDetail => {
                    self.dungeon_detail_mode = self.dungeon_detail_mode.next();
                }
                DungeonPanelLevel::EncounterDetail => {
                    self.detail_mode = self.detail_mode.next();
                }
                _ => {}
            },
        }
    }

    pub fn toggle_view(&mut self) {
        if !self.visible || self.confirm.is_some() {
            return;
        }
        self.loading = false;
        self.pending_loads = 0;
        match self.view {
            HistoryView::Encounters => {
                self.view = HistoryView::Dungeons;
                self.dungeon_level = DungeonPanelLevel::Dates;
                self.error = None;
            }
            HistoryView::Dungeons => {
                self.view = HistoryView::Encounters;
                self.level = HistoryPanelLevel::Dates;
                self.error = None;
            }
        }
    }

    pub fn enter(&mut self) {
        if !self.visible || self.loading || self.confirm.is_some() {
            return;
        }
        match self.view {
            HistoryView::Encounters => match self.level {
                HistoryPanelLevel::Dates => {
                    if let Some(day) = self.current_day() {
                        if day.encounters_loaded {
                            if !day.encounters.is_empty() {
                                self.level = HistoryPanelLevel::Encounters;
                                self.selected_encounter = 0;
                            }
                        } else if !day.encounter_ids.is_empty() {
                            self.level = HistoryPanelLevel::Encounters;
                            self.selected_encounter = 0;
                        }
                    }
                }
                HistoryPanelLevel::Encounters => {
                    if self.current_encounter().is_some() {
                        self.level = HistoryPanelLevel::EncounterDetail;
                    }
                }
                HistoryPanelLevel::EncounterDetail => {}
            },
            HistoryView::Dungeons => match self.dungeon_level {
                DungeonPanelLevel::Dates => {
                    if let Some(day) = self.current_dungeon_day() {
                        if day.runs_loaded {
                            if !day.runs.is_empty() {
                                self.dungeon_level = DungeonPanelLevel::Runs;
                                self.dungeon_selected_run = 0;
                            }
                        } else if !day.run_ids.is_empty() {
                            self.dungeon_level = DungeonPanelLevel::Runs;
                            self.dungeon_selected_run = 0;
                        }
                    }
                }
                DungeonPanelLevel::Runs => {
                    if self.current_dungeon_run().is_some() {
                        self.dungeon_level = DungeonPanelLevel::RunDetail;
                        self.dungeon_selected_child = 0;
                    }
                }
                DungeonPanelLevel::RunDetail => {
                    if let Some(run) = self.current_dungeon_run() {
                        if let Some(record) = run.record.as_ref() {
                            if !record.child_keys.is_empty() {
                                self.dungeon_level = DungeonPanelLevel::EncounterDetail;
                                self.dungeon_selected_child = 0;
                            }
                        }
                    }
                }
                DungeonPanelLevel::EncounterDetail => {}
            },
        }
    }

    pub fn back(&mut self) {
        if !self.visible || self.confirm.is_some() {
            return;
        }
        match self.view {
            HistoryView::Encounters => match self.level {
                HistoryPanelLevel::EncounterDetail => {
                    self.level = HistoryPanelLevel::Encounters;
                }
                HistoryPanelLevel::Encounters => {
                    self.level = HistoryPanelLevel::Dates;
                    self.selected_encounter = 0;
                }
                HistoryPanelLevel::Dates => {}
            },
            HistoryView::Dungeons => match self.dungeon_level {
                DungeonPanelLevel::EncounterDetail => {
                    self.dungeon_level = DungeonPanelLevel::RunDetail;
                }
                DungeonPanelLevel::RunDetail => {
                    self.dungeon_level = DungeonPanelLevel::Runs;
                    self.dungeon_selected_child = 0;
                }
                DungeonPanelLevel::Runs => {
                    self.dungeon_level = DungeonPanelLevel::Dates;
                    self.dungeon_selected_run = 0;
                }
                DungeonPanelLevel::Dates => {}
            },
        }
    }

    pub fn can_delete_at_current_level(&self) -> bool {
        if !self.visible || self.loading || self.confirm.is_some() {
            return false;
        }
        match self.view {
            HistoryView::Encounters => match self.level {
                HistoryPanelLevel::Dates => self
                    .current_day()
                    .map(|day| !day.encounter_ids.is_empty())
                    .unwrap_or(false),
                HistoryPanelLevel::Encounters | HistoryPanelLevel::EncounterDetail => {
                    self.current_encounter().is_some()
                }
            },
            HistoryView::Dungeons => match self.dungeon_level {
                DungeonPanelLevel::Dates => self
                    .current_dungeon_day()
                    .map(|day| !day.run_ids.is_empty())
                    .unwrap_or(false),
                DungeonPanelLevel::Runs | DungeonPanelLevel::RunDetail => {
                    self.current_dungeon_run().is_some()
                }
                DungeonPanelLevel::EncounterDetail => false,
            },
        }
    }

    pub fn begin_delete_confirm(&mut self) -> bool {
        if !self.can_delete_at_current_level() {
            return false;
        }

        let confirm = match self.view {
            HistoryView::Encounters => match self.level {
                HistoryPanelLevel::Dates => {
                    let Some(day) = self.current_day() else {
                        return false;
                    };
                    let count = day.encounter_ids.len();
                    HistoryConfirm {
                        message: format!(
                            "Delete all {count} encounter{} on {}?",
                            if count == 1 { "" } else { "s" },
                            day.iso_date
                        ),
                        options: vec![
                            ("Cancel".into(), None),
                            (
                                "Delete".into(),
                                Some(HistoryDeleteAction::EncounterDate {
                                    date_id: day.iso_date.clone(),
                                }),
                            ),
                        ],
                        focus: 0,
                    }
                }
                HistoryPanelLevel::Encounters | HistoryPanelLevel::EncounterDetail => {
                    let Some(day) = self.current_day() else {
                        return false;
                    };
                    let Some(enc) = self.current_encounter() else {
                        return false;
                    };
                    HistoryConfirm {
                        message: format!("Delete encounter \"{}\"?", enc.display_title),
                        options: vec![
                            ("Cancel".into(), None),
                            (
                                "Delete".into(),
                                Some(HistoryDeleteAction::Encounter {
                                    key: enc.key.clone(),
                                    date_id: day.iso_date.clone(),
                                }),
                            ),
                        ],
                        focus: 0,
                    }
                }
            },
            HistoryView::Dungeons => match self.dungeon_level {
                DungeonPanelLevel::Dates => {
                    let Some(day) = self.current_dungeon_day() else {
                        return false;
                    };
                    let count = day.run_ids.len();
                    HistoryConfirm {
                        message: format!(
                            "Delete all {count} dungeon run{} on {}?",
                            if count == 1 { "" } else { "s" },
                            day.iso_date
                        ),
                        options: vec![
                            ("Cancel".into(), None),
                            (
                                "Delete".into(),
                                Some(HistoryDeleteAction::DungeonDate {
                                    date_id: day.iso_date.clone(),
                                }),
                            ),
                        ],
                        focus: 0,
                    }
                }
                DungeonPanelLevel::Runs | DungeonPanelLevel::RunDetail => {
                    let Some(day) = self.current_dungeon_day() else {
                        return false;
                    };
                    let Some(run) = self.current_dungeon_run() else {
                        return false;
                    };
                    let child_hint = if run.child_count > 0 {
                        format!(
                            "\n\nThis run includes {} child encounter{}.",
                            run.child_count,
                            if run.child_count == 1 { "" } else { "s" }
                        )
                    } else {
                        String::new()
                    };
                    HistoryConfirm {
                        message: format!("Delete dungeon run \"{}\"?{child_hint}", run.zone),
                        options: vec![
                            ("Cancel".into(), None),
                            (
                                "Delete run only".into(),
                                Some(HistoryDeleteAction::DungeonRun {
                                    key: run.key.clone(),
                                    date_id: day.iso_date.clone(),
                                    with_children: false,
                                }),
                            ),
                            (
                                "Delete run + encounters".into(),
                                Some(HistoryDeleteAction::DungeonRun {
                                    key: run.key.clone(),
                                    date_id: day.iso_date.clone(),
                                    with_children: true,
                                }),
                            ),
                        ],
                        focus: 0,
                    }
                }
                DungeonPanelLevel::EncounterDetail => return false,
            },
        };

        self.confirm = Some(confirm);
        true
    }

    pub fn cancel_confirm(&mut self) {
        self.confirm = None;
    }

    pub fn take_confirm_action(&mut self) -> Option<HistoryDeleteAction> {
        let confirm = self.confirm.as_ref()?;
        let action = confirm.focused_action()?.clone();
        self.confirm = None;
        Some(action)
    }

    pub fn apply_delete(
        &mut self,
        action: &HistoryDeleteAction,
        deleted_encounter_keys: &[Vec<u8>],
    ) {
        match action {
            HistoryDeleteAction::Encounter { key, date_id } => {
                self.remove_encounter_from_day(date_id, key);
            }
            HistoryDeleteAction::EncounterDate { date_id } => {
                self.remove_encounter_day(date_id);
            }
            HistoryDeleteAction::DungeonRun { key, date_id, .. } => {
                self.remove_dungeon_run_from_day(date_id, key);
                for child_key in deleted_encounter_keys {
                    self.purge_encounter_key(child_key);
                }
            }
            HistoryDeleteAction::DungeonDate { date_id } => {
                self.remove_dungeon_day(date_id);
            }
        }
    }

    fn remove_encounter_from_day(&mut self, date_id: &str, key: &[u8]) {
        let Some(day_idx) = self.days.iter().position(|day| day.iso_date == date_id) else {
            return;
        };
        let day = &mut self.days[day_idx];
        day.encounter_ids.retain(|existing| existing != key);
        if day.encounters_loaded {
            day.encounters.retain(|enc| enc.key != key);
        }
        day.encounter_count = day.encounter_ids.len();
        day.label = format_day_label(date_id, day.encounter_count);

        if day.encounter_ids.is_empty() {
            self.days.remove(day_idx);
            self.selected_day = self.selected_day.min(self.days.len().saturating_sub(1));
            if self.level != HistoryPanelLevel::Dates {
                self.level = HistoryPanelLevel::Dates;
                self.selected_encounter = 0;
            }
            return;
        }

        if self.selected_encounter >= day.encounter_ids.len() {
            self.selected_encounter = day.encounter_ids.len().saturating_sub(1);
        }
    }

    fn remove_encounter_day(&mut self, date_id: &str) {
        if let Some(idx) = self.days.iter().position(|day| day.iso_date == date_id) {
            self.days.remove(idx);
            self.selected_day = self.selected_day.min(self.days.len().saturating_sub(1));
            self.selected_encounter = 0;
            self.level = HistoryPanelLevel::Dates;
        }
    }

    fn remove_dungeon_run_from_day(&mut self, date_id: &str, key: &[u8]) {
        let Some(day_idx) = self
            .dungeon_days
            .iter()
            .position(|day| day.iso_date == date_id)
        else {
            return;
        };
        let day = &mut self.dungeon_days[day_idx];
        day.run_ids.retain(|existing| existing != key);
        if day.runs_loaded {
            day.runs.retain(|run| run.key != key);
        }
        day.run_count = day.run_ids.len();
        day.label = format_dungeon_day_label(date_id, day.run_count);

        if day.run_ids.is_empty() {
            self.dungeon_days.remove(day_idx);
            self.dungeon_selected_day = self
                .dungeon_selected_day
                .min(self.dungeon_days.len().saturating_sub(1));
            if self.dungeon_level != DungeonPanelLevel::Dates {
                self.dungeon_level = DungeonPanelLevel::Dates;
                self.dungeon_selected_run = 0;
            }
            return;
        }

        if self.dungeon_selected_run >= day.run_ids.len() {
            self.dungeon_selected_run = day.run_ids.len().saturating_sub(1);
        }
        if matches!(
            self.dungeon_level,
            DungeonPanelLevel::RunDetail | DungeonPanelLevel::EncounterDetail
        ) {
            self.dungeon_selected_child = 0;
        }
    }

    fn remove_dungeon_day(&mut self, date_id: &str) {
        if let Some(idx) = self
            .dungeon_days
            .iter()
            .position(|day| day.iso_date == date_id)
        {
            self.dungeon_days.remove(idx);
            self.dungeon_selected_day = self
                .dungeon_selected_day
                .min(self.dungeon_days.len().saturating_sub(1));
            self.dungeon_selected_run = 0;
            self.dungeon_selected_child = 0;
            self.dungeon_level = DungeonPanelLevel::Dates;
        }
    }

    fn purge_encounter_key(&mut self, key: &[u8]) {
        for day in &mut self.days {
            let had_key = day.encounter_ids.iter().any(|existing| existing == key);
            if !had_key {
                continue;
            }
            day.encounter_ids.retain(|existing| existing != key);
            if day.encounters_loaded {
                day.encounters.retain(|enc| enc.key != key);
            }
            day.encounter_count = day.encounter_ids.len();
            day.label = format_day_label(&day.iso_date, day.encounter_count);
        }
        self.days.retain(|day| !day.encounter_ids.is_empty());
        self.selected_day = self.selected_day.min(self.days.len().saturating_sub(1));
    }
}

fn format_day_label(iso_date: &str, encounter_count: usize) -> String {
    match chrono::NaiveDate::parse_from_str(iso_date, "%Y-%m-%d") {
        Ok(date) => {
            let weekday = date.format("%a");
            format!(
                "{} ({}) · {} encounters",
                iso_date, weekday, encounter_count
            )
        }
        Err(_) => format!("{} · {} encounters", iso_date, encounter_count),
    }
}

fn format_dungeon_day_label(iso_date: &str, run_count: usize) -> String {
    match chrono::NaiveDate::parse_from_str(iso_date, "%Y-%m-%d") {
        Ok(date) => {
            let weekday = date.format("%a");
            format!("{} ({}) · {} runs", iso_date, weekday, run_count)
        }
        Err(_) => format!("{} · {} runs", iso_date, run_count),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn begin_and_finish_load_balance_clears_loading_flag() {
        let mut panel = HistoryPanel::default();
        panel.begin_load();
        panel.begin_load();
        assert!(panel.loading);
        assert_eq!(panel.pending_loads, 2);

        panel.finish_load();
        assert!(panel.loading);
        assert_eq!(panel.pending_loads, 1);

        panel.finish_load();
        assert!(!panel.loading);
        assert_eq!(panel.pending_loads, 0);
    }

    #[test]
    fn begin_delete_confirm_opens_for_encounter_list() {
        let mut panel = HistoryPanel::default();
        panel.visible = true;
        panel.level = HistoryPanelLevel::Encounters;
        panel.days.push(HistoryDay {
            iso_date: "2025-01-01".into(),
            label: "2025-01-01".into(),
            encounter_count: 1,
            encounters: vec![HistoryEncounterItem {
                key: vec![1],
                display_title: "Fight".into(),
                base_title: "Fight".into(),
                occurrence: 1,
                time_label: "12:00".into(),
                last_seen_ms: 1,
                timestamp_label: "2025-01-01 12:00:00".into(),
                record: None,
            }],
            encounter_ids: vec![vec![1]],
            encounters_loaded: true,
        });

        assert!(panel.begin_delete_confirm());
        assert!(panel.confirm.is_some());
        let confirm = panel.confirm.as_ref().unwrap();
        assert_eq!(confirm.options.len(), 2);
    }

    #[test]
    fn apply_delete_removes_encounter_and_empty_day() {
        let mut panel = HistoryPanel::default();
        panel.level = HistoryPanelLevel::Encounters;
        panel.days.push(HistoryDay {
            iso_date: "2025-01-01".into(),
            label: "2025-01-01".into(),
            encounter_count: 1,
            encounters: vec![HistoryEncounterItem {
                key: vec![1],
                display_title: "Fight".into(),
                base_title: "Fight".into(),
                occurrence: 1,
                time_label: "12:00".into(),
                last_seen_ms: 1,
                timestamp_label: "2025-01-01 12:00:00".into(),
                record: None,
            }],
            encounter_ids: vec![vec![1]],
            encounters_loaded: true,
        });

        panel.apply_delete(
            &HistoryDeleteAction::Encounter {
                key: vec![1],
                date_id: "2025-01-01".into(),
            },
            &[],
        );

        assert!(panel.days.is_empty());
        assert_eq!(panel.level, HistoryPanelLevel::Dates);
    }

    #[test]
    fn apply_delete_from_detail_stays_on_next_encounter() {
        let mut panel = HistoryPanel::default();
        panel.level = HistoryPanelLevel::EncounterDetail;
        panel.selected_encounter = 0;
        panel.days.push(HistoryDay {
            iso_date: "2025-01-01".into(),
            label: "2025-01-01".into(),
            encounter_count: 2,
            encounters: vec![
                HistoryEncounterItem {
                    key: vec![1],
                    display_title: "Fight A".into(),
                    base_title: "Fight A".into(),
                    occurrence: 1,
                    time_label: "12:00".into(),
                    last_seen_ms: 1,
                    timestamp_label: "2025-01-01 12:00:00".into(),
                    record: None,
                },
                HistoryEncounterItem {
                    key: vec![2],
                    display_title: "Fight B".into(),
                    base_title: "Fight B".into(),
                    occurrence: 1,
                    time_label: "12:30".into(),
                    last_seen_ms: 2,
                    timestamp_label: "2025-01-01 12:30:00".into(),
                    record: None,
                },
            ],
            encounter_ids: vec![vec![1], vec![2]],
            encounters_loaded: true,
        });

        panel.apply_delete(
            &HistoryDeleteAction::Encounter {
                key: vec![1],
                date_id: "2025-01-01".into(),
            },
            &[],
        );

        assert_eq!(panel.level, HistoryPanelLevel::EncounterDetail);
        assert_eq!(panel.selected_encounter, 0);
        assert_eq!(panel.current_encounter().unwrap().key, vec![2]);
    }
}
