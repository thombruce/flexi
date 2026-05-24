use assert_cmd::Command;
use std::path::Path;
use tempfile::tempdir;

fn flexi(args: &[&str], data_dir: &Path) -> assert_cmd::assert::Assert {
    Command::cargo_bin("flexi")
        .unwrap()
        .env("XDG_DATA_HOME", data_dir)
        .env("XDG_CONFIG_HOME", data_dir)
        .args(args)
        .assert()
}

#[test]
fn display_fresh() {
    let dir = tempdir().unwrap();
    flexi(&[], dir.path()).success().stdout("0 min\n");
}

#[test]
fn add_and_display() {
    let dir = tempdir().unwrap();
    flexi(&["add", "1", "hr", "30", "min"], dir.path()).success();
    flexi(&[], dir.path()).success().stdout("1 hr 30 min\n");
}

#[test]
fn add_compact_format() {
    let dir = tempdir().unwrap();
    flexi(&["add", "1h30m"], dir.path()).success();
    flexi(&[], dir.path()).success().stdout("1 hr 30 min\n");
}

#[test]
fn remove_subtracts() {
    let dir = tempdir().unwrap();
    flexi(&["add", "2", "hr"], dir.path()).success();
    flexi(&["remove", "30", "min"], dir.path()).success();
    flexi(&[], dir.path()).success().stdout("1 hr 30 min\n");
}

#[test]
fn rm_alias_subtracts() {
    let dir = tempdir().unwrap();
    flexi(&["add", "2", "hr"], dir.path()).success();
    flexi(&["rm", "30", "min"], dir.path()).success();
    flexi(&[], dir.path()).success().stdout("1 hr 30 min\n");
}

#[test]
fn negative_balance() {
    let dir = tempdir().unwrap();
    flexi(&["rm", "1", "hr"], dir.path()).success();
    flexi(&[], dir.path()).success().stdout("-1 hr\n");
}

#[test]
fn set_exact_value() {
    let dir = tempdir().unwrap();
    flexi(&["add", "3", "hr"], dir.path()).success();
    flexi(&["set", "2", "hr"], dir.path()).success();
    flexi(&[], dir.path()).success().stdout("2 hr\n");
}

#[test]
fn reset_to_zero() {
    let dir = tempdir().unwrap();
    flexi(&["add", "1", "hr", "30", "min"], dir.path()).success();
    flexi(&["reset"], dir.path()).success();
    flexi(&[], dir.path()).success().stdout("0 min\n");
}

#[test]
fn copy_prints_balance() {
    let dir = tempdir().unwrap();
    flexi(&["add", "1", "hr"], dir.path()).success();
    flexi(&["copy"], dir.path()).success().stdout("1 hr\n");
}

#[test]
fn cp_alias_prints_balance() {
    let dir = tempdir().unwrap();
    flexi(&["add", "1", "hr"], dir.path()).success();
    flexi(&["cp"], dir.path()).success().stdout("1 hr\n");
}

#[test]
fn add_prints_delta() {
    let dir = tempdir().unwrap();
    flexi(&["add", "1", "hr", "30", "min"], dir.path())
        .success()
        .stdout("+1 hr 30 min → 1 hr 30 min\n");
}

#[test]
fn remove_prints_delta() {
    let dir = tempdir().unwrap();
    flexi(&["add", "2", "hr"], dir.path()).success();
    flexi(&["remove", "30", "min"], dir.path())
        .success()
        .stdout("-30 min → 1 hr 30 min\n");
}

#[test]
fn log_records_mutations() {
    let dir = tempdir().unwrap();
    flexi(&["add", "1", "hr"], dir.path()).success();
    flexi(&["remove", "30", "min"], dir.path()).success();
    let out = flexi(&["log"], dir.path()).success().get_output().stdout.clone();
    let text = String::from_utf8_lossy(&out);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("+1 hr → 1 hr"));
    assert!(lines[1].contains("-30 min → 30 min"));
}

#[test]
fn undo_reverses_last_change() {
    let dir = tempdir().unwrap();
    flexi(&["add", "1", "hr"], dir.path()).success();
    flexi(&["add", "30", "min"], dir.path()).success();
    flexi(&["undo"], dir.path()).success().stdout("-30 min → 1 hr\n");
    flexi(&[], dir.path()).success().stdout("1 hr\n");
}

#[test]
fn undo_empty_prints_message() {
    let dir = tempdir().unwrap();
    flexi(&["undo"], dir.path()).success().stdout("nothing to undo\n");
}

#[test]
fn negative_roundtrip() {
    let dir = tempdir().unwrap();
    flexi(&["add", "1", "hr", "30", "min"], dir.path()).success();
    flexi(&["rm", "3", "hr"], dir.path()).success();
    flexi(&["rm", "1", "hr"], dir.path()).success();
    flexi(&[], dir.path()).success().stdout("-2 hr 30 min\n");
}
