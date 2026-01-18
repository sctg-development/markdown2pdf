/// Build and serialize a cmap (character map) table in pure Rust
///
/// This module handles creating a cmap table from a mapping of codepoints to glyph IDs.
/// Uses OpenType cmap format 12 (segmented coverage) for full Unicode support.
use byteorder::{BigEndian, WriteBytesExt};
use log::{debug, info};
use std::collections::BTreeMap;
use std::io::Cursor;

/// Build a cmap table with the given codepoint to glyph ID mappings
/// Returns the serialized cmap table data
pub fn build_cmap_table(
    codepoint_to_glyph_id: &BTreeMap<u32, u16>,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    info!(
        "Building cmap table with {} codepoint mappings",
        codepoint_to_glyph_id.len()
    );

    // We'll build a cmap with format 4 (BMP only) for efficiency
    // For non-BMP codepoints, add format 12 subtable

    let mut cmap_data = Vec::new();
    let mut cursor = Cursor::new(&mut cmap_data);

    // cmap table version
    cursor.write_u16::<BigEndian>(0)?; // version

    // Count subtables: 1 for format 4 (BMP) + 1 for format 12 (non-BMP if needed)
    let has_non_bmp = codepoint_to_glyph_id.keys().any(|&cp| cp > 0xFFFF);
    let num_subtables = if has_non_bmp { 2 } else { 1 };
    cursor.write_u16::<BigEndian>(num_subtables)?;

    // Build format 4 subtable for BMP (0x0000-0xFFFF)
    let fmt4_data = build_cmap_format4(codepoint_to_glyph_id)?;
    let fmt4_offset = 4 + (num_subtables as usize * 8); // After header and subtable records

    // Write format 4 subtable record
    cursor.write_u16::<BigEndian>(3)?; // platformID (Windows)
    cursor.write_u16::<BigEndian>(1)?; // platEncID (Unicode BMP)
    cursor.write_u32::<BigEndian>(fmt4_offset as u32)?; // offset

    // Write format 12 subtable record if needed
    let fmt12_offset = if has_non_bmp {
        fmt4_offset + fmt4_data.len()
    } else {
        0
    };

    if has_non_bmp {
        cursor.write_u16::<BigEndian>(3)?; // platformID (Windows)
        cursor.write_u16::<BigEndian>(10)?; // platEncID (Unicode full repertoire)
        cursor.write_u32::<BigEndian>(fmt12_offset as u32)?; // offset
    }

    // Append format 4 subtable
    cmap_data.extend_from_slice(&fmt4_data);

    // Build and append format 12 subtable if needed
    if has_non_bmp {
        let fmt12_data = build_cmap_format12(codepoint_to_glyph_id)?;
        cmap_data.extend_from_slice(&fmt12_data);
    }

    debug!("Built cmap table: {} bytes", cmap_data.len());
    Ok(cmap_data)
}

