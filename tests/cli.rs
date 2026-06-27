use assert_cmd::Command;
use chrono::Local;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

// Helpers for log filter tests
fn write_log(dir: &Path, content: &str) {
    fs::create_dir_all(dir.join("flexi")).unwrap();
    fs::write(dir.join("flexi").join("flexi.txt"), content).unwrap();
}

fn log_lines(dir: &Path, args: &[&str]) -> Vec<String> {
    let mut full_args = vec!["log"];
    full_args.extend_from_slice(args);
    let out = flexi(&full_args, dir).success().get_output().stdout.clone();
    String::from_utf8_lossy(&out)
        .lines()
        .map(|l| l.to_string())
        .collect()
}

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

#[test]
fn reset_logs_set_zero() {
    let dir = tempdir().unwrap();
    flexi(&["add", "1", "hr"], dir.path()).success();
    flexi(&["reset"], dir.path()).success();
    let out = flexi(&["log"], dir.path()).success().get_output().stdout.clone();
    let text = String::from_utf8_lossy(&out);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[1].contains("= 0 min"));
}

#[test]
fn reset_with_note() {
    let dir = tempdir().unwrap();
    flexi(&["reset", "--note", "end of month"], dir.path()).success();
    let log = fs::read_to_string(dir.path().join("flexi").join("flexi.txt")).unwrap();
    assert!(log.contains("= 0 min"));
    assert!(log.contains("# end of month"));
    flexi(&[], dir.path()).success().stdout("0 min\n");
}

#[test]
fn full_timestamp_format_round_trips() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("flexi")).unwrap();
    fs::write(
        dir.path().join("flexi").join("flexi.toml"),
        "timestamp_format = \"full\"\n",
    ).unwrap();
    flexi(&["add", "2", "hr"], dir.path()).success();
    flexi(&["add", "30", "min"], dir.path()).success();
    flexi(&[], dir.path()).success().stdout("2 hr 30 min\n");
    flexi(&["undo"], dir.path()).success().stdout("-30 min → 2 hr\n");
    flexi(&[], dir.path()).success().stdout("2 hr\n");
}

#[test]
fn hand_edited_log_with_spaces_is_readable() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("flexi")).unwrap();
    fs::write(
        dir.path().join("flexi").join("flexi.txt"),
        "2026-05-24 10:20 = 3 hr\n",
    ).unwrap();
    flexi(&[], dir.path()).success().stdout("3 hr\n");
}

#[test]
fn hand_edited_log_with_multiple_spaces_is_readable() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("flexi")).unwrap();
    fs::write(
        dir.path().join("flexi").join("flexi.txt"),
        "2026-05-24 10:20   = 3 hr\n",
    ).unwrap();
    flexi(&[], dir.path()).success().stdout("3 hr\n");
}

#[test]
fn hand_edited_log_with_tab_is_readable() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("flexi")).unwrap();
    fs::write(
        dir.path().join("flexi").join("flexi.txt"),
        "2026-05-24 10:20\t= 3 hr\n",
    ).unwrap();
    flexi(&[], dir.path()).success().stdout("3 hr\n");
}

#[test]
fn undo_after_set() {
    let dir = tempdir().unwrap();
    flexi(&["add", "2", "hr"], dir.path()).success();
    flexi(&["set", "30", "min"], dir.path()).success();
    flexi(&["undo"], dir.path()).success().stdout("+1 hr 30 min → 2 hr\n");
    flexi(&[], dir.path()).success().stdout("2 hr\n");
}

#[test]
fn undo_after_reset() {
    let dir = tempdir().unwrap();
    flexi(&["add", "1", "hr"], dir.path()).success();
    flexi(&["reset"], dir.path()).success();
    flexi(&["undo"], dir.path()).success().stdout("+1 hr → 1 hr\n");
    flexi(&[], dir.path()).success().stdout("1 hr\n");
}

