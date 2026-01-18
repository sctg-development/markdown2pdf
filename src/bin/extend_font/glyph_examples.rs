/// Example implementation showing how to integrate glyph extraction and copying
/// into the current markdown2pdf extend_font workflow
///
/// This example demonstrates the complete workflow:
/// 1. Load fonts with both skrifa and read-fonts
/// 2. Find missing glyphs
/// 3. Extract glyph information
/// 4. Apply weight transformations
/// 5. Prepare for merging (or merge with write-fonts when available)
use log::{debug, info, warn};
use read_fonts::TableProvider;
use skrifa::{FontRef as SkrifaFontRef, GlyphId, MetadataProvider};
use std::collections::HashMap;

/// Complete workflow example: extract and prepare glyphs for copying
pub fn example_complete_glyph_extraction(
    src_font_data: &[u8],
    combine_font_data: &[u8],
) -> Result<GlyphExtractionSummary, Box<dyn std::error::Error>> {
    info!("=== EXAMPLE: Complete Glyph Extraction Workflow ===\n");

    // STEP 1: Parse fonts with both libraries
    let skrifa_src = SkrifaFontRef::new(src_font_data)?;
    let skrifa_combine = SkrifaFontRef::new(combine_font_data)?;

    let read_src = read_fonts::FontRef::new(src_font_data)?;
    let read_combine = read_fonts::FontRef::new(combine_font_data)?;

    info!("✓ Fonts parsed successfully");

    // STEP 2: Get basic metrics
    let src_maxp = read_src.maxp()?;
    let combine_maxp = read_combine.maxp()?;

    info!("Source font: {} glyphs", src_maxp.num_glyphs());
    info!("Combine font: {} glyphs", combine_maxp.num_glyphs());

    // STEP 3: Find missing codepoints
    let src_charmap = skrifa_src.charmap();
    let combine_charmap = skrifa_combine.charmap();

    let src_codepoints: HashMap<u32, GlyphId> = src_charmap.mappings().collect();
    let combine_codepoints: HashMap<u32, GlyphId> = combine_charmap.mappings().collect();

    let missing_codepoints: Vec<u32> = combine_codepoints
        .keys()
        .filter(|cp| !src_codepoints.contains_key(cp))
        .copied()
        .collect();

    info!("\nFound {} missing codepoints", missing_codepoints.len());

    // STEP 4: Extract information for first 5 missing glyphs as example
    let mut extracted_glyphs = Vec::new();
    let sample_size = missing_codepoints.len().min(5);

    info!("\nExtracting details for {} example glyphs...", sample_size);

    for &codepoint in missing_codepoints.iter().take(sample_size) {
        if let Some(glyph_id) = combine_codepoints.get(&codepoint) {
            // Get metrics
            let hmtx = read_combine.hmtx()?;
            let metrics = hmtx.metrics(*glyph_id).ok();

            // Get bbox
            let glyf = read_combine.glyf();
            let bbox = if let Ok(glyf_table) = glyf {
                glyf_table
                    .get(*glyph_id)
                    .ok()
                    .flatten()
                    .and_then(|g| g.bounding_box().ok())
                    .map(|bb| (bb.x_min, bb.y_min, bb.x_max, bb.y_max))
            } else {
                None
            };

            // Analyze structure (simple vs composite)
            let is_composite = if let Ok(glyf_table) = glyf {
                glyf_table
                    .get(*glyph_id)
                    .ok()
                    .flatten()
                    .map(|g| g.is_composite())
                    .unwrap_or(false)
            } else {
                false
            };

            let extraction = ExtractedGlyphInfo {
                codepoint,
                glyph_id: *glyph_id,
                advance_width: metrics.map(|m| m.advance_width),
                lsb: metrics.map(|m| m.left_side_bearing),
                bbox,
                is_composite,
            };

            extracted_glyphs.push(extraction);
        }
    }

    // Print extracted details
    for glyph in &extracted_glyphs {
        let char_repr = char::from_u32(glyph.codepoint)
            .map(|c| format!("'{}'", c))
            .unwrap_or_else(|| "?".to_string());

        info!(
            "\n  U+{:04X} {}: glyph_id={:?}",
            glyph.codepoint, char_repr, glyph.glyph_id
        );

        if let Some(adv) = glyph.advance_width {
            info!("    - Advance width: {}", adv);
        }

        if let Some((x_min, y_min, x_max, y_max)) = glyph.bbox {
            info!(
                "    - BBox: ({}, {}) to ({}, {})",
                x_min, y_min, x_max, y_max
            );
        }

        info!("    - Composite: {}", glyph.is_composite);
    }

    // STEP 5: Demonstrate weight transformation
    info!("\n=== Weight Transformation Example ===");

    // Get weight classes
    if let (Ok(src_os2), Ok(combine_os2)) = (read_src.os_2(), read_combine.os_2()) {
        let src_weight = src_os2.us_weight_class();
        let combine_weight = combine_os2.us_weight_class();

        info!("Source weight: {}", src_weight);
        info!("Combine weight: {}", combine_weight);

        if src_weight != combine_weight {
            let scale_factor = calculate_weight_scale(combine_weight as u16, src_weight as u16);
            info!("Scale factor needed: {:.3}x", scale_factor);

            if let Some(first_glyph) = extracted_glyphs.first_mut() {
                info!(
                    "Example: applying {:.3}x scale to U+{:04X}",
                    scale_factor, first_glyph.codepoint
                );

                if let Some((x_min, y_min, x_max, y_max)) = first_glyph.bbox {
                    let scaled = (
                        (x_min as f32 * scale_factor) as i16,
                        (y_min as f32 * scale_factor) as i16,
                        (x_max as f32 * scale_factor) as i16,
                        (y_max as f32 * scale_factor) as i16,
                    );
                    info!("  Original BBox: {:?}", first_glyph.bbox);
                    info!("  Scaled BBox: {:?}", scaled);
                }

                if let Some(adv) = first_glyph.advance_width {
                    let scaled_adv = (adv as f32 * scale_factor) as u16;
                    info!("  Original advance width: {}", adv);
                    info!("  Scaled advance width: {}", scaled_adv);
                }
            }
        }
    }

    // STEP 6: Demonstrate glyph outline access (for TrueType)
    info!("\n=== Glyph Outline Access Example ===");

    if let Some(first_glyph) = extracted_glyphs.first() {
        if let Ok(glyf) = read_combine.glyf() {
            if let Ok(Some(glyph)) = glyf.get(first_glyph.glyph_id) {
                if let Some(simple) = glyph.simple() {
                    let num_contours = simple.contours().count();
                    info!(
                        "Glyph U+{:04X} has {} contours",
                        first_glyph.codepoint, num_contours
                    );

                    let mut total_points = 0;
                    for (i, contour) in simple.contours().enumerate() {
                        let points: Vec<_> = contour
                            .points()
                            .collect::<Result<Vec<_>, _>>()
                            .unwrap_or_default();
                        let flags: Vec<_> = contour
                            .flags()
                            .collect::<Result<Vec<_>, _>>()
                            .unwrap_or_default();

                        total_points += points.len();

                        if i < 2 {
                            // Show first 2 contours only
                            let on_curve_count = flags.iter().filter(|f| f.on_curve()).count();
                            info!(
                                "  Contour {}: {} points ({} on-curve, {} off-curve)",
                                i,
                                points.len(),
                                on_curve_count,
                                points.len() - on_curve_count
                            );
                        }
                    }

                    info!("  Total points across all contours: {}", total_points);
                } else if glyph.is_composite() {
                    if let Some(composite) = glyph.composite() {
                        let num_components = composite.components().count();
                        info!(
                            "Glyph U+{:04X} is COMPOSITE with {} components",
                            first_glyph.codepoint, num_components
                        );

                        for (i, comp) in composite.components().enumerate() {
                            if let Ok(component) = comp {
                                info!("  Component {}: glyph_id={:?}", i, component.glyph_index);
                            }
                        }
                    }
                }
            }
        }
    }

    // STEP 7: Summary
    info!("\n=== Summary ===");
    info!("Missing codepoints: {}", missing_codepoints.len());
    info!("Successfully extracted: {}", extracted_glyphs.len());

    let composite_count = extracted_glyphs.iter().filter(|g| g.is_composite).count();
    info!(
        "  - Simple glyphs: {}",
        extracted_glyphs.len() - composite_count
    );
    info!("  - Composite glyphs: {}", composite_count);

    Ok(GlyphExtractionSummary {
        total_missing: missing_codepoints.len(),
        extracted: extracted_glyphs,
        requires_weight_transform: true,  // Would check actual weights
        recommend_strategy: "Read extracted glyphs with read-fonts, transform with custom code, merge using write-fonts when API available".to_string(),
    })
}

