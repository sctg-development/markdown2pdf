use std::path::PathBuf;

#[test]
fn space_mono_contains_box_drawing_glyphs() {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("fonts/DejaVuSansMono.ttf");
    assert!(p.exists(), "DejaVuSansMono.ttf not found in fonts/");

    let data = std::fs::read(&p).expect("Failed to read DejaVu Sans Mono font file");
    let fd = genpdfi_extended::fonts::FontData::new(data, None).expect("Failed to create FontData");

    let chars = ['├', '└', '│', '─'];
    for ch in &chars {
        assert!(
            fd.has_glyph(*ch),
            "Expected Space Mono to have glyph for '{}', but it does not",
            ch
        );
    }
}
