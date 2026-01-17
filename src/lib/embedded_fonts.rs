use genpdfi_extended::fonts::{FontData, FontFamily};

use crate::fonts::{
    MONO_SANS_BOLD, MONO_SANS_BOLD_ITALIC, MONO_SANS_ITALIC, MONO_SANS_REGULAR, MONO_SERIF_BOLD,
    MONO_SERIF_BOLD_ITALIC, MONO_SERIF_ITALIC, MONO_SERIF_REGULAR, SANS_BOLD, SANS_BOLD_ITALIC,
    SANS_ITALIC, SANS_REGULAR, SERIF_BOLD, SERIF_BOLD_ITALIC, SERIF_ITALIC, SERIF_REGULAR,
};

/// Try to construct an embedded `FontFamily<FontData>` from bundled TTF bytes.
/// Returns `Some(family)` on success, `None` on failure.
fn family_from_embedded(
    reg: &'static [u8],
    bold: &'static [u8],
    italic: &'static [u8],
    bold_italic: &'static [u8],
) -> Option<FontFamily<FontData>> {
    let reg_fd = FontData::new(reg.to_vec(), None).ok()?;
    let bold_fd = FontData::new(bold.to_vec(), None).ok()?;
    let italic_fd = FontData::new(italic.to_vec(), None).ok()?;
    let bold_italic_fd = FontData::new(bold_italic.to_vec(), None).ok()?;
    Some(FontFamily {
        regular: reg_fd,
        bold: bold_fd,
        italic: italic_fd,
        bold_italic: bold_italic_fd,
    })
}

/// Try to return an embedded font family matching `name` (case-insensitive).
/// Known names include variants like "dejavu sans mono", "dejavu sans", "dejavu serif",
/// and "cmu typewriter". Returns `None` if the name is unrecognized or loading fails.
pub fn try_embedded_font_family(name: &str) -> Option<FontFamily<FontData>> {
    let l = name.to_ascii_lowercase();

    // DejaVu Sans Mono (monospace)
    if l.contains("dejavu") && l.contains("mono") {
        return family_from_embedded(
            MONO_SANS_REGULAR,
            MONO_SANS_BOLD,
            MONO_SANS_ITALIC,
            MONO_SANS_BOLD_ITALIC,
        );
    }

    // DejaVu Sans (sans serif)
    if l.contains("dejavu") && l.contains("sans") {
        return family_from_embedded(SANS_REGULAR, SANS_BOLD, SANS_ITALIC, SANS_BOLD_ITALIC);
    }

    // DejaVu Serif
    if l.contains("dejavu") && l.contains("serif") {
        return family_from_embedded(SERIF_REGULAR, SERIF_BOLD, SERIF_ITALIC, SERIF_BOLD_ITALIC);
    }

    // CMU Typewriter (monospace serif-ish)
    if l.contains("cmu") || l.contains("typewriter") {
        return family_from_embedded(
            MONO_SERIF_REGULAR,
            MONO_SERIF_BOLD,
            MONO_SERIF_ITALIC,
            MONO_SERIF_BOLD_ITALIC,
        );
    }

    None
}

/// Lists canonical names this helper recognizes (useful for tests and debug).
pub fn known_embedded_families() -> Vec<&'static str> {
    vec![
        "DejaVu Sans Mono",
        "DejaVu Sans",
        "DejaVu Serif",
        "CMU Typewriter Text",
    ]
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
    assert_eq!(family.regular.get_data().unwrap().len(), SANS_REGULAR.len());
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
