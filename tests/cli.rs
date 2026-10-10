use std::fs;
use std::path::Path;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn dirsize() -> Command {
    Command::cargo_bin("dirsize").expect("binary is built")
}

fn write_bytes(path: &Path, len: usize) {
    fs::write(path, vec![0u8; len]).expect("write test file");
}

/// tmp/
///   big/   a.bin (3000), b.bin (1000)
///   mid.txt (500)
///   tiny.txt (10)
fn fixture() -> TempDir {
    let tmp = TempDir::new().expect("create temp dir");
    fs::create_dir(tmp.path().join("big")).expect("create dir");
    write_bytes(&tmp.path().join("big/a.bin"), 3000);
    write_bytes(&tmp.path().join("big/b.bin"), 1000);
    write_bytes(&tmp.path().join("mid.txt"), 500);
    write_bytes(&tmp.path().join("tiny.txt"), 10);
    tmp
}

fn stdout_lines(cmd: &mut Command) -> Vec<String> {
    let output = cmd.assert().success().get_output().stdout.clone();
    String::from_utf8(output)
        .expect("utf-8 output")
        .lines()
        .map(str::to_string)
        .collect()
}

#[test]
fn prints_totals_and_sorts_largest_first() {
    let tmp = fixture();
    let lines = stdout_lines(dirsize().arg(tmp.path()).arg("--bytes"));

    assert_eq!(lines.len(), 6);
    assert!(lines[0].trim_start().starts_with("4510 "));
    assert!(lines[0].contains("100%"));
    assert!(lines[1].contains("4000") && lines[1].ends_with("├── big/"));
    assert!(lines[2].contains("3000") && lines[2].ends_with("│   ├── a.bin"));
    assert!(lines[3].contains("1000") && lines[3].ends_with("│   └── b.bin"));
    assert!(lines[4].contains("500") && lines[4].ends_with("├── mid.txt"));
    assert!(lines[5].contains("10") && lines[5].ends_with("└── tiny.txt"));
}

#[test]
fn human_readable_sizes_by_default() {
    let tmp = fixture();
    dirsize()
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("4.4 KiB"))
        .stdout(predicate::str::contains("500 B"));
}

#[test]
fn depth_limits_how_far_the_tree_goes() {
    let tmp = fixture();
    dirsize()
        .arg(tmp.path())
        .args(["--depth", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("big/"))
        .stdout(predicate::str::contains("a.bin").not());
}

#[test]
fn top_collapses_the_remaining_entries() {
    let tmp = fixture();
    let lines = stdout_lines(
        dirsize()
            .arg(tmp.path())
            .args(["--bytes", "-n", "1", "-d", "1"]),
    );

    assert_eq!(lines.len(), 3);
    assert!(lines[1].ends_with("├── big/"));
    assert!(lines[2].contains("510") && lines[2].ends_with("└── (2 more)"));
}

#[test]
fn dirs_only_hides_files() {
    let tmp = fixture();
    dirsize()
        .arg(tmp.path())
        .arg("--dirs-only")
        .assert()
        .success()
        .stdout(predicate::str::contains("big/"))
        .stdout(predicate::str::contains("mid.txt").not())
        .stdout(predicate::str::contains("a.bin").not());
}

#[test]
fn defaults_to_the_current_directory() {
    let tmp = fixture();
    dirsize()
        .current_dir(tmp.path())
        .arg("--bytes")
        .assert()
        .success()
        .stdout(predicate::str::contains("4510"))
        .stdout(predicate::str::contains("./"));
}

#[test]
fn missing_path_is_an_error() {
    let tmp = TempDir::new().expect("create temp dir");
    dirsize()
        .arg(tmp.path().join("does-not-exist"))
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("cannot read"));
}

#[cfg(unix)]
#[test]
fn does_not_follow_symlinks() {
    let tmp = fixture();
    std::os::unix::fs::symlink(tmp.path().join("big"), tmp.path().join("link"))
        .expect("create symlink");

    // If the link were followed, big/'s 4000 bytes would be counted twice.
    let lines = stdout_lines(dirsize().arg(tmp.path()).args(["--bytes", "-d", "0"]));
    let total: u64 = lines[0]
        .split_whitespace()
        .next()
        .and_then(|n| n.parse().ok())
        .expect("total size");
    assert!((4510..4510 + 4000).contains(&total), "total was {total}");
}

#[cfg(unix)]
#[test]
fn unreadable_directories_are_skipped_with_a_warning() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = fixture();
    let locked = tmp.path().join("locked");
    fs::create_dir(&locked).expect("create dir");
    write_bytes(&locked.join("secret.bin"), 9999);
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).expect("lock dir");

    // Root ignores permission bits, so there is nothing to observe.
    let can_still_read = fs::read_dir(&locked).is_ok();

    let assert = dirsize().arg(tmp.path()).arg("--bytes").assert();
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).expect("unlock dir");

    if can_still_read {
        return;
    }
    assert
        .success()
        .stdout(predicate::str::contains("4510"))
        .stdout(predicate::str::contains("locked/"))
        .stderr(predicate::str::contains("dirsize: skipped"))
        .stderr(predicate::str::contains("locked"));
}