#[test]
fn full_timestamp_log_display_format() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("flexi")).unwrap();
    fs::write(
        dir.path().join("flexi").join("flexi.toml"),
        "timestamp_format = \"full\"\n",
    ).unwrap();
    flexi(&["add", "1", "hr"], dir.path()).success();
    let out = flexi(&["log"], dir.path()).success().get_output().stdout.clone();
    let text = String::from_utf8_lossy(&out);
    let line = text.lines().next().unwrap();
    // timestamp rendered as "YYYY-MM-DD HH:MM" regardless of stored format
    assert!(line.starts_with(&chrono::Local::now().format("%Y-%m-%d").to_string()));
    assert!(!line.contains('T'));
    assert!(line.contains("+1 hr → 1 hr"));
}

// --- log filter tests ---

#[test]
fn log_filter_today() {
    let dir = tempdir().unwrap();
    write_log(dir.path(), "2020-01-01 09:00 = 1 hr\n");
    flexi(&["add", "30", "min"], dir.path()).success();
    let lines = log_lines(dir.path(), &["--today"]);
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("+30 min"));
}

#[test]
fn log_filter_yesterday() {
    let dir = tempdir().unwrap();
    let yesterday = (Local::now() - chrono::Duration::days(1))
        .format("%Y-%m-%d")
        .to_string();
    write_log(dir.path(), &format!("{} 09:00 = 1 hr\n", yesterday));
    flexi(&["add", "30", "min"], dir.path()).success();
    let lines = log_lines(dir.path(), &["--yesterday"]);
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("= 1 hr"));
}

#[test]
fn log_filter_day_alias() {
    let dir = tempdir().unwrap();
    write_log(dir.path(), "2020-01-01 09:00 = 1 hr\n");
    flexi(&["add", "30", "min"], dir.path()).success();
    let lines = log_lines(dir.path(), &["--day"]);
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("+30 min"));
}

#[test]
fn log_filter_week() {
    let dir = tempdir().unwrap();
    write_log(dir.path(), "2020-01-01 09:00 = 1 hr\n");
    flexi(&["add", "30", "min"], dir.path()).success();
    let lines = log_lines(dir.path(), &["--week"]);
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("+30 min"));
}

#[test]
fn log_filter_month() {
    let dir = tempdir().unwrap();
    write_log(dir.path(), "2020-01-01 09:00 = 1 hr\n");
    flexi(&["add", "30", "min"], dir.path()).success();
    let lines = log_lines(dir.path(), &["--month"]);
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("+30 min"));
}

#[test]
fn log_filter_since() {
    let dir = tempdir().unwrap();
    write_log(dir.path(), "\
2026-05-01 09:00 = 1 hr\n\
2026-05-15 10:00 +30 min → 1 hr 30 min\n\
2026-06-01 09:00 +1 hr → 2 hr 30 min\n");
    let lines = log_lines(dir.path(), &["--since", "2026-05-10"]);
    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("2026-05-15"));
    assert!(lines[1].contains("2026-06-01"));
}

#[test]
fn log_filter_until() {
    let dir = tempdir().unwrap();
    write_log(dir.path(), "\
2026-05-01 09:00 = 1 hr\n\
2026-05-15 10:00 +30 min → 1 hr 30 min\n\
2026-06-01 09:00 +1 hr → 2 hr 30 min\n");
    let lines = log_lines(dir.path(), &["--until", "2026-05-20"]);
    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("2026-05-01"));
    assert!(lines[1].contains("2026-05-15"));
}

#[test]
fn log_filter_since_until_range() {
    let dir = tempdir().unwrap();
    write_log(dir.path(), "\
2026-05-01 09:00 = 1 hr\n\
2026-05-15 10:00 +30 min → 1 hr 30 min\n\
2026-06-01 09:00 +1 hr → 2 hr 30 min\n");
    let lines = log_lines(dir.path(), &["--since", "2026-05-10", "--until", "2026-05-20"]);
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("2026-05-15"));
}

