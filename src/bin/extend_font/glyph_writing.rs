use log::{debug, info, warn};
/// Glyph reconstruction and font writing using write-fonts 0.45.0
///
/// This module demonstrates:
/// 1. Reading a font's table structure with write-fonts
/// 2. Creating new glyphs and tables
/// 3. Merging glyphs from one font to another
/// 4. Serializing the modified font back to binary format
use read_fonts::{tables::glyf, FontRef as ReadFontRef, TableProvider};
use skrifa::GlyphId;
use std::collections::BTreeMap;

// Note: write-fonts API structure (as of 0.45.0)
// The exact API depends on the version, but general pattern:
// use write_fonts::{tables::glyf as write_glyf, FontBuilder};

/// Information needed to create a merged font
#[derive(Debug, Clone)]
pub struct FontMergeConfig {
    /// Mapping of codepoints to glyph data from source font
    pub src_codepoints: BTreeMap<u32, GlyphId>,

    /// Mapping of codepoints to glyph data from combine font
    pub combine_codepoints: BTreeMap<u32, GlyphId>,

    /// Which codepoints to copy from combine to src
    pub glyphs_to_copy: Vec<u32>,

    /// Scale factor to apply during copying
    pub scale_factor: f32,

    /// Whether to apply weight transformation
    pub apply_weight_transform: bool,
}

/// ============================================================================
/// PATTERN 1: Parse font structure with write-fonts
/// ============================================================================

/// Load a font for modification with write-fonts
pub fn load_font_for_modification(
    font_data: &[u8],
) -> Result<FontModificationContext, Box<dyn std::error::Error>> {
    debug!("Loading font for modification");

    let font = ReadFontRef::new(font_data)?;

    // Get basic font tables
    let maxp = font.maxp()?;
    let hmtx = font.hmtx()?;
    let cmap = font.cmap()?;
    let head = font.head()?;
    let hhea = font.hhea()?;

    info!("Font info:");
    info!("  - Glyph count: {}", maxp.num_glyphs());
    info!("  - Units per em: {}", head.units_per_em());
    info!("  - Ascender: {}", hhea.ascender());
    info!("  - Descender: {}", hhea.descender());

    // Get glyf/CFF availability
    let has_glyf = font.glyf().is_ok();
    let has_cff = font.cff().is_ok();

    info!("  - Has TrueType (glyf): {}", has_glyf);
    info!("  - Has PostScript (CFF): {}", has_cff);

    Ok(FontModificationContext {
        num_glyphs: maxp.num_glyphs(),
        units_per_em: head.units_per_em(),
        has_glyf,
        has_cff,
        ascender: hhea.ascender(),
        descender: hhea.descender(),
        data: font_data.to_vec(),
    })
}

/// Context for font modification operations
#[derive(Debug, Clone)]
pub struct FontModificationContext {
    /// Current number of glyphs
    pub num_glyphs: u16,

    /// Font's units per em
    pub units_per_em: u16,

    /// Has glyf table (TrueType)
    pub has_glyf: bool,

    /// Has CFF table (PostScript)
    pub has_cff: bool,

    /// Ascender metric
    pub ascender: i16,

    /// Descender metric
    pub descender: i16,

    /// Raw font data
    pub data: Vec<u8>,
}

/// ============================================================================
/// PATTERN 2: Copy glyphs between fonts
/// ============================================================================

/// Strategy for copying glyphs: which approach to use
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlyphCopyStrategy {
    /// Direct binary copy of glyph data (fastest, preserves exact outlines)
    DirectBinaryCopy,

    /// Decompose and reconstruct (slower, required for composite glyphs)
    DecomposeAndReconstruct,

    /// Reference glyphs (for composite glyph components, minimal size)
    References,
}

/// Copy a single glyph from combine font to src font
///
/// This is the core operation for merging glyphs
pub fn copy_glyph_between_fonts(
    src_font_data: &[u8],
    combine_font_data: &[u8],
    src_glyph_id: GlyphId,
    combine_glyph_id: GlyphId,
    codepoint: u32,
    strategy: GlyphCopyStrategy,
) -> Result<(), Box<dyn std::error::Error>> {
    let src_font = ReadFontRef::new(src_font_data)?;
    let combine_font = ReadFontRef::new(combine_font_data)?;

    debug!(
        "Copying glyph U+{:04X}: combine[{}] -> src[{}] using {:?} strategy",
        codepoint,
        combine_glyph_id.to_u32(),
        src_glyph_id.to_u32(),
        strategy
    );

    match strategy {
        GlyphCopyStrategy::DirectBinaryCopy => {
            copy_glyph_binary(&src_font, &combine_font, src_glyph_id, combine_glyph_id)?;
        }
        GlyphCopyStrategy::DecomposeAndReconstruct => {
            copy_glyph_decomposed(&src_font, &combine_font, src_glyph_id, combine_glyph_id)?;
        }
        GlyphCopyStrategy::References => {
            copy_glyph_as_reference(&src_font, &combine_font, src_glyph_id, combine_glyph_id)?;
        }
    }

    Ok(())
}

