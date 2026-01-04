use lazy_static::lazy_static;
/// Syntax highlighting module using syntect
/// Provides colored code blocks similar to GitHub's rendering
use std::collections::HashMap;
use syntect::easy::HighlightLines;
use syntect::highlighting::Color;
use syntect::parsing::{SyntaxDefinition, SyntaxSet, SyntaxSetBuilder};

/// A color representation suitable for PDF rendering using RGB values.
///
/// This struct wraps RGB color values (0-255 for each channel) commonly used
/// in syntax highlighting to represent text colors in generated PDFs.
///
/// # Fields
///
/// * `r` - Red component (0-255)
/// * `g` - Green component (0-255)
/// * `b` - Blue component (0-255)
///
/// # Examples
///
/// Create a color directly from RGB values:
///
/// ```
/// use markdown2pdf::highlighting::HighlightColor;
///
/// let red = HighlightColor::from_rgb(255, 0, 0);
/// assert_eq!(red.r, 255);
/// assert_eq!(red.g, 0);
/// assert_eq!(red.b, 0);
/// ```
///
/// Convert from a syntect Color and retrieve as tuple:
///
/// ```
/// use markdown2pdf::highlighting::HighlightColor;
///
/// let blue = HighlightColor::from_rgb(0, 100, 200);
/// let (r, g, b) = blue.as_rgb_u8();
/// assert_eq!((r, g, b), (0, 100, 200));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HighlightColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl HighlightColor {
    /// Creates a new color from RGB component values.
    ///
    /// # Arguments
    ///
    /// * `r` - Red component (0-255)
    /// * `g` - Green component (0-255)
    /// * `b` - Blue component (0-255)
    ///
    /// # Examples
    ///
    /// ```
    /// use markdown2pdf::highlighting::HighlightColor;
    ///
    /// let color = HighlightColor::from_rgb(128, 64, 200);
    /// assert_eq!(color.r, 128);
    /// assert_eq!(color.g, 64);
    /// assert_eq!(color.b, 200);
    /// ```
    pub fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Converts a syntect Color to a HighlightColor.
    ///
    /// This method extracts the RGB components from a syntect Color struct,
    /// which is used internally by the syntect syntax highlighting library.
    ///
    /// # Arguments
    ///
    /// * `color` - A syntect Color object with RGB components
    ///
    /// # Examples
    ///
    /// ```
    /// use markdown2pdf::highlighting::HighlightColor;
    /// use syntect::highlighting::Color;
    ///
    /// let syntect_color = Color {
    ///     r: 200,
    ///     g: 100,
    ///     b: 50,
    ///     a: 255,
    /// };
    /// let highlight_color = HighlightColor::from_syntect_color(syntect_color);
    /// assert_eq!(highlight_color.r, 200);
    /// assert_eq!(highlight_color.g, 100);
    /// assert_eq!(highlight_color.b, 50);
    /// ```
    pub fn from_syntect_color(color: Color) -> Self {
        Self {
            r: color.r,
            g: color.g,
            b: color.b,
        }
    }

    /// Converts the color to a tuple of RGB values.
    ///
    /// # Returns
    ///
    /// A tuple `(r, g, b)` containing the red, green, and blue components.
    ///
    /// # Examples
    ///
    /// ```
    /// use markdown2pdf::highlighting::HighlightColor;
    ///
    /// let color = HighlightColor::from_rgb(100, 150, 200);
    /// let (r, g, b) = color.as_rgb_u8();
    /// assert_eq!(r, 100);
    /// assert_eq!(g, 150);
    /// assert_eq!(b, 200);
    /// ```
    pub fn as_rgb_u8(&self) -> (u8, u8, u8) {
        (self.r, self.g, self.b)
    }
}

