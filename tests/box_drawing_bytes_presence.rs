use markdown2pdf::config;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

// This test *is expected to fail* with the current behavior: box-drawing characters are
// not present as UTF-8 bytes in the generated PDF. We call the Python helper script
// `tests/check_pdf_fonts.py` and assert it reports the box-drawing characters as found.
// With the current bug the script reports them as NON TROUVÉ and this test will fail.
#[test]
fn test_box_drawing_bytes_present_in_pdf() {
    let markdown = r#"
# Box Drawing Bytes Presence

```
rust/src/
├── lib.rs                      # Library entry point
├── main.rs                     # Application entry point
└── build_info.rs               # Git/build metadata
```
"#
    .to_string();

    let result = markdown2pdf::parse_into_bytes(markdown, config::ConfigSource::Default, None);
    assert!(result.is_ok(), "Failed to render markdown to PDF bytes");
    let pdf_bytes = result.unwrap();

    // write PDF to a temporary file
    let mut tmp = std::env::temp_dir();
    tmp.push("box_drawing_test_temp.pdf");
    fs::write(&tmp, &pdf_bytes).expect("Failed to write temporary PDF file");

    // Call the python script that analyzes presence of box-drawing chars
    let output = Command::new("python3")
        .arg("tests/check_pdf_fonts.py")
        .arg(tmp.to_str().unwrap())
        .output()
        .expect("Failed to execute check_pdf_fonts.py");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // We expect the script to report that the box-drawing characters are present.
    // This assertion is intentionally written so that the test will FAIL with the
    // current behavior (script reports NON TROUVÉ).
    assert!(
        stdout.contains(" - '├': TROUVÉ"),
        "Expected script to report '├' found, stdout:\n{}",
        stdout
    );
    assert!(
        stdout.contains(" - '└': TROUVÉ"),
        "Expected script to report '└' found, stdout:\n{}",
        stdout
    );
    assert!(
        stdout.contains(" - '│': TROUVÉ"),
        "Expected script to report '│' found, stdout:\n{}",
        stdout
    );
    assert!(
        stdout.contains(" - '─': TROUVÉ"),
        "Expected script to report '─' found, stdout:\n{}",
        stdout
    );
}

// This test should PASS: the ASCII string "Library entry point" is present in the
// generated PDF and can be detected by searching the PDF bytes or via the Python script.
#[test]
fn test_library_entry_point_bytes_present_in_pdf() {
    let markdown = r#"
# Box Drawing Bytes Presence

```
rust/src/
├── lib.rs                      # Library entry point
├── main.rs                     # Application entry point
└── build_info.rs               # Git/build metadata
```
"#
    .to_string();

    let result = markdown2pdf::parse_into_bytes(markdown, config::ConfigSource::Default, None);
    assert!(result.is_ok(), "Failed to render markdown to PDF bytes");
    let pdf_bytes = result.unwrap();

    // write PDF to a temporary file
    let mut tmp = std::env::temp_dir();
    tmp.push("box_drawing_test_temp.pdf");
    fs::write(&tmp, &pdf_bytes).expect("Failed to write temporary PDF file");

    // Call the python script and check it reports the Library entry point occurrence
    let output = Command::new("python3")
        .arg("tests/check_pdf_fonts.py")
        .arg(tmp.to_str().unwrap())
        .output()
        .expect("Failed to execute check_pdf_fonts.py");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("Police(s) utilisée(s) pour 'Library entry point'"),
        "Expected script to report a font for 'Library entry point', stdout:\n{}",
        stdout
    );
    assert!(
        stdout.contains("Library entry point"),
        "Expected script to include the string 'Library entry point' in its output, stdout:\n{}",
        stdout
    );
}
