//! Dates and times as they appear in record lines and CLI arguments
//! (`dates` feature).
//!
//! The rule is bounded leniency, the same for files and arguments: a date is
//! a 4-digit year then a 1–2 digit month and day (`2030-1-5`), and a time is
//! a 1–2 digit hour then exactly 2 minute digits (`9:00`). Anything looser
//! (chrono alone accepts `1-2-3`, `24-1-5`, `+2030-01-05`, `9:5`) is not a
//! date or time, so ordinary title words aren't misclaimed. Always write them
//! back padded (`%Y-%m-%d`, `%H:%M`).

use chrono::{NaiveDate, NaiveTime};

fn digits(s: &str, min: usize, max: usize) -> Option<u32> {
    if (min..=max).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_digit()) {
        s.parse().ok()
    } else {
        None
    }
}

/// Parses `YYYY-M-D`, with 1–2 digit month and day.
pub fn parse_date(s: &str) -> Option<NaiveDate> {
    let mut parts = s.split('-');
    let (y, m, d) = (parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some() {
        return None;
    }
    NaiveDate::from_ymd_opt(digits(y, 4, 4)? as i32, digits(m, 1, 2)?, digits(d, 1, 2)?)
}

/// Parses `H:MM`, with a 1–2 digit hour and exactly 2 minute digits.
pub fn parse_time(s: &str) -> Option<NaiveTime> {
    let (h, m) = s.split_once(':')?;
    NaiveTime::from_hms_opt(digits(h, 1, 2)?, digits(m, 2, 2)?, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_padded_and_bounded_unpadded_dates() {
        let d = NaiveDate::from_ymd_opt(2030, 1, 5);
        assert_eq!(parse_date("2030-01-05"), d);
        assert_eq!(parse_date("2030-1-5"), d);
    }

    #[test]
    fn rejects_looser_dates() {
        for s in ["1-2-3", "24-1-5", "+2030-01-05", "02030-01-05", "2030-001-05", "2030-02-30", "2030-01-05-1", "2030-01"] {
            assert_eq!(parse_date(s), None, "{s}");
        }
    }

    #[test]
    fn accepts_bounded_times_and_rejects_looser_ones() {
        let t = NaiveTime::from_hms_opt(9, 0, 0);
        assert_eq!(parse_time("09:00"), t);
        assert_eq!(parse_time("9:00"), t);
        for s in ["9:5", "09:5", "900", "24:00", "9:60", "009:00", "+9:00"] {
            assert_eq!(parse_time(s), None, "{s}");
        }
    }
}
