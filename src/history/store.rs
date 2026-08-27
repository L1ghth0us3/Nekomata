use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{DateTime, Local, NaiveDate, TimeZone};

use crate::config;

use super::types::{
    DateSummaryRecord, DungeonAggregateRecord, DungeonHistoryDay, DungeonHistoryItem,
    DungeonSummaryRecord, EncounterRecord, EncounterSummaryRecord, HistoryDay,
    HistoryEncounterItem, HistoryKey, DUNGEON_NAMESPACE, ENCOUNTER_NAMESPACE,
    META_SCHEMA_VERSION_KEY, SCHEMA_VERSION,
};
use super::retention::{
    cutoff_ms_for_days, format_oldest_date, HistoryLimitKind, HistoryRetentionPolicy,
    RetentionPlan,
};
use super::util::resolve_title;

/// Thin wrapper around the sled database.
pub struct HistoryStore {
    encounters: sled::Tree,
    encounter_summaries: sled::Tree,
    date_index: sled::Tree,
    dungeon_runs: sled::Tree,
    dungeon_summaries: sled::Tree,
    dungeon_dates: sled::Tree,
    meta: sled::Tree,
    db: sled::Db,
    root: PathBuf,
}

impl HistoryStore {
    pub const ENCOUNTERS_TREE: &'static str = "encounters";
    pub const ENCOUNTER_SUMMARIES_TREE: &'static str = "enc_summaries";
    pub const DATES_TREE: &'static str = "dates";
    pub const DUNGEON_RUNS_TREE: &'static str = "dungeons";
    pub const DUNGEON_SUMMARIES_TREE: &'static str = "dun_summaries";
    pub const DUNGEON_DATES_TREE: &'static str = "dun_dates";
    pub const META_TREE: &'static str = "meta";

    pub fn open(path: &Path) -> Result<Self> {
        let db = sled::open(path)
            .with_context(|| format!("Failed to open history database at {}", path.display()))?;
        let encounters = db
            .open_tree(Self::ENCOUNTERS_TREE)
            .context("Unable to open encounters history tree")?;
        let encounter_summaries = db
            .open_tree(Self::ENCOUNTER_SUMMARIES_TREE)
            .context("Unable to open encounter summaries history tree")?;
        let date_index = db
            .open_tree(Self::DATES_TREE)
            .context("Unable to open history date index tree")?;
        let dungeon_runs = db
            .open_tree(Self::DUNGEON_RUNS_TREE)
            .context("Unable to open dungeon aggregate tree")?;
        let dungeon_summaries = db
            .open_tree(Self::DUNGEON_SUMMARIES_TREE)
            .context("Unable to open dungeon summary tree")?;
        let dungeon_dates = db
            .open_tree(Self::DUNGEON_DATES_TREE)
            .context("Unable to open dungeon date index tree")?;
        let meta = db
            .open_tree(Self::META_TREE)
            .context("Unable to open history metadata tree")?;
        let store = Self {
            encounters,
            encounter_summaries,
            date_index,
            dungeon_runs,
            dungeon_summaries,
            dungeon_dates,
            meta,
            db,
            root: path.to_path_buf(),
        };
        store.init_schema()?;
        Ok(store)
    }

