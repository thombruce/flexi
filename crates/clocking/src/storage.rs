use crate::config::TimestampFormat;
use crate::time::parse_duration;
use anyhow::{Context, Result};
use std::path::Path;

pub struct LogEntry {
    pub timestamp: String,
    pub description: String,
}

impl LogEntry {
    fn body(&self) -> &str {
        self.description.split(" # ").next().unwrap_or(&self.description)
    }

    /// True if this entry is an open session marker (`@in`).
    pub fn is_open(&self) -> bool {
        self.body() == "@in"
    }

    /// Worked minutes for a completed session line (`17:00 = 8 hr 30 min`, the
    /// recorded duration after ` = `). Returns None for open `@in` markers.
    pub fn session_minutes(&self) -> Option<i32> {
        if self.is_open() {
            return None;
        }
        let (_, dur) = self.body().rsplit_once(" = ")?;
        parse_duration(dur).ok()
    }
}

/// The current local time rendered in the configured timestamp format.
pub fn now_timestamp(ts_format: TimestampFormat) -> String {
    let now = chrono::Local::now();
    match ts_format {
        TimestampFormat::Simple => now.format("%Y-%m-%d %H:%M").to_string(),
        TimestampFormat::Full => now.to_rfc3339_opts(chrono::SecondsFormat::Secs, false),
    }
}

/// Appends a line `<timestamp> <description>`. The caller supplies the leading
/// timestamp so a closed session can be keyed by its clock-in time (see the
/// `out` handler), not the moment the line is written.
pub fn append_log(path: &Path, timestamp: &str, description: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating directory {:?}", parent))?;
    }
    let line = format!("{} {}\n", timestamp, description);
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("opening {:?}", path))?;
    file.write_all(line.as_bytes())
        .with_context(|| format!("writing {:?}", path))
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

pub fn last_entry(path: &Path) -> Result<Option<LogEntry>> {
    if !path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("reading {:?}", path))?;
    match raw.lines().rev().find(|l| !l.is_empty()) {
        None => Ok(None),
        Some(line) => Ok(Some(parse_log_line(line)?)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(desc: &str) -> LogEntry {
        LogEntry { timestamp: "2026-07-11 17:30".to_string(), description: desc.to_string() }
    }

    #[test]
    fn open_marker_detection() {
        assert!(entry("@in").is_open());
        assert!(entry("@in # project x").is_open());
        assert!(!entry("17:30 = 8 hr 30 min").is_open());
    }

    #[test]
    fn session_minutes_parses_duration_after_equals() {
        assert_eq!(entry("17:30 = 8 hr 30 min").session_minutes(), Some(510));
    }

    #[test]
    fn session_minutes_cross_day_end() {
        assert_eq!(entry("2026-07-12 04:00 = 8 hr").session_minutes(), Some(480));
    }

    #[test]
    fn session_minutes_strips_note() {
        assert_eq!(entry("11:00 = 2 hr # standup").session_minutes(), Some(120));
    }

    #[test]
    fn open_marker_has_no_session_minutes() {
        assert_eq!(entry("@in").session_minutes(), None);
        assert_eq!(entry("@in # note").session_minutes(), None);
    }

    #[test]
    fn parse_log_line_simple_and_full() {
        let e = parse_log_line("2026-07-11 09:00 17:30 = 8 hr 30 min").unwrap();
        assert_eq!(e.timestamp, "2026-07-11 09:00");
        assert_eq!(e.description, "17:30 = 8 hr 30 min");
        let f = parse_log_line("2026-07-11T09:00:16+01:00 @in").unwrap();
        assert_eq!(f.timestamp, "2026-07-11T09:00:16+01:00");
        assert_eq!(f.description, "@in");
    }

    #[test]
    fn parse_log_line_rejects_malformed() {
        assert!(parse_log_line("2026-07-11").is_err());
        assert!(parse_log_line("2026-07-11 17:30").is_err());
    }
}
