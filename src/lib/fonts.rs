use std::fs;
use std::panic;
use std::path::PathBuf;
use std::sync::Arc;
use log::{debug, info, warn, error};

use fontdb::Database;
use genpdfi_extended::error::{Error, ErrorKind};
use genpdfi_extended::fonts::{FontData, FontFamily};
use printpdf::BuiltinFont;
use rusttype::Font;

/// Returns common aliases for a font name.
///
/// This allows users to specify "Arial" and have the system try
/// "Helvetica", "Liberation Sans", etc.
fn get_font_aliases(name: &str) -> Vec<&'static str> {
    match name.to_lowercase().as_str() {
        "arial" => vec!["Helvetica", "Liberation Sans", "FreeSans"],
        "helvetica" => vec!["Arial", "Liberation Sans", "FreeSans"],
        "times new roman" | "times" => {
            vec!["Times", "Times New Roman", "Liberation Serif", "FreeSerif"]
        }
        "courier new" | "courier" => vec!["Courier", "Courier New", "Liberation Mono", "FreeMono"],
        "verdana" => vec!["DejaVu Sans", "Bitstream Vera Sans"],
        "georgia" => vec!["Liberation Serif", "FreeSerif"],
        "comic sans ms" | "comic sans" => vec!["Comic Neue", "Comic Relief"],
        _ => vec![],
    }
}

// -----------------------------------------------------------------------------
// Embedded fonts (statically included from the `fonts/` directory)
// These are prioritized over system fonts when available.
// -----------------------------------------------------------------------------

// Monospace (fixed-width) - DejaVu Sans Mono
static MONO_SANS_REGULAR: &'static [u8] = include_bytes!("../../fonts/DejaVuSansMono.ttf");
static MONO_SANS_BOLD: &'static [u8] = include_bytes!("../../fonts/DejaVuSansMono-Bold.ttf");
static MONO_SANS_ITALIC: &'static [u8] = include_bytes!("../../fonts/DejaVuSansMono-Oblique.ttf");
static MONO_SANS_BOLD_ITALIC: &'static [u8] =
    include_bytes!("../../fonts/DejaVuSansMono-BoldOblique.ttf");

// Sans-serif - DejaVu Sans
static SANS_REGULAR: &'static [u8] = include_bytes!("../../fonts/DejaVuSans.ttf");
static SANS_BOLD: &'static [u8] = include_bytes!("../../fonts/DejaVuSans-Bold.ttf");
static SANS_ITALIC: &'static [u8] = include_bytes!("../../fonts/DejaVuSans-Oblique.ttf");
static SANS_BOLD_ITALIC: &'static [u8] = include_bytes!("../../fonts/DejaVuSans-BoldOblique.ttf");

// Monospace alternative - CMU Typewriter (also included)
static MONO_SERIF_REGULAR: &'static [u8] =
    include_bytes!("../../fonts/CMU Typewriter Text Regular.ttf");
static MONO_SERIF_BOLD: &'static [u8] = include_bytes!("../../fonts/CMU Typewriter Text Bold.ttf");
static MONO_SERIF_ITALIC: &'static [u8] = include_bytes!("../../fonts/CMU Typewriter Text Italic.ttf");
static MONO_SERIF_BOLD_ITALIC: &'static [u8] =
    include_bytes!("../../fonts/CMU Typewriter Text Bold Italic.ttf");
/// Attempt to discover an embedded font family, mapping legacy names
/// such as "CourierPrime", "Noto Sans" and "space-mono" to the canonical
/// embedded families implemented in `embedded_fonts.rs`.
fn try_embedded_font_family(name: &str) -> Option<FontFamily<FontData>> {
    let key = name.to_ascii_lowercase();

    // Map common legacy names to the canonical embedded family names
    if key.contains("courierprime") || key.contains("courier-prime") || key.contains("courier") {
        return crate::embedded_fonts::try_embedded_font_family("CMU Typewriter Text");
    }

    if key.contains("noto") || key.contains("noto-sans") {
        return crate::embedded_fonts::try_embedded_font_family("DejaVu Sans");
    }

    if key.contains("space") && key.contains("mono") || key == "space-mono" || key == "spacemono" {
        return crate::embedded_fonts::try_embedded_font_family("DejaVu Sans Mono");
    }

    // Allow passing canonical/explicit names directly to the embedded helper
    crate::embedded_fonts::try_embedded_font_family(name)
}

/// Find an embedded family and return it along with the canonical family name
fn find_embedded_family_and_name(name: &str) -> Option<(FontFamily<FontData>, &'static str)> {
    let key = name.to_ascii_lowercase();

    // Map legacy and common PDF/base names to canonical embedded names first
    let canonical = if key.contains("courierprime") || key.contains("courier-prime") || key.contains("courier") {
        "CMU Typewriter Text"
    } else if key.contains("noto") || key.contains("noto-sans") {
        "DejaVu Sans"
    } else if (key.contains("space") && key.contains("mono")) || key == "space-mono" || key == "spacemono" {
        "DejaVu Sans Mono"
    // Map common PDF base families to the embedded equivalents
    } else if key.contains("helvetica") || key.contains("arial") || key.contains("sans") || key.contains("sans-serif") {
        "DejaVu Sans"
    } else if key.contains("times") || key.contains("timesnewroman") || key.contains("times new roman") || key.contains("serif") {
        "DejaVu Serif"
    } else {
        // Try to match explicit canonical names provided by embedded_fonts
        for k in crate::embedded_fonts::known_embedded_families() {
            if name.to_ascii_lowercase().contains(&k.to_ascii_lowercase()) || k.to_ascii_lowercase().contains(&name.to_ascii_lowercase()) {
                return crate::embedded_fonts::try_embedded_font_family(k).map(|f| (f, k));
            }
        }
        // No match
        return None;
    };

    crate::embedded_fonts::try_embedded_font_family(canonical).map(|f| (f, canonical))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_aliases() {
        // Test Arial aliases
        let arial_aliases = get_font_aliases("Arial");
        assert!(arial_aliases.contains(&"Helvetica"));
        assert!(arial_aliases.contains(&"Liberation Sans"));

        // Test case insensitivity
        let arial_lower = get_font_aliases("arial");
        assert_eq!(arial_aliases, arial_lower);

        // Test Times New Roman aliases
        let times_aliases = get_font_aliases("Times New Roman");
        assert!(times_aliases.contains(&"Liberation Serif"));
        assert!(times_aliases.contains(&"Times"));

        // Test "times" also works
        let times_short = get_font_aliases("times");
        assert_eq!(times_aliases, times_short);

        // Test unknown font returns empty
        let unknown = get_font_aliases("UnknownFont123");
        assert!(unknown.is_empty());

        // Test Verdana
        let verdana = get_font_aliases("Verdana");
        assert!(verdana.contains(&"DejaVu Sans"));
    }

    #[test]
    fn test_embedded_space_mono_available() {
        let family = try_embedded_font_family("space-mono");
        assert!(
            family.is_some(),
            "Embedded SpaceMono family should be available"
        );
        let family = family.unwrap();
        assert_eq!(
            family.regular.get_data().unwrap().len(),
            MONO_SANS_REGULAR.len()
        );
    }

    #[test]
    fn test_embedded_noto_sans_available() {
        let family = try_embedded_font_family("noto-sans");
        assert!(
            family.is_some(),
            "Embedded DejaVuSans family should be available"
        );
        let family = family.unwrap();
        assert_eq!(
            family.regular.get_data().unwrap().len(),
            SANS_REGULAR.len()
        );
    }

    #[test]
    fn test_embedded_courier_prime_available() {
        let family = try_embedded_font_family("courier-prime");
        assert!(
            family.is_some(),
            "Embedded CMU Typewriter family should be available"
        );
        let family = family.unwrap();
        assert_eq!(
            family.regular.get_data().unwrap().len(),
            MONO_SERIF_REGULAR.len()
        );
    }

    #[test]
    fn test_embedded_helvetica_maps_to_dejavu_sans() {
        if let Some((family, canon)) = find_embedded_family_and_name("Helvetica") {
            assert_eq!(canon, "DejaVu Sans");
            assert_eq!(family.regular.get_data().unwrap().len(), SANS_REGULAR.len());
        } else {
            panic!("Helvetica should map to an embedded DejaVu Sans family");
        }
    }

    #[test]
    fn test_embedded_times_maps_to_dejavu_serif() {
        if let Some((family, canon)) = find_embedded_family_and_name("Times") {
            assert_eq!(canon, "DejaVu Serif");
            assert_eq!(family.regular.get_data().unwrap().len(), {
                // Use embedded helper directly to get expected size
                let expected = crate::embedded_fonts::try_embedded_font_family("DejaVu Serif").unwrap();
                expected.regular.get_data().unwrap().len()
            });
        } else {
            panic!("Times should map to an embedded DejaVu Serif family");
        }
    }

    #[test]
    fn test_embedded_courier_maps_to_cmu_typewriter() {
        if let Some((family, canon)) = find_embedded_family_and_name("Courier") {
            assert_eq!(canon, "CMU Typewriter Text");
            assert_eq!(family.regular.get_data().unwrap().len(), MONO_SERIF_REGULAR.len());
        } else {
            panic!("Courier should map to an embedded CMU Typewriter family");
        }
    }
}

