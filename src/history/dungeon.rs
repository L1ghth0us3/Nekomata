use std::sync::Arc;

use crate::dungeon::DungeonCatalog;
use crate::history::types::{DungeonAggregateRecord, EncounterRecord, SCHEMA_VERSION};
use crate::history::util::{parse_duration_secs, parse_number, party_signature, resolve_title};

#[derive(Debug, Clone)]
pub enum DungeonZoneState {
    Active(String),
    Inactive,
}

#[derive(Debug, Default)]
pub struct DungeonRecorderUpdate {
    pub aggregates: Vec<DungeonAggregateRecord>,
    pub zone_state: Option<DungeonZoneState>,
}

pub struct DungeonRecorder {
    catalog: Option<Arc<DungeonCatalog>>,
    enabled: bool,
    session: Option<DungeonSession>,
}

impl DungeonRecorder {
    pub fn new(catalog: Option<Arc<DungeonCatalog>>, enabled: bool) -> Self {
        let has_catalog = catalog.is_some();
        Self {
            catalog,
            enabled: enabled && has_catalog,
            session: None,
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) -> DungeonRecorderUpdate {
        let mut update = DungeonRecorderUpdate::default();
        let effective = enabled && self.catalog.is_some();
        if !effective {
            if let Some(aggregate) = self.end_session(true) {
                update.aggregates.push(aggregate);
                update.zone_state = Some(DungeonZoneState::Inactive);
            }
        }
        self.enabled = effective;
        update
    }

    pub fn on_encounter(
        &mut self,
        record: &EncounterRecord,
        key: Vec<u8>,
    ) -> DungeonRecorderUpdate {
        let mut update = DungeonRecorderUpdate::default();

        if !self.enabled {
            return update;
        }

        let catalog = match &self.catalog {
            Some(catalog) => catalog,
            None => return update,
        };

        let zone = record.encounter.zone.as_str();
        let Some(canonical_zone) = catalog.canonical_zone(zone) else {
            if self.session.is_some() {
                if let Some(aggregate) = self.end_session(false) {
                    update.zone_state = Some(DungeonZoneState::Inactive);
                    update.aggregates.push(aggregate);
                }
            }
            return update;
        };
        let canonical_zone = canonical_zone.to_string();

        if let Some(session) = self.session.as_mut() {
            if session.zone != canonical_zone {
                if let Some(aggregate) = self.end_session(false) {
                    update.aggregates.push(aggregate);
                }
                update.zone_state = Some(DungeonZoneState::Active(canonical_zone.clone()));
                self.session = Some(DungeonSession::new(canonical_zone, record, key));
            } else {
                session.append(record, key);
            }
        } else {
            update.zone_state = Some(DungeonZoneState::Active(canonical_zone.clone()));
            self.session = Some(DungeonSession::new(canonical_zone, record, key));
        }

        update
    }

    pub fn flush(&mut self, incomplete: bool) -> DungeonRecorderUpdate {
        let mut update = DungeonRecorderUpdate::default();
        if let Some(aggregate) = self.end_session(incomplete) {
            update.zone_state = Some(DungeonZoneState::Inactive);
            update.aggregates.push(aggregate);
        }
        update
    }

    fn end_session(&mut self, incomplete: bool) -> Option<DungeonAggregateRecord> {
        let session = self.session.take()?;
        Some(session.into_record(incomplete))
    }
}

struct DungeonSession {
    zone: String,
    started_ms: u64,
    last_seen_ms: u64,
    party_signature: Vec<String>,
    total_duration_secs: u64,
    total_damage: f64,
    total_healed: f64,
    child_keys: Vec<Vec<u8>>,
    child_titles: Vec<String>,
}

impl DungeonSession {
    fn new(zone: String, record: &EncounterRecord, key: Vec<u8>) -> Self {
        let mut session = Self {
            zone,
            started_ms: record.first_seen_ms,
            last_seen_ms: record.last_seen_ms,
            party_signature: party_signature(&record.rows),
            total_duration_secs: 0,
            total_damage: 0.0,
            total_healed: 0.0,
            child_keys: Vec::new(),
            child_titles: Vec::new(),
        };
        session.append(record, key);
        session
    }

    fn append(&mut self, record: &EncounterRecord, key: Vec<u8>) {
        self.last_seen_ms = record.last_seen_ms;
        self.child_keys.push(key);
        self.child_titles.push(resolve_title(record));
        if let Some(duration) = parse_duration_secs(&record.encounter.duration) {
            self.total_duration_secs = self.total_duration_secs.saturating_add(duration);
        }
        self.total_damage += parse_number(&record.encounter.damage);
        self.total_healed += parse_number(&record.encounter.healed);
    }

