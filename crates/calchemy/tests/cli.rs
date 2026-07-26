use assert_cmd::Command;
use chrono::{Datelike, Duration, Local, NaiveDate};
use std::fs;
use std::path::Path;
use tempfile::tempdir;

fn calchemy(args: &[&str], dir: &Path) -> assert_cmd::assert::Assert {
    Command::cargo_bin("calchemy")
        .unwrap()
        .env("XDG_DATA_HOME", dir)
        .env("XDG_CONFIG_HOME", dir)
        .args(args)
        .assert()
}

fn write_log(dir: &Path, content: &str) {
    fs::create_dir_all(dir.join("calchemy")).unwrap();
    fs::write(dir.join("calchemy").join("calchemy.txt"), content).unwrap();
}

fn read_log(dir: &Path) -> String {
    fs::read_to_string(dir.join("calchemy").join("calchemy.txt")).unwrap()
}

fn out(assert: assert_cmd::assert::Assert) -> String {
    String::from_utf8_lossy(&assert.success().get_output().stdout).to_string()
}

fn day(offset: i64) -> String {
    (Local::now().date_naive() + Duration::days(offset)).format("%Y-%m-%d").to_string()
}

#[test]
fn add_writes_canonical_lines() {
    let dir = tempdir().unwrap();
    calchemy(&["add", "2026-12-25", "Christmas"], dir.path()).success();
    calchemy(&["add", "2026-07-14", "09:00", "Team", "sync"], dir.path()).success();
    calchemy(&["add", "2026-07-14", "09:00-10:00", "Dentist", "@clinic"], dir.path()).success();
    calchemy(&["add", "2026-07-14", "20:00-02:00", "Party"], dir.path()).success();
    let log = read_log(dir.path());
    assert!(log.contains("2026-12-25 Christmas"), "{log}");
    assert!(log.contains("2026-07-14 09:00 Team sync"), "{log}");
    assert!(log.contains("2026-07-14 09:00 10:00 Dentist @clinic"), "{log}");
    assert!(log.contains("2026-07-14 20:00 2026-07-15 02:00 Party"), "cross-day end: {log}");
}

#[test]
fn multi_day_all_day_spans() {
    let dir = tempdir().unwrap();
    // Started yesterday, ends tomorrow: ongoing.
    calchemy(&["add", &day(-1), &day(1), "Wedding"], dir.path()).success();
    let log = read_log(dir.path());
    assert!(log.contains(&format!("{} {} Wedding", day(-1), day(1))), "{log}");
    // Shows in today's agenda, the default list, and week — despite starting in the past.
    for args in [vec![], vec!["list"], vec!["week"]] {
        let o = out(calchemy(&args, dir.path()));
        assert!(o.contains("Wedding"), "{args:?}: {o}");
    }
    // Not in --past while ongoing.
    let past = out(calchemy(&["list", "--past"], dir.path()));
    assert!(!past.contains("Wedding"), "{past}");
}

#[test]
fn add_timed_multi_day() {
    let dir = tempdir().unwrap();
    calchemy(&["add", "2026-07-17", "09:00", "2026-07-20", "17:00", "Conference"], dir.path()).success();
    // Same-day explicit end collapses to the canonical HH:MM form.
    calchemy(&["add", "2026-08-01", "09:00", "2026-08-01", "10:00", "Sameday"], dir.path()).success();
    // A date after the start time with no end time belongs to the title.
    calchemy(&["add", "2026-09-01", "09:00", "2026-09-02", "deadline"], dir.path()).success();
    let log = read_log(dir.path());
    assert!(log.contains("2026-07-17 09:00 2026-07-20 17:00 Conference"), "{log}");
    assert!(log.contains("2026-08-01 09:00 10:00 Sameday"), "{log}");
    assert!(log.contains("2026-09-01 09:00 2026-09-02 deadline"), "{log}");
}

#[test]
fn add_timed_multi_day_rejects_end_before_start() {
    let dir = tempdir().unwrap();
    calchemy(&["add", "2026-07-17", "09:00", "2026-07-16", "17:00", "Backwards"], dir.path())
        .failure()
        .stderr(predicates::str::contains("must be after"));
}

#[test]
fn add_rejects_end_date_not_after_start() {
    let dir = tempdir().unwrap();
    calchemy(&["add", "2026-07-17", "2026-07-16", "Backwards"], dir.path())
        .failure()
        .stderr(predicates::str::contains("must be after"));
}

#[test]
fn add_requires_title() {
    let dir = tempdir().unwrap();
    calchemy(&["add", "2026-07-14", "09:00"], dir.path())
        .failure()
        .stderr(predicates::str::contains("needs a title"));
}

#[test]
fn bare_shows_today_agenda() {
    let dir = tempdir().unwrap();
    write_log(
        dir.path(),
        &format!("{} 14:00 Team sync\n{} 09:00 Standup\n{} Future thing\n", day(0), day(0), day(3)),
    );
    let o = out(calchemy(&[], dir.path()));
    assert!(o.contains("(today)"), "{o}");
    // Sorted by time, and only today's events.
    let standup = o.find("Standup").unwrap();
    let sync = o.find("Team sync").unwrap();
    assert!(standup < sync, "09:00 should precede 14:00: {o}");
    assert!(!o.contains("Future thing"), "only today: {o}");
}

