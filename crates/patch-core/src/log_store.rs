use crate::error::Result;
use crate::process::LogLine;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;

const MAX_TAIL_LINES: usize = 2000;

pub fn log_file_path(logs_dir: &Path, network_id: Uuid) -> PathBuf {
    logs_dir.join(format!("{network_id}.log"))
}

pub fn append_log_line(logs_dir: &Path, line: &LogLine) -> Result<()> {
    std::fs::create_dir_all(logs_dir)?;
    let path = log_file_path(logs_dir, line.network_id);
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    let row = format!(
        "{}\t{}\t{}\n",
        chrono::Utc::now().timestamp(),
        line.source.replace('\t', " "),
        line.line.replace('\n', " ")
    );
    file.write_all(row.as_bytes())?;
    Ok(())
}

pub fn read_log_tail(logs_dir: &Path, network_id: Uuid, limit: usize) -> Result<Vec<LogLine>> {
    let path = log_file_path(logs_dir, network_id);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let file = File::open(&path)?;
    let reader = BufReader::new(file);
    let mut lines: Vec<LogLine> = Vec::new();
    for row in reader.lines() {
        let row = row?;
        if row.trim().is_empty() {
            continue;
        }
        let parts: Vec<&str> = row.splitn(3, '\t').collect();
        if parts.len() < 3 {
            continue;
        }
        lines.push(LogLine {
            network_id,
            source: parts[1].to_string(),
            line: parts[2].to_string(),
        });
    }
    let keep = limit.min(MAX_TAIL_LINES);
    if lines.len() > keep {
        lines = lines.split_off(lines.len() - keep);
    }
    Ok(lines)
}

pub fn clear_logs(logs_dir: &Path, network_id: Uuid) -> Result<()> {
    let path = log_file_path(logs_dir, network_id);
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}
