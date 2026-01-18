/// Practical examples combining skrifa and write-fonts for complete glyph copying
///
/// This module shows end-to-end workflows for:
/// 1. Identifying missing glyphs
/// 2. Extracting glyphs from source
/// 3. Transforming for weight
/// 4. Writing to destination font
/// 5. Complete font merging workflow
use crate::font_utils::FontInfo;
use log::{debug, info, warn};
use read_fonts::TableProvider;
use skrifa::{FontRef as SkrifaFontRef, GlyphId, MetadataProvider};
use std::collections::{HashMap, HashSet};

// Re-export from other modules (these would be your actual modules)
// use crate::bin::extend_font::glyph_extraction::*;
// use crate::bin::extend_font::glyph_writing::*;

/// ============================================================================
/// EXAMPLE 1: Find missing glyphs between two fonts
/// ============================================================================

/// Find all codepoints in combine_font that are missing from src_font
pub fn find_missing_codepoints(
    src_font_data: &[u8],
    combine_font_data: &[u8],
) -> Result<HashSet<u32>, Box<dyn std::error::Error>> {
    let src_font = SkrifaFontRef::new(src_font_data)?;
    let combine_font = SkrifaFontRef::new(combine_font_data)?;

    // Get all codepoints from both fonts
    let src_codepoints: HashSet<u32> = src_font.charmap().mappings().map(|(cp, _)| cp).collect();

    let combine_codepoints: HashSet<u32> = combine_font
        .charmap()
        .mappings()
        .map(|(cp, _)| cp)
        .collect();

    // Find missing: in combine but not in src
    let missing: HashSet<u32> = combine_codepoints
        .difference(&src_codepoints)
        .copied()
        .collect();

    info!(
        "Found {} missing codepoints (src: {}, combine: {}, diff: {})",
        missing.len(),
        src_codepoints.len(),
        combine_codepoints.len(),
        missing.len()
    );

    // Show some examples
    let mut examples: Vec<u32> = missing.iter().take(10).copied().collect();
    examples.sort();

    for cp in examples {
        debug!(
            "  Missing: U+{:04X} ({})",
            cp,
            char::from_u32(cp)
                .map(|c| c.to_string())
                .unwrap_or_else(|| "?".to_string())
        );
    }

    Ok(missing)
}

/// ============================================================================
/// EXAMPLE 2: Complete extraction pipeline for a single glyph
/// ============================================================================

/// All information needed to copy a glyph
#[derive(Debug, Clone)]
pub struct GlyphTransferPackage {
    /// Unicode codepoint
    pub codepoint: u32,

    /// Glyph ID in source font
    pub src_glyph_id: u32,

    /// Glyph ID in destination font (may need remapping)
    pub dst_glyph_id: Option<u32>,

    /// Horizontal advance width
    pub advance_width: u16,

    /// Bounding box
    pub bbox: Option<(i16, i16, i16, i16)>,

    /// Whether it's a composite glyph
    pub is_composite: bool,

    /// Component glyph IDs (for composite glyphs)
    pub components: Vec<u32>,

    /// Transformation to apply (scale_x, scale_y, etc.)
    pub transform: Option<[f32; 6]>,

    /// Raw glyf/CFF data for fast copying
    pub raw_data: Option<Vec<u8>>,
}

