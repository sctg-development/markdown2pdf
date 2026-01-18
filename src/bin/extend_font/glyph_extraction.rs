use log::{debug, warn};
/// Glyph extraction and manipulation using skrifa 0.40.0
///
/// This module demonstrates:
/// 1. Extracting glyph outlines from TrueType (glyf) and PostScript (CFF) tables
/// 2. Accessing glyph metrics (advance width, bounds)
/// 3. Transforming glyphs (scaling for weight adjustment)
/// 4. Preparing glyphs for copying to another font
use read_fonts::{tables::glyf, FontRef as ReadFontRef, TableProvider};
use skrifa::{
    outline::{DrawError, Outline, OutlineGlyph},
    FontRef as SkrifaFontRef, GlyphId, MetadataProvider,
};
use std::collections::HashMap;

/// Represents a glyph that has been extracted and can be transformed
#[derive(Debug, Clone)]
pub struct ExtractedGlyph {
    /// Glyph ID
    pub glyph_id: GlyphId,

    /// Unicode codepoint this glyph represents
    pub codepoint: u32,

    /// Horizontal advance width
    pub advance_width: i32,

    /// Vertical advance width (if applicable)
    pub vertical_advance: Option<i32>,

    /// Bounding box: (xMin, yMin, xMax, yMax)
    pub bbox: Option<(i16, i16, i16, i16)>,

    /// Raw outline points and contours
    pub outline_data: GlyphOutlineData,

    /// Is this a composite glyph?
    pub is_composite: bool,

    /// Raw glyf/CFF data for direct copying (optional)
    pub raw_glyph_bytes: Option<Vec<u8>>,
}

/// Different types of outline data that skrifa can extract
#[derive(Debug, Clone)]
pub enum GlyphOutlineData {
    /// TrueType simple glyph with points
    TrueType(TrueTypeOutline),

    /// PostScript CFF outlines
    PostScript(CffOutline),

    /// Composite glyph (references other glyphs)
    Composite(Vec<CompositeGlyphComponent>),

    /// Empty glyph (space, etc.)
    Empty,
}

/// TrueType outline representation
#[derive(Debug, Clone)]
pub struct TrueTypeOutline {
    /// Contours (each contour is a list of point indices)
    pub contours: Vec<Vec<OutlinePoint>>,

    /// All points in order
    pub points: Vec<OutlinePoint>,
}

/// A point in a glyph outline
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OutlinePoint {
    /// X coordinate
    pub x: i32,

    /// Y coordinate
    pub y: i32,

    /// Is this an on-curve point? (false = off-curve/control point)
    pub on_curve: bool,
}

/// CFF outline representation
#[derive(Debug, Clone)]
pub struct CffOutline {
    /// Raw CFF charstring data
    pub charstring_data: Vec<u8>,

    /// Whether this is a serifed font (for weight adjustment)
    pub has_serifs: bool,
}

/// A component of a composite glyph
#[derive(Debug, Clone)]
pub struct CompositeGlyphComponent {
    /// Glyph ID of the referenced glyph
    pub glyph_id: GlyphId,

    /// 2x2 transformation matrix + translation
    /// [a, b, c, d, e, f] representing:
    /// | a  b  e |
    /// | c  d  f |
    /// | 0  0  1 |
    pub transform: [f32; 6],
}

/// ============================================================================
/// PATTERN 1: Extract glyph from a font file
/// ============================================================================
pub fn extract_single_glyph(
    font_data: &[u8],
    glyph_id: GlyphId,
    codepoint: u32,
) -> Result<ExtractedGlyph, Box<dyn std::error::Error>> {
    debug!("Extracting glyph ID {:?} (U+{:04X})", glyph_id, codepoint);

    // Parse the font with both skrifa and read-fonts
    let skrifa_font = SkrifaFontRef::new(font_data)?;
    let read_font = ReadFontRef::new(font_data)?;

    // Get metrics from hmtx table
    let (advance_width, vertical_advance) = get_glyph_metrics(&skrifa_font, glyph_id)?;

    // Get bounding box from glyf/CFF
    let bbox = get_glyph_bbox(&read_font, glyph_id)?;

    // Extract outline based on font type
    let outline_data = extract_outline(&skrifa_font, &read_font, glyph_id)?;

    let is_composite = matches!(outline_data, GlyphOutlineData::Composite(_));

    Ok(ExtractedGlyph {
        glyph_id,
        codepoint,
        advance_width: advance_width as i32,
        vertical_advance,
        bbox,
        outline_data,
        is_composite,
        raw_glyph_bytes: None,
    })
}

