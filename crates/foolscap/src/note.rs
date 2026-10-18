//! An opt-in free-text note at the end of a record: `<body> # <note>`.
//!
//! The note starts at the first bare `#` token (` # `), so it can't clash
//! with a `#hashtag` (see `tags`), which is `#` immediately followed by a
//! word. Tools that don't use notes simply never call this.

/// The separator between a record's body and its note.
pub const MARKER: &str = " # ";

/// Splits `text` at the first ` # ` into the body and the note, if any.
pub fn split(text: &str) -> (&str, Option<&str>) {
    match text.split_once(MARKER) {
        Some((body, note)) => (body, Some(note)),
        None => (text, None),
    }
}

/// Appends ` # <note>` to `text` when a note is given.
pub fn append(text: &mut String, note: Option<&str>) {
    if let Some(note) = note {
        text.push_str(MARKER);
        text.push_str(note);
    }
}

/// Validates a note from user input (e.g. a clap `value_parser`): it must be
/// non-blank and a single line, so the record stays one line.
pub fn parse(s: &str) -> Result<String, String> {
    if s.contains('\n') || s.contains('\r') {
        return Err("note must not contain newlines".to_string());
    }
    if s.trim().is_empty() {
        return Err("note must not be empty".to_string());
    }
    Ok(s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_at_first_marker() {
        assert_eq!(split("+30 min > 2 hr # stayed late"), ("+30 min > 2 hr", Some("stayed late")));
        assert_eq!(split("@in # a # b"), ("@in", Some("a # b")));
        assert_eq!(split("= 0 min"), ("= 0 min", None));
    }

    #[test]
    fn hashtag_is_not_a_note() {
        assert_eq!(split("Fix gutter #urgent"), ("Fix gutter #urgent", None));
    }

    #[test]
    fn append_round_trips_with_split() {
        let mut s = "@in".to_string();
        append(&mut s, None);
        assert_eq!(s, "@in");
        append(&mut s, Some("project x"));
        assert_eq!(s, "@in # project x");
        assert_eq!(split(&s), ("@in", Some("project x")));
    }

    #[test]
    fn parse_rejects_blank_and_multiline() {
        assert!(parse("ok").is_ok());
        assert!(parse("  ").is_err());
        assert!(parse("a\nb").is_err());
    }
}