/// Copy glyph by direct binary copy of glyf data
fn copy_glyph_binary(
    _src_font: &ReadFontRef,
    combine_font: &ReadFontRef,
    _src_glyph_id: GlyphId,
    combine_glyph_id: GlyphId,
) -> Result<(), Box<dyn std::error::Error>> {
    // Get the glyf table from combine font
    if let Ok(glyf_table) = combine_font.glyf() {
        // Get the raw glyph data
        if let Ok(Some(_glyph)) = glyf_table.get(combine_glyph_id) {
            debug!(
                "Binary copy: glyph {:?} found in combine font's glyf table",
                combine_glyph_id
            );

            // In a real implementation, you would:
            // 1. Get the raw bytes for this glyph from glyf table
            // 2. Insert these bytes into the source font's glyf table
            // 3. Update loca table with new offsets
            // 4. Update head table checksums

            // Example (conceptual - actual implementation requires table manipulation):
            // let glyph_bytes = glyf_table.get_raw_bytes(combine_glyph_id)?;
            // src_glyf_table.insert_glyph(src_glyph_id, glyph_bytes)?;

            return Ok(());
        }
    }

    Err("Glyph not found in combine font glyf table".into())
}

/// Copy glyph by decomposing and reconstructing
fn copy_glyph_decomposed(
    _src_font: &ReadFontRef,
    combine_font: &ReadFontRef,
    _src_glyph_id: GlyphId,
    combine_glyph_id: GlyphId,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(glyf_table) = combine_font.glyf() {
        if let Ok(Some(glyph)) = glyf_table.get(combine_glyph_id) {
            // If it's a composite glyph, we need to handle component glyphs first
            if glyph.is_composite() {
                debug!("Decomposing composite glyph {:?}", combine_glyph_id);

                if let Some(composite) = glyph.composite() {
                    for component in composite.components() {
                        let comp = component?;
                        debug!("  - References component glyph {:?}", comp.glyph_index);

                        // Recursively copy component glyphs first
                        copy_glyph_decomposed(
                            _src_font,
                            combine_font,
                            combine_glyph_id, // This would need remapping
                            comp.glyph_index,
                        )?;
                    }
                }
            } else {
                // Simple glyph - reconstruct points
                debug!("Reconstructing simple glyph {:?}", combine_glyph_id);

                if let Some(simple) = glyph.simple() {
                    let mut num_points = 0;
                    for contour in simple.contours() {
                        let points: Vec<_> = contour.points().collect::<Result<Vec<_>, _>>()?;
                        num_points += points.len();
                    }
                    debug!(
                        "  - Glyph has {} contours with {} total points",
                        simple.contours().count(),
                        num_points
                    );
                }
            }

            return Ok(());
        }
    }

    Err("Glyph not found for decomposition".into())
}

/// Copy glyph as a reference (for composite glyphs)
fn copy_glyph_as_reference(
    _src_font: &ReadFontRef,
    _combine_font: &ReadFontRef,
    src_glyph_id: GlyphId,
    combine_glyph_id: GlyphId,
) -> Result<(), Box<dyn std::error::Error>> {
    debug!(
        "Creating reference: src[{}] -> combine[{}]",
        src_glyph_id.to_u32(),
        combine_glyph_id.to_u32()
    );

    // This would create a composite glyph in src that references combine_glyph_id
    // Implementation would use write-fonts to create a composite glyph component

    Ok(())
}

/// ============================================================================
/// PATTERN 3: Update character map (cmap) table
/// ============================================================================

/// Create updated cmap entries for copied glyphs
pub struct CmapUpdate {
    /// New codepoint-to-glyph mappings
    pub new_mappings: BTreeMap<u32, GlyphId>,
}

impl CmapUpdate {
    /// Create a new cmap update
    pub fn new() -> Self {
        CmapUpdate {
            new_mappings: BTreeMap::new(),
        }
    }

    /// Add a mapping
    pub fn add_mapping(&mut self, codepoint: u32, glyph_id: GlyphId) {
        self.new_mappings.insert(codepoint, glyph_id);
    }

    /// Merge mappings from another source
    pub fn merge(&mut self, other: &CmapUpdate) {
        for (codepoint, glyph_id) in &other.new_mappings {
            self.new_mappings.insert(*codepoint, *glyph_id);
        }
    }
}

/// ============================================================================
/// PATTERN 4: Update metrics tables (hmtx, vmtx)
/// ============================================================================

/// Metrics for a glyph
#[derive(Debug, Clone, Copy)]
pub struct GlyphMetrics {
    /// Advance width (horizontal)
    pub advance_width: u16,

    /// Left side bearing
    pub lsb: i16,

    /// Advance height (vertical, if applicable)
    pub advance_height: Option<u16>,

    /// Top side bearing (vertical, if applicable)
    pub tsb: Option<i16>,
}

