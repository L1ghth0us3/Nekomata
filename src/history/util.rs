use crate::history::types::EncounterRecord;
use crate::model::CombatantRow;

pub(crate) fn parse_duration_secs(s: &str) -> Option<u64> {
    if s.trim().is_empty() {
        return None;
    }
    let mut parts: Vec<&str> = s.trim().split(':').collect();
    if parts.is_empty() || parts.len() > 3 {
        return None;
    }
    let mut value = 0u64;
    let mut multiplier = 1u64;
    while let Some(part) = parts.pop() {
        let part = part.trim();
        if part.is_empty() || part.contains('-') {
            return None;
        }
        let parsed = part.parse::<u64>().ok()?;
        value += parsed.saturating_mul(multiplier);
        multiplier = multiplier.saturating_mul(60);
    }
    Some(value)
}

pub(crate) fn parse_number(s: &str) -> f64 {
    let mut buf = String::with_capacity(s.len());
    for ch in s.chars() {
        if ch.is_ascii_digit() || matches!(ch, '.' | '+' | '-') {
            buf.push(ch);
        }
    }
    if buf.is_empty() {
        return 0.0;
    }
    buf.parse::<f64>().unwrap_or(0.0)
}

pub(crate) fn party_signature(rows: &[CombatantRow]) -> Vec<String> {
    let mut entries: Vec<String> = rows
        .iter()
        .map(|row| format!("{}|{}", row.name.trim(), row.job.trim()))
        .collect();
    entries.sort_unstable();
    entries.dedup();
    entries
}

pub(crate) fn resolve_title(record: &EncounterRecord) -> String {
    let primary = record.encounter.title.trim();
    if !primary.is_empty() {
        return primary.to_string();
    }
    let zone = record.encounter.zone.trim();
    if !zone.is_empty() {
        return zone.to_string();
    }
    "Unknown Encounter".to_string()
}

#[cfg(test)]
mod tests {
    use crate::history::types::EncounterRecord;
    use crate::model::CombatantRow;

    use super::*;

    #[test]
    fn duration_parsing_supports_mm_ss() {
        assert_eq!(parse_duration_secs("01:30"), Some(90));
        assert_eq!(parse_duration_secs("1:02:03"), Some(3723));
        assert_eq!(parse_duration_secs("--:--"), None);
    }

    #[test]
    fn parse_number_handles_commas_and_percent() {
        assert_eq!(parse_number("12,345.6"), 12345.6);
        assert_eq!(parse_number("98%"), 98.0);
    }

    #[test]
    fn party_signature_sorts_and_dedups() {
        let rows = vec![
            CombatantRow {
                name: "Alice".into(),
                job: "NIN".into(),
                ..Default::default()
            },
            CombatantRow {
                name: "Bob".into(),
                job: "WHM".into(),
                ..Default::default()
            },
            CombatantRow {
                name: "Alice".into(),
                job: "NIN".into(),
                ..Default::default()
            },
        ];
        let sig = party_signature(&rows);
        assert_eq!(sig, vec!["Alice|NIN".to_string(), "Bob|WHM".to_string()]);
    }

    #[test]
    fn resolve_title_prefers_encounter_title_then_zone() {
        let mut record = EncounterRecord {
            version: 1,
            stored_ms: 0,
            first_seen_ms: 0,
            last_seen_ms: 0,
            encounter: Default::default(),
            rows: Vec::new(),
            raw_last: None,
            snapshots: 0,
            saw_active: false,
            frames: Vec::new(),
        };
        record.encounter.title = "Boss Fight".into();
        assert_eq!(resolve_title(&record), "Boss Fight");
        record.encounter.title = "".into();
        record.encounter.zone = "Sastasha".into();
        assert_eq!(resolve_title(&record), "Sastasha");
        record.encounter.zone = "".into();
        assert_eq!(resolve_title(&record), "Unknown Encounter");
    }
}
