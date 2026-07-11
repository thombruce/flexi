use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize, Clone, Copy, Default)]
#[serde(rename_all = "lowercase")]
pub enum WeekStart {
    #[default]
    Monday,
    Sunday,
}

#[derive(Deserialize, Default)]
struct RawConfig {
    pub path: Option<PathBuf>,
    pub week_start: Option<WeekStart>,
}

pub struct ResolvedConfig {
    pub path: PathBuf,
    pub week_start: WeekStart,
}

pub fn resolve() -> Result<ResolvedConfig> {
    let raw = load_config()?;

    let path = if let Some(p) = raw.path {
        p
    } else {
        let data_dir = xdg_data_dir().context("cannot determine data directory")?;
        data_dir.join("calchemy").join("calchemy.txt")
    };

    Ok(ResolvedConfig {
        path,
        week_start: raw.week_start.unwrap_or_default(),
    })
}

fn load_config() -> Result<RawConfig> {
    let config_dir = match xdg_config_dir() {
        Some(d) => d,
        None => return Ok(RawConfig::default()),
    };

    let config_path = config_dir.join("calchemy").join("calchemy.toml");
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
