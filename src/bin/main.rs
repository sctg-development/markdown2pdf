use clap::{Arg, Command};
use log::{debug, error, info, warn};
use markdown2pdf::validation;
#[cfg(feature = "fetch")]
use reqwest::blocking::Client;
use std::fs;
use std::path::PathBuf;
use std::process;

#[derive(Debug)]
enum AppError {
    FileReadError(std::io::Error),
    ConversionError(String),
    PathError(String),
    #[cfg(feature = "fetch")]
    NetworkError(String),
}

/// Verbosity level for output
#[derive(Debug, Clone, Copy, PartialEq)]
enum Verbosity {
    Quiet,   // No output except errors
    Normal,  // Standard output
    Verbose, // Detailed output
}

fn get_markdown_input(matches: &clap::ArgMatches) -> Result<String, AppError> {
    if let Some(file_path) = matches.get_one::<String>("path") {
        fs::read_to_string(file_path).map_err(|e| AppError::FileReadError(e))
    } else {
        #[cfg(feature = "fetch")]
        if let Some(_url) = matches.get_one::<String>("url") {
            return Client::new()
                .get(_url)
                .send()
                .map_err(|e| AppError::NetworkError(e.to_string()))?
                .text()
                .map_err(|e| AppError::NetworkError(e.to_string()));
        }

        if let Some(markdown_string) = matches.get_one::<String>("string") {
            Ok(markdown_string.to_string())
        } else {
            Err(AppError::ConversionError("No input provided".to_string()))
        }
    }
}

/// Get the markdown document path if provided as a file.
fn get_markdown_path(matches: &clap::ArgMatches) -> Option<PathBuf> {
    matches.get_one::<String>("path").map(PathBuf::from)
}

/// Get the configuration source based on CLI arguments or default behavior.
///
/// Priority order:
/// 1. If `--config` is explicitly provided, use that file
/// 2. If `markdown2pdfrc.toml` exists in current directory, use it
/// 3. Otherwise use default configuration
///
/// # Arguments
/// * `matches` - The parsed command-line arguments
///
/// # Returns
/// A `ConfigSource` that specifies where to load configuration from
fn get_config_source(matches: &clap::ArgMatches) -> markdown2pdf::config::ConfigSource {
    // Check if --config was explicitly provided
    if let Some(config_file) = matches.get_one::<String>("config") {
        return markdown2pdf::config::ConfigSource::File(Box::leak(
            config_file.to_string().into_boxed_str(),
        ));
    }

    // Check if markdown2pdfrc.toml exists in current directory
    if std::path::Path::new("markdown2pdfrc.toml").exists() {
        return markdown2pdf::config::ConfigSource::File("markdown2pdfrc.toml");
    }

    // Fall back to default configuration
    markdown2pdf::config::ConfigSource::Default
}

fn get_output_path(matches: &clap::ArgMatches) -> Result<PathBuf, AppError> {
    let current_dir = std::env::current_dir().map_err(|e| AppError::PathError(e.to_string()))?;

    Ok(matches
        .get_one::<String>("output")
        .map(|p| current_dir.join(p))
        .unwrap_or_else(|| current_dir.join("output.pdf")))
}

