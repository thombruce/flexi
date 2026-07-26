use anyhow::{Context, Result};
use std::path::Path;

/// One item, parsed from a line `[x ][(A) ][PRICE ]NAME[ xQTY]`.
///
/// - Leading `x` marks the item as got; absent means not-got.
/// - `(A)`-`(Z)` is an optional priority.
/// - `PRICE` is an optional decimal amount (e.g. `12.99`), stored in minor
///   units (cents). It must contain a `.` — that's what distinguishes it
///   from a bare integer, which can never be a price.
/// - `NAME` is required free text.
/// - A trailing `xN` token (e.g. `x3`) is the desired quantity; absent means 1.
///   `to_line` omits it when the quantity is 1, so the common case
///   (`Eggs`) round-trips as plain text.
#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub got: bool,
    pub priority: Option<char>,
    pub price: Option<i32>,
    pub qty: u32,
    pub name: String,
    /// The exact source line, so `rm`/`got` can locate it without reformatting.
    pub raw: String,
}

impl Item {
    pub fn parse(line: &str) -> Result<Item> {
        let mut tokens: Vec<&str> = line.split_whitespace().collect();
        anyhow::ensure!(!tokens.is_empty(), "empty line");

        let got = if tokens.first() == Some(&"x") {
            tokens.remove(0);
            true
        } else {
            false
        };

        let priority = tokens.first().and_then(|t| parse_priority(t));
        if priority.is_some() {
            tokens.remove(0);
        }

        let price = tokens.first().and_then(|t| parse_price(t));
        if price.is_some() {
            tokens.remove(0);
        }

        let qty = tokens.last().and_then(|t| parse_qty_tag(t));
        if qty.is_some() {
            tokens.pop();
        }

        anyhow::ensure!(!tokens.is_empty(), "missing name in line: {:?}", line);
        let name = tokens.join(" ");

        Ok(Item { got, priority, price, qty: qty.unwrap_or(1), name, raw: line.to_string() })
    }

    /// Sort key: not-got before got, then priority A..Z (none last), then
    /// price cheap-first (none last), stable on insertion order for ties.
    pub fn sort_key(&self) -> (u8, u8, i32) {
        let status = if self.got { 1 } else { 0 };
        let priority = self.priority.map(|p| p as u8).unwrap_or(u8::MAX);
        let price = self.price.unwrap_or(i32::MAX);
        (status, priority, price)
    }

    /// Renders the canonical storage line for this item.
    pub fn to_line(&self) -> String {
        let mut parts = Vec::new();
        if self.got {
            parts.push("x".to_string());
        }
        if let Some(p) = self.priority {
            parts.push(format!("({})", p));
        }
        if let Some(cents) = self.price {
            parts.push(format_price(cents));
        }
        parts.push(self.name.clone());
        if self.qty != 1 {
            parts.push(format!("x{}", self.qty));
        }
        parts.join(" ")
    }
}

fn parse_priority(s: &str) -> Option<char> {
    let bytes = s.as_bytes();
    if bytes.len() == 3 && bytes[0] == b'(' && bytes[2] == b')' && bytes[1].is_ascii_uppercase() {
        Some(bytes[1] as char)
    } else {
        None
    }
}

/// Parses a price token: `\d+\.\d{2}`, in minor units (cents). The
/// required `.` is what keeps a price token shape-distinct from a bare
/// integer (which is never a price — only ever the trailing qty tag).
pub fn parse_price(s: &str) -> Option<i32> {
    let (whole, frac) = s.split_once('.')?;
    if whole.is_empty() || !whole.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    if frac.len() != 2 || !frac.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let whole: i32 = whole.parse().ok()?;
    let frac: i32 = frac.parse().ok()?;
    Some(whole * 100 + frac)
}

pub fn format_price(cents: i32) -> String {
    format!("{}.{:02}", cents / 100, cents % 100)
}

fn parse_qty_tag(s: &str) -> Option<u32> {
    let rest = s.strip_prefix('x')?;
    if rest.is_empty() || !rest.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    rest.parse().ok()
}

fn read_lines(path: &Path) -> Result<Vec<String>> {
    if !path.exists() {
        return Ok(vec![]);
    }
    let raw = std::fs::read_to_string(path).with_context(|| format!("reading {:?}", path))?;
    Ok(raw.lines().filter(|l| !l.trim().is_empty()).map(str::to_string).collect())
}

pub fn read_items(path: &Path) -> Result<Vec<Item>> {
    read_lines(path)?.iter().map(|l| Item::parse(l)).collect()
}