/// A syntax-highlighted code token with styling information.
///
/// This struct represents a single token of code extracted by the syntax highlighter,
/// including the token text, its color, and optional bold and italic styling.
///
/// # Fields
///
/// * `text` - The content of the token (e.g., a keyword, identifier, or operator)
/// * `color` - The RGB color to apply to this token for rendering
/// * `bold` - Whether the token should be rendered in bold
/// * `italic` - Whether the token should be rendered in italics
///
/// # Examples
///
/// Create a token for a keyword with bold styling:
///
/// ```
/// use markdown2pdf::highlighting::{HighlightColor, HighlightedToken};
///
/// let token = HighlightedToken {
///     text: "fn".to_string(),
///     color: HighlightColor::from_rgb(167, 29, 93), // Magenta
///     bold: true,
///     italic: false,
/// };
///
/// assert_eq!(token.text, "fn");
/// assert_eq!(token.bold, true);
/// assert_eq!(token.italic, false);
/// ```
#[derive(Debug, Clone)]
pub struct HighlightedToken {
    pub text: String,
    pub color: HighlightColor,
    pub bold: bool,
    pub italic: bool,
}

lazy_static! {
    // Custom syntax set containing TypeScript, Bash, PowerShell, etc.
    static ref CUSTOM_SYNTAX_SET: SyntaxSet = load_custom_syntaxes();
    // Default syntax set for fallback (C, Python, Java, etc.)
    static ref SYNTAX_SET: SyntaxSet = SyntaxSet::load_defaults_newlines();
    static ref THEME_SET: syntect::highlighting::ThemeSet =
        syntect::highlighting::ThemeSet::load_defaults();
}

/// Load custom syntaxes from embedded .sublime-syntax files using SyntaxSetBuilder
fn load_custom_syntaxes() -> SyntaxSet {
    let mut builder = SyntaxSetBuilder::new();

    // Load custom grammars from embedded .sublime-syntax files
    let syntax_files = vec![
        (
            "TypeScript",
            include_str!("../../syntaxes/typescript/TypeScript.sublime-syntax"),
        ),
        (
            "TypeScriptReact",
            include_str!("../../syntaxes/typescript/TypeScriptReact.sublime-syntax"),
        ),
        (
            "Bash",
            include_str!("../../syntaxes/bash/Bash.sublime-syntax"),
        ),
        (
            "Shell-Unix-Generic",
            include_str!("../../syntaxes/bash/Shell-Unix-Generic.sublime-syntax"),
        ),
        (
            "PowerShell",
            include_str!("../../syntaxes/powershell/PowerShell.sublime-syntax"),
        ),
    ];

    // Add each custom syntax to the builder
    for (name, content) in syntax_files {
        match SyntaxDefinition::load_from_str(content, true, None) {
            Ok(syntax) => {
                builder.add(syntax);
            }
            Err(e) => {
                eprintln!("⚠ Failed to load custom syntax {}: {}", name, e);
            }
        }
    }

    // Build the custom syntax set (we'll fallback to defaults if needed)
    builder.build()
}

/// Maps language names to syntect syntax definitions
fn get_syntax_mapping() -> HashMap<&'static str, &'static str> {
    let mut map = HashMap::new();
    // Primary language names
    map.insert("c", "C");
    map.insert("cpp", "C++");
    map.insert("c++", "C++");
    map.insert("java", "Java");
    map.insert("python", "Python");
    map.insert("py", "Python");
    // TypeScript and TSX use custom grammars loaded from syntaxes/ folder
    map.insert("typescript", "TypeScript");
    map.insert("ts", "TypeScript");
    map.insert("javascript", "JavaScript");
    map.insert("js", "JavaScript");
    map.insert("jsx", "JavaScript");
    map.insert("tsx", "TypeScriptReact");
    map.insert("rust", "Rust");
    map.insert("rs", "Rust");
    map.insert("go", "Go");
    // Bash and PowerShell now have custom grammars loaded from syntaxes/ folder
    map.insert("bash", "Bash");
    map.insert("sh", "Bash");
    map.insert("shell", "Bash");
    map.insert("powershell", "PowerShell");
    map.insert("ps1", "PowerShell");
    map.insert("html", "HTML");
    map.insert("xml", "XML");
    map.insert("json", "JSON");
    map.insert("yaml", "YAML");
    map.insert("yml", "YAML");
    map.insert("sql", "SQL");
    map.insert("dockerfile", "Dockerfile");
    map.insert("docker", "Dockerfile");
    map.insert("markdown", "Markdown");
    map.insert("md", "Markdown");

    map
}