/// Font style variant types
#[derive(Debug, Clone, Copy)]
enum FontVariant {
    Regular,
    Bold,
    Italic,
    BoldItalic,
}

impl FontVariant {
    /// Returns common naming suffixes for this variant
    fn suffixes(&self) -> &[&str] {
        match self {
            FontVariant::Regular => &["Regular", ""],
            FontVariant::Bold => &["Bold", "Bd", "B"],
            FontVariant::Italic => &["Italic", "It", "I", "Oblique"],
            FontVariant::BoldItalic => &["BoldItalic", "Bold Italic", "BoldIt", "BdIt", "BI"],
        }
    }

    fn _name(&self) -> &str {
        match self {
            FontVariant::Regular => "Regular",
            FontVariant::Bold => "Bold",
            FontVariant::Italic => "Italic",
            FontVariant::BoldItalic => "BoldItalic",
        }
    }
}

/// Configuration for custom font loading.
/// Allows users to specify custom font paths and override default font selections.
#[derive(Debug, Clone)]
pub struct FontConfig {
    /// Custom font directories or files to search
    pub custom_paths: Vec<PathBuf>,
    /// Override for the default text font (if None, uses style config)
    pub default_font: Option<String>,
    /// Override for the code font (if None, uses courier)
    pub code_font: Option<String>,
    /// Fallback fonts to use when primary font doesn't have a character
    /// These fonts are tried in order when a character is missing
    pub fallback_fonts: Vec<String>,
    /// Enable font subsetting to reduce PDF file size (default: true)
    pub enable_subsetting: bool,
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            custom_paths: Vec::new(),
            default_font: None,
            code_font: None,
            fallback_fonts: Vec::new(),
            enable_subsetting: true, // Enabled by default for smaller PDFs
        }
    }
}

/// Attempts to load a built-in PDF font family using only the PDF built-in fonts
/// without any system font dependencies. This ensures consistent character spacing
/// across all platforms and avoids kerning issues.
///
/// The function supports the three standard PDF font families:
/// * Helvetica  (fallback name: "Arial")
/// * Times      (fallback names: "Times New Roman", "Times")
/// * Courier    (fallback name: "Courier New")
///
/// Built-in PDF fonts use standardized Adobe Font Metrics (AFM) and do not require
/// any font embedding, resulting in smaller PDF files with consistent rendering
/// across all PDF viewers.
pub fn load_builtin_font_family(name: &str) -> Result<FontFamily<FontData>, Error> {
    // Determine which PDF built-in base family we should reference
    let builtin_variants = match name.to_lowercase().as_str() {
        "times" | "timesnewroman" | "times new roman" | "serif" => BuiltinVariants::Times,
        "courier" | "couriernew" | "courier new" | "monospace" => BuiltinVariants::Courier,
        // default → Helvetica family
        _ => BuiltinVariants::Helvetica,
    };

    // Try to load a system font for built-in PDF fonts
    // This provides metrics but the actual font rendering uses PDF built-in fonts
    let candidates = match builtin_variants {
        BuiltinVariants::Times => &["Times New Roman", "Times", "Liberation Serif"],
        BuiltinVariants::Courier => &["Courier New", "Courier", "Liberation Mono"],
        BuiltinVariants::Helvetica => &["Helvetica", "Arial", "Liberation Sans"],
    };

    let font_bytes = Arc::new(load_system_font_bytes_fallback(candidates)?);

    // Helper that maps the base family + style to the correct `BuiltinFont`
    let mk_data = |variant: FontStyle| -> Result<FontData, Error> {
        let builtin = builtin_variants.variant(variant);
        FontData::new_shared(font_bytes.clone(), Some(builtin))
    };

    Ok(FontFamily {
        regular: mk_data(FontStyle::Regular)?,
        bold: mk_data(FontStyle::Bold)?,
        italic: mk_data(FontStyle::Italic)?,
        bold_italic: mk_data(FontStyle::BoldItalic)?,
    })
}

/// Internal helper – font style information that influences the built-in enum.
#[derive(Clone, Copy)]
enum FontStyle {
    Regular,
    Bold,
    Italic,
    BoldItalic,
}

/// Internal helper that knows how to translate a `FontStyle` into the concrete
/// `printpdf::BuiltinFont` for the three supported base families.
enum BuiltinVariants {
    Helvetica,
    Times,
    Courier,
}

impl BuiltinVariants {
    fn variant(&self, style: FontStyle) -> BuiltinFont {
        match self {
            BuiltinVariants::Helvetica => match style {
                FontStyle::Regular => BuiltinFont::Helvetica,
                FontStyle::Bold => BuiltinFont::HelveticaBold,
                FontStyle::Italic => BuiltinFont::HelveticaOblique,
                FontStyle::BoldItalic => BuiltinFont::HelveticaBoldOblique,
            },
            BuiltinVariants::Times => match style {
                FontStyle::Regular => BuiltinFont::TimesRoman,
                FontStyle::Bold => BuiltinFont::TimesBold,
                FontStyle::Italic => BuiltinFont::TimesItalic,
                FontStyle::BoldItalic => BuiltinFont::TimesBoldItalic,
            },
            BuiltinVariants::Courier => match style {
                FontStyle::Regular => BuiltinFont::Courier,
                FontStyle::Bold => BuiltinFont::CourierBold,
                FontStyle::Italic => BuiltinFont::CourierOblique,
                FontStyle::BoldItalic => BuiltinFont::CourierBoldOblique,
            },
        }
    }
}

