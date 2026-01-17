/// Module Architecture and Code Organization
///
/// ## Overview
///
/// The `extend_font` binary is organized into 4 specialized modules that handle
/// different aspects of the font extension process.
///
/// ## Module Structure
///
/// ```text
/// src/bin/extend_font/
/// ├── mod.rs                    # Module declarations
/// ├── args.rs                   # CLI argument parsing (clap derive)
/// ├── font_utils.rs             # Font reading and glyph analysis (skrifa)
/// ├── glyph_copier.rs           # Core glyph copying logic
/// └── logging.rs                # Logging initialization (env_logger)
/// 
/// src/bin/extend_font.rs        # Main entry point
/// ```
///
/// ## Module Details
///
/// ### extend_font.rs (Main Entry Point)
///
/// **Purpose**: Binary entry point and main orchestration
///
/// **Responsibilities**:
/// - Parse command line arguments
/// - Initialize logging
/// - Read font files
/// - Orchestrate the glyph copying process
/// - Handle errors and exit codes
///
/// **Key Functions**:
/// - `main()`: Entry point, parses args and calls run()
/// - `run(args)`: Main logic orchestrating all operations
///
/// **Dependencies**: clap, log, std::fs
///
/// ### args.rs (CLI Argument Parsing)
///
/// **Purpose**: Define and parse command line arguments using clap derive macros
///
/// **Responsibilities**:
/// - Define CLI argument structure
/// - Implement verbosity level priority logic
/// - Provide argument validation
///
/// **Key Components**:
/// ```rust
/// #[derive(Parser)]
/// pub struct ExtendFontArgs {
///     #[arg(long)] pub src_font: PathBuf,
///     #[arg(long)] pub combine_font: PathBuf,
///     #[arg(long)] pub dst_font: Option<PathBuf>,
///     #[arg(short, action = clap::ArgAction::Count)] pub verbose: u8,
///     #[arg(short)] pub quiet: bool,
///     #[arg(long)] pub verbose_level: Option<String>,
/// }
/// ```
///
/// **Key Methods**:
/// - `effective_log_level()`: Determines final log level based on priority
///
/// **Dependencies**: clap with derive feature, std::path::PathBuf, std::env
///
/// ### logging.rs (Logging Initialization)
///
/// **Purpose**: Initialize env_logger with appropriate verbosity from CLI args
///
/// **Responsibilities**:
/// - Initialize the logging system
/// - Set appropriate log level based on args
/// - Format log output
/// - Handle RUST_LOG environment variable
///
/// **Key Functions**:
/// - `init_logging(args)`: Initialize logger with arguments
///
/// **Priority Order** (highest to lowest):
/// 1. RUST_LOG environment variable
/// 2. --verbose-level CLI flag
/// 3. -v/-q flag counts
/// 4. Default: "info"
///
/// **Dependencies**: env_logger, log::LevelFilter
///
/// ### font_utils.rs (Font Reading and Analysis)
///
/// **Purpose**: High-level abstractions for font operations using skrifa
///
/// **Responsibilities**:
/// - Parse font files and validate format
/// - Analyze font properties (weight, variation axes)
/// - Extract glyph information and Unicode mappings
/// - Compare glyph sets between fonts
///
/// **Key Struct**:
/// ```rust
/// pub struct FontInfo {
///     font_data: Vec<u8>,
/// }
/// ```
///
/// **Key Methods**:
/// - `from_bytes(data)`: Load and validate font
/// - `is_variable_weight()`: Check for variation axes
/// - `weight_class()`: Get font weight (400, 700, etc.)
/// - `glyphs()`: Get all Unicode codepoints with glyphs
/// - `missing_glyphs(other)`: Find glyphs in other but not self
/// - `has_glyph(codepoint)`: Check single codepoint
///
/// **API Details**:
/// - Uses `skrifa::FontRef::new()` to parse font data
/// - Implements `MetadataProvider` trait patterns
/// - Charmap iteration returns (u32, GlyphId) tuples
/// - Weight accessed via `attributes().weight.value()`
///
/// **Dependencies**: skrifa with MetadataProvider trait, std::collections::HashSet, log
///
/// ### glyph_copier.rs (Glyph Copying Logic)
///
/// **Purpose**: Core logic for copying glyphs between fonts
///
/// **Responsibilities**:
/// - Identify missing glyphs
/// - Detect weight conversion needs
/// - Extract glyph information from source fonts
/// - Merge glyphs into destination font
///
/// **Key Function**:
/// ```rust
/// pub fn copy_missing_glyphs(
///     src_font_data: &[u8],
///     combine_font_data: &[u8],
///     src_font: &FontInfo,
///     combine_font: &FontInfo,
/// ) -> Result<Vec<u8>, Box<dyn std::error::Error>>
/// ```
///
/// **Key Struct**:
/// ```rust
/// pub struct GlyphInfo {
///     pub codepoint: u32,
///     pub glyph_id: u32,
///     pub bbox: Option<(i16, i16, i16, i16)>,
///     pub has_outlines: bool,
///     pub advance_width: Option<i32>,
/// }
/// ```
///
/// **Helper Functions**:
/// - `extract_missing_glyph_info()`: Get glyph info from combine font
/// - `merge_glyphs_into_font()`: Merge glyphs into source font
/// - `extract_glyph_info()`: Extract single glyph information
///
/// **Dependencies**: crate::font_utils, skrifa, std::collections::HashSet, log
///
/// ## Data Flow
///
/// ```
/// main()
///   └─> parse arguments (args::ExtendFontArgs)
///        └─> initialize logging (logging::init_logging)
///             └─> run(args)
///                  ├─> read src font file
///                  │    └─> FontInfo::from_bytes(data)
///                  ├─> read combine font file
///                  │    └─> FontInfo::from_bytes(data)
///                  ├─> call glyph_copier::copy_missing_glyphs()
///                  │    ├─> find missing glyphs (src_font.missing_glyphs(combine))
///                  │    ├─> detect weight conversion needs
///                  │    ├─> extract glyph info (extract_missing_glyph_info)
///                  │    └─> merge glyphs (merge_glyphs_into_font)
///                  └─> write result to file
///                       └─> create parent directories if needed
/// ```
///
/// ## Error Handling
///
/// All public functions return `Result<T, Box<dyn std::error::Error>>` to provide
/// flexible error handling:
///
/// - File I/O errors are propagated with context
/// - Invalid font data is caught early with validation
/// - Missing fonts result in clear error messages
/// - All errors are logged before returning
///
/// ## Testing
///
/// Integration tests in `tests/extend_font_integration.rs` validate:
///
/// - CLI argument parsing
/// - Missing glyph detection
/// - Variable weight font detection
/// - Verbosity level control
/// - Error handling
/// - Help and version output
///
/// Tests use real font files: DejaVuSans.ttf and NotoEmoji-VariableFont_wght.ttf
///
/// ## Performance Considerations
///
/// 1. **Memory Usage**: Full font files are loaded into memory (typical: 400KB-2MB)
/// 2. **Glyph Analysis**: HashSet comparisons are O(n) where n = glyph count
/// 3. **Font Parsing**: skrifa's lazy parsing minimizes unnecessary work
/// 4. **Logging**: Debug/trace level output has minimal performance impact when not enabled
///
/// ## Future Implementation Notes
///
/// When implementing actual glyph copying:
///
/// 1. Consider using write-fonts for font reconstruction
/// 2. Implement proper charmap table merging
/// 3. Add glyph outline and metrics copying
/// 4. Handle variable font instances and weight conversion
/// 5. Optimize for large font files with many glyphs
