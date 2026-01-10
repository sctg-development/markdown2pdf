// Test the proper handling of Markdown line breaks vs paragraph breaks

#[test]
fn test_single_newline_ignored() {
    // In Markdown, single newlines should be treated as whitespace
    // This is tested through rendering behavior
    let markdown = "First line\nSecond line".to_string();
    
    // This should parse without errors
    let result = markdown2pdf::parse_into_file(
        markdown,
        "/tmp/test_single_newline.pdf",
        markdown2pdf::config::ConfigSource::Default,
        None,
    );
    
    // The result should succeed (both lines in same paragraph if no line break specified)
    assert!(result.is_ok(), "Failed to parse single newline markdown: {:?}", result.err());
}

#[test]
fn test_double_newline_paragraph_break() {
    // Double newline should create a paragraph break
    let markdown = "First paragraph\n\nSecond paragraph".to_string();
    
    let result = markdown2pdf::parse_into_file(
        markdown,
        "/tmp/test_double_newline.pdf",
        markdown2pdf::config::ConfigSource::Default,
        None,
    );
    
    assert!(result.is_ok(), "Failed to parse double newline markdown: {:?}", result.err());
}

#[test]
fn test_explicit_linebreak_with_two_spaces() {
    // Two spaces before newline should create an explicit line break
    let markdown = "First line  \nSecond line".to_string();
    
    let result = markdown2pdf::parse_into_file(
        markdown,
        "/tmp/test_explicit_linebreak.pdf",
        markdown2pdf::config::ConfigSource::Default,
        None,
    );
    
    assert!(result.is_ok(), "Failed to parse explicit line break markdown: {:?}", result.err());
}

#[test]
fn test_badges_without_paragraph_break() {
    // Multiple images without blank lines should be rendered in same area
    let markdown = r#"# Badges
![codecov](https://example.com/badge1.svg)
![CI](https://example.com/badge2.svg)
![Rust](https://example.com/badge3.svg)"#.to_string();
    
    let result = markdown2pdf::parse_into_file(
        markdown,
        "/tmp/test_badges.pdf",
        markdown2pdf::config::ConfigSource::Default,
        None,
    );
    
    assert!(result.is_ok(), "Failed to parse badges markdown: {:?}", result.err());
}
