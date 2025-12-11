use std::collections::HashSet;

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

use crate::errors::AppError;
use crate::history::{
    DungeonAggregateRecord, DungeonHistoryDay, DungeonHistoryItem, EncounterRecord, HistoryDay,
    HistoryEncounterItem,
};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EncounterSummary {
    pub title: String,
    pub zone: String,
    pub duration: String,
    pub encdps: String,
    pub damage: String,
    pub enchps: String,
    pub healed: String,
    pub is_active: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CombatantRow {
    pub name: String,
    pub job: String,
    pub encdps: f64,
    pub encdps_str: String,
    pub damage: f64,
    pub damage_str: String,
    pub share: f64,        // 0.0..=1.0
    pub share_str: String, // e.g., "23.4%"
    pub enchps: f64,
    pub enchps_str: String,
    pub healed: f64,
    pub healed_str: String,
    pub heal_share: f64,
    pub heal_share_str: String,
    pub overheal_pct: String,
    pub crit: String,
    pub dh: String,
    pub deaths: String,
}

#[derive(Debug)]
pub enum AppEvent {
    Connected,
    Disconnected,
    CombatData {
        encounter: EncounterSummary,
        rows: Vec<CombatantRow>,
    },
    HistoryDatesLoaded {
        days: Vec<HistoryDay>,
    },
    HistoryEncountersLoaded {
        date_id: String,
        encounters: Vec<HistoryEncounterItem>,
    },
    HistoryEncounterLoaded {
        key: Vec<u8>,
        record: EncounterRecord,
    },
    DungeonDatesLoaded {
        days: Vec<DungeonHistoryDay>,
    },
    DungeonRunsLoaded {
        date_id: String,
        runs: Vec<DungeonHistoryItem>,
    },
    DungeonRunLoaded {
        key: Vec<u8>,
        record: DungeonAggregateRecord,
    },
    DungeonEncounterLoaded {
        key: Vec<u8>,
        record: EncounterRecord,
    },
    DungeonSessionUpdate {
        active_zone: Option<String>,
    },
    HistoryError {
        message: String,
    },
    SystemError {
        error: AppError,
    },
}

// Known job codes for party filtering and color mapping
pub fn known_jobs() -> &'static HashSet<&'static str> {
    static JOBS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
        [
            // Tanks
            "PLD", "WAR", "DRK", "GNB", 
            // Healers
            "WHM", "SCH", "AST", "SGE", 
            // Melee
            "MNK", "DRG", "NIN", "SAM", "RPR", "VPR", 
            // Ranged phys
            "BRD", "MCH", "DNC", 
            // Casters
            "BLM", "SMN", "RDM", "PCT", 
            // Limited
            "BLU",
            // Pre-Jobs
            "GLD", "PGL", "MRD", "LNC", "ARC", "CNJ", "THM", "ROG"
        ]
        .into_iter()
        .collect()
    });
    &JOBS
}
