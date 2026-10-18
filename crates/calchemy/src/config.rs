use anyhow::Result;
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
    let raw: RawConfig = foolscap::load_config("calchemy")?;

    let path = match raw.path {
        Some(p) => p,
        None => foolscap::default_data_path("calchemy")?,
    };

    Ok(ResolvedConfig {
        path,
        week_start: raw.week_start.unwrap_or_default(),
    })
}
