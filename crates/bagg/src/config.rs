use anyhow::Result;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize, Default)]
struct RawConfig {
    pub path: Option<PathBuf>,
    pub decimal_separator: Option<char>,
    pub decimal_places: Option<u32>,
    pub always_show_unspecified_price: Option<bool>,
}

pub struct ResolvedConfig {
    pub path: PathBuf,
    pub decimal_separator: char,
    pub decimal_places: u32,
    pub always_show_unspecified_price: bool,
}

pub fn resolve() -> Result<ResolvedConfig> {
    let raw: RawConfig = foolscap::load_config("bagg")?;

    let path = match raw.path {
        Some(p) => p,
        None => foolscap::default_data_path("bagg")?,
    };

    let decimal_separator = raw.decimal_separator.unwrap_or('.');
    anyhow::ensure!(
        !decimal_separator.is_ascii_alphanumeric()
            && !decimal_separator.is_whitespace()
            && !matches!(decimal_separator, '(' | ')' | '?'),
        "invalid decimal_separator {:?}: must not be alphanumeric, whitespace, or one of ( ) ?",
        decimal_separator
    );

    let decimal_places = raw.decimal_places.unwrap_or(2);
    anyhow::ensure!(
        decimal_places <= 9,
        "invalid decimal_places {}: must be <= 9, so 10^decimal_places fits in i32 cents",
        decimal_places
    );

    Ok(ResolvedConfig {
        path,
        decimal_separator,
        decimal_places,
        always_show_unspecified_price: raw.always_show_unspecified_price.unwrap_or(false),
    })
}
