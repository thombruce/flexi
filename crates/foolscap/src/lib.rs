//! Plumbing shared by the flexi family of plaintext tools: XDG path
//! discovery, toml config loading, line-oriented file I/O with atomic
//! rewrites, and `$EDITOR` launching. Deliberately minimal — per-tool config
//! structs, validation, and storage semantics stay in each tool's crate.

use anyhow::{Context, Result};
use serde::de::DeserializeOwned;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

pub fn xdg_config_dir() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|h| h.join(".config")))
}

pub fn xdg_data_dir() -> Option<PathBuf> {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|h| h.join(".local/share")))
}

/// The default data file, `<data dir>/<tool>/<tool>.txt`.
pub fn default_data_path(tool: &str) -> Result<PathBuf> {
    let data_dir = xdg_data_dir().context("cannot determine data directory")?;
    Ok(data_dir.join(tool).join(format!("{tool}.txt")))
}

/// Loads `<config dir>/<tool>/<tool>.toml`, or `T::default()` if it's absent.
pub fn load_config<T: DeserializeOwned + Default>(tool: &str) -> Result<T> {
    let Some(config_dir) = xdg_config_dir() else {
        return Ok(T::default());
    };
    let config_path = config_dir.join(tool).join(format!("{tool}.toml"));
    if !config_path.exists() {
        return Ok(T::default());
    }
    let raw = std::fs::read_to_string(&config_path).with_context(|| format!("reading {:?}", config_path))?;
    toml::from_str(&raw).with_context(|| format!("parsing {:?}", config_path))
}

/// Every non-blank line of the file, in order; empty if the file doesn't exist.
pub fn read_lines(path: &Path) -> Result<Vec<String>> {
    if !path.exists() {
        return Ok(vec![]);
    }
    let raw = std::fs::read_to_string(path).with_context(|| format!("reading {:?}", path))?;
    Ok(raw.lines().filter(|l| !l.trim().is_empty()).map(str::to_string).collect())
}

fn create_parent(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("creating directory {:?}", parent))?;
    }
    Ok(())
}

/// Appends `line` plus a newline. If the file was hand-edited and lacks a
/// final newline, one is inserted first so the new line isn't glued onto
/// the previous one.
pub fn append_line(path: &Path, line: &str) -> Result<()> {
    create_parent(path)?;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .append(true)
        .open(path)
        .with_context(|| format!("opening {:?}", path))?;
    let mut prefix = "";
    if file.metadata().with_context(|| format!("reading {:?}", path))?.len() > 0 {
        let mut last = [0u8];
        file.seek(SeekFrom::End(-1))
            .and_then(|_| file.read_exact(&mut last))
            .with_context(|| format!("reading {:?}", path))?;
        if last[0] != b'\n' {
            prefix = "\n";
        }
    }
    writeln!(file, "{prefix}{line}").with_context(|| format!("writing {:?}", path))
}

/// Rewrites the file to exactly `lines` (atomic via a `.tmp` swap).
pub fn write_lines(path: &Path, lines: &[String]) -> Result<()> {
    create_parent(path)?;
    // Append rather than replace the extension, so `a.txt` and `a.todo`
    // don't share a temp file and `a.tmp` isn't its own temp file.
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    let mut content = lines.join("\n");
    if !content.is_empty() {
        content.push('\n');
    }
    std::fs::write(&tmp, content).with_context(|| format!("writing {:?}", tmp))?;
    std::fs::rename(&tmp, path).with_context(|| format!("renaming {:?} to {:?}", tmp, path))
}

/// Removes the first line exactly equal to `raw`.
pub fn remove_line(path: &Path, raw: &str) -> Result<()> {
    let mut lines = read_lines(path)?;
    if let Some(pos) = lines.iter().position(|l| l == raw) {
        lines.remove(pos);
    }
    write_lines(path, &lines)
}

/// Replaces the first line exactly equal to `old_raw` with `new_line`,
/// preserving file order.
pub fn replace_line(path: &Path, old_raw: &str, new_line: &str) -> Result<()> {
    let mut lines = read_lines(path)?;
    if let Some(pos) = lines.iter().position(|l| l == old_raw) {
        lines[pos] = new_line.to_string();
    }
    write_lines(path, &lines)
}

/// Opens `path` in `$EDITOR` (default `vi`), run through `sh` the way git
/// does, so it may carry arguments (`code -w`) or a quoted path with spaces.
/// A non-zero exit only warns: the file is already saved by then, and vim
/// exits non-zero for things like a vimrc error.
pub fn open_in_editor(path: &Path) -> Result<()> {
    let editor = std::env::var("EDITOR").ok().filter(|e| !e.trim().is_empty()).unwrap_or_else(|| "vi".to_string());
    let status = std::process::Command::new("sh")
        .arg("-c")
        .arg(format!("{editor} \"$@\""))
        .arg(&editor)
        .arg(path)
        .status()
        .with_context(|| format!("failed to launch editor {:?}", editor))?;
    if !status.success() {
        eprintln!("warning: editor {:?} exited with {}", editor, status);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_line_repairs_missing_final_newline() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub").join("f.txt");
        append_line(&path, "first").unwrap();
        std::fs::write(&path, "first\nhand-edited").unwrap();
        append_line(&path, "next").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "first\nhand-edited\nnext\n");
    }

    #[test]
    fn write_lines_temp_file_keeps_the_extension() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("f.tmp");
        write_lines(&path, &["a".into()]).unwrap();
        assert_eq!(read_lines(&path).unwrap(), ["a"]);
        assert!(!dir.path().join("f.tmp.tmp").exists());
    }

    #[test]
    fn line_rewrites_preserve_order() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("f.txt");
        write_lines(&path, &["a".into(), "b".into(), "c".into()]).unwrap();
        replace_line(&path, "b", "B").unwrap();
        remove_line(&path, "a").unwrap();
        assert_eq!(read_lines(&path).unwrap(), ["B", "c"]);
        assert!(!dir.path().join("f.txt.tmp").exists());
    }
}
