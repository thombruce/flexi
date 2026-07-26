use assert_cmd::Command;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

fn holler(args: &[&str], dir: &Path) -> assert_cmd::assert::Assert {
    Command::cargo_bin("holler")
        .unwrap()
        .env("XDG_DATA_HOME", dir)
        .env("XDG_CONFIG_HOME", dir)
        .args(args)
        .assert()
}

fn write_log(dir: &Path, content: &str) {
    fs::create_dir_all(dir.join("holler")).unwrap();
    fs::write(dir.join("holler").join("holler.txt"), content).unwrap();
}

fn read_log(dir: &Path) -> String {
    fs::read_to_string(dir.join("holler").join("holler.txt")).unwrap()
}

fn out(assert: assert_cmd::assert::Assert) -> String {
    String::from_utf8_lossy(&assert.success().get_output().stdout).to_string()
}

#[test]
fn add_writes_canonical_line() {
    let dir = tempdir().unwrap();
    holler(&["add", "John", "Smith", "phone:555-123-4567", "email:john@example.com"], dir.path()).success();
    let log = read_log(dir.path());
    assert!(log.contains("John Smith phone:555-123-4567 email:john@example.com"), "{log}");
}

#[test]
fn add_rejects_bare_tags_with_no_name() {
    let dir = tempdir().unwrap();
    holler(&["add", "phone:555-1234"], dir.path()).failure();
}

#[test]
fn list_sorts_alphabetically_by_name() {
    let dir = tempdir().unwrap();
    holler(&["add", "Zoe"], dir.path()).success();
    holler(&["add", "Alice"], dir.path()).success();
    holler(&["add", "Mallory"], dir.path()).success();
    let listing = out(holler(&["list"], dir.path()));
    let pos = |s: &str| listing.find(s).unwrap();
    assert!(pos("Alice") < pos("Mallory"));
    assert!(pos("Mallory") < pos("Zoe"));
}

#[test]
fn rm_removes_by_list_index() {
    let dir = tempdir().unwrap();
    holler(&["add", "Alice"], dir.path()).success();
    holler(&["add", "Bob"], dir.path()).success();
    holler(&["rm", "1"], dir.path()).success();
    let log = read_log(dir.path());
    assert!(!log.contains("Alice"));
    assert!(log.contains("Bob"));
}

#[test]
fn list_filters_by_tag() {
    let dir = tempdir().unwrap();
    holler(&["add", "Jane", "Doe", "+family"], dir.path()).success();
    holler(&["add", "John", "Smith"], dir.path()).success();
    let tagged = out(holler(&["list", "+family"], dir.path()));
    assert!(tagged.contains("Jane"), "{tagged}");
    assert!(!tagged.contains("John Smith"), "{tagged}");
}

#[test]
fn phone_lookup_single_match_prints_bare_values() {
    let dir = tempdir().unwrap();
    write_log(dir.path(), "John Smith phone:555-987-6543 phone:555-123-4567\nJane Doe phone:555-0000\n");
    let o = out(holler(&["phone", "John"], dir.path()));
    assert_eq!(o.trim(), "555-987-6543\n555-123-4567".trim());
}

#[test]
fn phone_lookup_no_value_on_file() {
    let dir = tempdir().unwrap();
    write_log(dir.path(), "John Smith\n");
    let o = out(holler(&["phone", "John"], dir.path()));
    assert!(o.contains("no phone on file for John Smith"), "{o}");
}

#[test]
fn email_lookup_multiple_matches_disambiguates() {
    let dir = tempdir().unwrap();
    write_log(dir.path(), "John Smith email:john.smith@x.com\nJohn Doe email:john.doe@x.com\n");
    let o = out(holler(&["email", "John"], dir.path()));
    assert!(o.contains("John Smith: john.smith@x.com"), "{o}");
    assert!(o.contains("John Doe: john.doe@x.com"), "{o}");
}

#[test]
fn lookup_no_match_errors() {
    let dir = tempdir().unwrap();
    write_log(dir.path(), "John Smith\n");
    holler(&["phone", "Nobody"], dir.path())
        .failure()
        .stderr(predicates::str::contains("no contact matching"));
}

#[test]
fn empty_states() {
    let dir = tempdir().unwrap();
    holler(&[], dir.path()).success().stdout("no contacts\n");
    holler(&["list"], dir.path()).success().stdout("no contacts\n");
}

#[test]
fn completions_and_man_smoke() {
    let dir = tempdir().unwrap();
    assert!(out(holler(&["completions", "bash"], dir.path())).contains("holler"));
    assert!(out(holler(&["man"], dir.path())).contains("holler"));
}
