# markdown2pdf

<p align="center">

[![Repository](https://img.shields.io/badge/repo-sctg--development%2Fmarkdown2pdf-green)](https://github.com/sctg-development/markdown2pdf)
[![License](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

</p>

A Rust toolkit to convert Markdown into professional PDFs. Key features:

- LaTeX math (inline and display) via `genpdfi_extended::elements::Latex` 🎓
- Mermaid diagrams via `genpdfi_extended::elements::Mermaid` 🧩 (uses headless Chrome; on first run Chrome may be downloaded and rendering can be slow)
- Syntax-highlighted code blocks 🔧
- Scalable SVG images and sizing controls 🖼️
- Clickable images/badges (e.g. `[![alt](img)](url)`) 🔗
- Embedded fonts and Unicode fallback for wide language coverage 🔤
- Full configuration through `markdown2pdfrc.toml` ⚙️

---

## Highlights

- Renders display math (`$$...$$`) and inline math (`$...$`). When the Cargo feature `latex` is **not** enabled, LaTeX blocks display `need LaTeX feature`.
- Full SVG support with `[image.svg]` options: `width` (percentage) and `scale_factor`.
- Images with links and grouping of consecutive images for badge layouts.
- Font embedding and subsetting to minimize PDF size while keeping correct glyph coverage.
- Both a CLI and a library API for programmatic use.

---

## Quick Install

From the repository:

```bash
git clone https://github.com/sctg-development/markdown2pdf
cd markdown2pdf
cargo build --release
```

Install the binary from the repo:

```bash
cargo install --git https://github.com/sctg-development/markdown2pdf
```

> Note: LaTeX support is available as a Cargo feature (`latex`). This fork includes it in the default feature set, but you can enable/disable it explicitly:
>
> ```bash
> # Run with latex feature enabled
> cargo run --features latex --bin markdown2pdf -- -p tests/latex_examples.md -o /tmp/out.pdf
>
> # Build without default features
> cargo build --no-default-features
> ```

---

## CLI Usage

Convert a file:

```bash
markdown2pdf -p "docs/resume.md" -o "resume.pdf"
```

Convert a string:

```bash
markdown2pdf -s "**bold** and *italic*" -o output.pdf
```

Useful flags:
- `-p` source path
- `-o` output path (default `output.pdf`)
- `--verbose`, `--quiet`, `--dry-run`
- `--list-embedded-fonts` to list bundled font families

---

## `[latex]` configuration (markdown2pdfrc)

Default recommended LaTeX configuration:

```toml
[latex]
size = 8
textcolor = { r = 0, g = 0, b = 0 }
beforespacing = 0.0
afterspacing = 0.0
alignment = "center"
backgroundcolor = { r = 255, g = 255, b = 255 }
```

- `size`: font size in points used for LaTeX rendering
- `beforespacing` / `afterspacing`: vertical space around math blocks
- `alignment`: `left` | `center` | `right` (applies to block math)

---

## Library API (short example)

```rust
use markdown2pdf::{parse_into_file, config::ConfigSource};

let md = "# Title\n\nSome text and a formula: $E=mc^2$".to_string();
parse_into_file(md, "out.pdf", ConfigSource::Default, None)?;
```

Use `ConfigSource::File("path")` or `ConfigSource::Embedded("...toml...")` to customize styling.

---

## Tests & Examples

Run unit and integration tests:

```bash
cargo test
```

Verify LaTeX rendering (with feature):

```bash
cargo run --features latex --bin markdown2pdf -- -p tests/latex_examples.md -o /tmp/latex.pdf
```

---

## Contributing

Contributions welcome: issues, PRs, tests, and examples. Please:

- Fork → branch → PR
- Add tests for new features and follow commit conventions

---

## License

MIT — see `LICENSE` for details.

---

If you want, I can add a dedicated "Examples" section with small Markdown snippets (LaTeX, SVG, badges) and non-ignored doctests. Let me know whether you prefer a user-focused tutorial or a developer-focused reference.

> **Fork notice:** This repository is a fork maintained at `https://github.com/sctg-development/markdown2pdf`.
> 
> **Important:** The fork is **not published on crates.io**. To install this fork use:
>
> ```bash
> cargo install --git https://github.com/sctg-development/markdown2pdf
> ```
>
> Or build from source with:
>
> ```bash
> cargo build --release
> ```
>
markdown2pdf convertit des fichiers Markdown en PDFs professionnels.

Il fournit :
- une CLI simple pour convertir fichiers, URLs ou chaînes de texte en PDF,
- une API Rust pour intégrer la génération de PDF (rendu en mémoire ou sauvegarde sur disque),
- un système de configuration via `markdown2pdfrc.toml` pour contrôler style, polices et rendu des images/LaTeX.

Both binary and library are provided. The binary offers CLI conversion from files, URLs, or strings. The library enables programmatic PDF generation with full control over styling and fonts. Configuration can be loaded at runtime or embedded at compile time for containerized deployments.

Built in Rust for performance and memory safety. Handles standard Markdown syntax including headings, lists, code blocks, links, and images. Supports multiple input sources and outputs to files or bytes for in-memory processing.

## Install binary

### Homebrew

```sh
brew install theiskaa/tap/markdown2pdf
```

### Cargo

Install the binary globally using cargo:

```bash
cargo install markdown2pdf
```

For the latest git version:

```bash
cargo install --git https://github.com/sctg-development/markdown2pdf
```

### Prebuilt binaries

Prebuilt versions are available in our [GitHub releases](https://github.com/sctg-development/markdown2pdf/releases/latest):

|  File  | Platform | Checksum |
|--------|----------|----------|
| [markdown2pdf-aarch64-apple-darwin.tar.xz](https://github.com/theiskaa/markdown2pdf/releases/latest/download/markdown2pdf-aarch64-apple-darwin.tar.xz) | Apple Silicon macOS | [checksum](https://github.com/theiskaa/markdown2pdf/releases/latest/download/markdown2pdf-aarch64-apple-darwin.tar.xz.sha256) |
| [markdown2pdf-x86_64-apple-darwin.tar.xz](https://github.com/theiskaa/markdown2pdf/releases/latest/download/markdown2pdf-x86_64-apple-darwin.tar.xz) | Intel macOS | [checksum](https://github.com/theiskaa/markdown2pdf/releases/latest/download/markdown2pdf-x86_64-apple-darwin.tar.xz.sha256) |
| [markdown2pdf-x86_64-pc-windows-msvc.zip](https://github.com/theiskaa/markdown2pdf/releases/latest/download/markdown2pdf-x86_64-pc-windows-msvc.zip) | x64 Windows | [checksum](https://github.com/theiskaa/markdown2pdf/releases/latest/download/markdown2pdf-x86_64-pc-windows-msvc.zip.sha256) |
| [markdown2pdf-aarch64-unknown-linux-gnu.tar.xz](https://github.com/theiskaa/markdown2pdf/releases/latest/download/markdown2pdf-aarch64-unknown-linux-gnu.tar.xz) | ARM64 Linux | [checksum](https://github.com/theiskaa/markdown2pdf/releases/latest/download/markdown2pdf-aarch64-unknown-linux-gnu.tar.xz.sha256) |
| [markdown2pdf-x86_64-unknown-linux-gnu.tar.xz](https://github.com/theiskaa/markdown2pdf/releases/latest/download/markdown2pdf-x86_64-unknown-linux-gnu.tar.xz) | x64 Linux | [checksum](https://github.com/theiskaa/markdown2pdf/releases/latest/download/markdown2pdf-x86_64-unknown-linux-gnu.tar.xz.sha256) |

## Install as library

Add to your project:

```bash
cargo add markdown2pdf
```

Or add to your Cargo.toml:

```toml
markdown2pdf = "0.1.9"
```

### Feature Flags

The library provides optional feature flags to control dependencies:

- **Default (no features)**: Core PDF generation from files and strings. No network dependencies.
- **`fetch`**: Enables URL fetching support (requires one of the TLS features below).
- **`native-tls`**: Enables URL fetching with native TLS/OpenSSL (recommended for most users).
- **`rustls-tls`**: Enables URL fetching with pure-Rust TLS implementation (useful for static linking or avoiding OpenSSL).

```toml
# Minimal installation (no network dependencies)
markdown2pdf = "0.1.9"

# With URL fetching support (native TLS)
markdown2pdf = { version = "0.1.9", features = ["native-tls"] }

# With URL fetching support (rustls)
markdown2pdf = { version = "0.1.9", features = ["rustls-tls"] }
```

**Note**: Binary installations via cargo or prebuilt downloads do not include URL fetching by default. To build the binary with URL support:

```bash
# Install with URL fetching support
cargo install markdown2pdf --features native-tls

# Or build from source
cargo build --release --features native-tls
```

## Usage

The tool accepts file paths (`-p`), string content (`-s`), or URLs (`-u`) as input. Output path is specified with `-o`. Input precedence: path > url > string. Defaults to 'output.pdf'.

Convert a Markdown file:
```bash
markdown2pdf -p "docs/resume.md" -o "resume.pdf"
```

Convert string content:
```bash
markdown2pdf -s "**bold text** *italic text*." -o "output.pdf"
```

Convert from URL (requires `native-tls` or `rustls-tls` feature):
```bash
markdown2pdf -u "https://raw.githubusercontent.com/user/repo/main/README.md" -o "readme.pdf"
```

Use `--verbose` for detailed font selection output, `--quiet` for CI/CD pipelines, or `--dry-run` to validate syntax without generating PDF.

## Font Handling and Unicode Support

The library automatically detects Unicode characters and selects system fonts with good coverage. Font subsetting reduces PDF size by 98% by including only used glyphs. A document with DejaVu Sans embeds ~31 KB instead of 1.2 MB.

Fallback font chains specify multiple fonts tried in sequence for missing characters. Useful for mixed-script documents with Latin, Cyrillic, Greek, or CJK. The system analyzes each character and selects the best font from the chain.

When non-ASCII characters are detected, the library prioritizes DejaVu Sans, Noto Sans, and Liberation Sans. Coverage percentages are reported with suggestions if fonts lack support.

Note on PDF standard fonts: standard PDF base families (e.g., "Helvetica", "Times", "Courier") are mapped to embedded binary families when available:
- "Helvetica"/"Arial" → **DejaVu Sans**
- "Times"/"Times New Roman"/"Serif" → **DejaVu Serif**
- "Courier"/"Courier New"/"Monospace" → **CMU Typewriter Text**

Use `--list-embedded-fonts` (or `-E`) to print a list of embedded families bundled with the binary.

The system loads actual Bold, Italic, and Bold-Italic font files rather than synthetic rendering. Font name resolution includes fuzzy matching and aliasing for cross-platform compatibility. "Arial" automatically maps to Helvetica on macOS.

Custom fonts load from directories via `--font-path` with recursive search for TrueType and OpenType fonts.

```bash
# Unicode with fallback chain
markdown2pdf -p international.md --default-font "DejaVu Sans" \
  --fallback-font "Arial Unicode MS" -o output.pdf

# Custom fonts with subsetting
markdown2pdf -p document.md --font-path "./fonts" \
  --default-font "Roboto" --code-font "Fira Code" -o output.pdf
```

## SVG Image Configuration

SVG images embedded in Markdown can be sized using the `[image.svg]` configuration section. Control SVG rendering dimensions based on either percentage of page width or multiplier of original SVG dimensions.

### Configuration Options

**`width`**: Specifies SVG width as percentage of page width
- Format: `"50%"` (percentage of available page width)
- Default: Auto (uses original SVG dimensions)
- When specified, completely overrides `scale_factor`
- Examples: `"30%"`, `"50%"`, `"100%"`

**`scale_factor`**: Proportional multiplier for SVG sizing based on original dimensions
- Default: `1.0` (original SVG size from width/height attributes)
- Scales the intrinsic SVG dimensions by this factor
- Only used when `width` is not specified
- Examples: `0.5` = 50% of original, `2.0` = 200% of original

**Priority**: `width` parameter always takes precedence over `scale_factor`

### Example Configuration

```toml
[image.svg]
# Make all SVGs 50% of page width
width = "50%"
```

Or scale based on original SVG size:

```toml
[image.svg]
# Make all SVGs twice their original size
scale_factor = 2.0
```

When both are specified, `width` wins:

```toml
[image.svg]
width = "40%"
scale_factor = 2.0  # This is ignored because width is specified
```

### How It Works

- **`width = "50%"`**: SVG renders at 50% of the available page width, regardless of its original size
- **`scale_factor = 0.5`**: SVG renders at 50% of its original dimensions (from SVG attributes)
- **`scale_factor = 2.0`**: SVG renders at 200% of its original dimensions
- **`width = "50%" + scale_factor = 2.0`**: Uses the 50% width, `scale_factor` is ignored


## Library Usage

Two main functions: `parse_into_file()` saves PDF to disk, `parse_into_bytes()` returns bytes for web services. Both parse Markdown, apply styling, and render output.

Configuration uses `ConfigSource`: `Default` for built-in styling, `File("path")` for runtime loading, or `Embedded(content)` for compile-time embedding.

```rust
use markdown2pdf::{parse_into_file, config::ConfigSource};

// Default styling
parse_into_file(markdown, "output.pdf", ConfigSource::Default, None)?;

// File-based configuration
parse_into_file(markdown, "output.pdf", ConfigSource::File("config.toml"), None)?;

// Embedded configuration
const CONFIG: &str = include_str!("../config.toml");
parse_into_file(markdown, "output.pdf", ConfigSource::Embedded(CONFIG), None)?;
```

Embedded configuration uses `include_str!()` at compile time, eliminating runtime file dependencies.

Font configuration uses `FontConfig` for programmatic control over fonts, fallback chains, and subsetting.

```rust
use markdown2pdf::{parse_into_file, config::ConfigSource, fonts::FontConfig};
use std::path::PathBuf;

// Configure fonts for international document
let font_config = FontConfig {
    custom_paths: vec![PathBuf::from("./fonts")],
    default_font: Some("Noto Sans".to_string()),
    code_font: Some("Fira Code".to_string()),
    fallback_fonts: vec![
        "Arial Unicode MS".to_string(),
        "DejaVu Sans".to_string(),
    ],
    enable_subsetting: true,
};

parse_into_file(
    markdown,
    "output.pdf",
    ConfigSource::Default,
    Some(&font_config),
)?;
```

Font subsetting is enabled by default, analyzing text to create minimal subsets while maintaining full fidelity.

For advanced usage, work directly with the lexer and PDF components via `load_config_from_source()`.

## Configuration

TOML configuration customizes fonts, colors, spacing, and visual properties. Configuration translates to a `StyleMatch` instance. Three loading methods: default styles, runtime files, or compile-time embedding.

Embedded configuration creates self-contained binaries for Docker and containers with compile-time validation. Error handling falls back to default styling if files are missing or invalid.

For binary usage, create a config file at `~/markdown2pdfrc.toml` and copy the example configuration from `markdown2pdfrc.example.toml`. For library usage with embedded config, create your configuration file and embed it using `include_str!()` or define it as a string literal, then use it with `ConfigSource::Embedded(content)`.

## Contributing
For information regarding contributions, please refer to [CONTRIBUTING.md](CONTRIBUTING.md) file.

## Donations
For information regarding donations please refer to [DONATE.md](DONATE.md)
