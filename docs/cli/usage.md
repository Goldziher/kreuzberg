# CLI Usage

The Kreuzberg CLI provides command-line access to all extraction features. This guide covers installation, basic usage, and advanced features.

## Installation

=== "Homebrew (macOS/Linux)"

    ```bash
    brew install kreuzberg
    ```

=== "Cargo (Cross-platform)"

    ```bash
    cargo install kreuzberg-cli
    ```

=== "Docker"

    ```bash
    docker pull goldziher/kreuzberg:latest
    docker run -v $(pwd):/data goldziher/kreuzberg:latest extract /data/document.pdf
    ```

## Basic Usage

### Extract from Single File

```bash
# Extract to stdout
kreuzberg extract document.pdf

# Save to file
kreuzberg extract document.pdf -o output.txt

# Extract with metadata
kreuzberg extract document.pdf --metadata
```

### Extract from Multiple Files

```bash
# Multiple files
kreuzberg extract doc1.pdf doc2.docx doc3.pptx

# Glob patterns
kreuzberg extract documents/**/*.pdf

# Directory (all files)
kreuzberg extract documents/
```

### Output Formats

```bash
# Plain text (default)
kreuzberg extract document.pdf

# JSON output
kreuzberg extract document.pdf --format json

# JSON with metadata
kreuzberg extract document.pdf --format json --metadata

# Pretty JSON
kreuzberg extract document.pdf --format json --pretty
```

## OCR Extraction

### Enable OCR

```bash
# Basic OCR with Tesseract
kreuzberg extract scanned.pdf --ocr

# Specify language
kreuzberg extract scanned.pdf --ocr --language eng

# Multiple languages
kreuzberg extract scanned.pdf --ocr --language eng+deu+fra
```

### Force OCR

Force OCR even for PDFs with text layer:

```bash
kreuzberg extract document.pdf --ocr --force-ocr
```

### OCR Configuration

```bash
# Custom Tesseract config
kreuzberg extract scanned.pdf --ocr --tesseract-config "--psm 6"

# Available page segmentation modes (--psm):
# 0 = Orientation and script detection (OSD) only
# 1 = Automatic page segmentation with OSD
# 3 = Fully automatic page segmentation (default)
# 6 = Assume a single uniform block of text
# 11 = Sparse text. Find as much text as possible
```

## Configuration Files

### Using Config Files

Kreuzberg automatically discovers configuration files:

```bash
# Searches in order:
# 1. ./kreuzberg.toml
# 2. ./kreuzberg.yaml
# 3. ./kreuzberg.json
# 4. ./.kreuzberg/config.toml
# 5. ~/.config/kreuzberg/config.toml

kreuzberg extract document.pdf  # Uses config if found
```

### Specify Config File

```bash
kreuzberg extract document.pdf --config my-config.toml
```

### Example Config Files

**kreuzberg.toml:**

```toml
# OCR settings
[ocr]
backend = "tesseract"
language = "eng"
tesseract_config = "--psm 3"

# Quality processing
enable_quality_processing = true

# Caching
use_cache = true

# Chunking
[chunking]
max_chunk_size = 1000
overlap = 100

# Token reduction
[token_reduction]
enabled = true
target_reduction = 0.3

# Language detection
[language_detection]
enabled = true
detect_multiple = true
```

**kreuzberg.yaml:**

```yaml
ocr:
  backend: tesseract
  language: eng
  tesseract_config: "--psm 3"

enable_quality_processing: true
use_cache: true

chunking:
  max_chunk_size: 1000
  overlap: 100

token_reduction:
  enabled: true
  target_reduction: 0.3

language_detection:
  enabled: true
  detect_multiple: true
```

**kreuzberg.json:**

```json
{
  "ocr": {
    "backend": "tesseract",
    "language": "eng",
    "tesseract_config": "--psm 3"
  },
  "enable_quality_processing": true,
  "use_cache": true,
  "chunking": {
    "max_chunk_size": 1000,
    "overlap": 100
  },
  "token_reduction": {
    "enabled": true,
    "target_reduction": 0.3
  },
  "language_detection": {
    "enabled": true,
    "detect_multiple": true
  }
}
```

## Batch Processing

### Process Multiple Files

