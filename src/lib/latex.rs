//! LaTeX to SVG rendering module.
//!
//! This module provides functionality to convert LaTeX mathematical expressions
//! into SVG format using the microtex_rs library. It handles both inline ($...$)
//! and display ($$...$$) mathematical content.
//!
//! # Examples
//!
//! ```rust
//! use markdown2pdf::latex::latex_to_svg;
//!
//! // Render an inline mathematical expression
//! let svg = latex_to_svg("E = mc^2", false);
//! assert!(svg.is_ok());
//! ```
//!
//! Note: Full LaTeX rendering examples are excluded from doctests due to
//! underlying C++ dependencies that may cause runtime crashes in test environment.

use microtex_rs::{MicroTex, RenderConfig};
use once_cell::sync::OnceCell;
use std::sync::Mutex;

static MICRO_TEX: OnceCell<Mutex<MicroTex>> = OnceCell::new();

/// Converts a LaTeX mathematical expression to SVG format.
///
/// This function takes a LaTeX expression string and renders it as an SVG image
/// using the MicroTeX library. It supports both inline and display modes, which
/// affects the size and spacing of the mathematical expression.
///
/// # Arguments
/// * `latex_content` - The LaTeX mathematical expression to render (without delimiters)
/// * `display` - If true, uses display mode ($$...$$); if false, uses inline mode ($...$)
///
/// # Returns
/// * `Ok(String)` - The SVG string representation of the rendered LaTeX
/// * `Err(String)` - An error message if rendering fails
///
/// # Examples
///
/// ```rust
/// use markdown2pdf::latex::latex_to_svg;
///
/// // Inline math
/// let result = latex_to_svg("x^2 + y^2 = z^2", false);
/// assert!(result.is_ok());
///
/// // Display math (typically larger)
/// let result = latex_to_svg("\\int_0^\\infty e^{-x} dx", true);
/// assert!(result.is_ok());
/// ```
///
/// Note: Full LaTeX rendering examples are marked with `ignore` due to underlying
/// C++ dependencies that may cause crashes in test environments.
pub fn latex_to_svg(latex_content: &str, display: bool) -> Result<String, String> {
    // Trim whitespace
    let latex = latex_content.trim();

    // Validate that content is not empty
    if latex.is_empty() {
        return Err("LaTeX content is empty".to_string());
    }

    // Wrap LaTeX in appropriate delimiters
    // MicroTeX expects display math in \[...\] format and inline in $...$ format
    let latex_with_delimiters = if display {
        format!("\\[{}\\]", latex)
    } else {
        // Use single-dollar inline math
        format!("${}$", latex)
    }; 

    // Initialize render configuration with defaults
    let config = RenderConfig::default();

    // Initialize or get the global MicroTeX renderer (singleton) and lock it.
    let renderer_mutex = MICRO_TEX.get_or_try_init(|| {
        MicroTex::new()
            .map(Mutex::new)
            .map_err(|e| format!("Failed to initialize MicroTeX: {}", e))
    })?;

    let renderer = renderer_mutex
        .lock()
        .map_err(|e| format!("Failed to lock MicroTeX renderer: {}", e))?;

    // Render LaTeX to SVG using the shared renderer
    renderer
        .render(&latex_with_delimiters, &config)
        .map_err(|e| format!("Failed to render LaTeX: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: MicroTeX tests that actually render LaTeX can be unstable due to
    // underlying C++ dependencies. We test the wrapper logic but avoid rendering
    // full LaTeX expressions in the test suite.

    #[test]
    fn test_latex_to_svg_empty_content() {
        let result = latex_to_svg("", false);
        assert!(result.is_err());
    }

    #[test]
    fn test_latex_to_svg_whitespace_only() {
        let result = latex_to_svg("   \n  ", false);
        assert!(result.is_err());
    }

    #[test]
    #[ignore]
    fn test_multiple_latex_renders() {
        // This test is ignored by default because it depends on MicroTeX native deps.
        let exprs = vec![
            ("H(s) = \\prod_{i=1}^{n/2} \\frac{1}{s^2 + \\frac{\\omega_0}{Q_i}s + \\omega_0^2}", true),
            ("\\Delta f = \\frac{f_s}{N}", true),
            ("\\Delta f = \\frac{48000}{4096} \\approx 11.7 \\text{ Hz}", true),
            ("f_{peak} = f_k + \\frac{\\delta f}{2} \\cdot \\frac{m_{k-1} - m_{k+1}}{m_{k-1} - 2m_k + m_{k+1}}", true),
            ("C = a_0 + a_1 \\cdot S + a_2 \\cdot S^2 + a_3 \\cdot S^3 + a_4 \\cdot S^4", true),
        ];

        for (s, display) in exprs {
            if let Err(e) = super::latex_to_svg(s, display) {
                panic!("Failed to render {}: {}", s, e);
            }
        }
    }
}
