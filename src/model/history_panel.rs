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
    #[serde(default)]
    pub detail_mode: ViewMode,
    #[serde(default)]
    pub dungeon_detail_mode: ViewMode,
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
            detail_mode: ViewMode::Dps,
            dungeon_detail_mode: ViewMode::Dps,
        }
    }
}

impl HistoryPanel {
    pub fn reset(&mut self) {
        self.loading = false;
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
}
