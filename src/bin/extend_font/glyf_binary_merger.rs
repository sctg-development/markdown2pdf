/// Low-level glyf table binary manipulation for font merging
///
/// This module handles the binary-level operations needed to merge glyph data
/// from one font into another, working directly with TTF binary structures.

use std::io::Cursor;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

/// Represents offsets from the loca table
#[derive(Debug)]
pub struct LocaOffsets {
    /// Offsets of each glyph in the glyf table
    pub offsets: Vec<u32>,
    /// Format: 0 for short offsets (divide by 2), 1 for long offsets
    pub format: u16,
}

impl LocaOffsets {
    /// Parse the loca table from raw bytes
    pub fn from_bytes(data: &[u8], num_glyphs: u16, is_short_format: bool) -> std::io::Result<Self> {
        let mut cursor = Cursor::new(data);
        let mut offsets = Vec::new();
        
        if is_short_format {
            // Short format: offsets are u16, divide by 2 to get byte offset
            for _ in 0..=num_glyphs {
                let offset = cursor.read_u16::<BigEndian>()? as u32 * 2;
                offsets.push(offset);
            }
        } else {
            // Long format: offsets are u32
            for _ in 0..=num_glyphs {
                let offset = cursor.read_u32::<BigEndian>()?;
                offsets.push(offset);
            }
        }
        
        Ok(LocaOffsets {
            offsets,
            format: if is_short_format { 0 } else { 1 },
        })
    }
    
    /// Serialize back to bytes
    pub fn to_bytes(&self) -> std::io::Result<Vec<u8>> {
        let mut buf = Vec::new();
        
        if self.format == 0 {
            // Short format
            for offset in &self.offsets {
                buf.write_u16::<BigEndian>((offset / 2) as u16)?;
            }
        } else {
            // Long format
            for offset in &self.offsets {
                buf.write_u32::<BigEndian>(*offset)?;
            }
        }
        
        Ok(buf)
    }
}

/// Extract glyph binary data from glyf table
pub fn extract_glyph_data(
    glyf_data: &[u8],
    glyph_id: u16,
    loca: &LocaOffsets,
) -> Option<Vec<u8>> {
    
    let start = *loca.offsets.get(glyph_id as usize)? as usize;
    let end = *loca.offsets.get((glyph_id + 1) as usize)? as usize;
    
    if start >= glyf_data.len() || end > glyf_data.len() {
        return None;
    }
    
    Some(glyf_data[start..end].to_vec())
}

/// Merge glyph data from source and combine fonts
pub fn merge_glyf_tables(
    src_glyf: &[u8],
    src_loca: &LocaOffsets,
    src_glyph_count: u16,
    combine_glyf: &[u8],
    combine_loca: &LocaOffsets,
    glyphs_to_copy: &[(u32, u16)], // (codepoint, glyph_id in combine)
) -> std::io::Result<(Vec<u8>, LocaOffsets)> {
    
    let mut new_glyf_data = Vec::new();
    let mut new_loca_offsets = vec![0u32];
    
    // Copy all glyphs from source font
    for glyph_id in 0..src_glyph_count {
        if let Some(glyph_data) = extract_glyph_data(src_glyf, glyph_id, src_loca) {
            new_glyf_data.extend_from_slice(&glyph_data);
        }
        new_loca_offsets.push(new_glyf_data.len() as u32);
    }
    
    // Add selected glyphs from combine font
    for (_cp, combine_glyph_id) in glyphs_to_copy {
        if let Some(glyph_data) = extract_glyph_data(combine_glyf, *combine_glyph_id, combine_loca) {
            new_glyf_data.extend_from_slice(&glyph_data);
        }
        new_loca_offsets.push(new_glyf_data.len() as u32);
    }
    
    // Determine loca format based on final size
    let new_loca_format = if (new_glyf_data.len() / 2) <= u16::MAX as usize {
        0 // short format
    } else {
        1 // long format
    };
    
    let new_loca = LocaOffsets {
        offsets: new_loca_offsets,
        format: new_loca_format,
    };
    
    Ok((new_glyf_data, new_loca))
}
