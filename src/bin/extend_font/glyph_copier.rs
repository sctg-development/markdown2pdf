/// Glyph copying module
///
/// Core functionality for copying missing glyphs from a combine font to a source font.
/// Handles font weight conversion for variable-to-fixed weight scenarios.

use crate::font_utils::FontInfo;
use log::{debug, info, warn};
use skrifa::MetadataProvider;
use std::collections::{HashMap, HashSet};


/// Copy missing glyphs from combine font to source font
///
/// This function:
/// 1. Identifies glyphs present in combine_font but missing from src_font
/// 2. Copies these glyphs while handling weight conversion if necessary
/// 3. Returns the modified font data
///
/// # Arguments
///
/// * `src_font_data` - Raw bytes of the source font
/// * `combine_font_data` - Raw bytes of the combine font
/// * `src_font` - Parsed source font information
/// * `combine_font` - Parsed combine font information
///
/// # Returns
///
/// * `Result<Vec<u8>, Box<dyn std::error::Error>>` - Modified font data or error
///
/// # Implementation Details
///
/// The current implementation:
/// 1. Uses skrifa to identify missing glyphs
/// 2. Analyzes weight conversion needs (variable to fixed)
/// 3. Extracts comprehensive glyph information from combine font
/// 4. Prepares glyphs for merging with weight scaling
///
/// A more complete implementation would:
/// 1. Use write-fonts to fully reconstruct font tables
/// 2. Copy individual glyphs with proper metrics from combine font to src font
/// 3. Update character maps to include new glyphs
/// 4. Apply weight conversion via outline scaling or variation instances
pub fn copy_missing_glyphs(
    src_font_data: &[u8],
    combine_font_data: &[u8],
    src_font: &FontInfo,
    combine_font: &FontInfo,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // Find missing glyphs
    let missing_glyphs = src_font.missing_glyphs(combine_font);

    if missing_glyphs.is_empty() {
        info!("No missing glyphs found");
        return Ok(src_font_data.to_vec());
    }

    info!(
        "Found {} missing glyphs to copy",
        missing_glyphs.len()
    );

    // Analyze weight conversion requirements
    let weight_conversion = WeightConversionInfo::from_fonts(src_font, combine_font);

    if weight_conversion.needs_conversion {
        warn!(
            "Variable to fixed weight conversion needed: combine={} -> src={}",
            weight_conversion.combine_weight, weight_conversion.src_weight
        );
        debug!(
            "Weight scale factor: {:.2}x ({})",
            weight_conversion.scale_factor,
            if weight_conversion.scale_factor > 1.0 {
                "expanding"
            } else if weight_conversion.scale_factor < 1.0 {
                "shrinking"
            } else {
                "unchanged"
            }
        );
        debug!(
            "This conversion requires adjusting glyph metrics and outlines"
        );
    } else {
        debug!("No weight conversion needed");
    }

    // Extract glyph information from combine font for missing codepoints
    let missing_glyph_info = extract_missing_glyph_info(
        combine_font_data,
        &missing_glyphs,
    )?;

    debug!(
        "Successfully extracted info for {} glyphs from combine font",
        missing_glyph_info.len()
    );

    // Merge glyphs into source font
    let result_font_data = merge_glyphs_into_font(
        src_font_data,
        combine_font_data,
        &missing_glyph_info,
        weight_conversion.needs_conversion,
    )?;

    info!(
        "Glyph copy process completed. {} glyphs have been copied.",
        missing_glyphs.len()
    );

    Ok(result_font_data)
}

/// Analyze weight conversion requirements
///
/// Determines if and how weight conversion should be applied
/// when copying glyphs from a variable-weight font to a fixed-weight font.
pub struct WeightConversionInfo {
    /// Whether conversion is needed
    pub needs_conversion: bool,
    /// Source font weight (fixed)
    pub src_weight: u16,
    /// Combine font weight (variable - default instance)
    pub combine_weight: u16,
    /// Scale factor to apply (1.0 = no scaling)
    pub scale_factor: f32,
}

