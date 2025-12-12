# CLI Usage

The Kreuzberg CLI provides command-line access to all extraction features. This guide covers installation, basic usage, and advanced features.

## Installation

=== "Homebrew (macOS/Linux)"

    --8<-- "snippets/cli/install_homebrew.md"

=== "Cargo (Cross-platform)"

    --8<-- "snippets/cli/install_cargo.md"

=== "Docker"

    --8<-- "snippets/cli/install_docker.md"

=== "Go (SDK)"

    --8<-- "snippets/cli/install_go_sdk.md"

## Basic Usage

### Extract from Single File

```bash title="Terminal"
# Extract text content to stdout
kreuzberg extract document.pdf

# Extract and save to output file
kreuzberg extract document.pdf -o output.txt

# Extract with document metadata included
kreuzberg extract document.pdf --metadata
```

### Extract from Multiple Files

```bash title="Terminal"
# Extract from multiple files at once
kreuzberg extract doc1.pdf doc2.docx doc3.pptx

# Extract using glob patterns
kreuzberg extract documents/**/*.pdf

# Extract all files in a directory
kreuzberg extract documents/
```

### Output Formats

```bash title="Terminal"
# Output as plain text (default format)
kreuzberg extract document.pdf

# Output as JSON
kreuzberg extract document.pdf --format json

# Output as JSON including document metadata
kreuzberg extract document.pdf --format json --metadata

# Output as formatted JSON with indentation
kreuzberg extract document.pdf --format json --pretty
```

## OCR Extraction

### Enable OCR

```bash title="Terminal"
# Enable OCR using Tesseract backend
kreuzberg extract scanned.pdf --ocr

# Extract with specific OCR language
kreuzberg extract scanned.pdf --ocr --language eng

# Extract with multiple OCR languages
kreuzberg extract scanned.pdf --ocr --language eng+deu+fra
```

### Force OCR

Force OCR even for PDFs with text layer:

```bash title="Terminal"
kreuzberg extract document.pdf --ocr --force-ocr
```

### OCR Configuration

```bash title="Terminal"
# Extract with custom Tesseract page segmentation mode
kreuzberg extract scanned.pdf --ocr --tesseract-config "--psm 6"

# Page segmentation modes (--psm):
# 0  = Orientation and script detection only
# 1  = Automatic page segmentation with OSD
# 3  = Fully automatic page segmentation (default)
# 6  = Single uniform block of text
# 11 = Sparse text detection
```

## Configuration Files

### Using Config Files

Kreuzberg automatically discovers configuration files:

```bash title="Terminal"
# Configuration file search order:
# 1. ./kreuzberg.toml
# 2. ./kreuzberg.yaml
# 3. ./kreuzberg.json
# 4. ./.kreuzberg/config.toml
# 5. ~/.config/kreuzberg/config.toml

# Extract using discovered configuration
kreuzberg extract document.pdf
```

### Specify Config File

```bash title="Terminal"
kreuzberg extract document.pdf --config my-config.toml
```

### Example Config Files

**kreuzberg.toml:**

```toml title="kreuzberg.toml"
# OCR configuration
[ocr]
backend = "tesseract"
language = "eng"
tesseract_config = "--psm 3"

# Enable quality processing for better output
enable_quality_processing = true

# Enable result caching
use_cache = true

# Text chunking configuration
[chunking]
max_chunk_size = 1000
overlap = 100

# Token reduction for LLM processing
[token_reduction]
enabled = true
target_reduction = 0.3

# Automatic language detection
[language_detection]
enabled = true
detect_multiple = true
```

**kreuzberg.yaml:**

```yaml title="kreuzberg.yaml"
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

```json title="kreuzberg.json"
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

```bash title="Terminal"
# Extract all PDFs in directory
kreuzberg extract documents/*.pdf -o output/

# Extract PDFs recursively from subdirectories
kreuzberg extract documents/**/*.pdf -o output/

# Extract multiple file types
kreuzberg extract documents/**/*.{pdf,docx,txt}
```