#[test]
fn log_filter_last() {
    let dir = tempdir().unwrap();
    flexi(&["add", "1", "hr"], dir.path()).success();
    flexi(&["add", "30", "min"], dir.path()).success();
    flexi(&["add", "15", "min"], dir.path()).success();
    let lines = log_lines(dir.path(), &["--last", "2"]);
    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("+30 min"));
    assert!(lines[1].contains("+15 min"));
}

#[test]
fn log_filter_last_with_since() {
    let dir = tempdir().unwrap();
    write_log(dir.path(), "\
2026-05-01 09:00 = 1 hr\n\
2026-05-10 10:00 +30 min → 1 hr 30 min\n\
2026-05-20 11:00 +15 min → 1 hr 45 min\n\
2026-05-25 12:00 +1 hr → 2 hr 45 min\n");
    let lines = log_lines(dir.path(), &["--since", "2026-05-05", "--last", "2"]);
    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("2026-05-20"));
    assert!(lines[1].contains("2026-05-25"));
}

#[test]
fn log_summary() {
    let dir = tempdir().unwrap();
    write_log(dir.path(), "\
2026-05-01 09:00 +2 hr → 2 hr\n\
2026-05-02 09:00 +1 hr → 3 hr\n\
2026-05-03 09:00 -30 min → 2 hr 30 min\n");
    let out = flexi(&["log", "--summary"], dir.path()).success().get_output().stdout.clone();
    let text = String::from_utf8_lossy(&out);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 3);
    assert!(lines[0].contains("3 hr"));          // Added: 2 hr + 1 hr
    assert!(lines[1].contains("30 min"));         // Removed: -30 min
    assert!(lines[2].contains("+2 hr 30 min"));   // Net
}

#[test]
fn log_summary_with_filter() {
    let dir = tempdir().unwrap();
    write_log(dir.path(), "\
2026-05-01 09:00 +2 hr → 2 hr\n\
2026-05-10 09:00 +1 hr → 3 hr\n\
2026-05-10 10:00 -30 min → 2 hr 30 min\n");
    let out = flexi(&["log", "--since", "2026-05-09", "--summary"], dir.path())
        .success().get_output().stdout.clone();
    let text = String::from_utf8_lossy(&out);
    let lines: Vec<&str> = text.lines().collect();
    assert!(lines[0].contains("1 hr"));     // Added: only the May 10 add
    assert!(lines[1].contains("30 min"));   // Removed: -30 min
    assert!(lines[2].contains("+30 min"));  // Net: +30 min
}

#[test]
fn log_summary_with_until() {
    let dir = tempdir().unwrap();
    write_log(dir.path(), "\
2026-05-01 09:00 +1 hr → 1 hr\n\
2026-05-10 09:00 +30 min → 1 hr 30 min\n");
    let out = flexi(&["log", "--until", "2026-05-05", "--summary"], dir.path())
        .success().get_output().stdout.clone();
    let text = String::from_utf8_lossy(&out);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 3);
    assert!(lines[0].contains("1 hr"));    // Added: only May 1
    assert!(lines[2].contains("+1 hr"));   // Net
}

fn prose_out(dir: &Path, args: &[&str]) -> String {
    let mut full = vec!["log"];
    full.extend_from_slice(args);
    full.push("--prose");
    let out = flexi(&full, dir).success().get_output().stdout.clone();
    String::from_utf8_lossy(&out).trim_end().to_string()
}

#[test]
fn prose_both_sides() {
    let dir = tempdir().unwrap();
    let today = Local::now().format("%Y-%m-%d");
    write_log(dir.path(), &format!("\
{today} 09:00 +2 hr → 2 hr\n\
{today} 12:00 -30 min → 1 hr 30 min\n"));
    let line = prose_out(dir.path(), &["--today"]);
    assert_eq!(line, "Today: banked 1 hr 30 min (added 2 hr, removed 30 min). Balance now 1 hr 30 min.");
}

