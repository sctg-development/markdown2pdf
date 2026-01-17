# extend_font Examples and Use Cases

This document provides practical examples of using the `extend_font` binary for various scenarios.

## Basic Examples

### Example 1: Simple Font Extension
Extend DejaVuSans with emoji glyphs from NotoEmoji.

```bash
cargo run --bin extend_font -- \
    --src-font fonts/DejaVuSans.ttf \
    --combine-font fonts/NotoEmoji-VariableFont_wght.ttf
```

**Output**:
```
[INFO] Source font 'Font' loaded with 5918 glyphs
[INFO] Combine font 'Font' loaded with 1503 glyphs
[INFO] No destination specified, will modify source font in place
[INFO] Found 1305 missing glyphs to copy
[WARN] Variable to fixed weight conversion needed
[INFO] Font extension completed successfully
```

### Example 2: Save to New File
Create a new extended font without modifying the original.

```bash
cargo run --bin extend_font -- \
    --src-font fonts/DejaVuSans.ttf \
    --combine-font fonts/NotoEmoji-VariableFont_wght.ttf \
    --dst-font output/DejaVuSans-WithEmoji.ttf
```

**Output**:
```
[INFO] Writing extended font to: "output/DejaVuSans-WithEmoji.ttf"
[INFO] Extended font written to: "output/DejaVuSans-WithEmoji.ttf"
[INFO] Font extension completed successfully
```

### Example 3: With Debug Output
See detailed information about what's being processed.

```bash
cargo run --bin extend_font -- -vv \
    --src-font fonts/DejaVuSans.ttf \
    --combine-font fonts/NotoEmoji-VariableFont_wght.ttf
```

**Output** (partial - shows debug logs):
```
[DEBUG] Parsed arguments: ExtendFontArgs { ... }
[DEBUG] Reading source font from: "fonts/DejaVuSans.ttf"
[DEBUG] Font family name: (unavailable)
[DEBUG] Font glyphs count: 5918
[INFO] Source font 'Font' loaded with 5918 glyphs
[DEBUG] Reading combine font from: "fonts/NotoEmoji-VariableFont_wght.ttf"
[DEBUG] Font is_variable_weight: true
[DEBUG] Font weight class: 400
[WARN] Variable to fixed weight conversion needed
[DEBUG] Font glyphs count: 1503
[INFO] Found 1305 missing glyphs to copy
```

## Advanced Examples

### Example 4: Using Environment Variable for Logging
Control logging via environment variable (overrides CLI flags).

```bash
RUST_LOG=debug cargo run --bin extend_font -- \
    --src-font fonts/DejaVuSans.ttf \
    --combine-font fonts/NotoEmoji-VariableFont_wght.ttf
```

This is useful for CI/CD pipelines and scripts.

### Example 5: Quiet Mode
Minimize output (only show errors and critical messages).

```bash
cargo run --bin extend_font -- -q \
    --src-font fonts/DejaVuSans.ttf \
    --combine-font fonts/NotoEmoji-VariableFont_wght.ttf
```

### Example 6: Creating Parent Directories
The binary automatically creates parent directories if needed.

```bash
cargo run --bin extend_font -- \
    --src-font fonts/DejaVuSans.ttf \
    --combine-font fonts/NotoEmoji-VariableFont_wght.ttf \
    --dst-font extended-fonts/output/2024/DejaVuSans-Extended.ttf
```

If `extended-fonts/output/2024/` doesn't exist, it will be created.

### Example 7: Explicit Verbosity Level
Set a specific logging level without counting flags.

```bash
cargo run --bin extend_font -- \
    --verbose-level trace \
    --src-font fonts/DejaVuSans.ttf \
    --combine-font fonts/NotoEmoji-VariableFont_wght.ttf
```

**Supported levels**: `trace`, `debug`, `info`, `warn`, `error`

### Example 8: Version Information
Check the binary version.

```bash
cargo run --bin extend_font -- --version
```

### Example 9: Help Documentation
View all available options.

```bash
cargo run --bin extend_font -- --help
```

## Batch Processing Examples

### Example 10: Script Multiple Operations
Process multiple font pairs in a loop.

```bash
#!/bin/bash

SOURCE_FONTS=("fonts/DejaVuSans.ttf" "fonts/DejaVuSansMono.ttf")
COMBINE_FONT="fonts/NotoEmoji-VariableFont_wght.ttf"
OUTPUT_DIR="extended-fonts"

mkdir -p "$OUTPUT_DIR"

for src_font in "${SOURCE_FONTS[@]}"; do
    filename=$(basename "$src_font" .ttf)
    dst_font="$OUTPUT_DIR/${filename}-WithEmoji.ttf"
    
    echo "Processing: $src_font → $dst_font"
    
    cargo run --bin extend_font -- \
        --src-font "$src_font" \
        --combine-font "$COMBINE_FONT" \
        --dst-font "$dst_font"
done
```

### Example 11: CI/CD Integration
Run with environment-based configuration.

```bash
#!/bin/bash

# Environment variables from CI system
SRC_FONT="${SRC_FONT:-fonts/DejaVuSans.ttf}"
COMBINE_FONT="${COMBINE_FONT:-fonts/NotoEmoji-VariableFont_wght.ttf}"
OUTPUT_FONT="${OUTPUT_FONT:-output/extended.ttf}"
LOG_LEVEL="${LOG_LEVEL:-info}"

RUST_LOG="$LOG_LEVEL" cargo run --bin extend_font -- \
    --src-font "$SRC_FONT" \
    --combine-font "$COMBINE_FONT" \
    --dst-font "$OUTPUT_FONT"

if [ $? -eq 0 ]; then
    echo "Font extension succeeded"
    ls -lh "$OUTPUT_FONT"
else
    echo "Font extension failed"
    exit 1
fi
```

