#[cfg(test)]
mod integration_tests {
    use markdown2pdf::config::ConfigSource;
    use std::fs;
    use std::path::Path;

    #[test]
    fn test_render_images_md_with_relative_paths() {
        // Load the test markdown with images
        let markdown_path = Path::new("tests/images.md");
        if !markdown_path.exists() {
            eprintln!("Skipping test: {} not found", markdown_path.display());
            return;
        }

        let markdown = fs::read_to_string(markdown_path).expect("Failed to read tests/images.md");

        let output_path = "test_output_images.pdf";

        // Render with image support (using relative path resolution)
        let result = markdown2pdf::parse_into_file_with_images(
            markdown,
            output_path,
            markdown_path,
            ConfigSource::Default,
            None,
        );

        match result {
            Ok(_) => {
                // Verify PDF was created
                assert!(Path::new(output_path).exists(), "PDF file was not created");

                // Clean up
                fs::remove_file(output_path).expect("Failed to clean up test PDF");
            }
            Err(e) => {
                panic!("Failed to render images.md: {}", e);
            }
        }
    }

    #[test]
    fn test_render_images_md_to_bytes() {
        // Load the test markdown with images
        let markdown_path = Path::new("tests/images.md");
        if !markdown_path.exists() {
            eprintln!("Skipping test: {} not found", markdown_path.display());
            return;
        }

        let markdown = fs::read_to_string(markdown_path).expect("Failed to read tests/images.md");

        // Render to bytes with image support
        let result = markdown2pdf::parse_into_bytes_with_images(
            markdown,
            markdown_path,
            ConfigSource::Default,
            None,
        );

        match result {
            Ok(pdf_bytes) => {
                assert!(!pdf_bytes.is_empty(), "PDF bytes should not be empty");
                assert!(
                    pdf_bytes.starts_with(b"%PDF-"),
                    "PDF bytes should start with PDF header"
                );
            }
            Err(e) => {
                panic!("Failed to render images.md to bytes: {}", e);
            }
        }
    }
}
