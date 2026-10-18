use anyhow::Result;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize, Default)]
struct RawConfig {
    pub path: Option<PathBuf>,
}

pub struct ResolvedConfig {
    pub path: PathBuf,
}

pub fn resolve() -> Result<ResolvedConfig> {
    let raw: RawConfig = foolscap::load_config("holler")?;

    let path = match raw.path {
        Some(p) => p,
        None => foolscap::default_data_path("holler")?,
    };

    Ok(ResolvedConfig { path })
}
