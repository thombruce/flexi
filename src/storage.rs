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
        } else if let Some(pos) = self.description.rfind(" -> ") {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(desc: &str) -> LogEntry {
        LogEntry { timestamp: "2026-05-24 10:20".to_string(), description: desc.to_string() }
    }

    #[test]
    fn new_minutes_arrow_positive() {
        assert_eq!(entry("+45 min → 4 hr 45 min").new_minutes().unwrap(), 285);
    }

    #[test]
    fn new_minutes_arrow_negative() {
        assert_eq!(entry("-30 min → -30 min").new_minutes().unwrap(), -30);
    }

    #[test]
    fn new_minutes_set() {
        assert_eq!(entry("= 4 hr").new_minutes().unwrap(), 240);
    }

    #[test]
    fn new_minutes_set_zero() {
        assert_eq!(entry("= 0 min").new_minutes().unwrap(), 0);
    }

    #[test]
    fn new_minutes_ascii_arrow() {
        assert_eq!(entry("+45 min -> 4 hr 45 min").new_minutes().unwrap(), 285);
    }

    #[test]
    fn new_minutes_bad_description() {
        assert!(entry("bogus description").new_minutes().is_err());
    }

    #[test]
    fn parse_log_line_space_separator() {
        let e = parse_log_line("2026-05-24 10:20 +1 hr → 1 hr").unwrap();
        assert_eq!(e.timestamp, "2026-05-24 10:20");
        assert_eq!(e.description, "+1 hr → 1 hr");
    }

    #[test]
    fn parse_log_line_tab_separator() {
        let e = parse_log_line("2026-05-24 10:20\t+1 hr → 1 hr").unwrap();
        assert_eq!(e.timestamp, "2026-05-24 10:20");
        assert_eq!(e.description, "+1 hr → 1 hr");
    }

    #[test]
    fn parse_log_line_multiple_spaces() {
        let e = parse_log_line("2026-05-24 10:20   +1 hr → 1 hr").unwrap();
        assert_eq!(e.timestamp, "2026-05-24 10:20");
        assert_eq!(e.description, "+1 hr → 1 hr");
    }

    #[test]
    fn parse_log_line_full_timestamp() {
        let e = parse_log_line("2026-05-24T10:20:16+01:00 +1 hr → 1 hr").unwrap();
        assert_eq!(e.timestamp, "2026-05-24T10:20:16+01:00");
        assert_eq!(e.description, "+1 hr → 1 hr");
    }

    #[test]
    fn parse_log_line_too_short() {
        assert!(parse_log_line("2026-05-24").is_err());
    }

    #[test]
    fn parse_log_line_empty_description() {
        assert!(parse_log_line("2026-05-24 10:20").is_err());
        assert!(parse_log_line("2026-05-24 10:20   ").is_err());
    }
}