### Batch with JSON Output

```bash title="Terminal"
# Output all results as single JSON array
kreuzberg extract documents/*.pdf --format json --output results.json

# Output separate JSON file per input document
kreuzberg extract documents/*.pdf --format json --output-dir results/
```

### Parallel Processing

```bash title="Terminal"
# Enable parallel processing with automatic worker count
kreuzberg extract documents/*.pdf --parallel

# Process with specific number of worker threads
kreuzberg extract documents/*.pdf --parallel --workers 4
```

## Advanced Features

### Language Detection

```bash title="Terminal"
# Extract with automatic language detection
kreuzberg extract document.pdf --detect-language

# Output detected languages in JSON format
kreuzberg extract document.pdf --detect-language --format json
```

### Content Chunking

```bash title="Terminal"
# Split content into chunks for LLM processing
kreuzberg extract document.pdf --chunk --chunk-size 1000

# Split content with overlapping chunks
kreuzberg extract document.pdf --chunk --chunk-size 1000 --chunk-overlap 100

# Output chunked content as JSON
kreuzberg extract document.pdf --chunk --format json
```

### Token Reduction

```bash title="Terminal"
# Reduce token count by 30% while preserving meaning
kreuzberg extract document.pdf --reduce-tokens --reduction-target 0.3
```

### Quality Processing

```bash title="Terminal"
# Apply quality processing for improved formatting and cleanup
kreuzberg extract document.pdf --quality-processing
```

### Caching

```bash title="Terminal"
# Extract with result caching enabled (default)
kreuzberg extract scanned.pdf --ocr --cache

# Extract without caching results
kreuzberg extract scanned.pdf --ocr --no-cache

# Clear all cached results
kreuzberg cache clear
```

## Output Options

### Standard Output

```bash title="Terminal"
# Extract and print content to stdout
kreuzberg extract document.pdf

# Extract and redirect output to file
kreuzberg extract document.pdf > output.txt
```

### File Output

```bash title="Terminal"
# Extract and save to single output file
kreuzberg extract document.pdf -o output.txt

# Extract multiple files preserving directory structure
kreuzberg extract documents/*.pdf -o output_dir/
```

### JSON Output

```bash title="Terminal"
# Output as compact JSON
kreuzberg extract document.pdf --format json

# Output as formatted JSON with indentation
kreuzberg extract document.pdf --format json --pretty

# Output as JSON including document metadata
kreuzberg extract document.pdf --format json --metadata
```

**JSON Output Structure:**

```json title="JSON Response"
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

```bash title="Terminal"
# Extract tables from document
kreuzberg extract document.pdf --tables

# Extract tables and output as JSON
kreuzberg extract document.pdf --tables --format json

# Extract tables formatted as markdown
kreuzberg extract document.pdf --tables --table-format markdown
```

## Error Handling

### Verbose Output

```bash title="Terminal"
# Extract with detailed error messages
kreuzberg extract document.pdf --verbose

# Extract with debug-level logging
kreuzberg extract document.pdf --debug
```

### Continue on Errors

```bash title="Terminal"
# Process all files even if some fail
kreuzberg extract documents/*.pdf --continue-on-error

# Process all files and display error summary
kreuzberg extract documents/*.pdf --continue-on-error --show-errors
```

### Timeout

```bash title="Terminal"
# Set maximum extraction time per document (seconds)
kreuzberg extract document.pdf --timeout 30

# Process problematic files with timeout and error tolerance
kreuzberg extract problematic/*.pdf --timeout 10 --continue-on-error
```

## Examples

### Extract All PDFs in Directory

```bash title="Extract all PDFs from directory"
kreuzberg extract documents/*.pdf -o output/
```

### OCR All Scanned Documents

```bash title="OCR extraction from scanned documents"
kreuzberg extract scans/*.pdf --ocr --language eng -o ocr_output/
```

### Extract with Full Metadata

```bash title="Extract with complete metadata as pretty JSON"
kreuzberg extract document.pdf --format json --metadata --pretty
```

### Process Documents for LLM

```bash title="Prepare documents for LLM with chunking and token reduction"
kreuzberg extract documents/*.pdf \
  --chunk --chunk-size 1000 --chunk-overlap 100 \
  --reduce-tokens --reduction-target 0.3 \
  --format json -o llm_ready/
```

### Extract Tables from Spreadsheets

```bash title="Extract table data from spreadsheets"
kreuzberg extract data/*.xlsx --tables --format json --pretty
```

### Multilingual OCR

```bash title="OCR with multiple languages and detection"
kreuzberg extract international/*.pdf \
  --ocr --language eng+deu+fra+spa \
  --detect-language \
  --format json -o results/
```

### Batch Processing with Progress

```bash title="Parallel batch processing with error handling"
kreuzberg extract large_dataset/**/*.pdf \
  --parallel --workers 8 \
  --continue-on-error \
  --verbose \
  -o processed/
