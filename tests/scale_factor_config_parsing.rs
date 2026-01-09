//! Tests for TOML configuration parsing of SVG scale_factor.
//!
//! This module validates that scale_factor configuration works with both
//! integer and floating-point values in TOML files.

#[cfg(test)]
mod scale_factor_parsing_tests {
    use markdown2pdf::config;

    #[test]
    fn test_parse_svg_config_with_integer_scale_factor() {
        // Test parsing TOML with integer scale_factor (e.g., scale_factor = 2)
        let toml_content = r#"
[image.svg]
scale_factor = 2
"#;

        let parsed: toml::Table = toml::from_str(toml_content)
            .expect("Failed to parse TOML");

        let image_section = parsed
            .get("image")
            .and_then(|v| v.as_table())
            .expect("Missing [image] section");

        let svg_section = image_section
            .get("svg")
            .and_then(|v| v.as_table())
            .expect("Missing [image.svg] section");

        let scale_val = svg_section
            .get("scale_factor")
            .expect("Missing scale_factor");

        // The value should be readable as either integer or float
        let scale = if let Some(f) = scale_val.as_float() {
            f as f32
        } else if let Some(i) = scale_val.as_integer() {
            i as f32
        } else {
            panic!("scale_factor should be numeric");
        };

        assert_eq!(scale, 2.0, "Integer scale_factor should be parsed as 2.0");
    }

    #[test]
    fn test_parse_svg_config_with_float_scale_factor() {
        // Test parsing TOML with floating-point scale_factor (e.g., scale_factor = 2.5)
        let toml_content = r#"
[image.svg]
scale_factor = 2.5
"#;

        let parsed: toml::Table = toml::from_str(toml_content)
            .expect("Failed to parse TOML");

        let svg_section = parsed
            .get("image")
            .and_then(|v| v.as_table())
            .and_then(|v| v.get("svg").and_then(|s| s.as_table()))
            .expect("Missing [image.svg] section");

        let scale_val = svg_section
            .get("scale_factor")
            .expect("Missing scale_factor");

        let scale = scale_val
            .as_float()
            .expect("scale_factor should be a float") as f32;

        assert_eq!(scale, 2.5, "Float scale_factor should be parsed as 2.5");
    }

    #[test]
    fn test_scale_factor_greater_than_one() {
        // Test various scale_factor values > 1
        let test_values = vec![
            (r#"scale_factor = 2"#, 2.0),
            (r#"scale_factor = 3"#, 3.0),
            (r#"scale_factor = 1.5"#, 1.5),
            (r#"scale_factor = 2.5"#, 2.5),
            (r#"scale_factor = 10"#, 10.0),
            (r#"scale_factor = 0.5"#, 0.5),
        ];

        for (toml_line, expected) in test_values {
            let toml_content = format!(
                r#"
[image.svg]
{}
"#,
                toml_line
            );

            let parsed: toml::Table = toml::from_str(&toml_content)
                .expect("Failed to parse TOML");

            let svg_section = parsed
                .get("image")
                .and_then(|v| v.as_table())
                .and_then(|v| v.get("svg").and_then(|s| s.as_table()))
                .expect("Missing [image.svg] section");

            let scale_val = svg_section
                .get("scale_factor")
                .expect("Missing scale_factor");

            let scale = if let Some(f) = scale_val.as_float() {
                f as f32
            } else if let Some(i) = scale_val.as_integer() {
                i as f32
            } else {
                panic!("scale_factor should be numeric");
            };

            assert_eq!(
                scale, expected,
                "Scale factor {} should parse to {}",
                toml_line, expected
            );
        }
    }

    #[test]
    fn test_scale_factor_with_other_svg_config() {
        // Test that scale_factor works alongside other SVG configuration options
        let toml_content = r#"
[image.svg]
scale_factor = 3
width = "50%"
"#;

        let parsed: toml::Table = toml::from_str(toml_content)
            .expect("Failed to parse TOML");

        let svg_section = parsed
            .get("image")
            .and_then(|v| v.as_table())
            .and_then(|v| v.get("svg").and_then(|s| s.as_table()))
            .expect("Missing [image.svg] section");

        let scale_val = svg_section
            .get("scale_factor")
            .expect("Missing scale_factor");
        let width_val = svg_section
            .get("width")
            .expect("Missing width");

        let scale = if let Some(f) = scale_val.as_float() {
            f as f32
        } else if let Some(i) = scale_val.as_integer() {
            i as f32
        } else {
            panic!("scale_factor should be numeric");
        };

        assert_eq!(scale, 3.0, "scale_factor should be 3.0");
        assert_eq!(
            width_val.as_str().unwrap(),
            "50%",
            "width should be 50%"
        );
    }

    #[test]
    fn test_scale_factor_zero_and_negative() {
        // Test edge cases: zero and negative values
        // These might be invalid in real usage, but we test parsing capability
        let test_values = vec![
            (r#"scale_factor = 0"#, 0.0),
            (r#"scale_factor = -1"#, -1.0),
            (r#"scale_factor = -0.5"#, -0.5),
        ];

        for (toml_line, expected) in test_values {
            let toml_content = format!(
                r#"
[image.svg]
{}
"#,
                toml_line
            );

            let parsed: toml::Table = toml::from_str(&toml_content)
                .expect("Failed to parse TOML");

            let svg_section = parsed
                .get("image")
                .and_then(|v| v.as_table())
                .and_then(|v| v.get("svg").and_then(|s| s.as_table()))
                .expect("Missing [image.svg] section");

            let scale_val = svg_section
                .get("scale_factor")
                .expect("Missing scale_factor");

            let scale = if let Some(f) = scale_val.as_float() {
                f as f32
            } else if let Some(i) = scale_val.as_integer() {
                i as f32
            } else {
                panic!("scale_factor should be numeric");
            };

            assert_eq!(
                scale, expected,
                "Scale factor {} should parse to {}",
                toml_line, expected
            );
        }
    }

    #[test]
    fn test_scale_factor_very_large_value() {
        // Test with very large scale factors
        let toml_content = r#"
[image.svg]
scale_factor = 100
"#;

        let parsed: toml::Table = toml::from_str(toml_content)
            .expect("Failed to parse TOML");

        let svg_section = parsed
            .get("image")
            .and_then(|v| v.as_table())
            .and_then(|v| v.get("svg").and_then(|s| s.as_table()))
            .expect("Missing [image.svg] section");

        let scale_val = svg_section
            .get("scale_factor")
            .expect("Missing scale_factor");

        let scale = if let Some(f) = scale_val.as_float() {
            f as f32
        } else if let Some(i) = scale_val.as_integer() {
            i as f32
        } else {
            panic!("scale_factor should be numeric");
        };

        assert_eq!(scale, 100.0, "Very large scale_factor should parse correctly");
    }

    #[test]
    fn test_scale_factor_very_small_value() {
        // Test with very small scale factors (fractional)
        let toml_content = r#"
[image.svg]
scale_factor = 0.1
"#;

        let parsed: toml::Table = toml::from_str(toml_content)
            .expect("Failed to parse TOML");

        let svg_section = parsed
            .get("image")
            .and_then(|v| v.as_table())
            .and_then(|v| v.get("svg").and_then(|s| s.as_table()))
            .expect("Missing [image.svg] section");

        let scale_val = svg_section
            .get("scale_factor")
            .expect("Missing scale_factor");

        let scale = scale_val.as_float().expect("scale_factor should be a float") as f32;

        assert_eq!(scale, 0.1, "Very small scale_factor should parse correctly");
    }
}