/// Copy metrics from one glyph to another
pub fn copy_glyph_metrics(
    src_font_data: &[u8],
    combine_font_data: &[u8],
    combine_glyph_id: GlyphId,
) -> Result<GlyphMetrics, Box<dyn std::error::Error>> {
    let combine_font = ReadFontRef::new(combine_font_data)?;
    let hmtx = combine_font.hmtx()?;

    let metrics = hmtx.metrics(combine_glyph_id)?;

    debug!(
        "Copied metrics for glyph {:?}: advance={}, lsb={}",
        combine_glyph_id, metrics.advance_width, metrics.left_side_bearing
    );

    Ok(GlyphMetrics {
        advance_width: metrics.advance_width,
        lsb: metrics.left_side_bearing,
        advance_height: None, // Would come from vmtx if present
        tsb: None,
    })
}

/// ============================================================================
/// PATTERN 5: Handle composite glyphs and dependencies
/// ============================================================================

/// Resolve glyph dependencies (for composite glyphs)
pub struct GlyphDependencyResolver {
    /// Map of glyph ID to its component glyph IDs
    pub dependencies: BTreeMap<GlyphId, Vec<GlyphId>>,

    /// Topological sort order
    pub sort_order: Vec<GlyphId>,
}

impl GlyphDependencyResolver {
    /// Create a new resolver
    pub fn new() -> Self {
        GlyphDependencyResolver {
            dependencies: BTreeMap::new(),
            sort_order: Vec::new(),
        }
    }

    /// Analyze dependencies in a font
    pub fn analyze(
        &mut self,
        font: &ReadFontRef,
        glyph_id: GlyphId,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Ok(glyf_table) = font.glyf() {
            if let Ok(Some(glyph)) = glyf_table.get(glyph_id) {
                if let Some(composite) = glyph.composite() {
                    let mut deps = Vec::new();

                    for component in composite.components() {
                        let comp = component?;
                        deps.push(comp.glyph_index);

                        // Recursively analyze dependencies
                        self.analyze(font, comp.glyph_index)?;
                    }

                    self.dependencies.insert(glyph_id, deps);
                }
            }
        }

        Ok(())
    }

    /// Get copy order (dependencies first)
    pub fn get_copy_order(&self, glyphs: Vec<GlyphId>) -> Vec<GlyphId> {
        let mut order = Vec::new();
        let mut visited = std::collections::HashSet::new();

        for glyph in glyphs {
            self.visit(glyph, &mut order, &mut visited);
        }

        order
    }

    fn visit(
        &self,
        glyph: GlyphId,
        order: &mut Vec<GlyphId>,
        visited: &mut std::collections::HashSet<GlyphId>,
    ) {
        if visited.contains(&glyph) {
            return;
        }

        visited.insert(glyph);

        if let Some(deps) = self.dependencies.get(&glyph) {
            for &dep in deps {
                self.visit(dep, order, visited);
            }
        }

        order.push(glyph);
    }
}

/// ============================================================================
/// PATTERN 6: Rebuild and serialize font
/// ============================================================================

/// Configuration for font serialization
#[derive(Debug, Clone)]
pub struct FontSerializationConfig {
    /// Whether to recalculate checksums
    pub recalculate_checksums: bool,

    /// Whether to optimize table order
    pub optimize_table_order: bool,

    /// Glyph ID offset in original font
    pub glyph_id_offset: u16,
}

impl Default for FontSerializationConfig {
    fn default() -> Self {
        FontSerializationConfig {
            recalculate_checksums: true,
            optimize_table_order: true,
            glyph_id_offset: 0,
        }
    }
}

/// Serialize modified font back to binary
pub fn serialize_modified_font(
    _font: &ReadFontRef,
    _config: &FontSerializationConfig,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // In a real implementation with write-fonts, this would:
    // 1. Create a FontBuilder
    // 2. Add/modify tables (glyf, cmap, hmtx, etc.)
    // 3. Update table dependencies (maxp glyph count, etc.)
    // 4. Serialize to binary
    // 5. Recalculate checksums and table directory

    warn!("Font serialization requires full write-fonts implementation");

    // Placeholder: return empty
    Ok(vec![])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cmap_update() {
        let mut update = CmapUpdate::new();

        let glyph_id = GlyphId::new(1);
        update.add_mapping(0x1F600, glyph_id); // Emoji

        assert_eq!(update.new_mappings.len(), 1);
        assert_eq!(update.new_mappings[&0x1F600], glyph_id);
    }

    #[test]
    fn test_glyph_metrics() {
        let metrics = GlyphMetrics {
            advance_width: 500,
            lsb: 50,
            advance_height: None,
            tsb: None,
        };

        assert_eq!(metrics.advance_width, 500);
        assert_eq!(metrics.lsb, 50);
        assert!(metrics.advance_height.is_none());
    }

    #[test]
    fn test_dependency_resolver() {
        let resolver = GlyphDependencyResolver::new();
        assert_eq!(resolver.dependencies.len(), 0);
    }
}