```

## Environment Variables

Set default configuration via environment variables:

```bash title="Terminal"
# Configure default OCR settings
export KREUZBERG_OCR_BACKEND=tesseract
export KREUZBERG_OCR_LANGUAGE=eng

# Configure cache location and behavior
export KREUZBERG_CACHE_DIR=~/.cache/kreuzberg
export KREUZBERG_CACHE_ENABLED=true

# Configure parallel processing
export KREUZBERG_WORKERS=4

# Extract using configured environment variables
kreuzberg extract document.pdf --ocr
```

## Shell Integration

### Bash Completion

```bash title="Terminal"
# Generate and save bash completion script
kreuzberg completion bash > ~/.local/share/bash-completion/completions/kreuzberg

# Enable completion in current session
eval "$(kreuzberg completion bash)"
```

### Zsh Completion

```bash title="Terminal"
# Enable zsh completion (add to .zshrc)
eval "$(kreuzberg completion zsh)"
```

### Fish Completion

```bash title="Terminal"
# Generate and save fish completion script
kreuzberg completion fish > ~/.config/fish/completions/kreuzberg.fish
```

## Docker Usage

### Basic Docker

```bash title="Terminal"
# Extract document using Docker with mounted directory
docker run -v $(pwd):/data goldziher/kreuzberg:latest \
  extract /data/document.pdf

# Extract and save output to host directory
docker run -v $(pwd):/data goldziher/kreuzberg:latest \
  extract /data/document.pdf -o /data/output.txt
```

### Docker with OCR

```bash title="Terminal"
# Extract with OCR using Docker
docker run -v $(pwd):/data goldziher/kreuzberg:latest \
  extract /data/scanned.pdf --ocr --language eng
```

### Docker Compose

**docker-compose.yaml:**

```yaml title="docker-compose.yaml"
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

```bash title="Terminal"
docker-compose up
```

## Performance Tips

### Optimize for Large Files

```bash title="Terminal"
# Extract without quality processing for faster speed
kreuzberg extract large.pdf --no-quality-processing

# Extract with extended timeout for large files
kreuzberg extract large.pdf --timeout 300

# Extract using parallel processing for multiple large files
kreuzberg extract large_files/*.pdf --parallel --workers 8
```

### Optimize for Small Files

```bash title="Terminal"
# Extract small files without parallel overhead
kreuzberg extract small_files/*.txt --no-parallel

# Extract without caching for quick one-off processing
kreuzberg extract small_files/*.txt --no-cache
```

### Memory Management

```bash title="Terminal"
# Extract large files sequentially to minimize memory usage
kreuzberg extract huge_files/*.pdf --workers 1

# Extract and compress output to save disk space
kreuzberg extract huge_file.pdf | gzip > output.txt.gz
```

## Troubleshooting

### Check Installation

```bash title="Terminal"
# Display installed version
kreuzberg --version

# Verify system dependencies
kreuzberg doctor
```

### Common Issues

**Issue: "Tesseract not found"**

