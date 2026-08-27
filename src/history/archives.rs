use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

use anyhow::{Context, Result};
use chrono::{DateTime, Local};

use crate::config;

/// Metadata for a history archive directory.
#[derive(Clone, Debug)]
pub struct ArchiveEntry {
    pub name: String,
    pub path: PathBuf,
    pub size_bytes: u64,
    pub modified_ms: u64,
}

pub fn list_archives() -> Result<Vec<ArchiveEntry>> {
    let dir = config::history_archives_dir();
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut entries = Vec::new();
    for entry in fs::read_dir(&dir).with_context(|| format!("Failed to read {}", dir.display()))? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if !file_type.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path();
        let size_bytes = dir_size(&path)?;
        let modified_ms = entry
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(system_time_to_ms)
            .unwrap_or(0);
        entries.push(ArchiveEntry {
            name,
            path,
            size_bytes,
            modified_ms,
        });
    }
    entries.sort_by(|a, b| b.modified_ms.cmp(&a.modified_ms));
    Ok(entries)
}

pub fn archive_exists(name: &str) -> bool {
    config::history_archive_path(name).exists()
}

pub fn create_backup(name: &str) -> Result<PathBuf> {
    let sanitized = config::sanitize_archive_name(name)
        .ok_or_else(|| anyhow::anyhow!("Invalid archive name"))?;
    if archive_exists(&sanitized) {
        anyhow::bail!("An archive with that name already exists");
    }
    let source = config::history_db_path();
    if !source.exists() {
        anyhow::bail!("Nothing to back up");
    }
    let dest = config::history_archive_path(&sanitized);
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!("Unable to create archive directory {}", parent.display())
        })?;
    }
    copy_dir_all(&source, &dest)?;
    Ok(dest)
}

pub fn delete_archive(name: &str) -> Result<()> {
    let path = config::history_archive_path(name);
    if !path.exists() {
        anyhow::bail!("Archive not found");
    }
    fs::remove_dir_all(&path).with_context(|| format!("Failed to delete archive {}", path.display()))
}

pub fn delete_live_history() -> Result<()> {
    let path = config::history_db_path();
    if path.exists() {
        fs::remove_dir_all(&path).with_context(|| {
            format!("Failed to delete live history at {}", path.display())
        })?;
    }
    Ok(())
}

pub fn format_bytes(bytes: u64) -> String {
    const MB: f64 = 1024.0 * 1024.0;
    if bytes >= MB as u64 {
        format!("{:.1} MB", bytes as f64 / MB)
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}

pub fn default_backup_name() -> String {
    let now: DateTime<Local> = Local::now();
    format!("history-{}", now.format("%Y%m%d-%H%M%S"))
}

fn copy_dir_all(src: &std::path::Path, dst: &std::path::Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dest_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dest_path)?;
        } else {
            fs::copy(entry.path(), dest_path)?;
        }
    }
    Ok(())
}

fn dir_size(path: &std::path::Path) -> Result<u64> {
    let mut total = 0u64;
    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let ty = entry.file_type()?;
            if ty.is_dir() {
                total = total.saturating_add(dir_size(&entry.path())?);
            } else {
                total = total.saturating_add(entry.metadata()?.len());
            }
        }
    } else if path.is_file() {
        total = total.saturating_add(fs::metadata(path)?.len());
    }
    Ok(total)
}

fn system_time_to_ms(time: SystemTime) -> Option<u64> {
    time.duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_millis() as u64)
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_rejects_invalid_names() {
        assert!(config::sanitize_archive_name("").is_none());
        assert!(config::sanitize_archive_name("..").is_none());
        assert!(config::sanitize_archive_name("foo/bar").is_none());
        assert_eq!(
            config::sanitize_archive_name("my-backup_1").as_deref(),
            Some("my-backup_1")
        );
    }
}
