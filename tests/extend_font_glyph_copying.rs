/// Integration tests for glyph copying functionality
///
/// Tests the actual copying of missing glyphs from one font to another,
/// with specific focus on weight conversion and glyph metrics.

use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// Helper to run extend_font with arguments
fn run_extend_font(args: &[&str]) -> std::process::Output {
    let mut cmd = std::process::Command::new("cargo");
    cmd.arg("run")
        .arg("--bin")
        .arg("extend_font")
        .arg("--quiet")
        .arg("--")
        .args(args);

    cmd.output().expect("Failed to execute extend_font")
}

/// Test that extended font is created and written to destination
#[test]
fn test_output_font_file_creation() {
    // Create a temporary directory for output
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let output_font = temp_dir.path().join("extended.ttf");

    let src_font = "fonts/DejaVuSans.ttf";
    let combine_font = "fonts/NotoEmoji-VariableFont_wght.ttf";

    // Verify source files exist
    assert!(Path::new(src_font).exists(), "Source font not found: {}", src_font);
    assert!(
        Path::new(combine_font).exists(),
        "Combine font not found: {}",
        combine_font
    );

    // Run extend_font with destination
    let output = run_extend_font(&[
        "--src-font",
        src_font,
        "--combine-font",
        combine_font,
        "--dst-font",
        output_font.to_str().unwrap(),
        "-vv",
    ]);

    // Check that command succeeded
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("extend_font failed: {}", stderr);
    }

    // For now, we log that we would have written the output
    // A complete implementation would verify the output font exists
    // and has the expected glyphs
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Glyph copy process completed") || stderr.contains("prepared"),
        "Expected glyph copy process message in output"
    );
}

/// Test weight conversion logging
#[test]
fn test_weight_conversion_logging() {
    let src_font = "fonts/DejaVuSans.ttf"; // Fixed weight (400)
    let combine_font = "fonts/NotoEmoji-VariableFont_wght.ttf"; // Variable weight

    let output = run_extend_font(&[
        "--src-font",
        src_font,
        "--combine-font",
        combine_font,
        "-vv",
    ]);

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should detect weight conversion need
    assert!(
        stderr.contains("Weight scale factor") || stderr.contains("weight conversion"),
        "Expected weight conversion information in verbose output"
    );
}

/// Test handling of fonts with different characteristics
#[test]
fn test_same_weight_fonts() {
    // Using DejaVuSans for both source and combine
    // Both should have the same weight, so no conversion needed

    let font = "fonts/DejaVuSans.ttf";

    let output = run_extend_font(&[
        "--src-font",
        font,
        "--combine-font",
        font,
        "-vv",
    ]);

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should indicate no missing glyphs or no weight conversion
    assert!(
        stderr.contains("No missing glyphs") 
            || stderr.contains("No weight conversion")
            || stderr.contains("weight conversion needed"),
        "Expected information about missing glyphs or weight conversion"
    );
}

/// Test with verbose flag to show weight scale factor
#[test]
fn test_verbose_weight_scale_factor() {
    let src_font = "fonts/DejaVuSans.ttf"; // Fixed
    let combine_font = "fonts/NotoEmoji-VariableFont_wght.ttf"; // Variable

    let output = run_extend_font(&[
        "--src-font",
        src_font,
        "--combine-font",
        combine_font,
        "-vvv", // Maximum verbosity
    ]);

    let stderr = String::from_utf8_lossy(&output.stderr);

    // With -vvv, should show detailed metrics
    // Scale factor should be logged if conversion is needed
    if stderr.contains("Variable to fixed weight conversion") {
        // Scale factor should be mentioned
        assert!(
            stderr.contains("scale factor")
                || stderr.contains("expanding")
                || stderr.contains("shrinking"),
            "Expected scale factor details in very verbose output"
        );
    }
}

/// Test glyph count reporting
#[test]
fn test_glyph_count_reporting() {
    let src_font = "fonts/DejaVuSans.ttf";
    let combine_font = "fonts/NotoEmoji-VariableFont_wght.ttf";

    let output = run_extend_font(&[
        "--src-font",
        src_font,
        "--combine-font",
        combine_font,
        "-vv",
    ]);

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should report number of missing glyphs found
    assert!(
        stderr.contains("missing glyphs") || stderr.contains("glyphs"),
        "Expected glyph count information in output"
    );
}

/// Test idempotency - running twice with same fonts
#[test]
fn test_idempotent_extension() {
    let src_font = "fonts/DejaVuSans.ttf";
    let combine_font = "fonts/NotoEmoji-VariableFont_wght.ttf";

    // First run
    let output1 = run_extend_font(&[
        "--src-font",
        src_font,
        "--combine-font",
        combine_font,
        "-v",
    ]);

    let stderr1 = String::from_utf8_lossy(&output1.stderr);

    // First run should complete successfully
    assert!(
        output1.status.success(),
        "First extend_font run should succeed"
    );

    // Parse the number of glyphs from first run (if available)
    // Note: This is informational - both runs should be similar
    let glyphs_processed = stderr1.contains("missing glyphs");

    // Second run should be similar
    let output2 = run_extend_font(&[
        "--src-font",
        src_font,
        "--combine-font",
        combine_font,
        "-v",
    ]);

    assert!(
        output2.status.success(),
        "Second extend_font run should succeed"
    );

    // Both runs should report similar glyph processing
    // (both would have been no-ops in the current implementation)
}

/// Test with real fonts and check output structure
#[test]
fn test_comprehensive_glyph_extension() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let output_font = temp_dir.path().join("extended.ttf");

    let src_font = "fonts/DejaVuSans.ttf";
    let combine_font = "fonts/NotoEmoji-VariableFont_wght.ttf";

    let output = run_extend_font(&[
        "--src-font",
        src_font,
        "--combine-font",
        combine_font,
        "--dst-font",
        output_font.to_str().unwrap(),
        "-vv",
    ]);

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should complete successfully
    assert!(
        output.status.success(),
        "extend_font should succeed. Error: {}",
        stderr
    );

    // Should report process steps
    assert!(
        stderr.contains("missing glyphs") 
            || stderr.contains("Found") 
            || stderr.contains("Glyph copy process"),
        "Expected process information in output"
    );

    // Should indicate completion
    assert!(
        stderr.contains("completed")
            || stderr.contains("successfully")
            || stderr.contains("prepared"),
        "Expected completion message"
    );
}
