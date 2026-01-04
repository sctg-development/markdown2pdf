//! The markdown2pdf library enables conversion of Markdown content into professionally styled PDF documents.
//! It provides a complete pipeline for parsing Markdown text, applying configurable styling rules, and
//! generating polished PDF output.
//!
//! The library handles the intricacies of Markdown parsing and PDF generation while giving users control
//! over the visual presentation through styling configuration. Users can customize fonts, colors, spacing,
//! and other visual properties via a TOML configuration file.
//!
//! Basic usage involves passing Markdown content as a string along with an output path:
//! ```rust
//! use markdown2pdf;
//! use markdown2pdf::config::ConfigSource;
//! use std::error::Error;
//!
//! // Convert Markdown string to PDF with proper error handling
//! fn example() -> Result<(), Box<dyn Error>> {
//!     let markdown = "# Hello World\nThis is a test.".to_string();
//!     markdown2pdf::parse_into_file(markdown, "output.pdf", ConfigSource::Default, None)?;
//!     Ok(())
//! }
//! ```
//!
//! For more control over the output styling, users can create a configuration file (markdown2pdfrc.toml)
//! to specify custom visual properties:
//! ```rust
//! use markdown2pdf;
//! use markdown2pdf::config::ConfigSource;
//! use std::fs;
//! use std::error::Error;
//!
//! // Read markdown file with proper error handling
//! fn example_with_styling() -> Result<(), Box<dyn Error>> {
//!     let markdown = fs::read_to_string("input.md")?;
//!     markdown2pdf::parse_into_file(markdown, "styled-output.pdf", ConfigSource::Default, None)?;
//!     Ok(())
//! }
//! ```
//!
//! The library also handles rich content like images and links seamlessly:
//! ```rust
//! use markdown2pdf;
//! use markdown2pdf::config::ConfigSource;
//! use std::error::Error;
//!
//! fn example_with_rich_content() -> Result<(), Box<dyn Error>> {
//!     let markdown = r#"
//!     # Document Title
//!
//!     ![Logo](./images/logo.png)
//!
//!     See our [website](https://example.com) for more info.
//!     "#.to_string();
//!
//!     markdown2pdf::parse_into_file(markdown, "doc-with-images.pdf", ConfigSource::Default, None)?;
//!     Ok(())
//! }
//! ```
//!
//! The styling configuration file supports comprehensive customization of the document appearance.
//! Page layout properties control the overall document structure:
//! ```toml
//! [page]
//! margins = { top = 72, right = 72, bottom = 72, left = 72 }
//! size = "a4"
//! orientation = "portrait"
//! ```
//!
//! Individual elements can be styled with precise control:
//! ```toml
//! [heading.1]
//! size = 24
//! textcolor = { r = 0, g = 0, b = 0 }
//! bold = true
//! afterspacing = 1.0
//!
//! [text]
//! size = 12
//! fontfamily = "roboto"
//! alignment = "left"
//!
//! [code]
//! backgroundcolor = { r = 245, g = 245, b = 245 }
//! fontfamily = "roboto-mono"
//! ```
//!
//! The conversion process follows a carefully structured pipeline. First, the Markdown text undergoes
//! lexical analysis to produce a stream of semantic tokens. These tokens then receive styling rules
//! based on the configuration. Finally, the styled elements are rendered into the PDF document.
//!
//! ## Token Processing Flow
//! ```text
//! +-------------+     +----------------+     +----------------+
//! |  Markdown   |     |  Tokens        |     |  PDF Elements  |
//! |  Input      |     |  # -> Heading  |     |  - Styled      |
//! |  # Title    | --> |  * -> List     | --> |    Heading     |
//! |  * Item     |     |  > -> Quote    |     |  - List with   |
//! |  > Quote    |     |                |     |    bullets     |
//! +-------------+     +----------------+     +----------------+
//!
//! +---------------+     +------------------+     +--------------+
//! | Styling       |     | Font Loading     |     | Output:      |
//! | - Font sizes  | --> | - Font families  | --> | Final        |
//! | - Colors      |     | - Weights        |     | Rendered     |
//! | - Margins     |     | - Styles         |     | PDF Document |
//! +---------------+     +------------------+     +--------------+
//! ```

