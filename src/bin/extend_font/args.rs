/// Command-line argument parsing module
///
/// This module defines the CLI structure using clap with derive macros.
/// It supports:
/// - Standard verbosity flags: -v, -vv, -vvv
/// - Quiet flag: -q
/// - Verbose level flag: --verbose=LEVEL
/// - Environment variable RUST_LOG integration

use clap::Parser;
use std::path::PathBuf;

/// Extend a font by copying missing glyphs from another font
///
/// This utility reads two font files (source and combine), detects missing glyphs
/// in the source font, and copies them from the combine font. Optionally converts
/// variable weight fonts to fixed weight to match the source font's weight class.
#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct ExtendFontArgs {
    /// Path to the source font file to be extended
    #[arg(long, value_name = "PATH")]
    pub src_font: PathBuf,

    /// Path to the combine font file containing glyphs to copy
    #[arg(long, value_name = "PATH")]
    pub combine_font: PathBuf,

    /// Optional output path for the extended font
    /// If not specified, the source font will be modified in place.
    /// If the path contains a directory component, it will be created if it doesn't exist.
    #[arg(long, value_name = "PATH")]
    pub dst_font: Option<PathBuf>,

    /// Increase verbosity level (-v for debug, -vv for trace, -vvv for more trace)
    #[arg(short, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Decrease verbosity level (suppress most output except errors)
    #[arg(short)]
    pub quiet: bool,

    /// Set explicit verbosity level (trace, debug, info, warn, error)
    #[arg(long, value_name = "LEVEL")]
    pub verbose_level: Option<String>,
}

impl ExtendFontArgs {
    /// Determine the effective log level based on CLI flags and environment variables
    ///
    /// Priority:
    /// 1. RUST_LOG environment variable (highest priority)
    /// 2. --verbose-level flag
    /// 3. Count of -v flags or -q flag (lowest priority)
    pub fn effective_log_level(&self) -> String {
        // RUST_LOG environment variable takes precedence
        if let Ok(rust_log) = std::env::var("RUST_LOG") {
            return rust_log;
        }

        // Explicit verbose level
        if let Some(level) = &self.verbose_level {
            return level.clone();
        }

        // Quiet flag
        if self.quiet {
            return "error".to_string();
        }

        // Count of -v flags
        match self.verbose {
            0 => "info".to_string(),
            1 => "debug".to_string(),
            _ => "trace".to_string(),
        }
    }
}