/// Highlights code using syntax highlighting rules based on the specified language.
///
/// This function applies syntax highlighting to source code, breaking it into
/// tokens with appropriate colors for rendering in PDFs. It supports multiple
/// programming languages including Rust, TypeScript, Python, Java, C, C++, Go,
/// Bash, PowerShell, and many more.
///
/// # Arguments
///
/// * `code` - The source code to highlight
/// * `language` - The programming language identifier (e.g., "rust", "typescript", "python")
///              Common values: "rust", "ts", "tsx", "js", "jsx", "py", "java",
///              "c", "cpp", "go", "bash", "sh", "powershell", "ps1"
///
/// # Returns
///
/// A vector of `HighlightedToken` structs, each containing a piece of code text,
/// its RGB color, and styling information (bold, italic).
/// If the language is not recognized, falls back to plaintext (gray color).
///
/// # Examples
///
/// Highlight some Rust code:
///
/// ```
/// use markdown2pdf::highlighting::highlight_code;
///
/// let code = r#"
/// fn main() {
///     println!(\"Hello, world!\");
/// }
/// "#;
/// let tokens = highlight_code(code, "rust");
///
/// // Verify we got tokens back
/// assert!(!tokens.is_empty());
///
/// // Print highlighted tokens
/// for token in tokens {
///     if !token.text.trim().is_empty() {
///         let (r, g, b) = token.color.as_rgb_u8();
///         println!("Token: '{}' Color: RGB({}, {}, {})",
///                  token.text, r, g, b);
///     }
/// }
/// ```
///
/// Highlight TypeScript/React code:
///
/// ```
/// use markdown2pdf::highlighting::highlight_code;
///
/// let code = r#"
/// const Component: React.FC = () => {
///     return <div>Hello JSX</div>;
/// };
/// "#;
/// let tokens = highlight_code(code, "tsx");
///
/// // Verify TypeScript keywords are recognized
/// assert!(tokens.iter().any(|t| t.text.contains("const")));
/// assert!(tokens.iter().any(|t| t.text.contains("React")));
/// ```
pub fn highlight_code(code: &str, language: &str) -> Vec<HighlightedToken> {
    highlight_code_with_syntect(code, language)
}

