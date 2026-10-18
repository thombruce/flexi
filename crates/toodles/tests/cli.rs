use assert_cmd::Command;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

fn toodles(args: &[&str], dir: &Path) -> assert_cmd::assert::Assert {
    Command::cargo_bin("toodles")
        .unwrap()
        .env("XDG_DATA_HOME", dir)
        .env("XDG_CONFIG_HOME", dir)
        .args(args)
        .assert()
}

fn write_log(dir: &Path, content: &str) {
    fs::create_dir_all(dir.join("toodles")).unwrap();
    fs::write(dir.join("toodles").join("toodles.txt"), content).unwrap();
}

fn read_log(dir: &Path) -> String {
    fs::read_to_string(dir.join("toodles").join("toodles.txt")).unwrap()
}

fn out(assert: assert_cmd::assert::Assert) -> String {
    String::from_utf8_lossy(&assert.success().get_output().stdout).to_string()
}

fn today() -> String {
    chrono::Local::now().date_naive().format("%Y-%m-%d").to_string()
}

#[test]
fn add_stamps_created_and_due() {
    let dir = tempdir().unwrap();
    toodles(&["add", "--priority", "A", "--due", "2030-01-31", "Renew", "passport"], dir.path()).success();
    assert_eq!(read_log(dir.path()).trim(), format!("(A) {} 2030-01-31 Renew passport", today()));
}

#[test]
fn add_rejects_bad_due_and_priority() {
    let dir = tempdir().unwrap();
    toodles(&["add", "--due", "tomorrow", "Thing"], dir.path()).failure();
    toodles(&["add", "--priority", "1", "Thing"], dir.path()).failure();
}

#[test]
fn add_refuses_date_leading_title_without_due() {
    let dir = tempdir().unwrap();
    toodles(&["add", "2024-12-25", "party"], dir.path()).failure();
    toodles(&["add", "--due", "2030-01-01", "2024-12-25", "party"], dir.path()).success();
    assert_eq!(read_log(dir.path()).trim(), format!("{} 2030-01-01 2024-12-25 party", today()));
}

#[test]
fn list_sorts_by_status_then_priority_then_due() {
    let dir = tempdir().unwrap();
    write_log(
        dir.path(),
        "x 2024-09-20 2024-09-01 Finished\n2024-09-01 Undated\n2024-09-01 2024-09-30 Later\n2024-09-01 2024-09-10 Sooner\n(B) 2024-09-01 Prioritised\n",
    );
    let listing = out(toodles(&["list"], dir.path()));
    let pos = |s: &str| listing.find(s).unwrap();
    assert!(pos("Prioritised") < pos("Sooner"));
    assert!(pos("Sooner") < pos("Later"));
    assert!(pos("Later") < pos("Undated"));
    assert!(pos("Undated") < pos("Finished"));
}

#[test]
fn done_toggles_and_stamps_closed_in_place() {
    let dir = tempdir().unwrap();
    write_log(dir.path(), "(A) 2024-09-01 2024-09-30 Taxes\n");
    toodles(&["done", "1"], dir.path()).success();
    assert_eq!(read_log(dir.path()).trim(), format!("x (A) {} 2024-09-01 2024-09-30 Taxes", today()));
    toodles(&["done", "1"], dir.path()).success();
    assert_eq!(read_log(dir.path()).trim(), "(A) 2024-09-01 2024-09-30 Taxes");
}

#[test]
fn rm_removes_by_list_index() {
    let dir = tempdir().unwrap();
    write_log(dir.path(), "Second\n(A) First\n");
    toodles(&["rm", "1"], dir.path()).success();
    assert_eq!(read_log(dir.path()).trim(), "Second");
    toodles(&["rm", "5"], dir.path()).failure();
}

#[test]
fn list_filters_by_status_and_tag_without_renumbering() {
    let dir = tempdir().unwrap();
    write_log(dir.path(), "Paint +house\nx Gutters +house\nCall mum\n");
    let open = out(toodles(&["list", "--open", "+house"], dir.path()));
    assert!(open.contains("1  Paint +house"), "{open}");
    assert!(!open.contains("Gutters") && !open.contains("Call mum"));
    let done = out(toodles(&["list", "--done"], dir.path()));
    assert!(done.contains("3  x Gutters +house"), "{done}");
}

#[test]
fn edit_runs_editor_through_the_shell() {
    let dir = tempdir().unwrap();
    write_log(dir.path(), "Old\n");
    Command::cargo_bin("toodles")
        .unwrap()
        .env("XDG_DATA_HOME", dir.path())
        .env("XDG_CONFIG_HOME", dir.path())
        .env("EDITOR", "echo Edited >")
        .arg("edit")
        .assert()
        .success();
    assert_eq!(read_log(dir.path()).trim(), "Edited");
}

#[test]
fn edit_fails_when_editor_is_missing() {
    let dir = tempdir().unwrap();
    Command::cargo_bin("toodles")
        .unwrap()
        .env("XDG_DATA_HOME", dir.path())
        .env("XDG_CONFIG_HOME", dir.path())
        .env("EDITOR", "nonexistent-editor-xyz")
        .arg("edit")
        .assert()
        .failure()
        .stderr(predicates::str::contains("could not run editor"));
}

#[test]
fn edit_warns_but_succeeds_on_editor_failure() {
    let dir = tempdir().unwrap();
    Command::cargo_bin("toodles")
        .unwrap()
        .env("XDG_DATA_HOME", dir.path())
        .env("XDG_CONFIG_HOME", dir.path())
        .env("EDITOR", "false")
        .arg("edit")
        .assert()
        .success()
        .stderr(predicates::str::contains("warning: editor"));
}

#[test]
fn edit_works_before_the_first_add() {
    let dir = tempdir().unwrap();
    Command::cargo_bin("toodles")
        .unwrap()
        .env("XDG_DATA_HOME", dir.path())
        .env("XDG_CONFIG_HOME", dir.path())
        .env("EDITOR", "echo Fresh >")
        .arg("edit")
        .assert()
        .success();
    assert_eq!(read_log(dir.path()).trim(), "Fresh");
}

#[test]
fn list_filters_by_hashtag() {
    let dir = tempdir().unwrap();
    write_log(dir.path(), "Fix gutter #urgent\nSee issue #42\n");
    let listing = out(toodles(&["list", "#urgent"], dir.path()));
    assert!(listing.contains("Fix gutter #urgent"), "{listing}");
    assert!(!listing.contains("issue"), "{listing}");
    assert!(out(toodles(&["list", "#42"], dir.path())).trim().is_empty());
}
