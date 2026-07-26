use anyhow::{Context, Result};
use std::path::Path;

/// One contact, parsed from a line `NAME [phone:VALUE]... [email:VALUE]...`.
///
/// No delimiter and no positional grammar beyond extraction: `phone:`/
/// `email:` tokens (key matched case-insensitively) are found anywhere in
/// the line and pulled into `phone`/`email`, in encounter order. Everything
/// else — plain words and any unreserved tag (`+project`, `@context`, other
/// `key:value` pairs) — stays exactly where it was, joined into `name`.
/// Unreserved tags get no special treatment: the parser can't tell one from
/// a plain word, so they're never repositioned, same as bagg/calchemy never
/// reposition their own `+`/`@` tags.
///
/// A `key:value` token only counts as a tag at all if its key starts with a
/// letter (mirrors bagg/calchemy's `tags.rs` shape rule) — guards against
/// stray colon-containing text being misread. Accepted tradeoff: a name
/// that itself contains a literal `phone:`/`email:`-shaped substring would
/// misparse — rare in practice, lowest-risk collision in this family so far
/// (contact names essentially never contain colons).
#[derive(Debug, Clone, PartialEq)]
pub struct Contact {
    pub name: String,
    pub phone: Vec<String>,
    pub email: Vec<String>,
    /// The exact source line, so `rm` can locate it without reformatting.
    pub raw: String,
}

impl Contact {
    pub fn parse(line: &str) -> Result<Contact> {
        let mut name_tokens = Vec::new();
        let mut phone = Vec::new();
        let mut email = Vec::new();

        for token in line.split_whitespace() {
            match reserved_tag(token) {
                Some(("phone", value)) => phone.push(value.to_string()),
                Some(("email", value)) => email.push(value.to_string()),
                _ => name_tokens.push(token),
            }
        }

        let name = name_tokens.join(" ");
        anyhow::ensure!(!name.is_empty(), "missing name in line: {:?}", line);

        Ok(Contact { name, phone, email, raw: line.to_string() })
    }

    /// Sort key: alphabetical by name — the only universal field, unlike
    /// the other crates' domain-specific keys (chronological, status/price).
    pub fn sort_key(&self) -> String {
        self.name.to_lowercase()
    }

    /// Renders the canonical storage line: name (as-is, tags and all) then
    /// every phone then every email, in encounter order — the only
    /// canonicalization applied is where the *extracted* fields land, never
    /// where anything inside `name` sits.
    pub fn to_line(&self) -> String {
        let mut parts = vec![self.name.clone()];
        for p in &self.phone {
            parts.push(format!("phone:{p}"));
        }
        for e in &self.email {
            parts.push(format!("email:{e}"));
        }
        parts.join(" ")
    }
}

/// Classifies a single whitespace token as a reserved `phone:`/`email:` tag
/// (key matched case-insensitively), returning the canonical lowercase key
/// and the raw value. Any other `key:value`-shaped token (or a plain word)
/// returns `None` and is left untouched by the caller.
fn reserved_tag(token: &str) -> Option<(&'static str, &str)> {
    let (key, value) = token.split_once(':')?;
    if !key.chars().next()?.is_ascii_alphabetic() || value.is_empty() {
        return None;
    }
    match key.to_lowercase().as_str() {
        "phone" => Some(("phone", value)),
        "email" => Some(("email", value)),
        _ => None,
    }
}

fn read_lines(path: &Path) -> Result<Vec<String>> {
    if !path.exists() {
        return Ok(vec![]);
    }
    let raw = std::fs::read_to_string(path).with_context(|| format!("reading {:?}", path))?;
    Ok(raw.lines().filter(|l| !l.trim().is_empty()).map(str::to_string).collect())
}

pub fn read_contacts(path: &Path) -> Result<Vec<Contact>> {
    read_lines(path)?.iter().map(|l| Contact::parse(l)).collect()
}

/// Appends an already-rendered line (see `Contact::to_line`).
pub fn append_contact(path: &Path, line: &str) -> Result<()> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_bare_name() {
        let c = Contact::parse("John Smith").unwrap();
        assert_eq!(c.name, "John Smith");
        assert!(c.phone.is_empty());
        assert!(c.email.is_empty());
    }

    #[test]
    fn parse_single_phone_and_email() {
        let c = Contact::parse("John Smith phone:555-123-4567 email:john@example.com").unwrap();
        assert_eq!(c.name, "John Smith");
        assert_eq!(c.phone, vec!["555-123-4567"]);
        assert_eq!(c.email, vec!["john@example.com"]);
    }

    #[test]
    fn parse_multiple_phones_and_emails_in_encounter_order() {
        let c = Contact::parse(
            "John Smith email:john@home.com phone:555-987-6543 +family phone:555-123-4567 email:john@work.com",
        )
        .unwrap();
        assert_eq!(c.name, "John Smith +family");
        assert_eq!(c.phone, vec!["555-987-6543", "555-123-4567"]);
        assert_eq!(c.email, vec!["john@home.com", "john@work.com"]);
    }

    #[test]
    fn unreserved_tag_stays_embedded_in_name_untouched() {
        let c = Contact::parse("Jane +family Doe @work").unwrap();
        assert_eq!(c.name, "Jane +family Doe @work");
    }

    #[test]
    fn reserved_key_is_case_insensitive() {
        let c = Contact::parse("John Smith PHONE:555-1234 Email:j@x.com").unwrap();
        assert_eq!(c.phone, vec!["555-1234"]);
        assert_eq!(c.email, vec!["j@x.com"]);
    }

    #[test]
    fn other_key_value_tokens_are_not_reserved() {
        let c = Contact::parse("Dr. Lee title:Doctor").unwrap();
        assert_eq!(c.name, "Dr. Lee title:Doctor");
        assert!(c.phone.is_empty());
    }

    #[test]
    fn parse_rejects_empty_name() {
        assert!(Contact::parse("phone:555-1234").is_err());
        assert!(Contact::parse("").is_err());
    }

    #[test]
    fn to_line_canonicalizes_phone_then_email_after_name() {
        let c = Contact::parse(
            "John Smith email:john@home.com phone:555-987-6543 +family phone:555-123-4567 email:john@work.com",
        )
        .unwrap();
        assert_eq!(
            c.to_line(),
            "John Smith +family phone:555-987-6543 phone:555-123-4567 email:john@home.com email:john@work.com"
        );
    }

    #[test]
    fn to_line_round_trips_bare_name() {
        let c = Contact::parse("John Smith").unwrap();
        assert_eq!(c.to_line(), "John Smith");
    }

    #[test]
    fn sort_key_is_case_insensitive_alphabetical() {
        let a = Contact::parse("alice").unwrap();
        let b = Contact::parse("Bob").unwrap();
        assert!(a.sort_key() < b.sort_key());
    }
}
