//! Append-only BLAKE3 hash-chained audit ledger (plangem.md §6.5).

use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub seq: u64,
    pub timestamp_ms: i64,
    pub actor: String,
    pub action: String,
    pub detail: String,
    pub prev_hash: String,
    pub entry_hash: String,
}

#[derive(Debug, Clone)]
pub struct AuditLedger {
    path: std::path::PathBuf,
    last_hash: String,
    seq: u64,
}

impl AuditLedger {
    pub fn open(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut last_hash = String::from("0");
        let mut seq = 0u64;
        if path.exists() {
            let file = File::open(&path)?;
            for line in BufReader::new(file).lines() {
                let line = line?;
                if line.trim().is_empty() {
                    continue;
                }
                let entry: AuditEntry = serde_json::from_str(&line)?;
                seq = entry.seq;
                last_hash = entry.entry_hash;
            }
        }
        Ok(Self {
            path,
            last_hash,
            seq,
        })
    }

    pub fn last_hash(&self) -> &str {
        &self.last_hash
    }

    pub fn append(
        &mut self,
        actor: &str,
        action: &str,
        detail: &str,
    ) -> std::io::Result<AuditEntry> {
        self.seq += 1;
        let timestamp_ms = chrono_lite_now_ms();
        let payload = format!(
            "{}|{}|{}|{}|{}|{}",
            self.seq,
            timestamp_ms,
            actor,
            action,
            detail,
            self.last_hash
        );
        let entry_hash = format!("{}", blake3::hash(payload.as_bytes()).to_hex());
        let entry = AuditEntry {
            seq: self.seq,
            timestamp_ms,
            actor: actor.to_string(),
            action: action.to_string(),
            detail: detail.to_string(),
            prev_hash: self.last_hash.clone(),
            entry_hash,
        };
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        writeln!(file, "{}", serde_json::to_string(&entry)?)?;
        self.last_hash = entry.entry_hash.clone();
        Ok(entry)
    }

    pub fn verify_chain(path: impl AsRef<Path>) -> std::io::Result<bool> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(true);
        }
        let mut prev = String::from("0");
        let file = File::open(path)?;
        for line in BufReader::new(file).lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let entry: AuditEntry = serde_json::from_str(&line)?;
            if entry.prev_hash != prev {
                return Ok(false);
            }
            let payload = format!(
                "{}|{}|{}|{}|{}|{}",
                entry.seq,
                entry.timestamp_ms,
                entry.actor,
                entry.action,
                entry.detail,
                entry.prev_hash
            );
            let expected = format!("{}", blake3::hash(payload.as_bytes()).to_hex());
            if expected != entry.entry_hash {
                return Ok(false);
            }
            prev = entry.entry_hash;
        }
        Ok(true)
    }
}

fn chrono_lite_now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_links_and_verifies() {
        let path = std::env::temp_dir().join("patch-audit-chain-test.jsonl");
        let _ = std::fs::remove_file(&path);
        let mut ledger = AuditLedger::open(&path).unwrap();
        ledger.append("admin", "console", "say hi").unwrap();
        ledger.append("admin", "config", "edit velocity.toml").unwrap();
        assert!(AuditLedger::verify_chain(&path).unwrap());
    }
}