/// Summary of extraction results
#[derive(Debug)]
pub struct GlyphExtractionSummary {
    pub total_missing: usize,
    pub extracted: Vec<ExtractedGlyphInfo>,
    pub requires_weight_transform: bool,
    pub recommend_strategy: String,
}

/// Information about an extracted glyph
#[derive(Debug, Clone)]
pub struct ExtractedGlyphInfo {
    pub codepoint: u32,
    pub glyph_id: GlyphId,
    pub advance_width: Option<u16>,
    pub lsb: Option<i16>,
    pub bbox: Option<(i16, i16, i16, i16)>,
    pub is_composite: bool,
}

/// Calculate weight scale factor
fn calculate_weight_scale(from_weight: u16, to_weight: u16) -> f32 {
    let weight_diff = (to_weight as i32 - from_weight as i32) as f32 / 100.0;
    1.0 + (weight_diff * 0.15)
}

/// Example: Detailed glyph outline inspection
pub fn example_inspect_glyph_outline(
    font_data: &[u8],
    codepoint: u32,
) -> Result<String, Box<dyn std::error::Error>> {
    let font = SkrifaFontRef::new(font_data)?;
    let read_font = read_fonts::FontRef::new(font_data)?;

    let glyph_id = font
        .charmap()
        .map(codepoint)
        .ok_or(format!("Codepoint not found: U+{:04X}", codepoint))?;

    let mut report = String::new();

    report.push_str(&format!("=== Glyph Outline Inspection ===\n"));
    report.push_str(&format!("Codepoint: U+{:04X}\n", codepoint));
    report.push_str(&format!("Glyph ID: {:?}\n\n", glyph_id));

    // Metrics
    let hmtx = read_font.hmtx()?;
    let metrics = hmtx.metrics(glyph_id)?;
    report.push_str(&format!("Metrics:\n"));
    report.push_str(&format!("  Advance width: {}\n", metrics.advance_width));
    report.push_str(&format!(
        "  Left side bearing: {}\n\n",
        metrics.left_side_bearing
    ));

    // Outline structure
    let glyf = read_font.glyf()?;
    if let Ok(Some(glyph)) = glyf.get(glyph_id) {
        if glyph.is_composite() {
            report.push_str("Structure: COMPOSITE GLYPH\n");

            if let Some(composite) = glyph.composite() {
                for (i, comp_result) in composite.components().enumerate() {
                    if let Ok(component) = comp_result {
                        report.push_str(&format!("  Component {}:\n", i));
                        report.push_str(&format!("    Glyph ID: {:?}\n", component.glyph_index));

                        match component.transform {
                            read_fonts::tables::glyf::Transform::Offset { x, y } => {
                                report.push_str(&format!("    Transform: offset({}, {})\n", x, y));
                            }
                            read_fonts::tables::glyf::Transform::Scaled { x, y } => {
                                report.push_str(&format!("    Transform: scale({}, {})\n", x, y));
                            }
                            read_fonts::tables::glyf::Transform::TwoByTwo { xx, xy, yx, yy } => {
                                report.push_str(&format!(
                                    "    Transform: matrix([{}, {}; {}, {}])\n",
                                    xx, xy, yx, yy
                                ));
                            }
                        }
                    }
                }
            }
        } else {
            report.push_str("Structure: SIMPLE GLYPH\n");

            if let Some(simple) = glyph.simple() {
                let contours_count = simple.contours().count();
                report.push_str(&format!("  Contours: {}\n", contours_count));

                let mut total_points = 0;
                for (i, contour) in simple.contours().enumerate() {
                    let points: Vec<_> = contour.points().collect::<Result<Vec<_>, _>>()?;
                    let flags: Vec<_> = contour.flags().collect::<Result<Vec<_>, _>>()?;

                    total_points += points.len();

                    let on_curve = flags.iter().filter(|f| f.on_curve()).count();
                    report.push_str(&format!(
                        "    Contour {}: {} points ({} on-curve)\n",
                        i,
                        points.len(),
                        on_curve
                    ));
                }

                report.push_str(&format!("  Total points: {}\n", total_points));
            }
        }
    }

    Ok(report)
}