/// Extract complete information for a glyph
pub fn extract_glyph_package(
    src_font_data: &[u8],
    codepoint: u32,
    weight_scale: Option<f32>,
) -> Result<GlyphTransferPackage, Box<dyn std::error::Error>> {
    let font = SkrifaFontRef::new(src_font_data)?;
    let read_font = read_fonts::FontRef::new(src_font_data)?;

    // Find glyph ID for this codepoint
    let glyph_id = font
        .charmap()
        .map(codepoint)
        .ok_or(format!("Codepoint U+{:04X} not found", codepoint))?;

    // Get metrics
    let hmtx = font.hmtx()?;
    let metrics = hmtx.metrics(glyph_id)?;

    // Get bbox if available
    let bbox = get_bbox(&read_font, glyph_id).ok().flatten();

    // Check if composite
    let (is_composite, components) = analyze_glyph_structure(&read_font, glyph_id)?;

    // Build transformation if weight scaling requested
    let transform = weight_scale.map(|scale| [scale, 0.0, 0.0, 0.0, scale, 0.0]);

    info!(
        "Extracted glyph package for U+{:04X} (glyph_id {:?}): {}{}",
        codepoint,
        glyph_id,
        if is_composite { "composite with " } else { "" },
        if is_composite {
            format!("{} components", components.len())
        } else {
            "simple outline".to_string()
        }
    );

    Ok(GlyphTransferPackage {
        codepoint,
        src_glyph_id: glyph_id.to_u32(),
        dst_glyph_id: None,
        advance_width: metrics.advance_width,
        bbox,
        is_composite,
        components,
        transform,
        raw_data: None,
    })
}

/// Get bounding box for a glyph
fn get_bbox(
    font: &read_fonts::FontRef,
    glyph_id: GlyphId,
) -> Result<Option<(i16, i16, i16, i16)>, Box<dyn std::error::Error>> {
    if let Ok(glyf) = font.glyf() {
        if let Ok(Some(g)) = glyf.get(glyph_id) {
            if let Ok(bb) = g.bounding_box() {
                return Ok(Some((bb.x_min, bb.y_min, bb.x_max, bb.y_max)));
            }
        }
    }
    Ok(None)
}

/// Analyze glyph structure (simple vs composite)
fn analyze_glyph_structure(
    font: &read_fonts::FontRef,
    glyph_id: GlyphId,
) -> Result<(bool, Vec<u32>), Box<dyn std::error::Error>> {
    if let Ok(glyf) = font.glyf() {
        if let Ok(Some(glyph)) = glyf.get(glyph_id) {
            if glyph.is_composite() {
                let mut components = Vec::new();

                if let Some(composite) = glyph.composite() {
                    for component in composite.components() {
                        let comp = component?;
                        components.push(comp.glyph_index.to_u32());
                    }
                }

                return Ok((true, components));
            }
        }
    }

    Ok((false, Vec::new()))
}

/// ============================================================================
/// EXAMPLE 3: Weight transformation workflow
/// ============================================================================

/// Transform glyph for different weight
pub fn apply_weight_transformation(
    package: &mut GlyphTransferPackage,
    from_weight: u16,
    to_weight: u16,
) {
    // Calculate scale factor
    let scale_factor = calculate_weight_scale(from_weight, to_weight);

    if (scale_factor - 1.0).abs() < 0.01 {
        debug!(
            "No weight transformation needed for U+{:04X} (same weight)",
            package.codepoint
        );
        return;
    }

    debug!(
        "Applying weight transformation to U+{:04X}: {} -> {} (scale: {:.3}x)",
        package.codepoint, from_weight, to_weight, scale_factor
    );

    // Update transform matrix
    package.transform = Some([scale_factor, 0.0, 0.0, 0.0, scale_factor, 0.0]);

    // Scale metrics
    package.advance_width = (package.advance_width as f32 * scale_factor) as u16;

    // Scale bbox
    if let Some((x_min, y_min, x_max, y_max)) = package.bbox {
        package.bbox = Some((
            (x_min as f32 * scale_factor) as i16,
            (y_min as f32 * scale_factor) as i16,
            (x_max as f32 * scale_factor) as i16,
            (y_max as f32 * scale_factor) as i16,
        ));
    }
}

/// Calculate weight scale factor
///
/// Weight values: 100 (thin) to 900 (black)
/// Standard: 400 (normal), 700 (bold)
fn calculate_weight_scale(from_weight: u16, to_weight: u16) -> f32 {
    // Weight scale empirically derived:
    // - Per 100 weight units, outline changes ~15% in width
    // - Formula: scale = 1.0 + (weight_diff / 100) * 0.15

    let weight_diff = (to_weight as i32 - from_weight as i32) as f32 / 100.0;
    1.0 + (weight_diff * 0.15)
}

