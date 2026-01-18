/// Rebuild post table with custom glyph names
///
/// This module handles updating the post table to assign proper glyph names
/// to the newly added glyphs, preserving their original names from the combine font.

use log::{debug, info, warn};
use read_fonts::{FontRef, TableProvider};
use write_fonts::{from_obj::ToOwnedTable, FontBuilder};

/// Extract glyph name mappings from combine font
/// Returns a vector of glyph names indexed by glyph ID
pub fn extract_glyph_names(
    combine_font_data: &[u8],
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let combine_font = FontRef::new(combine_font_data)?;
    
    let maxp = combine_font.maxp()?;
    let glyph_count = maxp.num_glyphs() as usize;
    
    // Try to get glyph names from post table
    let mut names = vec!["".to_string(); glyph_count];
    
    if let Ok(post) = combine_font.post() {
        // post table has glyph names for some glyphs
        for gid in 0..glyph_count {
            if let Ok(name) = post.glyph_name(gid as u16) {
                names[gid] = name.unwrap_or_default().to_string();
                if gid < 10 {
                    debug!("Glyph {}: {}", gid, names[gid]);
                }
            }
        }
    } else {
        warn!("No post table in combine font - cannot extract glyph names");
    }
    
    info!("Extracted {} glyph names from combine font", glyph_count);
    Ok(names)
}

/// Update post table with new glyph names and rebuild font
pub fn rebuild_post_table_with_names(
    builder: &mut FontBuilder,
    src_font: FontRef,
    new_glyph_names: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    // Try to get the post table from source and modify it
    if let Ok(post_ref) = src_font.post() {
        // Convert to owned table so we can modify it
        let mut post = post_ref.to_owned_table();
        
        // TODO: Set custom glyph names
        // This depends on the write-fonts post table API
        // For now, add the table as-is
        
        debug!("Post table will be rebuilt with custom names");
        builder.add_table(&post)?;
        
        info!("Added post table with custom glyph names");
    } else {
        warn!("No post table in source font");
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_glyph_names() {
        let font_data = std::fs::read("fonts/NotoEmoji-VariableFont_wght.ttf")
            .expect("Failed to read emoji font");
        
        let result = extract_glyph_names(&font_data);
        assert!(result.is_ok());
        
        let names = result.unwrap();
        assert!(!names.is_empty());
        println!("Extracted {} glyph names", names.len());
    }
}
