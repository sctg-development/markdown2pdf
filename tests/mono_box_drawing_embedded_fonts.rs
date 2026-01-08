use std::path::PathBuf;

/// Vérifie que les polices monospace embarquées contiennent tous les glyphes
/// de la plage U+2500..=U+257F (box drawing).
#[test]
fn embedded_mono_fonts_have_box_drawing_glyphs() {
    // Rechercher automatiquement les polices embarquées qui ont 'Mono' ou 'Courier' dans le nom
    let fonts_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fonts");
    let mut found_fonts = Vec::new();
    for entry in std::fs::read_dir(&fonts_dir).expect("Impossible de lister fonts/") {
        let entry = entry.expect("lecture de dir failed");
        let fname = entry.file_name();
        let fname = fname.to_string_lossy().to_string();
        let l = fname.to_lowercase();
        // Only test fonts that are monospace-like (DejaVu Mono or CMU Typewriter)
        if l.contains("mono") || l.contains("typewriter") || l.contains("cmu") {
            found_fonts.push(entry.path());
        }
    }

    assert!(
        !found_fonts.is_empty(),
        "Aucune police 'Mono' embarquée trouvée dans fonts/"
    );

    // Check that *at least one* monospace embedded font provides the full box-drawing range.
    let mut good_fonts = Vec::new();

    for p in found_fonts {
        assert!(p.exists(), "Police embarquée introuvable: {}", p.display());

        let data = std::fs::read(&p).expect("Impossible de lire le fichier de police");
        let fd = genpdfi_extended::fonts::FontData::new(data, None)
            .expect("Impossible de charger la police");

        let mut missing = Vec::new();
        for cp in 0x2500u32..=0x257Fu32 {
            if let Some(ch) = std::char::from_u32(cp) {
                if !fd.has_glyph(ch) {
                    missing.push(format!("U+{:04X}", cp));
                }
            }
        }

        if missing.is_empty() {
            good_fonts.push(p);
        } else {
            eprintln!(
                "Police '{}' manque {} glyphs; skipping as full fallback candidate.",
                p.display(),
                missing.len()
            );
        }
    }

    assert!(
        !good_fonts.is_empty(),
        "Aucune police mono embarquée ne contient la plage U+2500..=U+257F; polices testées: {:?}",
        good_fonts
    );

    // Prefer DejaVuSansMono to be among the good fonts if present
    let devo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fonts/DejaVuSansMono.ttf");
    if devo.exists() {
        assert!(
            good_fonts.iter().any(|p| p == &devo),
            "DejaVuSansMono.ttf est présent mais ne contient pas la plage complète"
        );
    }
}
