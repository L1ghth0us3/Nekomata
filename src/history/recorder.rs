use std::sync::Arc;

use serde_json::Value;
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::task;

use crate::dungeon::DungeonCatalog;
use crate::errors::{AppError, AppErrorKind};
use crate::model::{AppEvent, CombatantRow, EncounterSummary};

use super::dungeon::{DungeonRecorder, DungeonRecorderUpdate, DungeonZoneState};
use super::store::HistoryStore;
use super::types::{DungeonAggregateRecord, EncounterFrame, EncounterRecord, EncounterSnapshot};
use super::util::{parse_duration_secs, parse_number};

pub struct RecorderHandle {
    inner: Arc<RecorderInner>,
}

struct RecorderInner {
    tx: mpsc::UnboundedSender<RecorderMessage>,
    shutdown: Mutex<Option<oneshot::Receiver<()>>>,
}

impl RecorderHandle {
    pub fn record(&self, snapshot: EncounterSnapshot) {
        let _ = self
            .inner
            .tx
            .send(RecorderMessage::Snapshot(Box::new(snapshot)));
    }

    pub fn record_components(
        &self,
        encounter: EncounterSummary,
        rows: Vec<CombatantRow>,
        raw: Value,
    ) {
        self.record(EncounterSnapshot::new(encounter, rows, raw));
    }

    pub fn flush(&self) {
        let _ = self.inner.tx.send(RecorderMessage::Flush);
    }

    pub fn set_dungeon_mode_enabled(&self, enabled: bool) {
        let _ = self.inner.tx.send(RecorderMessage::SetDungeonMode(enabled));
    }

    pub async fn shutdown(&self) {
        let _ = self.inner.tx.send(RecorderMessage::Shutdown);
        if let Some(rx) = self.take_shutdown_receiver().await {
            let _ = rx.await;
        }
    }

    async fn take_shutdown_receiver(&self) -> Option<oneshot::Receiver<()>> {
        let mut guard = self.inner.shutdown.lock().await;
        guard.take()
    }
}

impl Clone for RecorderHandle {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

enum RecorderMessage {
    Snapshot(Box<EncounterSnapshot>),
    Flush,
    SetDungeonMode(bool),
    Shutdown,
}

pub fn spawn_recorder(
    store: Arc<HistoryStore>,
    event_tx: mpsc::UnboundedSender<AppEvent>,
    dungeon_catalog: Option<Arc<DungeonCatalog>>,
    dungeon_mode_enabled: bool,
) -> RecorderHandle {
    let (tx, mut rx) = mpsc::unbounded_channel();
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    tokio::spawn(async move {
        let mut worker =
            RecorderWorker::new(store, event_tx, dungeon_catalog, dungeon_mode_enabled);
        loop {
            match rx.recv().await {
                Some(RecorderMessage::Snapshot(snapshot)) => worker.on_snapshot(*snapshot).await,
                Some(RecorderMessage::Flush) => worker.on_flush().await,
                Some(RecorderMessage::SetDungeonMode(enabled)) => {
                    worker.on_toggle_dungeon_mode(enabled).await;
                }
                Some(RecorderMessage::Shutdown) => {
                    worker.on_flush().await;
                    break;
                }
                None => {
                    worker.on_flush().await;
                    break;
                }
            }
        }
        let _ = shutdown_tx.send(());
    });
    RecorderHandle {
        inner: Arc::new(RecorderInner {
            tx,
            shutdown: Mutex::new(Some(shutdown_rx)),
        }),
    }
}

struct RecorderWorker {
    store: Arc<HistoryStore>,
    current: Option<ActiveEncounter>,
    events: mpsc::UnboundedSender<AppEvent>,
    dungeon: DungeonRecorder,
}

impl RecorderWorker {
    fn new(
        store: Arc<HistoryStore>,
        events: mpsc::UnboundedSender<AppEvent>,
        dungeon_catalog: Option<Arc<DungeonCatalog>>,
        dungeon_mode_enabled: bool,
    ) -> Self {
        Self {
            store,
            current: None,
            events,
            dungeon: DungeonRecorder::new(dungeon_catalog, dungeon_mode_enabled),
        }
    }

    async fn on_snapshot(&mut self, snapshot: EncounterSnapshot) {
        if self.current.is_none() {
            if !snapshot.encounter.is_active {
                return;
            }
            if !snapshot_has_activity(&snapshot) {
                return;
            }
        }

        if let Some(active) = self.current.as_ref() {
            if should_rollover(active, &snapshot) {
                self.flush_active().await;
            }
        }

        if let Some(active) = self.current.as_mut() {
            active.update(snapshot);
        } else {
            self.current = Some(ActiveEncounter::from_snapshot(snapshot));
        }

        if let Some(active) = self.current.as_ref() {
            if !active.latest_summary.is_active {
                self.flush_active().await;
            }
        }
    }

