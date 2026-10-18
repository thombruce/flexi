//! The todo.txt-style prefix at the start of a record: an optional status
//! marker, then an optional `(A)`–`(Z)` priority.
//!
//! A status marker is a bare token (`!`, `x`, `~`), so `!Call` or `xylophone`
//! stays ordinary text. A record has at most one status. Each tool parses the
//! rest of the line itself, after `take` has removed the prefix.

/// A record's status. Variants are declared in sort order: focused, open,
/// done, obsolete.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Status {
    /// `!`: being worked on now. Still open.
    Focus,
    /// No marker.
    #[default]
    Open,
    /// `x`: done.
    Done,
    /// `~`: won't do. Kept for the record rather than removed.
    Obsolete,
}

impl Status {
    /// Done or obsolete. Closed records can carry a closed date, for tools
    /// that track dates.
    pub fn is_closed(self) -> bool {
        matches!(self, Status::Done | Status::Obsolete)
    }

    /// The status after toggling towards `to`: `to`, or back to open if the
    /// record already has it. A tool's `done`/`focus`/`wont`-style commands
    /// all toggle this way.
    pub fn toggled(self, to: Status) -> Status {
        if self == to {
            Status::Open
        } else {
            to
        }
    }

    fn marker(self) -> Option<&'static str> {
        match self {
            Status::Focus => Some("!"),
            Status::Open => None,
            Status::Done => Some("x"),
            Status::Obsolete => Some("~"),
        }
    }

    fn from_marker(token: &str) -> Option<Status> {
        match token {
            "!" => Some(Status::Focus),
            "x" => Some(Status::Done),
            "~" => Some(Status::Obsolete),
            _ => None,
        }
    }
}

/// The parsed prefix of a record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Prefix {
    pub status: Status,
    pub priority: Option<char>,
}

impl Prefix {
    /// Removes the leading status and priority tokens from `tokens` and
    /// returns them; the remaining tokens are left for the tool to parse.
    pub fn take(tokens: &mut Vec<&str>) -> Prefix {
        let status = tokens.first().and_then(|t| Status::from_marker(t));
        if status.is_some() {
            tokens.remove(0);
        }
        let priority = tokens.first().and_then(|t| parse_priority(t));
        if priority.is_some() {
            tokens.remove(0);
        }
        Prefix { status: status.unwrap_or_default(), priority }
    }

    /// The prefix's tokens, in line order, for a tool to put at the start of
    /// its rendered line. Empty for an open record with no priority.
    pub fn tokens(&self) -> Vec<String> {
        self.status
            .marker()
            .map(str::to_string)
            .into_iter()
            .chain(self.priority.map(|p| format!("({p})")))
            .collect()
    }
}

/// Parses a `(A)`–`(Z)` priority token.
pub fn parse_priority(s: &str) -> Option<char> {
    let bytes = s.as_bytes();
    if bytes.len() == 3 && bytes[0] == b'(' && bytes[2] == b')' && bytes[1].is_ascii_uppercase() {
        Some(bytes[1] as char)
    } else {
        None
    }
}

/// Validates a priority from user input (e.g. a clap `value_parser`): a
/// single letter A–Z.
pub fn parse_priority_arg(s: &str) -> Result<char, String> {
    let mut chars = s.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) if c.is_ascii_uppercase() => Ok(c),
        _ => Err(format!("invalid priority {:?}, expected a single letter A-Z", s)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn take(line: &str) -> (Prefix, String) {
        let mut tokens: Vec<&str> = line.split_whitespace().collect();
        let prefix = Prefix::take(&mut tokens);
        (prefix, tokens.join(" "))
    }

    #[test]
    fn takes_each_status_and_priority() {
        assert_eq!(take("Plain"), (Prefix { status: Status::Open, priority: None }, "Plain".into()));
        assert_eq!(take("! (A) Now"), (Prefix { status: Status::Focus, priority: Some('A') }, "Now".into()));
        assert_eq!(take("x Done"), (Prefix { status: Status::Done, priority: None }, "Done".into()));
        assert_eq!(take("~ (C) Never"), (Prefix { status: Status::Obsolete, priority: Some('C') }, "Never".into()));
    }

    #[test]
    fn markers_must_be_bare_tokens() {
        assert_eq!(take("!Call mum").0.status, Status::Open);
        assert_eq!(take("xylophone").0.status, Status::Open);
        assert_eq!(take("~approx").0.status, Status::Open);
    }

    #[test]
    fn only_one_status_is_taken() {
        let (prefix, rest) = take("x ! Odd");
        assert_eq!(prefix.status, Status::Done);
        assert_eq!(rest, "! Odd");
    }

    #[test]
    fn tokens_round_trip() {
        for line in ["Plain", "(B) Pri", "! Now", "x (A) Done", "~ Never"] {
            let (prefix, rest) = take(line);
            let mut parts = prefix.tokens();
            parts.push(rest);
            assert_eq!(parts.join(" "), line);
        }
    }

    #[test]
    fn status_sorts_focus_open_done_obsolete() {
        assert!(Status::Focus < Status::Open);
        assert!(Status::Open < Status::Done);
        assert!(Status::Done < Status::Obsolete);
        assert!(Status::Done.is_closed() && Status::Obsolete.is_closed());
        assert!(!Status::Focus.is_closed() && !Status::Open.is_closed());
    }

    #[test]
    fn toggled_sets_or_reopens() {
        assert_eq!(Status::Open.toggled(Status::Focus), Status::Focus);
        assert_eq!(Status::Focus.toggled(Status::Focus), Status::Open);
        assert_eq!(Status::Focus.toggled(Status::Done), Status::Done);
        assert_eq!(Status::Done.toggled(Status::Obsolete), Status::Obsolete);
        assert_eq!(Status::Obsolete.toggled(Status::Obsolete), Status::Open);
    }

    #[test]
    fn priority_arg_validation() {
        assert_eq!(parse_priority_arg("A"), Ok('A'));
        assert!(parse_priority_arg("a").is_err());
        assert!(parse_priority_arg("AB").is_err());
    }
}