/// ============================================================================
/// PATTERN 2: Extract glyph outline points (for manipulation/transformation)
/// ============================================================================
pub fn extract_outline(
    skrifa_font: &SkrifaFontRef,
    read_font: &ReadFontRef,
    glyph_id: GlyphId,
) -> Result<GlyphOutlineData, Box<dyn std::error::Error>> {
    // Try TrueType first (glyf table)
    if let Ok(glyf_table) = read_font.glyf() {
        debug!("Font has glyf table, extracting TrueType outline");

        // Get the glyph from glyf table
        let glyph = glyf_table.get(glyph_id);

        match glyph {
            Ok(Some(glyph_data)) => {
                // Check if it's a composite glyph
                if glyph_data.is_composite() {
                    debug!("Glyph {:?} is composite", glyph_id);
                    return extract_composite_glyph(glyph_id, glyph_data);
                }

                // Simple glyph - extract points
                debug!("Glyph {:?} is simple, extracting points", glyph_id);
                return extract_truetype_points(glyph_id, glyph_data);
            }
            Ok(None) => {
                debug!("Glyph {:?} is empty (no glyph data)", glyph_id);
                return Ok(GlyphOutlineData::Empty);
            }
            Err(e) => {
                warn!("Error reading glyf data for glyph {:?}: {}", glyph_id, e);
            }
        }
    }

    // Try CFF (PostScript outlines)
    if let Ok(_cff) = read_font.cff() {
        debug!("Font has CFF table, extracting PostScript outline");
        return Ok(GlyphOutlineData::PostScript(CffOutline {
            charstring_data: vec![], // Would need proper CFF parsing
            has_serifs: false,
        }));
    }

    Ok(GlyphOutlineData::Empty)
}

/// Extract points from a TrueType glyph
fn extract_truetype_points(
    glyph_id: GlyphId,
    glyph: glyf::Glyph,
) -> Result<GlyphOutlineData, Box<dyn std::error::Error>> {
    let mut contours = Vec::new();
    let mut points = Vec::new();

    // Iterate through the glyph's contours
    if let Some(simple) = glyph.simple() {
        for contour in simple.contours() {
            let mut contour_points = Vec::new();
            let contour_start = points.len();

            // Get all flags and coordinates for this contour
            let flags: Vec<_> = contour.flags().collect::<Result<Vec<_>, _>>()?;
            let coords: Vec<_> = contour.points().collect::<Result<Vec<_>, _>>()?;

            for (flag, point) in flags.iter().zip(coords.iter()) {
                let outline_point = OutlinePoint {
                    x: point.x,
                    y: point.y,
                    on_curve: flag.on_curve(),
                };
                points.push(outline_point);
                contour_points.push(outline_point);
            }

            if !contour_points.is_empty() {
                contours.push(contour_points);
            }

            debug!(
                "Glyph {:?}: contour has {} points (indices {}..{})",
                glyph_id,
                contour_start,
                contour_start,
                points.len() - 1
            );
        }
    }

    debug!(
        "Glyph {:?}: extracted {} contours with {} total points",
        glyph_id,
        contours.len(),
        points.len()
    );

    Ok(GlyphOutlineData::TrueType(TrueTypeOutline {
        contours,
        points,
    }))
}

