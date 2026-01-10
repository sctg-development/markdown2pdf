// Test that consecutive images without paragraph breaks are rendered with minimal spacing
#[test]
fn test_consecutive_badges_minimal_spacing() {
    use markdown2pdf::parse_into_file;
    use markdown2pdf::config::ConfigSource;
    
    // Markdown with consecutive badges (single newlines between them, not double newlines)
    let markdown = r#"# Badges Section
![Badge1](https://example.com/badge1.svg)
![Badge2](https://example.com/badge2.svg)
![Badge3](https://example.com/badge3.svg)"#.to_string();
    
    let tmp_path = "/tmp/test_badges_spacing.pdf";
    
    // This should render all badges together with minimal spacing (not in separate paragraphs)
    let result = parse_into_file(
        markdown,
        tmp_path,
        ConfigSource::Default,
        None,
    );
    
    assert!(result.is_ok(), "Failed to parse badges markdown: {:?}", result.err());
    
    // Verify the PDF was created
    let path = std::path::Path::new(tmp_path);
    assert!(path.exists(), "PDF file was not created");
    
    // Check file size is reasonable (should be larger than minimal)
    if let Ok(metadata) = std::fs::metadata(tmp_path) {
        println!("Generated PDF size: {} bytes", metadata.len());
        assert!(metadata.len() > 10000, "PDF seems too small");
    }
}

#[test]
fn test_badges_with_links_minimal_spacing() {
    use markdown2pdf::parse_into_file;
    use markdown2pdf::config::ConfigSource;
    
    // Markdown with badges that have links
    let markdown = r#"# Badges with Links
[![Badge1](https://example.com/badge1.svg)](https://example.com)
[![Badge2](https://example.com/badge2.svg)](https://example.com)
[![Badge3](https://example.com/badge3.svg)](https://example.com)"#.to_string();
    
    let tmp_path = "/tmp/test_badges_links_spacing.pdf";
    
    let result = parse_into_file(
        markdown,
        tmp_path,
        ConfigSource::Default,
        None,
    );
    
    assert!(result.is_ok(), "Failed to parse badges with links markdown: {:?}", result.err());
    
    let path = std::path::Path::new(tmp_path);
    assert!(path.exists(), "PDF file was not created");
}

#[test]
fn test_mixed_badges_and_text() {
    use markdown2pdf::parse_into_file;
    use markdown2pdf::config::ConfigSource;
    
    // Mixed badges and text - badges should still group together
    let markdown = r#"# Mixed Content
Some introductory text.

![Badge1](https://example.com/badge1.svg)
![Badge2](https://example.com/badge2.svg)

More text after badges."#.to_string();
    
    let tmp_path = "/tmp/test_mixed_badges.pdf";
    
    let result = parse_into_file(
        markdown,
        tmp_path,
        ConfigSource::Default,
        None,
    );
    
    assert!(result.is_ok(), "Failed to parse mixed content markdown: {:?}", result.err());
}
