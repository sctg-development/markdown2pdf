//! PDF generation module for markdown-to-pdf conversion.
//!
//! This module handles the complete process of converting parsed markdown content into professionally formatted PDF documents.
//! It provides robust support for generating PDFs with proper typography, layout, and styling while maintaining the semantic
//! structure of the original markdown.
//!
//! The PDF generation process preserves the hierarchical document structure through careful handling of block-level and inline
//! elements. Block elements like headings, paragraphs, lists and code blocks are rendered with appropriate spacing and indentation.
//! Inline formatting such as emphasis, links and inline code maintain proper nesting and style inheritance.
//!
//! The styling system offers extensive customization options through a flexible configuration model. This includes control over:
//! fonts, text sizes, colors, margins, spacing, and special styling for different content types. The module automatically handles
//! font loading, page layout, and proper rendering of all markdown elements while respecting the configured styles.
//!
//! Error handling is built in throughout the generation process to provide meaningful feedback if issues occur during PDF creation.
//! The module is designed to be both robust for production use and flexible enough to accommodate various document structures
//! and styling needs.

use crate::{
    fonts::load_unicode_system_font,
    highlighting,
    styling::{StyleMatch, SvgWidth},
    Token,
};
use genpdfi_extended::{
    fonts::{FontData, FontFamily},
    Alignment, Document, Scale,
};
use log::{info, warn};
use std::cell::RefCell;

thread_local! {
    /// Thread-local storage for the current code font override during rendering
    /// This allows passing the code font through the rendering call stack without
    /// major structural changes.
    static CURRENT_CODE_FONT_OVERRIDE: RefCell<Option<genpdfi_extended::fonts::FontFamily<genpdfi_extended::fonts::Font>>> = RefCell::new(None);
}

/// The main PDF document generator that orchestrates the conversion process from markdown to PDF.
/// This struct serves as the central coordinator for document generation, managing the overall
/// structure, styling application, and proper sequencing of content elements.
/// It stores the input markdown tokens that will be processed into PDF content, along with style
/// configuration that controls the visual appearance and layout of the generated document.
/// The generator maintains two separate font families - a main text font used for regular document
/// content and a specialized monospace font specifically for code sections.
/// These fonts are loaded based on the style configuration and stored internally for use during
/// the PDF generation process.
#[allow(dead_code)]
pub struct Pdf {
    input: Vec<Token>,
    style: StyleMatch,
    font_family: FontFamily<FontData>,
    code_font_family: FontFamily<FontData>,
    font_fallback_chain: Option<FontFamily<genpdfi_extended::fonts::FontFallbackChain>>,
    code_font_fallback_chain: Option<FontFamily<genpdfi_extended::fonts::FontFallbackChain>>,
    image_loader: RefCell<Option<crate::images::ImageLoader>>,
}

impl Pdf {
    /// Creates a new PDF generator instance to process markdown tokens.
    /// The generator maintains document structure and applies styling/layout rules during conversion.
    ///
    /// It automatically loads two font families based on the style configuration:
    /// - A main text font for regular content
    /// - A code font specifically for code blocks and inline code segments
    ///
    /// Font loading is handled automatically but will panic if the specified fonts cannot be loaded
    /// successfully. The generator internally stores the input tokens, style configuration, and loaded
    /// font families for use during PDF generation.
    ///
    /// Through the style configuration, the generator controls all visual aspects of the output PDF
    /// including typography, dimensions, colors and spacing between elements. The style settings
    /// determine the complete visual appearance and layout characteristics of the final generated
    /// PDF document.
    ///
    /// # Arguments
    /// * `input` - The markdown tokens to convert
    /// * `style` - Style configuration for the document
    /// * `font_config` - Optional font configuration with custom paths and font overrides
    /// * `document_path` - Optional path to the markdown document (for resolving relative image paths)
    pub fn new(
        input: Vec<Token>,
        style: StyleMatch,
        font_config: Option<&crate::fonts::FontConfig>,
    ) -> Self {
        Self::with_document_path(input, style, font_config, None)
    }

    /// Creates a new PDF generator instance with document path support for image resolution.
    ///
    /// Similar to `new()` but accepts an optional document path, which is used to resolve
    /// relative image references and enable loading of local and remote images.
    ///
    /// # Arguments
    /// * `input` - The markdown tokens to convert
    /// * `style` - Style configuration for the document
    /// * `font_config` - Optional font configuration with custom paths and font overrides
    /// * `document_path` - Path to the markdown document (for resolving relative image paths)
    pub fn with_document_path(
        input: Vec<Token>,
        style: StyleMatch,
        font_config: Option<&crate::fonts::FontConfig>,
        document_path: Option<&std::path::Path>,
    ) -> Self {
        let all_text = if font_config.map(|c| c.enable_subsetting).unwrap_or(true) {
            Some(Token::collect_all_text(&input))
        } else {
            None
        };

        // Try to load fonts with fallback chains
        let (font_family, font_fallback_chain) = if let Some(family_name) = font_config
            .and_then(|cfg| cfg.default_font.as_deref())
            .or(style.text.font_family)
        {
            // User specified a font - try to load it with automatic fallbacks
            let fallback_fonts = if let Some(cfg) = font_config {
                if cfg.fallback_fonts.is_empty() {
                    crate::fonts::get_default_fallback_fonts(family_name)
                } else {
                    cfg.fallback_fonts.clone()
                }
            } else {
                crate::fonts::get_default_fallback_fonts(family_name)
            };

            if !fallback_fonts.is_empty() {
                eprintln!(
                    "Loading font '{}' with {} automatic fallback(s)...",
                    family_name,
                    fallback_fonts.len()
                );
                let custom_paths = font_config
                    .map(|c| c.custom_paths.as_slice())
                    .unwrap_or(&[]);

                // Try to load with fallback chains
                if let Ok(chain_family) = crate::fonts::load_font_with_fallback_chain(
                    family_name,
                    &fallback_fonts,
                    custom_paths,
                    all_text.as_deref(),
                ) {
                    // Note: Font subsetting for fallback chains is currently disabled because
                    // the subsetter crate creates CID fonts optimized for PDF rendering,
                    // which cannot be re-parsed by rusttype for metrics. The primary font
                    // still gets subset when loaded initially.
                    let final_chain = chain_family;

                    let primary_fonts = crate::fonts::extract_primary_fonts(&final_chain);
                    (primary_fonts, Some(final_chain))
                } else {
                    warn!("Fallback chain loading failed, using single best font...");
                    let single_font = crate::fonts::load_font_with_fallbacks(
                        family_name,
                        &fallback_fonts,
                        custom_paths,
                        all_text.as_deref(),
                    )
                    .unwrap_or_else(|_| {
                        crate::fonts::load_font_with_config(
                            family_name,
                            font_config,
                            all_text.as_deref(),
                        )
                        .unwrap_or_else(|_| {
                            load_unicode_system_font(all_text.as_deref()).unwrap_or_else(|_| {
                                crate::fonts::load_builtin_font_family("helvetica")
                                    .expect("Failed to load fallback font family")
                            })
                        })
                    });
                    (single_font, None)
                }
            } else {
                // No fallbacks available, use basic loading
                let single_font = crate::fonts::load_font_with_config(
                    family_name,
                    font_config,
                    all_text.as_deref(),
                )
                .unwrap_or_else(|_| {
                    load_unicode_system_font(all_text.as_deref()).unwrap_or_else(|_| {
                        crate::fonts::load_builtin_font_family("helvetica")
                            .expect("Failed to load fallback font family")
                    })
                });
                (single_font, None)
            }
        } else {
            info!("No font specified, searching for Unicode-capable system font...");
            let single_font = load_unicode_system_font(all_text.as_deref()).unwrap_or_else(|_| {
                crate::fonts::load_builtin_font_family("helvetica")
                    .expect("Failed to load fallback font family")
            });
            (single_font, None)
        };

        // For code blocks we prefer a monospace font (use config override or default to courier)
        let code_font_name = font_config
            .and_then(|cfg| cfg.code_font.as_deref())
            .unwrap_or("space mono");

        let code_font_family =
            crate::fonts::load_font_with_config(code_font_name, font_config, all_text.as_deref())
                .unwrap_or_else(|_| {
                    eprintln!(
                        "Warning: could not load code font '{}', falling back to Courier",
                        code_font_name
                    );
                    crate::fonts::load_builtin_font_family("space mono")
                        .expect("Failed to load fallback code font family")
                });

        Self {
            input,
            style,
            font_family,
            code_font_family,
            font_fallback_chain,
            code_font_fallback_chain: None,
            image_loader: RefCell::new(Some(crate::images::ImageLoader::new(document_path))),
        }
    }