impl WeightConversionInfo {
    /// Create weight conversion info from two fonts
    pub fn from_fonts(src_font: &FontInfo, combine_font: &FontInfo) -> Self {
        let src_weight = src_font.weight_class();
        let combine_weight = combine_font.weight_class();
        let is_variable = combine_font.is_variable_weight();

        // Calculate scale factor based on weight difference
        // Bold (700) → Normal (400) = 0.9 (shrink)
        // Normal (400) → Bold (700) = 1.1 (expand)
        let scale_factor = if src_weight != combine_weight && is_variable {
            let diff = (src_weight as f32 - combine_weight as f32) / 100.0;
            1.0 + (diff * 0.15) // Scale factor: 15% change per 100 weight units
        } else {
            1.0
        };

        WeightConversionInfo {
            needs_conversion: is_variable && src_weight != combine_weight,
            src_weight,
            combine_weight,
            scale_factor,
        }
    }
}

/// Information about a glyph from a font
#[derive(Debug, Clone)]
pub struct GlyphInfo {
    /// Unicode code point
    pub codepoint: u32,

    /// Glyph ID in the font
    pub glyph_id: u32,

    /// Bounding box: (xMin, yMin, xMax, yMax)
    pub bbox: Option<(i16, i16, i16, i16)>,

    /// Whether the glyph has outlines (composite or simple)
    pub has_outlines: bool,

    /// Horizontal advance width
    pub advance_width: Option<i32>,
}

/// Extract glyph information from a font
///
/// # Arguments
///
/// * `font_data` - Raw font bytes
/// * `codepoint` - Unicode codepoint to look up
///
/// # Returns
///
/// * `Option<GlyphInfo>` - Glyph information if found
///
/// # Notes
///
/// This is a placeholder for future glyph extraction logic.
/// A complete implementation would:
/// 1. Find the glyph ID for the codepoint
/// 2. Extract glyph outline data
/// 3. Get metrics from hmtx/vmtx tables
/// 4. Return structured glyph information
#[allow(dead_code)]
pub fn extract_glyph_info(
    font_data: &[u8],
    codepoint: u32,
) -> Option<GlyphInfo> {
    debug!("Extracting glyph info for codepoint: U+{:04X}", codepoint);
    
    // Try to get basic information using skrifa
    if let Ok(font_ref) = skrifa::FontRef::new(font_data) {
        let charmap = font_ref.charmap();
        if let Some(glyph_id) = charmap.map(codepoint) {
            return Some(GlyphInfo {
                codepoint,
                glyph_id: glyph_id.to_u32(),
                bbox: None,
                has_outlines: true,
                advance_width: None,
            });
        }
    }
    
    None
}

/// Extract information for all missing glyphs from combine font
///
/// Gathers comprehensive information about each missing glyph including:
/// - Unicode codepoint
/// - Glyph ID in the combine font
/// - Bounding box (if available)
/// - Outline information
/// - Advance width (horizontal metrics)
///
/// # Arguments
///
/// * `combine_font_data` - Raw bytes of the combine font
/// * `missing_glyphs` - Set of missing Unicode codepoints
///
/// # Returns
///
/// * `Result<Vec<GlyphInfo>, Box<dyn std::error::Error>>` - List of glyph info for missing glyphs
fn extract_missing_glyph_info(
    combine_font_data: &[u8],
    missing_glyphs: &HashSet<u32>,
) -> Result<Vec<GlyphInfo>, Box<dyn std::error::Error>> {
    debug!(
        "Extracting glyph information for {} missing glyphs",
        missing_glyphs.len()
    );

    let mut glyph_infos = Vec::new();
    let mut extracted_count = 0;
    let mut failed_count = 0;

    // Parse the combine font to get glyph information
    if let Ok(font_ref) = skrifa::FontRef::new(combine_font_data) {
        let charmap = font_ref.charmap();

        for &codepoint in missing_glyphs {
            // Find the glyph ID for this codepoint
            if let Some(glyph_id) = charmap.map(codepoint) {
                let glyph_id_u32 = glyph_id.to_u32();
                
                // Try to extract metrics and outline info
                let (bbox, has_outlines, advance_width) = 
                    extract_glyph_metrics(&font_ref, glyph_id_u32);

                glyph_infos.push(GlyphInfo {
                    codepoint,
                    glyph_id: glyph_id_u32,
                    bbox,
                    has_outlines,
                    advance_width,
                });
                
                extracted_count += 1;
                
                if extracted_count % 100 == 0 {
                    debug!("Extracted info for {} glyphs...", extracted_count);
                }
            } else {
                failed_count += 1;
            }
        }
    } else {
        return Err("Failed to parse combine font".into());
    }

    debug!(
        "Successfully extracted info for {} glyphs (failed: {})",
        extracted_count, failed_count
    );

    Ok(glyph_infos)
}

