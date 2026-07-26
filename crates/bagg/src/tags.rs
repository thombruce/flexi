/// Extracts a `+project`, `@context`, or `key:value` tag from a single
/// whitespace token, if it's shaped like one. The key of a `key:value` pair
/// must start with a letter — this excludes a bare numeric token (e.g. a
/// clock time `10:00`) from being misread as a tag.
fn extract_tag(token: &str) -> Option<String> {
    if token.len() > 1 && (token.starts_with('+') || token.starts_with('@')) {
        return Some(token.to_lowercase());
    }
    if let Some((key, value)) = token.split_once(':') {
        if key.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) && !value.is_empty() {
            return Some(token.to_lowercase());
        }
    }
    None
}

/// True if `text` carries any of `queries` as a `+project`/`@context`/
/// `key:value` tag (case-insensitive, matches if ANY query is present). An
/// empty `queries` matches everything (no filter).
pub fn matches(text: &str, queries: &[String]) -> bool {
    if queries.is_empty() {
        return true;
    }
    let tags: Vec<String> = text.split_whitespace().filter_map(extract_tag).collect();
    queries.iter().any(|q| tags.contains(&q.to_lowercase()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_project_and_context_tags() {
        assert!(matches("Buy screws +kitchen-reno", &["+kitchen-reno".to_string()]));
        assert!(matches("Call dentist @clinic", &["@clinic".to_string()]));
    }

    #[test]
    fn matches_is_case_insensitive() {
        assert!(matches("Buy screws +Kitchen-Reno", &["+kitchen-reno".to_string()]));
    }

    #[test]
    fn matches_any_of_multiple_queries() {
        assert!(matches("Milk +groceries", &["+other".to_string(), "+groceries".to_string()]));
    }

    #[test]
    fn no_queries_matches_everything() {
        assert!(matches("anything at all", &[]));
    }

    #[test]
    fn key_value_tag_matches() {
        assert!(matches("Pay invoice due:2026-08-01", &["due:2026-08-01".to_string()]));
    }

    #[test]
    fn numeric_key_is_not_a_tag() {
        assert!(!matches("Meeting at 10:00", &["10:00".to_string()]));
    }

    #[test]
    fn no_match_when_absent() {
        assert!(!matches("Milk", &["+groceries".to_string()]));
    }
}