    async fn on_flush(&mut self) {
        self.flush_active().await;
        let update = self.dungeon.flush(true);
        self.handle_dungeon_update(update).await;
    }

    async fn on_toggle_dungeon_mode(&mut self, enabled: bool) {
        let update = self.dungeon.set_enabled(enabled);
        self.handle_dungeon_update(update).await;
    }

    async fn handle_dungeon_update(&mut self, update: DungeonRecorderUpdate) {
        for aggregate in update.aggregates {
            self.persist_dungeon_record(aggregate).await;
        }
        if let Some(zone_state) = update.zone_state {
            match zone_state {
                DungeonZoneState::Active(zone) => {
                    let _ = self.events.send(AppEvent::DungeonSessionUpdate {
                        active_zone: Some(zone),
                    });
                }
                DungeonZoneState::Inactive => {
                    let _ = self
                        .events
                        .send(AppEvent::DungeonSessionUpdate { active_zone: None });
                }
            }
        }
    }

    async fn flush_active(&mut self) {
        if let Some(active) = self.current.take() {
            let store = Arc::clone(&self.store);
            let record = EncounterRecord::from_active(active);
            if !record.saw_active && record.rows.is_empty() {
                return;
            }
            match task::spawn_blocking(move || store.append(&record).map(|key| (key, record))).await
            {
                Ok(Ok((key, record))) => {
                    let key_bytes = key.as_bytes();
                    let update = self.dungeon.on_encounter(&record, key_bytes);
                    self.handle_dungeon_update(update).await;
                }
                Ok(Err(err)) => {
                    let message = format!("Failed to persist encounter history: {err}");
                    Self::report_error(&self.events, message, AppErrorKind::Storage);
                }
                Err(err) => {
                    let message = format!("History recorder task join error: {err}");
                    Self::report_error(&self.events, message, AppErrorKind::History);
                }
            }
        }
    }

    async fn persist_dungeon_record(&self, record: DungeonAggregateRecord) {
        let store = Arc::clone(&self.store);
        match task::spawn_blocking(move || store.append_dungeon(&record)).await {
            Ok(Ok(_)) => {}
            Ok(Err(err)) => {
                let message = format!("Failed to persist dungeon aggregate: {err}");
                Self::report_error(&self.events, message, AppErrorKind::Storage);
            }
            Err(err) => {
                let message = format!("Dungeon recorder task join error: {err}");
                Self::report_error(&self.events, message, AppErrorKind::History);
            }
        }
    }

    fn report_error(events: &mpsc::UnboundedSender<AppEvent>, message: String, kind: AppErrorKind) {
        let error = AppError::new(kind, message);
        let _ = events.send(AppEvent::SystemError { error });
    }
}

#[derive(Debug)]
struct ActiveEncounter {
    first_seen_ms: u64,
    last_seen_ms: u64,
    latest_summary: EncounterSummary,
    latest_rows: Vec<CombatantRow>,
    last_raw: Value,
    saw_active: bool,
    frames: Vec<EncounterFrame>,
}

impl ActiveEncounter {
    fn from_snapshot(snapshot: EncounterSnapshot) -> Self {
        let EncounterSnapshot {
            encounter,
            rows,
            raw,
            received_ms,
        } = snapshot;
        let is_active = encounter.is_active;
        let frame = EncounterFrame::new(received_ms, encounter.clone(), rows.clone(), raw.clone());
        Self {
            first_seen_ms: received_ms,
            last_seen_ms: received_ms,
            latest_summary: encounter,
            latest_rows: rows,
            last_raw: raw,
            saw_active: is_active,
            frames: vec![frame],
        }
    }