#[test]
fn prose_only_adds_omits_breakdown() {
    let dir = tempdir().unwrap();
    let today = Local::now().format("%Y-%m-%d");
    write_log(dir.path(), &format!("{today} 09:00 +2 hr → 2 hr\n"));
    let line = prose_out(dir.path(), &["--today"]);
    assert_eq!(line, "Today: banked 2 hr. Balance now 2 hr.");
}

#[test]
fn prose_net_negative_uses_used() {
    let dir = tempdir().unwrap();
    let today = Local::now().format("%Y-%m-%d");
    write_log(dir.path(), &format!("\
{today} 09:00 +30 min → 30 min\n\
{today} 12:00 -2 hr → -1 hr 30 min\n"));
    let line = prose_out(dir.path(), &["--today"]);
    assert_eq!(line, "Today: used 1 hr 30 min (added 30 min, removed 2 hr). Balance now -1 hr 30 min.");
}

#[test]
fn prose_net_zero_when_changes_cancel() {
    let dir = tempdir().unwrap();
    let today = Local::now().format("%Y-%m-%d");
    write_log(dir.path(), &format!("\
{today} 09:00 +1 hr → 1 hr\n\
{today} 12:00 -1 hr → 0 min\n"));
    let line = prose_out(dir.path(), &["--today"]);
    assert_eq!(line, "Today: net zero (added 1 hr, removed 1 hr). Balance now 0 min.");
}

#[test]
fn prose_no_change_for_empty_window() {
    let dir = tempdir().unwrap();
    let today = Local::now().format("%Y-%m-%d");
    write_log(dir.path(), &format!("{today} 09:00 +1 hr → 1 hr\n"));
    let line = prose_out(dir.path(), &["--yesterday"]);
    assert_eq!(line, "Yesterday: no change. Balance now 1 hr.");
}

#[test]
fn prose_label_since() {
    let dir = tempdir().unwrap();
    write_log(dir.path(), "2026-05-15 10:00 +30 min → 30 min\n");
    let line = prose_out(dir.path(), &["--since", "2026-05-01"]);
    assert_eq!(line, "Since 2026-05-01: banked 30 min. Balance now 30 min.");
}

#[test]
fn prose_label_until() {
    let dir = tempdir().unwrap();
    write_log(dir.path(), "2026-05-15 10:00 +30 min → 30 min\n");
    let line = prose_out(dir.path(), &["--until", "2026-05-31"]);
    assert_eq!(line, "Up to 2026-05-31: banked 30 min. Balance now 30 min.");
}

#[test]
fn prose_label_range() {
    let dir = tempdir().unwrap();
    write_log(dir.path(), "2026-05-15 10:00 +30 min → 30 min\n");
    let line = prose_out(dir.path(), &["--since", "2026-05-01", "--until", "2026-05-31"]);
    assert!(line.starts_with("Between 2026-05-01 and 2026-05-31: banked 30 min"));
}

#[test]
fn prose_label_overall() {
    let dir = tempdir().unwrap();
    write_log(dir.path(), "2026-05-15 10:00 +30 min → 30 min\n");
    let line = prose_out(dir.path(), &[]);
    assert_eq!(line, "Overall: banked 30 min. Balance now 30 min.");
}

#[test]
fn prose_label_yesterday() {
    let dir = tempdir().unwrap();
    let yesterday = (Local::now() - chrono::Duration::days(1))
        .format("%Y-%m-%d")
        .to_string();
    write_log(dir.path(), &format!("{yesterday} 09:00 +30 min → 30 min\n"));
    let line = prose_out(dir.path(), &["--yesterday"]);
    assert_eq!(line, "Yesterday: banked 30 min. Balance now 30 min.");
}