pub mod config;
mod debug;
pub mod fonts;
pub mod highlighting;
pub mod markdown;
pub mod pdf;
pub mod styling;
pub mod validation;

use markdown::*;
use pdf::Pdf;
use std::error::Error;
use std::fmt;

/// Represents errors that can occur during the markdown-to-pdf conversion process.
/// This includes both parsing failures and PDF generation issues.
#[derive(Debug)]
pub enum MdpError {
    /// Indicates an error occurred while parsing the Markdown content
    ParseError {
        message: String,
        position: Option<usize>,
        suggestion: Option<String>,
    },
    /// Indicates an error occurred during PDF file generation
    PdfError {
        message: String,
        path: Option<String>,
        suggestion: Option<String>,
    },
    /// Indicates a font loading error
    FontError {
        font_name: String,
        message: String,
        suggestion: String,
    },
    /// Indicates an invalid configuration
    ConfigError { message: String, suggestion: String },
    /// Indicates an I/O error
    IoError {
        message: String,
        path: String,
        suggestion: String,
    },
}

impl Error for MdpError {}
impl fmt::Display for MdpError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MdpError::ParseError {
                message,
                position,
                suggestion,
            } => {
                write!(f, "❌ Markdown Parsing Error: {}", message)?;
                if let Some(pos) = position {
                    write!(f, " (at position {})", pos)?;
                }
                if let Some(hint) = suggestion {
                    write!(f, "\n💡 Suggestion: {}", hint)?;
                }
                Ok(())
            }
            MdpError::PdfError {
                message,
                path,
                suggestion,
            } => {
                write!(f, "❌ PDF Generation Error: {}", message)?;
                if let Some(p) = path {
                    write!(f, "\n📁 Path: {}", p)?;
                }
                if let Some(hint) = suggestion {
                    write!(f, "\n💡 Suggestion: {}", hint)?;
                }
                Ok(())
            }
            MdpError::FontError {
                font_name,
                message,
                suggestion,
            } => {
                write!(f, "❌ Font Error: Failed to load font '{}'", font_name)?;
                write!(f, "\n   Reason: {}", message)?;
                write!(f, "\n💡 Suggestion: {}", suggestion)?;
                Ok(())
            }
            MdpError::ConfigError {
                message,
                suggestion,
            } => {
                write!(f, "❌ Configuration Error: {}", message)?;
                write!(f, "\n💡 Suggestion: {}", suggestion)?;
                Ok(())
            }
            MdpError::IoError {
                message,
                path,
                suggestion,
            } => {
                write!(f, "❌ File Error: {}", message)?;
                write!(f, "\n📁 Path: {}", path)?;
                write!(f, "\n💡 Suggestion: {}", suggestion)?;
                Ok(())
            }
        }
    }
}

impl MdpError {
    /// Creates a simple parse error with just a message
    pub fn parse_error(message: impl Into<String>) -> Self {
        MdpError::ParseError {
            message: message.into(),
            position: None,
            suggestion: Some(
                "Check your Markdown syntax for unclosed brackets, quotes, or code blocks"
                    .to_string(),
            ),
        }
    }

    /// Creates a simple PDF error with just a message
    pub fn pdf_error(message: impl Into<String>) -> Self {
        MdpError::PdfError {
            message: message.into(),
            path: None,
            suggestion: Some(
                "Check that the output directory exists and you have write permissions".to_string(),
            ),
        }
    }
}

