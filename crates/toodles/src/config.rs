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
    let raw: RawConfig = flexi_common::load_config("toodles")?;

    let path = match raw.path {
        Some(p) => p,
        None => flexi_common::default_data_path("toodles")?,
    };

    Ok(ResolvedConfig { path })
}
