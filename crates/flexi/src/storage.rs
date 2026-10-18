use crate::config::TimestampFormat;
use crate::time::parse_duration;
use anyhow::Result;
use std::path::Path;

pub struct LogEntry {
    pub timestamp: String,
    pub description: String,
}

impl LogEntry {
    pub fn delta_minutes(&self) -> Option<i32> {
        let base = self.description.split(" # ").next().unwrap_or(&self.description);
        let desc = base
            .replace(" -> ", " > ")
            .replace(" \u{2192} ", " > ");
        let pos = desc.find(" > ")?;
        parse_duration(&desc[..pos]).ok()
    }

    pub fn new_minutes(&self) -> Result<i32> {
        let desc = self.description.split(" # ").next().unwrap_or(&self.description);
        if let Some(pos) = desc.rfind(" > ") {
            parse_duration(&desc[pos + 3..])
        } else if let Some(pos) = desc.rfind(" \u{2192} ") {
            parse_duration(&desc[pos + 4..])
        } else if let Some(pos) = desc.rfind(" -> ") {
            parse_duration(&desc[pos + 4..])
        } else if let Some(stripped) = desc.strip_prefix("= ") {
            parse_duration(stripped)
        } else if let Some(stripped) = desc.strip_prefix("@in ") {
            parse_duration(stripped)
        } else if let Some(stripped) = desc.strip_prefix("@out ") {
            parse_duration(stripped)
        } else {
            anyhow::bail!("cannot parse value from log entry: {:?}", self.description)
        }
    }

    fn marker_body(&self) -> &str {
        self.description.split(" # ").next().unwrap_or(&self.description)
    }

    /// True if this entry is an open session marker (`@in`/`@out <balance>`).
    pub fn is_open_marker(&self) -> bool {
        let b = self.marker_body();
        b.starts_with("@in ") || b.starts_with("@out ")
    }

    /// True if the open marker is a spending session (`@out <balance>`).
    pub fn is_spend(&self) -> bool {
        self.marker_body().starts_with("@out ")
    }
}

pub fn append_log(path: &Path, description: &str, ts_format: TimestampFormat) -> Result<()> {
    let now = chrono::Local::now();
    let timestamp = match ts_format {
        TimestampFormat::Simple => now.format("%Y-%m-%d %H:%M").to_string(),
        TimestampFormat::Full => now.to_rfc3339_opts(chrono::SecondsFormat::Secs, false),
    };
    foolscap::append_line(path, &format!("{} {}", timestamp, description))
}

pub fn read_log(path: &Path) -> Result<Vec<LogEntry>> {
    foolscap::read_lines(path)?.iter().map(|l| parse_log_line(l)).collect()
}

pub fn pop_log(path: &Path) -> Result<Option<LogEntry>> {
    let mut lines = foolscap::read_lines(path)?;
    let Some(last) = lines.pop() else {
        return Ok(None);
    };
    let last = parse_log_line(&last)?;
    foolscap::write_lines(path, &lines)?;
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
    foolscap::read_lines(path)?.pop().map(|l| parse_log_line(&l)).transpose()
}

pub fn read_minutes(path: &Path) -> Result<i32> {
    last_entry(path)?.map_or(Ok(0), |e| e.new_minutes())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(desc: &str) -> LogEntry {
        LogEntry { timestamp: "2026-05-24 10:20".to_string(), description: desc.to_string() }
    }

    #[test]
    fn delta_minutes_positive() {
        assert_eq!(entry("+45 min > 4 hr 45 min").delta_minutes(), Some(45));
    }

    #[test]
    fn delta_minutes_negative() {
        assert_eq!(entry("-30 min > 4 hr").delta_minutes(), Some(-30));
    }

    #[test]
    fn delta_minutes_set_returns_none() {
        assert_eq!(entry("= 4 hr").delta_minutes(), None);
    }

    #[test]
    fn delta_minutes_unicode_arrow_compat() {
        assert_eq!(entry("+45 min → 4 hr 45 min").delta_minutes(), Some(45));
    }

    #[test]
    fn delta_minutes_ascii_arrow_compat() {
        assert_eq!(entry("+1 hr -> 1 hr").delta_minutes(), Some(60));
    }

    #[test]
    fn new_minutes_gt_positive() {
        assert_eq!(entry("+45 min > 4 hr 45 min").new_minutes().unwrap(), 285);
    }

    #[test]
    fn new_minutes_gt_negative() {
        assert_eq!(entry("-30 min > -30 min").new_minutes().unwrap(), -30);
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
    fn new_minutes_unicode_arrow_compat() {
        assert_eq!(entry("+45 min → 4 hr 45 min").new_minutes().unwrap(), 285);
    }

    #[test]
    fn new_minutes_ascii_arrow_compat() {
        assert_eq!(entry("+45 min -> 4 hr 45 min").new_minutes().unwrap(), 285);
    }

    #[test]
    fn new_minutes_bad_description() {
        assert!(entry("bogus description").new_minutes().is_err());
    }

    #[test]
    fn new_minutes_session_markers() {
        assert_eq!(entry("@in 2 hr").new_minutes().unwrap(), 120);
        assert_eq!(entry("@in 0 min").new_minutes().unwrap(), 0);
        assert_eq!(entry("@out 2 hr").new_minutes().unwrap(), 120);
    }

    #[test]
    fn session_markers_have_no_delta() {
        assert_eq!(entry("@in 2 hr").delta_minutes(), None);
        assert_eq!(entry("@out 2 hr").delta_minutes(), None);
    }

    #[test]
    fn open_marker_detection() {
        assert!(entry("@in 2 hr").is_open_marker());
        assert!(entry("@in 2 hr # project x").is_open_marker());
        assert!(entry("@out 30 min").is_open_marker());
        assert!(!entry("+30 min > 2 hr").is_open_marker());
        assert!(!entry("= 0 min").is_open_marker());
        assert!(!entry("+0 min > 2 hr # @in joke note").is_open_marker());
    }

    #[test]
    fn spend_detection() {
        assert!(entry("@out 30 min").is_spend());
        assert!(entry("@out 30 min # dentist").is_spend());
        assert!(!entry("@in 2 hr").is_spend());
        assert!(!entry("-30 min > 1 hr").is_spend());
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
