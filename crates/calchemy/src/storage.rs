use anyhow::{Context, Result};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use std::path::Path;

/// One appointment, parsed from a line `DATE [START [END]] TITLE`.
///
/// No delimiter marks where the machine fields end and `TITLE` begins —
/// parsing is purely shape-based: `DATE` is `YYYY-MM-DD`, `START`/times are
/// `HH:MM`, and the longest matching prefix of leading date/time-shaped
/// tokens is claimed, greedily, before whatever's left becomes the title.
///
/// - `START` is `HH:MM`; absent for an all-day event.
/// - `END` is `HH:MM` (same day) or `YYYY-MM-DD HH:MM` (crosses to a later day).
///   With no `START`, a bare `YYYY-MM-DD` END makes a multi-day all-day event
///   (inclusive last day).
///
/// Accepted tradeoff: a title that itself opens with well-formed date/time
/// tokens (e.g. "09:00 sync") can have those leading words misclaimed as
/// machine fields rather than title text. Rare in practice; not worth an
/// escaping/quoting scheme.
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
        let tokens: Vec<&str> = line.split_whitespace().collect();
        anyhow::ensure!(!tokens.is_empty(), "missing date in line: {:?}", line);
        let date = parse_date(tokens[0]).with_context(|| format!("invalid date {:?}", tokens[0]))?;

        let rest = &tokens[1..];
        // Longest shape-match first: `TIME DATE TIME` (start day/time to a
        // later end day/time), then `TIME TIME` (same-day end), then a lone
        // `TIME` (start only) or `DATE` (bare end date, multi-day all-day).
        let (start, end, consumed) = match rest {
            [s, ed, et, ..] if parse_time(s).is_ok() && parse_date(ed).is_ok() && parse_time(et).is_ok() => {
                (Some(parse_time(s)?), Some(parse_date(ed)?.and_time(parse_time(et)?)), 3)
            }
            [s, e, ..] if parse_time(s).is_ok() && parse_time(e).is_ok() => {
                (Some(parse_time(s)?), Some(date.and_time(parse_time(e)?)), 2)
            }
            [s, ..] if parse_time(s).is_ok() => (Some(parse_time(s)?), None, 1),
            [ed, ..] if parse_date(ed).is_ok() => {
                let end_date = parse_date(ed)?;
                anyhow::ensure!(end_date > date, "end date not after start in line: {:?}", line);
                (None, Some(end_date.and_time(NaiveTime::MIN)), 1)
            }
            _ => (None, None, 0),
        };

        let title = rest[consumed..].join(" ");
        anyhow::ensure!(!title.is_empty(), "missing title in line: {:?}", line);

        Ok(Appt { date, start, end, title, raw: line.to_string() })
    }

    /// Sort/comparison key: the start instant, with all-day events at day start.
    pub fn start_dt(&self) -> NaiveDateTime {
        self.date.and_time(self.start.unwrap_or(NaiveTime::MIN))
    }

    /// The last calendar day this appointment touches (inclusive).
    pub fn last_date(&self) -> NaiveDate {
        self.end.map(|e| e.date()).unwrap_or(self.date)
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
        } else if let Some(end) = self.end {
            s.push_str(&format!(" {}", end.format("%Y-%m-%d")));
        }
        format!("{} {}", s, self.title)
    }
}

fn parse_time(s: &str) -> Result<NaiveTime> {
    NaiveTime::parse_from_str(s, "%H:%M").with_context(|| format!("invalid time {:?}", s))
}

fn parse_date(s: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").with_context(|| format!("invalid date {:?}", s))
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
        let a = Appt::parse("2026-12-25 Christmas").unwrap();
        assert_eq!(a.date, NaiveDate::from_ymd_opt(2026, 12, 25).unwrap());
        assert_eq!(a.start, None);
        assert_eq!(a.end, None);
        assert_eq!(a.title, "Christmas");
    }

    #[test]
    fn parse_start_only() {
        let a = Appt::parse("2026-07-14 09:00 Team sync").unwrap();
        assert_eq!(a.start, NaiveTime::from_hms_opt(9, 0, 0));
        assert_eq!(a.end, None);
        assert_eq!(a.title, "Team sync");
    }

    #[test]
    fn parse_same_day_end() {
        let a = Appt::parse("2026-07-14 09:00 10:00 Dentist @clinic").unwrap();
        assert_eq!(a.start, NaiveTime::from_hms_opt(9, 0, 0));
        assert_eq!(a.end.unwrap(), NaiveDate::from_ymd_opt(2026, 7, 14).unwrap().and_hms_opt(10, 0, 0).unwrap());
        assert_eq!(a.title, "Dentist @clinic");
    }

    #[test]
    fn parse_cross_day_end() {
        let a = Appt::parse("2026-07-14 20:00 2026-07-15 02:00 Party").unwrap();
        assert_eq!(a.end.unwrap(), NaiveDate::from_ymd_opt(2026, 7, 15).unwrap().and_hms_opt(2, 0, 0).unwrap());
    }

    #[test]
    fn parse_multi_day_all_day() {
        let a = Appt::parse("2026-07-17 2026-07-20 Wedding").unwrap();
        assert_eq!(a.start, None);
        assert_eq!(a.end.unwrap(), NaiveDate::from_ymd_opt(2026, 7, 20).unwrap().and_time(NaiveTime::MIN));
        assert_eq!(a.last_date(), NaiveDate::from_ymd_opt(2026, 7, 20).unwrap());
        assert_eq!(a.title, "Wedding");
    }

    #[test]
    fn parse_rejects_end_date_not_after_start() {
        assert!(Appt::parse("2026-07-17 2026-07-17 x").is_err());
        assert!(Appt::parse("2026-07-17 2026-07-16 x").is_err());
    }

    #[test]
    fn parse_rejects_missing_title() {
        assert!(Appt::parse("2026-07-14 09:00").is_err());
        assert!(Appt::parse("2026-07-14").is_err());
    }

    #[test]
    fn parse_rejects_bad_date() {
        assert!(Appt::parse("not-a-date x").is_err());
    }

    #[test]
    fn to_line_round_trips() {
        for line in [
            "2026-12-25 Christmas",
            "2026-07-14 09:00 Team sync",
            "2026-07-14 09:00 10:00 Dentist @clinic",
            "2026-07-14 20:00 2026-07-15 02:00 Party",
            "2026-07-17 2026-07-20 Wedding",
        ] {
            assert_eq!(Appt::parse(line).unwrap().to_line(), line);
        }
    }

    #[test]
    fn start_dt_orders_all_day_first() {
        let allday = Appt::parse("2026-07-14 Holiday").unwrap();
        let timed = Appt::parse("2026-07-14 09:00 Standup").unwrap();
        assert!(allday.start_dt() < timed.start_dt());
    }

    #[test]
    fn known_edge_case_title_swallows_leading_time_shaped_word() {
        // Accepted tradeoff: a title that itself opens with a well-formed
        // time collides with START, documented rather than escaped around.
        let a = Appt::parse("2026-07-14 09:00 sync").unwrap();
        assert_eq!(a.start, NaiveTime::from_hms_opt(9, 0, 0));
        assert_eq!(a.title, "sync");
    }

    #[test]
    fn known_edge_case_title_swallows_full_end_shape() {
        let a = Appt::parse("2026-07-14 09:00 2026-08-01 17:00 conference debrief").unwrap();
        assert_eq!(a.end.unwrap(), NaiveDate::from_ymd_opt(2026, 8, 1).unwrap().and_hms_opt(17, 0, 0).unwrap());
        assert_eq!(a.title, "conference debrief");
    }
}
