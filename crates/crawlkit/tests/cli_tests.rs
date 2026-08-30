#![allow(clippy::unwrap_used)]

use assert_cmd::Command;
use predicates::prelude::*;

fn crawlkit_cmd() -> Command {
    Command::cargo_bin("crawlkit").unwrap()
}

#[test]
fn version_flag_exits_zero_and_shows_version() {
    crawlkit_cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("crawlkit"));
}

#[test]
fn help_flag_exits_zero_and_shows_subcommands() {
    crawlkit_cmd().arg("--help").assert().success().stdout(
        predicate::str::contains("crawl")
            .and(predicate::str::contains("compare"))
            .and(predicate::str::contains("report")),
    );
}

#[test]
fn crawl_help_exits_zero_and_shows_flags() {
    crawlkit_cmd()
        .args(["crawl", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--max-pages").and(predicate::str::contains("--timeout")));
}

#[test]
fn compare_help_exits_zero() {
    crawlkit_cmd()
        .args(["compare", "--help"])
        .assert()
        .success();
}

#[test]
fn report_help_exits_zero() {
    crawlkit_cmd().args(["report", "--help"]).assert().success();
}

#[test]
fn crawl_no_args_exits_nonzero() {
    crawlkit_cmd().arg("crawl").assert().failure();
}

#[test]
fn crawl_invalid_url_exits_nonzero() {
    crawlkit_cmd()
        .args(["crawl", "not-a-url"])
        .assert()
        .failure();
}

#[test]
#[ignore]
fn crawl_unreachable_host_exits_or_zero_with_no_pages() {
    let assert = crawlkit_cmd()
        .args([
            "crawl",
            "https://invalid.example.test",
            "--max-pages",
            "1",
            "--timeout",
            "2",
        ])
        .timeout(std::time::Duration::from_secs(10))
        .assert();

    let outcome = assert.get_output();
    assert!(
        !outcome.status.success() || outcome.stdout.is_empty(),
        "crawl of unreachable host should either fail or produce no pages"
    );
}