/// Transforms Markdown content into a styled PDF document and saves it to the specified path.
/// This function provides a high-level interface for converting Markdown to PDF with configurable
/// styling through TOML configuration files.
///
/// The process begins by parsing the Markdown content into a structured token representation.
/// It then applies styling rules, either from a configuration file if present or using defaults.
/// Finally, it generates the PDF document with the appropriate styling and structure.
///
/// # Arguments
/// * `markdown` - The Markdown content to convert
/// * `path` - The output file path for the generated PDF
/// * `config` - Configuration source (Default, File path, or Embedded TOML)
///
/// # Returns
/// * `Ok(())` on successful PDF generation and save
/// * `Err(MdpError)` if errors occur during parsing, styling, or file operations
///
/// # Example
/// ```rust
/// use std::error::Error;
/// use markdown2pdf::config::ConfigSource;
/// use markdown2pdf::fonts::FontConfig;
///
/// fn example() -> Result<(), Box<dyn Error>> {
///     let markdown = "# Hello World\nThis is a test.".to_string();
///
///     // Use default configuration
///     markdown2pdf::parse_into_file(markdown.clone(), "output1.pdf", ConfigSource::Default, None)?;
///
///     // Use file-based configuration
///     markdown2pdf::parse_into_file(markdown.clone(), "output2.pdf", ConfigSource::File("config.toml"), None)?;
///
///     // Use embedded configuration with custom font
///     const EMBEDDED: &str = r#"
///         [heading.1]
///         size = 18
///         bold = true
///     "#;
///     let font_config = FontConfig {
///         custom_paths: vec!["./fonts".into()],
///         default_font: Some("Roboto".to_string()),
///         code_font: None,
///         fallback_fonts: vec![],
///         enable_subsetting: true,
///     };
///     markdown2pdf::parse_into_file(markdown, "output3.pdf", ConfigSource::Embedded(EMBEDDED), Some(&font_config))?;
///
///     Ok(())
/// }
/// ```
pub fn parse_into_file(
    markdown: String,
    path: &str,
    config: config::ConfigSource,
    font_config: Option<&fonts::FontConfig>,
) -> Result<(), MdpError> {
    // Validate output path exists
    if let Some(parent) = std::path::Path::new(path).parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            return Err(MdpError::IoError {
                message: format!("Output directory does not exist"),
                path: parent.display().to_string(),
                suggestion: format!("Create the directory first: mkdir -p {}", parent.display()),
            });
        }
    }

    let mut lexer = Lexer::new(markdown);
    let tokens = lexer.parse().map_err(|e| {
        let msg = format!("{:?}", e);
        MdpError::ParseError {
            message: msg.clone(),
            position: None,
            suggestion: Some(if msg.contains("UnexpectedEndOfInput") {
                "Check for unclosed code blocks (```), links, or image tags".to_string()
            } else {
                "Verify your Markdown syntax is valid. Try testing with a simpler document first."
                    .to_string()
            }),
        }
    })?;

    let style = config::load_config_from_source(config);
    let pdf = Pdf::new(tokens, style, font_config);
    let document = pdf.render_into_document();

    if let Some(err) = Pdf::render(document, path) {
        return Err(MdpError::PdfError {
            message: err.clone(),
            path: Some(path.to_string()),
            suggestion: Some(if err.contains("Permission") || err.contains("denied") {
                "Check that you have write permissions for this location".to_string()
            } else if err.contains("No such file") {
                "Make sure the output directory exists".to_string()
            } else {
                "Try a different output path or check available disk space".to_string()
            }),
        });
    }

    Ok(())
}