    /// Finalizes and outputs the processed document to a PDF file at the specified path.
    /// Provides comprehensive error handling to catch and report any issues during the
    /// final rendering phase.
    pub fn render(document: genpdfi_extended::Document, path: &str) -> Option<String> {
        match document.render_to_file(path) {
            Ok(_) => None,
            Err(err) => Some(err.to_string()),
        }
    }

    /// Renders the processed document to bytes and returns the PDF data as a Vec<u8>.
    /// This method provides the same PDF generation as `render` but returns the content
    /// directly as bytes instead of writing to a file, making it suitable for cases
    /// where you need to handle the PDF data in memory or send it over a network.
    ///
    /// # Arguments
    /// * `document` - The generated PDF document to render
    ///
    /// # Returns
    /// * `Ok(Vec<u8>)` containing the PDF data on successful rendering
    /// * `Err(String)` with error message if rendering fails
    ///
    /// # Example
    /// ```rust
    /// // This example shows the basic usage pattern, but render_to_bytes
    /// // is typically called internally by parse_into_bytes
    /// use markdown2pdf::{parse_into_bytes, config::ConfigSource};
    ///
    /// let markdown = "# Test\nSome content".to_string();
    /// let pdf_bytes = parse_into_bytes(markdown, ConfigSource::Default, None).unwrap();
    /// // Use the bytes as needed (save, send, etc.)
    /// assert!(!pdf_bytes.is_empty());
    /// ```
    pub fn render_to_bytes(document: genpdfi_extended::Document) -> Result<Vec<u8>, String> {
        let mut buffer = std::io::Cursor::new(Vec::new());
        match document.render(&mut buffer) {
            Ok(_) => Ok(buffer.into_inner()),
            Err(err) => Err(err.to_string()),
        }
    }

    /// Initializes and returns a new PDF document with configured styling and layout.
    ///
    /// Creates a new document instance with the main font family and configures the page decorator
    /// with margins from the style settings. The document's base font size is set according to the
    /// text style configuration.
    ///
    /// The function processes all input tokens and renders them into the document structure before
    /// returning the complete document ready for final output. The document contains all content
    /// with proper styling, formatting and layout applied according to the style configuration.
    ///
    /// Through the style configuration, this method controls the overall document appearance including:
    /// - Page margins and layout
    /// - Base font size
    /// - Content processing and rendering
    pub fn render_into_document(&self) -> Document {
        let mut doc = genpdfi_extended::Document::new(self.font_family.clone());
        let mut decorator = genpdfi_extended::SimplePageDecorator::new();

        decorator.set_margins(genpdfi_extended::Margins::trbl(
            self.style.margins.top,
            self.style.margins.right,
            self.style.margins.bottom,
            self.style.margins.left,
        ));

        doc.set_page_decorator(decorator);
        doc.set_font_size(self.style.text.size);

        // Add code font to the document's font cache for use in code blocks
        let code_font = doc.add_font_family(self.code_font_family.clone());

        // Store it in thread-local storage for access in render_highlighted_line
        CURRENT_CODE_FONT_OVERRIDE.with(|f| {
            *f.borrow_mut() = Some(code_font);
        });

        self.process_tokens(&mut doc);

        // Clean up thread-local storage after rendering
        CURRENT_CODE_FONT_OVERRIDE.with(|f| {
            *f.borrow_mut() = None;
        });

        doc
    }

    /// Processes and renders tokens directly into the document structure.
    ///
    /// This method iterates through all input tokens and renders them into the document,
    /// handling each token type appropriately according to its semantic meaning. Block-level
    /// elements like headings, list items, and code blocks trigger the flushing of any
    /// accumulated inline tokens into paragraphs before being rendered themselves.
    ///
    /// The method maintains a buffer of current tokens that gets flushed into paragraphs
    /// when block-level elements are encountered or when explicit paragraph breaks are
    /// needed. This ensures proper document flow and maintains correct spacing between
    /// different content elements while preserving the intended document structure.
    ///
    /// Through careful token processing and rendering, this method builds up the complete
    /// document content with appropriate styling, formatting and layout applied according
    /// to the configured style settings.
    fn process_tokens(&self, doc: &mut Document) {
        let mut current_tokens = Vec::new();
        let mut consecutive_images = Vec::new();

        for token in &self.input {
            match token {
                Token::Heading(content, level) => {
                    self.flush_paragraph(doc, &current_tokens);
                    self.flush_consecutive_images(doc, &consecutive_images);
                    current_tokens.clear();
                    consecutive_images.clear();
                    self.render_heading(doc, content, *level);
                }
                Token::ListItem {
                    content,
                    ordered,
                    number,
                } => {
                    self.flush_paragraph(doc, &current_tokens);
                    self.flush_consecutive_images(doc, &consecutive_images);
                    current_tokens.clear();
                    consecutive_images.clear();
                    self.render_list_item(doc, content, *ordered, *number, 0);
                }
                Token::Code(lang, content) if content.contains('\n') => {
                    self.flush_paragraph(doc, &current_tokens);
                    self.flush_consecutive_images(doc, &consecutive_images);
                    current_tokens.clear();
                    consecutive_images.clear();
                    self.render_code_block(doc, lang, content);
                }
                Token::Math { content, display } if *display => {
                    // Display math ($$...$$) is a block-level element
                    self.flush_paragraph(doc, &current_tokens);
                    self.flush_consecutive_images(doc, &consecutive_images);
                    current_tokens.clear();
                    consecutive_images.clear();
                    self.render_math_block(doc, content);
                }
                Token::Math {
                    content: _,
                    display: false,
                } => {
                    // Inline math ($...$) - treat as inline content, not block
                    current_tokens.push(token.clone());
                }
                Token::HorizontalRule => {
                    self.flush_paragraph(doc, &current_tokens);
                    self.flush_consecutive_images(doc, &consecutive_images);
                    current_tokens.clear();
                    consecutive_images.clear();
                    doc.push(genpdfi_extended::elements::Break::new(
                        self.style.horizontal_rule.after_spacing,
                    ));
                }
                Token::LineBreak => {
                    // Line breaks are inline - treat as part of paragraph
                    current_tokens.push(token.clone());
                }
                Token::Newline => {
                    self.flush_paragraph(doc, &current_tokens);
                    self.flush_consecutive_images(doc, &consecutive_images);
                    current_tokens.clear();
                    consecutive_images.clear();
                }
                Token::Table {
                    headers,
                    aligns,
                    rows,
                } => {
                    self.flush_paragraph(doc, &current_tokens);
                    self.flush_consecutive_images(doc, &consecutive_images);
                    current_tokens.clear();
                    consecutive_images.clear();
                    self.render_table(doc, headers, aligns, rows)
                }
                Token::Image(alt, url) => {
                    // Collect consecutive images to render together with minimal spacing
                    consecutive_images.push((alt.clone(), url.clone(), false));
                }
                Token::ImageWithLink(alt, image_url, link_url) => {
                    // Collect consecutive images with links to render together with minimal spacing
                    consecutive_images.push((
                        format!("{}||{}", image_url, link_url),
                        alt.clone(),
                        true,
                    ));
                }
                Token::Text(content)
                    if !consecutive_images.is_empty() && content.trim().is_empty() =>
                {
                    // Ignore whitespace-only tokens (e.g., from single newlines) when we're collecting images
                    // This allows images separated by single newlines to be grouped together
                }
                _ => {
                    // If we have accumulated images and encounter non-image content, flush them first
                    if !consecutive_images.is_empty() {
                        self.flush_consecutive_images(doc, &consecutive_images);
                        consecutive_images.clear();
                    }
                    current_tokens.push(token.clone());
                }
            }
        }

        // Flush any remaining tokens
        self.flush_paragraph(doc, &current_tokens);
        self.flush_consecutive_images(doc, &consecutive_images);
    }