```bash title="Terminal"
# Install Tesseract OCR engine on macOS
brew install tesseract

# Install Tesseract OCR engine on Ubuntu
sudo apt-get install tesseract-ocr
```


**Issue: "Out of memory"**

```bash title="Terminal"
# Reduce memory usage by processing sequentially
kreuzberg extract large_files/*.pdf --workers 1 --no-parallel
```

**Issue: "Extraction timeout"**

```bash title="Terminal"
# Extend timeout for slow documents
kreuzberg extract slow_file.pdf --timeout 300
```

## Server Commands

### Start API Server

The `serve` command starts a RESTful HTTP API server:

=== "Rust CLI"

    --8<-- "snippets/cli/serve_rust.md"

=== "Python"

    --8<-- "snippets/cli/serve_python.md"

=== "TypeScript"

    --8<-- "snippets/cli/serve_typescript.md"

=== "Go"

    --8<-- "snippets/cli/serve_go.md"

=== "Java"

    --8<-- "snippets/cli/serve_java.md"

=== "Ruby"

    ```ruby title="Ruby"
    require 'kreuzberg'

    # Start API server on port 8000
    Kreuzberg::APIProxy.run(port: 8000, host: '0.0.0.0') do |server|
      puts "API server running on http://localhost:8000"
      # Server runs while block executes
      # Make HTTP requests to endpoint
      sleep
    end
    ```

The server provides endpoints for:
- `/extract` - Extract text from uploaded files
- `/health` - Health check
- `/info` - Server information
- `/cache/stats` - Cache statistics
- `/cache/clear` - Clear cache

See [API Server Guide](../guides/api-server.md) for full API details.

### Start MCP Server

The `mcp` command starts a Model Context Protocol server for AI integration:

=== "Rust CLI"

    --8<-- "snippets/cli/mcp_rust.md"

=== "Python"

    --8<-- "snippets/cli/mcp_python.md"

=== "TypeScript"

    --8<-- "snippets/cli/mcp_typescript.md"

=== "Go"

    --8<-- "snippets/cli/mcp_go.md"

=== "Java"

    --8<-- "snippets/cli/mcp_java.md"

=== "Ruby"

    ```ruby title="Ruby"
    require 'kreuzberg'

    # Start MCP server for Claude Desktop
    server = Kreuzberg::MCPProxy::Server.new(transport: 'stdio')
    server.start
    # Server communicates via stdio for Claude integration
    ```

The MCP server provides tools for AI agents:
- `extract_file` - Extract text from a file path
- `extract_bytes` - Extract text from base64-encoded bytes
- `batch_extract` - Extract from multiple files

See [API Server Guide](../guides/api-server.md) for MCP integration details.

## Cache Management

### View Cache Statistics

```bash title="Terminal"
# Display cache usage statistics
kreuzberg cache stats

# Display statistics for specific cache directory
kreuzberg cache stats --cache-dir /path/to/cache

# Output cache statistics as JSON
kreuzberg cache stats --format json
```

### Clear Cache

```bash title="Terminal"
# Remove all cached extraction results
kreuzberg cache clear

# Clear specific cache directory
kreuzberg cache clear --cache-dir /path/to/cache

# Clear cache and display removal details
kreuzberg cache clear --format json
```

## Getting Help

### CLI Help

```bash title="Terminal"
# Display general CLI help
kreuzberg --help

# Display command-specific help
kreuzberg extract --help
kreuzberg serve --help
kreuzberg mcp --help
kreuzberg cache --help

# Display all available options
kreuzberg extract --help-all
```

### Version Information

```bash title="Terminal"
# Display version number
kreuzberg --version

# Display detailed version information as JSON
kreuzberg version --format json
```

## Next Steps

- [API Server Guide](../guides/api-server.md) - API and MCP server setup
- [Advanced Features](../guides/advanced.md) - Advanced Kreuzberg features
- [Plugin Development](../guides/plugins.md) - Extend Kreuzberg functionality
- [API Reference](../reference/api-python.md) - Programmatic access