    pub fn open_default() -> Result<Self> {
        let path = config::history_db_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Unable to create history directory {}", parent.display())
            })?;
        }
        Self::open(&path)
    }

    pub fn append(&self, record: &EncounterRecord) -> Result<HistoryKey> {
        let timestamp = record.last_seen_ms;
        let discriminator = self
            .db
            .generate_id()
            .context("Failed to generate sled identifier for encounter key")?;
        let key = HistoryKey::new(ENCOUNTER_NAMESPACE, timestamp, discriminator);
        let key_bytes = key.as_bytes();
        let bytes = serde_cbor::to_vec(record).context("Failed to serialize encounter record")?;
        self.encounters
            .insert(key_bytes.as_slice(), bytes)
            .context("Failed to persist encounter record")?;

        let summary = self.build_encounter_summary(&key_bytes, record);
        let summary_bytes =
            serde_cbor::to_vec(&summary).context("Failed to serialize encounter summary")?;
        self.encounter_summaries
            .insert(key_bytes.as_slice(), summary_bytes)
            .context("Failed to persist encounter summary")?;

        self.update_date_summary(&summary)
            .context("Failed to update date summary")?;
        Ok(key)
    }

    pub fn append_dungeon(&self, record: &DungeonAggregateRecord) -> Result<HistoryKey> {
        let timestamp = record.last_seen_ms;
        let discriminator = self
            .db
            .generate_id()
            .context("Failed to generate sled identifier for dungeon key")?;
        let key = HistoryKey::new(DUNGEON_NAMESPACE, timestamp, discriminator);
        let key_bytes = key.as_bytes();
        let bytes =
            serde_cbor::to_vec(record).context("Failed to serialize dungeon aggregate record")?;
        self.dungeon_runs
            .insert(key_bytes.as_slice(), bytes)
            .context("Failed to persist dungeon aggregate record")?;

        let summary = self.build_dungeon_summary(&key_bytes, record);
        let summary_bytes =
            serde_cbor::to_vec(&summary).context("Failed to serialize dungeon summary record")?;
        self.dungeon_summaries
            .insert(key_bytes.as_slice(), summary_bytes)
            .context("Failed to persist dungeon summary")?;

        self.update_dungeon_date_summary(&summary)
            .context("Failed to update dungeon date summary")?;

        Ok(key)
    }

    #[allow(dead_code)]
    pub fn remove(&self, key: &HistoryKey) -> Result<()> {
        self.remove_encounter_cascade(key)
    }

    pub fn size_on_disk(&self) -> Result<u64> {
        self.db.size_on_disk().context("Failed to read history DB size")
    }

    pub fn flush(&self) -> Result<()> {
        self.db.flush().context("Failed to flush history database")?;
        Ok(())
    }

    pub fn dry_run_retention(&self, policy: &HistoryRetentionPolicy) -> Result<RetentionPlan> {
        let keys = self.collect_retention_keys(policy)?;
        let mut oldest_ms: Option<u64> = None;
        let mut encounter_count = 0usize;
        let mut dungeon_count = 0usize;
        for key in &keys {
            if key.namespace() == ENCOUNTER_NAMESPACE {
                encounter_count += 1;
            } else if key.namespace() == DUNGEON_NAMESPACE {
                dungeon_count += 1;
            }
            if let Some(ms) = self.key_timestamp_ms(key) {
                oldest_ms = Some(match oldest_ms {
                    Some(current) => current.min(ms),
                    None => ms,
                });
            }
        }
        let may_rebuild = matches!(policy.kind, HistoryLimitKind::MaxSizeMb)
            && !keys.is_empty()
            && self
                .size_on_disk()
                .map(|size| size > policy.size_mb.saturating_mul(1024 * 1024))
                .unwrap_or(false);
        Ok(RetentionPlan {
            encounter_count,
            dungeon_count,
            oldest_date: oldest_ms.and_then(format_oldest_date),
            may_rebuild,
        })
    }

    pub fn apply_retention(&self, policy: &HistoryRetentionPolicy) -> Result<RetentionPlan> {
        if matches!(policy.kind, HistoryLimitKind::None) {
            return Ok(RetentionPlan::default());
        }
        let keys = self.collect_retention_keys(policy)?;
        for key in keys {
            if key.namespace() == ENCOUNTER_NAMESPACE {
                self.remove_encounter_cascade(&key)?;
            } else if key.namespace() == DUNGEON_NAMESPACE {
                self.remove_dungeon_cascade(&key)?;
            }
        }
        self.flush()?;
        if matches!(policy.kind, HistoryLimitKind::MaxSizeMb) {
            let cap = policy.size_mb.saturating_mul(1024 * 1024);
            if self.size_on_disk().unwrap_or(0) > cap {
                self.compact_to_cap(policy)?;
            }
        }
        self.dry_run_retention(policy)
    }

    fn collect_retention_keys(&self, policy: &HistoryRetentionPolicy) -> Result<Vec<HistoryKey>> {
        match policy.kind {
            HistoryLimitKind::None => Ok(Vec::new()),
            HistoryLimitKind::MaxAgeDays => self.keys_older_than(cutoff_ms_for_days(policy.days)),
            HistoryLimitKind::MaxSizeMb => self.keys_for_size_cap(policy.size_mb),
        }
    }

    fn keys_older_than(&self, cutoff_ms: u64) -> Result<Vec<HistoryKey>> {
        let mut keys = Vec::new();
        for entry in self.encounter_summaries.iter() {
            let (key_bytes, value_bytes) =
                entry.context("Failed to iterate encounter summaries")?;
            let summary: EncounterSummaryRecord = serde_cbor::from_slice(value_bytes.as_ref())
                .context("Failed to deserialize encounter summary")?;
            if summary.last_seen_ms < cutoff_ms {
                if let Some(key) = HistoryKey::from_bytes(key_bytes.as_ref()) {
                    keys.push(key);
                }
            }
        }
        for entry in self.dungeon_summaries.iter() {
            let (key_bytes, value_bytes) = entry.context("Failed to iterate dungeon summaries")?;
            let summary: DungeonSummaryRecord = serde_cbor::from_slice(value_bytes.as_ref())
                .context("Failed to deserialize dungeon summary")?;
            if summary.last_seen_ms < cutoff_ms {
                if let Some(key) = HistoryKey::from_bytes(key_bytes.as_ref()) {
                    keys.push(key);
                }
            }
        }
        Ok(keys)
    }

    fn keys_for_size_cap(&self, size_mb: u64) -> Result<Vec<HistoryKey>> {
        let cap = size_mb.saturating_mul(1024 * 1024);
        if self.size_on_disk().unwrap_or(0) <= cap {
            return Ok(Vec::new());
        }
        let mut keys = Vec::new();
        let mut dates: Vec<String> = self
            .date_index
            .iter()
            .filter_map(|entry| {
                let (key_bytes, _) = entry.ok()?;
                String::from_utf8(key_bytes.to_vec()).ok()
            })
            .collect();
        dates.sort();
        for date in dates {
            if self.size_on_disk().unwrap_or(0) <= cap {
                break;
            }
            keys.extend(self.keys_for_date(&date)?);
        }
        Ok(keys)
    }

    fn keys_for_date(&self, date_id: &str) -> Result<Vec<HistoryKey>> {
        let mut keys = Vec::new();
        if let Some(bytes) = self.date_index.get(date_id.as_bytes())? {
            let summary: DateSummaryRecord =
                serde_cbor::from_slice(bytes.as_ref()).context("Failed to read date summary")?;
            for key_bytes in summary.encounter_ids {
                if let Some(key) = HistoryKey::from_bytes(&key_bytes) {
                    keys.push(key);
                }
            }
        }
        if let Some(bytes) = self.dungeon_dates.get(date_id.as_bytes())? {
            let summary: DateSummaryRecord = serde_cbor::from_slice(bytes.as_ref())
                .context("Failed to read dungeon date summary")?;
            for key_bytes in summary.encounter_ids {
                if let Some(key) = HistoryKey::from_bytes(&key_bytes) {
                    keys.push(key);
                }
            }
        }
        Ok(keys)
    }

    fn remove_encounter_cascade(&self, key: &HistoryKey) -> Result<()> {
        let key_bytes = key.as_bytes();
        let date_id = self
            .encounter_summaries
            .get(key_bytes.as_slice())
            .context("Failed to read encounter summary for delete")?
            .map(|bytes| {
                serde_cbor::from_slice::<EncounterSummaryRecord>(bytes.as_ref())
                    .map(|s| s.date_id)
            })
            .transpose()
            .context("Failed to deserialize encounter summary for delete")?;

        self.encounters
            .remove(key_bytes.as_slice())
            .context("Failed to delete encounter record")?;
        self.encounter_summaries
            .remove(key_bytes.as_slice())
            .context("Failed to delete encounter summary")?;

        if let Some(date_id) = date_id {
            self.remove_key_from_date_index(&date_id, key_bytes.as_slice(), false)?;
        }
        Ok(())
    }

    fn remove_dungeon_cascade(&self, key: &HistoryKey) -> Result<()> {
        let key_bytes = key.as_bytes();
        let date_id = self
            .dungeon_summaries
            .get(key_bytes.as_slice())
            .context("Failed to read dungeon summary for delete")?
            .map(|bytes| {
                serde_cbor::from_slice::<DungeonSummaryRecord>(bytes.as_ref())
                    .map(|s| s.date_id)
            })
            .transpose()
            .context("Failed to deserialize dungeon summary for delete")?;

        self.dungeon_runs
            .remove(key_bytes.as_slice())
            .context("Failed to delete dungeon run")?;
        self.dungeon_summaries
            .remove(key_bytes.as_slice())
            .context("Failed to delete dungeon summary")?;

        if let Some(date_id) = date_id {
            self.remove_key_from_date_index(&date_id, key_bytes.as_slice(), true)?;
        }
        Ok(())
    }

    fn remove_key_from_date_index(
        &self,
        date_id: &str,
        key_bytes: &[u8],
        dungeon: bool,
    ) -> Result<()> {
        let tree = if dungeon {
            &self.dungeon_dates
        } else {
            &self.date_index
        };
        let Some(bytes) = tree.get(date_id.as_bytes())? else {
            return Ok(());
        };
        let mut record: DateSummaryRecord =
            serde_cbor::from_slice(bytes.as_ref()).context("Failed to deserialize date summary")?;
        record
            .encounter_ids
            .retain(|existing| existing.as_slice() != key_bytes);
        if record.encounter_ids.is_empty() {
            tree.remove(date_id.as_bytes())?;
        } else {
            let updated =
                serde_cbor::to_vec(&record).context("Failed to serialize date summary")?;
            tree.insert(date_id.as_bytes(), updated)?;
        }
        Ok(())
    }

    fn key_timestamp_ms(&self, key: &HistoryKey) -> Option<u64> {
        Some(key.timestamp_ms())
    }

    fn compact_to_cap(&self, policy: &HistoryRetentionPolicy) -> Result<()> {
        let cap = policy.size_mb.saturating_mul(1024 * 1024);
        let mut guard = 0usize;
        while self.size_on_disk().unwrap_or(0) > cap && guard < 512 {
            let keys = self.keys_for_size_cap(policy.size_mb)?;
            if keys.is_empty() {
                break;
            }
            for key in keys {
                if key.namespace() == ENCOUNTER_NAMESPACE {
                    self.remove_encounter_cascade(&key)?;
                } else if key.namespace() == DUNGEON_NAMESPACE {
                    self.remove_dungeon_cascade(&key)?;
                }
            }
            self.flush()?;
            guard += 1;
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub fn tree(&self, name: &str) -> Result<sled::Tree> {
        self.db
            .open_tree(name)
            .with_context(|| format!("Unable to open history tree {name}"))
    }

    fn build_encounter_summary(
        &self,
        key: &[u8],
        record: &EncounterRecord,
    ) -> EncounterSummaryRecord {
        let date_time = millis_to_local(record.last_seen_ms);
        let (date_id, time_label, timestamp_label) = match date_time {
            Some(dt) => (
                dt.date_naive().to_string(),
                dt.format("%H:%M").to_string(),
                dt.format("%Y-%m-%d %H:%M:%S").to_string(),
            ),
            None => (
                "unknown".to_string(),
                "--:--".to_string(),
                "unknown".to_string(),
            ),
        };

        let base_title = resolve_title(record);

        EncounterSummaryRecord {
            key: key.to_vec(),
            date_id,
            base_title,
            encounter_title: record.encounter.title.clone(),
            time_label,
            timestamp_label,
            last_seen_ms: record.last_seen_ms,
            duration: record.encounter.duration.clone(),
            encdps: record.encounter.encdps.clone(),
            damage: record.encounter.damage.clone(),
            zone: record.encounter.zone.clone(),
            snapshots: record.snapshots,
            frames: record.frames.len() as u32,
        }
    }

    fn build_dungeon_summary(
        &self,
        key: &[u8],
        record: &DungeonAggregateRecord,
    ) -> DungeonSummaryRecord {
        let start_time = millis_to_local(record.started_ms);
        let (date_id, started_label) = match start_time {
            Some(dt) => (dt.date_naive().to_string(), dt.format("%H:%M").to_string()),
            None => ("unknown".to_string(), "--:--".to_string()),
        };

        DungeonSummaryRecord {
            key: key.to_vec(),
            date_id,
            zone: record.zone.clone(),
            started_ms: record.started_ms,
            last_seen_ms: record.last_seen_ms,
            duration_secs: record.total_duration_secs,
            total_damage: record.total_damage,
            total_healed: record.total_healed,
            total_encdps: record.total_encdps,
            child_count: record.child_keys.len(),
            incomplete: record.incomplete,
            party_signature: record.party_signature.clone(),
            started_label,
        }
    }

    fn update_date_summary(&self, summary: &EncounterSummaryRecord) -> Result<()> {
        let key = summary.date_id.as_bytes();
        let existing = self
            .date_index
            .get(key)
            .context("Failed to read date summary")?;

        let record = if let Some(bytes) = existing {
            let mut record: DateSummaryRecord =
                serde_cbor::from_slice(&bytes).context("Failed to deserialize date summary")?;
            if !record
                .encounter_ids
                .iter()
                .any(|existing_key| existing_key == &summary.key)
            {
                record.encounter_ids.insert(0, summary.key.clone());
            }
            if summary.last_seen_ms > record.last_seen_ms {
                record.last_seen_ms = summary.last_seen_ms;
            }
            record
        } else {
            DateSummaryRecord {
                date_id: summary.date_id.clone(),
                last_seen_ms: summary.last_seen_ms,
                encounter_ids: vec![summary.key.clone()],
            }
        };

        let bytes =
            serde_cbor::to_vec(&record).context("Failed to serialize updated date summary")?;
        self.date_index
            .insert(key, bytes)
            .context("Failed to persist date summary")?;
        Ok(())
    }

    fn update_dungeon_date_summary(&self, summary: &DungeonSummaryRecord) -> Result<()> {
        let key = summary.date_id.as_bytes();
        let existing = self
            .dungeon_dates
            .get(key)
            .context("Failed to read dungeon date summary")?;

        let record = if let Some(bytes) = existing {
            let mut record: DateSummaryRecord = serde_cbor::from_slice(&bytes)
                .context("Failed to deserialize dungeon date summary")?;
            if !record
                .encounter_ids
                .iter()
                .any(|existing_key| existing_key == &summary.key)
            {
                record.encounter_ids.insert(0, summary.key.clone());
            }
            if summary.last_seen_ms > record.last_seen_ms {
                record.last_seen_ms = summary.last_seen_ms;
            }
            record
        } else {
            DateSummaryRecord {
                date_id: summary.date_id.clone(),
                last_seen_ms: summary.last_seen_ms,
                encounter_ids: vec![summary.key.clone()],
            }
        };

        let bytes = serde_cbor::to_vec(&record)
            .context("Failed to serialize updated dungeon date summary")?;
        self.dungeon_dates
            .insert(key, bytes)
            .context("Failed to persist dungeon date summary")?;
        Ok(())
    }

    pub fn load_dates(&self) -> Result<Vec<HistoryDay>> {
        let mut days = Vec::new();
        for entry in self.date_index.iter() {
            let (key_bytes, value_bytes) = entry.context("Failed to iterate history date index")?;
            let record: DateSummaryRecord = serde_cbor::from_slice(value_bytes.as_ref())
                .context("Failed to deserialize date summary")?;
            let iso_date = String::from_utf8(key_bytes.to_vec()).unwrap_or(record.date_id.clone());
            let label = format_date_label(&iso_date, record.encounter_ids.len());
            days.push(HistoryDay {
                iso_date,
                label,
                encounter_count: record.encounter_ids.len(),
                encounters: Vec::new(),
                encounter_ids: record.encounter_ids,
                encounters_loaded: false,
            });
        }
        days.sort_by(|a, b| b.iso_date.cmp(&a.iso_date));
        Ok(days)
    }

    pub fn load_dungeon_days(&self) -> Result<Vec<DungeonHistoryDay>> {
        let mut days = Vec::new();
        for entry in self.dungeon_dates.iter() {
            let (key_bytes, value_bytes) = entry.context("Failed to iterate dungeon date index")?;
            let record: DateSummaryRecord = serde_cbor::from_slice(value_bytes.as_ref())
                .context("Failed to deserialize dungeon date summary")?;
            let iso_date = String::from_utf8(key_bytes.to_vec()).unwrap_or(record.date_id.clone());
            let label = format_dungeon_date_label(&iso_date, record.encounter_ids.len());
            days.push(DungeonHistoryDay {
                iso_date,
                label,
                run_count: record.encounter_ids.len(),
                runs: Vec::new(),
                run_ids: record.encounter_ids,
                runs_loaded: false,
            });
        }
        days.sort_by(|a, b| b.iso_date.cmp(&a.iso_date));
        Ok(days)
    }

    pub fn load_encounter_summaries(&self, date_id: &str) -> Result<Vec<HistoryEncounterItem>> {
        let key = date_id.as_bytes();
        let Some(bytes) = self
            .date_index
            .get(key)
            .context("Failed to read date summary for encounters")?
        else {
            return Ok(Vec::new());
        };

        let date_summary: DateSummaryRecord =
            serde_cbor::from_slice(bytes.as_ref()).context("Failed to deserialize date summary")?;

        let mut summaries = Vec::new();
        for encounter_id in &date_summary.encounter_ids {
            if let Some(bytes) = self
                .encounter_summaries
                .get(encounter_id)
                .context("Failed to read encounter summary")?
            {
                let summary: EncounterSummaryRecord = serde_cbor::from_slice(bytes.as_ref())
                    .context("Failed to deserialize encounter summary")?;
                summaries.push(summary);
            }
        }

        summaries.sort_by_key(|b| std::cmp::Reverse(b.last_seen_ms));

        Ok(build_history_items_from_summaries(summaries))
    }

    pub fn load_dungeon_summaries(&self, date_id: &str) -> Result<Vec<DungeonHistoryItem>> {
        let key = date_id.as_bytes();
        let Some(bytes) = self
            .dungeon_dates
            .get(key)
            .context("Failed to read dungeon date summary")?
        else {
            return Ok(Vec::new());
        };

        let date_summary: DateSummaryRecord = serde_cbor::from_slice(bytes.as_ref())
            .context("Failed to deserialize dungeon date summary")?;

        let mut summaries = Vec::new();
        for run_id in &date_summary.encounter_ids {
            if let Some(bytes) = self
                .dungeon_summaries
                .get(run_id)
                .context("Failed to read dungeon summary")?
            {
                let summary: DungeonSummaryRecord = serde_cbor::from_slice(bytes.as_ref())
                    .context("Failed to deserialize dungeon summary record")?;
                summaries.push(summary);
            }
        }

        summaries.sort_by_key(|b| std::cmp::Reverse(b.last_seen_ms));
        Ok(build_dungeon_history_items(summaries))
    }

    pub fn load_encounter_record(&self, key: &[u8]) -> Result<EncounterRecord> {
        let Some(bytes) = self
            .encounters
            .get(key)
            .context("Failed to read encounter record")?
        else {
            anyhow::bail!("Encounter record not found");
        };
        serde_cbor::from_slice(bytes.as_ref()).context("Failed to deserialize encounter record")
    }

    pub fn load_dungeon_record(&self, key: &[u8]) -> Result<DungeonAggregateRecord> {
        let Some(bytes) = self
            .dungeon_runs
            .get(key)
            .context("Failed to read dungeon aggregate record")?
        else {
            anyhow::bail!("Dungeon aggregate record not found");
        };
        serde_cbor::from_slice(bytes.as_ref())
            .context("Failed to deserialize dungeon aggregate record")
    }

    fn init_schema(&self) -> Result<()> {
        match self
            .meta
            .get(META_SCHEMA_VERSION_KEY)
            .context("Failed to read schema version from history metadata")?
        {
            Some(bytes) if bytes.len() == 4 => {
                let mut arr = [0u8; 4];
                arr.copy_from_slice(&bytes);
                let version = u32::from_be_bytes(arr);
                if version != SCHEMA_VERSION {
                    eprintln!(
                        "Warning: history schema version mismatch (stored: {}, expected: {})",
                        version, SCHEMA_VERSION
                    );
                }
            }
            Some(bytes) => {
                eprintln!(
                    "Warning: history schema version entry had unexpected size: {} bytes",
                    bytes.len()
                );
            }
            None => {
                let version_bytes = SCHEMA_VERSION.to_be_bytes();
                self.meta
                    .insert(META_SCHEMA_VERSION_KEY, &version_bytes)
                    .context("Failed to initialize history schema version")?;
            }
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub fn root(&self) -> &Path {
        &self.root
    }
}

fn millis_to_local(ms: u64) -> Option<DateTime<Local>> {
    let millis = i64::try_from(ms).ok()?;
    Local.timestamp_millis_opt(millis).single()
}

fn format_date_label(iso_date: &str, encounter_count: usize) -> String {
    match NaiveDate::parse_from_str(iso_date, "%Y-%m-%d") {
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

fn format_dungeon_date_label(iso_date: &str, run_count: usize) -> String {
    match NaiveDate::parse_from_str(iso_date, "%Y-%m-%d") {
        Ok(date) => {
            let weekday = date.format("%a");
            format!("{} ({}) · {} runs", iso_date, weekday, run_count)
        }
        Err(_) => format!("{} · {} runs", iso_date, run_count),
    }
}

fn build_history_items_from_summaries(
    summaries: Vec<EncounterSummaryRecord>,
) -> Vec<HistoryEncounterItem> {
    let mut totals: HashMap<String, u32> = HashMap::new();
    for summary in &summaries {
        *totals.entry(summary.base_title.clone()).or_insert(0) += 1;
    }

    let mut chronological: HashMap<String, Vec<(u64, Vec<u8>)>> = HashMap::new();
    for summary in &summaries {
        chronological
            .entry(summary.base_title.clone())
            .or_default()
            .push((summary.last_seen_ms, summary.key.clone()));
    }

    let mut occurrence_by_key: HashMap<Vec<u8>, u32> = HashMap::new();
    for entries in chronological.values_mut() {
        entries.sort_by_key(|a| a.0);
        for (idx, (_, key)) in entries.iter().enumerate() {
            occurrence_by_key.insert(key.clone(), (idx + 1) as u32);
        }
    }

    summaries
        .into_iter()
        .map(|summary| {
            let total = totals.get(&summary.base_title).copied().unwrap_or(1);
            let occurrence = occurrence_by_key.get(&summary.key).copied().unwrap_or(1);
            let display_title = if total > 1 {
                format!("{} ({})", summary.base_title.as_str(), occurrence)
            } else {
                summary.base_title.clone()
            };
            HistoryEncounterItem {
                key: summary.key,
                display_title,
                base_title: summary.base_title,
                occurrence,
                time_label: summary.time_label,
                last_seen_ms: summary.last_seen_ms,
                timestamp_label: summary.timestamp_label,
                record: None,
            }
        })
        .collect()
}

fn build_dungeon_history_items(summaries: Vec<DungeonSummaryRecord>) -> Vec<DungeonHistoryItem> {
    summaries
        .into_iter()
        .map(|summary| {
            let duration_label = format_duration_label(summary.duration_secs);
            let started_label = if summary.started_label.is_empty() {
                "--:--".to_string()
            } else {
                summary.started_label.clone()
            };
            DungeonHistoryItem {
                key: summary.key,
                zone: summary.zone,
                started_label,
                duration_label,
                total_damage: summary.total_damage,
                total_healed: summary.total_healed,
                total_encdps: summary.total_encdps,
                child_count: summary.child_count,
                last_seen_ms: summary.last_seen_ms,
                incomplete: summary.incomplete,
                party_signature: summary.party_signature,
                record: None,
                child_records: Vec::new(),
            }
        })
        .collect()
}

fn format_duration_label(total_secs: u64) -> String {
    if total_secs == 0 {
        return "00:00".to_string();
    }
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    } else {
        format!("{:02}:{:02}", minutes, seconds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::EncounterSummary;
    use crate::history::retention::{cutoff_ms_for_days, now_ms, HistoryLimitKind, HistoryRetentionPolicy};
    use crate::history::types::SCHEMA_VERSION;

    fn make_summary(key: &[u8], base_title: &str, last_seen: u64) -> EncounterSummaryRecord {
        EncounterSummaryRecord {
            key: key.to_vec(),
            date_id: "2025-01-01".into(),
            base_title: base_title.into(),
            encounter_title: base_title.into(),
            time_label: "12:00".into(),
            timestamp_label: "2025-01-01 12:00:00".into(),
            last_seen_ms: last_seen,
            duration: "00:30".into(),
            encdps: "1000".into(),
            damage: "100000".into(),
            zone: "Zone".into(),
            snapshots: 3,
            frames: 3,
        }
    }

    #[test]
    fn build_history_items_numbers_duplicate_titles() {
        let summaries = vec![
            make_summary(&[1], "Doma Castle", 2_000),
            make_summary(&[2], "Doma Castle", 3_000),
            make_summary(&[3], "Striking Dummy", 1_000),
        ];
        let items = build_history_items_from_summaries(summaries);
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].display_title, "Doma Castle (1)");
        assert_eq!(items[1].display_title, "Doma Castle (2)");
        assert_eq!(items[2].display_title, "Striking Dummy");
        assert!(items.iter().all(|item| item.record.is_none()));
    }

    #[test]
    fn build_history_items_numbers_respect_chronology() {
        let mut summaries = vec![
            make_summary(&[1], "Rubicante", 1_000),
            make_summary(&[2], "Rubicante", 3_000),
            make_summary(&[3], "Rubicante", 2_000),
        ];
        summaries.sort_by(|a, b| b.last_seen_ms.cmp(&a.last_seen_ms));

        let items = build_history_items_from_summaries(summaries);
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].display_title, "Rubicante (3)");
        assert_eq!(items[1].display_title, "Rubicante (2)");
        assert_eq!(items[2].display_title, "Rubicante (1)");
    }

    #[test]
    fn build_dungeon_history_items_formats_labels() {
        let summary = DungeonSummaryRecord {
            key: vec![1],
            date_id: "2025-09-30".into(),
            zone: "Sastasha".into(),
            started_ms: 1_000,
            started_label: "12:00".into(),
            last_seen_ms: 2_000,
            duration_secs: 125,
            total_damage: 12345.0,
            total_healed: 234.0,
            total_encdps: 98.7,
            child_count: 3,
            incomplete: false,
            party_signature: vec!["Alice|NIN".into()],
        };
        let items = build_dungeon_history_items(vec![summary]);
        assert_eq!(items.len(), 1);
        let item = &items[0];
        assert_eq!(item.duration_label, "02:05");
        assert_eq!(item.child_count, 3);
        assert_eq!(item.zone, "Sastasha");
    }

    #[test]
    fn dry_run_age_retention_counts_old_records() {
        let base = std::env::temp_dir().join(format!("nekomata-retention-{}", now_ms()));
        std::fs::create_dir_all(&base).expect("create temp dir");
        let db_path = base.join("encounters.sled");
        let store = HistoryStore::open(&db_path).expect("open store");

        let old_ms = cutoff_ms_for_days(10);
        let record = EncounterRecord {
            version: SCHEMA_VERSION,
            stored_ms: old_ms,
            first_seen_ms: old_ms,
            last_seen_ms: old_ms,
            encounter: EncounterSummary {
                title: "Old Fight".into(),
                zone: "Zone".into(),
                duration: "01:00".into(),
                encdps: "100".into(),
                damage: "1000".into(),
                enchps: "0".into(),
                healed: "0".into(),
                is_active: false,
            },
            rows: Vec::new(),
            raw_last: None,
            snapshots: 1,
            saw_active: true,
            frames: Vec::new(),
            lb_summary: None,
        };
        store.append(&record).expect("append old record");

        let policy = HistoryRetentionPolicy {
            kind: HistoryLimitKind::MaxAgeDays,
            days: 5,
            size_mb: 256,
        };
        let plan = store.dry_run_retention(&policy).expect("dry run");
        assert_eq!(plan.encounter_count, 1);
    }
}
