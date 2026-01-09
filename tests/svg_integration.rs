#[cfg(test)]
mod svg_integration_tests {
    use markdown2pdf::config::ConfigSource;
    use std::fs;
    use std::path::Path;

    /// Test rendering SVG with default scale_factor (1.0 = original size)
    /// This verifies that SVGs render at their original dimensions when no config is provided
    #[test]
    fn test_render_svg_default_scale_factor() {
        let markdown_path = Path::new("tests/render_badge.md");
        if !markdown_path.exists() {
            eprintln!("Skipping test: {} not found", markdown_path.display());
            return;
        }

        let markdown = fs::read_to_string(markdown_path).expect("Failed to read tests/render_badge.md");
        let output_path = "test_svg_default.pdf";

        // Render with default config (scale_factor = 1.0)
        let result = markdown2pdf::parse_into_file_with_images(
            markdown,
            output_path,
            markdown_path,
            ConfigSource::Default,
            None,
        );

        match result {
            Ok(_) => {
                assert!(Path::new(output_path).exists(), "Output PDF was not created");
                // Clean up
                let _ = fs::remove_file(output_path);
            }
            Err(e) => panic!("Failed to render SVG with default scale_factor: {}", e),
        }
    }

    /// Test rendering SVG with explicit scale_factor configuration
    /// This verifies that SVG size scales correctly with scale_factor
    /// scale_factor multiplies the intrinsic SVG dimensions
    #[test]
    fn test_render_svg_with_scale_factor() {
        let markdown_path = Path::new("tests/render_badge.md");
        if !markdown_path.exists() {
            eprintln!("Skipping test: {} not found", markdown_path.display());
            return;
        }

        let markdown = fs::read_to_string(markdown_path).expect("Failed to read tests/render_badge.md");
        let output_path = "test_svg_scale_0_5.pdf";

        // Create a temporary config with scale_factor = 0.5
        let config_path = "test_svg_scale_0_5.toml";
        let config_content = r#"[image.svg]
scale_factor = 0.5
"#;
        fs::write(config_path, config_content)
            .expect("Failed to write temporary config file");

        // Render with scale_factor = 0.5 (50% of original size)
        let result = markdown2pdf::parse_into_file_with_images(
            markdown,
            output_path,
            markdown_path,
            ConfigSource::File(config_path),
            None,
        );

        // Clean up
        let _ = fs::remove_file(config_path);

        match result {
            Ok(_) => {
                assert!(Path::new(output_path).exists(), "Output PDF was not created");
                // Clean up
                let _ = fs::remove_file(output_path);
            }
            Err(e) => panic!("Failed to render SVG with scale_factor = 0.5: {}", e),
        }
    }

    /// Test rendering SVG with scale_factor > 1 to enlarge the image
    /// This verifies that scale_factor correctly supports values > 1 for enlarging SVGs
    /// scale_factor = 2.0 should make SVG 200% (2x) the original size
    #[test]
    fn test_render_svg_with_scale_factor_greater_than_one() {
        let markdown_path = Path::new("tests/render_badge.md");
        if !markdown_path.exists() {
            eprintln!("Skipping test: {} not found", markdown_path.display());
            return;
        }

        let markdown = fs::read_to_string(markdown_path).expect("Failed to read tests/render_badge.md");
        let output_path = "test_svg_scale_2_0.pdf";

        // Create a temporary config with scale_factor = 2.0 (enlarge to 2x original size)
        let config_path = "test_svg_scale_2_0.toml";
        let config_content = r#"[image.svg]
scale_factor = 2.0
"#;
        fs::write(config_path, config_content)
            .expect("Failed to write temporary config file");

        // Render with scale_factor = 2.0 (200% of original size)
        let result = markdown2pdf::parse_into_file_with_images(
            markdown,
            output_path,
            markdown_path,
            ConfigSource::File(config_path),
            None,
        );

        // Clean up
        let _ = fs::remove_file(config_path);

        match result {
            Ok(_) => {
                assert!(Path::new(output_path).exists(), "Output PDF was not created");
                // Clean up
                let _ = fs::remove_file(output_path);
            }
            Err(e) => panic!("Failed to render SVG with scale_factor = 2.0: {}", e),
        }
    }

    /// Test rendering SVG with width configuration
    /// This verifies that width parameter works correctly
    /// width = "50%" should make SVG 50% of page width
    #[test]
    fn test_render_svg_with_width_percentage() {
        let markdown_path = Path::new("tests/render_badge.md");
        if !markdown_path.exists() {
            eprintln!("Skipping test: {} not found", markdown_path.display());
            return;
        }

        let markdown = fs::read_to_string(markdown_path).expect("Failed to read tests/render_badge.md");
        let output_path = "test_svg_width_50.pdf";

        // Create a temporary config with width = "50%"
        let config_path = "test_svg_width_50.toml";
        let config_content = r#"[image.svg]
width = "50%"
"#;
        fs::write(config_path, config_content)
            .expect("Failed to write temporary config file");

        // Render with width = "50%"
        let result = markdown2pdf::parse_into_file_with_images(
            markdown,
            output_path,
            markdown_path,
            ConfigSource::File(config_path),
            None,
        );

        // Clean up
        let _ = fs::remove_file(config_path);

        match result {
            Ok(_) => {
                assert!(Path::new(output_path).exists(), "Output PDF was not created");
                // Clean up
                let _ = fs::remove_file(output_path);
            }
            Err(e) => panic!("Failed to render SVG with width = '50%': {}", e),
        }
    }

    /// Test that width parameter surcharges scale_factor
    /// When both width and scale_factor are provided, width should take priority
    /// width = "50%" should override scale_factor = 0.5
    #[test]
    fn test_render_svg_width_surcharges_scale_factor() {
        let markdown_path = Path::new("tests/render_badge.md");
        if !markdown_path.exists() {
            eprintln!("Skipping test: {} not found", markdown_path.display());
            return;
        }

        let markdown = fs::read_to_string(markdown_path).expect("Failed to read tests/render_badge.md");
        let output_path = "test_svg_width_surcharge.pdf";

        // Create a temporary config with both width and scale_factor
        // width should take priority, scale_factor should be ignored
        let config_path = "test_svg_width_surcharge.toml";
        let config_content = r#"[image.svg]
width = "50%"
scale_factor = 0.8
"#;
        fs::write(config_path, config_content)
            .expect("Failed to write temporary config file");

        // Render with both width and scale_factor
        // Expected: width takes priority, SVG should be at 50% of page (not 80%)
        let result = markdown2pdf::parse_into_file_with_images(
            markdown,
            output_path,
            markdown_path,
            ConfigSource::File(config_path),
            None,
        );

        // Clean up
        let _ = fs::remove_file(config_path);

        match result {
            Ok(_) => {
                assert!(Path::new(output_path).exists(), "Output PDF was not created");
                // Clean up
                let _ = fs::remove_file(output_path);
            }
            Err(e) => panic!("Failed to render SVG with width surcharging scale_factor: {}", e),
        }
    }
}
