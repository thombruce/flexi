use assert_cmd::Command;
use chrono::{Duration, Local};
use std::fs;
use std::path::Path;
use tempfile::tempdir;

fn clocking(args: &[&str], data_dir: &Path) -> assert_cmd::assert::Assert {
    Command::cargo_bin("clocking")
        .unwrap()
        .env("XDG_DATA_HOME", data_dir)
        .env("XDG_CONFIG_HOME", data_dir)
        .args(args)
        .assert()
}

fn write_log(dir: &Path, content: &str) {
    fs::create_dir_all(dir.join("clocking")).unwrap();
    fs::write(dir.join("clocking").join("clocking.txt"), content).unwrap();
}

fn read_log(dir: &Path) -> String {
    fs::read_to_string(dir.join("clocking").join("clocking.txt")).unwrap()
}

fn stdout(assert: assert_cmd::assert::Assert) -> String {
    String::from_utf8_lossy(&assert.success().get_output().stdout).to_string()
}

/// A `@in` marker timestamped `mins` minutes before now (simple format).
fn open_marker_ago(mins: i64) -> String {
    let start = Local::now() - Duration::minutes(mins);
    format!("{} @in\n", start.format("%Y-%m-%d %H:%M"))
}

#[test]
fn fresh_status_is_not_clocked_in() {
    let dir = tempdir().unwrap();
    clocking(&[], dir.path()).success().stdout("not clocked in\n");
}

#[test]
fn in_then_status_shows_clocked_in() {
    let dir = tempdir().unwrap();
    clocking(&["in"], dir.path()).success();
    let out = stdout(clocking(&[], dir.path()));
    assert!(out.contains("clocked in since"), "got: {out}");
    assert!(read_log(dir.path()).contains("@in"));
}

#[test]
fn in_twice_is_rejected() {
    let dir = tempdir().unwrap();
    clocking(&["in"], dir.path()).success();
    clocking(&["in"], dir.path())
        .failure()
        .stderr(predicates::str::contains("already clocked in"));
}

#[test]
fn out_without_session_is_rejected() {
    let dir = tempdir().unwrap();
    clocking(&["out"], dir.path())
        .failure()
        .stderr(predicates::str::contains("not clocked in"));
}

#[test]
fn out_records_elapsed_session() {
    let dir = tempdir().unwrap();
    write_log(dir.path(), &open_marker_ago(90));
    let out = stdout(clocking(&["out"], dir.path()));
    assert!(out.contains("worked 1 hr 30 min"), "got: {out}");
    let log = read_log(dir.path());
    assert!(log.contains("1 hr 30 min ("), "got: {log}");
    assert!(!log.contains("@in"), "marker should be gone: {log}");
}

#[test]
fn status_after_out_shows_today_total() {
    let dir = tempdir().unwrap();
    write_log(dir.path(), &open_marker_ago(90));
    clocking(&["out"], dir.path()).success();
    let out = stdout(clocking(&[], dir.path()));
    assert!(out.contains("not clocked in"), "got: {out}");
    assert!(out.contains("1 hr 30 min worked today"), "got: {out}");
}

#[test]
fn out_preserves_in_and_out_notes() {
    let dir = tempdir().unwrap();
    let start = (Local::now() - Duration::minutes(30)).format("%Y-%m-%d %H:%M");
    write_log(dir.path(), &format!("{start} @in # project x\n"));
    clocking(&["out", "-m", "wrapped up"], dir.path()).success();
    let log = read_log(dir.path());
    assert!(log.contains("# project x; wrapped up"), "got: {log}");
}

#[test]
fn out_over_max_session_needs_force() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("clocking")).unwrap();
    fs::write(dir.path().join("clocking").join("clocking.toml"), "max_session = 60\n").unwrap();
    write_log(dir.path(), &open_marker_ago(120));
    clocking(&["out"], dir.path())
        .failure()
        .stderr(predicates::str::contains("over max_session"));
    clocking(&["out", "--force"], dir.path()).success();
}

#[test]
fn summary_sums_worked_sessions() {
    let dir = tempdir().unwrap();
    let today = Local::now().format("%Y-%m-%d");
    write_log(
        dir.path(),
        &format!(
            "{today} 12:00 3 hr (09:00–12:00)\n{today} 17:30 4 hr 30 min (13:00–17:30)\n"
        ),
    );
    let out = stdout(clocking(&["summary", "--today"], dir.path()));
    assert!(out.contains("Worked: 7 hr 30 min (2 sessions)"), "got: {out}");
}

#[test]
fn log_today_filters_by_date() {
    let dir = tempdir().unwrap();
    let today = Local::now().format("%Y-%m-%d");
    write_log(
        dir.path(),
        &format!("2020-01-01 12:00 8 hr (09:00–17:00)\n{today} 17:00 6 hr (11:00–17:00)\n"),
    );
    let out = stdout(clocking(&["log", "--today"], dir.path()));
    assert!(out.contains("6 hr"), "got: {out}");
    assert!(!out.contains("8 hr"), "old entry should be filtered: {out}");
}

#[test]
fn undo_removes_last_entry() {
    let dir = tempdir().unwrap();
    let today = Local::now().format("%Y-%m-%d");
    write_log(dir.path(), &format!("{today} 12:00 3 hr (09:00–12:00)\n"));
    clocking(&["undo"], dir.path())
        .success()
        .stdout(predicates::str::contains("removed:"));
    assert!(read_log(dir.path()).trim().is_empty());
}
