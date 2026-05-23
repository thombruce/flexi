use crate::time::{format_duration, parse_duration};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

pub struct LogEntry {
    pub timestamp: String,
    pub prev: i32,
    pub new: i32,
    pub description: String,
}

pub fn log_path(path: &Path) -> PathBuf {
    path.with_extension("log")
}

pub fn append_log(path: &Path, prev: i32, new: i32, description: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating directory {:?}", parent))?;
    }
    let timestamp = chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, false);
    let line = format!("{}\t{}\t{}\t{}\n", timestamp, prev, new, description);
    let tmp = path.with_extension("log.tmp");
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
    let tmp = path.with_extension("log.tmp");
    let content = lines.join("\n");
    let content = if content.is_empty() { content } else { content + "\n" };
    std::fs::write(&tmp, content)
        .with_context(|| format!("writing {:?}", tmp))?;
    std::fs::rename(&tmp, path)
        .with_context(|| format!("renaming {:?} to {:?}", tmp, path))?;
    Ok(Some(last))
}

fn parse_log_line(line: &str) -> Result<LogEntry> {
    let parts: Vec<&str> = line.splitn(4, '\t').collect();
    anyhow::ensure!(parts.len() == 4, "malformed log line: {:?}", line);
    Ok(LogEntry {
        timestamp: parts[0].to_string(),
        prev: parts[1].parse().with_context(|| format!("parsing prev in {:?}", line))?,
        new: parts[2].parse().with_context(|| format!("parsing new in {:?}", line))?,
        description: parts[3].to_string(),
    })
}

pub fn read_minutes(path: &Path) -> Result<i32> {
    if !path.exists() {
        return Ok(0);
    }
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("reading {:?}", path))?;
    parse_duration(raw.trim()).with_context(|| format!("parsing time in {:?}", path))
}

pub fn write_minutes(path: &Path, mins: i32) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating directory {:?}", parent))?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, format_duration(mins))
        .with_context(|| format!("writing {:?}", tmp))?;
    std::fs::rename(&tmp, path)
        .with_context(|| format!("renaming {:?} to {:?}", tmp, path))
}
