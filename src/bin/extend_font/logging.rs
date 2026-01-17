/// Logging initialization module
///
/// Handles initialization of env_logger based on CLI arguments and environment variables.
/// The RUST_LOG environment variable takes precedence over CLI flags.

use crate::args::ExtendFontArgs;
use env_logger::Builder;
use log::LevelFilter;


/// Initialize logging based on CLI arguments and environment variables
///
/// # Arguments
///
/// * `args` - Command-line arguments containing verbosity information
///
/// # Returns
///
/// * `Result<(), Box<dyn std::error::Error>>` - Success or error result
///
/// # Log Level Selection Priority
///
/// 1. RUST_LOG environment variable (highest priority)
/// 2. --verbose-level flag
/// 3. Count of -v flags (-v = debug, -vv/-vvv = trace)
/// 4. -q flag (quiet, only show errors)
/// 5. Default (info level)
pub fn init_logging(args: &ExtendFontArgs) -> Result<(), Box<dyn std::error::Error>> {
    let mut builder = Builder::new();

    // Determine the log level
    let level_str = args.effective_log_level();
    let level_filter = match level_str.to_lowercase().as_str() {
        "trace" => LevelFilter::Trace,
        "debug" => LevelFilter::Debug,
        "info" => LevelFilter::Info,
        "warn" => LevelFilter::Warn,
        "error" => LevelFilter::Error,
        "off" => LevelFilter::Off,
        _ => {
            // Try to parse RUST_LOG format with module names
            builder.parse_filters(&level_str);
            LevelFilter::Info
        }
    };

    // Set the log level filter
    if level_filter != LevelFilter::Off {
        builder.filter_level(level_filter);
    }

    // Set a simple format: level + message
    builder.format(|buf, record| {
        use std::io::Write;
        writeln!(buf, "[{}] {}", record.level(), record.args())
    });

    builder.try_init()
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    Ok(())
}
