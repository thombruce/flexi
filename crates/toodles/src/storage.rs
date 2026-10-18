use anyhow::Result;
use chrono::NaiveDate;
use std::path::Path;

use flexi_common::read_lines;
pub use flexi_common::{append_line, remove_line, replace_line};

/// One todo, parsed from a line `[x ][(A) ][CLOSED ][CREATED ][DUE ]TITLE`.
///
/// todo.txt's two dates plus a third, positional due date:
///
/// - Open: `[CREATED [DUE]]` — one leading date is created, two are created + due.
/// - Closed: `x [CLOSED [CREATED [DUE]]]` — todo.txt's `x CLOSED CREATED`,
///   with due appended as the third date.
///
/// The date count is what tells the dates apart, so a due date requires a
/// created date (`add` always stamps one). Priority is kept on completion
/// (`x (A) ...`), matching bagg rather than todo.txt's drop-on-complete.
///
/// Accepted tradeoff (same as calchemy): a title that opens with a
/// date-shaped word, on a line with fewer dates than its maximum, has that
/// word claimed as a date.
#[derive(Debug, Clone, PartialEq)]
pub struct Todo {
    pub done: bool,
    pub priority: Option<char>,
    pub closed: Option<NaiveDate>,
    pub created: Option<NaiveDate>,
    pub due: Option<NaiveDate>,
    pub title: String,
    /// The exact source line, so `rm`/`done` can locate it without reformatting.
    pub raw: String,
}

impl Todo {
    pub fn parse(line: &str) -> Result<Todo> {
        let mut tokens: Vec<&str> = line.split_whitespace().collect();
        anyhow::ensure!(!tokens.is_empty(), "empty line");

        let done = if tokens.first() == Some(&"x") {
            tokens.remove(0);
            true
        } else {
            false
        };

        let priority = tokens.first().and_then(|t| parse_priority(t));
        if priority.is_some() {
            tokens.remove(0);
        }

        let max_dates = if done { 3 } else { 2 };
        let mut dates = Vec::new();
        while dates.len() < max_dates {
            match tokens.first().and_then(|t| parse_date(t)) {
                Some(d) => {
                    dates.push(d);
                    tokens.remove(0);
                }
                None => break,
            }
        }
        let mut dates = dates.into_iter();
        let closed = if done { dates.next() } else { None };
        let created = dates.next();
        let due = dates.next();

        anyhow::ensure!(!tokens.is_empty(), "missing title in line: {:?}", line);
        let title = tokens.join(" ");

        Ok(Todo { done, priority, closed, created, due, title, raw: line.to_string() })
    }

    /// Sort key: open before closed, then priority A..Z (none last), then
    /// due soonest-first (none last), stable on file order for ties.
    pub fn sort_key(&self) -> (bool, u8, NaiveDate) {
        let priority = self.priority.map(|p| p as u8).unwrap_or(u8::MAX);
        (self.done, priority, self.due.unwrap_or(NaiveDate::MAX))
    }

    /// Renders the canonical storage line. Dates are written in order and
    /// stop at the first absent one — a later date can't be written without
    /// the earlier ones, since position is what identifies it.
    pub fn to_line(&self) -> String {
        let mut parts = Vec::new();
        if self.done {
            parts.push("x".to_string());
        }
        if let Some(p) = self.priority {
            parts.push(format!("({})", p));
        }
        let dates = if self.done { vec![self.closed, self.created, self.due] } else { vec![self.created, self.due] };
        parts.extend(dates.into_iter().map_while(|d| d.map(|d| d.format("%Y-%m-%d").to_string())));
        parts.push(self.title.clone());
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

/// Strict `YYYY-MM-DD`: chrono alone also accepts unpadded `2024-9-1`, which
/// would widen the set of title words misclaimed as dates.
pub fn parse_date(s: &str) -> Option<NaiveDate> {
    if s.len() != 10 {
        return None;
    }
    NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
}

pub fn read_todos(path: &Path) -> Result<Vec<Todo>> {
    read_lines(path)?.iter().map(|l| Todo::parse(l)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn todo(line: &str) -> Todo {
        Todo::parse(line).unwrap()
    }

    fn d(s: &str) -> Option<NaiveDate> {
        parse_date(s)
    }

    #[test]
    fn parse_bare_title() {
        let t = todo("Call mum");
        assert!(!t.done);
        assert_eq!((t.priority, t.closed, t.created, t.due), (None, None, None, None));
        assert_eq!(t.title, "Call mum");
    }

    #[test]
    fn parse_open_dates_by_count() {
        let t = todo("2024-09-01 Created only");
        assert_eq!((t.created, t.due), (d("2024-09-01"), None));
        let t = todo("(A) 2024-09-01 2024-09-30 Created and due");
        assert_eq!(t.priority, Some('A'));
        assert_eq!((t.created, t.due), (d("2024-09-01"), d("2024-09-30")));
        assert_eq!(t.title, "Created and due");
    }

    #[test]
    fn parse_closed_dates_by_count() {
        let t = todo("x 2024-09-20 Closed only");
        assert_eq!((t.closed, t.created, t.due), (d("2024-09-20"), None, None));
        let t = todo("x 2024-09-20 2024-09-01 Closed and created");
        assert_eq!((t.closed, t.created, t.due), (d("2024-09-20"), d("2024-09-01"), None));
        let t = todo("x (B) 2024-09-20 2024-09-01 2024-09-30 All three");
        assert!(t.done);
        assert_eq!(t.priority, Some('B'));
        assert_eq!((t.closed, t.created, t.due), (d("2024-09-20"), d("2024-09-01"), d("2024-09-30")));
        assert_eq!(t.title, "All three");
    }

    #[test]
    fn open_line_never_claims_a_third_date() {
        let t = todo("2024-09-01 2024-09-30 2024-10-01 is the launch");
        assert_eq!(t.title, "2024-10-01 is the launch");
    }

    #[test]
    fn unpadded_date_is_title_text() {
        let t = todo("2024-9-1 Thing");
        assert_eq!(t.created, None);
        assert_eq!(t.title, "2024-9-1 Thing");
    }

    #[test]
    fn parse_rejects_missing_title() {
        assert!(Todo::parse("").is_err());
        assert!(Todo::parse("x (A) 2024-09-20").is_err());
    }

    #[test]
    fn to_line_round_trips() {
        for l in [
            "Call mum",
            "(A) Call mum +family",
            "2024-09-01 Created only",
            "2024-09-01 2024-09-30 Created and due",
            "x Closed bare",
            "x 2024-09-20 2024-09-01 Closed and created",
            "x (B) 2024-09-20 2024-09-01 2024-09-30 All three",
        ] {
            assert_eq!(todo(l).to_line(), l);
        }
    }

    #[test]
    fn sort_key_orders_status_then_priority_then_due() {
        assert!(todo("Open").sort_key() < todo("x Closed").sort_key());
        assert!(todo("(A) a").sort_key() < todo("(B) b").sort_key());
        assert!(todo("(B) b").sort_key() < todo("c").sort_key());
        let soon = todo("2024-09-01 2024-09-10 soon");
        let later = todo("2024-09-01 2024-09-30 later");
        let undated = todo("2024-09-01 undated");
        assert!(soon.sort_key() < later.sort_key());
        assert!(later.sort_key() < undated.sort_key());
    }
}
