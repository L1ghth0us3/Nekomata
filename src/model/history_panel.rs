use serde::{Deserialize, Serialize};

use crate::history::{DungeonHistoryDay, DungeonHistoryItem, HistoryDay, HistoryEncounterItem};

use super::ViewMode;

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
        if !self.visible || self.loading {
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
        if !self.visible || self.loading {
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
        if !self.visible {
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
        if !self.visible || self.loading {
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
        if !self.visible {
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
}
