use crate::config::TimestampFormat;
use crate::time::parse_duration;
use anyhow::{Context, Result};
use std::path::Path;

pub struct LogEntry {
    pub timestamp: String,
    pub description: String,
}

impl LogEntry {
    pub fn new_minutes(&self) -> Result<i32> {
        if let Some(pos) = self.description.rfind(" \u{2192} ") {
            parse_duration(&self.description[pos + 4..])
        } else if let Some(stripped) = self.description.strip_prefix("= ") {
            parse_duration(stripped)
        } else {
            anyhow::bail!("cannot parse value from log entry: {:?}", self.description)
        }
    }
}

pub fn append_log(path: &Path, description: &str, ts_format: TimestampFormat) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating directory {:?}", parent))?;
    }
    let now = chrono::Local::now();
    let timestamp = match ts_format {
        TimestampFormat::Simple => now.format("%Y-%m-%d %H:%M").to_string(),
        TimestampFormat::Full => now.to_rfc3339_opts(chrono::SecondsFormat::Secs, false),
    };
    let line = format!("{} {}\n", timestamp, description);
    let tmp = path.with_extension("tmp");
    let existing = if path.exists() {
        std::fs::read_to_string(path).with_context(|| format!("reading {:?}", path))?
    } else {
        String::new()
    };
    std::fs::write(&tmp, existing + &line)
        .with_context(|| format!("writing {:?}", tmp))?;
    std::fs::rename(&tmp, path)
        .with_context(|| format!("renaming {:?} to {:?}", tmp, path))
}

pub fn read_log(path: &Path) -> Result<Vec<LogEntry>> {
    if !path.exists() {
        return Ok(vec![]);
    }
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("reading {:?}", path))?;
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(parse_log_line)
        .collect()
}

pub fn pop_log(path: &Path) -> Result<Option<LogEntry>> {
    if !path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("reading {:?}", path))?;
    let mut lines: Vec<&str> = raw.lines().filter(|l| !l.is_empty()).collect();
    if lines.is_empty() {
        return Ok(None);
    }
    let last = parse_log_line(lines.pop().unwrap())?;
    let tmp = path.with_extension("tmp");
    let content = lines.join("\n");
    let content = if content.is_empty() { content } else { content + "\n" };
    std::fs::write(&tmp, content)
        .with_context(|| format!("writing {:?}", tmp))?;
    std::fs::rename(&tmp, path)
        .with_context(|| format!("renaming {:?} to {:?}", tmp, path))?;
    Ok(Some(last))
}

fn parse_log_line(line: &str) -> Result<LogEntry> {
    let ts_len = if line.len() > 10 && line.as_bytes()[10] == b'T' { 25 } else { 16 };
    anyhow::ensure!(line.len() > ts_len, "malformed log line: {:?}", line);
    let (ts, rest) = line.split_at(ts_len);
    let desc = rest.trim_start();
    anyhow::ensure!(!desc.is_empty(), "malformed log line: {:?}", line);
    Ok(LogEntry { timestamp: ts.to_string(), description: desc.to_string() })
}

pub fn read_minutes(path: &Path) -> Result<i32> {
    let entries = read_log(path)?;
    match entries.last() {
        None => Ok(0),
        Some(e) => e.new_minutes(),
    }
}
