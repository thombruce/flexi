use anyhow::Result;
use chrono::NaiveDate;
pub use foolscap::dates::parse_date;
use foolscap::prefix::{Prefix, Status};
use std::path::Path;

use foolscap::read_lines;
pub use foolscap::{append_line, remove_line, replace_line};

/// One todo, parsed from a line `[STATUS ][(A) ][CLOSED ][CREATED ][DUE ]TITLE`.
///
/// `STATUS` is `!` focus, `x` done or `~` obsolete (`foolscap::prefix`); none
/// means open. todo.txt's two dates plus a third, positional due date:
///
/// - Open or focused: `[CREATED [DUE]]` — one leading date is created, two
///   are created + due.
/// - Closed (`x` or `~`): `[CLOSED [CREATED [DUE]]]` — todo.txt's
///   `x CLOSED CREATED`, with due appended as the third date.
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
    pub status: Status,
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

        let Prefix { status, priority } = Prefix::take(&mut tokens);

        let max_dates = if status.is_closed() { 3 } else { 2 };
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
        let closed = if status.is_closed() { dates.next() } else { None };
        let created = dates.next();
        let due = dates.next();

        anyhow::ensure!(!tokens.is_empty(), "missing title in line: {:?}", line);
        let title = tokens.join(" ");

        Ok(Todo { status, priority, closed, created, due, title, raw: line.to_string() })
    }

    /// Sort key: focused, open, done, then obsolete; then priority A..Z (none
    /// last), then due soonest-first (none last), stable on file order for ties.
    pub fn sort_key(&self) -> (Status, u8, NaiveDate) {
        let priority = self.priority.map(|p| p as u8).unwrap_or(u8::MAX);
        (self.status, priority, self.due.unwrap_or(NaiveDate::MAX))
    }

    /// Renders the canonical storage line. Dates are written in order and
    /// stop at the first absent one — a later date can't be written without
    /// the earlier ones, since position is what identifies it.
    pub fn to_line(&self) -> String {
        let mut parts = Prefix { status: self.status, priority: self.priority }.tokens();
        let dates = if self.status.is_closed() { vec![self.closed, self.created, self.due] } else { vec![self.created, self.due] };
        parts.extend(dates.into_iter().map_while(|d| d.map(|d| d.format("%Y-%m-%d").to_string())));
        parts.push(self.title.clone());
        parts.join(" ")
    }
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
        assert_eq!(t.status, Status::Open);
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
        assert_eq!(t.status, Status::Done);
        assert_eq!(t.priority, Some('B'));
        assert_eq!((t.closed, t.created, t.due), (d("2024-09-20"), d("2024-09-01"), d("2024-09-30")));
        assert_eq!(t.title, "All three");
    }

    #[test]
    fn focus_takes_open_dates_and_obsolete_takes_closed_dates() {
        let t = todo("! 2024-09-01 2024-09-30 2024-10-01 launch");
        assert_eq!(t.status, Status::Focus);
        assert_eq!((t.closed, t.created, t.due), (None, d("2024-09-01"), d("2024-09-30")));
        assert_eq!(t.title, "2024-10-01 launch");
        let t = todo("~ (C) 2024-09-20 2024-09-01 2024-09-30 Never");
        assert_eq!(t.status, Status::Obsolete);
        assert_eq!(t.priority, Some('C'));
        assert_eq!((t.closed, t.created, t.due), (d("2024-09-20"), d("2024-09-01"), d("2024-09-30")));
    }

    #[test]
    fn open_line_never_claims_a_third_date() {
        let t = todo("2024-09-01 2024-09-30 2024-10-01 is the launch");
        assert_eq!(t.title, "2024-10-01 is the launch");
    }

    #[test]
    fn bounded_unpadded_dates_parse_and_loose_ones_are_title_text() {
        let t = todo("2024-9-1 Thing");
        assert_eq!(t.created, d("2024-09-01"));
        assert_eq!(t.to_line(), "2024-09-01 Thing");
        let t = todo("1-2-3 go");
        assert_eq!((t.created, t.title.as_str()), (None, "1-2-3 go"));
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
            "! (A) 2024-09-01 2024-09-30 Focused",
            "~ 2024-09-20 2024-09-01 Won't do",
        ] {
            assert_eq!(todo(l).to_line(), l);
        }
    }

    #[test]
    fn sort_key_orders_status_then_priority_then_due() {
        assert!(todo("! Focused").sort_key() < todo("(A) Open").sort_key());
        assert!(todo("Open").sort_key() < todo("x Closed").sort_key());
        assert!(todo("x Closed").sort_key() < todo("~ Obsolete").sort_key());
        assert!(todo("(A) a").sort_key() < todo("(B) b").sort_key());
        assert!(todo("(B) b").sort_key() < todo("c").sort_key());
        let soon = todo("2024-09-01 2024-09-10 soon");
        let later = todo("2024-09-01 2024-09-30 later");
        let undated = todo("2024-09-01 undated");
        assert!(soon.sort_key() < later.sort_key());
        assert!(later.sort_key() < undated.sort_key());
    }
}
