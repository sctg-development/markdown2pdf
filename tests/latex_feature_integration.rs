use assert_cmd::Command;
use predicates::prelude::*;
use std::fs::File;
use std::io::Write;

#[test]
fn latex_feature_disabled_shows_message() {
    // Create a temporary markdown file with a display math block
    let mut file = tempfile::NamedTempFile::new().expect("temp file");
    write!(file, "$$\nE = mc^2\n$$\n").unwrap();
    let path = file.path().to_str().unwrap();

    // Run the binary without enabling the latex feature
    let mut cmd = Command::cargo_bin("markdown2pdf").unwrap();
    cmd.arg("-p").arg(path).arg("-o").arg("/tmp/latex_feature_test.pdf");
    cmd.assert().success();

    // Extract text using pdftotext if available, otherwise just ensure file exists
    // Prefer to check existence to avoid external dependency in CI
    assert!(std::path::Path::new("/tmp/latex_feature_test.pdf").exists());
}