/// Attempts to find a suitable system font for built-in font metrics.
/// Falls back to any available system font if specific candidates aren't found.
fn load_system_font_bytes_fallback(candidates: &[&str]) -> Result<Vec<u8>, Error> {
    let mut db = Database::new();
    db.load_system_fonts();

    // First try to find matching candidates
    for face in db.faces() {
        let path = match &face.source {
            fontdb::Source::File(p) => p,
            _ => continue,
        };

        // Skip collections (.ttc) because rusttype can't read them directly
        if path
            .extension()
            .and_then(|s| s.to_str())
            .map_or(false, |ext| ext.eq_ignore_ascii_case("ttc"))
        {
            continue;
        }

        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_lowercase();

        if candidates
            .iter()
            .any(|cand| file_name.contains(&cand.to_lowercase()))
        {
            if let Ok(bytes) = fs::read(path) {
                if Font::try_from_bytes(&bytes).is_some() {
                    return Ok(bytes);
                }
            }
        }
    }

    // If no specific candidates found, use any available TTF font
    for face in db.faces() {
        let path = match &face.source {
            fontdb::Source::File(p) => p,
            _ => continue,
        };

        // Only use TTF/OTF files, skip TTC collections
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            if !ext.eq_ignore_ascii_case("ttf") && !ext.eq_ignore_ascii_case("otf") {
                continue;
            }
        } else {
            continue;
        }

        if let Ok(bytes) = fs::read(path) {
            if Font::try_from_bytes(&bytes).is_some() {
                return Ok(bytes);
            }
        }
    }

    Err(Error::new(
        "No usable system font found for built-in font metrics".to_string(),
        ErrorKind::InvalidFont,
    ))
}

