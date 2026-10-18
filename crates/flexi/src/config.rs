use anyhow::Result;
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
    let raw: RawConfig = flexi_common::load_config("flexi")?;

    let path = match raw.path {
        Some(p) => p,
        None => flexi_common::default_data_path("flexi")?,
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
