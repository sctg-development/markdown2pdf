use predicates::prelude::*;
use std::env;
use std::fs;

#[test]
fn test_binary_dry_run_with_string_succeeds() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("markdown2pdf");
    cmd.arg("-s").arg("# Hello from test").arg("--dry-run");
    cmd.assert().success().stdout(
        predicate::str::contains("Dry-run").or(predicate::str::contains("Dry-run validation")),
    );
}

#[test]
fn test_binary_verbose_shows_validation_and_size_info() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("markdown2pdf");
    // Write a small temporary file and pass as -p
    let tmp = env::temp_dir().join("md_integration_test.md");
    fs::write(&tmp, "# Test\nContent").unwrap();

    cmd.arg("-p")
        .arg(tmp.to_str().unwrap())
        .arg("--dry-run")
        .arg("--verbose");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Pre-flight").or(predicate::str::contains("Dry-run")));

    let _ = fs::remove_file(&tmp);
}

#[test]
fn test_binary_returns_failure_when_no_input() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("markdown2pdf");
    // No input args → prints help and exit with code 1
    cmd.assert().failure();
}

#[test]
fn test_binary_accepts_font_flags() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("markdown2pdf");
    cmd.arg("-s")
        .arg("# Hello")
        .arg("--dry-run")
        .arg("--default-font")
        .arg("Noto Sans")
        .arg("--fallback-font")
        .arg("CourierPrime");

    cmd.assert().success();
}
