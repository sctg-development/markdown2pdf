/// Integration tests for the extend_font binary
///
/// Tests the font extension functionality with real font files,
/// including missing glyph detection, weight conversion detection,
/// and directory creation.
use std::path::Path;
use std::process::Command;

/// Helper function to run the extend_font binary with arguments
/// Returns (stdout, stderr)
fn run_extend_font(args: &[&str]) -> (String, String) {
    let mut cmd = Command::new("cargo");
    cmd.arg("run")
        .arg("--bin")
        .arg("extend_font")
        .arg("--quiet")
        .arg("--")
        .args(args);

    let output = cmd.output().expect("Failed to execute extend_font");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    (stdout, stderr)
}

/// Test that extend_font can detect missing glyphs
#[test]
fn test_missing_glyphs_detection() {
    // This test verifies that the binary correctly identifies glyphs in one font
    // that are missing from another.
    //
    // Setup: Use DejaVuSans (broad Latin support) as source and
    // NotoEmoji (emoji support) as combine font
    //
    // Expected: Binary should report missing emoji glyphs in DejaVuSans

    let src_font = "fonts/DejaVuSans.ttf";
    let combine_font = "fonts/NotoEmoji-VariableFont_wght.ttf";

    // Verify test font files exist
    assert!(
        Path::new(src_font).exists(),
        "Test font {} not found",
        src_font
    );
    assert!(
        Path::new(combine_font).exists(),
        "Test font {} not found",
        combine_font
    );

    // Run with verbose output to see what was detected
    let (_, stderr) = run_extend_font(&[
        "--src-font",
        src_font,
        "--combine-font",
        combine_font,
        "-vv",
    ]);

    // Check that the output indicates missing glyphs were found
    assert!(
        stderr.contains("missing glyphs"),
        "Expected 'missing glyphs' in output, got: {}",
        stderr
    );

    // Verify successful completion
    assert!(
        stderr.contains("successfully"),
        "Expected successful completion message, got: {}",
        stderr
    );
}

/// Test variable weight detection
#[test]
fn test_variable_weight_detection() {
    // This test verifies that the binary correctly detects when
    // the combine font is variable weight (NotoEmoji) while
    // the source font is fixed weight (DejaVuSans)

    let src_font = "fonts/DejaVuSans.ttf";
    let combine_font = "fonts/NotoEmoji-VariableFont_wght.ttf";

    let (_, stderr) = run_extend_font(&[
        "--src-font",
        src_font,
        "--combine-font",
        combine_font,
        "-vv",
    ]);

    // Should detect weight conversion need
    assert!(
        stderr.contains("Variable to fixed weight conversion")
            || stderr.contains("weight conversion"),
        "Expected weight conversion warning in output, got: {}",
        stderr
    );
}

/// Test in-place modification when no destination is specified
#[test]
fn test_inplace_modification() {
    // When --dst-font is not provided, the binary should modify
    // the source font in place

    let src_font = "fonts/DejaVuSans.ttf";
    let combine_font = "fonts/NotoEmoji-VariableFont_wght.ttf";

    let (_, stderr) = run_extend_font(&["--src-font", src_font, "--combine-font", combine_font]);

    // Should indicate in-place modification
    assert!(
        stderr.contains("in place")
            || stderr.contains("Modified")
            || stderr.contains("successfully"),
        "Expected in-place modification message, got: {}",
        stderr
    );
}

/// Test help output
#[test]
fn test_help_output() {
    let (stdout, _) = run_extend_font(&["--help"]);

    // Verify all expected argument names appear in help
    assert!(stdout.contains("--src-font"), "Missing --src-font in help");
    assert!(
        stdout.contains("--combine-font"),
        "Missing --combine-font in help"
    );
    assert!(stdout.contains("--dst-font"), "Missing --dst-font in help");
    assert!(stdout.contains("-v"), "Missing -v option in help");
    assert!(stdout.contains("-q"), "Missing -q option in help");
    assert!(
        stdout.contains("--verbose-level"),
        "Missing --verbose-level in help"
    );
}

/// Test version output
#[test]
fn test_version_output() {
    let (stdout, _) = run_extend_font(&["--version"]);

    // Should output version information
    assert!(!stdout.is_empty(), "Expected version output");
}

/// Test verbosity control
#[test]
fn test_verbosity_levels() {
    let src_font = "fonts/DejaVuSans.ttf";
    let combine_font = "fonts/NotoEmoji-VariableFont_wght.ttf";

    // Test with -vv (debug level)
    let (_, stderr_debug) = run_extend_font(&[
        "--src-font",
        src_font,
        "--combine-font",
        combine_font,
        "-vv",
    ]);

    // Should contain debug-level messages
    assert!(
        stderr_debug.contains("DEBUG"),
        "Expected DEBUG level output with -vv, got: {}",
        stderr_debug
    );

    // Test with quiet flag
    let (_, stderr_quiet) =
        run_extend_font(&["--src-font", src_font, "--combine-font", combine_font, "-q"]);

    // Quiet should have less output
    // (It might still have some INFO level, but definitely less DEBUG)
    assert!(
        stderr_quiet.lines().count() < stderr_debug.lines().count(),
        "Expected less output with -q flag"
    );
}

/// Test with same font for both source and combine
#[test]
fn test_same_source_and_combine_font() {
    // When source and combine fonts are the same,
    // there should be no missing glyphs to copy

    let font = "fonts/DejaVuSans.ttf";

    let (_, stderr) = run_extend_font(&["--src-font", font, "--combine-font", font, "-vv"]);

    // Should report no missing glyphs
    assert!(
        stderr.contains("No missing glyphs"),
        "Expected 'No missing glyphs' message, got: {}",
        stderr
    );
}

/// Test error handling with non-existent font file
#[test]
fn test_error_nonexistent_font() {
    let mut cmd = std::process::Command::new("cargo");
    cmd.arg("run")
        .arg("--bin")
        .arg("extend_font")
        .arg("--quiet")
        .arg("--")
        .arg("--src-font")
        .arg("nonexistent.ttf")
        .arg("--combine-font")
        .arg("fonts/DejaVuSans.ttf");

    let output = cmd.output().expect("Failed to run command");

    // Should fail with appropriate error
    assert!(
        !output.status.success(),
        "Expected command to fail with non-existent font"
    );
}