#[test]
fn prose_label_week() {
    let dir = tempdir().unwrap();
    let today = Local::now().format("%Y-%m-%d");
    write_log(dir.path(), &format!("{today} 09:00 +30 min → 30 min\n"));
    let line = prose_out(dir.path(), &["--week"]);
    assert_eq!(line, "This week: banked 30 min. Balance now 30 min.");
}

#[test]
fn prose_label_month() {
    let dir = tempdir().unwrap();
    let today = Local::now().format("%Y-%m-%d");
    write_log(dir.path(), &format!("{today} 09:00 +30 min → 30 min\n"));
    let line = prose_out(dir.path(), &["--month"]);
    assert_eq!(line, "This month: banked 30 min. Balance now 30 min.");
}

#[test]
fn prose_balance_reflects_set_but_excludes_from_totals() {
    let dir = tempdir().unwrap();
    write_log(dir.path(), "\
2026-05-01 09:00 +1 hr → 1 hr\n\
2026-05-02 09:00 = 5 hr\n");
    // `set` does not count as added/removed, but balance shows its value.
    let line = prose_out(dir.path(), &[]);
    assert_eq!(line, "Overall: banked 1 hr. Balance now 5 hr.");
}

#[test]
fn prose_conflicts_with_summary() {
    let dir = tempdir().unwrap();
    flexi(&["log", "--prose", "--summary"], dir.path()).failure();
}

#[test]
fn log_filter_week_start_sunday() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("flexi")).unwrap();
    fs::write(dir.path().join("flexi").join("flexi.toml"), "week_start = \"sunday\"\n").unwrap();
    write_log(dir.path(), "2020-01-01 09:00 = 1 hr\n");
    flexi(&["add", "30", "min"], dir.path()).success();
    let lines = log_lines(dir.path(), &["--week"]);
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("+30 min"));
}

// --- note tests ---

#[test]
fn note_stored_in_log() {
    let dir = tempdir().unwrap();
    flexi(&["add", "30", "min", "--note", "overtime"], dir.path()).success();
    let log = fs::read_to_string(dir.path().join("flexi").join("flexi.txt")).unwrap();
    assert!(log.contains("# overtime"));
}

#[test]
fn note_displayed_in_log() {
    let dir = tempdir().unwrap();
    flexi(&["add", "30", "min", "--note", "overtime"], dir.path()).success();
    let lines = log_lines(dir.path(), &[]);
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("# overtime"));
}

#[test]
fn note_does_not_break_balance() {
    let dir = tempdir().unwrap();
    flexi(&["add", "1", "hr", "--note", "test note"], dir.path()).success();
    flexi(&[], dir.path()).success().stdout("1 hr\n");
}

#[test]
fn note_on_set() {
    let dir = tempdir().unwrap();
    flexi(&["set", "1", "hr", "-m", "manual correction"], dir.path()).success();
    let log = fs::read_to_string(dir.path().join("flexi").join("flexi.txt")).unwrap();
    assert!(log.contains("# manual correction"));
    flexi(&[], dir.path()).success().stdout("1 hr\n");
}

#[test]
fn note_command_records_without_changing_balance() {
    let dir = tempdir().unwrap();
    flexi(&["add", "1", "hr", "30", "min"], dir.path()).success();
    flexi(&["note", "annual leave"], dir.path())
        .success()
        .stdout("+0 min → 1 hr 30 min\n");
    let log = fs::read_to_string(dir.path().join("flexi").join("flexi.txt")).unwrap();
    assert!(log.contains("+0 min > 1 hr 30 min # annual leave"));
    flexi(&[], dir.path()).success().stdout("1 hr 30 min\n");
}

