use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use serde::Deserialize;
use serde_json::Value;
use tracing::{debug, info, warn};

const EMBEDDED_CATALOG: &str = include_str!("../../dungeon-catalog.json");
const DUNGEON_CATALOG_ENV: &str = "NEKOMATA_DUNGEON_CATALOG";
const LEGACY_DUNGEON_CATALOG_ENV: &str = "IINACT_DUNGEON_CATALOG";

static DEFAULT_CATALOG_FILENAMES: Lazy<[&str; 1]> = Lazy::new(|| ["dungeon-catalog.json"]);

#[derive(Debug, Deserialize)]
struct RawCatalog {
    #[serde(default)]
    dungeons: HashMap<String, Value>,
}

/// Lookup helper for determining whether a zone should participate in dungeon aggregation.
#[derive(Debug, Clone, Default)]
pub struct DungeonCatalog {
    canonical_by_norm: HashMap<String, String>,
}

impl DungeonCatalog {
    /// Load the catalog from the first discovered default location.
    pub fn load_default() -> Result<Self> {
        if let Some(path) = locate_default_file() {
            match Self::load_from_path(&path) {
                Ok(catalog) => return Ok(catalog),
                Err(err) => {
                    warn!(
                        error = ?err,
                        path = %path.display(),
                        "Failed to load dungeon catalog from disk; falling back to embedded copy"
                    );
                }
            }
        } else {
            info!("Dungeon catalog file not found on disk; using embedded copy");
        }

        Self::from_str(EMBEDDED_CATALOG)
            .context("Failed to load embedded dungeon catalog definition")
    }

    /// Load the catalog from the provided path.
    pub fn load_from_path(path: &Path) -> Result<Self> {
        let mut file = File::open(path)
            .with_context(|| format!("Unable to open dungeon catalog {}", path.display()))?;
        Self::load_from_reader(&mut file)
    }

    /// Load the catalog from an arbitrary reader (useful for tests).
    pub fn load_from_reader(reader: &mut dyn Read) -> Result<Self> {
        let mut buf = String::new();
        reader
            .read_to_string(&mut buf)
            .context("Failed to read dungeon catalog contents")?;
        Self::from_str(&buf)
    }

    /// Parse the catalog from an in-memory string.
    pub fn from_str(input: &str) -> Result<Self> {
        let raw: RawCatalog =
            json5::from_str(input).context("Failed to parse dungeon catalog JSON")?;
        Ok(Self::from_raw(raw))
    }

    fn from_raw(raw: RawCatalog) -> Self {
        let mut canonical_by_norm = HashMap::new();
        let mut duplicates = 0usize;

        for (zone, _metadata) in raw.dungeons {
            if let Some(normalized) = normalize_zone(&zone) {
                if canonical_by_norm.contains_key(&normalized) {
                    duplicates += 1;
                    warn!(zone = %zone, normalized = %normalized, "Duplicate dungeon zone in catalog; keeping first entry");
                    continue;
                }
                canonical_by_norm.insert(normalized, collapse_whitespace(zone.trim()));
            } else {
                debug!(original = %zone, "Skipping empty/invalid dungeon zone entry");
            }
        }

        if duplicates > 0 {
            info!(
                duplicates,
                "Dungeon catalog contained duplicate zone entries"
            );
        }

        info!(count = canonical_by_norm.len(), "Dungeon catalog loaded");

        Self { canonical_by_norm }
    }

    /// Returns the canonical zone name if the provided zone is recognised.
    pub fn canonical_zone<'a>(&'a self, zone: &str) -> Option<&'a str> {
        let key = normalize_zone(zone)?;
        self.canonical_by_norm.get(&key).map(|s| s.as_str())
    }

    /// Returns true when the provided zone exists in the catalog.
    #[allow(dead_code)]
    pub fn is_zone(&self, zone: &str) -> bool {
        self.canonical_zone(zone).is_some()
    }

    /// Number of catalogued dungeon zones.
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.canonical_by_norm.len()
    }

    /// Returns true when the catalog has no entries.
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.canonical_by_norm.is_empty()
    }
}

fn locate_default_file() -> Option<PathBuf> {
    if let Some(env_path) = std::env::var_os(DUNGEON_CATALOG_ENV) {
        let candidate = PathBuf::from(env_path);
        if candidate.exists() {
            return Some(candidate);
        }
    }

    if let Some(env_path) = std::env::var_os(LEGACY_DUNGEON_CATALOG_ENV) {
        let candidate = PathBuf::from(env_path);
        if candidate.exists() {
            return Some(candidate);
        }
    }

    for filename in DEFAULT_CATALOG_FILENAMES.iter().copied() {
        let candidate = PathBuf::from(filename);
        if candidate.exists() {
            return Some(candidate);
        }
    }

    if let Ok(mut exe_path) = std::env::current_exe() {
        exe_path.pop();
        for filename in DEFAULT_CATALOG_FILENAMES.iter().copied() {
            let candidate = exe_path.join(filename);
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }

    None
}

fn normalize_zone(zone: &str) -> Option<String> {
    let collapsed = collapse_whitespace(zone.trim());
    if collapsed.is_empty() {
        return None;
    }
    Some(collapsed.to_lowercase())
}

fn collapse_whitespace(input: &str) -> String {
    let mut buf = String::with_capacity(input.len());
    let mut in_whitespace = false;
    for ch in input.chars() {
        if ch.is_whitespace() {
            if !in_whitespace {
                buf.push(' ');
                in_whitespace = true;
            }
        } else {
            in_whitespace = false;
            buf.push(ch);
        }
    }
    buf.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_zone_trims_and_folds() {
        assert_eq!(normalize_zone("  Sastasha  "), Some("sastasha".to_string()));
        assert_eq!(
            normalize_zone("The Tam-Tara Deepcroft"),
            Some("the tam-tara deepcroft".to_string())
        );
        assert!(normalize_zone("   ").is_none());
    }

    #[test]
    fn catalog_deduplicates_by_normalized_zone() {
        let catalog = DungeonCatalog::from_str(
            r#"{
            "dungeons": {
                "Sastasha": {},
                "  sastasha  ": {},
                "Copperbell Mines": {}
            }
        }"#,
        )
        .expect("catalog parse");
        assert!(catalog.is_zone("SASTASHA"));
        assert!(catalog.is_zone("Copperbell Mines"));
        assert!(!catalog.is_zone("Unknown"));
        assert_eq!(catalog.len(), 2);
    }

    #[test]
    fn catalog_allows_trailing_commas() {
        let src = "{ \"dungeons\": { \"Sastasha\": {}, }}";
        let catalog = DungeonCatalog::from_str(src).expect("catalog parse");
        assert!(catalog.is_zone("Sastasha"));
    }

    #[test]
    fn collapse_whitespace_collapses_sequences() {
        assert_eq!(collapse_whitespace("A   B"), "A B");
        assert_eq!(collapse_whitespace("A\nB\tC"), "A B C");
    }
}