#[test]
fn list_default_hides_past() {
    let dir = tempdir().unwrap();
    write_log(dir.path(), &format!("{} Past\n{} Future\n", day(-5), day(2)));
    let o = out(calchemy(&["list"], dir.path()));
    assert!(o.contains("Future"), "{o}");
    assert!(!o.contains("Past"), "default list is upcoming only: {o}");
}

#[test]
fn list_all_and_past_views() {
    let dir = tempdir().unwrap();
    write_log(dir.path(), &format!("{} Past\n{} Future\n", day(-5), day(2)));
    let all = out(calchemy(&["list", "--all"], dir.path()));
    assert!(all.contains("Past") && all.contains("Future"), "{all}");
    let past = out(calchemy(&["list", "--past"], dir.path()));
    assert!(past.contains("Past") && !past.contains("Future"), "{past}");
}

#[test]
fn next_picks_soonest_upcoming() {
    let dir = tempdir().unwrap();
    // Both timed and future-dated, so independent of the current clock.
    write_log(dir.path(), &format!("{} 09:00 Later\n{} 09:00 Sooner\n", day(3), day(1)));
    let o = out(calchemy(&["next"], dir.path()));
    assert!(o.contains("Sooner"), "{o}");
    assert!(!o.contains("Later"), "{o}");
}

#[test]
fn week_filter_is_full_week() {
    let dir = tempdir().unwrap();
    // today is always in this week; today+8 never is.
    write_log(dir.path(), &format!("{} ThisWeek\n{} NextWeek\n", day(0), day(8)));
    let o = out(calchemy(&["week"], dir.path()));
    assert!(o.contains("ThisWeek"), "{o}");
    assert!(!o.contains("NextWeek"), "week is bounded: {o}");
}

#[test]
fn month_filter_is_full_month() {
    let dir = tempdir().unwrap();
    let today = Local::now().date_naive();
    let first = NaiveDate::from_ymd_opt(today.year(), today.month(), 1)
        .unwrap()
        .format("%Y-%m-%d")
        .to_string();
    write_log(dir.path(), &format!("{first} ThisMonth\n{} NextMonthish\n", day(40)));
    let o = out(calchemy(&["list", "--month"], dir.path()));
    assert!(o.contains("ThisMonth"), "{o}");
    assert!(!o.contains("NextMonthish"), "month is bounded: {o}");
}

#[test]
fn rm_by_index_removes_and_preserves_order() {
    let dir = tempdir().unwrap();
    write_log(
        dir.path(),
        &format!("{} 09:00 First\n{} 10:00 Second\n{} 11:00 Third\n", day(1), day(1), day(1)),
    );
    let o = out(calchemy(&["rm", "2"], dir.path()));
    assert!(o.contains("removed:") && o.contains("Second"), "{o}");
    let log = read_log(dir.path());
    assert!(log.contains("First") && log.contains("Third"), "{log}");
    assert!(!log.contains("Second"), "{log}");
    // Original file order preserved.
    assert!(log.find("First").unwrap() < log.find("Third").unwrap(), "{log}");
}

#[test]
fn rm_bad_index_errors() {
    let dir = tempdir().unwrap();
    write_log(dir.path(), &format!("{} 09:00 Only\n", day(1)));
    calchemy(&["rm", "5"], dir.path())
        .failure()
        .stderr(predicates::str::contains("no appointment 5"));
}

#[test]
fn empty_states() {
    let dir = tempdir().unwrap();
    calchemy(&[], dir.path()).success().stdout("no appointments today\n");
    calchemy(&["next"], dir.path()).success().stdout("no upcoming appointments\n");
    calchemy(&["list"], dir.path()).success().stdout("no appointments\n");
}

#[test]
fn list_filters_by_tag() {
    let dir = tempdir().unwrap();
    write_log(dir.path(), &format!("{} Dentist @clinic\n{} Standup\n", day(1), day(1)));
    let tagged = out(calchemy(&["list", "@clinic"], dir.path()));
    assert!(tagged.contains("Dentist"), "{tagged}");
    assert!(!tagged.contains("Standup"), "{tagged}");
    // Case-insensitive.
    let upper = out(calchemy(&["list", "@CLINIC"], dir.path()));
    assert!(upper.contains("Dentist"), "{upper}");
}

#[test]
fn tag_filter_composes_as_and_with_date_window() {
    let dir = tempdir().unwrap();
    write_log(
        dir.path(),
        &format!("{} Standup @clinic\n{} Sync\n{} Checkup @clinic\n", day(0), day(0), day(3)),
    );
    // --today AND @clinic: only the tagged appointment that's also today.
    let o = out(calchemy(&["list", "--today", "@clinic"], dir.path()));
    assert!(o.contains("Standup"), "{o}");
    assert!(!o.contains("Sync"), "today but untagged: {o}");
    assert!(!o.contains("Checkup"), "tagged but not today: {o}");
}

#[test]
fn completions_and_man_smoke() {
    let dir = tempdir().unwrap();
    assert!(out(calchemy(&["completions", "bash"], dir.path())).contains("calchemy"));
    assert!(out(calchemy(&["man"], dir.path())).contains("calchemy"));
}
