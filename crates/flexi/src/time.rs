use anyhow::{bail, Result};

pub fn parse_duration(s: &str) -> Result<i32> {
    let s = s.trim().to_lowercase();
    let (negative, s) = if let Some(stripped) = s.strip_prefix('-') {
        (true, stripped.trim_start().to_string())
    } else {
        (false, s)
    };

    if s.is_empty() {
        bail!("empty time string");
    }

    let s = s.replace(',', ".");

    let total = if !s.contains(' ') {
        parse_compact(&s)
            .ok_or_else(|| anyhow::anyhow!("invalid time format {:?}", s))?
    } else {
        parse_spaced(&s)?
    };

    Ok(if negative { -total } else { total })
}

fn parse_compact(s: &str) -> Option<i32> {
    if let Ok(n) = s.parse::<f64>() {
        return Some((n * 60.0).round() as i32);
    }
    if let Some(h_pos) = s.find('h') {
        let hours: f64 = if s[..h_pos].is_empty() { 0.0 } else { s[..h_pos].parse().ok()? };
        let rest = &s[h_pos + 1..];
        let mins: f64 = if rest.is_empty() {
            0.0
        } else {
            rest.strip_suffix('m')?.parse().ok()?
        };
        Some((hours * 60.0 + mins).round() as i32)
    } else if let Some(stripped) = s.strip_suffix('m') {
        let mins: f64 = stripped.parse().ok()?;
        Some(mins.round() as i32)
    } else {
        None
    }
}

fn parse_spaced(s: &str) -> Result<i32> {
    let tokens: Vec<&str> = s.split_whitespace().collect();
    let mut total_mins: i32 = 0;
    let mut i = 0;

    while i < tokens.len() {
        let n: f64 = tokens[i]
            .parse()
            .map_err(|_| anyhow::anyhow!("expected number, got {:?}", tokens[i]))?;
        i += 1;

        if i >= tokens.len() {
            bail!("expected unit after {}", n);
        }

        match tokens[i] {
            "hr" | "hrs" | "hour" | "hours" => total_mins += (n * 60.0).round() as i32,
            "min" | "mins" | "minute" | "minutes" => total_mins += n.round() as i32,
            u => bail!("unknown time unit {:?}", u),
        }
        i += 1;
    }

    Ok(total_mins)
}

/// Rounds a duration to the nearest multiple of `increment` minutes (half away
/// from zero). `increment <= 1` is a no-op. Sign is preserved so debit deltas
/// round symmetrically.
pub fn round_to_increment(mins: i32, increment: i32) -> i32 {
    if increment <= 1 {
        return mins;
    }
    let sign = if mins < 0 { -1 } else { 1 };
    let abs = mins.abs();
    let rounded = ((abs + increment / 2) / increment) * increment;
    sign * rounded
}

pub fn format_duration(total_mins: i32) -> String {
    let negative = total_mins < 0;
    let abs = total_mins.unsigned_abs() as i32;
    let h = abs / 60;
    let m = abs % 60;

    let body = match (h, m) {
        (0, 0) => "0 min".to_string(),
        (0, m) => format!("{} min", m),
        (h, 0) => format!("{} hr", h),
        (h, m) => format!("{} hr {} min", h, m),
    };

    if negative {
        format!("-{}", body)
    } else {
        body
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hr_and_min() {
        assert_eq!(parse_duration("1 hr 30 min").unwrap(), 90);
    }

    #[test]
    fn parse_hr_only() {
        assert_eq!(parse_duration("2 hr").unwrap(), 120);
    }

    #[test]
    fn parse_min_only() {
        assert_eq!(parse_duration("45 min").unwrap(), 45);
    }

    #[test]
    fn parse_plural_forms() {
        assert_eq!(parse_duration("1 hour 30 minutes").unwrap(), 90);
        assert_eq!(parse_duration("2 hours").unwrap(), 120);
        assert_eq!(parse_duration("45 mins").unwrap(), 45);
    }

    #[test]
    fn format_zero() {
        assert_eq!(format_duration(0), "0 min");
    }

    #[test]
    fn format_hr_and_min() {
        assert_eq!(format_duration(90), "1 hr 30 min");
    }

    #[test]
    fn format_hr_only() {
        assert_eq!(format_duration(120), "2 hr");
    }

    #[test]
    fn format_min_only() {
        assert_eq!(format_duration(45), "45 min");
    }

    #[test]
    fn format_negative() {
        assert_eq!(format_duration(-90), "-1 hr 30 min");
        assert_eq!(format_duration(-45), "-45 min");
    }

    #[test]
    fn parse_negative_hr_and_min() {
        assert_eq!(parse_duration("-1 hr 30 min").unwrap(), -90);
    }

    #[test]
    fn parse_negative_min_only() {
        assert_eq!(parse_duration("-45 min").unwrap(), -45);
    }

    #[test]
    fn roundtrip_negative() {
        assert_eq!(parse_duration(&format_duration(-90)).unwrap(), -90);
        assert_eq!(parse_duration(&format_duration(-45)).unwrap(), -45);
        assert_eq!(parse_duration(&format_duration(-120)).unwrap(), -120);
    }

    #[test]
    fn parse_bare_integer_as_hours() {
        assert_eq!(parse_duration("2").unwrap(), 120);
        assert_eq!(parse_duration("1").unwrap(), 60);
    }

    #[test]
    fn parse_missing_unit_errors() {
        assert!(parse_duration("1 hr 30").is_err());
        assert!(parse_duration("").is_err());
    }

    #[test]
    fn parse_compact_formats() {
        assert_eq!(parse_duration("1h30m").unwrap(), 90);
        assert_eq!(parse_duration("1h").unwrap(), 60);
        assert_eq!(parse_duration("30m").unwrap(), 30);
        assert_eq!(parse_duration("1.5h").unwrap(), 90);
        assert_eq!(parse_duration("-1h30m").unwrap(), -90);
    }

    #[test]
    fn parse_decimal_formats() {
        assert_eq!(parse_duration("1.5").unwrap(), 90);
        assert_eq!(parse_duration("0.5").unwrap(), 30);
        assert_eq!(parse_duration("1.5 hours").unwrap(), 90);
        assert_eq!(parse_duration("1.5 hr").unwrap(), 90);
    }

    #[test]
    fn parse_european_decimal() {
        assert_eq!(parse_duration("1,5").unwrap(), 90);
        assert_eq!(parse_duration("1,5 hours").unwrap(), 90);
    }

    #[test]
    fn round_increment_one_is_noop() {
        assert_eq!(round_to_increment(7, 1), 7);
        assert_eq!(round_to_increment(-7, 1), -7);
        assert_eq!(round_to_increment(7, 0), 7);
    }

    #[test]
    fn round_increment_nearest() {
        assert_eq!(round_to_increment(7, 15), 0);
        assert_eq!(round_to_increment(8, 15), 15);
        assert_eq!(round_to_increment(22, 15), 15);
        assert_eq!(round_to_increment(23, 15), 30);
        assert_eq!(round_to_increment(0, 15), 0);
        assert_eq!(round_to_increment(15, 15), 15);
    }

    #[test]
    fn round_increment_half_rounds_up() {
        assert_eq!(round_to_increment(5, 10), 10);
        assert_eq!(round_to_increment(4, 10), 0);
    }

    #[test]
    fn round_increment_negative_symmetric() {
        assert_eq!(round_to_increment(-7, 15), 0);
        assert_eq!(round_to_increment(-8, 15), -15);
        assert_eq!(round_to_increment(-23, 15), -30);
    }
}