/// Example: Create transformation matrix for weight adjustment
pub fn example_create_weight_transform(src_weight: u16, dst_weight: u16) -> [f32; 6] {
    let scale = calculate_weight_scale(src_weight, dst_weight);

    // 2D affine transformation matrix: [a, b, c, d, e, f]
    // Where: new_x = a*x + c*y + e
    //        new_y = b*x + d*y + f
    // For uniform scaling with no translation:
    [scale, 0.0, 0.0, scale, 0.0, 0.0]
}

/// Example: Copy glyph outline points with transformation
pub fn example_transform_glyph_points(
    font_data: &[u8],
    codepoint: u32,
    transform: [f32; 6],
) -> Result<Vec<(i32, i32, bool)>, Box<dyn std::error::Error>> {
    let font = SkrifaFontRef::new(font_data)?;
    let read_font = read_fonts::FontRef::new(font_data)?;

    let glyph_id = font.charmap().map(codepoint).ok_or("Codepoint not found")?;

    let glyf = read_font.glyf()?;
    let glyph = glyf.get(glyph_id)?.ok_or("Glyph not found")?;

    let mut transformed_points = Vec::new();

    if let Some(simple) = glyph.simple() {
        for contour in simple.contours() {
            let points: Vec<_> = contour.points().collect::<Result<Vec<_>, _>>()?;
            let flags: Vec<_> = contour.flags().collect::<Result<Vec<_>, _>>()?;

            for (point, flag) in points.iter().zip(flags.iter()) {
                let [a, b, c, d, e, f] = transform;

                let new_x = (a * point.x as f32 + c * point.y as f32 + e) as i32;
                let new_y = (b * point.x as f32 + d * point.y as f32 + f) as i32;

                transformed_points.push((new_x, new_y, flag.on_curve()));
            }
        }
    }

    Ok(transformed_points)
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
    }

    #[test]
    fn test_create_weight_transform() {
        let transform = example_create_weight_transform(400, 700);
        assert!(transform[0] > 1.0); // scale_x
        assert!(transform[3] > 1.0); // scale_y
        assert_eq!(transform[1], 0.0); // no rotation
        assert_eq!(transform[2], 0.0); // no rotation
    }
}