    fn into_record(mut self, incomplete: bool) -> DungeonAggregateRecord {
        // Avoid duplicates if all child encounters shared the same key somehow
        dedup_keys(&mut self.child_keys, &mut self.child_titles);
        let total_encdps = if self.total_duration_secs > 0 {
            self.total_damage / self.total_duration_secs as f64
        } else {
            0.0
        };

        DungeonAggregateRecord {
            version: SCHEMA_VERSION,
            zone: self.zone,
            started_ms: self.started_ms,
            last_seen_ms: self.last_seen_ms,
            party_signature: self.party_signature,
            total_duration_secs: self.total_duration_secs,
            total_damage: self.total_damage,
            total_healed: self.total_healed,
            total_encdps,
            child_keys: self.child_keys,
            child_titles: self.child_titles,
            incomplete,
        }
    }
}

fn dedup_keys(keys: &mut Vec<Vec<u8>>, titles: &mut Vec<String>) {
    let mut seen = Vec::new();
    let mut filtered_keys = Vec::with_capacity(keys.len());
    let mut filtered_titles = Vec::with_capacity(titles.len());
    for (idx, key) in keys.iter().enumerate() {
        if seen.iter().any(|existing: &Vec<u8>| existing == key) {
            continue;
        }
        seen.push(key.clone());
        filtered_keys.push(key.clone());
        if let Some(title) = titles.get(idx) {
            filtered_titles.push(title.clone());
        }
    }
    *keys = filtered_keys;
    *titles = filtered_titles;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::types::{now_ms, EncounterRecord};
    use crate::model::{CombatantRow, EncounterSummary};

    fn make_record(
        zone: &str,
        title: &str,
        duration: &str,
        damage: &str,
        healed: &str,
    ) -> EncounterRecord {
        EncounterRecord {
            version: SCHEMA_VERSION,
            stored_ms: now_ms(),
            first_seen_ms: 100,
            last_seen_ms: 200,
            encounter: EncounterSummary {
                title: title.to_string(),
                zone: zone.to_string(),
                duration: duration.to_string(),
                encdps: String::new(),
                damage: damage.to_string(),
                enchps: String::new(),
                healed: healed.to_string(),
                is_active: false,
            },
            rows: vec![CombatantRow {
                name: "Alice".into(),
                job: "NIN".into(),
                ..Default::default()
            }],
            raw_last: None,
            snapshots: 1,
            saw_active: true,
            frames: Vec::new(),
        }
    }

    fn build_catalog() -> Arc<DungeonCatalog> {
        let catalog = DungeonCatalog::from_str(
            r#"{ "dungeons": { "Sastasha": {}, "Copperbell Mines": {} } }"#,
        )
        .expect("catalog parse");
        Arc::new(catalog)
    }

    #[test]
    fn recorder_starts_and_updates_session() {
        let catalog = Some(build_catalog());
        let mut recorder = DungeonRecorder::new(catalog, true);
        let first = make_record("Sastasha", "Pull 1", "00:30", "10000", "0");
        let update = recorder.on_encounter(&first, vec![1]);
        assert!(update.aggregates.is_empty());
        assert!(matches!(
            update.zone_state,
            Some(DungeonZoneState::Active(_))
        ));

        let second = make_record("Sastasha", "Pull 2", "00:45", "15000", "0");
        let update = recorder.on_encounter(&second, vec![2]);
        assert!(update.aggregates.is_empty());
        assert!(update.zone_state.is_none());

        let flush = recorder.flush(false);
        assert_eq!(flush.aggregates.len(), 1);
        assert!(matches!(flush.zone_state, Some(DungeonZoneState::Inactive)));
        let agg = flush.aggregates.first().unwrap();
        assert_eq!(agg.child_keys.len(), 2);
        assert_eq!(agg.total_duration_secs, 75);
        assert!((agg.total_damage - 25000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn recorder_handles_zone_change() {
        let catalog = Some(build_catalog());
        let mut recorder = DungeonRecorder::new(catalog, true);
        let first = make_record("Sastasha", "Pull 1", "00:10", "1000", "0");
        recorder.on_encounter(&first, vec![1]);
        let second = make_record("Copperbell Mines", "Pull 1", "00:20", "2000", "0");
        let update = recorder.on_encounter(&second, vec![2]);
        assert_eq!(update.aggregates.len(), 1);
        assert!(
            matches!(update.zone_state, Some(DungeonZoneState::Active(zone)) if zone == "Copperbell Mines")
        );
    }

    #[test]
    fn recorder_flushes_when_zone_not_whitelisted() {
        let catalog = Some(build_catalog());
        let mut recorder = DungeonRecorder::new(catalog, true);
        let first = make_record("Sastasha", "Pull 1", "00:30", "1000", "0");
        recorder.on_encounter(&first, vec![1]);

        let overworld = make_record("Middle La Noscea", "FATE", "00:15", "500", "0");
        let update = recorder.on_encounter(&overworld, vec![2]);
        assert_eq!(update.aggregates.len(), 1);
        assert!(matches!(update.zone_state, Some(DungeonZoneState::Inactive)));
        let aggregate = update.aggregates.first().expect("aggregate");
        assert_eq!(aggregate.zone, "Sastasha");
        assert_eq!(aggregate.child_keys.len(), 1);
    }

    #[test]
    fn recorder_disables_when_catalog_missing() {
        let mut recorder = DungeonRecorder::new(None, true);
        let record = make_record("Sastasha", "Pull 1", "00:30", "1000", "0");
        let update = recorder.on_encounter(&record, vec![1]);
        assert!(update.aggregates.is_empty());
        assert!(update.zone_state.is_none());
        let flush = recorder.flush(false);
        assert!(flush.aggregates.is_empty());
    }

    #[test]
    fn recorder_set_enabled_flushes_session() {
        let catalog = Some(build_catalog());
        let mut recorder = DungeonRecorder::new(catalog, true);
        let record = make_record("Sastasha", "Pull 1", "00:30", "1000", "0");
        recorder.on_encounter(&record, vec![1]);
        let update = recorder.set_enabled(false);
        assert_eq!(update.aggregates.len(), 1);
        assert!(matches!(
            update.zone_state,
            Some(DungeonZoneState::Inactive)
        ));
    }
}