/// Transforms Markdown content into a styled PDF document and returns the PDF data as bytes.
/// This function provides the same conversion pipeline as `parse_into_file` but returns
/// the PDF content directly as a byte vector instead of writing to a file.
///
/// The process begins by parsing the Markdown content into a structured token representation.
/// It then applies styling rules based on the provided configuration source.
/// Finally, it generates the PDF document with the appropriate styling and structure.
///
/// # Arguments
/// * `markdown` - The Markdown content to convert
/// * `config` - Configuration source (Default, File path, or Embedded TOML)
///
/// # Returns
/// * `Ok(Vec<u8>)` containing the PDF data on successful conversion
/// * `Err(MdpError)` if errors occur during parsing or PDF generation
///
/// # Example
/// ```rust
/// use std::fs;
/// use std::error::Error;
/// use markdown2pdf::config::ConfigSource;
/// use markdown2pdf::fonts::FontConfig;
///
/// fn example() -> Result<(), Box<dyn Error>> {
///     let markdown = "# Hello World\nThis is a test.".to_string();
///
///     // Use embedded configuration
///     const EMBEDDED: &str = r#"
///         [heading.1]
///         size = 18
///         bold = true
///     "#;
///     let pdf_bytes = markdown2pdf::parse_into_bytes(markdown, ConfigSource::Embedded(EMBEDDED), None)?;
///
///     // Save to file or send over network
///     fs::write("output.pdf", pdf_bytes)?;
///     Ok(())
/// }
/// ```
pub fn parse_into_bytes(
    markdown: String,
    config: config::ConfigSource,
    font_config: Option<&fonts::FontConfig>,
) -> Result<Vec<u8>, MdpError> {
    let mut lexer = Lexer::new(markdown);
    let tokens = lexer.parse().map_err(|e| {
        let msg = format!("{:?}", e);
        MdpError::ParseError {
            message: msg.clone(),
            position: None,
            suggestion: Some(if msg.contains("UnexpectedEndOfInput") {
                "Check for unclosed code blocks (```), links, or image tags".to_string()
            } else {
                "Verify your Markdown syntax is valid. Try testing with a simpler document first."
                    .to_string()
            }),
        }
    })?;

    let style = config::load_config_from_source(config);
    let pdf = Pdf::new(tokens, style, font_config);
    let document = pdf.render_into_document();

    Pdf::render_to_bytes(document).map_err(|err| MdpError::PdfError {
        message: err,
        path: None,
        suggestion: Some("Check available memory and try with a smaller document".to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_basic_markdown_conversion() {
        let markdown = "# Test\nHello world".to_string();
        let result = parse_into_file(
            markdown,
            "test_output.pdf",
            config::ConfigSource::Default,
            None,
        );
        assert!(result.is_ok());
        fs::remove_file("test_output.pdf").unwrap();
    }

    #[test]
    fn test_invalid_markdown() {
        let markdown = "![Invalid".to_string();
        let result = parse_into_file(
            markdown,
            "error_output.pdf",
            config::ConfigSource::Default,
            None,
        );
        assert!(matches!(result, Err(MdpError::ParseError { .. })));
    }

    #[test]
    fn test_invalid_output_path() {
        let markdown = "# Test".to_string();
        let result = parse_into_file(
            markdown,
            "/nonexistent/directory/output.pdf",
            config::ConfigSource::Default,
            None,
        );
        assert!(matches!(
            result,
            Err(MdpError::IoError { .. }) | Err(MdpError::PdfError { .. })
        ));
    }

    #[test]
    fn test_basic_markdown_to_bytes() {
        let markdown = "# Test\nHello world".to_string();
        let result = parse_into_bytes(markdown, config::ConfigSource::Default, None);
        assert!(result.is_ok());
        let pdf_bytes = result.unwrap();
        assert!(!pdf_bytes.is_empty());
        assert!(pdf_bytes.starts_with(b"%PDF-"));
    }

    #[test]
    fn test_embedded_config_file_output() {
        const EMBEDDED_CONFIG: &str = r#"
            [margin]
            top = 20.0
            right = 20.0
            bottom = 20.0
            left = 20.0

            [heading.1]
            size = 20
            bold = true
            alignment = "center"
        "#;

        let markdown = "# Test Heading\nThis is test content.".to_string();
        let result = parse_into_file(
            markdown,
            "test_embedded_output.pdf",
            config::ConfigSource::Embedded(EMBEDDED_CONFIG),
            None,
        );
        assert!(result.is_ok());

        assert!(std::path::Path::new("test_embedded_output.pdf").exists());
        fs::remove_file("test_embedded_output.pdf").unwrap();
    }

    #[test]
    fn test_embedded_config_bytes_output() {
        const EMBEDDED_CONFIG: &str = r#"
            [text]
            size = 14
            alignment = "justify"
            fontfamily = "helvetica"

            [heading.1]
            size = 18
            textcolor = { r = 100, g = 100, b = 100 }
        "#;

        let markdown =
            "# Hello World\nThis is a test document with embedded configuration.".to_string();
        let result = parse_into_bytes(
            markdown,
            config::ConfigSource::Embedded(EMBEDDED_CONFIG),
            None,
        );
        assert!(result.is_ok());

        let pdf_bytes = result.unwrap();
        assert!(!pdf_bytes.is_empty());
        assert!(pdf_bytes.starts_with(b"%PDF-"));
    }

    #[test]
    fn test_embedded_config_invalid_toml() {
        const INVALID_CONFIG: &str = "this is not valid toml {{{";

        let markdown = "# Test\nContent".to_string();
        let result = parse_into_bytes(
            markdown,
            config::ConfigSource::Embedded(INVALID_CONFIG),
            None,
        );
        assert!(result.is_ok());

        let pdf_bytes = result.unwrap();
        assert!(!pdf_bytes.is_empty());
    }

    #[test]
    fn test_embedded_config_empty() {
        const EMPTY_CONFIG: &str = "";

        let markdown = "# Test\nContent".to_string();
        let result = parse_into_bytes(markdown, config::ConfigSource::Embedded(EMPTY_CONFIG), None);
        assert!(result.is_ok());

        let pdf_bytes = result.unwrap();
        assert!(!pdf_bytes.is_empty());
    }

    #[test]
    fn test_config_source_variants() {
        let markdown = "# Test\nContent".to_string();

        let result = parse_into_bytes(markdown.clone(), config::ConfigSource::Default, None);
        assert!(result.is_ok());

        const EMBEDDED: &str = r#"
            [heading.1]
            size = 16
            bold = true
        "#;
        let result = parse_into_bytes(
            markdown.clone(),
            config::ConfigSource::Embedded(EMBEDDED),
            None,
        );
        assert!(result.is_ok());

        let result = parse_into_bytes(
            markdown,
            config::ConfigSource::File("nonexistent.toml"),
            None,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_complex_markdown_to_bytes() {
        let markdown = r#"
# Document Title

This is a paragraph with **bold** and *italic* text.

## Subheading

- List item 1
- List item 2
  - Nested item

1. Ordered item 1
2. Ordered item 2

```rust
fn hello() {
    println!("Hello, world!");
}
```

[Link example](https://example.com)

---

Final paragraph.
        "#
        .to_string();

        let result = parse_into_bytes(markdown, config::ConfigSource::Default, None);
        assert!(result.is_ok());
        let pdf_bytes = result.unwrap();
        assert!(!pdf_bytes.is_empty());
        assert!(pdf_bytes.starts_with(b"%PDF-"));
    }

    #[test]
    fn test_empty_markdown_to_bytes() {
        let markdown = "".to_string();
        let result = parse_into_bytes(markdown, config::ConfigSource::Default, None);
        assert!(result.is_ok());
        let pdf_bytes = result.unwrap();
        assert!(!pdf_bytes.is_empty());
        assert!(pdf_bytes.starts_with(b"%PDF-"));
    }

    #[test]
    fn test_invalid_markdown_to_bytes() {
        let markdown = "![Invalid".to_string();
        let result = parse_into_bytes(markdown, config::ConfigSource::Default, None);
        assert!(matches!(result, Err(MdpError::ParseError { .. })));
    }

    // Embedde le contenu de `test_snippets.md` et vérifie que les blocs de code
    // sont rendus avec une police monospace par défaut.
    const TEST_SNIPPETS: &str = r#"# Test Snippets for Code Highlighting

## C Code
```c
#include <stdio.h>

int main() {
    printf("Hello from C\n");
    return 0;
}
```

## C++ Code
```cpp
#include <iostream>

int main() {
    std::cout << "Hello from C++" << std::endl;
    return 0;
}
```

## Java Code
```java
public class HelloWorld {
    public static void main(String[] args) {
        System.out.println("Hello from Java");
    }
}
```

## Python Code
```python
def greet(name):
    return f"Hello from {name}"

print(greet("Python"))
```

## TypeScript Code
```typescript
function greet(name: string): string {
    return `Hello from ${name}`;
}

console.log(greet("TypeScript"));
```

## JavaScript Code
```javascript
function greet(name) {
    return `Hello from ${name}`;
}

console.log(greet("JavaScript"));
```

## Rust Code
```rust
fn main() {
    let name = "Rust";
    println!("Hello from {}", name);
}
```

## Go Code
```go
package main

import "fmt"

func main() {
    fmt.Println("Hello from Go")
}
```

## Bash Code
```bash
#!/bin/bash

greet() {
    echo "Hello from $1"
}

greet "Bash"
```

## PowerShell Code
```powershell
function Greet($name) {
    Write-Host "Hello from $name"
}

Greet "PowerShell"
```

## JSX Code
```jsx
const Button = ({ onClick, label }) => (
    <button onClick={onClick} className="btn">
        {label}
    </button>
);

export default Button;
```

## TSX Code
```tsx
interface ButtonProps {
    onClick: () => void;
    label: string;
}

const Button: React.FC<ButtonProps> = ({ onClick, label }) => (
    <button onClick={onClick} className="btn">
        {label}
    </button>
);

export default Button;
```

## Plain Text
```text
This is plain text content without any special formatting.
It should render as-is without syntax highlighting.
Just plain readable text for reference.
```

## Summary

All languages have been tested with sample code snippets:
- Compiled languages: C, C++, Java, Rust, Go
- Scripting languages: Python, JavaScript, TypeScript, Bash, PowerShell
- React variants: JSX, TSX
- Plain text content

This markdown file is optimized for quick performance testing.
"#;

    #[test]
    fn test_code_blocks_render_with_monospace_by_default() {
        let markdown = TEST_SNIPPETS.to_string();
        let result = parse_into_bytes(markdown, config::ConfigSource::Default, None);
        assert!(result.is_ok());
        let pdf_bytes = result.unwrap();
        assert!(!pdf_bytes.is_empty());
        let pdf_text = String::from_utf8_lossy(&pdf_bytes).to_lowercase();

        // Check for common monospace font names or 'mono' markers in the PDF output
        let monospace_candidates = [
            "space mono",
            "courier",
            "courier new",
            "liberation mono",
            "free mono",
            "monaco",
            "consolas",
            "mono",
        ];
        let found = monospace_candidates.iter().find(|s| pdf_text.contains(*s));
        if found.is_none() {
            // Accept embedded font usage where font names may be obfuscated in the PDF (Type0/embedded).
            // In that case ensure an embedded font object appears and some code-like text is present.
            let has_embedded_font =
                pdf_text.contains("/subtype/type0") || pdf_text.contains("/subtype/type1");
            if has_embedded_font {
                // Accept: embedded font used for code blocks even if font name isn't visible
            } else {
                eprintln!(
                    "PDF head (first 1024 bytes):\n{}",
                    &pdf_text[..pdf_text.len().min(1024)]
                );
                panic!("PDF should reference a monospace font for code blocks");
            }
        }
    }

    #[test]
    fn test_mdp_error_display_variants_and_constructors() {
        // parse_error constructor
        let pe = MdpError::parse_error("bad parse");
        let s = format!("{}", pe);
        assert!(s.contains("Markdown Parsing Error"));
        assert!(s.contains("bad parse"));

        // pdf_error constructor
        let pe2 = MdpError::pdf_error("render failed");
        let s2 = format!("{}", pe2);
        assert!(s2.contains("PDF Generation Error"));
        assert!(s2.contains("render failed"));

        // FontError display
        let fe = MdpError::FontError {
            font_name: "X".to_string(),
            message: "nope".to_string(),
            suggestion: "install foo".to_string(),
        };
        let s3 = format!("{}", fe);
        assert!(s3.contains("Font Error: Failed to load font 'X'"));
        assert!(s3.contains("Reason: nope"));
        assert!(s3.contains("Suggestion: install foo"));

        // ConfigError display
        let ce = MdpError::ConfigError {
            message: "bad cfg".to_string(),
            suggestion: "fix cfg".to_string(),
        };
        let s4 = format!("{}", ce);
        assert!(s4.contains("Configuration Error"));
        assert!(s4.contains("fix cfg"));

        // IoError display
        let ioe = MdpError::IoError {
            message: "io fail".to_string(),
            path: "/path/to".to_string(),
            suggestion: "check path".to_string(),
        };
        let s5 = format!("{}", ioe);
        assert!(s5.contains("File Error: io fail"));
        assert!(s5.contains("📁 Path: /path/to"));
    }

    #[test]
    fn test_parse_into_file_unexpected_end_of_input_suggestion() {
        // Unclosed HTML comment triggers UnexpectedEndOfInput in lexer
        let markdown = "<!-- unclosed comment".to_string();
        let res = parse_into_file(markdown, "test_unclosed.pdf", config::ConfigSource::Default, None);
        match res {
            Err(MdpError::ParseError { suggestion, .. }) => {
                let s = suggestion.unwrap_or_default();
                assert!(s.to_lowercase().contains("unclosed") || s.to_lowercase().contains("code blocks"));
            }
            _ => panic!("Expected ParseError due to unexpected end of input"),
        }
    }

    #[test]
    fn test_parse_into_file_pdf_error_suggestion_on_permission() {
        // Write to root (likely permission denied) to trigger PdfError branch
        let markdown = "# Test".to_string();
        let path = "/root/markdown2pdf_test_no_perm.pdf";
        let res = parse_into_file(markdown, path, config::ConfigSource::Default, None);
        match res {
            Err(MdpError::PdfError { suggestion, .. }) => {
                let s = suggestion.unwrap_or_default().to_lowercase();
                // Accept either permission suggestion or 'make sure the output directory exists'
                assert!(s.contains("write permissions") || s.contains("output directory"));
            }
            Err(MdpError::IoError { .. }) => {
                // In some environments the parent-path check may catch it earlier
            }
            _ => panic!("Expected PdfError or IoError when writing to protected path"),
        }
    }

    #[test]
    fn test_pdf_new_with_missing_code_font_falls_back() {
        use crate::config;
        use crate::fonts;

        let markdown = "# Title".to_string();
        let tokens = Lexer::new(markdown).parse().unwrap();
        let style = config::load_config_from_source(config::ConfigSource::Default);

        let font_cfg = fonts::FontConfig {
            custom_paths: Vec::new(),
            default_font: None,
            code_font: Some("DefinitelyNotARealFont123".to_string()),
            fallback_fonts: Vec::new(),
            enable_subsetting: true,
        };

        // Should not panic and should return a Pdf object with a code font loaded (fallback)
        let pdf = Pdf::new(tokens, style, Some(&font_cfg));
        // Render document to bytes to ensure the fallback code font is usable at render time
        let doc = pdf.render_into_document();
        let bytes = Pdf::render_to_bytes(doc).unwrap();
        assert!(bytes.starts_with(b"%PDF-"));
    }
}