```bash
# All PDFs in directory
kreuzberg extract documents/*.pdf -o output/

# Recursive search
kreuzberg extract documents/**/*.pdf -o output/

# Multiple file types
kreuzberg extract documents/**/*.{pdf,docx,txt}
```

### Batch with JSON Output

```bash
# Single JSON array with all results
kreuzberg extract documents/*.pdf --format json --output results.json

# One JSON file per input
kreuzberg extract documents/*.pdf --format json --output-dir results/
```

### Parallel Processing

```bash
# Process files in parallel (default: CPU count)
kreuzberg extract documents/*.pdf --parallel

# Specify worker count
kreuzberg extract documents/*.pdf --parallel --workers 4
```

## Advanced Features

### Language Detection

```bash
# Enable language detection
kreuzberg extract document.pdf --detect-language

# JSON output shows detected languages
kreuzberg extract document.pdf --detect-language --format json
```

### Content Chunking

```bash
# Enable chunking for LLM processing
kreuzberg extract document.pdf --chunk --chunk-size 1000

# With overlap
kreuzberg extract document.pdf --chunk --chunk-size 1000 --chunk-overlap 100

# JSON output includes chunks
kreuzberg extract document.pdf --chunk --format json
```

### Token Reduction

```bash
# Enable token reduction
kreuzberg extract document.pdf --reduce-tokens --reduction-target 0.3

# Reduces content by ~30% while preserving meaning
```

### Quality Processing

```bash
# Enable quality processing (formatting, cleanup)
kreuzberg extract document.pdf --quality-processing
```

### Caching

```bash
# Enable caching (default: enabled)
kreuzberg extract scanned.pdf --ocr --cache

# Disable caching
kreuzberg extract scanned.pdf --ocr --no-cache

# Clear cache
kreuzberg cache clear
```

## Output Options

### Standard Output

```bash
# Print to stdout
kreuzberg extract document.pdf

# Redirect to file
kreuzberg extract document.pdf > output.txt
```

### File Output

```bash
# Single file output
kreuzberg extract document.pdf -o output.txt

# Directory output (preserves structure)
kreuzberg extract documents/*.pdf -o output_dir/
```

### JSON Output

```bash
# Compact JSON
kreuzberg extract document.pdf --format json

# Pretty JSON
kreuzberg extract document.pdf --format json --pretty

# JSON with metadata
kreuzberg extract document.pdf --format json --metadata
```

**JSON Output Structure:**

```json
{
  "content": "Extracted text content...",
  "metadata": {
    "mime_type": "application/pdf",
    "page_count": 10,
    "author": "John Doe"
  },
  "tables": [
    {
      "cells": [["Name", "Age"], ["Alice", "30"]],
      "markdown": "| Name | Age |\n|------|-----|\n| Alice | 30 |"
    }
  ],
  "chunks": [],
  "detected_languages": ["eng"],
  "keywords": []
}
```

### Table Extraction

```bash
# Extract tables
kreuzberg extract document.pdf --tables

# JSON output includes table data
kreuzberg extract document.pdf --tables --format json

# Markdown tables in output
kreuzberg extract document.pdf --tables --table-format markdown
```

## Error Handling

### Verbose Output

```bash
# Show detailed error messages
kreuzberg extract document.pdf --verbose

# Show debug information
kreuzberg extract document.pdf --debug
```

### Continue on Errors

```bash
# Continue processing other files on error
kreuzberg extract documents/*.pdf --continue-on-error

# Show summary of errors
kreuzberg extract documents/*.pdf --continue-on-error --show-errors
```

### Timeout

```bash
# Set extraction timeout (seconds)
kreuzberg extract document.pdf --timeout 30

# Useful for hanging documents
kreuzberg extract problematic/*.pdf --timeout 10 --continue-on-error
```

## Examples

### Extract All PDFs in Directory

```bash
kreuzberg extract documents/*.pdf -o output/
```

### OCR All Scanned Documents

```bash
kreuzberg extract scans/*.pdf --ocr --language eng -o ocr_output/
```

### Extract with Full Metadata

```bash
kreuzberg extract document.pdf --format json --metadata --pretty
```

### Process Documents for LLM

```bash
kreuzberg extract documents/*.pdf \
  --chunk --chunk-size 1000 --chunk-overlap 100 \
  --reduce-tokens --reduction-target 0.3 \
  --format json -o llm_ready/
```

### Extract Tables from Spreadsheets