/// Detects whether the markdown contains a mermaid fenced code block (```mermaid)
fn has_mermaid_block(markdown: &str) -> bool {
    for line in markdown.lines() {
        let s = line.trim_start();
        if s.starts_with("```") {
            // remainder after backticks
            let rest = s.trim_start_matches('`').trim_start();
            if rest.to_lowercase().starts_with("mermaid") {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod cli_tests {
    use super::*;

    #[test]
    fn detects_mermaid_block() {
        let md = "Some text\n```mermaid\ngraph LR\nA-->B\n```";
        assert!(has_mermaid_block(md));
    }

    #[test]
    fn no_mermaid_block() {
        let md = "# Title\n```rust\nfn main() {}\n```";
        assert!(!has_mermaid_block(md));
    }
}

fn run(matches: clap::ArgMatches) -> Result<(), AppError> {
    // Determine verbosity level
    let verbosity = if matches.get_flag("quiet") {
        Verbosity::Quiet
    } else if matches.get_flag("verbose") {
        Verbosity::Verbose
    } else {
        Verbosity::Normal
    };

    // Check for dry-run mode
    let dry_run = matches.get_flag("dry-run");

    let markdown = get_markdown_input(&matches)?;
    let markdown_path = get_markdown_path(&matches);
    let output_path = get_output_path(&matches)?;
    let output_path_str = output_path
        .to_str()
        .ok_or_else(|| AppError::PathError("Invalid output path".to_string()))?;

    // Extract font configuration from CLI arguments
    let fallback_fonts: Vec<String> = matches
        .get_many::<String>("fallback-font")
        .map(|values| values.map(|s| s.to_string()).collect())
        .unwrap_or_default();

    let font_config = if matches.contains_id("font-path")
        || matches.contains_id("default-font")
        || matches.contains_id("code-font")
        || !fallback_fonts.is_empty()
    {
        let custom_paths: Vec<PathBuf> = matches
            .get_many::<String>("font-path")
            .map(|values| values.map(PathBuf::from).collect())
            .unwrap_or_default();

        let default_font = matches
            .get_one::<String>("default-font")
            .map(|s| s.to_string());

        let code_font = matches
            .get_one::<String>("code-font")
            .map(|s| s.to_string());

        Some(markdown2pdf::fonts::FontConfig {
            custom_paths,
            default_font,
            code_font,
            fallback_fonts,
            enable_subsetting: true, // Enable subsetting by default for smaller PDFs
        })
    } else {
        None
    };

    // Run validation checks
    if verbosity != Verbosity::Quiet {
        let warnings =
            validation::validate_conversion(&markdown, font_config.as_ref(), Some(output_path_str));

        if !warnings.is_empty() {
            if verbosity == Verbosity::Verbose {
                info!("🔍 Pre-flight validation:");
            }
            for warning in &warnings {
                warn!("{}", warning);
            }
        } else if verbosity == Verbosity::Verbose {
            info!("✓ Pre-flight validation passed");
        }

        // If dry-run, stop here
        if dry_run {
            println!("✓ Dry-run validation complete. No PDF generated.");
            if warnings.is_empty() {
                println!("✓ No issues detected. Run without --dry-run to generate PDF.");
            } else {
                println!("⚠️  {} warning(s) found. Review above and run without --dry-run to generate PDF anyway.", warnings.len());
            }
            return Ok(());
        }
    } else if dry_run {
        let warnings =
            validation::validate_conversion(&markdown, font_config.as_ref(), Some(output_path_str));
        if warnings.is_empty() {
            return Ok(());
        } else {
            return Err(AppError::ConversionError(format!(
                "{} validation warnings",
                warnings.len()
            )));
        }
    }

    // Show missing glyphs if requested (best-effort)
    if matches.get_flag("show-missing-glyphs") {
        match markdown2pdf::fonts::report_missing_glyphs(&markdown, font_config.as_ref()) {
            Ok(results) => {
                println!("🔎 Missing glyphs report:");
                for (font_name, missing) in results {
                    if missing.is_empty() {
                        println!("  • {}: complete coverage", font_name);
                    } else {
                        let s = missing
                            .iter()
                            .map(|c| {
                                let hex = format!("U+{:04X}", *c as u32);
                                let ch = if c.is_control() {
                                    format!("{:?}", c)
                                } else {
                                    c.to_string()
                                };
                                format!("{} ({})", hex, ch)
                            })
                            .collect::<Vec<_>>()
                            .join(", ");
                        println!(
                            "  • {}: missing {} glyph(s): {}",
                            font_name,
                            missing.len(),
                            s
                        );
                    }
                }
            }
            Err(e) => {
                warn!("Could not compute missing glyphs: {}", e);
            }
        }
    }

    // Generate PDF
    if verbosity == Verbosity::Verbose {
        info!("📄 Generating PDF...");
        if let Some(cfg) = &font_config {
            if let Some(font) = &cfg.default_font {
                info!("   Font: {}", font);
            }
            if !cfg.fallback_fonts.is_empty() {
                info!("   Fallbacks: {}", cfg.fallback_fonts.join(", "));
            }
        }
    }

    // If the document contains Mermaid code blocks, notify the user that rendering may be slow
    // because genpdfi_extended uses headless_chrome (Chrome may be downloaded on first run).
    if verbosity != Verbosity::Quiet && has_mermaid_block(&markdown) {
        println!("⚠️  Mermaid blocks detected: rendering uses headless Chrome and may be slow; Chrome may be downloaded on first use.");
    }

    // Use parse_into_file_with_images if we have a document path (for relative image resolution)
    // Otherwise use the basic parse_into_file

    // Determine configuration source based on CLI args or defaults
    let config_source = get_config_source(&matches);

    if let Some(path) = markdown_path {
        markdown2pdf::parse_into_file_with_images(
            markdown,
            output_path_str,
            &path,
            config_source,
            font_config.as_ref(),
        )
        .map_err(|e| AppError::ConversionError(e.to_string()))?;
    } else {
        markdown2pdf::parse_into_file(
            markdown,
            output_path_str,
            config_source,
            font_config.as_ref(),
        )
        .map_err(|e| AppError::ConversionError(e.to_string()))?;
    }

    if verbosity != Verbosity::Quiet {
        println!("✅ Successfully saved PDF to {}", output_path_str);

        // Show file size in verbose mode
        if verbosity == Verbosity::Verbose {
            if let Ok(metadata) = fs::metadata(output_path_str) {
                let size_kb = metadata.len() as f64 / 1024.0;
                if size_kb < 1024.0 {
                    println!("   Size: {:.1} KB", size_kb);
                } else {
                    println!("   Size: {:.2} MB", size_kb / 1024.0);
                }
            }
        }
    }

    Ok(())
}

fn main() {
    // Initialize logger with environment variable control (RUST_LOG)
    env_logger::Builder::from_default_env()
        .format_timestamp_millis()
        .init();

    let cmd = Command::new("markdown2pdf")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Convert Markdown files or strings to PDF")
        .after_help(
            "EXAMPLES:\n  \
            markdown2pdf -p document.md -o output.pdf\n  \
            markdown2pdf -s \"# Hello World\" --default-font \"DejaVu Sans\"\n  \
            markdown2pdf -p doc.md --verbose --dry-run\n  \
            markdown2pdf -p unicode.md --default-font \"Arial\" --fallback-font \"DejaVu Sans\"\n",
        )
        .arg({
            let arg = Arg::new("path")
                .short('p')
                .long("path")
                .value_name("FILE_PATH")
                .help("Path to the markdown file");
            #[cfg(feature = "fetch")]
            let arg = arg.conflicts_with_all(["string", "url"]);
            #[cfg(not(feature = "fetch"))]
            let arg = arg.conflicts_with("string");
            arg
        });

    #[cfg(test)]
    mod tests {
        use super::*;
        use clap::Arg;
        use clap::Command;
        use std::env;
        use std::fs;

        #[test]
        fn test_get_output_path_default_and_custom() {
            // Default
            let cmd = Command::new("test").arg(Arg::new("output").short('o').long("output"));
            let matches = cmd.clone().get_matches_from(vec!["test"]);
            let default_path = get_output_path(&matches).unwrap();
            assert!(default_path.ends_with("output.pdf"));

            // Custom
            let matches = cmd.get_matches_from(vec!["test", "-o", "my.pdf"]);
            let custom_path = get_output_path(&matches).unwrap();
            assert!(custom_path.ends_with("my.pdf"));
        }

        #[test]
        fn test_get_markdown_input_from_string_and_file() {
            // From string
            let cmd = Command::new("test").arg(
                Arg::new("string")
                    .short('s')
                    .long("string")
                    .value_name("MARKDOWN_STRING"),
            );
            let matches = cmd.clone().get_matches_from(vec!["test", "-s", "# Hello"]);
            let s = get_markdown_input(&matches).unwrap();
            assert!(s.contains("Hello"));

            // From file
            let tmp = env::temp_dir().join("md_test_input.md");
            fs::write(&tmp, "# From file").unwrap();
            let cmd = Command::new("test").arg(Arg::new("path").short('p').long("path"));
            let matches = cmd.get_matches_from(vec!["test", "-p", tmp.to_str().unwrap()]);
            let s = get_markdown_input(&matches).unwrap();
            assert!(s.contains("From file"));
            let _ = fs::remove_file(&tmp);
        }

        #[test]
        fn test_list_embedded_flag() {
            let cmd = Command::new("test")
                .arg(Arg::new("list-embedded-fonts").long("list-embedded-fonts"));
            let matches = cmd.get_matches_from(vec!["test", "--list-embedded-fonts"]);
            assert!(matches.get_flag("list-embedded-fonts"));
        }

        #[test]
        fn test_get_default_configuration_flag() {
            let cmd = Command::new("test")
                .arg(Arg::new("get-default-configuration").long("get-default-configuration"));
            let matches = cmd.get_matches_from(vec!["test", "--get-default-configuration"]);
            assert!(matches.get_flag("get-default-configuration"));

            // The default configuration string should parse back into defaults
            let s = markdown2pdf::config::default_config_toml();
            let parsed = markdown2pdf::config::parse_config_string(&s);
            let default = markdown2pdf::styling::StyleMatch::default();
            assert_eq!(parsed.heading_1.size, default.heading_1.size);
            assert_eq!(parsed.mermaid.auto_scale, default.mermaid.auto_scale);
        }

        #[test]
        fn test_show_missing_glyphs_flag() {
            let cmd = Command::new("test")
                .arg(Arg::new("show-missing-glyphs").long("show-missing-glyphs"));
            let matches = cmd.get_matches_from(vec!["test", "--show-missing-glyphs"]);
            assert!(matches.get_flag("show-missing-glyphs"));
        }

        #[test]
        fn test_run_dry_run_returns_ok() {
            let tmp = env::temp_dir().join("md_test_run.md");
            fs::write(&tmp, "# Small").unwrap();

            let cmd = Command::new("test")
                .arg(Arg::new("path").short('p').long("path"))
                .arg(Arg::new("dry-run").long("dry-run"));

            let matches =
                cmd.get_matches_from(vec!["test", "-p", tmp.to_str().unwrap(), "--dry-run"]);
            // run() returns Ok on dry-run when no warnings
            let res = run(matches);
            let _ = fs::remove_file(&tmp);
            assert!(res.is_ok());
        }

        #[test]
        fn test_get_config_source_explicit_config() {
            // Test explicit --config argument takes priority
            let cmd = Command::new("test").arg(Arg::new("config").short('c').long("config"));
            let matches = cmd.get_matches_from(vec!["test", "--config", "custom.toml"]);

            let config_source = get_config_source(&matches);
            match config_source {
                markdown2pdf::config::ConfigSource::File(path) => {
                    assert_eq!(path, "custom.toml");
                }
                _ => panic!("Expected File config source"),
            }
        }

        #[test]
        fn test_get_config_source_default_when_no_args() {
            // Test that Default is returned when no config args and no markdown2pdfrc.toml
            let cmd = Command::new("test").arg(Arg::new("config").short('c').long("config"));
            let matches = cmd.get_matches_from(vec!["test"]);

            // Save current dir and change to a temp dir without markdown2pdfrc.toml
            let original_dir = env::current_dir().unwrap();
            let temp_dir = env::temp_dir().join("md_test_no_config");
            let _ = fs::create_dir(&temp_dir);
            let _ = env::set_current_dir(&temp_dir);

            // Ensure markdown2pdfrc.toml doesn't exist
            let _ = fs::remove_file("markdown2pdfrc.toml");

            let config_source = get_config_source(&matches);
            match config_source {
                markdown2pdf::config::ConfigSource::Default => {
                    // Expected
                }
                _ => panic!("Expected Default config source when no markdown2pdfrc.toml exists"),
            }

            // Cleanup and restore
            let _ = env::set_current_dir(&original_dir);
            let _ = fs::remove_dir_all(&temp_dir);
        }
    }

    #[cfg(feature = "fetch")]
    let cmd = cmd.arg(
        Arg::new("url")
            .short('u')
            .long("url")
            .value_name("URL")
            .help("URL to fetch markdown content from (requires 'fetch' feature)")
            .conflicts_with_all(["string", "path"]),
    );

    let mut cmd = cmd
        .arg({
            let arg = Arg::new("string")
                .short('s')
                .long("string")
                .value_name("MARKDOWN_STRING")
                .help("Markdown content as a string");
            #[cfg(feature = "fetch")]
            let arg = arg.conflicts_with_all(["path", "url"]);
            #[cfg(not(feature = "fetch"))]
            let arg = arg.conflicts_with("path");
            arg
        })
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .value_name("OUTPUT_PATH")
                .help("Path to the output PDF file (defaults to ./output.pdf)"),
        )
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("CONFIG_FILE")
                .help("Path to configuration file (TOML format). Auto-detects markdown2pdfrc.toml if not specified"),
        )
        .arg(
            Arg::new("font-path")
                .long("font-path")
                .value_name("PATH")
                .help("Path to custom font directory or font file")
                .action(clap::ArgAction::Append),
        )
        .arg(
            Arg::new("default-font")
                .long("default-font")
                .value_name("FONT_NAME")
                .help("Default font family to use (default: helvetica)"),
        )
        .arg(
            Arg::new("code-font")
                .long("code-font")
                .value_name("FONT_NAME")
                .help("Font for code blocks (default: courier)"),
        )
        .arg(
            Arg::new("fallback-font")
                .long("fallback-font")
                .value_name("FONT_NAME")
                .help("Fallback font for missing characters (can be specified multiple times)")
                .action(clap::ArgAction::Append),
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .help("Show detailed output including validation warnings and file size")
                .action(clap::ArgAction::SetTrue)
                .conflicts_with("quiet"),
        )
        .arg(
            Arg::new("quiet")
                .short('q')
                .long("quiet")
                .help("Suppress all output except errors")
                .action(clap::ArgAction::SetTrue)
                .conflicts_with("verbose"),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .help("Validate input without generating PDF")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("list-embedded-fonts")
                .short('E')
                .long("list-embedded-fonts")
                .help("List embedded binary fonts and exit")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("get-default-configuration")
                .long("get-default-configuration")
                .help("Print a default markdown2pdfrc.toml to stdout and exit")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("show-missing-glyphs")
                .long("show-missing-glyphs")
                .help("List missing glyphs detected by font coverage checks before generating PDF")
                .action(clap::ArgAction::SetTrue),
        );

    let matches = cmd.clone().get_matches();

    // Handle listing of embedded fonts early so the option can be used without other inputs
    if matches.get_flag("list-embedded-fonts") {
        // Print canonical embedded family names
        for f in markdown2pdf::embedded_fonts::known_embedded_families() {
            println!("{}", f);
        }
        process::exit(0);
    }

    // Print a default configuration TOML and exit if requested
    if matches.get_flag("get-default-configuration") {
        println!("{}", markdown2pdf::config::default_config_toml());
        process::exit(0);
    }

    #[cfg(feature = "fetch")]
    let has_url = matches.contains_id("url");
    #[cfg(not(feature = "fetch"))]
    let has_url = false;

    if !matches.contains_id("path") && !matches.contains_id("string") && !has_url {
        cmd.print_help().unwrap();
        println!();
        process::exit(1);
    }

    if let Err(e) = run(matches) {
        match e {
            AppError::FileReadError(e) => error!("[X] Error reading file: {}", e),
            AppError::ConversionError(e) => error!("[X] Conversion error: {}", e),
            AppError::PathError(e) => error!("[X] Path error: {}", e),
            #[cfg(feature = "fetch")]
            AppError::NetworkError(e) => error!("[X] Network error: {}", e),
        }
        process::exit(1);
    }
}
