use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize, Clone, Copy, Default)]
#[serde(rename_all = "lowercase")]
pub enum TimestampFormat {
    #[default]
    Simple,
    Full,
}

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
    pub timestamp_format: Option<TimestampFormat>,
    pub week_start: Option<WeekStart>,
    pub increment: Option<u32>,
    pub max_session: Option<u32>,
}

pub struct ResolvedConfig {
    pub path: PathBuf,
    pub timestamp_format: TimestampFormat,
    pub week_start: WeekStart,
    pub increment: i32,
    pub max_session: Option<i32>,
}

pub fn resolve() -> Result<ResolvedConfig> {
    let raw = load_config()?;

    let path = if let Some(p) = raw.path {
        p
    } else {
        let data_dir = xdg_data_dir().context("cannot determine data directory")?;
        data_dir.join("clocking").join("clocking.txt")
    };

    let increment = raw.increment.unwrap_or(1);
    if increment < 1 {
        anyhow::bail!("increment must be at least 1 minute");
    }

    if let Some(0) = raw.max_session {
        anyhow::bail!("max_session must be at least 1 minute (omit it to disable)");
    }

    Ok(ResolvedConfig {
        path,
        timestamp_format: raw.timestamp_format.unwrap_or_default(),
        week_start: raw.week_start.unwrap_or_default(),
        increment: increment as i32,
        max_session: raw.max_session.map(|m| m as i32),
    })
}

fn load_config() -> Result<RawConfig> {
    let config_dir = match xdg_config_dir() {
        Some(d) => d,
        None => return Ok(RawConfig::default()),
    };

    let config_path = config_dir.join("clocking").join("clocking.toml");
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
