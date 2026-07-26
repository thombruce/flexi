use anyhow::{Context, Result};
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
    let raw = load_config()?;

    let path = if let Some(p) = raw.path {
        p
    } else {
        let data_dir = xdg_data_dir().context("cannot determine data directory")?;
        data_dir.join("bagg").join("bagg.txt")
    };

    let decimal_separator = raw.decimal_separator.unwrap_or('.');
    anyhow::ensure!(
        !decimal_separator.is_ascii_alphanumeric() && !matches!(decimal_separator, '(' | ')' | '?'),
        "invalid decimal_separator {:?}: must not be alphanumeric or one of ( ) ?",
        decimal_separator
    );

    Ok(ResolvedConfig {
        path,
        decimal_separator,
        decimal_places: raw.decimal_places.unwrap_or(2),
        always_show_unspecified_price: raw.always_show_unspecified_price.unwrap_or(false),
    })
}

fn load_config() -> Result<RawConfig> {
    let config_dir = match xdg_config_dir() {
        Some(d) => d,
        None => return Ok(RawConfig::default()),
    };

    let config_path = config_dir.join("bagg").join("bagg.toml");
    if !config_path.exists() {
        return Ok(RawConfig::default());
    }

    let raw = std::fs::read_to_string(&config_path)
        .with_context(|| format!("reading {:?}", config_path))?;

    toml::from_str(&raw).with_context(|| format!("parsing {:?}", config_path))
}

fn xdg_config_dir() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|h| h.join(".config")))
}

fn xdg_data_dir() -> Option<PathBuf> {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|h| h.join(".local/share")))
}