#[test]
fn note_command_excluded_from_summary() {
    let dir = tempdir().unwrap();
    write_log(dir.path(), "\
2026-05-01 09:00 +2 hr → 2 hr\n\
2026-05-02 09:00 +0 min → 2 hr # annual leave\n");
    let out = flexi(&["log", "--summary"], dir.path()).success().get_output().stdout.clone();
    let text = String::from_utf8_lossy(&out);
    let lines: Vec<&str> = text.lines().collect();
    assert!(lines[0].contains("2 hr"));    // Added: only the +2 hr
    assert!(lines[1].contains("0 min"));   // Removed: nothing
    assert!(lines[2].contains("+2 hr"));   // Net: +2 hr, note ignored
}

#[test]
fn note_command_rejects_empty() {
    let dir = tempdir().unwrap();
    flexi(&["note", ""], dir.path()).failure();
    flexi(&["note", "   "], dir.path()).failure();
}

// --- summary / prose subcommand tests ---

#[test]
fn summary_command_matches_log_summary() {
    let dir = tempdir().unwrap();
    write_log(dir.path(), "\
2026-05-01 09:00 +2 hr → 2 hr\n\
2026-05-02 09:00 +1 hr → 3 hr\n\
2026-05-03 09:00 -30 min → 2 hr 30 min\n");
    let out = flexi(&["summary"], dir.path()).success().get_output().stdout.clone();
    let text = String::from_utf8_lossy(&out);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 3);
    assert!(lines[0].contains("3 hr"));         // Added
    assert!(lines[1].contains("30 min"));        // Removed
    assert!(lines[2].contains("+2 hr 30 min"));  // Net
}

#[test]
fn summary_command_accepts_filter() {
    let dir = tempdir().unwrap();
    write_log(dir.path(), "\
2026-05-01 09:00 +2 hr → 2 hr\n\
2026-05-10 09:00 +1 hr → 3 hr\n");
    let out = flexi(&["summary", "--since", "2026-05-05"], dir.path())
        .success().get_output().stdout.clone();
    let text = String::from_utf8_lossy(&out);
    let lines: Vec<&str> = text.lines().collect();
    assert!(lines[0].contains("1 hr"));   // Added: only May 10
    assert!(lines[2].contains("+1 hr"));  // Net
}

#[test]
fn prose_command_defaults_overall() {
    let dir = tempdir().unwrap();
    write_log(dir.path(), "2026-05-15 10:00 +30 min → 30 min\n");
    let out = flexi(&["prose"], dir.path()).success().get_output().stdout.clone();
    let line = String::from_utf8_lossy(&out).trim_end().to_string();
    assert_eq!(line, "Overall: banked 30 min. Balance now 30 min.");
}

#[test]
fn prose_command_accepts_filter() {
    let dir = tempdir().unwrap();
    let today = Local::now().format("%Y-%m-%d");
    write_log(dir.path(), &format!("{today} 09:00 +30 min → 30 min\n"));
    let out = flexi(&["prose", "--today"], dir.path()).success().get_output().stdout.clone();
    let line = String::from_utf8_lossy(&out).trim_end().to_string();
    assert_eq!(line, "Today: banked 30 min. Balance now 30 min.");
}

// --- JSON output tests ---

fn json_out(dir: &Path, args: &[&str]) -> serde_json::Value {
    let mut full = vec!["log"];
    full.extend_from_slice(args);
    full.push("--json");
    let out = flexi(&full, dir).success().get_output().stdout.clone();
    serde_json::from_slice(&out).expect("output is valid JSON")
}

#[test]
fn json_entries_shape() {
    let dir = tempdir().unwrap();
    write_log(dir.path(), "\
2026-05-01 09:00 +2 hr → 2 hr\n\
2026-05-02 09:00 -30 min → 1 hr 30 min # lunch\n\
2026-05-03 09:00 = 1 hr\n");
    let v = json_out(dir.path(), &[]);
    let arr = v.as_array().unwrap();
    assert_eq!(arr.len(), 3);

    assert_eq!(arr[0]["timestamp"], "2026-05-01 09:00");
    assert_eq!(arr[0]["delta_minutes"], 120);
    assert_eq!(arr[0]["balance_minutes"], 120);
    assert!(arr[0]["note"].is_null());

    assert_eq!(arr[1]["delta_minutes"], -30);
    assert_eq!(arr[1]["balance_minutes"], 90);
    assert_eq!(arr[1]["note"], "lunch");

    // `set` entries have no delta but carry a balance
    assert!(arr[2]["delta_minutes"].is_null());
    assert_eq!(arr[2]["balance_minutes"], 60);
}

