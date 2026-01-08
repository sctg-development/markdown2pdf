/// Return default monospace fallback fonts for code rendering.
/// Prefer comprehensive Unicode monospace or nearby families available in the repo.
pub fn get_default_code_font_fallbacks(_primary_name: &str) -> Vec<String> {
    vec![
        "DejaVu Sans Mono".to_string(),
        "DejaVu Sans".to_string(),
        "CMU Typewriter Text".to_string(),
        "DejaVu Serif".to_string(),
        "Courier New".to_string(),
        "Courier".to_string(),
    ]
}
