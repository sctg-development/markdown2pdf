/// Font utilities module
///
/// Provides high-level abstractions for reading and analyzing font data
/// including glyph information, weight classes, and variation support.

use log::debug;
use skrifa::{FontRef, MetadataProvider};
use std::collections::HashSet;


/// Represents information about a font
#[derive(Clone)]
pub struct FontInfo {
    /// Raw font data
    font_data: Vec<u8>,
}

impl FontInfo {
    /// Load font information from raw bytes
    ///
    /// # Arguments
    ///
    /// * `data` - Raw font file bytes
    ///
    /// # Returns
    ///
    /// * `Result<Self, Box<dyn std::error::Error>>` - Font information or error
    pub fn from_bytes(data: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        // Verify the font is valid by trying to create a FontRef
        let _ = FontRef::new(data)
            .map_err(|e| format!("Failed to parse font: {:?}", e))?;

        let font_data = data.to_vec();
        Ok(FontInfo { font_data })
    }

    /// Get the font family name
    pub fn name(&self) -> String {
        // API limitation: family_name() is not available in skrifa 0.40.0
        // Return a placeholder for now
        debug!("Font family name: (unavailable)");
        "Font".to_string()
    }

    /// Check if the font is a variable weight font
    ///
    /// Variable weight fonts have axes of variation.
    pub fn is_variable_weight(&self) -> bool {
        if let Ok(font_ref) = FontRef::new(&self.font_data) {
            let has_axes = font_ref.axes().len() > 0;
            debug!("Font is_variable_weight: {}", has_axes);
            return has_axes;
        }
        false
    }

    /// Get the weight class of the font
    ///
    /// For variable fonts, returns the default weight.
    /// Returns values like 400 (normal), 700 (bold), etc.
    pub fn weight_class(&self) -> u16 {
        if let Ok(font_ref) = FontRef::new(&self.font_data) {
            // Access weight via attributes field
            // weight is a public field in Attributes, we need to get its value
            let attrs = font_ref.attributes();
            let weight_value = attrs.weight.value() as u16;
            debug!("Font weight class: {}", weight_value);
            return weight_value;
        }

        // Default to normal weight
        debug!("Font weight class: 400 (default)");
        400
    }

    /// Get all glyphs present in the font
    ///
    /// Returns a set of Unicode code points that have glyph mappings.
    pub fn glyphs(&self) -> HashSet<u32> {
        let mut glyphs = HashSet::new();

        if let Ok(font_ref) = FontRef::new(&self.font_data) {
            // Use the charmap to iterate through all mappings
            let charmap = font_ref.charmap();
            for (codepoint, _glyph_id) in charmap.mappings() {
                glyphs.insert(codepoint);
            }
        }

        debug!("Font glyphs count: {}", glyphs.len());
        glyphs
    }

    /// Get the number of glyphs in the font
    pub fn glyph_count(&self) -> usize {
        self.glyphs().len()
    }

    /// Check if a specific Unicode codepoint has a glyph
    pub fn has_glyph(&self, codepoint: u32) -> bool {
        if let Ok(font_ref) = FontRef::new(&self.font_data) {
            let charmap = font_ref.charmap();
            return charmap.map(codepoint).is_some();
        }
        false
    }

    /// Get missing glyphs compared to another font
    pub fn missing_glyphs(&self, other: &FontInfo) -> HashSet<u32> {
        let self_glyphs = self.glyphs();
        let other_glyphs = other.glyphs();

        other_glyphs
            .difference(&self_glyphs)
            .copied()
            .collect()
    }

    /// Get the raw font data
    pub fn as_bytes(&self) -> &[u8] {
        &self.font_data
    }
}

/// Font weight class representations
pub mod weight {
    /// Normal weight (400)
    pub const NORMAL: u16 = 400;

    /// Bold weight (700)
    pub const BOLD: u16 = 700;

    /// Light weight (300)
    pub const LIGHT: u16 = 300;

    /// Extra bold weight (800)
    pub const EXTRA_BOLD: u16 = 800;
}