pub fn append_item(path: &Path, item: &Item) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("creating directory {:?}", parent))?;
    }
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("opening {:?}", path))?;
    writeln!(file, "{}", item.to_line()).with_context(|| format!("writing {:?}", path))
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

/// Removes the first line exactly equal to `raw`.
pub fn remove_line(path: &Path, raw: &str) -> Result<()> {
    let mut lines = read_lines(path)?;
    if let Some(pos) = lines.iter().position(|l| l == raw) {
        lines.remove(pos);
    }
    write_lines(path, &lines)
}

/// Replaces the first line exactly equal to `old_raw` with `new_line`,
/// preserving file order.
pub fn replace_line(path: &Path, old_raw: &str, new_line: &str) -> Result<()> {
    let mut lines = read_lines(path)?;
    if let Some(pos) = lines.iter().position(|l| l == old_raw) {
        lines[pos] = new_line.to_string();
    }
    write_lines(path, &lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_bare_name() {
        let i = Item::parse("Eggs").unwrap();
        assert!(!i.got);
        assert_eq!(i.priority, None);
        assert_eq!(i.price, None);
        assert_eq!(i.qty, 1);
        assert_eq!(i.name, "Eggs");
    }

    #[test]
    fn parse_all_fields() {
        let i = Item::parse("x (A) 12.99 Widget thing x2").unwrap();
        assert!(i.got);
        assert_eq!(i.priority, Some('A'));
        assert_eq!(i.price, Some(1299));
        assert_eq!(i.qty, 2);
        assert_eq!(i.name, "Widget thing");
    }

    #[test]
    fn parse_priority_only() {
        let i = Item::parse("(B) Cheap thing, price unknown").unwrap();
        assert_eq!(i.priority, Some('B'));
        assert_eq!(i.price, None);
        assert_eq!(i.name, "Cheap thing, price unknown");
    }

    #[test]
    fn parse_price_no_qty() {
        let i = Item::parse("5.00 Coffee").unwrap();
        assert_eq!(i.price, Some(500));
        assert_eq!(i.qty, 1);
        assert_eq!(i.name, "Coffee");
    }

    #[test]
    fn parse_got_only() {
        let i = Item::parse("x Got already").unwrap();
        assert!(i.got);
        assert_eq!(i.name, "Got already");
    }

    #[test]
    fn parse_rejects_empty_name() {
        assert!(Item::parse("x (A) 12.99 x2").is_err());
        assert!(Item::parse("").is_err());
    }

    #[test]
    fn to_line_omits_qty_when_one() {
        let i = Item { got: false, priority: None, price: None, qty: 1, name: "Eggs".to_string(), raw: String::new() };
        assert_eq!(i.to_line(), "Eggs");
    }

    #[test]
    fn to_line_round_trips() {
        for line in [
            "Eggs",
            "x Got already",
            "(B) Cheap thing, price unknown",
            "5.00 Coffee",
            "x (A) 12.99 Widget thing x2",
        ] {
            assert_eq!(Item::parse(line).unwrap().to_line(), line);
        }
    }

    #[test]
    fn known_edge_cases_documented_not_panicking() {
        // A name whose first word looks like a price is claimed as PRICE,
        // not part of the name — accepted tradeoff, documented in the plan.
        let i = Item::parse("12.99 discount voucher").unwrap();
        assert_eq!(i.price, Some(1299));
        assert_eq!(i.name, "discount voucher");

        // A name whose last word looks like a qty tag is claimed as QTY.
        let i = Item::parse("Model x3").unwrap();
        assert_eq!(i.qty, 3);
        assert_eq!(i.name, "Model");
    }

    #[test]
    fn sort_key_orders_status_then_priority_then_price() {
        let not_got = Item::parse("Eggs").unwrap();
        let got = Item::parse("x Eggs").unwrap();
        assert!(not_got.sort_key() < got.sort_key());

        let high_pri = Item::parse("(A) Eggs").unwrap();
        let low_pri = Item::parse("(B) Eggs").unwrap();
        let no_pri = Item::parse("Eggs").unwrap();
        assert!(high_pri.sort_key() < low_pri.sort_key());
        assert!(low_pri.sort_key() < no_pri.sort_key());

        let cheap = Item::parse("1.00 Eggs").unwrap();
        let pricey = Item::parse("5.00 Eggs").unwrap();
        let unpriced = Item::parse("Eggs").unwrap();
        assert!(cheap.sort_key() < pricey.sort_key());
        assert!(pricey.sort_key() < unpriced.sort_key());
    }
}
