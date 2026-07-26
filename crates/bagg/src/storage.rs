use anyhow::{Context, Result};
use std::path::Path;

/// The decimal shape a price is parsed/formatted with — configurable so a
/// user's own locale convention (separator character, decimal-place count)
/// is honored rather than hardcoded to `.`/2 places.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PriceFormat {
    pub separator: char,
    pub places: u32,
}

impl Default for PriceFormat {
    fn default() -> Self {
        PriceFormat { separator: '.', places: 2 }
    }
}

/// Explicit "price unspecified" token — occupies the PRICE slot positionally
/// without being a real value, so a NAME that itself opens with a
/// PRICE-shaped token can still be told apart from a real price, and so a
/// price can be deliberately marked unknown under any `PriceFormat`, not
/// just `places == 0`. Recognized on read and can be written on write
/// (auto-emitted by `Item::to_line` when needed, or always via
/// `always_show_unspecified_price`) regardless of `places` — the collision
/// it guards against isn't specific to bare integers (see `Item` docs).
pub const UNSPECIFIED_PRICE: &str = "?";

/// One item, parsed from a line `[x ][(A) ][PRICE ]NAME[ xQTY]`.
///
/// - Leading `x` marks the item as got; absent means not-got.
/// - `(A)`-`(Z)` is an optional priority.
/// - `PRICE` is an optional decimal amount (e.g. `12.99`), stored in minor
///   units (cents), or the literal `?` for "unspecified". A NAME whose
///   leading token happens to fully match the current `PriceFormat` shape
///   (any `places`, e.g. "1.99 and the Psychology of Pricing" under the
///   default `.`/2 places, not just a bare integer under `places == 0`)
///   collides with PRICE the same way it always has in this format —
///   accepted, documented, same tier as the other shape-based tradeoffs
///   below, not something `?` was ever meant to eliminate entirely. What
///   `?` *does* let you do: mark a price as deliberately unspecified even
///   when there'd be no collision otherwise, under any locale config.
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
    pub fn parse(line: &str, fmt: PriceFormat) -> Result<Item> {
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

        let price = if tokens.first() == Some(&UNSPECIFIED_PRICE) {
            tokens.remove(0);
            None
        } else if let Some(p) = tokens.first().and_then(|t| parse_price(t, fmt)) {
            tokens.remove(0);
            Some(p)
        } else {
            None
        };

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

    /// Renders the canonical storage line for this item. When there's no
    /// price, `?` is written if omitting it would let the name's leading
    /// word be misread as a price on re-read (or always, if
    /// `always_show_unspecified_price` is set) — the common case (`Eggs`)
    /// stays untouched. This applies for any `PriceFormat`, not just
    /// `places == 0` — a name can collide with a full decimal shape too.
    pub fn to_line(&self, fmt: PriceFormat, always_show_unspecified_price: bool) -> String {
        let mut parts = Vec::new();
        if self.got {
            parts.push("x".to_string());
        }
        if let Some(p) = self.priority {
            parts.push(format!("({})", p));
        }
        match self.price {
            Some(cents) => parts.push(format_price(cents, fmt)),
            None => {
                let name_would_collide =
                    self.name.split_whitespace().next().is_some_and(|first| parse_price(first, fmt).is_some());
                if always_show_unspecified_price || name_would_collide {
                    parts.push(UNSPECIFIED_PRICE.to_string());
                }
            }
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

/// Parses a price token in minor units (cents), per `fmt`. With
/// `places == 0` this is a bare integer; otherwise it's `\d+<sep>\d{places}`
/// — the required separator is what keeps a price token shape-distinct from
/// a bare integer (which is never a price — only ever the trailing qty tag,
/// or, when `places == 0`, potentially the name's own leading word).
pub fn parse_price(s: &str, fmt: PriceFormat) -> Option<i32> {
    if fmt.places == 0 {
        if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        return s.parse().ok();
    }
    let (whole, frac) = s.split_once(fmt.separator)?;
    if whole.is_empty() || !whole.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    if frac.len() != fmt.places as usize || !frac.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let whole: i32 = whole.parse().ok()?;
    let frac: i32 = frac.parse().ok()?;
    Some(whole * 10i32.pow(fmt.places) + frac)
}

pub fn format_price(cents: i32, fmt: PriceFormat) -> String {
    if fmt.places == 0 {
        return cents.to_string();
    }
    let scale = 10i32.pow(fmt.places);
    format!("{}{}{:0width$}", cents / scale, fmt.separator, cents % scale, width = fmt.places as usize)
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

pub fn read_items(path: &Path, fmt: PriceFormat) -> Result<Vec<Item>> {
    read_lines(path)?.iter().map(|l| Item::parse(l, fmt)).collect()
}

/// Appends an already-rendered line (see `Item::to_line`).
pub fn append_item(path: &Path, line: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("creating directory {:?}", parent))?;
    }
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("opening {:?}", path))?;
    writeln!(file, "{}", line).with_context(|| format!("writing {:?}", path))
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

    fn item(line: &str) -> Item {
        Item::parse(line, PriceFormat::default()).unwrap()
    }

    fn line(i: &Item) -> String {
        i.to_line(PriceFormat::default(), false)
    }

    #[test]
    fn parse_bare_name() {
        let i = item("Eggs");
        assert!(!i.got);
        assert_eq!(i.priority, None);
        assert_eq!(i.price, None);
        assert_eq!(i.qty, 1);
        assert_eq!(i.name, "Eggs");
    }

    #[test]
    fn parse_all_fields() {
        let i = item("x (A) 12.99 Widget thing x2");
        assert!(i.got);
        assert_eq!(i.priority, Some('A'));
        assert_eq!(i.price, Some(1299));
        assert_eq!(i.qty, 2);
        assert_eq!(i.name, "Widget thing");
    }

    #[test]
    fn parse_priority_only() {
        let i = item("(B) Cheap thing, price unknown");
        assert_eq!(i.priority, Some('B'));
        assert_eq!(i.price, None);
        assert_eq!(i.name, "Cheap thing, price unknown");
    }

    #[test]
    fn parse_price_no_qty() {
        let i = item("5.00 Coffee");
        assert_eq!(i.price, Some(500));
        assert_eq!(i.qty, 1);
        assert_eq!(i.name, "Coffee");
    }

    #[test]
    fn parse_got_only() {
        let i = item("x Got already");
        assert!(i.got);
        assert_eq!(i.name, "Got already");
    }

    #[test]
    fn parse_rejects_empty_name() {
        assert!(Item::parse("x (A) 12.99 x2", PriceFormat::default()).is_err());
        assert!(Item::parse("", PriceFormat::default()).is_err());
    }

    #[test]
    fn to_line_omits_qty_when_one() {
        let i = Item { got: false, priority: None, price: None, qty: 1, name: "Eggs".to_string(), raw: String::new() };
        assert_eq!(line(&i), "Eggs");
    }

    #[test]
    fn to_line_round_trips() {
        for l in [
            "Eggs",
            "x Got already",
            "(B) Cheap thing, price unknown",
            "5.00 Coffee",
            "x (A) 12.99 Widget thing x2",
        ] {
            assert_eq!(line(&item(l)), l);
        }
    }

    #[test]
    fn known_edge_cases_documented_not_panicking() {
        // A name whose first word looks like a price is claimed as PRICE,
        // not part of the name — accepted tradeoff, documented in the plan.
        // This holds for any PriceFormat, not just decimal_places == 0: a
        // name can collide with a full decimal shape under the default
        // config too (a book literally titled starting with a price).
        let i = item("12.99 discount voucher");
        assert_eq!(i.price, Some(1299));
        assert_eq!(i.name, "discount voucher");

        let i = item("1.99 and the Psychology of Pricing");
        assert_eq!(i.price, Some(199));
        assert_eq!(i.name, "and the Psychology of Pricing");

        // A name whose last word looks like a qty tag is claimed as QTY.
        let i = item("Model x3");
        assert_eq!(i.qty, 3);
        assert_eq!(i.name, "Model");
    }

    #[test]
    fn sort_key_orders_status_then_priority_then_price() {
        let not_got = item("Eggs");
        let got = item("x Eggs");
        assert!(not_got.sort_key() < got.sort_key());

        let high_pri = item("(A) Eggs");
        let low_pri = item("(B) Eggs");
        let no_pri = item("Eggs");
        assert!(high_pri.sort_key() < low_pri.sort_key());
        assert!(low_pri.sort_key() < no_pri.sort_key());

        let cheap = item("1.00 Eggs");
        let pricey = item("5.00 Eggs");
        let unpriced = item("Eggs");
        assert!(cheap.sort_key() < pricey.sort_key());
        assert!(pricey.sort_key() < unpriced.sort_key());
    }

    #[test]
    fn comma_decimal_separator() {
        let fmt = PriceFormat { separator: ',', places: 2 };
        let i = Item::parse("12,99 Widget", fmt).unwrap();
        assert_eq!(i.price, Some(1299));
        assert_eq!(i.name, "Widget");
        assert_eq!(i.to_line(fmt, false), "12,99 Widget");
        // The default `.` separator no longer matches under this fmt.
        assert_eq!(Item::parse("12.99 Widget", fmt).unwrap().price, None);
    }

    #[test]
    fn zero_decimal_places() {
        let fmt = PriceFormat { separator: '.', places: 0 };
        let i = Item::parse("1500 Yen item", fmt).unwrap();
        assert_eq!(i.price, Some(1500));
        assert_eq!(i.name, "Yen item");
        assert_eq!(i.to_line(fmt, false), "1500 Yen item");
    }

    #[test]
    fn three_decimal_places() {
        let fmt = PriceFormat { separator: '.', places: 3 };
        let i = Item::parse("12.500 Dinar item", fmt).unwrap();
        assert_eq!(i.price, Some(12500));
        assert_eq!(i.to_line(fmt, false), "12.500 Dinar item");
    }

    #[test]
    fn unspecified_price_placeholder_parses_as_no_price() {
        // Works under any PriceFormat, not just places == 0 -- a user with
        // a decimal currency can still deliberately mark a price unknown.
        let i = item("? Eggs");
        assert_eq!(i.price, None);
        assert_eq!(i.name, "Eggs");

        let fmt0 = PriceFormat { separator: '.', places: 0 };
        assert_eq!(Item::parse("? Eggs", fmt0).unwrap().price, None);
    }

    #[test]
    fn known_edge_case_name_starting_with_placeholder_char() {
        // Same structural tradeoff as the price/qty collisions above: `?`
        // is unconditionally the marker in leading position, so a name
        // that itself opens with a literal `?` loses that character on
        // parse. Accepted and documented, not engineered around -- the
        // format has no way to tell the two apart.
        let i = item("? Kidding, no idea what to get");
        assert_eq!(i.price, None);
        assert_eq!(i.name, "Kidding, no idea what to get");
    }

    #[test]
    fn price_present_disambiguates_digit_leading_name() {
        // With decimal_places = 0, a real price always claims exactly the
        // first token, so a digit-leading name is unaffected as long as a
        // price is actually given.
        let fmt = PriceFormat { separator: '.', places: 0 };
        let i = Item::parse("1500 4 slice toaster", fmt).unwrap();
        assert_eq!(i.price, Some(1500));
        assert_eq!(i.name, "4 slice toaster");
    }

    #[test]
    fn to_line_auto_emits_placeholder_when_name_would_collide() {
        // Under decimal_places = 0, a priceless item whose name starts with
        // a bare number would misparse as PRICE on the next read — to_line
        // must auto-insert `?` to stay safely re-parseable.
        let fmt = PriceFormat { separator: '.', places: 0 };
        let i = Item { got: false, priority: None, price: None, qty: 1, name: "4 slice toaster".to_string(), raw: String::new() };
        let rendered = i.to_line(fmt, false);
        assert_eq!(rendered, "? 4 slice toaster");
        // And it round-trips safely from here on.
        assert_eq!(Item::parse(&rendered, fmt).unwrap().name, "4 slice toaster");
    }

    #[test]
    fn to_line_omits_placeholder_when_no_collision() {
        let i = Item { got: false, priority: None, price: None, qty: 1, name: "Eggs".to_string(), raw: String::new() };
        assert_eq!(i.to_line(PriceFormat::default(), false), "Eggs");
        let fmt0 = PriceFormat { separator: '.', places: 0 };
        assert_eq!(i.to_line(fmt0, false), "Eggs");
    }

    #[test]
    fn always_show_unspecified_price_forces_placeholder() {
        // Works under the default (decimal) config too -- not just
        // places == 0 -- so a user can always opt into visibly marking
        // every priceless item as a matter of preference.
        let i = Item { got: false, priority: None, price: None, qty: 1, name: "Eggs".to_string(), raw: String::new() };
        assert_eq!(i.to_line(PriceFormat::default(), true), "? Eggs");

        let fmt0 = PriceFormat { separator: '.', places: 0 };
        assert_eq!(i.to_line(fmt0, true), "? Eggs");
    }
}