/// Build cmap format 4 subtable (BMP: 0x0000-0xFFFF)
fn build_cmap_format4(
    codepoint_to_glyph_id: &BTreeMap<u32, u16>,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut data = Vec::new();
    let mut cursor = Cursor::new(&mut data);

    // Collect BMP entries only (0x0000-0xFFFF)
    let bmp_entries: BTreeMap<u16, u16> = codepoint_to_glyph_id
        .iter()
        .filter_map(|(&cp, &glyph_id)| {
            if cp <= 0xFFFF {
                Some((cp as u16, glyph_id))
            } else {
                None
            }
        })
        .collect();

    if bmp_entries.is_empty() {
        // Empty table - just write header
        cursor.write_u16::<BigEndian>(4)?; // format
        cursor.write_u16::<BigEndian>(12)?; // length (header only)
        cursor.write_u16::<BigEndian>(0)?; // language
        return Ok(data);
    }

    // Build segments for format 4
    // A segment is a contiguous range of codepoints mapping to sequential glyph IDs
    let mut segments = Vec::new();

    let mut prev_cp = 0u16;
    let mut prev_gid = 0u16;
    let mut segment_start = 0u16;
    let mut segment_start_gid = 0u16;

    for (&cp, &glyph_id) in &bmp_entries {
        if cp != prev_cp + 1 || glyph_id != prev_gid + 1 {
            // End previous segment and start new one
            if prev_cp >= segment_start {
                segments.push((segment_start, prev_cp, segment_start_gid));
            }
            segment_start = cp;
            segment_start_gid = glyph_id;
        }
        prev_cp = cp;
        prev_gid = glyph_id;
    }

    // Add final segment
    if prev_cp >= segment_start {
        segments.push((segment_start, prev_cp, segment_start_gid));
    }

    // Add end-of-segments marker
    segments.push((0xFFFF, 0xFFFF, 0));

    let seg_count = segments.len();
    let seg_count_u16 = seg_count as u16;
    // Compute largest power of two less than or equal to seg_count
    let mut power = 1u16;
    while power * 2 <= seg_count_u16 {
        power <<= 1;
    }
    let search_range = power << 1;
    let entry_selector = power.trailing_zeros() as u16;
    let range_shift = seg_count_u16 * 2 - search_range;

    // Write format 4 header
    cursor.write_u16::<BigEndian>(4)?; // format

    // Length: header + 4 arrays of seg_count entries
    let length = (14 + seg_count * 8) as u16;
    cursor.write_u16::<BigEndian>(length)?;
    cursor.write_u16::<BigEndian>(0)?; // language

    cursor.write_u16::<BigEndian>(seg_count as u16 * 2)?; // segCountX2
    cursor.write_u16::<BigEndian>(search_range)?;
    cursor.write_u16::<BigEndian>(entry_selector)?;
    cursor.write_u16::<BigEndian>(range_shift)?;

    // Write end code array
    for (_, end, _) in &segments {
        cursor.write_u16::<BigEndian>(*end)?;
    }

    // Write reserved pad
    cursor.write_u16::<BigEndian>(0)?;

    // Write start code array
    for (start, _, _) in &segments {
        cursor.write_u16::<BigEndian>(*start)?;
    }

    // Write idDelta and idRangeOffset arrays (simplified: just store glyph IDs directly)
    // For simplicity, we use idRangeOffset approach with a glyphIdArray
    // This is complex; for now use idDelta (works if mappings are sequential)

    for (start, _, start_gid) in &segments {
        // idDelta = start_glyph_id - start_codepoint
        let delta = *start_gid as i16 - *start as i16;
        cursor.write_i16::<BigEndian>(delta)?;
    }

    // For this simplified implementation, write 0 for idRangeOffset
    for _ in 0..seg_count {
        cursor.write_u16::<BigEndian>(0)?;
    }

    Ok(data)
}

/// Build cmap format 12 subtable (full Unicode including supplementary planes)
fn build_cmap_format12(
    codepoint_to_glyph_id: &BTreeMap<u32, u16>,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut data = Vec::new();
    let mut cursor = Cursor::new(&mut data);

    // Collect all entries (including non-BMP)
    let mut groups = Vec::new();

    let mut prev_cp = 0u32;
    let mut prev_gid = 0u16;
    let mut group_start_cp = 0u32;
    let mut group_start_gid = 0u16;

    for (&cp, &glyph_id) in codepoint_to_glyph_id {
        if cp != prev_cp + 1 || glyph_id != prev_gid + 1 {
            // End previous group
            if prev_cp >= group_start_cp {
                groups.push((group_start_cp, prev_cp, group_start_gid));
            }
            group_start_cp = cp;
            group_start_gid = glyph_id;
        }
        prev_cp = cp;
        prev_gid = glyph_id;
    }

    // Add final group
    if prev_cp >= group_start_cp {
        groups.push((group_start_cp, prev_cp, group_start_gid));
    }

    let num_groups = groups.len() as u32;

    // Write format 12 header
    cursor.write_u16::<BigEndian>(12)?; // format
    cursor.write_u16::<BigEndian>(0)?; // reserved
    let length = (16 + num_groups * 12) as u32;
    cursor.write_u32::<BigEndian>(length)?;
    cursor.write_u32::<BigEndian>(0)?; // language

    cursor.write_u32::<BigEndian>(num_groups)?; // nGroups

    // Write groups
    for (start_cp, end_cp, start_gid) in groups {
        cursor.write_u32::<BigEndian>(start_cp)?;
        cursor.write_u32::<BigEndian>(end_cp)?;
        cursor.write_u32::<BigEndian>(start_gid as u32)?;
    }

    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_cmap_single_bmp_entry() {
        let mut mappings = BTreeMap::new();
        mappings.insert(0x41, 1); // 'A' -> glyph 1

        let result = build_cmap_table(&mappings);
        assert!(result.is_ok());
        let cmap = result.unwrap();
        assert!(cmap.len() > 0);
    }

    #[test]
    fn test_build_cmap_with_non_bmp() {
        let mut mappings = BTreeMap::new();
        mappings.insert(0x41, 1); // BMP
        mappings.insert(0x1F600, 2); // Non-BMP emoji

        let result = build_cmap_table(&mappings);
        assert!(result.is_ok());
        let cmap = result.unwrap();
        assert!(cmap.len() > 0);
    }
}
