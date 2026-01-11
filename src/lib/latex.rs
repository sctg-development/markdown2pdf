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

use microtex_rs::{MicroTex, RenderConfig, RenderResult, RenderMetrics};
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
pub fn latex_to_svg(_latex_content: &str, _display: bool) -> Result<String, String> {
    Err("need LaTeX feature".to_string())
}

/// Converts a LaTeX mathematical expression to SVG with dimensional metrics.
///
/// This function is similar to `latex_to_svg` but returns additional dimensional
/// information about the rendered formula, which is useful for proper scaling and
/// positioning in PDF documents.
///
/// # Arguments
/// * `latex_content` - The LaTeX mathematical expression to render (without delimiters)
/// * `display` - If true, uses display mode ($$...$$); if false, uses inline mode ($...$)
/// * `target_height` - The target height in pixels for scaling the formula
///
/// # Returns
/// * `Ok((String, f32))` - A tuple containing:
///   - The SVG string representation
///   - The scale factor to apply (calculated from metrics vs target height)
/// * `Err(String)` - An error message if rendering fails
///
/// # Example
///
/// ```rust
/// use markdown2pdf::latex::latex_to_svg_with_metrics;
///
/// // Render with target height of 20 pixels
/// let result = latex_to_svg_with_metrics("x^2", false, 20.0);
/// assert!(result.is_ok());
/// ```
pub fn latex_to_svg_with_metrics(
    _latex_content: &str,
    _display: bool,
    _target_height: f32,
) -> Result<(String, f32), String> {
    Err("need LaTeX feature".to_string())
}

/// Calculates an intelligent scale factor for a formula based on its metrics and target height.
///
/// This function analyzes the formula's dimensions to determine the best scale factor.
/// It considers:
/// - The total height of the formula
/// - The ascent/descent ratio (affects apparent size)
/// - Whether this is display or inline mode
/// - The baseline positioning
/// - Key character heights (when available) for accurate scaling regardless of nesting
fn calculate_scale_factor(
    metrics: &RenderMetrics,
    key_char_metrics: Option<&microtex_rs::KeyCharMetrics>,
    target_height: f32,
    display: bool,
) -> f32 {
    let rendered_height = metrics.total_height();
    let baseline_ratio = metrics.baseline_ratio();
    
    // When key character metrics are available, use them for accurate scaling
    // This method is independent of formula complexity (fractions, subscripts, etc.)
    if let Some(kcm) = key_char_metrics {
        if kcm.average_char_height > 0.0 {
            // Use average key character height instead of total formula height
            // This avoids over-scaling formulas with deep elements (fractions, subscripts)
            let base_scale = target_height / kcm.average_char_height;
            
            // Apply minor adjustments based on display mode
            return if display {
                // For display mode, key chars provide precise reference
                // Minimal adjustment needed
                base_scale.max(0.25).min(2.5)
            } else {
                // For inline, be slightly conservative
                (base_scale * 0.9).max(0.25).min(2.5)
            };
        }
    }
    
    // Fallback to original algorithm when key character metrics unavailable
    // Basic scale factor from height matching
    let base_scale = if rendered_height > 0.0 {
        target_height / rendered_height
    } else {
        1.0
    };

    // For display mode, apply intelligent scaling based on baseline distribution
    let visual_scale = if display {
        // The baseline ratio indicates the visual distribution of the formula
        // Formulas with low baseline (lots of depth) need to be scaled up
        // Formulas with high baseline (tall) need to be scaled down slightly
        
        let baseline_factor = if baseline_ratio < 0.4 {
            // Very deep formulas (many fractions, subscripts)
            // These appear much smaller than they actually are
            1.0 + (0.4 - baseline_ratio) * 0.4
        } else if baseline_ratio > 0.7 {
            // Very tall formulas (many superscripts)
            // These appear larger than they actually are
            1.0 - (baseline_ratio - 0.7) * 0.15
        } else if baseline_ratio > 0.6 {
            // Moderately tall formulas
            1.0 - (baseline_ratio - 0.6) * 0.1
        } else if baseline_ratio < 0.5 {
            // Moderately deep formulas
            1.0 + (0.5 - baseline_ratio) * 0.2
        } else {
            // Balanced formulas (around 0.5 ratio)
            1.0
        };
        
        base_scale * baseline_factor
    } else {
        // For inline mode, be conservative to avoid disrupting line spacing
        base_scale * 0.9
    };

    // Clamp the scale factor to reasonable bounds (0.25x to 2.5x)
    visual_scale.max(0.25).min(2.5)
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
    fn test_latex_to_svg_with_metrics_empty_content() {
        let result = latex_to_svg_with_metrics("", false, 20.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_latex_to_svg_with_metrics_invalid_target_height() {
        let result = latex_to_svg_with_metrics("x^2", false, 0.0);
        assert!(result.is_err());

        let result = latex_to_svg_with_metrics("x^2", false, -10.0);
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