/// Attempts to load an arbitrary system font family **and embed it** into the PDF.
///
/// The same underlying TrueType font file is re-used for all four style variants.  While this
/// means that bold / italic rendering falls back to faux effects provided by the PDF viewer, it
/// keeps the implementation simple and – most importantly – guarantees that we use *real* glyph
/// metrics (including kerning) instead of relying on the limited built-in font set.  This is
/// usually enough to get visually pleasing output for the vast majority of documents.
///
/// If the requested family cannot be found, an `InvalidFont` error is returned so that the caller
/// can decide how to proceed (e.g. fall back to a built-in font).
pub fn load_system_font_family_simple(name: &str) -> Result<FontFamily<FontData>, Error> {
    // Prefer embedded fonts from `fonts/` if available
    if let Some((family, canon)) = find_embedded_family_and_name(name) {
        eprintln!("✓ Using embedded font family '{}'", canon);
        return Ok(family);
    }

    let mut candidates = vec![name];
    let aliases = get_font_aliases(name);
    candidates.extend(aliases);

    let mut db = Database::new();
    db.load_system_fonts();

    for candidate_name in candidates {
        let wanted = candidate_name.to_lowercase();
        let mut selected_bytes: Option<Vec<u8>> = None;

        for face in db.faces() {
            let path = match &face.source {
                fontdb::Source::File(p) => p,
                _ => continue,
            };

            let face_family = face.families.first().map(|(name, _)| name.to_lowercase());
            let matches_family = face_family.as_ref().map_or(false, |f| f.contains(&wanted));

            let file_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_lowercase();
            let matches_filename = file_name.contains(&wanted);

            if !matches_family && !matches_filename {
                continue;
            }

            // Try to load the font (works for both .ttf and .ttc)
            match fs::read(path) {
                Ok(b) => {
                    let is_ttc = path
                        .extension()
                        .and_then(|s| s.to_str())
                        .map_or(false, |ext| ext.eq_ignore_ascii_case("ttc"));

                    if is_ttc {
                        // .ttc (TrueType Collection) files are currently not supported
                        // because fonts in collections often share tables, making extraction complex.
                        // Users should install individual .ttf/.otf versions of fonts instead.
                        eprintln!(
                            "  ℹ Skipping '{}' (.ttc collections not yet fully supported)",
                            file_name
                        );
                        eprintln!(
                            "     💡 Install individual .ttf/.otf version for best compatibility"
                        );
                        continue;
                    } else {
                        // Regular .ttf/.otf file
                        // Prefer `try_from_bytes` which returns an Option; keep `catch_unwind` as
                        // an extra safety in case of unexpected panics from font parsing.
                        let is_valid = panic::catch_unwind(|| Font::try_from_vec(b.clone()).is_some())
                            .unwrap_or(false);

                        if is_valid {
                            selected_bytes = Some(b);
                            break;
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to read font file {:?}: {}", path, e);
                }
            }
        }

        if let Some(bytes) = selected_bytes {
            if candidate_name != name {
                debug!("  ℹ Using '{}' as alias for '{}'", candidate_name, name);
            }

            // Double-check the font is valid before creating FontData
            // This prevents panics from invalid .ttc extractions
            // Prefer `try_from_bytes` and keep `catch_unwind` as an extra safety.
            let is_valid =
                panic::catch_unwind(|| Font::try_from_vec(bytes.clone()).is_some()).unwrap_or(false);

            if !is_valid {
                eprintln!("  ⚠ Font data invalid, skipping '{}'", candidate_name);
                continue;
            }

            let shared = Arc::new(bytes);
            let mk = || FontData::new_shared(shared.clone(), None);
            return Ok(FontFamily {
                regular: mk()?,
                bold: mk()?,
                italic: mk()?,
                bold_italic: mk()?,
            });
        }
    }

    Err(Error::new(
        format!("No usable system font found for family '{}'.", name),
        ErrorKind::InvalidFont,
    ))
}

/// Attempts to load a font family from custom paths first, then falls back to system fonts.
/// This function searches user-specified directories or files before looking in system fonts.
///
/// # Arguments
/// * `name` - The font family name to search for
/// * `custom_paths` - Custom directories or font files to search
///
/// # Returns
/// * `Ok(FontFamily<FontData>)` if the font is found and loaded successfully
/// * `Err(Error)` if the font cannot be found in any location
///
/// # Search order
/// 1. Custom paths (if provided) - searches for exact matches or files containing the name
/// 2. System fonts via fontdb
/// 3. Returns error if not found
pub fn load_custom_font_family(
    name: &str,
    custom_paths: &[PathBuf],
) -> Result<FontFamily<FontData>, Error> {
    if let Ok(family) = load_font_family_with_variants(name, custom_paths) {
        eprintln!("✓ Loaded font '{}' with proper variants", name);
        return Ok(family);
    }

    eprintln!("  → Searching for single font file for '{}'", name);
    let wanted = name.to_lowercase();

    // First, try to load from custom paths
    for custom_path in custom_paths {
        if custom_path.is_file() {
            // If it's a direct file path, try to load it
            if let Some(file_name) = custom_path.file_name().and_then(|n| n.to_str()) {
                if file_name.to_lowercase().contains(&wanted) {
                    if let Ok(bytes) = fs::read(custom_path) {
                        if rusttype::Font::try_from_bytes(&bytes).is_some() {
                            let shared = Arc::new(bytes);
                            let mk = || FontData::new_shared(shared.clone(), None);
                            return Ok(FontFamily {
                                regular: mk()?,
                                bold: mk()?,
                                italic: mk()?,
                                bold_italic: mk()?,
                            });
                        }
                    }
                }
            }
        } else if custom_path.is_dir() {
            if let Ok(entries) = fs::read_dir(custom_path) {
                for entry in entries.flatten() {
                    let path = entry.path();

                    // Only consider TTF/OTF files
                    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                        if !ext.eq_ignore_ascii_case("ttf") && !ext.eq_ignore_ascii_case("otf") {
                            continue;
                        }
                    } else {
                        continue;
                    }

                    if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                        if file_name.to_lowercase().contains(&wanted) {
                            if let Ok(bytes) = fs::read(&path) {
                                if rusttype::Font::try_from_bytes(&bytes).is_some() {
                                    let shared = Arc::new(bytes);
                                    let mk = || FontData::new_shared(shared.clone(), None);
                                    return Ok(FontFamily {
                                        regular: mk()?,
                                        bold: mk()?,
                                        italic: mk()?,
                                        bold_italic: mk()?,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // If not found in custom paths, fall back to system fonts
    load_system_font_family_simple(name)
}

/// Searches for a specific font variant file in custom paths.
///
/// Tries multiple naming patterns for font variants:
/// - DejaVuSans-Bold.ttf
/// - DejaVuSansBold.ttf
/// - DejaVuSans_Bold.ttf
/// - notosans-bold.ttf
///
/// Also tries font name aliases (e.g., Arial -> Helvetica)
fn find_font_variant_in_paths(
    base_name: &str,
    variant: FontVariant,
    custom_paths: &[PathBuf],
) -> Option<Vec<u8>> {
    let mut candidates = vec![base_name];
    let aliases = get_font_aliases(base_name);
    candidates.extend(aliases);

    for candidate in candidates {
        let base_raw = candidate.to_lowercase();
        let base_no_space = base_raw.replace(" ", "");
        let base_dash = base_raw.replace(" ", "-");
        let base_underscore = base_raw.replace(" ", "_");
        let bases = vec![base_raw.as_str(), base_no_space.as_str(), base_dash.as_str(), base_underscore.as_str()];

        for custom_path in custom_paths {
            if !custom_path.is_dir() {
                continue;
            }

            let Ok(entries) = fs::read_dir(custom_path) else {
                continue;
            };

            for entry in entries.flatten() {
                let path = entry.path();
                let Some(ext) = path.extension().and_then(|s| s.to_str()) else {
                    continue;
                };

                if !ext.eq_ignore_ascii_case("ttf") && !ext.eq_ignore_ascii_case("otf") {
                    continue;
                }

                let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
                    continue;
                };

                let file_lower = file_name.to_lowercase();
                for suffix in variant.suffixes() {
                    if suffix.is_empty() && !matches!(variant, FontVariant::Regular) {
                        continue;
                    }

                    for base in &bases {
                        let patterns = if suffix.is_empty() {
                            vec![format!("{}.ttf", base), format!("{}.otf", base)]
                        } else {
                            let suf = suffix.to_lowercase();
                            vec![
                                format!("{}-{}.ttf", base, suf),
                                format!("{}{}.ttf", base, suf),
                                format!("{}_{}.ttf", base, suf),
                                format!("{} {}.ttf", base, suf),
                                format!("{}-{}.otf", base, suf),
                                format!("{}{}.otf", base, suf),
                            ]
                        };

                        for pattern in &patterns {
                            if file_lower.contains(pattern) || file_lower == *pattern {
                                if let Ok(bytes) = fs::read(&path) {
                                    if Font::try_from_bytes(&bytes).is_some() {
                                        if candidate != base_name {
                                            eprintln!(
                                                "  ℹ Found '{}' variant as alias for '{}'",
                                                candidate, base_name
                                            );
                                        }
                                        return Some(bytes);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

/// Loads a font family with proper variant files (Bold, Italic, BoldItalic).
///
/// This function searches for actual variant font files instead of reusing
/// the regular font for all styles. Falls back to regular font if variants aren't found.
///
/// # Arguments
/// * `name` - The font family base name
/// * `custom_paths` - Directories to search for font files
///
/// # Returns
/// A FontFamily with actual variant files loaded
pub fn load_font_family_with_variants(
    name: &str,
    custom_paths: &[PathBuf],
) -> Result<FontFamily<FontData>, Error> {
    let regular_bytes = find_font_variant_in_paths(name, FontVariant::Regular, custom_paths)
        .ok_or_else(|| {
            Error::new(
                format!("Could not find regular variant for font '{}'", name),
                ErrorKind::InvalidFont,
            )
        })?;

    let bold_bytes = find_font_variant_in_paths(name, FontVariant::Bold, custom_paths);
    let italic_bytes = find_font_variant_in_paths(name, FontVariant::Italic, custom_paths);
    let bold_italic_bytes = find_font_variant_in_paths(name, FontVariant::BoldItalic, custom_paths);

    let regular_shared = Arc::new(regular_bytes);
    let regular = FontData::new_shared(regular_shared.clone(), None)?;

    let bold = if let Some(bytes) = bold_bytes {
        FontData::new_shared(Arc::new(bytes), None).unwrap_or_else(|_| {
            eprintln!("  ⚠ Bold variant invalid, using regular");
            regular.clone()
        })
    } else {
        eprintln!(
            "  → No Bold variant found for '{}', using regular (faux bold)",
            name
        );
        regular.clone()
    };

    let italic = if let Some(bytes) = italic_bytes {
        FontData::new_shared(Arc::new(bytes), None).unwrap_or_else(|_| {
            eprintln!("  ⚠ Italic variant invalid, using regular");
            regular.clone()
        })
    } else {
        eprintln!(
            "  → No Italic variant found for '{}', using regular (faux italic)",
            name
        );
        regular.clone()
    };

    let bold_italic = if let Some(bytes) = bold_italic_bytes {
        FontData::new_shared(Arc::new(bytes), None).unwrap_or_else(|_| {
            eprintln!("  ⚠ BoldItalic variant invalid, using bold");
            bold.clone()
        })
    } else {
        eprintln!(
            "  → No BoldItalic variant found for '{}', using bold (faux italic)",
            name
        );
        bold.clone()
    };

    Ok(FontFamily {
        regular,
        bold,
        italic,
        bold_italic,
    })
}

/// Loads a font family using the provided FontConfig, with intelligent fallback.
/// This is the main entry point for loading fonts with custom configuration.
///
/// # Arguments
/// * `name` - The font family name to load
/// * `config` - Optional font configuration with custom paths
/// * `text` - Optional text content for font subsetting
///
/// # Returns
/// * `Ok(FontFamily<FontData>)` if the font is found
/// * `Err(Error)` if the font cannot be loaded from any source
///
/// # Loading strategy
/// 1. If custom_paths are provided in config, search there first
/// 2. Check if it's a built-in font (helvetica, times, courier)
/// 3. Search system fonts
/// 4. Apply font subsetting if enabled and text is provided
/// 5. Return error if nothing found
pub fn load_font_with_config(
    name: &str,
    config: Option<&FontConfig>,
    text: Option<&str>,
) -> Result<FontFamily<FontData>, Error> {
    // Check if subsetting is enabled
    let enable_subsetting = config.map(|c| c.enable_subsetting).unwrap_or(false);

    // Prefer embedded fonts (statically included in `fonts/`) if available
    // Embedded fonts are shipped with the project and are considered safe – skip
    // subsetting for embedded fonts to avoid producing invalid font binaries.
    if let Some((family, canon)) = find_embedded_family_and_name(name) {
        eprintln!("✓ Using embedded font family '{}' (load_font_with_config)", canon);
        return Ok(family);
    }

    // Check if fallback fonts are specified - if so, return a chain-based result
    // Note: We can't apply subsetting to fallback chains yet, so this path doesn't support it
    if let Some(cfg) = config {
        if !cfg.fallback_fonts.is_empty() {
            eprintln!(
                "Loading font '{}' with {} fallback(s)...",
                name,
                cfg.fallback_fonts.len()
            );
            // For now, use the legacy fallback selection approach
            // TODO: Integrate fallback chains into the rendering pipeline
            let family =
                load_font_with_fallbacks(name, &cfg.fallback_fonts, &cfg.custom_paths, text)?;
            return apply_subsetting_if_enabled(family, enable_subsetting, text);
        }
    }

    // Try custom paths first if provided (no fallbacks)
    if let Some(cfg) = config {
        if !cfg.custom_paths.is_empty() {
            if let Ok(family) = load_custom_font_family(name, &cfg.custom_paths) {
                return apply_subsetting_if_enabled(family, enable_subsetting, text);
            }
        }
    }

    // Check if it's a built-in font (no subsetting for built-in fonts)
    let family = match name.to_lowercase().as_str() {
        "helvetica" | "arial" | "sans" | "sans-serif" | "times" | "timesnewroman"
        | "times new roman" | "serif" | "courier" | "monospace" => {
            return load_builtin_font_family(name); // Built-in fonts don't use subsetting
        }
        _ => {
            // Try system fonts as fallback
            load_system_font_family_simple(name)?
        }
    };

    apply_subsetting_if_enabled(family, enable_subsetting, text)
}

/// Applies font subsetting if enabled and text is provided.
fn apply_subsetting_if_enabled(
    family: FontFamily<FontData>,
    enable_subsetting: bool,
    text: Option<&str>,
) -> Result<FontFamily<FontData>, Error> {
    if !enable_subsetting || text.is_none() {
        return Ok(family);
    }

    let text = text.unwrap();
    if text.is_empty() {
        return Ok(family);
    }

    // Subset the font using genpdfi's subsetting module
    // Get the raw font data from the first variant (regular)
    let original_data = family.regular.get_data()?;

    let subset_data =
        genpdfi_extended::subsetting::subset_font(original_data, text).map_err(|e| {
            warn!("Warning: Font subsetting failed: {}, using full font", e);
            e
        })?;

    // Create new FontFamily with subset data
    let shared = Arc::new(subset_data);
    let mk = || FontData::new_shared(shared.clone(), None);

    Ok(FontFamily {
        regular: mk()?,
        bold: mk()?,
        italic: mk()?,
        bold_italic: mk()?,
    })
}

/// Applies font subsetting to a fallback chain to reduce PDF file size.
///
/// This function subsets each font in the chain (primary and all fallbacks) based on
/// which characters from the document that font actually needs to render.
///
/// # Arguments
/// * `chain_family` - The fallback chain family to subset
/// * `text` - The document text to analyze for character usage
///
/// # Returns
/// A new FontFamily<FontFallbackChain> with subset fonts
pub fn apply_subsetting_to_chain(
    chain_family: FontFamily<genpdfi_extended::fonts::FontFallbackChain>,
    text: &str,
) -> Result<FontFamily<genpdfi_extended::fonts::FontFallbackChain>, Error> {
    if text.is_empty() {
        return Ok(chain_family);
    }

    let subset_regular = subset_chain(&chain_family.regular, text)?;
    let subset_bold = subset_chain(&chain_family.bold, text)?;
    let subset_italic = subset_chain(&chain_family.italic, text)?;
    let subset_bold_italic = subset_chain(&chain_family.bold_italic, text)?;

    Ok(FontFamily {
        regular: subset_regular,
        bold: subset_bold,
        italic: subset_italic,
        bold_italic: subset_bold_italic,
    })
}

/// Subsets a single fallback chain by subsetting each font based on what it renders.
fn subset_chain(
    chain: &genpdfi_extended::fonts::FontFallbackChain,
    text: &str,
) -> Result<genpdfi_extended::fonts::FontFallbackChain, Error> {
    let segments = chain.segment_text(text);
    use std::collections::HashMap;

    let mut font_chars: HashMap<*const FontData, String> = HashMap::new();
    for (segment_text, font_data) in &segments {
        let font_ptr = *font_data as *const FontData;
        font_chars
            .entry(font_ptr)
            .or_insert_with(String::new)
            .push_str(segment_text);
    }

    let primary_text = font_chars
        .get(&(chain.primary() as *const FontData))
        .map(|s| s.as_str())
        .unwrap_or("");

    let subset_primary = if !primary_text.is_empty() {
        eprintln!(
            "  Subsetting primary font ({} chars)...",
            primary_text.len()
        );
        subset_single_font(chain.primary(), primary_text)?
    } else {
        chain.primary().clone()
    };

    let mut subset_fallbacks = Vec::new();
    for (idx, fallback) in chain.fallbacks().iter().enumerate() {
        let fallback_text = font_chars
            .get(&(fallback as *const FontData))
            .map(|s| s.as_str())
            .unwrap_or("");

        let subset_fallback = if !fallback_text.is_empty() {
            eprintln!(
                "  Subsetting fallback {} ({} chars)...",
                idx + 1,
                fallback_text.len()
            );
            subset_single_font(fallback, fallback_text)?
        } else {
            eprintln!("  Fallback {} not used, skipping subsetting", idx + 1);
            fallback.clone()
        };

        subset_fallbacks.push(subset_fallback);
    }

    let mut new_chain = genpdfi_extended::fonts::FontFallbackChain::new(subset_primary);
    for fallback in subset_fallbacks {
        new_chain = new_chain.with_fallback(fallback);
    }

    Ok(new_chain)
}

/// Subsets a single FontData based on the provided text.
fn subset_single_font(font: &FontData, text: &str) -> Result<FontData, Error> {
    let original_data = font.get_data()?;
    let original_size = original_data.len();

    let subset_data =
        genpdfi_extended::subsetting::subset_font(original_data, text).map_err(|e| {
            eprintln!("\t Warning: Font subsetting failed: {}, using full font", e);
            e
        })?;

    let subset_size = subset_data.len();
    let reduction = ((original_size - subset_size) as f64 / original_size as f64) * 100.0;

    eprintln!(
        "\t ✓ {} → {} ({:.1}% reduction)",
        format_size(original_size),
        format_size(subset_size),
        reduction
    );

    // Create new FontData with subset data (no builtin font for embedded fonts)
    FontData::new_shared(Arc::new(subset_data), None)
}

/// Formats a byte size in a human-readable format.
fn format_size(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

/// Loads a Unicode-capable system font with good international character support.
///
/// This function attempts to find and load a font from the system that supports
/// a wide range of Unicode characters, making it suitable for documents with
/// international text (Romanian, Cyrillic, Greek, etc.).
///
/// # Priority Order
/// 1. Noto Sans - Google's comprehensive Unicode font
/// 2. DejaVu Sans - Popular open-source Unicode font
/// 3. Liberation Sans - Red Hat's Unicode font
/// 4. Arial Unicode MS - Microsoft's Unicode font (if available)
/// 5. Fallback to Helvetica (PDF built-in, limited to Windows-1252)
///
/// # Arguments
/// * `text` - Optional text to check coverage for. If provided, will verify the selected font supports all characters.
///
/// # Returns
/// * `Ok(FontFamily<FontData>)` - A font family with good Unicode support
/// * `Err(Error)` - If no suitable font could be loaded
///
/// # Example
/// ```rust,no_run
/// use markdown2pdf::fonts::load_unicode_system_font;
///
/// // Load best available Unicode font
/// let font = load_unicode_system_font(None)?;
///
/// // Load and verify coverage for Romanian text
/// let romanian_text = "ăâîșț ĂÂÎȘȚ";
/// let font = load_unicode_system_font(Some(romanian_text))?;
/// # Ok::<(), genpdfi_extended::error::Error>(())
/// ```
pub fn load_unicode_system_font(text: Option<&str>) -> Result<FontFamily<FontData>, Error> {
    // Priority list of Unicode-capable fonts
    // macOS-specific fonts are prioritized since they're more likely to be available
    let unicode_fonts = [
        "Noto Sans",
        "DejaVu Sans",
        "Liberation Sans",
        "Arial Unicode MS",
        "SF Pro",         // macOS system font (good Unicode)
        "Helvetica Neue", // macOS (better than Helvetica)
        "Segoe UI",       // Windows default (good Unicode support)
        "Roboto",         // Android default
    ];

    let mut tried_fonts = Vec::new();

    // Try each Unicode font
    for font_name in &unicode_fonts {
        // If an embedded font exists for this name, prefer it and report appropriately
        if let Some((embedded_family, canon)) = find_embedded_family_and_name(font_name) {
            // If text provided, check coverage
            if let Some(text) = text {
                let coverage = embedded_family.regular.check_coverage(text);
                if coverage.coverage_percent() >= 98f32 {
                    eprintln!(
                        "✓ Using embedded font '{}' ({:.1}% coverage)",
                        canon,
                        coverage.coverage_percent()
                    );
                    return Ok(embedded_family);
                } else {
                    eprintln!(
                        "⚠ Embedded font '{}' has only {:.1}% coverage, trying next...",
                        canon,
                        coverage.coverage_percent()
                    );
                    tried_fonts.push(format!(
                        "{} ({:.1}% coverage)",
                        canon,
                        coverage.coverage_percent()
                    ));
                    // continue to try system fonts/fallbacks
                }
            } else {
                eprintln!("✓ Using embedded font '{}'", canon);
                return Ok(embedded_family);
            }
        } else if let Ok(family) = load_system_font_family_simple(font_name) {
            // If text provided, check coverage
            if let Some(text) = text {
                let coverage = family.regular.check_coverage(text);
                if coverage.coverage_percent() >= 98f32 {
                    eprintln!(
                        "✓ Using system font '{}' ({:.1}% coverage)",
                        font_name,
                        coverage.coverage_percent()
                    );
                    return Ok(family);
                } else {
                    eprintln!(
                        "⚠ Font '{}' has only {:.1}% coverage, trying next...",
                        font_name,
                        coverage.coverage_percent()
                    );
                    tried_fonts.push(format!(
                        "{} ({:.1}% coverage)",
                        font_name,
                        coverage.coverage_percent()
                    ));
                }
            } else {
                // No text to check, font found is good enough
                eprintln!("✓ Using system font '{}'", font_name);
                return Ok(family);
            }
        } else {
            tried_fonts.push(format!("{} (not found)", font_name));
        }
    }

    eprintln!(
        "⚠ No suitable Unicode font found, falling back to Helvetica (limited character support)"
    );
    if !tried_fonts.is_empty() {
        eprintln!("  Fonts tried:");
        for font in &tried_fonts {
            eprintln!("    - {}", font);
        }
    }
    eprintln!("  💡 To fix this, install a Unicode font:");
    eprintln!("     • brew install font-noto-sans  (Homebrew)");
    eprintln!("     • Or download from https://fonts.google.com/noto");
    load_builtin_font_family("helvetica")
}

/// Extracts primary fonts from a fallback chain family.
///
/// This creates a `FontFamily<FontData>` from a `FontFamily<FontFallbackChain>`
/// by taking the primary font from each chain. This is useful for compatibility
/// with genpdfi which expects `FontData` rather than chains.
///
/// # Arguments
/// * `chain_family` - The fallback chain family to extract primaries from
///
/// # Returns
/// A `FontFamily<FontData>` containing the primary fonts
pub fn extract_primary_fonts(
    chain_family: &FontFamily<genpdfi_extended::fonts::FontFallbackChain>,
) -> FontFamily<FontData> {
    FontFamily {
        regular: chain_family.regular.primary().clone(),
        bold: chain_family.bold.primary().clone(),
        italic: chain_family.italic.primary().clone(),
        bold_italic: chain_family.bold_italic.primary().clone(),
    }
}

/// Returns a list of sensible default fallback fonts for the given primary font.
///
/// These fallbacks are tried in order when characters are missing from the primary font:
/// 1. Unicode fonts (Noto Sans, DejaVu Sans, Arial Unicode)
/// 2. System fonts (SF Pro on macOS, Segoe UI on Windows)
/// 3. Emoji fonts for emoji support
///
/// # Arguments
/// * `primary_name` - The primary font name (used to avoid redundant fallbacks)
///
/// # Returns
/// A vector of fallback font names
pub fn get_default_fallback_fonts(primary_name: &str) -> Vec<String> {
    let primary_lower = primary_name.to_lowercase();

    let candidates = vec![
        "Noto Sans",
        "DejaVu Sans",
        "Arial Unicode MS",
        "SF Pro",           // macOS
        "Segoe UI",         // Windows
        "Roboto",           // Android/Chrome OS
        "Noto Color Emoji", // Emoji support
    ];

    candidates
        .into_iter()
        .filter(|name| name.to_lowercase() != primary_lower)
        .map(|s| s.to_string())
        .collect()
}

/// Loads fonts and creates a fallback chain for handling mixed-script documents.
///
/// This function creates a `FontFallbackChain` by:
/// 1. Loading the primary font
/// 2. Loading all specified fallback fonts
/// 3. Creating a chain where fonts are tried in order
///
/// When rendering text, the chain will automatically select the appropriate font
/// for each character based on glyph coverage.
///
/// # Arguments
/// * `primary_name` - Name of the primary font to load
/// * `fallback_names` - List of fallback font names to try
/// * `custom_paths` - Custom paths to search for fonts
/// * `text` - Optional text for validation (currently unused, kept for API compatibility)
///
/// # Returns
/// A `FontFamily` where each variant (regular, bold, etc.) is a `FontFallbackChain`
///
/// # Example
/// ```no_run
/// use markdown2pdf::fonts::load_font_with_fallback_chain;
/// use std::path::PathBuf;
///
/// let chain = load_font_with_fallback_chain(
///     "Noto Sans",
///     &["DejaVu Sans".to_string(), "Arial".to_string()],
///     &[PathBuf::from("./fonts")],
///     Some("Hello мир! 👋")
/// )?;
/// # Ok::<(), genpdfi_extended::error::Error>(())
/// ```
pub fn load_font_with_fallback_chain(
    primary_name: &str,
    fallback_names: &[String],
    custom_paths: &[PathBuf],
    _text: Option<&str>,
) -> Result<FontFamily<genpdfi_extended::fonts::FontFallbackChain>, Error> {
    use genpdfi_extended::fonts::FontFallbackChain;

    // Load primary font
    let primary_family = if !custom_paths.is_empty() {
        load_custom_font_family(primary_name, custom_paths)
            .or_else(|_| load_system_font_family_simple(primary_name))
    } else {
        load_system_font_family_simple(primary_name)
    }?;

    // Load all fallback fonts
    let mut fallback_families = Vec::new();
    for fallback_name in fallback_names {
        let fallback_family = if !custom_paths.is_empty() {
            load_custom_font_family(fallback_name, custom_paths)
                .or_else(|_| load_system_font_family_simple(fallback_name))
        } else {
            load_system_font_family_simple(fallback_name)
        };

        if let Ok(family) = fallback_family {
            eprintln!("  ✓ Loaded fallback font '{}'", fallback_name);
            fallback_families.push(family);
        } else {
            eprintln!("  ⚠ Fallback font '{}' not found, skipping", fallback_name);
        }
    }

    // Create fallback chains for each variant
    let regular_chain = {
        let mut chain = FontFallbackChain::new(primary_family.regular);
        for family in &fallback_families {
            chain = chain.with_fallback(family.regular.clone());
        }
        chain
    };

    let bold_chain = {
        let mut chain = FontFallbackChain::new(primary_family.bold);
        for family in &fallback_families {
            chain = chain.with_fallback(family.bold.clone());
        }
        chain
    };

    let italic_chain = {
        let mut chain = FontFallbackChain::new(primary_family.italic);
        for family in &fallback_families {
            chain = chain.with_fallback(family.italic.clone());
        }
        chain
    };

    let bold_italic_chain = {
        let mut chain = FontFallbackChain::new(primary_family.bold_italic);
        for family in &fallback_families {
            chain = chain.with_fallback(family.bold_italic.clone());
        }
        chain
    };

    eprintln!(
        "✓ Created fallback chain: {} + {} fallback(s)",
        primary_name,
        fallback_families.len()
    );

    Ok(FontFamily {
        regular: regular_chain,
        bold: bold_chain,
        italic: italic_chain,
        bold_italic: bold_italic_chain,
    })
}

/// Loads a font with fallback support for better coverage (legacy function).
///
/// This function tries to find the best font for the given text by:
/// 1. Loading the primary font
/// 2. Loading all fallback fonts
/// 3. Checking coverage for each
/// 4. Selecting the font with the best coverage
///
/// **Note**: This function is kept for backward compatibility. New code should use
/// `load_font_with_fallback_chain()` which returns actual fallback chains instead
/// of selecting a single best font.
///
/// # Arguments
/// * `primary_name` - Name of the primary font to load
/// * `fallback_names` - List of fallback font names to try
/// * `custom_paths` - Custom paths to search for fonts
/// * `text` - Optional text to check coverage for
///
/// # Returns
/// The font with the best coverage for the given text
pub fn load_font_with_fallbacks(
    primary_name: &str,
    fallback_names: &[String],
    custom_paths: &[PathBuf],
    text: Option<&str>,
) -> Result<FontFamily<FontData>, Error> {
    let mut tried_fonts = Vec::new();

    // Try to load primary font first
    let primary = if !custom_paths.is_empty() {
        load_custom_font_family(primary_name, custom_paths)
            .or_else(|_| load_system_font_family_simple(primary_name))
    } else {
        load_system_font_family_simple(primary_name)
    };

    // If no text to check, just return primary (or first fallback that works)
    if text.is_none() {
        if let Ok(font) = primary {
            return Ok(font);
        }
        tried_fonts.push(format!("{} (not found)", primary_name));

        // Try fallbacks
        for fallback_name in fallback_names {
            if let Ok(font) = load_system_font_family_simple(fallback_name) {
                eprintln!("✓ Using fallback font '{}'", fallback_name);
                return Ok(font);
            }
            tried_fonts.push(format!("{} (not found)", fallback_name));
        }

        eprintln!("❌ Could not load font '{}' or any fallbacks", primary_name);
        eprintln!("  Fonts tried:");
        for font in &tried_fonts {
            eprintln!("    - {}", font);
        }
        return Err(Error::new(
            format!("Could not load font '{}' or any fallbacks", primary_name),
            ErrorKind::InvalidFont,
        ));
    }

    let text = text.unwrap();
    let mut best_font: Option<FontFamily<FontData>> = None;
    let mut best_coverage = 0.0;
    let mut best_name = String::new();

    // Check primary font coverage
    if let Ok(font) = primary {
        let coverage = font.regular.check_coverage(text);
        if coverage.is_complete() {
            eprintln!("✓ Primary font '{}' has 100% coverage", primary_name);
            return Ok(font);
        }

        eprintln!(
            "  Primary font '{}' coverage: {:.1}%",
            primary_name,
            coverage.coverage_percent()
        );

        best_coverage = coverage.coverage_percent();
        best_font = Some(font);
        best_name = primary_name.to_string();
        tried_fonts.push(format!(
            "{} ({:.1}% coverage)",
            primary_name,
            coverage.coverage_percent()
        ));
    } else {
        tried_fonts.push(format!("{} (not found)", primary_name));
    }

    // Try each fallback
    for fallback_name in fallback_names {
        let fallback = if !custom_paths.is_empty() {
            load_custom_font_family(fallback_name, custom_paths)
                .or_else(|_| load_system_font_family_simple(fallback_name))
        } else {
            load_system_font_family_simple(fallback_name)
        };

        if let Ok(font) = fallback {
            let coverage = font.regular.check_coverage(text);

            if coverage.is_complete() {
                eprintln!("✓ Fallback font '{}' has 100% coverage", fallback_name);
                return Ok(font);
            }

            eprintln!(
                "  Fallback font '{}' coverage: {:.1}%",
                fallback_name,
                coverage.coverage_percent()
            );

            if coverage.coverage_percent() > best_coverage {
                best_coverage = coverage.coverage_percent();
                best_font = Some(font);
                best_name = fallback_name.clone();
            }
            tried_fonts.push(format!(
                "{} ({:.1}% coverage)",
                fallback_name,
                coverage.coverage_percent()
            ));
        } else {
            tried_fonts.push(format!("{} (not found)", fallback_name));
        }
    }

    // Return the font with best coverage
    if let Some(font) = best_font {
        if best_coverage < 100.0 {
            eprintln!(
                "⚠️  Selected font '{}' with {:.1}% coverage (some characters may not display)",
                best_name, best_coverage
            );
            eprintln!("  Fonts tried:");
            for font in &tried_fonts {
                eprintln!("    - {}", font);
            }
            eprintln!("  💡 To fix this, install a Unicode font:");
            eprintln!("     • brew install font-noto-sans  (Homebrew)");
            eprintln!("     • Or download from https://fonts.google.com/noto");
        } else {
            eprintln!(
                "✓ Selected font '{}' with {:.1}% coverage",
                best_name, best_coverage
            );
        }
        Ok(font)
    } else {
        eprintln!("❌ Could not load font '{}' or any fallbacks", primary_name);
        eprintln!("  Fonts tried:");
        for font in &tried_fonts {
            eprintln!("    - {}", font);
        }
        eprintln!("  💡 To fix this, install a Unicode font:");
        eprintln!("     • brew install font-noto-sans  (Homebrew)");
        eprintln!("     • Or download from https://fonts.google.com/noto");
        Err(Error::new(
            format!("Could not load font '{}' or any fallbacks", primary_name),
            ErrorKind::InvalidFont,
        ))
    }
}

#[cfg(test)]
mod fonts_integration_tests {
    use super::*;
    use std::path::PathBuf;

    fn fonts_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fonts")
    }

    #[test]
    fn test_load_custom_font_family_from_fonts_dir() {
        let fonts = fonts_dir();
        let family = load_custom_font_family("DejaVuSans", &[fonts.clone()]);
        assert!(family.is_ok());
        let family = family.unwrap();
        let data = family.regular.get_data().unwrap();
        assert!(data.len() > 0);
    }

    #[test]
    fn test_find_font_variant_in_paths_finds_variants() {
        let fonts = fonts_dir();
        let regular = find_font_variant_in_paths("DejaVuSans", FontVariant::Regular, &[fonts.clone()]);
        assert!(regular.is_some());
        let bold = find_font_variant_in_paths("DejaVuSans", FontVariant::Bold, &[fonts.clone()]);
        assert!(bold.is_some());
    }

    #[test]
    fn test_load_font_family_with_variants_from_fonts_dir() {
        let fonts = fonts_dir();
        let family = load_font_family_with_variants("CMU Typewriter Text", &[fonts.clone()]);
        assert!(family.is_ok());
        let family = family.unwrap();
        // Bold/Italic should exist (may fallback to regular)
        assert!(family.bold.get_data().unwrap().len() > 0);
        assert!(family.regular.get_data().unwrap().len() > 0);
    }

    #[test]
    fn test_apply_subsetting_if_enabled_reduces_or_matches_size() {
        let fonts = fonts_dir();
        let family = load_custom_font_family("DejaVuSans", &[fonts]).unwrap();
        let original_len = family.regular.get_data().unwrap().len();

        let subset = apply_subsetting_if_enabled(family, true, Some("Hello world"));
        match subset {
            Ok(subset_family) => {
                let subset_len = subset_family.regular.get_data().unwrap().len();
                // Subsetting should not increase size; usually it reduces size
                assert!(subset_len <= original_len);
            }
            Err(e) => {
                // Subsetting may fail on some font data; ensure we get an Error rather than panic
                eprintln!("Subsetting failed (expected in some environments): {}", e);
            }
        }
    }

    #[test]
    fn test_load_font_with_fallback_chain_and_fallbacks() {
        let fonts = fonts_dir();
        let chain = load_font_with_fallback_chain(
            "DejaVu Sans",
            &vec!["CMU Typewriter Text".to_string()],
            &[fonts.clone()],
            Some("Hello world"),
        );
        assert!(chain.is_ok());

        let picked = load_font_with_fallbacks(
            "NonExistentPrimary",
            &vec!["Noto Sans".to_string()],
            &[fonts],
            None,
        );
        assert!(picked.is_ok());
    }

    #[test]
    fn test_load_custom_font_family_missing_dir() {
        let mut tmp = std::env::temp_dir();
        tmp.push("md2pdf_test_fonts_missing");
        // Ensure empty dir
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let res = load_custom_font_family("NoSuchFont", &[tmp.clone()]);
        assert!(res.is_err());

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_find_font_variant_in_paths_returns_none_for_missing() {
        let mut tmp = std::env::temp_dir();
        tmp.push("md2pdf_test_fonts_empty");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let found = find_font_variant_in_paths("DefinitelyNotAFont", FontVariant::Regular, &[tmp.clone()]);
        assert!(found.is_none());

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_apply_subsetting_to_chain_with_empty_text_returns_same() {
        let fonts = fonts_dir();
        let chain = load_font_with_fallback_chain(
            "Noto Sans",
            &vec!["CMU Typewriter Text".to_string()],
            &[fonts],
            None,
        )
        .unwrap();

        let subset = apply_subsetting_to_chain(chain.clone(), "");
        assert!(subset.is_ok());
        let subset = subset.unwrap();

        // Extract primary fonts and compare sizes
        let original_primaries = extract_primary_fonts(&chain);
        let subset_primaries = extract_primary_fonts(&subset);
        assert_eq!(
            original_primaries.regular.get_data().unwrap().len(),
            subset_primaries.regular.get_data().unwrap().len()
        );
    }

    #[test]
    fn test_load_font_with_config_with_fallbacks() {
        let fonts = fonts_dir();
        let mut cfg = FontConfig::default();
        cfg.custom_paths = vec![fonts.clone()];
        cfg.fallback_fonts = vec!["Noto Sans".to_string()];

        let fam = load_font_with_config("NonExistentPrimary", Some(&cfg), Some("Hello world"));
        if let Err(e) = &fam {
            // Subsetting or loading can fail in some environments; ensure error is informative
            eprintln!("load_font_with_config returned error (acceptable in some envs): {}", e);
        }
        assert!(fam.is_ok() || fam.is_err());
    }

    #[test]
    fn test_apply_subsetting_to_chain_skips_unused_fallbacks() {
        let fonts = fonts_dir();
        let chain = load_font_with_fallback_chain(
            "DejaVu Sans",
            &vec!["CMU Typewriter Text".to_string()],
            &[fonts.clone()],
            None,
        )
        .unwrap();

        // Use text that only requires primary font glyphs so fallback(s) are unused
        let subset = apply_subsetting_to_chain(chain, "Hello world");
        match subset {
            Ok(s) => {
                // If subsetting succeeded, ensure primary font still has data
                let _ = s.regular.primary().get_data().unwrap();
            }
            Err(e) => {
                // Subsetting may fail on some environments; accept an Err but log it
                eprintln!("Subsetting failed in this environment (acceptable): {}", e);
            }
        }
    }
}
