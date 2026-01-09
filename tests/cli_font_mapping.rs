use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

#[test]
fn test_cli_maps_helvetica_to_dejavu_sans() {
    let tmp = tempdir().unwrap();
    let out = tmp.path().join("out.pdf");

    let mut cmd = Command::cargo_bin("markdown2pdf").unwrap();
    cmd.arg("-s")
        .arg("# Hello")
        .arg("-o")
        .arg(out.to_str().unwrap())
        .arg("--default-font")
        .arg("Helvetica");

    // Expect the binary to succeed and to emit the embedded font message to stderr
    cmd.assert()
        .success()
        .stderr(predicate::str::contains("Using embedded font family 'DejaVu Sans'"));
}
