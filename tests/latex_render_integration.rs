#[cfg(test)]
mod integration_tests {
    use markdown2pdf::config::ConfigSource;
    use std::fs;
    use std::path::Path;

    /// Tests that generate actual PDFs from the LaTeX example. These are marked
    /// `#[ignore]` because they rely on MicroTeX native dependencies and can be slow.
    /// Run locally with: `cargo test --test latex_render_integration -- --ignored`

    #[test]
    #[ignore]
    fn test_render_latex_md_with_file_output() {
        let markdown_path = Path::new("tests/latex_examples.md");
        if !markdown_path.exists() {
            eprintln!("Skipping test: {} not found", markdown_path.display());
            return;
        }

        let markdown =
            fs::read_to_string(markdown_path).expect("Failed to read tests/latex_examples.md");
        let output_path = "test_output_latex.pdf";

        let result = markdown2pdf::parse_into_file_with_images(
            markdown,
            output_path,
            markdown_path,
            ConfigSource::Default,
            None,
        );

        match result {
            Ok(_) => {
                assert!(Path::new(output_path).exists(), "PDF file was not created");
                fs::remove_file(output_path).expect("Failed to clean up test PDF");
            }
            Err(e) => {
                panic!("Failed to render latex_examples.md: {}", e);
            }
        }
    }

    #[test]
    #[ignore]
    fn test_render_latex_md_to_bytes() {
        let markdown_path = Path::new("tests/latex_examples.md");
        if !markdown_path.exists() {
            eprintln!("Skipping test: {} not found", markdown_path.display());
            return;
        }

        let markdown =
            fs::read_to_string(markdown_path).expect("Failed to read tests/latex_examples.md");

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
                panic!("Failed to render latex_examples.md to bytes: {}", e);
            }
        }
    }
}
