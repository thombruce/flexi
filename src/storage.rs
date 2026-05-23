use crate::time::{format_duration, parse_duration};
use anyhow::{Context, Result};
use std::path::Path;

pub fn read_minutes(path: &Path) -> Result<i32> {
    if !path.exists() {
        return Ok(0);
    }
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("reading {:?}", path))?;
    parse_duration(raw.trim()).with_context(|| format!("parsing time in {:?}", path))
}

pub fn write_minutes(path: &Path, mins: i32) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating directory {:?}", parent))?;
    }
    std::fs::write(path, format_duration(mins))
        .with_context(|| format!("writing {:?}", path))
}
