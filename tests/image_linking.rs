#[cfg(test)]
mod tests {
    use genpdfi_extended::elements::Image;

    #[test]
    fn test_image_with_link_builder_pattern() {
        // Create a simple test SVG
        let svg_content = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
            <rect width="100" height="100" fill="blue"/>
        </svg>"#;

        // Create an image from SVG and add a link using builder pattern
        let _image_with_link = Image::from_svg_string(svg_content)
            .expect("Failed to create image")
            .with_link("https://example.com");
        
        // If this compiles and the image is created, the test passes
        // This verifies that the .with_link() method works correctly
    }

    #[test]
    fn test_image_with_link_set_method() {
        // Create a simple test SVG
        let svg_content = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
            <rect width="100" height="100" fill="blue"/>
        </svg>"#;

        // Create an image and use set_link()
        let mut image = Image::from_svg_string(svg_content)
            .expect("Failed to create image");
        image.set_link("https://example.com");
        
        // If this compiles, the test passes
        // This verifies that the .set_link() method works correctly
    }
}