#[test]
fn json_respects_filter() {
    let dir = tempdir().unwrap();
    write_log(dir.path(), "\
2026-05-01 09:00 +2 hr → 2 hr\n\
2026-05-10 09:00 +1 hr → 3 hr\n");
    let v = json_out(dir.path(), &["--since", "2026-05-05"]);
    let arr = v.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["timestamp"], "2026-05-10 09:00");
}

#[test]
fn json_summary_totals() {
    let dir = tempdir().unwrap();
    write_log(dir.path(), "\
2026-05-01 09:00 +2 hr → 2 hr\n\
2026-05-02 09:00 +1 hr → 3 hr\n\
2026-05-03 09:00 -30 min → 2 hr 30 min\n");
    let v = json_out(dir.path(), &["--summary"]);
    assert_eq!(v["added_minutes"], 180);
    assert_eq!(v["removed_minutes"], -30);
    assert_eq!(v["net_minutes"], 150);
}

#[test]
fn json_conflicts_with_prose() {
    let dir = tempdir().unwrap();
    flexi(&["log", "--json", "--prose"], dir.path()).failure();
}

// --- clock in / out tests ---

#[test]
fn in_creates_open_marker() {
    let dir = tempdir().unwrap();
    flexi(&["add", "2", "hr"], dir.path()).success();
    flexi(&["in"], dir.path()).success();
    let log = fs::read_to_string(dir.path().join("flexi").join("flexi.txt")).unwrap();
    assert!(log.lines().last().unwrap().contains("@in 2 hr"));
}

#[test]
fn bare_balance_shows_clocked_in() {
    let dir = tempdir().unwrap();
    flexi(&["add", "1", "hr"], dir.path()).success();
    flexi(&["in"], dir.path()).success();
    let out = flexi(&[], dir.path()).success().get_output().stdout.clone();
    let text = String::from_utf8_lossy(&out);
    assert!(text.contains("clocked in since"));
}

#[test]
fn clocked_in_blocks_mutations() {
    let dir = tempdir().unwrap();
    flexi(&["in"], dir.path()).success();
    flexi(&["add", "30", "min"], dir.path()).failure();
    flexi(&["remove", "30", "min"], dir.path()).failure();
    flexi(&["set", "1", "hr"], dir.path()).failure();
    flexi(&["reset"], dir.path()).failure();
    flexi(&["note", "x"], dir.path()).failure();
    flexi(&["in"], dir.path()).failure();
}

#[test]
fn out_without_in_fails() {
    let dir = tempdir().unwrap();
    flexi(&["out"], dir.path()).failure();
}

#[test]
fn out_banks_elapsed_time() {
    let dir = tempdir().unwrap();
    // Backdate the open marker by 90 minutes (floored to the minute, so the
    // elapsed computed by `out` is deterministic regardless of seconds).
    let start = (Local::now() - chrono::Duration::minutes(90))
        .format("%Y-%m-%d %H:%M")
        .to_string();
    write_log(dir.path(), &format!("\
2026-05-01 09:00 +2 hr > 2 hr\n\
{start} @in 2 hr # project x\n"));
    flexi(&["out"], dir.path()).success();

    let v = json_out(dir.path(), &[]);
    let arr = v.as_array().unwrap();
    let last = arr.last().unwrap();
    // 90 or 91 to tolerate a minute rollover between setup and `out`.
    let delta = last["delta_minutes"].as_i64().unwrap();
    assert!((90..=91).contains(&delta), "delta was {delta}");
    let balance = last["balance_minutes"].as_i64().unwrap();
    assert!((210..=211).contains(&balance), "balance was {balance}");
    // span + carried in-note recorded
    let note = last["note"].as_str().unwrap();
    assert!(note.contains('–') && note.contains("project x"), "note was {note}");
    // marker consumed, session closed
    assert_eq!(arr.len(), 2);
}

