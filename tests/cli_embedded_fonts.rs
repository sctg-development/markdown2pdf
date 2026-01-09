use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

#[test]
fn test_cli_lists_embedded_fonts() {
    // Run the built binary and ensure it prints canonical embedded family names
    let mut cmd = cargo_bin_cmd!("markdown2pdf");
    cmd.arg("--list-embedded-fonts");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("DejaVu Sans"))
        .stdout(predicate::str::contains("DejaVu Sans Mono"))
        .stdout(predicate::str::contains("CMU Typewriter Text"));
}