/// Extract composite glyph components
fn extract_composite_glyph(
    glyph_id: GlyphId,
    glyph: glyf::Glyph,
) -> Result<GlyphOutlineData, Box<dyn std::error::Error>> {
    let mut components = Vec::new();

    if let Some(composite) = glyph.composite() {
        for component in composite.components() {
            let component_data = component?;

            // Get the transformation matrix
            let (scale_x, scale_y, rotate_x, rotate_y, translate_x, translate_y) =
                get_transform_matrix(&component_data);

            components.push(CompositeGlyphComponent {
                glyph_id: component_data.glyph_index,
                transform: [
                    scale_x,
                    rotate_x,
                    translate_x,
                    scale_y,
                    rotate_y,
                    translate_y,
                ],
            });

            debug!(
                "Glyph {:?}: component references glyph {:?} with transform {:?}",
                glyph_id,
                component_data.glyph_index,
                [
                    scale_x,
                    rotate_x,
                    translate_x,
                    scale_y,
                    rotate_y,
                    translate_y
                ]
            );
        }
    }

    Ok(GlyphOutlineData::Composite(components))
}

/// Extract transformation matrix from composite component
fn get_transform_matrix(component: &glyf::CompositeGlyph) -> (f32, f32, f32, f32, f32, f32) {
    match component.transform {
        glyf::Transform::Offset { x, y } => {
            // No scaling, just translation
            (1.0, 0.0, x as f32, 0.0, 1.0, y as f32)
        }
        glyf::Transform::Scaled { x, y } => {
            // Uniform scaling in X and Y
            (x, 0.0, 0.0, 0.0, y, 0.0)
        }
        glyf::Transform::TwoByTwo { xx, xy, yx, yy } => {
            // Full 2x2 transformation matrix
            (xx, xy, 0.0, yx, yy, 0.0)
        }
    }
}

/// ============================================================================
/// PATTERN 3: Get glyph metrics (advance width, bounds)
/// ============================================================================
pub fn get_glyph_metrics(
    font: &SkrifaFontRef,
    glyph_id: GlyphId,
) -> Result<(u16, Option<i32>), Box<dyn std::error::Error>> {
    let hmtx = font.hmtx()?;
    let metrics = hmtx.metrics(glyph_id);

    let advance_width = match metrics {
        Ok(metric) => metric.advance_width,
        Err(_) => {
            warn!(
                "Failed to get metrics for glyph {:?}, using default",
                glyph_id
            );
            0
        }
    };

    // Vertical metrics are less commonly used
    let vertical_advance = None;

    Ok((advance_width, vertical_advance))
}

/// Get bounding box for a glyph
pub fn get_glyph_bbox(
    font: &ReadFontRef,
    glyph_id: GlyphId,
) -> Result<Option<(i16, i16, i16, i16)>, Box<dyn std::error::Error>> {
    // Try to get bbox from glyf table
    if let Ok(glyf_table) = font.glyf() {
        if let Ok(Some(glyph)) = glyf_table.get(glyph_id) {
            if let Ok(bbox) = glyph.bounding_box() {
                return Ok(Some((bbox.x_min, bbox.y_min, bbox.x_max, bbox.y_max)));
            }
        }
    }

    Ok(None)
}

/// ============================================================================
/// PATTERN 4: Transform glyph for weight adjustment
/// ============================================================================