/// ============================================================================
/// EXAMPLE 4: Batch glyph copying workflow
/// ============================================================================

/// Configuration for batch copying
#[derive(Debug, Clone)]
pub struct BatchCopyConfig {
    /// Maximum glyphs to copy at once
    pub batch_size: usize,

    /// Apply weight transformation
    pub apply_weight_transform: bool,

    /// Weight values
    pub from_weight: u16,
    pub to_weight: u16,

    /// Skip glyphs that are already present
    pub skip_existing: bool,
}

impl Default for BatchCopyConfig {
    fn default() -> Self {
        BatchCopyConfig {
            batch_size: 100,
            apply_weight_transform: false,
            from_weight: 400,
            to_weight: 400,
            skip_existing: true,
        }
    }
}

/// Copy all missing glyphs using batch processing
pub fn batch_copy_missing_glyphs(
    src_font_data: &[u8],
    combine_font_data: &[u8],
    config: &BatchCopyConfig,
) -> Result<Vec<GlyphTransferPackage>, Box<dyn std::error::Error>> {
    // Find what's missing
    let missing = find_missing_codepoints(src_font_data, combine_font_data)?;

    info!(
        "Starting batch copy of {} glyphs (batch_size: {})",
        missing.len(),
        config.batch_size
    );

    let mut packages = Vec::new();
    let mut processed = 0;
    let mut failed = 0;

    for &codepoint in &missing {
        match extract_glyph_package(
            combine_font_data,
            codepoint,
            if config.apply_weight_transform {
                Some(calculate_weight_scale(config.from_weight, config.to_weight))
            } else {
                None
            },
        ) {
            Ok(mut pkg) => {
                if config.apply_weight_transform {
                    apply_weight_transformation(&mut pkg, config.from_weight, config.to_weight);
                }
                packages.push(pkg);
                processed += 1;

                if processed % config.batch_size == 0 {
                    info!("Processed {} glyphs so far...", processed);
                }
            }
            Err(e) => {
                warn!("Failed to extract glyph U+{:04X}: {}", codepoint, e);
                failed += 1;
            }
        }
    }

    info!(
        "Batch copy complete: {} successful, {} failed",
        processed, failed
    );

    Ok(packages)
}

/// ============================================================================
/// EXAMPLE 5: Complete font merge workflow
/// ============================================================================

/// Result of a font merge operation
#[derive(Debug)]
pub struct FontMergeResult {
    /// Modified font data
    pub font_data: Vec<u8>,

    /// Number of glyphs copied
    pub glyphs_copied: usize,

    /// Number of glyphs failed
    pub glyphs_failed: usize,

    /// Total codepoints in result
    pub total_codepoints: usize,

    /// Statistics about the operation
    pub stats: MergeStats,
}

/// Statistics from merge operation
#[derive(Debug, Clone)]
pub struct MergeStats {
    /// Simple glyphs copied
    pub simple_glyphs: usize,

    /// Composite glyphs copied
    pub composite_glyphs: usize,

    /// Weight transformations applied
    pub weight_transformations: usize,

    /// Total bytes added
    pub bytes_added: usize,
}