    /// Renders accumulated consecutive images horizontally in a table.
    /// This allows multiple images to be displayed side-by-side when they are not
    /// separated by a Newline (paragraph break) in the source markdown.
    /// According to CommonMark: single newline = whitespace (no line break), so images
    /// on consecutive lines without double newlines should appear horizontally.
    fn flush_consecutive_images(&self, doc: &mut Document, images: &[(String, String, bool)]) {
        if images.is_empty() {
            return;
        }

        // Render all consecutive images together in a single container with minimal spacing
        doc.push(genpdfi_extended::elements::Break::new(
            self.style.text.before_spacing,
        ));

        // Render each image without the standard breaks (which would separate them vertically)
        for (idx, (first_part, second_part, is_link)) in images.iter().enumerate() {
            if *is_link {
                // ImageWithLink case: first_part is "url||link_url", second_part is alt
                let parts: Vec<&str> = first_part.split("||").collect();
                if parts.len() == 2 {
                    self.render_image_with_link_no_breaks(doc, second_part, parts[0], parts[1]);
                }
            } else {
                // Regular Image case: first_part is alt, second_part is url
                self.render_image_no_breaks(doc, first_part, second_part);
            }

            // Add a small space between consecutive images
            if idx < images.len() - 1 {
                doc.push(genpdfi_extended::elements::Break::new(0.05));
            }
        }

        doc.push(genpdfi_extended::elements::Break::new(
            self.style.text.after_spacing,
        ));
    }