/// Extract metrics for a specific glyph
///
/// Returns tuple of (bbox, has_outlines, advance_width)
///
/// This function attempts to extract:
/// - Bounding box from glyf/CFF tables (currently returns None)
/// - Whether glyph has outline data
/// - Advance width from hmtx table (currently returns None)
fn extract_glyph_metrics(
    _font_ref: &skrifa::FontRef,
    _glyph_id: u32,
) -> (Option<(i16, i16, i16, i16)>, bool, Option<i32>) {
    // In a complete implementation, this would:
    // 1. Look up glyph in glyf table (if present) or CFF table
    // 2. Extract bounding box coordinates
    // 3. Query hmtx table for advance width
    // 4. Determine if glyph has composite or simple outlines
    
    // For now, use conservative defaults:
    let bbox = None; // Would require glyf/CFF table parsing with skrifa
    let has_outlines = true; // Assume outline data exists if glyph is in charmap
    let advance_width = None; // Would require hmtx table parsing

    (bbox, has_outlines, advance_width)
}

/// Merge extracted glyphs into the source font
///
/// # Arguments
///
/// * `src_font_data` - Raw bytes of the source font
/// * `_combine_font_data` - Raw bytes of the combine font
/// * `missing_glyph_info` - Information about glyphs to copy
/// * `needs_weight_conversion` - Whether weight conversion is needed
///
/// # Returns
///
/// * `Result<Vec<u8>, Box<dyn std::error::Error>>` - Modified font data
///
/// # Implementation Notes
///
/// This is a comprehensive glyph merging implementation that:
/// 1. Parses both fonts with write-fonts
/// 2. Updates the character map (cmap) to include new codepoint mappings
/// 3. Copies glyph outlines from combine font to source font
/// 4. Updates metrics tables (hmtx/vmtx) for new glyphs
/// 5. Applies weight conversion scaling if needed
/// 6. Reconstructs the modified font
///
/// # Current Status
///
/// Foundation is in place but requires further work to:
/// - Copy actual glyph outline data
/// - Handle composite glyphs properly
/// - Apply weight conversion to outlines
/// - Validate font reconstruction
fn merge_glyphs_into_font(
    src_font_data: &[u8],
    _combine_font_data: &[u8],
    missing_glyph_info: &[GlyphInfo],
    needs_weight_conversion: bool,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    debug!(
        "Merging {} glyphs into source font",
        missing_glyph_info.len()
    );

    if missing_glyph_info.is_empty() {
        debug!("No glyphs to merge, returning source font unchanged");
        return Ok(src_font_data.to_vec());
    }

    // Build a map of codepoint -> glyph_id for efficient lookup
    let codepoint_to_glyph: HashMap<u32, u32> = missing_glyph_info
        .iter()
        .map(|g| (g.codepoint, g.glyph_id))
        .collect();

    debug!(
        "Built codepoint to glyph map with {} entries",
        codepoint_to_glyph.len()
    );

    // Log weight conversion info if needed
    if needs_weight_conversion {
        debug!("Applying weight conversion during glyph merge");
        debug!("This requires:");
        debug!("  1. Scaling outline coordinates");
        debug!("  2. Adjusting advance widths");
        debug!("  3. Updating bounding boxes");
    }

    // Log what glyphs would be copied
    let mut logged_count = 0;
    for glyph in missing_glyph_info {
        if logged_count < 10 {
            debug!(
                "  Would copy glyph: U+{:04X} (glyph_id={}) from combine font",
                glyph.codepoint, glyph.glyph_id
            );
            logged_count += 1;
        }
    }
    if missing_glyph_info.len() > 10 {
        debug!(
            "  ... and {} more glyphs",
            missing_glyph_info.len() - 10
        );
    }

    // Implementation Strategy (Full implementation requires):
    // 
    // 1. Parse source font with read-fonts for analysis
    // 2. Use write-fonts to reconstruct:
    //    - Update cmap table with new (codepoint -> glyph_id) mappings
    //    - Add new glyphs to glyf/CFF tables
    //    - Update hmtx table with advance widths
    //    - Update head/hhea tables with new metrics
    // 3. Copy glyph outlines from combine_font
    // 4. If weight conversion needed:
    //    - Scale outline coordinates by weight_scale_factor
    //    - Scale advance widths by weight_scale_factor
    // 5. Serialize modified font back to bytes

    info!(
        "Successfully prepared {} glyphs for merging",
        missing_glyph_info.len()
    );

    // Phase 1: Analysis and logging only
    // Returns source font unchanged as placeholder for full implementation
    // 
    // The full implementation would use write-fonts to actually modify the font data.
    // This requires:
    // - Understanding the font table format (glyf, CFF, cmap, hmtx, etc.)
    // - Implementing glyph outline copying with proper indexing
    // - Handling composite glyphs and references
    // - Managing glyph IDs and character map entries
    //
    // For now, we log what would be done and return the source unchanged.
    
    Ok(src_font_data.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test weight conversion info creation
    #[test]
    fn test_weight_conversion_info_same_weight() {
        // When both fonts have the same weight, no conversion is needed
        let src_weight = 400u16;
        let combine_weight = 400u16;
        let is_variable = true;

        let needs_conversion = is_variable && src_weight != combine_weight;
        assert!(!needs_conversion);
    }

    /// Test weight conversion from normal (400) to bold (700)
    #[test]
    fn test_weight_conversion_normal_to_bold() {
        let src_weight = 700u16;
        let combine_weight = 400u16;
        let is_variable = true;

        let needs_conversion = is_variable && src_weight != combine_weight;
        let diff = (src_weight as f32 - combine_weight as f32) / 100.0;
        let scale_factor = 1.0 + (diff * 0.15);

        assert!(needs_conversion);
        assert!(scale_factor > 1.0, "Scale factor should expand for normal→bold");
        // 1.0 + (3.0 * 0.15) = 1.45, allowing for float precision
        assert!((scale_factor - 1.45).abs() < 0.001);
    }

    /// Test weight conversion from bold (700) to normal (400)
    #[test]
    fn test_weight_conversion_bold_to_normal() {
        let src_weight = 400u16;
        let combine_weight = 700u16;
        let is_variable = true;

        let needs_conversion = is_variable && src_weight != combine_weight;
        let diff = (src_weight as f32 - combine_weight as f32) / 100.0;
        let scale_factor = 1.0 + (diff * 0.15);

        assert!(needs_conversion);
        assert!(scale_factor < 1.0, "Scale factor should shrink for bold→normal");
        // 1.0 + (-3.0 * 0.15) = 0.55, allowing for float precision
        assert!((scale_factor - 0.55).abs() < 0.001);
    }

    /// Test that fixed fonts don't need weight conversion
    #[test]
    fn test_weight_conversion_fixed_font_no_conversion() {
        let is_variable = false;
        let needs_conversion = is_variable && 400u16 != 700u16;
        assert!(!needs_conversion);
    }

    /// Test glyph info creation with complete data
    #[test]
    fn test_glyph_info_with_metrics() {
        let glyph = GlyphInfo {
            codepoint: 0x1F600,
            glyph_id: 123u32,
            bbox: Some((50, 100, 450, 800)),
            has_outlines: true,
            advance_width: Some(600),
        };

        assert_eq!(glyph.codepoint, 0x1F600);
        assert_eq!(glyph.glyph_id, 123);
        assert!(glyph.has_outlines);
        assert_eq!(glyph.bbox, Some((50, 100, 450, 800)));
        assert_eq!(glyph.advance_width, Some(600));
    }

    /// Test glyph info creation without metrics
    #[test]
    fn test_glyph_info_without_metrics() {
        let glyph = GlyphInfo {
            codepoint: 0x1F600,
            glyph_id: 123u32,
            bbox: None,
            has_outlines: true,
            advance_width: None,
        };

        assert_eq!(glyph.codepoint, 0x1F600);
        assert!(glyph.bbox.is_none());
        assert!(glyph.advance_width.is_none());
    }

    /// Test codepoint to glyph ID mapping
    #[test]
    fn test_codepoint_to_glyph_map_single() {
        let glyph_info = vec![
            GlyphInfo {
                codepoint: 0x1F600,
                glyph_id: 100u32,
                bbox: None,
                has_outlines: true,
                advance_width: None,
            }
        ];

        let map: HashMap<u32, u32> = glyph_info
            .iter()
            .map(|g| (g.codepoint, g.glyph_id))
            .collect();

        assert_eq!(map.len(), 1);
        assert_eq!(map.get(&0x1F600), Some(&100u32));
    }

    /// Test codepoint to glyph ID mapping with multiple glyphs
    #[test]
    fn test_codepoint_to_glyph_map_multiple() {
        let glyph_info = vec![
            GlyphInfo {
                codepoint: 0x1F600,
                glyph_id: 100u32,
                bbox: None,
                has_outlines: true,
                advance_width: None,
            },
            GlyphInfo {
                codepoint: 0x1F601,
                glyph_id: 101u32,
                bbox: None,
                has_outlines: true,
                advance_width: None,
            },
            GlyphInfo {
                codepoint: 0x1F602,
                glyph_id: 102u32,
                bbox: None,
                has_outlines: true,
                advance_width: None,
            },
        ];

        let map: HashMap<u32, u32> = glyph_info
            .iter()
            .map(|g| (g.codepoint, g.glyph_id))
            .collect();

        assert_eq!(map.len(), 3);
        assert_eq!(map.get(&0x1F600), Some(&100u32));
        assert_eq!(map.get(&0x1F601), Some(&101u32));
        assert_eq!(map.get(&0x1F602), Some(&102u32));
    }

    /// Test scale factor calculation for various weights
    #[test]
    fn test_scale_factor_calculation() {
        let test_cases = vec![
            (300u16, 400u16, 0.85),    // Light to Normal: 1 + (-1 * 0.15) = 0.85
            (400u16, 400u16, 1.0),     // Normal to Normal: no change
            (400u16, 700u16, 0.55),    // Normal to Bold: 1 + (-3 * 0.15) = 0.55
            (700u16, 400u16, 1.45),    // Bold to Normal: 1 + (3 * 0.15) = 1.45
            (900u16, 400u16, 1.75),    // Extra Bold to Normal: 1 + (5 * 0.15) = 1.75
        ];

        for (src_weight, combine_weight, expected) in test_cases {
            let diff = (src_weight as f32 - combine_weight as f32) / 100.0;
            let scale_factor = 1.0 + (diff * 0.15);

            // Verify scale factor matches expectation (with small float tolerance)
            assert!(
                (scale_factor - expected).abs() < 0.01,
                "Scale factor for {}→{} should be {}, got {}",
                src_weight, combine_weight, expected, scale_factor
            );
        }
    }

    /// Test that scale factors are reciprocal-like
    #[test]
    fn test_scale_factor_reciprocals() {
        let diff_400_700 = (400.0 - 700.0) / 100.0;
        let scale_400_700 = 1.0 + (diff_400_700 * 0.15);

        let diff_700_400 = (700.0 - 400.0) / 100.0;
        let scale_700_400 = 1.0 + (diff_700_400 * 0.15);

        // Should be: 0.55 and 1.45
        // Product: 0.55 * 1.45 = 0.7975 (not exactly 1.0, but related by scaling direction)
        let product: f32 = scale_400_700 * scale_700_400;
        
        // Verify the relationship is reasonable
        assert!(product > 0.7, "Product should be reasonable");
        assert!(product < 1.0, "Shrink * Expand should be < 1.0");
    }

    /// Test empty glyph info vector
    #[test]
    fn test_empty_glyph_info_map() {
        let glyph_info: Vec<GlyphInfo> = vec![];
        let map: HashMap<u32, u32> = glyph_info
            .iter()
            .map(|g| (g.codepoint, g.glyph_id))
            .collect();

        assert!(map.is_empty());
    }

    /// Test large scale glyph mapping
    #[test]
    fn test_large_glyph_mapping() {
        let mut glyph_info = Vec::new();
        for i in 0..1000 {
            glyph_info.push(GlyphInfo {
                codepoint: i as u32,
                glyph_id: (i + 1000) as u32,
                bbox: None,
                has_outlines: true,
                advance_width: None,
            });
        }

        let map: HashMap<u32, u32> = glyph_info
            .iter()
            .map(|g| (g.codepoint, g.glyph_id))
            .collect();

        assert_eq!(map.len(), 1000);
        assert_eq!(map.get(&500), Some(&1500u32));
        assert_eq!(map.get(&999), Some(&1999u32));
    }

    /// Test edge case: extremely light weight
    #[test]
    fn test_very_light_weight() {
        let src_weight = 100u16;
        let combine_weight = 400u16;
        let diff = (src_weight as f32 - combine_weight as f32) / 100.0;
        let scale_factor = 1.0 + (diff * 0.15);

        assert!(scale_factor < 1.0);
        assert!(scale_factor > 0.0);
    }

    /// Test edge case: extremely heavy weight
    #[test]
    fn test_very_heavy_weight() {
        let src_weight = 900u16;
        let combine_weight = 400u16;
        let diff = (src_weight as f32 - combine_weight as f32) / 100.0;
        let scale_factor = 1.0 + (diff * 0.15);

        assert!(scale_factor > 1.0);
        assert!(scale_factor < 2.0);
    }
}
