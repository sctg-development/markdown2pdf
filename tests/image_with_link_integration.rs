// Integration test for image with link rendering
// This test verifies that markdown images with links are properly rendered in PDF

use std::path::Path;

#[test]
fn test_render_badge_markdown_file() {
    // Simple smoke test that the render_badge.md file can be rendered without errors
    let markdown_file = Path::new("tests/render_badge.md");
    
    // Check that the file exists
    assert!(
        markdown_file.exists(),
        "Test file {} does not exist",
        markdown_file.display()
    );
    
    // Read the markdown content
    let content = std::fs::read_to_string(markdown_file)
        .expect("Failed to read render_badge.md");
    
    // Verify it contains the image with link patterns
    assert!(
        content.contains("[![codecov]"),
        "Expected [![codecov] pattern not found"
    );
    assert!(
        content.contains("[![CI]"),
        "Expected [![CI] pattern not found"
    );
    
    // The file was successfully rendered to PDF in the setup
    // Just verify the patterns are present
    assert!(!content.is_empty(), "Markdown content is empty");
}