/// Apply weight transformation to a glyph
pub fn transform_glyph_for_weight(
    glyph: &mut ExtractedGlyph,
    scale_factor: f32,
) -> Result<(), Box<dyn std::error::Error>> {
    debug!(
        "Applying weight transformation (scale: {:.2}x) to glyph {:?}",
        scale_factor, glyph.glyph_id
    );

    match &mut glyph.outline_data {
        GlyphOutlineData::TrueType(tt_outline) => {
            // Scale all points
            for point in &mut tt_outline.points {
                let scaled_x = (point.x as f32 * scale_factor) as i32;
                let scaled_y = (point.y as f32 * scale_factor) as i32;
                *point = OutlinePoint {
                    x: scaled_x,
                    y: scaled_y,
                    on_curve: point.on_curve,
                };
            }
        }

        GlyphOutlineData::Composite(components) => {
            // Scale transformation matrices in composite components
            for component in components {
                component.transform[0] *= scale_factor; // scale_x
                component.transform[1] *= scale_factor; // rotate_x
                component.transform[2] *= scale_factor; // translate_x
                component.transform[3] *= scale_factor; // scale_y
                component.transform[4] *= scale_factor; // rotate_y
                component.transform[5] *= scale_factor; // translate_y
            }
        }

        GlyphOutlineData::PostScript(_) => {
            warn!("Weight transformation for CFF outlines not yet implemented");
        }

        GlyphOutlineData::Empty => {
            // Nothing to transform
        }
    }

    // Scale metrics
    glyph.advance_width = (glyph.advance_width as f32 * scale_factor) as i32;

    // Scale bbox if present
    if let Some((x_min, y_min, x_max, y_max)) = glyph.bbox {
        glyph.bbox = Some((
            (x_min as f32 * scale_factor) as i16,
            (y_min as f32 * scale_factor) as i16,
            (x_max as f32 * scale_factor) as i16,
            (y_max as f32 * scale_factor) as i16,
        ));
    }

    Ok(())
}

/// ============================================================================
/// PATTERN 5: Access codepoint mapping
/// ============================================================================

/// Get all codepoints and their corresponding glyph IDs
pub fn get_codepoint_to_glyph_map(
    font: &SkrifaFontRef,
) -> Result<HashMap<u32, GlyphId>, Box<dyn std::error::Error>> {
    let charmap = font.charmap();
    let mut map = HashMap::new();

    // Iterate through all codepoints in the font's character map
    for (codepoint, glyph_id) in charmap.mappings() {
        map.insert(codepoint, glyph_id);
    }

    debug!("Font has {} codepoint-to-glyph mappings", map.len());

    Ok(map)
}

/// Check if a font has a specific codepoint
pub fn has_codepoint(font: &SkrifaFontRef, codepoint: u32) -> bool {
    font.charmap().map(codepoint).is_some()
}

/// ============================================================================
/// PATTERN 6: Copy glyph data between fonts (for low-level access)
/// ============================================================================

/// Get raw glyph bytes from glyf table for direct copying
pub fn get_raw_glyf_bytes(
    font: &ReadFontRef,
    glyph_id: GlyphId,
) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
    // This accesses the raw binary data for the glyph
    // Useful for copying glyphs without full deserialization

    if let Ok(glyf_table) = font.glyf() {
        if let Ok(Some(glyph)) = glyf_table.get(glyph_id) {
            // Get the raw bytes that represent this glyph
            // Note: read-fonts doesn't expose raw bytes directly,
            // but you can access the underlying data via the glyf table

            // For now, return None - proper implementation would need
            // to work with the binary glyf data directly
            return Ok(None);
        }
    }

    Ok(None)
}

/// Get raw CFF charstring data
pub fn get_raw_cff_charstring(
    font: &ReadFontRef,
    glyph_id: GlyphId,
) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
    if let Ok(_cff) = font.cff() {
        // CFF charstring extraction would go here
        // This requires parsing CFF INDEX structures
        return Ok(None);
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_outline_point_creation() {
        let point = OutlinePoint {
            x: 100,
            y: 200,
            on_curve: true,
        };

        assert_eq!(point.x, 100);
        assert_eq!(point.y, 200);
        assert!(point.on_curve);
    }

    #[test]
    fn test_outline_point_comparison() {
        let p1 = OutlinePoint {
            x: 10,
            y: 20,
            on_curve: true,
        };
        let p2 = OutlinePoint {
            x: 10,
            y: 20,
            on_curve: true,
        };
        let p3 = OutlinePoint {
            x: 15,
            y: 20,
            on_curve: true,
        };

        assert_eq!(p1, p2);
        assert_ne!(p1, p3);
    }
}
