# Kreuzberg for Ruby

[![RubyGems](https://img.shields.io/gem/v/kreuzberg)](https://rubygems.org/gems/kreuzberg)
[![Crates.io](https://img.shields.io/crates/v/kreuzberg)](https://crates.io/crates/kreuzberg)
[![PyPI](https://img.shields.io/pypi/v/kreuzberg)](https://pypi.org/project/kreuzberg/)
[![npm](https://img.shields.io/npm/v/@goldziher/kreuzberg)](https://www.npmjs.com/package/@goldziher/kreuzberg)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Documentation](https://img.shields.io/badge/docs-kreuzberg.dev-blue)](https://kreuzberg.dev)

High-performance document intelligence for Ruby, powered by Rust.

Extract text, tables, images, and metadata from 30+ file formats including PDF, DOCX, PPTX, XLSX, images, and more.

> **🚀 Version 4.0.0 Release Candidate**
> This is a pre-release version. We invite you to test the library and [report any issues](https://github.com/Goldziher/kreuzberg/issues) you encounter.

## Features

- **30+ File Formats**: PDF, DOCX, PPTX, XLSX, images, HTML, Markdown, XML, JSON, and more
- **OCR Support**: Built-in Tesseract OCR for scanned documents and images
- **High Performance**: Rust-powered extraction for native-level performance
- **Table Extraction**: Extract structured tables from documents
- **Language Detection**: Automatic language detection for extracted text
- **Text Chunking**: Split long documents into manageable chunks
- **Caching**: Built-in result caching for faster repeated extractions
- **Type-Safe**: Comprehensive typed configuration and result objects

## Requirements

- Ruby 3.2 or higher
- Rust toolchain (for building from source)

### Optional System Dependencies

- **Tesseract**: For OCR functionality
  - macOS: `brew install tesseract`
  - Ubuntu: `sudo apt-get install tesseract-ocr`
  - Windows: Download from [GitHub](https://github.com/tesseract-ocr/tesseract)

- **LibreOffice**: For legacy MS Office formats (.doc, .ppt)
  - macOS: `brew install libreoffice`
  - Ubuntu: `sudo apt-get install libreoffice`

- **Pandoc**: For advanced document conversion
  - macOS: `brew install pandoc`
  - Ubuntu: `sudo apt-get install pandoc`

## Installation

Add to your Gemfile:

```ruby
gem 'kreuzberg'
```

Then run:

```bash
bundle install
```

Or install directly:

```bash
gem install kreuzberg
```

## Quick Start

### Basic Extraction

```ruby
require 'kreuzberg'

# Extract from a file
result = Kreuzberg.extract_file_sync("document.pdf")
puts result.content
puts "MIME type: #{result.mime_type}"
```

### With Configuration

```ruby
# Create configuration
config = Kreuzberg::Config::Extraction.new(
  use_cache: true,
  force_ocr: false
)

result = Kreuzberg.extract_file_sync("document.pdf", config: config)
```

### With OCR

```ruby
# Configure OCR
ocr_config = Kreuzberg::Config::OCR.new(
  backend: "tesseract",
  language: "eng",
  preprocessing: true
)

config = Kreuzberg::Config::Extraction.new(ocr: ocr_config)
result = Kreuzberg.extract_file_sync("scanned.pdf", config: config)
```

### Extract from Bytes

```ruby
data = File.binread("document.pdf")
result = Kreuzberg.extract_bytes_sync(data, "application/pdf")
puts result.content
```

### Batch Processing

```ruby
paths = ["doc1.pdf", "doc2.docx", "doc3.xlsx"]
results = Kreuzberg.batch_extract_files_sync(paths)

results.each do |result|
  puts "Content: #{result.content[0..100]}"
  puts "MIME: #{result.mime_type}"
end
```

## Configuration

### Extraction Configuration

```ruby
config = Kreuzberg::Config::Extraction.new(
  use_cache: true,                      # Enable result caching
  enable_quality_processing: false,     # Enable text quality processing
  force_ocr: false                      # Force OCR even for digital PDFs
)
```

### OCR Configuration

```ruby
ocr = Kreuzberg::Config::OCR.new(
  backend: "tesseract",           # OCR backend (tesseract, easyocr, paddleocr)
  language: "eng",                # Language code (eng, deu, fra, etc.)
  preprocessing: true,            # Enable image preprocessing
  table_extraction: false,        # Extract tables via OCR
  confidence_threshold: 0.0       # Minimum confidence threshold
)

config = Kreuzberg::Config::Extraction.new(ocr: ocr)
```

### Chunking Configuration

```ruby
chunking = Kreuzberg::Config::Chunking.new(
  enabled: true,
  chunk_size: 1000,       # Characters per chunk
  chunk_overlap: 200,     # Overlap between chunks
  strategy: "sentence"    # Chunking strategy
)

config = Kreuzberg::Config::Extraction.new(chunking: chunking)
result = Kreuzberg.extract_file_sync("long_document.pdf", config: config)

result.chunks.each do |chunk|
  puts "Chunk: #{chunk.content}"
  puts "Tokens: #{chunk.token_count}"
end
```

### Language Detection

```ruby
lang_detection = Kreuzberg::Config::LanguageDetection.new(
  enabled: true,
  min_confidence: 0.8
)

config = Kreuzberg::Config::Extraction.new(language_detection: lang_detection)
result = Kreuzberg.extract_file_sync("multilingual.pdf", config: config)

result.detected_languages&.each do |lang|
  puts "Language: #{lang.lang}, Confidence: #{lang.confidence}"
end
```

### PDF Options

```ruby
pdf_options = Kreuzberg::Config::PDF.new(
  extract_images: true,
  image_min_size: 10000,    # Minimum image size in bytes
  password: "secret"        # PDF password
)

config = Kreuzberg::Config::Extraction.new(pdf_options: pdf_options)
```

## Working with Results

```ruby
result = Kreuzberg.extract_file_sync("invoice.pdf")

# Access extracted text
puts result.content

# Access MIME type
puts result.mime_type

# Access metadata
puts result.metadata.inspect

# Access extracted tables
result.tables.each do |table|
  puts "Headers: #{table.headers.join(', ')}"
  table.rows.each do |row|
    puts row.join(', ')
  end
end

# Convert to hash
hash = result.to_h

# Convert to JSON
json = result.to_json
```

## CLI Usage

Kreuzberg provides a Ruby wrapper for the CLI:

```ruby
# Extract content
output = Kreuzberg::CLI.extract("document.pdf", output: "text")

# Detect MIME type
mime_type = Kreuzberg::CLI.detect("document.pdf")

# Get version
version = Kreuzberg::CLI.version
```

## API Server

Start an API server (requires kreuzberg CLI):

```ruby
Kreuzberg::APIProxy.run(port: 8000) do |server|
  # Server runs in background
  # Make HTTP requests to http://localhost:8000
end
```

## MCP Server

Start a Model Context Protocol server for Claude Desktop:

```ruby
server = Kreuzberg::MCPProxy::Server.new(transport: 'stdio')
server.start

# Use with Claude Desktop integration
```

## Cache Management

```ruby
# Get cache statistics
stats = Kreuzberg.cache_stats
puts "Entries: #{stats[:total_entries]}"
puts "Size: #{stats[:total_size_bytes]} bytes"

# Clear cache
Kreuzberg.clear_cache
```

## Error Handling

```ruby
begin
  result = Kreuzberg.extract_file_sync("document.pdf")
rescue Kreuzberg::Errors::ParsingError => e
  puts "Parsing failed: #{e.message}"
  puts "Context: #{e.context}"
rescue Kreuzberg::Errors::OCRError => e
  puts "OCR failed: #{e.message}"
rescue Kreuzberg::Errors::MissingDependencyError => e
  puts "Missing dependency: #{e.dependency}"
rescue Kreuzberg::Errors::Error => e
  puts "Kreuzberg error: #{e.message}"
end
```

## Supported Formats

- **Documents**: PDF, DOCX, DOC, PPTX, PPT, ODT, ODP
- **Spreadsheets**: XLSX, XLS, ODS, CSV
- **Images**: PNG, JPEG, TIFF, BMP, GIF
- **Web**: HTML, MHTML, Markdown
- **Data**: JSON, YAML, TOML, XML
- **Email**: EML, MSG
- **Archives**: ZIP, TAR, 7Z
- **Text**: TXT, RTF, MD

## Performance

Kreuzberg's Rust core provides significant performance improvements:

- **PDF extraction**: 10-50x faster than pure Ruby solutions
- **Batch processing**: Parallel extraction with Tokio async runtime
- **Memory efficient**: Streaming parsers for large files
- **Caching**: Automatic result caching for repeated extractions

## Development

```bash
# Clone the repository
git clone https://github.com/Goldziher/kreuzberg.git
cd kreuzberg/packages/ruby

# Install dependencies
bundle install

# Build the Rust extension
bundle exec rake compile

# Run tests
bundle exec rspec

# Run RuboCop
bundle exec rubocop
```

## License

MIT License. See [LICENSE](../../LICENSE) for details.

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](../../CONTRIBUTING.md) for guidelines.

## Links

- **Documentation**: https://docs.kreuzberg.dev
- **GitHub**: https://github.com/Goldziher/kreuzberg
- **Issues**: https://github.com/Goldziher/kreuzberg/issues
