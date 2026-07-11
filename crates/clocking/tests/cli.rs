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

/// A `@in` marker `mins` minutes ago in the RFC 3339 "full" timestamp format.
fn open_marker_ago_full(mins: i64) -> String {
    let start = Local::now() - Duration::minutes(mins);
    format!("{} @in\n", start.to_rfc3339_opts(chrono::SecondsFormat::Secs, false))
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
    assert!(log.contains("= 1 hr 30 min"), "got: {log}");
    assert!(!log.contains("@in"), "marker should be gone: {log}");
}

/// The description of the single log line, i.e. everything after the leading
/// fixed-width (16-char simple) timestamp.
fn only_line_desc(dir: &Path) -> String {
    let log = read_log(dir);
    let line = log.lines().next().expect("one line").to_string();
    line[16..].trim_start().to_string()
}

#[test]
fn same_day_session_end_is_bare_time() {
    let dir = tempdir().unwrap();
    write_log(dir.path(), &open_marker_ago(90));
    clocking(&["out"], dir.path()).success();
    let desc = only_line_desc(dir.path());
    let end = desc.split(" = ").next().unwrap();
    assert_eq!(end.len(), 5, "same-day end should be bare HH:MM: {desc}");
    assert!(!end.contains('-'), "same-day end should carry no date: {desc}");
}

#[test]
fn cross_midnight_session_end_carries_date() {
    let dir = tempdir().unwrap();
    let yesterday = (Local::now() - Duration::days(1)).format("%Y-%m-%d").to_string();
    let today = Local::now().format("%Y-%m-%d").to_string();
    write_log(dir.path(), &format!("{yesterday} 23:00 @in\n"));
    clocking(&["out"], dir.path()).success();
    let log = read_log(dir.path());
    let line = log.lines().next().unwrap();
    assert!(line.starts_with(&format!("{yesterday} 23:00")), "keyed by clock-in: {line}");
    let desc = only_line_desc(dir.path());
    let end = desc.split(" = ").next().unwrap();
    assert!(end.contains(&today), "cross-midnight end must carry the end date: {desc}");
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
            "{today} 09:00 12:00 = 3 hr\n{today} 13:00 17:30 = 4 hr 30 min\n"
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
        &format!("2020-01-01 09:00 17:00 = 8 hr\n{today} 11:00 17:00 = 6 hr\n"),
    );
    let out = stdout(clocking(&["log", "--today"], dir.path()));
    assert!(out.contains("6 hr"), "got: {out}");
    assert!(!out.contains("8 hr"), "old entry should be filtered: {out}");
}

#[test]
fn full_timestamp_format_records_and_reads() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("clocking")).unwrap();
    fs::write(dir.path().join("clocking").join("clocking.toml"), "timestamp_format = \"full\"\n").unwrap();
    // Seed an open marker with a full (25-char, T-separated) timestamp.
    write_log(dir.path(), &open_marker_ago_full(90));
    // Status must parse the full timestamp back.
    let status = stdout(clocking(&[], dir.path()));
    assert!(status.contains("clocked in since"), "got: {status}");
    // Clocking out records the elapsed session and writes a full timestamp.
    let out = stdout(clocking(&["out"], dir.path()));
    assert!(out.contains("worked 1 hr 30 min"), "got: {out}");
    let log = read_log(dir.path());
    assert_eq!(log.as_bytes()[10], b'T', "closing line should use full format: {log}");
}

#[test]
fn summary_since_until_window() {
    let dir = tempdir().unwrap();
    write_log(
        dir.path(),
        "2026-07-05 10:00 12:00 = 2 hr\n\
         2026-07-08 12:00 17:00 = 5 hr\n\
         2026-07-20 03:00 12:00 = 9 hr\n",
    );
    let out = stdout(clocking(&["summary", "--since", "2026-07-06", "--until", "2026-07-09"], dir.path()));
    assert!(out.contains("Worked: 5 hr (1 session)"), "got: {out}");
}

#[test]
fn log_last_limits_entries() {
    let dir = tempdir().unwrap();
    write_log(
        dir.path(),
        "2026-07-05 10:00 12:00 = 2 hr\n\
         2026-07-06 09:00 12:00 = 3 hr\n\
         2026-07-07 08:00 12:00 = 4 hr\n",
    );
    let out = stdout(clocking(&["log", "--last", "2"], dir.path()));
    assert_eq!(out.lines().count(), 2, "got: {out}");
    assert!(out.contains("3 hr") && out.contains("4 hr"), "got: {out}");
    assert!(!out.contains("2 hr"), "oldest should be dropped: {out}");
}

#[test]
fn completions_and_man_smoke() {
    let dir = tempdir().unwrap();
    let comp = stdout(clocking(&["completions", "bash"], dir.path()));
    assert!(comp.contains("clocking"), "completion script should mention the binary");
    let man = stdout(clocking(&["man"], dir.path()));
    assert!(man.contains("clocking"), "man page should mention the binary");
}

#[test]
fn undo_removes_last_entry() {
    let dir = tempdir().unwrap();
    let today = Local::now().format("%Y-%m-%d");
    write_log(dir.path(), &format!("{today} 09:00 12:00 = 3 hr\n"));
    clocking(&["undo"], dir.path())
        .success()
        .stdout(predicates::str::contains("removed:"));
    assert!(read_log(dir.path()).trim().is_empty());
}
