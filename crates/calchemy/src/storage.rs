use anyhow::{Context, Result};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use std::path::Path;

/// One appointment, parsed from a line `DATE [START [END]] # TITLE`.
///
/// - `START` is `HH:MM`; absent for an all-day event.
/// - `END` is `HH:MM` (same day) or `YYYY-MM-DD HH:MM` (crosses to a later day).
#[derive(Debug, Clone, PartialEq)]
pub struct Appt {
    pub date: NaiveDate,
    pub start: Option<NaiveTime>,
    pub end: Option<NaiveDateTime>,
    pub title: String,
    /// The exact source line, so `rm` can delete it without reformatting.
    pub raw: String,
}

impl Appt {
    pub fn parse(line: &str) -> Result<Appt> {
        let (left, title) = line
            .split_once(" # ")
            .with_context(|| format!("missing ' # title' in line: {:?}", line))?;
        let title = title.trim();
        anyhow::ensure!(!title.is_empty(), "empty title in line: {:?}", line);

        let tokens: Vec<&str> = left.split_whitespace().collect();
        anyhow::ensure!(!tokens.is_empty(), "missing date in line: {:?}", line);
        let date = NaiveDate::parse_from_str(tokens[0], "%Y-%m-%d")
            .with_context(|| format!("invalid date {:?}", tokens[0]))?;

        let (start, end) = match tokens[1..] {
            [] => (None, None),
            [s] => (Some(parse_time(s)?), None),
            [s, e] => {
                let start = parse_time(s)?;
                (Some(start), Some(date.and_time(parse_time(e)?)))
            }
            [s, ed, et] => {
                let start = parse_time(s)?;
                let end_date = NaiveDate::parse_from_str(ed, "%Y-%m-%d")
                    .with_context(|| format!("invalid end date {:?}", ed))?;
                (Some(start), Some(end_date.and_time(parse_time(et)?)))
            }
            _ => anyhow::bail!("too many time fields in line: {:?}", line),
        };

        Ok(Appt { date, start, end, title: title.to_string(), raw: line.to_string() })
    }

    /// Sort/comparison key: the start instant, with all-day events at day start.
    pub fn start_dt(&self) -> NaiveDateTime {
        self.date.and_time(self.start.unwrap_or(NaiveTime::MIN))
    }

    /// Renders the canonical storage line for this appointment.
    pub fn to_line(&self) -> String {
        let mut s = self.date.format("%Y-%m-%d").to_string();
        if let Some(start) = self.start {
            s.push_str(&format!(" {}", start.format("%H:%M")));
            if let Some(end) = self.end {
                if end.date() == self.date {
                    s.push_str(&format!(" {}", end.format("%H:%M")));
                } else {
                    s.push_str(&format!(" {}", end.format("%Y-%m-%d %H:%M")));
                }
            }
        }
        format!("{} # {}", s, self.title)
    }
}

fn parse_time(s: &str) -> Result<NaiveTime> {
    NaiveTime::parse_from_str(s, "%H:%M").with_context(|| format!("invalid time {:?}", s))
}

fn read_lines(path: &Path) -> Result<Vec<String>> {
    if !path.exists() {
        return Ok(vec![]);
    }
    let raw = std::fs::read_to_string(path).with_context(|| format!("reading {:?}", path))?;
    Ok(raw.lines().filter(|l| !l.trim().is_empty()).map(str::to_string).collect())
}

pub fn read_appts(path: &Path) -> Result<Vec<Appt>> {
    read_lines(path)?.iter().map(|l| Appt::parse(l)).collect()
}

pub fn append_appt(path: &Path, appt: &Appt) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("creating directory {:?}", parent))?;
    }
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("opening {:?}", path))?;
    writeln!(file, "{}", appt.to_line()).with_context(|| format!("writing {:?}", path))
}

/// Rewrites the file to exactly `lines` (atomic via a `.tmp` swap).
pub fn write_lines(path: &Path, lines: &[String]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("creating directory {:?}", parent))?;
    }
    let tmp = path.with_extension("tmp");
    let mut content = lines.join("\n");
    if !content.is_empty() {
        content.push('\n');
    }
    std::fs::write(&tmp, content).with_context(|| format!("writing {:?}", tmp))?;
    std::fs::rename(&tmp, path).with_context(|| format!("renaming {:?} to {:?}", tmp, path))
}

/// Removes the first line exactly equal to `raw`; returns the remaining lines.
pub fn remove_line(path: &Path, raw: &str) -> Result<()> {
    let mut lines = read_lines(path)?;
    if let Some(pos) = lines.iter().position(|l| l == raw) {
        lines.remove(pos);
    }
    write_lines(path, &lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_all_day() {
        let a = Appt::parse("2026-12-25 # Christmas").unwrap();
        assert_eq!(a.date, NaiveDate::from_ymd_opt(2026, 12, 25).unwrap());
        assert_eq!(a.start, None);
        assert_eq!(a.end, None);
        assert_eq!(a.title, "Christmas");
    }

    #[test]
    fn parse_start_only() {
        let a = Appt::parse("2026-07-14 09:00 # Team sync").unwrap();
        assert_eq!(a.start, NaiveTime::from_hms_opt(9, 0, 0));
        assert_eq!(a.end, None);
        assert_eq!(a.title, "Team sync");
    }

    #[test]
    fn parse_same_day_end() {
        let a = Appt::parse("2026-07-14 09:00 10:00 # Dentist @clinic").unwrap();
        assert_eq!(a.start, NaiveTime::from_hms_opt(9, 0, 0));
        assert_eq!(a.end.unwrap(), NaiveDate::from_ymd_opt(2026, 7, 14).unwrap().and_hms_opt(10, 0, 0).unwrap());
        assert_eq!(a.title, "Dentist @clinic");
    }

    #[test]
    fn parse_cross_day_end() {
        let a = Appt::parse("2026-07-14 20:00 2026-07-15 02:00 # Party").unwrap();
        assert_eq!(a.end.unwrap(), NaiveDate::from_ymd_opt(2026, 7, 15).unwrap().and_hms_opt(2, 0, 0).unwrap());
    }

    #[test]
    fn parse_rejects_missing_title() {
        assert!(Appt::parse("2026-07-14 09:00").is_err());
        assert!(Appt::parse("2026-07-14 09:00 # ").is_err());
    }

    #[test]
    fn parse_rejects_bad_date() {
        assert!(Appt::parse("not-a-date # x").is_err());
    }

    #[test]
    fn to_line_round_trips() {
        for line in [
            "2026-12-25 # Christmas",
            "2026-07-14 09:00 # Team sync",
            "2026-07-14 09:00 10:00 # Dentist @clinic",
            "2026-07-14 20:00 2026-07-15 02:00 # Party",
        ] {
            assert_eq!(Appt::parse(line).unwrap().to_line(), line);
        }
    }

    #[test]
    fn start_dt_orders_all_day_first() {
        let allday = Appt::parse("2026-07-14 # Holiday").unwrap();
        let timed = Appt::parse("2026-07-14 09:00 # Standup").unwrap();
        assert!(allday.start_dt() < timed.start_dt());
    }
}