    fn update(&mut self, snapshot: EncounterSnapshot) {
        self.last_seen_ms = snapshot.received_ms;
        let EncounterSnapshot {
            encounter,
            rows,
            raw,
            received_ms,
        } = snapshot;
        let frame = EncounterFrame::new(received_ms, encounter.clone(), rows.clone(), raw.clone());
        self.latest_summary = encounter;
        self.latest_rows = rows;
        self.last_raw = raw;
        self.frames.push(frame);
        self.saw_active |= self.latest_summary.is_active;
    }
}

impl EncounterRecord {
    fn from_active(active: ActiveEncounter) -> Self {
        let ActiveEncounter {
            first_seen_ms,
            last_seen_ms,
            latest_summary,
            latest_rows,
            last_raw,
            saw_active,
            frames,
        } = active;
        let snapshots = frames.len() as u32;
        let raw_last = if let Some(frame) = frames.last() {
            Some(frame.raw.clone())
        } else {
            Some(last_raw)
        };

        Self {
            version: super::types::SCHEMA_VERSION,
            stored_ms: super::types::now_ms(),
            first_seen_ms,
            last_seen_ms,
            encounter: latest_summary,
            rows: latest_rows,
            raw_last,
            snapshots,
            saw_active,
            frames,
        }
    }
}

impl EncounterFrame {
    fn new(
        received_ms: u64,
        encounter: EncounterSummary,
        rows: Vec<CombatantRow>,
        raw: Value,
    ) -> Self {
        Self {
            received_ms,
            encounter,
            rows,
            raw,
        }
    }
}

fn should_rollover(active: &ActiveEncounter, incoming: &EncounterSnapshot) -> bool {
    let previous = &active.latest_summary;
    let next = &incoming.encounter;

    if next.is_active {
        if !active.saw_active {
            return true;
        }

        if let (Some(prev_secs), Some(next_secs)) = (
            parse_duration_secs(&previous.duration),
            parse_duration_secs(&next.duration),
        ) {
            if next_secs + 2 < prev_secs {
                return true;
            }
            if prev_secs > 10 && next_secs == 0 {
                return true;
            }
        }

        let prev_damage = parse_number(&previous.damage);
        let next_damage = parse_number(&next.damage);
        if next_damage + 1.0 < prev_damage {
            return true;
        }
    }

    false
}

fn snapshot_has_activity(snapshot: &EncounterSnapshot) -> bool {
    if snapshot.encounter.is_active {
        return true;
    }
    if parse_number(&snapshot.encounter.damage) > 0.0
        || parse_number(&snapshot.encounter.healed) > 0.0
        || parse_number(&snapshot.encounter.encdps) > 0.0
        || parse_number(&snapshot.encounter.enchps) > 0.0
    {
        return true;
    }
    snapshot
        .rows
        .iter()
        .any(|row| row.damage > 0.0 || row.healed > 0.0 || row.encdps > 0.0 || row.enchps > 0.0)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use serde_json::json;
    use tokio::sync::mpsc;

    use crate::dungeon::DungeonCatalog;
    use crate::history::types::now_ms;
    use crate::history::util::parse_number;

    use super::*;

    fn build_snapshot(active: bool, duration: &str, damage: &str) -> EncounterSnapshot {
        let encounter = EncounterSummary {
            title: "Test Encounter".into(),
            zone: "Test Zone".into(),
            duration: duration.into(),
            encdps: "1000".into(),
            damage: damage.into(),
            enchps: "0".into(),
            healed: "0".into(),
            is_active: active,
        };
        let row = CombatantRow {
            name: "Alice".into(),
            job: "NIN".into(),
            encdps: 1000.0,
            encdps_str: "1000".into(),
            damage: 1000.0,
            damage_str: damage.into(),
            share: 1.0,
            share_str: "100%".into(),
            enchps: 0.0,
            enchps_str: "0".into(),
            healed: 0.0,
            healed_str: "0".into(),
            heal_share: 0.0,
            heal_share_str: "0%".into(),
            overheal_pct: "0".into(),
            crit: "0".into(),
            dh: "0".into(),
            deaths: "0".into(),
        };
        EncounterSnapshot::new(encounter, vec![row], json!({ "type": "CombatData" }))
    }

    #[test]
    fn rollover_detects_duration_reset() {
        let active = ActiveEncounter::from_snapshot(build_snapshot(true, "01:20", "5000"));
        let incoming = build_snapshot(true, "00:05", "100");
        assert!(should_rollover(&active, &incoming));
    }

    #[test]
    fn rollover_ignores_inactive_duration_reset() {
        let active = ActiveEncounter::from_snapshot(build_snapshot(true, "01:20", "5000"));
        let incoming = build_snapshot(false, "00:00", "5000");
        assert!(!should_rollover(&active, &incoming));
    }

    #[test]
    fn rollover_ignores_title_change_mid_fight() {
        let active = ActiveEncounter::from_snapshot(build_snapshot(true, "01:20", "5000"));
        let mut incoming = build_snapshot(true, "01:21", "5200");
        incoming.encounter.title = "Renamed Encounter".into();
        incoming.encounter.zone = "Updated Zone".into();
        assert!(!should_rollover(&active, &incoming));
    }

    #[test]
    fn encounter_record_preserves_all_frames() {
        let mut active = ActiveEncounter::from_snapshot(build_snapshot(true, "00:01", "100"));
        active.update(build_snapshot(true, "00:02", "200"));
        active.update(build_snapshot(false, "00:02", "200"));
        let record = EncounterRecord::from_active(active);
        assert_eq!(record.snapshots, 3);
        assert_eq!(record.frames.len(), 3);
        assert!(record.frames.first().unwrap().encounter.is_active);
        assert!(!record.frames.last().unwrap().encounter.is_active);
    }

    #[test]
    fn snapshot_activity_detects_idle_state() {
        let idle = EncounterSnapshot::new(
            EncounterSummary {
                title: String::new(),
                zone: String::new(),
                duration: "00:00".into(),
                encdps: "0".into(),
                damage: "0".into(),
                enchps: "0".into(),
                healed: "0".into(),
                is_active: false,
            },
            Vec::new(),
            json!({ "type": "CombatData" }),
        );
        assert!(!snapshot_has_activity(&idle));

        let mut tick = idle.clone();
        tick.encounter.encdps = "15".into();
        assert!(snapshot_has_activity(&tick));
    }

    #[test]
    fn parse_number_handles_commas_and_percent() {
        assert_eq!(parse_number("12,345.6"), 12345.6);
        assert_eq!(parse_number("98%"), 98.0);
    }

    #[tokio::test]
    async fn recorder_aggregates_dungeon_runs_end_to_end() {
        let base = std::env::temp_dir().join(format!("nekomata-test-{}", now_ms()));
        std::fs::create_dir_all(&base).expect("create temp history dir");
        let db_path = base.join("encounters.sled");
        let store = Arc::new(HistoryStore::open(&db_path).expect("open history"));

        let (tx, _rx) = mpsc::unbounded_channel();
        let catalog = DungeonCatalog::from_str(r#"{ "dungeons": { "Sastasha": {} } }"#)
            .expect("catalog parse");
        let mut worker = RecorderWorker::new(store.clone(), tx, Some(Arc::new(catalog)), true);

        fn snapshot(
            zone: &str,
            title: &str,
            duration: &str,
            damage: &str,
            healed: &str,
            encdps: &str,
            enchps: &str,
            active: bool,
        ) -> EncounterSnapshot {
            let encounter = EncounterSummary {
                title: title.to_string(),
                zone: zone.to_string(),
                duration: duration.to_string(),
                encdps: encdps.to_string(),
                damage: damage.to_string(),
                enchps: enchps.to_string(),
                healed: healed.to_string(),
                is_active: active,
            };
            let row = CombatantRow {
                name: "Alice".into(),
                job: "NIN".into(),
                encdps: encdps.parse().unwrap_or(0.0),
                encdps_str: encdps.to_string(),
                damage: damage.replace(',', "").parse().unwrap_or(0.0),
                damage_str: damage.to_string(),
                share: 1.0,
                share_str: "100%".into(),
                enchps: enchps.parse().unwrap_or(0.0),
                enchps_str: enchps.to_string(),
                healed: healed.replace(',', "").parse().unwrap_or(0.0),
                healed_str: healed.to_string(),
                heal_share: 1.0,
                heal_share_str: "100%".into(),
                overheal_pct: "0".into(),
                crit: "0".into(),
                dh: "0".into(),
                deaths: "0".into(),
            };
            EncounterSnapshot::new(encounter, vec![row], json!({ "type": "CombatData" }))
        }

        let snapshots = vec![
            snapshot(
                "Sastasha", "Pull 1", "00:30", "1,000", "200", "120", "50", true,
            ),
            snapshot(
                "Sastasha", "Pull 1", "00:30", "1,000", "200", "0", "0", false,
            ),
            snapshot(
                "Sastasha", "Pull 2", "00:45", "1,500", "250", "140", "60", true,
            ),
            snapshot(
                "Sastasha", "Pull 2", "00:45", "1,500", "250", "0", "0", false,
            ),
            snapshot(
                "Middle La Noscea",
                "Overworld",
                "00:15",
                "200",
                "0",
                "20",
                "0",
                true,
            ),
            snapshot(
                "Middle La Noscea",
                "Overworld",
                "00:15",
                "200",
                "0",
                "0",
                "0",
                false,
            ),
        ];

        for snap in snapshots {
            worker.on_snapshot(snap).await;
        }

        worker.on_flush().await;

        let days = store.load_dungeon_days().expect("load days");
        assert_eq!(days.len(), 1);
        let day = &days[0];
        assert_eq!(day.run_count, 1);

        let runs = store
            .load_dungeon_summaries(&day.iso_date)
            .expect("load summaries");
        assert_eq!(runs.len(), 1);
        let run = &runs[0];
        assert_eq!(run.child_count, 2);
        assert!(!run.incomplete);

        let aggregate = store.load_dungeon_record(&run.key).expect("load aggregate");
        assert_eq!(aggregate.zone, "Sastasha");
        assert_eq!(aggregate.child_keys.len(), 2);
        assert!(!aggregate.incomplete);
        assert!((aggregate.total_damage - 2500.0).abs() < f64::EPSILON);
        assert!((aggregate.total_healed - 450.0).abs() < f64::EPSILON);

        drop(worker);
        drop(store);
        let _ = std::fs::remove_dir_all(&base);
    }
}
