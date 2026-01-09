use genpdfi_extended::fonts::{FontData, FontFamily};

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
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/fonts/DejaVuSansMono.ttf"
            )),
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/fonts/DejaVuSansMono-Bold.ttf"
            )),
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/fonts/DejaVuSansMono-Oblique.ttf"
            )),
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/fonts/DejaVuSansMono-BoldOblique.ttf"
            )),
        );
    }

    // DejaVu Sans (sans serif)
    if l.contains("dejavu") && l.contains("sans") {
        return family_from_embedded(
            include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/fonts/DejaVuSans.ttf")),
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/fonts/DejaVuSans-Bold.ttf"
            )),
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/fonts/DejaVuSans-Oblique.ttf"
            )),
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/fonts/DejaVuSans-BoldOblique.ttf"
            )),
        );
    }

    // DejaVu Serif
    if l.contains("dejavu") && l.contains("serif") {
        return family_from_embedded(
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/fonts/DejaVuSerif.ttf"
            )),
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/fonts/DejaVuSerif-Bold.ttf"
            )),
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/fonts/DejaVuSerif-Italic.ttf"
            )),
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/fonts/DejaVuSerif-BoldItalic.ttf"
            )),
        );
    }

    // CMU Typewriter (monospace serif-ish)
    if l.contains("cmu") || l.contains("typewriter") {
        return family_from_embedded(
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/fonts/CMU Typewriter Text Regular.ttf"
            )),
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/fonts/CMU Typewriter Text Bold.ttf"
            )),
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/fonts/CMU Typewriter Text Italic.ttf"
            )),
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/fonts/CMU Typewriter Text Bold Italic.ttf"
            )),
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