    /// Renders an image without the standard before/after breaks.
    /// Used for consecutive images that should appear horizontally.
    fn render_image_no_breaks(&self, doc: &mut Document, alt: &str, url: &str) {
        let mut loader_opt = self.image_loader.borrow_mut();

        if let Some(ref mut loader) = *loader_opt {
            match loader.load(url) {
                Ok(image_data) => {
                    match image_data.format {
                        crate::images::ImageFormat::Svg => {
                            match String::from_utf8(image_data.bytes.clone()) {
                                Ok(svg_string) => {
                                    match genpdfi_extended::elements::Image::from_svg_string(
                                        &svg_string,
                                    ) {
                                        Ok(image) => {
                                            let image = match self.style.svg_config.width {
                                                crate::styling::SvgWidth::Percentage(percent) => {
                                                    image.resizing_page_with(percent / 100.0)
                                                }
                                                crate::styling::SvgWidth::Pixels(_pixels) => image,
                                                crate::styling::SvgWidth::Auto => {
                                                    if self.style.svg_config.scale_factor != 1.0 {
                                                        image.with_scale(Scale::new(
                                                            self.style.svg_config.scale_factor,
                                                            self.style.svg_config.scale_factor,
                                                        ))
                                                    } else {
                                                        image
                                                    }
                                                }
                                            };
                                            // Render without centered alignment to allow side-by-side display
                                            doc.push(image);
                                        }
                                        Err(e) => {
                                            warn!("Failed to render SVG: {}", e);
                                            let mut para =
                                                genpdfi_extended::elements::Paragraph::default();
                                            let style = genpdfi_extended::style::Style::new()
                                                .with_font_size(self.style.text.size)
                                                .italic();
                                            para.push_styled(
                                                format!("[SVG Image: {}]", alt),
                                                style,
                                            );
                                            doc.push(para);
                                        }
                                    }
                                }
                                Err(e) => {
                                    warn!("Failed to decode SVG as UTF-8: {}", e);
                                    let mut para = genpdfi_extended::elements::Paragraph::default();
                                    let style = genpdfi_extended::style::Style::new()
                                        .with_font_size(self.style.text.size)
                                        .italic();
                                    para.push_styled(
                                        format!("[Image: {} - decode error]", alt),
                                        style,
                                    );
                                    doc.push(para);
                                }
                            }
                        }
                        crate::images::ImageFormat::Jpeg
                        | crate::images::ImageFormat::Png
                        | crate::images::ImageFormat::WebP
                        | crate::images::ImageFormat::Gif => {
                            match genpdfi_extended::elements::Image::from_reader(
                                std::io::Cursor::new(image_data.bytes.clone()),
                            ) {
                                Ok(image) => {
                                    doc.push(image);
                                }
                                Err(e) => {
                                    warn!("Failed to load image: {}", e);
                                    let mut para = genpdfi_extended::elements::Paragraph::default();
                                    let style = genpdfi_extended::style::Style::new()
                                        .with_font_size(self.style.text.size)
                                        .italic();
                                    para.push_styled(format!("[Image: {}]", alt), style);
                                    doc.push(para);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to load image from {}: {}", url, e);
                    let mut para = genpdfi_extended::elements::Paragraph::default();
                    let style = genpdfi_extended::style::Style::new()
                        .with_font_size(self.style.text.size)
                        .italic();
                    para.push_styled(format!("[Image not found: {}]", alt), style);
                    doc.push(para);
                }
            }
        }
    }

    /// Renders an image with a link without the standard before/after breaks.
    /// Used for consecutive images that should appear horizontally.
    fn render_image_with_link_no_breaks(
        &self,
        doc: &mut Document,
        alt: &str,
        image_url: &str,
        link_url: &str,
    ) {
        let mut loader_opt = self.image_loader.borrow_mut();

        if let Some(ref mut loader) = *loader_opt {
            match loader.load(image_url) {
                Ok(image_data) => {
                    match image_data.format {
                        crate::images::ImageFormat::Svg => {
                            match String::from_utf8(image_data.bytes.clone()) {
                                Ok(svg_string) => {
                                    match genpdfi_extended::elements::Image::from_svg_string(
                                        &svg_string,
                                    ) {
                                        Ok(image) => {
                                            let image = match self.style.svg_config.width {
                                                crate::styling::SvgWidth::Percentage(percent) => {
                                                    image.resizing_page_with(percent / 100.0)
                                                }
                                                crate::styling::SvgWidth::Pixels(_pixels) => image,
                                                crate::styling::SvgWidth::Auto => {
                                                    if self.style.svg_config.scale_factor != 1.0 {
                                                        image.with_scale(Scale::new(
                                                            self.style.svg_config.scale_factor,
                                                            self.style.svg_config.scale_factor,
                                                        ))
                                                    } else {
                                                        image
                                                    }
                                                }
                                            };

                                            let image = image.with_link(link_url.to_string());
                                            // Render without centered alignment to allow side-by-side display
                                            doc.push(image);
                                        }
                                        Err(e) => {
                                            warn!("Failed to render SVG with link: {}", e);
                                            let mut para =
                                                genpdfi_extended::elements::Paragraph::default();
                                            let style = genpdfi_extended::style::Style::new()
                                                .with_font_size(self.style.text.size)
                                                .italic();
                                            para.push_styled(
                                                format!("[SVG Image: {}]", alt),
                                                style,
                                            );
                                            doc.push(para);
                                        }
                                    }
                                }
                                Err(e) => {
                                    warn!("Failed to decode SVG as UTF-8: {}", e);
                                    let mut para = genpdfi_extended::elements::Paragraph::default();
                                    let style = genpdfi_extended::style::Style::new()
                                        .with_font_size(self.style.text.size)
                                        .italic();
                                    para.push_styled(
                                        format!("[Image: {} - decode error]", alt),
                                        style,
                                    );
                                    doc.push(para);
                                }
                            }
                        }
                        crate::images::ImageFormat::Jpeg
                        | crate::images::ImageFormat::Png
                        | crate::images::ImageFormat::WebP
                        | crate::images::ImageFormat::Gif => {
                            match genpdfi_extended::elements::Image::from_reader(
                                std::io::Cursor::new(image_data.bytes.clone()),
                            ) {
                                Ok(image) => {
                                    let image = image.with_link(link_url.to_string());
                                    doc.push(image);
                                }
                                Err(e) => {
                                    warn!("Failed to load image: {}", e);
                                    let mut para = genpdfi_extended::elements::Paragraph::default();
                                    let style = genpdfi_extended::style::Style::new()
                                        .with_font_size(self.style.text.size)
                                        .italic();
                                    para.push_styled(format!("[Image: {}]", alt), style);
                                    doc.push(para);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to load image from {}: {}", image_url, e);
                    let mut para = genpdfi_extended::elements::Paragraph::default();
                    let style = genpdfi_extended::style::Style::new()
                        .with_font_size(self.style.text.size)
                        .italic();
                    para.push_styled(format!("[Image not found: {}]", alt), style);
                    doc.push(para);
                }
            }
        }
    }

    /// Renders accumulated tokens as a paragraph in the document.
    ///
    /// This method takes a document and a slice of tokens, and renders them as a paragraph
    /// with appropriate styling. If the tokens slice is empty, no paragraph is rendered.
    /// After rendering the paragraph content, it adds spacing after the paragraph according
    /// to the configured text style.
    fn flush_paragraph(&self, doc: &mut Document, tokens: &[Token]) {
        if tokens.is_empty() {
            return;
        }

        doc.push(genpdfi_extended::elements::Break::new(
            self.style.text.before_spacing,
        ));
        let mut para = genpdfi_extended::elements::Paragraph::default();
        self.render_inline_content(&mut para, tokens, doc);
        doc.push(para);
        doc.push(genpdfi_extended::elements::Break::new(
            self.style.text.after_spacing,
        ));
    }

    /// Renders a heading with the appropriate level styling.
    ///
    /// This method takes a document, heading content tokens, and a level number to render
    /// a heading with the corresponding style settings. It applies font size, bold/italic effects,
    /// and text color based on the heading level configuration. After rendering the heading,
    /// it adds the configured spacing.
    fn render_heading(&self, doc: &mut Document, content: &[Token], level: usize) {
        let heading_style = match level {
            1 => &self.style.heading_1,
            2 => &self.style.heading_2,
            3 | _ => &self.style.heading_3,
        };
        doc.push(genpdfi_extended::elements::Break::new(
            heading_style.before_spacing,
        ));

        let mut para = genpdfi_extended::elements::Paragraph::default();
        let mut style = genpdfi_extended::style::Style::new().with_font_size(heading_style.size);

        if heading_style.bold {
            style = style.bold();
        }
        if heading_style.italic {
            style = style.italic();
        }
        if let Some(color) = heading_style.text_color {
            style = style.with_color(genpdfi_extended::style::Color::Rgb(
                color.0, color.1, color.2,
            ));
        }

        self.render_inline_content_with_style_simple(&mut para, content, style);
        doc.push(para);
        doc.push(genpdfi_extended::elements::Break::new(
            heading_style.after_spacing,
        ));
    }

    /// Renders inline content with a specified style.
    ///
    /// This method processes a sequence of inline tokens and renders them with the given style.
    /// It handles various inline elements like plain text, emphasis, strong emphasis, links, and
    /// inline code, applying appropriate styling modifications for each type while maintaining
    /// the base style properties.
    fn render_inline_content_with_style(
        &self,
        para: &mut genpdfi_extended::elements::Paragraph,
        tokens: &[Token],
        style: genpdfi_extended::style::Style,
        doc: &mut Document,
    ) {
        for token in tokens {
            match token {
                Token::Text(content) => {
                    para.push_styled(content.clone(), style.clone());
                }
                Token::Emphasis { level, content } => {
                    let mut nested_style = style.clone();
                    match level {
                        1 => nested_style = nested_style.italic(),
                        2 => nested_style = nested_style.bold(),
                        _ => nested_style = nested_style.bold().italic(),
                    }
                    self.render_inline_content_with_style(para, content, nested_style, doc);
                }
                Token::StrongEmphasis(content) => {
                    let nested_style = style.clone().bold();
                    self.render_inline_content_with_style(para, content, nested_style, doc);
                }
                Token::Link(text, url) => {
                    let mut link_style = style.clone();
                    if let Some(color) = self.style.link.text_color {
                        link_style = link_style.with_color(genpdfi_extended::style::Color::Rgb(
                            color.0, color.1, color.2,
                        ));
                    }
                    para.push_link(text.clone(), url.clone(), link_style);
                }
                Token::Code(_, content) => {
                    let mut code_style = style.clone();
                    if let Some(color) = self.style.code.text_color {
                        code_style = code_style.with_color(genpdfi_extended::style::Color::Rgb(
                            color.0, color.1, color.2,
                        ));
                    }
                    para.push_styled(content.clone(), code_style);
                }
                Token::Math {
                    content,
                    display: false,
                } => {
                    // Inline math - render to SVG and create image
                    self.render_inline_math_as_image(doc, content);
                }
                Token::LineBreak => {
                    // Line break within a paragraph - push a newline
                    para.push_styled("\n".to_string(), style.clone());
                }
                Token::Image(_, _) => {
                    // Images are handled as block-level elements in process_tokens,
                    // not as inline elements within paragraphs
                }
                Token::ImageWithLink(_, _, _) => {
                    // Images with links are handled as block-level elements in process_tokens,
                    // not as inline elements within paragraphs
                }
                _ => {}
            }
        }
    }

    /// Version without Document - for headings and other places where we can't render images
    fn render_inline_content_with_style_simple(
        &self,
        para: &mut genpdfi_extended::elements::Paragraph,
        tokens: &[Token],
        style: genpdfi_extended::style::Style,
    ) {
        for token in tokens {
            match token {
                Token::Text(content) => {
                    para.push_styled(content.clone(), style.clone());
                }
                Token::Emphasis { level, content } => {
                    let mut nested_style = style.clone();
                    match level {
                        1 => nested_style = nested_style.italic(),
                        2 => nested_style = nested_style.bold(),
                        _ => nested_style = nested_style.bold().italic(),
                    }
                    self.render_inline_content_with_style_simple(para, content, nested_style);
                }
                Token::StrongEmphasis(content) => {
                    let nested_style = style.clone().bold();
                    self.render_inline_content_with_style_simple(para, content, nested_style);
                }
                Token::Link(text, url) => {
                    let mut link_style = style.clone();
                    if let Some(color) = self.style.link.text_color {
                        link_style = link_style.with_color(genpdfi_extended::style::Color::Rgb(
                            color.0, color.1, color.2,
                        ));
                    }
                    para.push_link(text.clone(), url.clone(), link_style);
                }
                Token::Code(_, content) => {
                    let mut code_style = style.clone();
                    if let Some(color) = self.style.code.text_color {
                        code_style = code_style.with_color(genpdfi_extended::style::Color::Rgb(
                            color.0, color.1, color.2,
                        ));
                    }
                    para.push_styled(content.clone(), code_style);
                }
                Token::Math {
                    content,
                    display: false,
                } => {
                    // Inline math - render as styled text
                    self.render_inline_math(para, content, style.clone());
                }
                Token::LineBreak => {
                    // Line break within a paragraph - push a newline
                    para.push_styled("\n".to_string(), style.clone());
                }
                Token::Image(_, _) => {
                    // Images are handled as block-level elements in process_tokens,
                    // not as inline elements
                }
                _ => {}
            }
        }
    }

    fn render_inline_content(
        &self,
        para: &mut genpdfi_extended::elements::Paragraph,
        tokens: &[Token],
        doc: &mut Document,
    ) {
        let style = genpdfi_extended::style::Style::new().with_font_size(self.style.text.size);
        self.render_inline_content_with_style(para, tokens, style, doc);
    }

    /// Renders inline content with a specified style.

    /// Renders a code block with appropriate styling.
    ///
    /// This method handles multi-line code blocks, rendering each line as a separate
    /// paragraph with the configured code style. It applies the code font size and
    /// text color settings, and adds the configured spacing after the block.
    fn render_code_block(&self, doc: &mut Document, lang: &str, content: &str) {
        doc.push(genpdfi_extended::elements::Break::new(
            self.style.code.before_spacing,
        ));

        // Get syntax highlighted tokens
        let highlighted_tokens = highlighting::highlight_code(content, lang);

        let indent = "    "; // TODO: make this configurable from style match.
        let mut current_line = String::new();
        let mut line_tokens = Vec::new();

        for token in highlighted_tokens {
            // Check if we need to flush current line
            if token.text.contains('\n') {
                // Add remaining text before newline
                let parts: Vec<&str> = token.text.split('\n').collect();
                for (i, part) in parts.iter().enumerate() {
                    if i > 0 {
                        // Render previous line and start new one
                        self.render_highlighted_line(doc, indent, &line_tokens);
                        line_tokens.clear();
                        current_line.clear();
                    }
                    if !part.is_empty() {
                        line_tokens.push((part.to_string(), token.color, token.bold, token.italic));
                        current_line.push_str(part);
                    }
                }
            } else {
                line_tokens.push((token.text.clone(), token.color, token.bold, token.italic));
                current_line.push_str(&token.text);
            }
        }

        // Render final line if there's any content
        if !line_tokens.is_empty() {
            self.render_highlighted_line(doc, indent, &line_tokens);
        }

        doc.push(genpdfi_extended::elements::Break::new(
            self.style.code.after_spacing,
        ));
    }

    /// Renders a single line of highlighted code
    fn render_highlighted_line(
        &self,
        doc: &mut Document,
        indent: &str,
        tokens: &[(String, highlighting::HighlightColor, bool, bool)],
    ) {
        let mut para = genpdfi_extended::elements::Paragraph::default();

        // Create base code style with font override
        let mut code_style =
            genpdfi_extended::style::Style::new().with_font_size(self.style.code.size);

        // Apply code font override if available
        CURRENT_CODE_FONT_OVERRIDE.with(|f| {
            if let Some(code_font) = f.borrow().as_ref() {
                code_style = code_style.with_font_override(*code_font);
            }
        });

        // Add indentation
        let mut style = code_style;
        if let Some(color) = self.style.code.text_color {
            style = style.with_color(genpdfi_extended::style::Color::Rgb(
                color.0, color.1, color.2,
            ));
        }
        para.push_styled(indent.to_string(), style);

        // Add colored tokens
        for (text, color, _bold, _italic) in tokens {
            let mut token_style = code_style;
            let (r, g, b) = color.as_rgb_u8();
            token_style = token_style.with_color(genpdfi_extended::style::Color::Rgb(r, g, b));

            // Note: genpdfi doesn't support bold/italic in its current version,
            // so we only apply the color for now
            para.push_styled(text.clone(), token_style);
        }

        doc.push(para);
    }

    /// Renders a list item with appropriate styling and formatting.
    ///
    /// This method handles both ordered and unordered list items, with support for nested lists.
    /// For ordered lists, it includes the item number prefixed with a period (like "1."), while
    /// unordered lists use a bullet point dash character. The content is rendered with the
    /// configured list item style settings from the document style configuration.
    ///
    /// The method processes both the direct content of the list item as well as any nested list
    /// items recursively. Each nested level increases the indentation by 4 spaces to create a
    /// visual hierarchy. The method filters the content to separate inline elements from nested
    /// list items, rendering the inline content first before processing any nested items.
    ///
    /// After rendering each list item's content, appropriate spacing is added based on the
    /// configured after_spacing value. The method maintains consistent styling throughout the
    /// list hierarchy while allowing for proper nesting and indentation of complex list structures.
    fn render_list_item(
        &self,
        doc: &mut Document,
        content: &[Token],
        ordered: bool,
        number: Option<usize>,
        nesting_level: usize,
    ) {
        doc.push(genpdfi_extended::elements::Break::new(
            self.style.list_item.before_spacing,
        ));
        let mut para = genpdfi_extended::elements::Paragraph::default();
        let style = genpdfi_extended::style::Style::new().with_font_size(self.style.list_item.size);

        let indent = "    ".repeat(nesting_level);
        if !ordered {
            para.push_styled(format!("{}- ", indent), style.clone());
        } else if let Some(n) = number {
            para.push_styled(format!("{}{}. ", indent, n), style.clone());
        }

        let inline_content: Vec<Token> = content
            .iter()
            .filter(|token| !matches!(token, Token::ListItem { .. }))
            .cloned()
            .collect();
        self.render_inline_content_with_style_simple(&mut para, &inline_content, style);
        doc.push(para);
        doc.push(genpdfi_extended::elements::Break::new(
            self.style.list_item.after_spacing,
        ));

        for token in content {
            if let Token::ListItem {
                content: nested_content,
                ordered: nested_ordered,
                number: nested_number,
            } = token
            {
                self.render_list_item(
                    doc,
                    nested_content,
                    *nested_ordered,
                    *nested_number,
                    nesting_level + 1,
                );
            }
        }
    }

    /// Renders a table with headers, alignment information, and rows.
    ///
    /// Each row is a vector of cells.
    ///
    /// The table is rendered using genpdfi's TableLayout with proper column weights
    /// and cell borders. Each cell content is processed as inline tokens to handle
    /// formatting within table them.
    fn render_table(
        &self,
        doc: &mut Document,
        headers: &Vec<Vec<Token>>,
        aligns: &Vec<Alignment>,
        rows: &Vec<Vec<Vec<Token>>>,
    ) {
        doc.push(genpdfi_extended::elements::Break::new(
            self.style.text.before_spacing,
        ));

        let column_count = headers.len();
        let column_weights = vec![1; column_count];

        let mut table = genpdfi_extended::elements::TableLayout::new(column_weights);
        table.set_cell_decorator(genpdfi_extended::elements::FrameCellDecorator::new(
            true, true, false,
        ));

        // Render header row
        let mut header_row = table.row();
        for (i, header_cell) in headers.iter().enumerate() {
            let mut para = genpdfi_extended::elements::Paragraph::default();
            let style =
                genpdfi_extended::style::Style::new().with_font_size(self.style.table_header.size);

            if let Some(align) = aligns.get(i) {
                para.set_alignment(*align);
            }

            self.render_inline_content_with_style_simple(&mut para, header_cell, style);
            header_row.push_element(para);
        }

        if let Err(_) = header_row.push() {
            warn!("Failed rendering a table");
            return; // Skip the entire table if header fails
        }

        // Render data rows
        for (row_idx, row) in rows.iter().enumerate() {
            let mut table_row = table.row();

            for (i, cell_tokens) in row.iter().enumerate() {
                let mut para = genpdfi_extended::elements::Paragraph::default();
                let style = genpdfi_extended::style::Style::new()
                    .with_font_size(self.style.table_cell.size);

                if let Some(align) = aligns.get(i) {
                    para.set_alignment(*align);
                }

                self.render_inline_content_with_style_simple(&mut para, cell_tokens, style);
                table_row.push_element(para);
            }

            if let Err(_) = table_row.push() {
                warn!("Failed to push row {} in a table", row_idx);
                continue; // Continue with next row
            }
        }

        doc.push(table);
        doc.push(genpdfi_extended::elements::Break::new(
            self.style.text.after_spacing,
        ));
    }

    /// Renders an image token as a block-level element in the document.
    ///
    /// Attempts to load the image from the configured ImageLoader and embed it
    /// into the PDF. For SVG images, uses native SVG rendering with configurable sizing.
    /// For raster formats, uses Image::from_reader(). If loading fails or no loader is
    /// configured, renders the alt text.
    ///
    /// # SVG Sizing Configuration
    ///
    /// SVG images can be sized using the `[image.svg]` configuration section:
    ///
    /// - `width`: Percentage of page width (e.g., "50%")
    ///   - When specified, completely overrides `scale_factor`
    ///   - Example: `width = "50%"` renders at 50% of page width
    ///
    /// - `scale_factor`: Multiplier of original SVG dimensions (default: 1.0)
    ///   - Only used when `width` is not specified
    ///   - Scales the intrinsic SVG dimensions (from width/height attributes)
    ///   - Example: `scale_factor = 2.0` renders at 200% of original size
    ///   - Example: `scale_factor = 0.5` renders at 50% of original size
    ///
    /// # Priority
    ///
    /// If both `width` and `scale_factor` are specified, `width` takes priority
    /// and `scale_factor` is completely ignored.
    ///
    /// # Examples
    ///
    /// With `markdown2pdfrc.toml`:
    /// ```toml
    /// [image.svg]
    /// width = "50%"     # SVG will be 50% of page width
    /// ```
    ///
    /// Or with scale_factor only:
    /// ```toml
    /// [image.svg]
    /// scale_factor = 0.5  # SVG will be 50% of its original size
    /// ```
    ///
    /// Or with scale_factor = 2.0:
    /// ```toml
    /// [image.svg]
    /// scale_factor = 2.0  # SVG will be 200% of its original size
    /// ```
    fn render_image(&self, doc: &mut Document, alt: &str, url: &str) {
        doc.push(genpdfi_extended::elements::Break::new(0.5));

        let mut loader_opt = self.image_loader.borrow_mut();

        if let Some(ref mut loader) = *loader_opt {
            match loader.load(url) {
                Ok(image_data) => {
                    // Try to load the image based on its format
                    match image_data.format {
                        crate::images::ImageFormat::Svg => {
                            // For SVG, use native SVG rendering with configuration
                            match String::from_utf8(image_data.bytes.clone()) {
                                Ok(svg_string) => {
                                    match genpdfi_extended::elements::Image::from_svg_string(
                                        &svg_string,
                                    ) {
                                        Ok(image) => {
                                            // Apply width and scale_factor configuration
                                            // Priority: width > scale_factor (width surcharges scale_factor)
                                            //
                                            // width: percentage of page width (e.g., "50%")
                                            //        When specified, scale_factor is completely ignored
                                            // scale_factor: multiplier of original SVG dimensions (1.0 = original, 0.5 = 50%, 2.0 = 200%)
                                            //              Only used when width is not specified
                                            //              Scales the intrinsic SVG dimensions by this factor
                                            let image = match self.style.svg_config.width {
                                                SvgWidth::Percentage(percent) => {
                                                    // width surcharges scale_factor
                                                    image.resizing_page_with(percent / 100.0)
                                                }
                                                SvgWidth::Pixels(_pixels) => {
                                                    // For pixel-based sizing, genpdfi only supports page-fraction sizing
                                                    // So we can't properly implement this yet
                                                    image
                                                }
                                                SvgWidth::Auto => {
                                                    // When width is not specified, use scale_factor
                                                    // scale_factor multiplies the intrinsic SVG dimensions
                                                    if self.style.svg_config.scale_factor != 1.0 {
                                                        image.with_scale(Scale::new(
                                                            self.style.svg_config.scale_factor,
                                                            self.style.svg_config.scale_factor,
                                                        ))
                                                    } else {
                                                        image
                                                    }
                                                }
                                            };

                                            let resized_image =
                                                image.with_alignment(Alignment::Center);
                                            doc.push(resized_image);
                                        }
                                        Err(e) => {
                                            warn!("Failed to render SVG: {}", e);
                                            let mut para =
                                                genpdfi_extended::elements::Paragraph::default();
                                            let style = genpdfi_extended::style::Style::new()
                                                .with_font_size(self.style.text.size)
                                                .italic();
                                            para.push_styled(
                                                format!("[SVG Image: {}]", alt),
                                                style,
                                            );
                                            doc.push(para);
                                        }
                                    }
                                }
                                Err(e) => {
                                    warn!("Failed to decode SVG as UTF-8: {}", e);
                                    let mut para = genpdfi_extended::elements::Paragraph::default();
                                    let style = genpdfi_extended::style::Style::new()
                                        .with_font_size(self.style.text.size)
                                        .italic();
                                    para.push_styled(format!("[SVG Image: {}]", alt), style);
                                    doc.push(para);
                                }
                            }
                        }
                        _ => {
                            // For raster formats (JPEG, PNG, WebP, GIF), use from_reader
                            match genpdfi_extended::elements::Image::from_reader(
                                std::io::Cursor::new(image_data.bytes),
                            ) {
                                Ok(image) => {
                                    let resized_image = image
                                        .resizing_page_with(0.8)
                                        .with_alignment(Alignment::Center);
                                    doc.push(resized_image);
                                }
                                Err(e) => {
                                    warn!("Failed to create image from data: {}", e);
                                    let mut para = genpdfi_extended::elements::Paragraph::default();
                                    let style = genpdfi_extended::style::Style::new()
                                        .with_font_size(self.style.text.size)
                                        .italic();
                                    para.push_styled(format!("[Image: {}]", alt), style);
                                    doc.push(para);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to load image {}: {}", url, e);
                    let mut para = genpdfi_extended::elements::Paragraph::default();
                    let style = genpdfi_extended::style::Style::new()
                        .with_font_size(self.style.text.size)
                        .italic();
                    para.push_styled(format!("[Image not found: {}]", alt), style);
                    doc.push(para);
                }
            }
        } else {
            // No loader configured, just show alt text
            let mut para = genpdfi_extended::elements::Paragraph::default();
            let style = genpdfi_extended::style::Style::new()
                .with_font_size(self.style.text.size)
                .italic();
            para.push_styled(format!("[Image: {}]", alt), style);
            doc.push(para);
        }

        doc.push(genpdfi_extended::elements::Break::new(0.5));
    }

    /// Renders an image with a hyperlink ([![alt](image)](url))
    fn render_image_with_link(
        &self,
        doc: &mut Document,
        alt: &str,
        image_url: &str,
        link_url: &str,
    ) {
        doc.push(genpdfi_extended::elements::Break::new(0.5));

        let mut loader_opt = self.image_loader.borrow_mut();

        if let Some(ref mut loader) = *loader_opt {
            match loader.load(image_url) {
                Ok(image_data) => {
                    // Try to load the image based on its format
                    match image_data.format {
                        crate::images::ImageFormat::Svg => {
                            // For SVG, use native SVG rendering with configuration
                            match String::from_utf8(image_data.bytes.clone()) {
                                Ok(svg_string) => {
                                    match genpdfi_extended::elements::Image::from_svg_string(
                                        &svg_string,
                                    ) {
                                        Ok(image) => {
                                            // Apply width and scale_factor configuration
                                            let image = match self.style.svg_config.width {
                                                SvgWidth::Percentage(percent) => {
                                                    image.resizing_page_with(percent / 100.0)
                                                }
                                                SvgWidth::Pixels(_pixels) => image,
                                                SvgWidth::Auto => {
                                                    if self.style.svg_config.scale_factor != 1.0 {
                                                        image.with_scale(Scale::new(
                                                            self.style.svg_config.scale_factor,
                                                            self.style.svg_config.scale_factor,
                                                        ))
                                                    } else {
                                                        image
                                                    }
                                                }
                                            };

                                            let resized_image = image
                                                .with_link(link_url.to_string())
                                                .with_alignment(Alignment::Center);
                                            doc.push(resized_image);
                                        }
                                        Err(e) => {
                                            warn!("Failed to render SVG with link: {}", e);
                                            let mut para =
                                                genpdfi_extended::elements::Paragraph::default();
                                            let style = genpdfi_extended::style::Style::new()
                                                .with_font_size(self.style.text.size)
                                                .italic();
                                            para.push_styled(
                                                format!("[SVG Image: {}]", alt),
                                                style,
                                            );
                                            doc.push(para);
                                        }
                                    }
                                }
                                Err(e) => {
                                    warn!("Failed to decode SVG as UTF-8: {}", e);
                                    let mut para = genpdfi_extended::elements::Paragraph::default();
                                    let style = genpdfi_extended::style::Style::new()
                                        .with_font_size(self.style.text.size)
                                        .italic();
                                    para.push_styled(format!("[SVG Image: {}]", alt), style);
                                    doc.push(para);
                                }
                            }
                        }
                        _ => {
                            // For raster formats (JPEG, PNG, WebP, GIF), use from_reader
                            match genpdfi_extended::elements::Image::from_reader(
                                std::io::Cursor::new(image_data.bytes),
                            ) {
                                Ok(image) => {
                                    let resized_image = image
                                        .resizing_page_with(0.8)
                                        .with_link(link_url.to_string())
                                        .with_alignment(Alignment::Center);
                                    doc.push(resized_image);
                                }
                                Err(e) => {
                                    warn!("Failed to create image with link from data: {}", e);
                                    let mut para = genpdfi_extended::elements::Paragraph::default();
                                    let style = genpdfi_extended::style::Style::new()
                                        .with_font_size(self.style.text.size)
                                        .italic();
                                    para.push_styled(format!("[Image: {}]", alt), style);
                                    doc.push(para);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to load image {}: {}", image_url, e);
                    let mut para = genpdfi_extended::elements::Paragraph::default();
                    let style = genpdfi_extended::style::Style::new()
                        .with_font_size(self.style.text.size)
                        .italic();
                    para.push_styled(format!("[Image not found: {}]", alt), style);
                    doc.push(para);
                }
            }
        } else {
            // No loader configured, just show alt text
            let mut para = genpdfi_extended::elements::Paragraph::default();
            let style = genpdfi_extended::style::Style::new()
                .with_font_size(self.style.text.size)
                .italic();
            para.push_styled(format!("[Image: {}]", alt), style);
            doc.push(para);
        }

        doc.push(genpdfi_extended::elements::Break::new(0.5));
    }

    /// Renders a display math block ($$...$$).
    ///
    /// This method converts LaTeX mathematical expressions to SVG and embeds them
    /// as centered images in the PDF. Display math is rendered as a block-level
    /// element with appropriate spacing. The rendering uses dimensional metrics to
    /// ensure consistent sizing with the surrounding text.
    #[cfg(feature = "latex")]
    fn render_math_block(&self, doc: &mut Document, latex_content: &str) {
        // Use genpdfi_extended's native Latex element when the feature is enabled.
        // Size in points is derived from the configured text size (slightly larger for display math).
        let size_pt = (self.style.text.size as f32) * 1.2;
        let mut latex_elem = genpdfi_extended::elements::Latex::new(latex_content.to_string(), size_pt);
        // center display math
        let latex_elem = latex_elem.with_alignment(Alignment::Center);
        doc.push(latex_elem);
    }

    #[cfg(not(feature = "latex"))]
    fn render_math_block(&self, doc: &mut Document, _latex_content: &str) {
        // Feature disabled: show an informative message instead of rendering
        let mut para = genpdfi_extended::elements::Paragraph::default();
        let style = genpdfi_extended::style::Style::new()
            .with_font_size(self.style.text.size)
            .italic();
        para.push_styled("need LaTeX feature".to_string(), style);
        doc.push(para);
    }

    /// Safely render LaTeX to SVG with error recovery.
    ///
    /// This wraps the LaTeX rendering call with error handling to prevent
    /// panics from underlying C++ dependencies.
    /// Legacy stub: LaTeX rendering is now provided by the `genpdfi_extended` crate
    /// when the "latex" feature is enabled. These functions remain as safe stubs
    /// to avoid panics in older code paths and return a clear error when used.
    fn safe_latex_to_svg(&self, _content: &str, _display: bool) -> Result<String, String> {
        Err("need LaTeX feature".to_string())
    }

    /// Legacy stub for compatibility. Returns an error when called.
    fn safe_latex_to_svg_with_metrics(
        &self,
        _content: &str,
        _display: bool,
        _target_height: f32,
    ) -> Result<(String, f32), String> {
        Err("need LaTeX feature".to_string())
    }

    /// Renders inline math ($...$).
    ///
    /// This method converts inline LaTeX expressions to SVG and embeds them
    /// as inline images in the paragraph. The rendering uses dimensional metrics
    /// to ensure consistent sizing with the surrounding text.
    #[cfg(feature = "latex")]
    fn render_inline_math_as_image(&self, doc: &mut Document, latex_content: &str) {
        // Use genpdfi_extended's Latex element in inline mode and size it slightly smaller
        let size_pt = (self.style.text.size as f32) * 0.9;
        let latex_elem = genpdfi_extended::elements::Latex::new(latex_content.to_string(), size_pt);
        doc.push(latex_elem);
    }

    #[cfg(not(feature = "latex"))]
    fn render_inline_math_as_image(&self, doc: &mut Document, _latex_content: &str) {
        // Feature disabled: show a small placeholder
        let mut para = genpdfi_extended::elements::Paragraph::default();
        let style = genpdfi_extended::style::Style::new()
            .with_font_size(self.style.text.size)
            .italic();
        para.push_styled("need LaTeX feature".to_string(), style);
        doc.push(para);
    }

    fn render_inline_math(
        &self,
        para: &mut genpdfi_extended::elements::Paragraph,
        latex_content: &str,
        style: genpdfi_extended::style::Style,
    ) {
        // For inline math in paragraphs, we render the LaTeX code as styled text
        // This ensures consistent rendering with the surrounding text and proper baseline alignment
        let mut math_style = style.clone();

        // Apply styling for code/math content
        if let Some(color) = self.style.code.text_color {
            math_style = math_style.with_color(genpdfi_extended::style::Color::Rgb(
                color.0, color.1, color.2,
            ));
        }

        // Render the LaTeX source as styled text
        para.push_styled(latex_content.to_string(), math_style);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::styling::StyleMatch;

    // Helper function to create a basic PDF instance for testing
    fn create_test_pdf(tokens: Vec<Token>) -> Pdf {
        Pdf::new(tokens, StyleMatch::default(), None)
    }

    #[test]
    fn test_pdf_creation() {
        let pdf = create_test_pdf(vec![]);
        assert!(pdf.input.is_empty());

        // Test that both font families exist
        let _font_family = &pdf.font_family;
        let _code_font_family = &pdf.code_font_family;

        // Since FontData's fields are private and it doesn't implement comparison traits,
        // we can only verify that the PDF was created successfully with these fonts
        let doc = pdf.render_into_document();
        assert!(Pdf::render(doc, "/dev/null").is_none());
    }

    #[test]
    fn test_render_heading() {
        let tokens = vec![
            Token::Heading(vec![Token::Text("Test Heading".to_string())], 1),
            Token::Heading(vec![Token::Text("Subheading".to_string())], 2),
            Token::Heading(vec![Token::Text("Sub-subheading".to_string())], 3),
        ];
        let pdf = create_test_pdf(tokens);
        let doc = pdf.render_into_document();
        // Document should be created successfully
        assert!(Pdf::render(doc, "/dev/null").is_none());
    }

    #[test]
    fn test_render_paragraphs() {
        let tokens = vec![
            Token::Text("First paragraph".to_string()),
            Token::Newline,
            Token::Text("Second paragraph".to_string()),
        ];
        let pdf = create_test_pdf(tokens);
        let doc = pdf.render_into_document();
        assert!(Pdf::render(doc, "/dev/null").is_none());
    }

    #[test]
    fn test_render_list_items() {
        let tokens = vec![
            Token::ListItem {
                content: vec![Token::Text("First item".to_string())],
                ordered: false,
                number: None,
            },
            Token::ListItem {
                content: vec![Token::Text("Second item".to_string())],
                ordered: true,
                number: Some(1),
            },
        ];
        let pdf = create_test_pdf(tokens);
        let doc = pdf.render_into_document();
        assert!(Pdf::render(doc, "/dev/null").is_none());
    }

    #[test]
    fn test_render_nested_list_items() {
        let tokens = vec![Token::ListItem {
            content: vec![
                Token::Text("Parent item".to_string()),
                Token::ListItem {
                    content: vec![Token::Text("Child item".to_string())],
                    ordered: false,
                    number: None,
                },
            ],
            ordered: false,
            number: None,
        }];
        let pdf = create_test_pdf(tokens);
        let doc = pdf.render_into_document();
        assert!(Pdf::render(doc, "/dev/null").is_none());
    }

    #[test]
    fn test_render_code_blocks() {
        let tokens = vec![Token::Code(
            "rust".to_string(),
            "fn main() {\n    println!(\"Hello\");\n}".to_string(),
        )];
        let pdf = create_test_pdf(tokens);
        let doc = pdf.render_into_document();
        assert!(Pdf::render(doc, "/dev/null").is_none());
    }

    #[test]
    fn test_render_inline_formatting() {
        let tokens = vec![
            Token::Text("Normal ".to_string()),
            Token::Emphasis {
                level: 1,
                content: vec![Token::Text("italic".to_string())],
            },
            Token::Text(" and ".to_string()),
            Token::StrongEmphasis(vec![Token::Text("bold".to_string())]),
            Token::Text(" text".to_string()),
        ];
        let pdf = create_test_pdf(tokens);
        let doc = pdf.render_into_document();
        assert!(Pdf::render(doc, "/dev/null").is_none());
    }

    #[test]
    fn test_render_links() {
        let tokens = vec![
            Token::Text("Here is a ".to_string()),
            Token::Link("link".to_string(), "https://example.com".to_string()),
            Token::Text(" to click".to_string()),
        ];
        let pdf = create_test_pdf(tokens);
        let doc = pdf.render_into_document();
        assert!(Pdf::render(doc, "/dev/null").is_none());
    }

    #[test]
    fn test_render_horizontal_rule() {
        let tokens = vec![
            Token::Text("Before rule".to_string()),
            Token::HorizontalRule,
            Token::Text("After rule".to_string()),
        ];
        let pdf = create_test_pdf(tokens);
        let doc = pdf.render_into_document();
        assert!(Pdf::render(doc, "/dev/null").is_none());
    }

    #[test]
    fn test_render_mixed_content() {
        let tokens = vec![
            Token::Heading(vec![Token::Text("Title".to_string())], 1),
            Token::Text("Some text ".to_string()),
            Token::Link("with link".to_string(), "https://example.com".to_string()),
            Token::Newline,
            Token::ListItem {
                content: vec![Token::Text("List item".to_string())],
                ordered: false,
                number: None,
            },
            Token::Code("rust".to_string(), "let x = 42;".to_string()),
        ];
        let pdf = create_test_pdf(tokens);
        let doc = pdf.render_into_document();
        assert!(Pdf::render(doc, "/dev/null").is_none());
    }

    #[test]
    fn test_render_empty_content() {
        let pdf = create_test_pdf(vec![]);
        let doc = pdf.render_into_document();
        assert!(Pdf::render(doc, "/dev/null").is_none());
    }

    #[test]
    fn test_render_invalid_path() {
        let pdf = create_test_pdf(vec![Token::Text("Test".to_string())]);
        let doc = pdf.render_into_document();
        let result = Pdf::render(doc, "/nonexistent/path/file.pdf");
        assert!(result.is_some()); // Should return an error message
    }

    #[test]
    fn test_render_to_bytes() {
        let tokens = vec![
            Token::Heading(vec![Token::Text("Test Document".to_string())], 1),
            Token::Text("This is a test paragraph.".to_string()),
        ];
        let pdf = create_test_pdf(tokens);
        let doc = pdf.render_into_document();
        let result = Pdf::render_to_bytes(doc);

        assert!(result.is_ok());
        let pdf_bytes = result.unwrap();
        assert!(!pdf_bytes.is_empty());
        // PDF files should start with "%PDF-"
        assert!(pdf_bytes.starts_with(b"%PDF-"));
    }

    #[test]
    fn test_render_to_bytes_empty_document() {
        let pdf = create_test_pdf(vec![]);
        let doc = pdf.render_into_document();
        let result = Pdf::render_to_bytes(doc);

        assert!(result.is_ok());
        let pdf_bytes = result.unwrap();
        assert!(!pdf_bytes.is_empty());
        assert!(pdf_bytes.starts_with(b"%PDF-"));
    }

    #[test]
    fn test_render_to_bytes_complex_content() {
        let tokens = vec![
            Token::Heading(vec![Token::Text("Main Title".to_string())], 1),
            Token::Text("Introduction paragraph.".to_string()),
            Token::Heading(vec![Token::Text("Section 1".to_string())], 2),
            Token::ListItem {
                content: vec![Token::Text("First item".to_string())],
                ordered: false,
                number: None,
            },
            Token::ListItem {
                content: vec![Token::Text("Second item".to_string())],
                ordered: false,
                number: None,
            },
            Token::Code(
                "rust".to_string(),
                "fn main() {\n    println!(\"Hello\");\n}".to_string(),
            ),
            Token::Link(
                "Example Link".to_string(),
                "https://example.com".to_string(),
            ),
        ];
        let pdf = create_test_pdf(tokens);
        let doc = pdf.render_into_document();
        let result = Pdf::render_to_bytes(doc);

        assert!(result.is_ok());
        let pdf_bytes = result.unwrap();
        assert!(!pdf_bytes.is_empty());
        assert!(pdf_bytes.starts_with(b"%PDF-"));
    }
}
