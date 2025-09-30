use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::model::{CombatantRow, EncounterSummary};

pub(crate) const ENCOUNTER_NAMESPACE: &str = "enc";
pub(crate) const DUNGEON_NAMESPACE: &str = "dun";
pub(crate) const KEY_SEPARATOR: u8 = 0x1F;
pub(crate) const SCHEMA_VERSION: u32 = 2;
pub(crate) const META_SCHEMA_VERSION_KEY: &[u8] = b"schema/version";

/// Snapshot prepared for persistence; keeps the raw payload around for future use.
#[derive(Debug, Clone)]
pub struct EncounterSnapshot {
    pub encounter: EncounterSummary,
    pub rows: Vec<CombatantRow>,
    pub raw: Value,
    pub received_ms: u64,
}

impl EncounterSnapshot {
    pub fn new(encounter: EncounterSummary, rows: Vec<CombatantRow>, raw: Value) -> Self {
        Self {
            encounter,
            rows,
            raw,
            received_ms: now_ms(),
        }
    }
}

/// Opaque key wrapper whose encoded representation sorts by timestamp, then discriminator.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct HistoryKey {
    namespace: String,
    timestamp_ms: u64,
    discriminator: u64,
}

impl HistoryKey {
    pub fn new(namespace: impl Into<String>, timestamp_ms: u64, discriminator: u64) -> Self {
        Self {
            namespace: namespace.into(),
            timestamp_ms,
            discriminator,
        }
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        encode_key(&self.namespace, self.timestamp_ms, self.discriminator)
    }

    #[allow(dead_code)]
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        decode_key(bytes)
    }

    #[allow(dead_code)]
    pub fn prefix(namespace: &str) -> Vec<u8> {
        let mut buf = Vec::with_capacity(namespace.len() + 1);
        buf.extend_from_slice(namespace.as_bytes());
        buf.push(KEY_SEPARATOR);
        buf
    }
}

/// Data captured for each concluded encounter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncounterRecord {
    pub version: u32,
    pub stored_ms: u64,
    pub first_seen_ms: u64,
    pub last_seen_ms: u64,
    pub encounter: EncounterSummary,
    pub rows: Vec<CombatantRow>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_last: Option<Value>,
    #[serde(default)]
    pub snapshots: u32,
    #[serde(default)]
    pub saw_active: bool,
    #[serde(default)]
    pub frames: Vec<EncounterFrame>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DungeonAggregateRecord {
    pub version: u32,
    pub zone: String,
    pub started_ms: u64,
    pub last_seen_ms: u64,
    pub party_signature: Vec<String>,
    pub total_duration_secs: u64,
    pub total_damage: f64,
    pub total_healed: f64,
    pub total_encdps: f64,
    pub child_keys: Vec<Vec<u8>>,
    pub child_titles: Vec<String>,
    pub incomplete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncounterFrame {
    pub received_ms: u64,
    pub encounter: EncounterSummary,
    pub rows: Vec<CombatantRow>,
    pub raw: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HistoryEncounterItem {
    pub key: Vec<u8>,
    pub display_title: String,
    pub base_title: String,
    pub occurrence: u32,
    pub time_label: String,
    pub last_seen_ms: u64,
    pub timestamp_label: String,
    #[serde(default)]
    pub record: Option<EncounterRecord>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HistoryDay {
    pub iso_date: String,
    pub label: String,
    pub encounter_count: usize,
    #[serde(default)]
    pub encounters: Vec<HistoryEncounterItem>,
    #[serde(default)]
    pub encounter_ids: Vec<Vec<u8>>,
    #[serde(default)]
    pub encounters_loaded: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncounterSummaryRecord {
    pub key: Vec<u8>,
    pub date_id: String,
    pub base_title: String,
    pub encounter_title: String,
    pub time_label: String,
    pub timestamp_label: String,
    pub last_seen_ms: u64,
    pub duration: String,
    pub encdps: String,
    pub damage: String,
    pub zone: String,
    pub snapshots: u32,
    pub frames: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DungeonSummaryRecord {
    pub key: Vec<u8>,
    pub date_id: String,
    pub zone: String,
    pub started_ms: u64,
    pub started_label: String,
    pub last_seen_ms: u64,
    pub duration_secs: u64,
    pub total_damage: f64,
    pub total_healed: f64,
    pub total_encdps: f64,
    pub child_count: usize,
    pub incomplete: bool,
    pub party_signature: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DungeonHistoryItem {
    pub key: Vec<u8>,
    pub zone: String,
    pub started_label: String,
    pub duration_label: String,
    pub total_damage: f64,
    pub total_healed: f64,
    pub total_encdps: f64,
    pub child_count: usize,
    pub last_seen_ms: u64,
    pub incomplete: bool,
    pub party_signature: Vec<String>,
    #[serde(default)]
    pub record: Option<DungeonAggregateRecord>,
    #[serde(default, skip_serializing, skip_deserializing)]
    pub child_records: Vec<Option<EncounterRecord>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DungeonHistoryDay {
    pub iso_date: String,
    pub label: String,
    pub run_count: usize,
    #[serde(default)]
    pub runs: Vec<DungeonHistoryItem>,
    #[serde(default)]
    pub run_ids: Vec<Vec<u8>>,
    #[serde(default)]
    pub runs_loaded: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateSummaryRecord {
    pub date_id: String,
    pub last_seen_ms: u64,
    pub encounter_ids: Vec<Vec<u8>>,
}

pub(crate) fn encode_key(namespace: &str, timestamp_ms: u64, discriminator: u64) -> Vec<u8> {
    let mut buf = Vec::with_capacity(namespace.len() + 1 + 8 + 1 + 8);
    buf.extend_from_slice(namespace.as_bytes());
    buf.push(KEY_SEPARATOR);
    buf.extend_from_slice(&timestamp_ms.to_be_bytes());
    buf.push(KEY_SEPARATOR);
    buf.extend_from_slice(&discriminator.to_be_bytes());
    buf
}

#[allow(dead_code)]
pub(crate) fn decode_key(bytes: &[u8]) -> Option<HistoryKey> {
    let mut parts = bytes.split(|b| *b == KEY_SEPARATOR);
    let namespace = parts.next()?;
    let timestamp_bytes = parts.next()?;
    let discriminator_bytes = parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    if timestamp_bytes.len() != 8 || discriminator_bytes.len() != 8 {
        return None;
    }
    let mut ts_arr = [0u8; 8];
    ts_arr.copy_from_slice(timestamp_bytes);
    let mut disc_arr = [0u8; 8];
    disc_arr.copy_from_slice(discriminator_bytes);
    Some(HistoryKey {
        namespace: String::from_utf8(namespace.to_vec()).ok()?,
        timestamp_ms: u64::from_be_bytes(ts_arr),
        discriminator: u64::from_be_bytes(disc_arr),
    })
}

pub(crate) fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_roundtrip() {
        let key = HistoryKey::new("enc", 12345, 42);
        let encoded = key.as_bytes();
        let decoded = HistoryKey::from_bytes(&encoded).expect("decode key");
        assert_eq!(decoded.namespace, "enc");
        assert_eq!(decoded.timestamp_ms, 12345);
        assert_eq!(decoded.discriminator, 42);
    }
}
