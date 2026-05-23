use anyhow::{bail, Result};

pub fn parse_duration(s: &str) -> Result<i32> {
    let s = s.trim().to_lowercase();
    let (negative, s) = if s.starts_with('-') {
        (true, s[1..].trim_start().to_string())
    } else {
        (false, s)
    };

    let tokens: Vec<&str> = s.split_whitespace().collect();

    let mut hours: i32 = 0;
    let mut mins: i32 = 0;
    let mut i = 0;

    while i < tokens.len() {
        let n: i32 = tokens[i]
            .parse()
            .map_err(|_| anyhow::anyhow!("expected number, got {:?}", tokens[i]))?;
        i += 1;

        if i >= tokens.len() {
            bail!("expected unit after {}", n);
        }

        match tokens[i] {
            "hr" | "hrs" | "hour" | "hours" => hours = n,
            "min" | "mins" | "minute" | "minutes" => mins = n,
            u => bail!("unknown time unit {:?}", u),
        }
        i += 1;
    }

    if hours == 0 && mins == 0 && !s.is_empty() && tokens.is_empty() {
        bail!("empty time string");
    }

    let total = hours * 60 + mins;
    Ok(if negative { -total } else { total })
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
}