#[test]
fn out_rejects_future_clock_in() {
    let dir = tempdir().unwrap();
    let future = (Local::now() + chrono::Duration::hours(1))
        .format("%Y-%m-%d %H:%M")
        .to_string();
    write_log(dir.path(), &format!("{future} @in 0 min\n"));
    flexi(&["out"], dir.path()).failure();
}

#[test]
fn undo_aborts_clock_in() {
    let dir = tempdir().unwrap();
    flexi(&["add", "1", "hr"], dir.path()).success();
    flexi(&["in"], dir.path()).success();
    flexi(&["undo"], dir.path()).success();
    // session gone — mutations allowed again, balance intact
    flexi(&["add", "30", "min"], dir.path()).success();
    flexi(&[], dir.path()).success().stdout("1 hr 30 min\n");
}

/// Last entry's `note` field via the JSON output.
fn last_note(dir: &Path) -> String {
    let v = json_out(dir, &[]);
    v.as_array().unwrap().last().unwrap()["note"]
        .as_str()
        .unwrap()
        .to_string()
}

#[test]
fn out_merges_in_and_out_notes() {
    let dir = tempdir().unwrap();
    let start = (Local::now() - chrono::Duration::minutes(30))
        .format("%Y-%m-%d %H:%M")
        .to_string();
    write_log(dir.path(), &format!("{start} @in 0 min # project x\n"));
    flexi(&["out", "-m", "wrapped up"], dir.path()).success();
    let note = last_note(dir.path());
    assert!(note.contains("project x"), "note was {note}");
    assert!(note.contains("wrapped up"), "note was {note}");
}

#[test]
fn out_note_is_span_only_without_notes() {
    let dir = tempdir().unwrap();
    let start = (Local::now() - chrono::Duration::minutes(30))
        .format("%Y-%m-%d %H:%M")
        .to_string();
    write_log(dir.path(), &format!("{start} @in 0 min\n"));
    flexi(&["out"], dir.path()).success();
    let note = last_note(dir.path());
    assert!(note.contains('–'), "note was {note}");      // the worked span
    assert!(!note.contains(';'), "note was {note}");     // no extra notes appended
}

#[test]
fn out_same_minute_banks_zero() {
    let dir = tempdir().unwrap();
    let start = Local::now().format("%Y-%m-%d %H:%M").to_string();
    write_log(dir.path(), &format!("{start} @in 1 hr\n"));
    flexi(&["out"], dir.path()).success();
    let v = json_out(dir.path(), &[]);
    let delta = v.as_array().unwrap().last().unwrap()["delta_minutes"]
        .as_i64()
        .unwrap();
    assert!((0..=1).contains(&delta), "delta was {delta}");
}

#[test]
fn clock_out_full_timestamp_format() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("flexi")).unwrap();
    fs::write(
        dir.path().join("flexi").join("flexi.toml"),
        "timestamp_format = \"full\"\n",
    ).unwrap();
    // Backdated rfc3339 marker exercises the full-timestamp branch of parse_entry_time.
    let start = (Local::now() - chrono::Duration::minutes(90))
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, false);
    write_log(dir.path(), &format!("{start} @in 1 hr\n"));
    flexi(&["out"], dir.path()).success();
    let v = json_out(dir.path(), &[]);
    let delta = v.as_array().unwrap().last().unwrap()["delta_minutes"]
        .as_i64()
        .unwrap();
    assert!((90..=91).contains(&delta), "delta was {delta}");
}
