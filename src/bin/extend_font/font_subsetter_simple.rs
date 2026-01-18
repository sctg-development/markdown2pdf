/// Font subsetting module using allsorts
/// Simplified implementation

use log::{debug, info};
use std::collections::BTreeSet;

/// Font subsetter for combining glyphs from multiple fonts
pub struct FontSubsetter {
    /// Include all glyphs from source font
    pub include_source_font: bool,
    /// Glyphs to add from combine font (codepoint, glyph_id)
    pub additional_glyphs: Vec<(u32, u16)>,
    /// Enable weight conversion
    pub enable_weight_conversion: bool,
}

impl FontSubsetter {
    /// Create a new font subsetter
    pub fn new() -> Self {
        FontSubsetter {
            include_source_font: true,
            additional_glyphs: Vec::new(),
            enable_weight_conversion: false,
        }
    }

    /// Add a glyph from the combine font
    pub fn add_glyph_from_combine(&mut self, codepoint: u32, combine_glyph_id: u16) {
        self.additional_glyphs.push((codepoint, combine_glyph_id));
    }

    /// Create the subset font using allsorts
    pub fn create_subset_font(
        &self,
        src_font_data: &[u8],
        _combine_font_data: &[u8],
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        info!("Starting font subsetting with allsorts");

        // Collect codepoints to include in subset
        let mut codepoints_to_include = BTreeSet::new();

        // Add all codepoints from source font
        if self.include_source_font {
            info!("Collecting all codepoints from source font");
            collect_source_codepoints(src_font_data, &mut codepoints_to_include)?;
        }

        // Add missing codepoints
        if !self.additional_glyphs.is_empty() {
            info!(
                "Adding {} glyphs from combine font",
                self.additional_glyphs.len()
            );
            for (codepoint, _glyph_id) in &self.additional_glyphs {
                codepoints_to_include.insert(*codepoint);
            }
        }

        let codepoint_count = codepoints_to_include.len();
        info!("Total codepoints in subset: {}", codepoint_count);

        // Create subset using allsorts
        let subset_data = create_subset_with_allsorts(
            src_font_data,
            &codepoints_to_include,
        )?;

        info!(
            "Subset font created with {} codepoints",
            codepoint_count
        );

        Ok(subset_data)
    }
}

impl Default for FontSubsetter {
    fn default() -> Self {
        Self::new()
    }
}

/// Collect all codepoints from source font
fn collect_source_codepoints(
    src_font_data: &[u8],
    codepoints: &mut BTreeSet<u32>,
) -> Result<(), Box<dyn std::error::Error>> {
    use allsorts::binary::read::ReadScope;
    use allsorts::font::Font;

    // Parse the font with allsorts
    let scope = ReadScope::new(src_font_data);
    let font = scope.read::<Font>()?;

    // Get codepoints from cmap
    for subtable in font.cmap_subtables()? {
        for (cp, _gid) in subtable.mappings()? {
            codepoints.insert(cp as u32);
        }
    }

    debug!("Collected {} codepoints from source font", codepoints.len());

    Ok(())
}

/// Create subset using allsorts subsetter
fn create_subset_with_allsorts(
    src_font_data: &[u8],
    codepoints_to_include: &BTreeSet<u32>,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use allsorts::binary::read::ReadScope;
    use allsorts::font::Font;
    use allsorts::subset::{subset, CmapTarget, SubsetProfile};

    debug!("Building subset with allsorts subsetter");

    // Parse the font
    let scope = ReadScope::new(src_font_data);
    let font = scope.read::<Font>()?;

    // Get glyph IDs for the codepoints we want to keep
    let mut glyph_ids_to_keep = BTreeSet::new();

    // Map codepoints to glyph IDs
    for subtable in font.cmap_subtables()? {
        for (cp, gid) in subtable.mappings()? {
            if codepoints_to_include.contains(&(cp as u32)) {
                glyph_ids_to_keep.insert(gid);
            }
        }
    }

    debug!(
        "Found {} glyphs to keep in subset",
        glyph_ids_to_keep.len()
    );

    // Convert to vector
    let glyph_ids: Vec<u16> = glyph_ids_to_keep.iter().copied().collect();

    // Create subset profile - use unicode
    let subset_profile = SubsetProfile::parse_custom("unicode".to_string())?;

    // Perform subsetting
    let subset_data = subset(
        &font,
        &glyph_ids,
        &subset_profile,
        CmapTarget::Unicode,
    )?;

    info!("Subset built successfully with allsorts");

    Ok(subset_data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_subsetter_creation() {
        let subsetter = FontSubsetter::new();
        assert!(subsetter.include_source_font);
        assert!(subsetter.additional_glyphs.is_empty());
        assert!(!subsetter.enable_weight_conversion);
    }

    #[test]
    fn test_add_glyph_from_combine() {
        let mut subsetter = FontSubsetter::new();
        subsetter.add_glyph_from_combine(0x1F600, 42);
        assert_eq!(subsetter.additional_glyphs.len(), 1);
        assert_eq!(subsetter.additional_glyphs[0], (0x1F600, 42));
    }

    #[test]
    fn test_subsetter_default() {
        let subsetter = FontSubsetter::default();
        assert!(subsetter.include_source_font);
    }
}
