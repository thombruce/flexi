use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize, Default)]
pub struct Config {
    pub path: Option<PathBuf>,
}

pub fn resolve_flexi_path() -> Result<PathBuf> {
    let config = load_config()?;
    if let Some(p) = config.path {
        return Ok(p);
    }

    let data_dir = xdg_data_dir()
        .context("cannot determine data directory")?;

    Ok(data_dir.join("flexi").join("flexi.txt"))
}

fn load_config() -> Result<Config> {
    let config_dir = match xdg_config_dir() {
        Some(d) => d,
        None => return Ok(Config::default()),
    };

    let config_path = config_dir.join("flexi").join("flexi.toml");
    if !config_path.exists() {
        return Ok(Config::default());
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
