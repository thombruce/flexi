use assert_cmd::Command;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

fn bagg(args: &[&str], dir: &Path) -> assert_cmd::assert::Assert {
    Command::cargo_bin("bagg")
        .unwrap()
        .env("XDG_DATA_HOME", dir)
        .env("XDG_CONFIG_HOME", dir)
        .args(args)
        .assert()
}

fn write_log(dir: &Path, content: &str) {
    fs::create_dir_all(dir.join("bagg")).unwrap();
    fs::write(dir.join("bagg").join("bagg.txt"), content).unwrap();
}

fn read_log(dir: &Path) -> String {
    fs::read_to_string(dir.join("bagg").join("bagg.txt")).unwrap()
}

fn out(assert: assert_cmd::assert::Assert) -> String {
    String::from_utf8_lossy(&assert.success().get_output().stdout).to_string()
}

#[test]
fn add_writes_canonical_lines() {
    let dir = tempdir().unwrap();
    bagg(&["add", "Eggs"], dir.path()).success();
    bagg(&["add", "--price", "12.99", "--qty", "2", "--priority", "A", "Widget", "thing"], dir.path()).success();
    let log = read_log(dir.path());
    assert!(log.contains("Eggs"), "{log}");
    assert!(log.contains("(A) 12.99 Widget thing x2"), "{log}");
}

#[test]
fn add_rejects_bad_price_and_priority() {
    let dir = tempdir().unwrap();
    bagg(&["add", "--price", "twelve", "Eggs"], dir.path()).failure();
    bagg(&["add", "--priority", "1", "Eggs"], dir.path()).failure();
}

#[test]
fn list_sorts_by_status_then_priority_then_price() {
    let dir = tempdir().unwrap();
    bagg(&["add", "--price", "5.00", "Cheap"], dir.path()).success();
    bagg(&["add", "--priority", "A", "Priority item"], dir.path()).success();
    bagg(&["add", "Plain"], dir.path()).success();
    let listing = out(bagg(&["list"], dir.path()));
    let pos = |s: &str| listing.find(s).unwrap();
    assert!(pos("Priority item") < pos("Cheap"));
    assert!(pos("Cheap") < pos("Plain"));
}

#[test]
fn got_toggles_status_in_place() {
    let dir = tempdir().unwrap();
    bagg(&["add", "Eggs"], dir.path()).success();
    bagg(&["got", "1"], dir.path()).success();
    assert!(read_log(dir.path()).contains("x Eggs"));
    bagg(&["got", "1"], dir.path()).success();
    assert_eq!(read_log(dir.path()).trim(), "Eggs");
}

#[test]
fn rm_removes_by_list_index() {
    let dir = tempdir().unwrap();
    bagg(&["add", "Eggs"], dir.path()).success();
    bagg(&["add", "Milk"], dir.path()).success();
    bagg(&["rm", "1"], dir.path()).success();
    let log = read_log(dir.path());
    assert!(!log.contains("Eggs"));
    assert!(log.contains("Milk"));
}

#[test]
fn list_filters_keep_original_index() {
    let dir = tempdir().unwrap();
    // Got items sort after not-got, so the full list is: Eggs(1), Milk(2), x Bread(3).
    write_log(dir.path(), "Eggs\nx Bread\nMilk\n");
    let pending = out(bagg(&["list", "--pending"], dir.path()));
    assert!(pending.contains("1  Eggs"), "{pending}");
    assert!(pending.contains("2  Milk"), "{pending}");
    assert!(!pending.contains("Bread"), "{pending}");
    let got = out(bagg(&["list", "--got"], dir.path()));
    // Bread keeps index 3 (its position in the full list) rather than
    // renumbering to 1 within the filtered subset.
    assert!(got.contains("3  x Bread"), "{got}");
}

#[test]
fn list_filters_by_tag() {
    let dir = tempdir().unwrap();
    bagg(&["add", "Screws", "+kitchen-reno"], dir.path()).success();
    bagg(&["add", "Milk"], dir.path()).success();
    let tagged = out(bagg(&["list", "+kitchen-reno"], dir.path()));
    assert!(tagged.contains("Screws"), "{tagged}");
    assert!(!tagged.contains("Milk"), "{tagged}");
    // Case-insensitive.
    let upper = out(bagg(&["list", "+KITCHEN-RENO"], dir.path()));
    assert!(upper.contains("Screws"), "{upper}");
    // Multiple queries match ANY.
    let either = out(bagg(&["list", "+other", "+kitchen-reno"], dir.path()));
    assert!(either.contains("Screws"), "{either}");
}

#[test]
fn tag_filter_composes_as_and_with_other_filters() {
    let dir = tempdir().unwrap();
    // Got item, tagged +kitchen-reno; not-got item, same tag.
    write_log(dir.path(), "x Screws +kitchen-reno\nPaint +kitchen-reno\nMilk\n");
    // --pending AND +kitchen-reno: only the not-got tagged item.
    let pending_tagged = out(bagg(&["list", "--pending", "+kitchen-reno"], dir.path()));
    assert!(pending_tagged.contains("Paint"), "{pending_tagged}");
    assert!(!pending_tagged.contains("Screws"), "{pending_tagged}");
    assert!(!pending_tagged.contains("Milk"), "{pending_tagged}");
    // --got AND +kitchen-reno: only the got tagged item.
    let got_tagged = out(bagg(&["list", "--got", "+kitchen-reno"], dir.path()));
    assert!(got_tagged.contains("Screws"), "{got_tagged}");
    assert!(!got_tagged.contains("Paint"), "{got_tagged}");
}