/// Core syntax highlighting using syntect
fn highlight_code_with_syntect(code: &str, language: &str) -> Vec<HighlightedToken> {
    let language_lower = language.to_lowercase();
    let language_mapping = get_syntax_mapping();

    let syntax_name = language_mapping
        .get(language_lower.as_str())
        .copied()
        .unwrap_or_else(|| {
            if SYNTAX_SET.find_syntax_by_name(&language).is_some() {
                language
            } else if SYNTAX_SET.find_syntax_by_first_line(code).is_some() {
                return "";
            } else {
                "Plain Text"
            }
        });

    let syntax = if syntax_name.is_empty() {
        CUSTOM_SYNTAX_SET
            .find_syntax_by_first_line(code)
            .or_else(|| SYNTAX_SET.find_syntax_by_first_line(code))
            .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text())
    } else {
        let found = CUSTOM_SYNTAX_SET
            .find_syntax_by_name(syntax_name)
            .or_else(|| SYNTAX_SET.find_syntax_by_name(syntax_name));
        if found.is_none() {
            eprintln!(
                "Warning: Could not find syntax '{}' for language '{}', using plain text",
                syntax_name, language
            );
        }
        found.unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text())
    };

    // Use InspiredGitHub theme which mimics GitHub's syntax highlighting and has good colors
    let theme = THEME_SET
        .themes
        .get("InspiredGitHub")
        .or_else(|| THEME_SET.themes.get("base16-ocean.dark"))
        .or_else(|| THEME_SET.themes.values().next())
        .expect("No themes available");

    let mut highlighter = HighlightLines::new(syntax, theme);
    let mut tokens = Vec::new();

    for line in code.lines() {
        // Try custom syntax set first, then default
        let ranges = highlighter
            .highlight_line(line, &CUSTOM_SYNTAX_SET)
            .or_else(|_| highlighter.highlight_line(line, &SYNTAX_SET))
            .unwrap_or_default();

        for (style, text) in ranges {
            if !text.is_empty() {
                let mut color = HighlightColor::from_syntect_color(style.foreground);

                if color.r == 255 && color.g == 255 && color.b == 255 {
                    color = HighlightColor::from_rgb(220, 220, 220);
                }

                let bold = style
                    .font_style
                    .contains(syntect::highlighting::FontStyle::BOLD);
                let italic = style
                    .font_style
                    .contains(syntect::highlighting::FontStyle::ITALIC);

                tokens.push(HighlightedToken {
                    text: text.to_string(),
                    color,
                    bold,
                    italic,
                });
            }
        }

        tokens.push(HighlightedToken {
            text: "\n".to_string(),
            color: HighlightColor::from_rgb(200, 200, 200),
            bold: false,
            italic: false,
        });
    }

    if tokens.last().map(|t| t.text.as_str()) == Some("\n") {
        tokens.pop();
    }

    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test snippets extracted from test_snippets.md
    const C_CODE: &str = r#"#include <stdio.h>

int main() {
    printf("Hello from C\n");
    return 0;
}"#;

    const CPP_CODE: &str = r#"#include <iostream>

int main() {
    std::cout << "Hello from C++" << std::endl;
    return 0;
}"#;

    const JAVA_CODE: &str = r#"public class HelloWorld {
    public static void main(String[] args) {
        System.out.println("Hello from Java");
    }
}"#;

    const PYTHON_CODE: &str = r#"def greet(name):
    return f"Hello from {name}"

print(greet("Python"))"#;

    const TYPESCRIPT_CODE: &str = r#"function greet(name: string): string {
    return `Hello from ${name}`;
}

console.log(greet("TypeScript"));"#;

    const JAVASCRIPT_CODE: &str = r#"function greet(name) {
    return `Hello from ${name}`;
}

console.log(greet("JavaScript"));"#;

    const RUST_CODE: &str = r#"fn main() {
    let name = "Rust";
    println!("Hello from {}", name);
}"#;

    const GO_CODE: &str = r#"package main

import "fmt"

func main() {
    fmt.Println("Hello from Go")
}"#;

    const BASH_CODE: &str = r#"#!/bin/bash

greet() {
    echo "Hello from $1"
}

greet "Bash""#;

    const POWERSHELL_CODE: &str = r#"function Greet($name) {
    Write-Host "Hello from $name"
}

Greet "PowerShell""#;

    const JSX_CODE: &str = r#"const Button = ({ onClick, label }) => (
    <button onClick={onClick} className="btn">
        {label}
    </button>
);

export default Button;"#;

    const TSX_CODE: &str = r#"interface ButtonProps {
    onClick: () => void;
    label: string;
}

const Button: React.FC<ButtonProps> = ({ onClick, label }) => (
    <button onClick={onClick} className="btn">
        {label}
    </button>
);

export default Button;"#;

    const TEXT_CODE: &str = r#"This is plain text content without any special formatting.
