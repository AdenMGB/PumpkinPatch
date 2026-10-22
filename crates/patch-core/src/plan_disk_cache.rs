//! Plan §5.1 disk JSON cache (2-day TTL) for offline / static hosting.

use crate::error::{Error, Result};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const DISK_CACHE_TTL_SECS: u64 = 2 * 24 * 60 * 60;

fn cache_path(dir: &Path, key: &str) -> PathBuf {
    let safe = key
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect::<String>();
    dir.join("http-cache").join(format!("{}.json", safe))
}

pub fn read_disk_cache(dir: &Path, key: &str) -> Result<Option<String>> {
    let path = cache_path(dir, key);
    if !path.exists() {
        return Ok(None);
    }
    let meta = std::fs::metadata(&path)?;
    let modified = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    if now.saturating_sub(modified) > DISK_CACHE_TTL_SECS {
        let _ = std::fs::remove_file(&path);
        return Ok(None);
    }
    let body = std::fs::read_to_string(&path)?;
    Ok(Some(body))
}

pub fn write_disk_cache(dir: &Path, key: &str, body: &str) -> Result<()> {
    let path = cache_path(dir, key);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("part");
    std::fs::write(&tmp, body)?;
    std::fs::rename(tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_disk_cache() {
        let dir = std::env::temp_dir().join("patch-plan-disk-cache-test");
        let _ = std::fs::remove_dir_all(&dir);
        write_disk_cache(&dir, "serverOverview|net|srv|", r#"{"ok":true}"#).unwrap();
        let hit = read_disk_cache(&dir, "serverOverview|net|srv|").unwrap();
        assert_eq!(hit.as_deref(), Some(r#"{"ok":true}"#));
    }
}