```bash
kreuzberg extract data/*.xlsx --tables --format json --pretty
```

### Multilingual OCR

```bash
kreuzberg extract international/*.pdf \
  --ocr --language eng+deu+fra+spa \
  --detect-language \
  --format json -o results/
```

### Batch Processing with Progress

```bash
kreuzberg extract large_dataset/**/*.pdf \
  --parallel --workers 8 \
  --continue-on-error \
  --verbose \
  -o processed/
```

## Environment Variables

Set default configuration via environment variables:

```bash
# OCR settings
export KREUZBERG_OCR_BACKEND=tesseract
export KREUZBERG_OCR_LANGUAGE=eng

# Cache settings
export KREUZBERG_CACHE_DIR=~/.cache/kreuzberg
export KREUZBERG_CACHE_ENABLED=true

# Parallel processing
export KREUZBERG_WORKERS=4

# Use in commands
kreuzberg extract document.pdf --ocr  # Uses env vars
```

## Shell Integration

### Bash Completion

```bash
# Generate completion script
kreuzberg completion bash > ~/.local/share/bash-completion/completions/kreuzberg

# Or add to .bashrc
eval "$(kreuzberg completion bash)"
```

### Zsh Completion

```bash
# Add to .zshrc
eval "$(kreuzberg completion zsh)"
```

### Fish Completion

```bash
kreuzberg completion fish > ~/.config/fish/completions/kreuzberg.fish
```

## Docker Usage

### Basic Docker

```bash
# Mount current directory
docker run -v $(pwd):/data goldziher/kreuzberg:latest \
  extract /data/document.pdf

# Save output to host
docker run -v $(pwd):/data goldziher/kreuzberg:latest \
  extract /data/document.pdf -o /data/output.txt
```

### Docker with OCR

```bash
# Use OCR-enabled image
docker run -v $(pwd):/data goldziher/kreuzberg:latest \
  extract /data/scanned.pdf --ocr --language eng
```

### Docker Compose

**docker-compose.yml:**

```yaml
version: '3.8'

services:
  kreuzberg:
    image: goldziher/kreuzberg:latest
    volumes:
      - ./documents:/input
      - ./output:/output
    command: extract /input --ocr -o /output
```

Run:

```bash
docker-compose up
```

## Performance Tips

### Optimize for Large Files

```bash
# Disable quality processing for speed
kreuzberg extract large.pdf --no-quality-processing

# Increase timeout
kreuzberg extract large.pdf --timeout 300

# Use parallel processing
kreuzberg extract large_files/*.pdf --parallel --workers 8
```

### Optimize for Small Files

```bash
# Single thread for small files (less overhead)
kreuzberg extract small_files/*.txt --no-parallel

# Disable caching
kreuzberg extract small_files/*.txt --no-cache
```

### Memory Management

```bash
# Process files one at a time (low memory)
kreuzberg extract huge_files/*.pdf --workers 1

# Stream large outputs
kreuzberg extract huge_file.pdf | gzip > output.txt.gz
```

## Troubleshooting

### Check Installation

```bash
# Verify CLI is installed
kreuzberg --version

# Check dependencies
kreuzberg doctor
```

### Common Issues

**Issue: "Tesseract not found"**

```bash
# Install Tesseract
brew install tesseract  # macOS
sudo apt-get install tesseract-ocr  # Ubuntu
```

**Issue: "Pandoc not found"**

```bash
# Install Pandoc
brew install pandoc  # macOS
sudo apt-get install pandoc  # Ubuntu
```

**Issue: "Out of memory"**

```bash
# Process one file at a time
kreuzberg extract large_files/*.pdf --workers 1 --no-parallel
```

**Issue: "Extraction timeout"**

```bash
# Increase timeout
kreuzberg extract slow_file.pdf --timeout 300
```

## Getting Help

### CLI Help

```bash
# General help
kreuzberg --help

# Command help
kreuzberg extract --help

# List all options
kreuzberg extract --help-all
```

### Version Information

```bash
# Show version
kreuzberg --version

# Show detailed version info
kreuzberg version --verbose
```

## Next Steps

- [Core Concepts](../concepts/architecture.md) - Understand Kreuzberg architecture
- [Plugin Development](../plugins/overview.md) - Extend Kreuzberg functionality
- [API Reference](../api-reference/python/) - Programmatic access