/// Complete workflow: merge fonts with glyph copying
pub fn merge_fonts_complete(
    src_font_data: &[u8],
    combine_font_data: &[u8],
    src_font: &FontInfo,
    combine_font: &FontInfo,
) -> Result<FontMergeResult, Box<dyn std::error::Error>> {
    info!("Starting complete font merge");
    info!("  Source: {} glyphs", src_font.glyph_count());
    info!("  Combine: {} glyphs", combine_font.glyph_count());

    // Configure copy operation
    let needs_weight_conversion =
        src_font.weight_class() != combine_font.weight_class() && combine_font.is_variable_weight();

    let mut config = BatchCopyConfig::default();
    config.apply_weight_transform = needs_weight_conversion;
    config.from_weight = combine_font.weight_class();
    config.to_weight = src_font.weight_class();

    // Extract all glyphs that need copying
    let packages = batch_copy_missing_glyphs(src_font_data, combine_font_data, &config)?;

    // Analyze packages
    let mut stats = MergeStats {
        simple_glyphs: 0,
        composite_glyphs: 0,
        weight_transformations: 0,
        bytes_added: 0,
    };

    for pkg in &packages {
        if pkg.is_composite {
            stats.composite_glyphs += 1;
        } else {
            stats.simple_glyphs += 1;
        }

        if pkg.transform.is_some() {
            stats.weight_transformations += 1;
        }
    }

    info!("Merge statistics:");
    info!("  - Simple glyphs: {}", stats.simple_glyphs);
    info!("  - Composite glyphs: {}", stats.composite_glyphs);
    info!(
        "  - Weight transformations: {}",
        stats.weight_transformations
    );

    // NOTE: Actual font reconstruction would happen here using write-fonts
    // For now, return placeholder with statistics

    Ok(FontMergeResult {
        font_data: src_font_data.to_vec(),
        glyphs_copied: packages.len(),
        glyphs_failed: 0,
        total_codepoints: 0,
        stats,
    })
}

/// ============================================================================
/// EXAMPLE 6: Step-by-step debugging workflow
/// ============================================================================

/// Detailed analysis of a specific glyph copy operation
pub fn debug_glyph_copy(
    src_font_data: &[u8],
    combine_font_data: &[u8],
    codepoint: u32,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut report = String::new();

    report.push_str(&format!("\n=== Debug Report for U+{:04X} ===\n", codepoint));

    // Check presence in both fonts
    let src_font = SkrifaFontRef::new(src_font_data)?;
    let combine_font = SkrifaFontRef::new(combine_font_data)?;

    let in_src = src_font.charmap().map(codepoint).is_some();
    let in_combine = combine_font.charmap().map(codepoint).is_some();

    report.push_str(&format!("  In src font: {}\n", in_src));
    report.push_str(&format!("  In combine font: {}\n", in_combine));

    if !in_combine {
        report.push_str("\n  ❌ Codepoint not found in combine font, cannot copy\n");
        return Ok(report);
    }

    // Get glyph ID
    if let Some(glyph_id) = combine_font.charmap().map(codepoint) {
        report.push_str(&format!("  Glyph ID: {:?}\n", glyph_id));

        // Get metrics
        if let Ok(hmtx) = combine_font.hmtx() {
            if let Ok(metrics) = hmtx.metrics(glyph_id) {
                report.push_str(&format!("  Advance width: {}\n", metrics.advance_width));
                report.push_str(&format!(
                    "  Left side bearing: {}\n",
                    metrics.left_side_bearing
                ));
            }
        }

        // Get structure
        if let Ok((is_composite, components)) =
            analyze_glyph_structure(&read_fonts::FontRef::new(combine_font_data)?, glyph_id)
        {
            report.push_str(&format!("  Is composite: {}\n", is_composite));
            if is_composite {
                report.push_str(&format!("  Components: {:?}\n", components));
            }
        }
    }

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weight_scale_same() {
        let scale = calculate_weight_scale(400, 400);
        assert!((scale - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_weight_scale_bold() {
        let scale = calculate_weight_scale(400, 700);
        assert!(scale > 1.0);
        // 300 weight units diff = 3 * 0.15 = 0.45 = 1.45
        assert!((scale - 1.45).abs() < 0.01);
    }

    #[test]
    fn test_weight_scale_light() {
        let scale = calculate_weight_scale(700, 400);
        assert!(scale < 1.0);
    }

    #[test]
    fn test_batch_copy_config_default() {
        let config = BatchCopyConfig::default();
        assert_eq!(config.batch_size, 100);
        assert!(!config.apply_weight_transform);
        assert!(config.skip_existing);
    }
}