## Comparison Examples

### Example 12: Identifying Font Coverage
Check how many glyphs are missing before and after.

```bash
#!/bin/bash

echo "=== Font Coverage Analysis ==="

echo ""
echo "Source font (DejaVuSans): "
cargo run --bin extend_font -- \
    --src-font fonts/DejaVuSans.ttf \
    --combine-font fonts/DejaVuSans.ttf \
    --verbose-level debug 2>&1 | grep "glyphs count"

echo ""
echo "Combine font (NotoEmoji):"
cargo run --bin extend_font -- \
    --src-font fonts/NotoEmoji-VariableFont_wght.ttf \
    --combine-font fonts/NotoEmoji-VariableFont_wght.ttf \
    --verbose-level debug 2>&1 | grep "glyphs count"

echo ""
echo "Missing glyphs (NotoEmoji → DejaVuSans):"
cargo run --bin extend_font -- \
    --src-font fonts/DejaVuSans.ttf \
    --combine-font fonts/NotoEmoji-VariableFont_wght.ttf \
    --verbose-level info 2>&1 | grep "missing glyphs"
```

## Testing and Development Examples

### Example 13: Running the Test Suite
Validate all functionality.

```bash
# Run all integration tests
cargo test --test extend_font_integration

# Run specific test
cargo test --test extend_font_integration test_missing_glyphs_detection

# Run with verbose output
cargo test --test extend_font_integration -- --nocapture

# Run tests sequentially (useful for debugging)
cargo test --test extend_font_integration -- --test-threads=1
```

### Example 14: Building a Release Binary
Create an optimized binary for distribution.

```bash
# Build release version
cargo build --bin extend_font --release

# Binary location
./target/release/extend_font --help

# File size comparison
ls -lh target/debug/extend_font target/release/extend_font
```

### Example 15: Code Quality Checks
Run formatting and linting checks.

```bash
# Check code style
cargo fmt --check

# Run clippy linter
cargo clippy --bin extend_font

# Fix issues automatically
cargo fmt
cargo clippy --fix
```

## Error Handling Examples

### Example 16: Handling Missing Files
What happens when a font file doesn't exist.

```bash
cargo run --bin extend_font -- \
    --src-font nonexistent.ttf \
    --combine-font fonts/NotoEmoji-VariableFont_wght.ttf
```

**Output**:
```
Error: Failed to read font file: No such file or directory
```

Exit code: 1 (failure)

### Example 17: Using Same Font
Edge case where source and combine fonts are identical.

```bash
cargo run --bin extend_font -- \
    --src-font fonts/DejaVuSans.ttf \
    --combine-font fonts/DejaVuSans.ttf
```

**Output**:
```
[INFO] Source font 'Font' loaded with 5918 glyphs
[INFO] Combine font 'Font' loaded with 5918 glyphs
[INFO] No missing glyphs found
[INFO] Font extension completed successfully
```

## Performance Examples

### Example 18: Measuring Execution Time
See how long processing takes.

```bash
time cargo run --bin extend_font -- \
    --src-font fonts/DejaVuSans.ttf \
    --combine-font fonts/NotoEmoji-VariableFont_wght.ttf \
    --quiet
```

**Output**:
```
real    0m0.450s
user    0m0.380s
sys     0m0.070s
```

### Example 19: Profile Memory Usage
Monitor memory consumption.

```bash
/usr/bin/time -v cargo run --bin extend_font -- \
    --src-font fonts/DejaVuSans.ttf \
    --combine-font fonts/NotoEmoji-VariableFont_wght.ttf \
    --quiet
```

## Troubleshooting Examples

### Example 20: Debugging with Full Output
Show everything that happens during processing.

```bash
cargo run --bin extend_font -- \
    --src-font fonts/DejaVuSans.ttf \
    --combine-font fonts/NotoEmoji-VariableFont_wght.ttf \
    --verbose-level trace 2>&1 | head -100
```

Use `head` to limit output if processing many glyphs.

### Example 21: Redirecting Output
Save logs to a file.

```bash
# Save all output
cargo run --bin extend_font -- \
    -vv \
    --src-font fonts/DejaVuSans.ttf \
    --combine-font fonts/NotoEmoji-VariableFont_wght.ttf \
    2>&1 | tee extend_font.log

# Just errors
cargo run --bin extend_font -- \
    --src-font fonts/DejaVuSans.ttf \
    --combine-font fonts/NotoEmoji-VariableFont_wght.ttf \
    2>&1 | grep -i error > errors.log
```

## Summary of Examples

| Example | Use Case | Key Features |
|---------|----------|--------------|
| 1 | Basic usage | In-place modification |
| 2 | Preserve original | --dst-font flag |
| 3 | Debugging | Verbose output (-vv) |
| 4 | Automation | Environment variables |
| 5 | Silent mode | Quiet flag (-q) |
| 6 | Directory creation | Automatic mkdir |
| 7 | Explicit control | --verbose-level |
| 8-9 | Help | --version, --help |
| 10-11 | Scripting | Batch processing |
| 12 | Analysis | Font coverage |
| 13-15 | Development | Testing and building |
| 16-19 | Error handling | Edge cases |
| 20-21 | Troubleshooting | Debugging techniques |

---

All examples use test fonts available in `fonts/`:
- `fonts/DejaVuSans.ttf` - Latin characters
- `fonts/NotoEmoji-VariableFont_wght.ttf` - Emoji characters
