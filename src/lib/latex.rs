//! LaTeX to SVG rendering module.
//!
//! This module provides functionality to convert LaTeX mathematical expressions
//! into SVG format using the microtex_rs library. It handles both inline ($...$)
//! and display ($$...$$) mathematical content.
//!
//! # Examples
//!
//! ```ignore
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
/// ```ignore
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
        format!("${{{}}}", latex)
    };

    // Create a MicroTeX instance
    let renderer = MicroTex::new().map_err(|e| format!("Failed to initialize MicroTeX: {}", e))?;

    // Create render configuration with defaults
    let config = RenderConfig::default();

    // Render LaTeX to SVG
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
}
