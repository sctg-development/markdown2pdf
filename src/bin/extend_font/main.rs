/// Font glyph extension utility binary entry point
///
/// This binary extends a source font by copying missing glyphs from a combine font.
///
/// # Examples
///
/// Basic usage:
/// ```sh
/// cargo run --bin extend_font -- --src-font fonts/DejaVuSans.ttf --combine-font fonts/NotoEmoji-VariableFont_wght.ttf
/// ```
///
/// With verbosity control:
/// ```sh
/// cargo run --bin extend_font -- -vv --src-font fonts/DejaVuSans.ttf --combine-font fonts/NotoEmoji-VariableFont_wght.ttf
/// ```
use clap::Parser;
use log::{debug, error, info};
use std::fs;
use std::process;

mod args;
mod cmap_builder;
mod font_subsetter;
mod font_utils;
mod glyph_copier;
mod glyph_copier_impl;
mod glyf_binary_merger;
mod logging;

use args::ExtendFontArgs;
use font_utils::FontInfo;
use glyph_copier::copy_missing_glyphs;
use logging::init_logging;

fn main() {
    let args = ExtendFontArgs::parse();

    // Initialize logging based on CLI arguments and RUST_LOG environment variable
    if let Err(e) = init_logging(&args) {
        eprintln!("Failed to initialize logging: {}", e);
        process::exit(1);
    }

    debug!("Parsed arguments: {:?}", args);

    // Run the font extension process
    if let Err(e) = run(&args) {
        error!("Error: {}", e);
        process::exit(1);
    }

    info!("Font extension completed successfully");
}

/// Main entry point for font extension logic
fn run(args: &ExtendFontArgs) -> Result<(), Box<dyn std::error::Error>> {
    // Read source font
    debug!("Reading source font from: {:?}", args.src_font);
    let src_font_data = fs::read(&args.src_font)?;
    let src_font = FontInfo::from_bytes(&src_font_data)?;
    info!(
        "Source font '{}' loaded with {} glyphs",
        src_font.name(),
        src_font.glyph_count()
    );

    // Read combine font
    debug!("Reading combine font from: {:?}", args.combine_font);
    let combine_font_data = fs::read(&args.combine_font)?;
    let combine_font = FontInfo::from_bytes(&combine_font_data)?;
    info!(
        "Combine font '{}' loaded with {} glyphs",
        combine_font.name(),
        combine_font.glyph_count()
    );

    // Determine output path
    let output_path = args.dst_font.as_ref().cloned().unwrap_or_else(|| {
        info!("No destination specified, will modify source font in place");
        args.src_font.clone()
    });

    // Create destination directory if needed
    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            debug!("Creating destination directory: {:?}", parent);
            fs::create_dir_all(parent)?;
            info!("Created directory structure for: {:?}", parent);
        }
    }

    // Copy missing glyphs
    debug!("Starting glyph copy process");
    let extended_font_data = copy_missing_glyphs(
        &src_font_data,
        &combine_font_data,
        &src_font,
        &combine_font,
    )?;

    // Write output font
    debug!("Writing extended font to: {:?}", output_path);
    fs::write(&output_path, extended_font_data)?;
    info!("Extended font written to: {:?}", output_path);

    Ok(())
}