It should render as-is without syntax highlighting.
Just plain readable text for reference."#;

    #[test]
    fn test_highlight_c() {
        let tokens = highlight_code(C_CODE, "c");
        assert!(!tokens.is_empty());

        let colored_tokens: Vec<_> = tokens
            .iter()
            .filter(|t| !t.text.trim().is_empty() && t.text != "\n")
            .collect();

        let colors: std::collections::HashSet<_> = colored_tokens
            .iter()
            .map(|t| (t.color.r, t.color.g, t.color.b))
            .collect();

        assert!(colors.len() > 1, "C should have multiple colors");

        let text_content: String = tokens.iter().map(|t| t.text.clone()).collect();
        assert!(text_content.contains("printf"));
        assert!(text_content.contains("main"));
    }

    #[test]
    fn test_highlight_cpp() {
        let tokens = highlight_code(CPP_CODE, "cpp");
        assert!(!tokens.is_empty());

        let colored_tokens: Vec<_> = tokens
            .iter()
            .filter(|t| !t.text.trim().is_empty() && t.text != "\n")
            .collect();

        let colors: std::collections::HashSet<_> = colored_tokens
            .iter()
            .map(|t| (t.color.r, t.color.g, t.color.b))
            .collect();

        assert!(colors.len() > 1, "C++ should have multiple colors");

        let text_content: String = tokens.iter().map(|t| t.text.clone()).collect();
        assert!(text_content.contains("std"));
        assert!(text_content.contains("cout"));
    }

    #[test]
    fn test_highlight_java() {
        let tokens = highlight_code(JAVA_CODE, "java");
        assert!(!tokens.is_empty());

        let colored_tokens: Vec<_> = tokens
            .iter()
            .filter(|t| !t.text.trim().is_empty() && t.text != "\n")
            .collect();

        let colors: std::collections::HashSet<_> = colored_tokens
            .iter()
            .map(|t| (t.color.r, t.color.g, t.color.b))
            .collect();

        assert!(colors.len() > 1, "Java should have multiple colors");

        let text_content: String = tokens.iter().map(|t| t.text.clone()).collect();
        assert!(text_content.contains("HelloWorld"));
        assert!(text_content.contains("println"));
    }

    #[test]
    fn test_highlight_python() {
        let tokens = highlight_code(PYTHON_CODE, "python");
        assert!(!tokens.is_empty());

        let colored_tokens: Vec<_> = tokens
            .iter()
            .filter(|t| !t.text.trim().is_empty() && t.text != "\n")
            .collect();

        let colors: std::collections::HashSet<_> = colored_tokens
            .iter()
            .map(|t| (t.color.r, t.color.g, t.color.b))
            .collect();

        assert!(colors.len() > 1, "Python should have multiple colors");

        let text_content: String = tokens.iter().map(|t| t.text.clone()).collect();
        assert!(text_content.contains("def"));
        assert!(text_content.contains("greet"));
    }

    #[test]
    fn test_highlight_rust() {
        let tokens = highlight_code(RUST_CODE, "rust");
        assert!(!tokens.is_empty());

        let colored_tokens: Vec<_> = tokens
            .iter()
            .filter(|t| !t.text.trim().is_empty() && t.text != "\n")
            .collect();

        let colors: std::collections::HashSet<_> = colored_tokens
            .iter()
            .map(|t| (t.color.r, t.color.g, t.color.b))
            .collect();

        assert!(colors.len() > 1, "Rust should have multiple colors");

        let text_content: String = tokens.iter().map(|t| t.text.clone()).collect();
        assert!(text_content.contains("fn"));
        assert!(text_content.contains("main"));
    }

    #[test]
    fn test_highlight_typescript_has_colors() {
        let tokens = highlight_code(TYPESCRIPT_CODE, "typescript");
        assert!(!tokens.is_empty(), "TypeScript code should produce tokens");

        let colored_tokens: Vec<_> = tokens
            .iter()
            .filter(|t| !t.text.trim().is_empty() && t.text != "\n")
            .collect();

        // Check that TypeScript-specific keywords and types are properly colored
        // The actual color from TypeScript grammar: (0, 134, 179) - a blue tone
        let ts_keyword_color = (0, 134, 179);

        // Find tokens for TypeScript-specific keywords
        let string_token = colored_tokens.iter().find(|t| t.text == "string");
        assert!(string_token.is_some(), "Should contain 'string' token");
        let string_color = string_token.unwrap().color.as_rgb_u8();
        assert_eq!(
            string_color, ts_keyword_color,
            "'string' type should be colored blue (0, 100, 200)"
        );

        // Check general color variety
        let colors: std::collections::HashSet<_> = colored_tokens
            .iter()
            .map(|t| (t.color.r, t.color.g, t.color.b))
            .collect();

        assert!(colors.len() > 1, "TypeScript should have multiple colors");

        let text_content: String = tokens.iter().map(|t| t.text.clone()).collect();
        assert!(text_content.contains("function"));
        assert!(text_content.contains("greet"));
    }

    #[test]
    fn test_highlight_javascript_has_colors() {
        let tokens = highlight_code(JAVASCRIPT_CODE, "javascript");
        assert!(!tokens.is_empty(), "JavaScript code should produce tokens");

        let colored_tokens: Vec<_> = tokens
            .iter()
            .filter(|t| !t.text.trim().is_empty() && t.text != "\n")
            .collect();

        let colors: std::collections::HashSet<_> = colored_tokens
            .iter()
            .map(|t| (t.color.r, t.color.g, t.color.b))
            .collect();

        assert!(colors.len() > 1, "JavaScript should have multiple colors");

        let text_content: String = tokens.iter().map(|t| t.text.clone()).collect();
        assert!(text_content.contains("function"));
        assert!(text_content.contains("greet"));
    }

    #[test]
    fn test_highlight_jsx_has_colors() {
        let tokens = highlight_code(JSX_CODE, "jsx");
        assert!(!tokens.is_empty(), "JSX code should produce tokens");

        let colored_tokens: Vec<_> = tokens
            .iter()
            .filter(|t| !t.text.trim().is_empty() && t.text != "\n")
            .collect();

        let colors: std::collections::HashSet<_> = colored_tokens
            .iter()
            .map(|t| (t.color.r, t.color.g, t.color.b))
            .collect();

        assert!(colors.len() > 1, "JSX should have multiple colors");

        let text_content: String = tokens.iter().map(|t| t.text.clone()).collect();
        assert!(text_content.contains("Button"));
        assert!(text_content.contains("button"));
    }

    #[test]
    fn test_highlight_tsx_has_colors() {
        let tokens = highlight_code(TSX_CODE, "tsx");
        assert!(!tokens.is_empty(), "TSX code should produce tokens");

        let colored_tokens: Vec<_> = tokens
            .iter()
            .filter(|t| !t.text.trim().is_empty() && t.text != "\n")
            .collect();

        // Check that TypeScript-specific keywords are properly colored in TSX
        // TypeScriptReact grammar uses (167, 29, 93) - a magenta tone for keywords
        let ts_keyword_color = (167, 29, 93);

        // Find tokens for TypeScript-specific keywords and types
        let interface_token = colored_tokens.iter().find(|t| t.text == "interface");
        assert!(
            interface_token.is_some(),
            "Should contain 'interface' keyword"
        );
        let interface_color = interface_token.unwrap().color.as_rgb_u8();
        assert_eq!(
            interface_color, ts_keyword_color,
            "'interface' keyword should be colored according to TypeScriptReact grammar"
        );

        let string_token = colored_tokens.iter().find(|t| t.text == "string");
        assert!(string_token.is_some(), "Should contain 'string' type");
        let string_color = string_token.unwrap().color.as_rgb_u8();
        assert_eq!(
            string_color,
            (0, 134, 179),
            "'string' type should be colored blue in TSX"
        );

        // Check general color variety
        let colors: std::collections::HashSet<_> = colored_tokens
            .iter()
            .map(|t| (t.color.r, t.color.g, t.color.b))
            .collect();

        assert!(colors.len() > 1, "TSX should have multiple colors");

        let text_content: String = tokens.iter().map(|t| t.text.clone()).collect();
        assert!(text_content.contains("Button"));
        assert!(text_content.contains("interface"));
    }

    #[test]
    fn test_highlight_temperature_card_tsx() {
        // Test the specific TemperatureCard component from COMPLETE_TECHNICAL_DOCUMENTATION.md
        let temperature_card_code = r#"// TODO: Create a component that displays temperature data
// Requirements:
// 1. Fetch from /api/thermal/current
// 2. Display current temperature and setpoint
// 3. Show a color indicator (green if within range, red if not)
// 4. Auto-refresh every 10 seconds

interface TemperatureData {
  current: number;
  setpoint: number;
  unit: string;
}

function TemperatureCard() {
  // TODO: Implement this component
  // Hint: Use useState, useEffect, useAuth
  
  return (
    <div className="card">
      {/* Your implementation here */}
    </div>
  );
}"#;

        let tokens = highlight_code(temperature_card_code, "tsx");
        assert!(
            !tokens.is_empty(),
            "TemperatureCard TSX code should produce tokens"
        );

        let colored_tokens: Vec<_> = tokens
            .iter()
            .filter(|t| !t.text.trim().is_empty() && t.text != "\n")
            .collect();

        // Check for TypeScript keywords and types
        // TypeScriptReact grammar uses (167, 29, 93) - a magenta tone for keywords
        let ts_keyword_color = (167, 29, 93);

        // Debug: print first few tokens to understand the structure
        eprintln!("Total tokens: {}", tokens.len());
        eprintln!("Colored tokens: {}", colored_tokens.len());
        for (i, token) in colored_tokens.iter().take(20).enumerate() {
            eprintln!(
                "  Token {}: '{}' -> {:?}",
                i,
                token.text.replace("\n", "\\n"),
                token.color.as_rgb_u8()
            );
        }

        // Verify 'interface' keyword is colored
        let interface_token = colored_tokens.iter().find(|t| t.text == "interface");
        assert!(
            interface_token.is_some(),
            "TemperatureCard should contain 'interface' keyword"
        );
        let interface_color = interface_token.unwrap().color.as_rgb_u8();
        assert_eq!(
            interface_color, ts_keyword_color,
            "'interface' keyword should be colored according to TypeScriptReact grammar in TemperatureCard"
        );

        // Verify 'number' and 'string' types are colored
        let number_token = colored_tokens.iter().find(|t| t.text == "number");
        assert!(
            number_token.is_some(),
            "TemperatureCard should contain 'number' type"
        );
        let number_color = number_token.unwrap().color.as_rgb_u8();
        assert_eq!(
            number_color,
            (0, 134, 179),
            "'number' type should be colored blue in TemperatureCard"
        );

        let string_token = colored_tokens.iter().find(|t| t.text == "string");
        assert!(
            string_token.is_some(),
            "TemperatureCard should contain 'string' type"
        );
        let string_color = string_token.unwrap().color.as_rgb_u8();
        assert_eq!(
            string_color,
            (0, 134, 179),
            "'string' type should be colored blue in TemperatureCard"
        );

        // Check that code contains expected identifiers
        let text_content: String = tokens.iter().map(|t| t.text.clone()).collect();
        assert!(text_content.contains("TemperatureData"));
        assert!(text_content.contains("TemperatureCard"));
        assert!(text_content.contains("current"));
        assert!(text_content.contains("setpoint"));
        assert!(text_content.contains("className"));

        // Check color variety
        let colors: std::collections::HashSet<_> = colored_tokens
            .iter()
            .map(|t| (t.color.r, t.color.g, t.color.b))
            .collect();

        assert!(
            colors.len() > 1,
            "TemperatureCard TSX should have multiple colors"
        );
    }

    #[test]
    fn test_highlight_go() {
        let tokens = highlight_code(GO_CODE, "go");
        assert!(!tokens.is_empty());

        let colored_tokens: Vec<_> = tokens
            .iter()
            .filter(|t| !t.text.trim().is_empty() && t.text != "\n")
            .collect();

        let colors: std::collections::HashSet<_> = colored_tokens
            .iter()
            .map(|t| (t.color.r, t.color.g, t.color.b))
            .collect();

        assert!(colors.len() > 1, "Go should have multiple colors");

        let text_content: String = tokens.iter().map(|t| t.text.clone()).collect();
        assert!(text_content.contains("main"));
        assert!(text_content.contains("fmt"));
    }

    #[test]
    fn test_highlight_bash_no_syntax() {
        // Bash highlighting limitation: syntect has no Bash grammar
        // Falls back to plain text rendering (all same color)
        let tokens = highlight_code(BASH_CODE, "bash");
        assert!(!tokens.is_empty());

        let text_content: String = tokens.iter().map(|t| t.text.clone()).collect();
        assert!(text_content.contains("greet"));

        // NOTE: Bash typically renders as monochrome (50,50,50) due to fallback
        // This is a limitation of syntect, not our implementation
    }

    #[test]
    fn test_highlight_powershell_no_syntax() {
        // PowerShell highlighting limitation: syntect has no PowerShell grammar
        // Falls back to plain text rendering (all same color)
        let tokens = highlight_code(POWERSHELL_CODE, "powershell");
        assert!(!tokens.is_empty());

        let text_content: String = tokens.iter().map(|t| t.text.clone()).collect();
        assert!(text_content.contains("Write-Host"));

        // NOTE: PowerShell typically renders as monochrome (50,50,50) due to fallback
        // This is a limitation of syntect, not our implementation
    }

    #[test]
    fn test_language_mapping() {
        let mapping = get_syntax_mapping();
        assert_eq!(mapping.get("rs"), Some(&"Rust"));
        assert_eq!(mapping.get("python"), Some(&"Python"));
        assert_eq!(mapping.get("js"), Some(&"JavaScript"));
        assert_eq!(mapping.get("ts"), Some(&"TypeScript")); // TypeScript has custom grammar
        assert_eq!(mapping.get("jsx"), Some(&"JavaScript")); // JSX uses JavaScript syntax
        assert_eq!(mapping.get("tsx"), Some(&"TypeScriptReact")); // TSX uses custom grammar
        assert_eq!(mapping.get("cpp"), Some(&"C++"));
        assert_eq!(mapping.get("java"), Some(&"Java"));
        assert_eq!(mapping.get("go"), Some(&"Go"));
        assert_eq!(mapping.get("bash"), Some(&"Bash")); // Bash has custom grammar
        assert_eq!(mapping.get("powershell"), Some(&"PowerShell")); // PowerShell has custom grammar
    }

    #[test]
    fn test_highlight_preserves_whitespace() {
        let code = "  let x = 42;";
        let tokens = highlight_code(code, "rust");
        let text_content: String = tokens.iter().map(|t| t.text.clone()).collect();
        assert!(text_content.contains("  "));
    }

    #[test]
    fn test_highlight_multiline_string() {
        let code = r#"let text = """
Hello
World
""";"#;
        let tokens = highlight_code(code, "rust");
        assert!(!tokens.is_empty());
        // Should preserve multiple lines
        let newline_count = tokens.iter().filter(|t| t.text.contains('\n')).count();
        assert!(newline_count > 0);
    }

    #[test]
    fn test_highlight_colors_differ_by_token_type() {
        let code = r#"fn test() { return 42; }"#;
        let tokens = highlight_code(code, "rust");

        // Should have multiple different colors (keywords, identifiers, numbers)
        let colors: std::collections::HashSet<_> = tokens
            .iter()
            .map(|t| (t.color.r, t.color.g, t.color.b))
            .collect();

        // For a real code snippet, we should have at least some color variation
        // (unless all tokens are exactly the same, which is unlikely for Rust)
        assert!(
            colors.len() > 1,
            "Should have multiple colors for different token types"
        );
    }

    #[test]
    fn test_highlight_text_plain() {
        let tokens = highlight_code(TEXT_CODE, "text");
        assert!(!tokens.is_empty());

        let text_content: String = tokens.iter().map(|t| t.text.clone()).collect();
        assert!(text_content.contains("plain text"));
    }
}
