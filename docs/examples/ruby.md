# Ruby Examples

This page provides comprehensive examples of using Kreuzberg with Ruby. The Ruby gem provides native bindings to the high-performance Rust core library via Magnus FFI.

## Installation

```bash
gem install kreuzberg
```

## Basic Extraction

### Simple Extraction

```ruby
require 'kreuzberg'

# Extract from a file
result = Kreuzberg.extract_file_sync('document.pdf')

puts result.content
puts result.mime_type
```

### Extraction with Configuration

```ruby
require 'kreuzberg'

config = Kreuzberg::Config::Extraction.new(
  use_cache: true,
  enable_quality_processing: true
)

result = Kreuzberg.extract_file_sync('document.pdf', config: config)
puts result.content
```

### Async Extraction (Tokio Runtime)

Ruby doesn't have native async/await like Python or TypeScript. The async methods use a blocking Tokio runtime internally:

```ruby
require 'kreuzberg'

# This uses Tokio runtime internally but blocks in Ruby
result = Kreuzberg.extract_file('document.pdf')
puts result.content
```

For true background processing in Ruby, use threads:

```ruby
require 'kreuzberg'

thread = Thread.new do
  result = Kreuzberg.extract_file_sync('document.pdf')
  puts result.content
end

thread.join
```

### Extract from Bytes

```ruby
require 'kreuzberg'

data = File.binread('document.pdf')

result = Kreuzberg.extract_bytes_sync(
  data,
  'application/pdf'
)
puts result.content
```

### Accessing Metadata

```ruby
require 'kreuzberg'

result = Kreuzberg.extract_file_sync('document.pdf')

# Metadata is returned as a hash
if result.metadata['pdf']
  pdf_meta = result.metadata['pdf']
  puts "Pages: #{pdf_meta['page_count']}"
  puts "Author: #{pdf_meta['author']}"
  puts "Title: #{pdf_meta['title']}"
  puts "Created: #{pdf_meta['created']}"
end

# Access format-specific metadata
if result.metadata['format_type']
  puts "Format: #{result.metadata['format_type']}"
end
```

### Batch Processing

```ruby
require 'kreuzberg'

files = ['doc1.pdf', 'doc2.docx', 'doc3.pptx']
results = Kreuzberg.batch_extract_files_sync(files)

results.each_with_index do |result, i|
  puts "File #{i + 1}:"
  puts "  Content length: #{result.content.length}"
  puts "  MIME type: #{result.mime_type}"
end
```

### Parallel Batch Processing with Threads

For better performance with multiple files, use Ruby threads:

```ruby
require 'kreuzberg'

files = ['doc1.pdf', 'doc2.docx', 'doc3.pptx']

threads = files.map do |file|
  Thread.new do
    Kreuzberg.extract_file_sync(file)
  end
end

results = threads.map(&:value)

results.each do |result|
  puts "Content length: #{result.content.length}"
end
```

### Error Handling

```ruby
require 'kreuzberg'

files = ['doc1.pdf', '/nonexistent.pdf', 'doc3.pptx']

files.each do |file|
  begin
    result = Kreuzberg.extract_file_sync(file)
    puts "✓ #{file}: #{result.content.length} characters"
  rescue StandardError => e
    puts "✗ #{file}: #{e.message}"
  end
end
```

## OCR Extraction

### Basic OCR

```ruby
require 'kreuzberg'

config = Kreuzberg::Config::Extraction.new(
  ocr: Kreuzberg::Config::OCR.new(
    backend: 'tesseract',
    language: 'eng'
  )
)

result = Kreuzberg.extract_file_sync('scanned.pdf', config: config)
puts result.content
```

### OCR with Multiple Languages

```ruby
require 'kreuzberg'

config = Kreuzberg::Config::Extraction.new(
  ocr: Kreuzberg::Config::OCR.new(
    backend: 'tesseract',
    language: 'eng+deu+fra'  # English + German + French
  )
)

result = Kreuzberg.extract_file_sync('multilingual.pdf', config: config)
puts result.content
```

### Force OCR on Text PDFs

Sometimes you want to extract images and run OCR even if the PDF already has text:

```ruby
require 'kreuzberg'

config = Kreuzberg::Config::Extraction.new(
  ocr: Kreuzberg::Config::OCR.new(
    backend: 'tesseract',
    language: 'eng'
  ),
  force_ocr: true  # Force OCR even if text exists
)

result = Kreuzberg.extract_file_sync('document.pdf', config: config)
puts result.content
```

### OCR from Images

```ruby
require 'kreuzberg'

config = Kreuzberg::Config::Extraction.new(
  ocr: Kreuzberg::Config::OCR.new(
    backend: 'tesseract',
    language: 'eng'
  )
)

# Works with various image formats
['image.png', 'scan.jpg', 'photo.tiff'].each do |image_file|
  result = Kreuzberg.extract_file_sync(image_file, config: config)
  puts "#{image_file}: #{result.content[0..50]}..."
end
```

### OCR with Tesseract Configuration

```ruby
require 'kreuzberg'

tesseract_config = Kreuzberg::Config::Tesseract.new(
  psm: 6,  # Page segmentation mode (6 = uniform block of text)
  oem: 3   # OCR Engine Mode (3 = default, based on what is available)
)

config = Kreuzberg::Config::Extraction.new(
  ocr: Kreuzberg::Config::OCR.new(
    backend: 'tesseract',
    language: 'eng',
    tesseract_config: tesseract_config
  )
)

result = Kreuzberg.extract_file_sync('scanned.pdf', config: config)
puts result.content
```

## Configuration Options

### ExtractionConfig

The `Kreuzberg::Config::Extraction` class controls extraction behavior:

```ruby
require 'kreuzberg'

config = Kreuzberg::Config::Extraction.new(
  # Quality processing
  enable_quality_processing: true,

  # Caching
  use_cache: true,

  # OCR configuration
  ocr: Kreuzberg::Config::OCR.new(
    backend: 'tesseract',
    language: 'eng'
  ),

  # Force OCR even for text-based PDFs
  force_ocr: false,

  # Chunking for large documents
  chunking: Kreuzberg::Config::Chunking.new(
    max_chars: 1000,
    max_overlap: 100
  ),

  # Language detection
  language_detection: Kreuzberg::Config::LanguageDetection.new,

  # PDF options
  pdf_options: Kreuzberg::Config::PDF.new(
    extract_images: true,
    passwords: ['password1', 'password2']  # Try multiple passwords
  )
)

result = Kreuzberg.extract_file_sync('document.pdf', config: config)
```

### OcrConfig

Configure OCR behavior:

```ruby
require 'kreuzberg'

ocr_config = Kreuzberg::Config::OCR.new(
  backend: 'tesseract',  # OCR backend to use
  language: 'eng',       # Language code
  tesseract_config: Kreuzberg::Config::Tesseract.new(
    psm: 6,  # Page segmentation mode
    oem: 3   # OCR Engine Mode
  )
)

config = Kreuzberg::Config::Extraction.new(ocr: ocr_config)
```

### ChunkingConfig

Configure content chunking for large documents:

```ruby
require 'kreuzberg'

chunking_config = Kreuzberg::Config::Chunking.new(
  max_chars: 1000,   # Maximum characters per chunk
  max_overlap: 100   # Overlap between chunks (must be < max_chars)
)

config = Kreuzberg::Config::Extraction.new(chunking: chunking_config)
result = Kreuzberg.extract_file_sync('large_document.pdf', config: config)

# Access chunks
if result.chunks
  puts "Total chunks: #{result.chunks.length}"
  result.chunks.each_with_index do |chunk, i|
    puts "Chunk #{i + 1}: #{chunk[0..50]}..."
  end
end
```

### PdfConfig

Configure PDF-specific extraction options:

```ruby
require 'kreuzberg'

pdf_config = Kreuzberg::Config::PDF.new(
  extract_images: true,              # Extract embedded images
  passwords: ['pass1', 'pass2'],     # Try multiple passwords (if needed)
  extract_metadata: true             # Extract PDF metadata
)

config = Kreuzberg::Config::Extraction.new(pdf_options: pdf_config)
result = Kreuzberg.extract_file_sync('document.pdf', config: config)
```

### LanguageDetectionConfig

Configure language detection:

```ruby
require 'kreuzberg'

lang_detect_config = Kreuzberg::Config::LanguageDetection.new

config = Kreuzberg::Config::Extraction.new(language_detection: lang_detect_config)
result = Kreuzberg.extract_file_sync('document.pdf', config: config)

# Access detected languages
if result.detected_languages
  puts "Detected languages: #{result.detected_languages.join(', ')}"
end
```

## Working with Results

### Result Class

The `Kreuzberg::Result` class contains all extraction information:

```ruby
require 'kreuzberg'

result = Kreuzberg.extract_file_sync('document.pdf')

# Extracted text content
puts "Content: #{result.content[0..100]}..."

# MIME type
puts "MIME type: #{result.mime_type}"

# Metadata (hash with format-specific fields)
puts "Metadata: #{result.metadata}"

# Extracted tables (array of hashes)
puts "Tables: #{result.tables.length}"

# Detected languages (array of strings, or nil)
puts "Languages: #{result.detected_languages}"

# Chunks (array of strings, or nil)
puts "Chunks: #{result.chunks&.length || 0}"

# Raw metadata JSON string
puts "Metadata JSON: #{result.metadata_json}"
```

### Accessing Metadata by Format

Metadata structure varies by document format:

```ruby
require 'kreuzberg'

# PDF metadata
pdf_result = Kreuzberg.extract_file_sync('document.pdf')
if pdf_result.metadata['pdf']
  pdf_meta = pdf_result.metadata['pdf']
  puts "Pages: #{pdf_meta['page_count']}"
  puts "Author: #{pdf_meta['author']}"
  puts "Title: #{pdf_meta['title']}"
  puts "Subject: #{pdf_meta['subject']}"
  puts "Keywords: #{pdf_meta['keywords']}"
end

# HTML metadata
html_result = Kreuzberg.extract_file_sync('page.html')
if html_result.metadata['html']
  html_meta = html_result.metadata['html']
  puts "Title: #{html_meta['title']}"
  puts "Description: #{html_meta['description']}"
  puts "Author: #{html_meta['author']}"
  puts "Keywords: #{html_meta['keywords']}"
  puts "Open Graph Title: #{html_meta['og_title']}"
  puts "Open Graph Image: #{html_meta['og_image']}"
end

# Excel metadata
xlsx_result = Kreuzberg.extract_file_sync('spreadsheet.xlsx')
if xlsx_result.metadata['excel']
  excel_meta = xlsx_result.metadata['excel']
  puts "Sheet count: #{excel_meta['sheet_count']}"
  puts "Sheet names: #{excel_meta['sheet_names'].join(', ')}"
end

# Email metadata
eml_result = Kreuzberg.extract_file_sync('message.eml')
if eml_result.metadata['email']
  email_meta = eml_result.metadata['email']
  puts "From: #{email_meta['from']}"
  puts "To: #{email_meta['to']}"
  puts "Subject: #{email_meta['subject']}"
  puts "Date: #{email_meta['date']}"
end
```

### Tables

```ruby
require 'kreuzberg'

result = Kreuzberg.extract_file_sync('document.pdf')

# Tables are returned as an array of hashes
result.tables.each_with_index do |table, i|
  puts "\n=== Table #{i + 1} ==="
  puts "Rows: #{table['cells'].length}"
  puts "Markdown:\n#{table['markdown']}"

  # Access individual cells
  table['cells'].each_with_index do |row, row_idx|
    puts "Row #{row_idx + 1}: #{row.join(' | ')}"
  end
end
```

### Detected Languages

```ruby
require 'kreuzberg'

config = Kreuzberg::Config::Extraction.new(
  language_detection: Kreuzberg::Config::LanguageDetection.new
)

result = Kreuzberg.extract_file_sync('multilingual.pdf', config: config)

if result.detected_languages
  puts "Detected #{result.detected_languages.length} language(s):"
  result.detected_languages.each do |lang|
    puts "  - #{lang}"
  end
else
  puts "Language detection not enabled or no languages detected"
end
```

### Chunks

```ruby
require 'kreuzberg'

config = Kreuzberg::Config::Extraction.new(
  chunking: Kreuzberg::Config::Chunking.new(
    max_chars: 500,
    max_overlap: 50
  )
)

result = Kreuzberg.extract_file_sync('large_document.pdf', config: config)

if result.chunks
  puts "Document split into #{result.chunks.length} chunks:"

  result.chunks.each_with_index do |chunk, i|
    puts "\n--- Chunk #{i + 1} ---"
    puts "Length: #{chunk.length} characters"
    puts "Preview: #{chunk[0..100]}..."
  end
else
  puts "Chunking not enabled"
end
```

## Error Handling

All errors inherit from `StandardError`:

```ruby
require 'kreuzberg'

begin
  result = Kreuzberg.extract_file_sync('document.pdf')
  puts "Extracted #{result.content.length} characters"
rescue StandardError => e
  puts "Extraction failed: #{e.message}"
  puts "Backtrace: #{e.backtrace.first(3).join("\n")}"
end
```

### Handling Specific Error Cases

```ruby
require 'kreuzberg'

begin
  result = Kreuzberg.extract_file_sync('document.pdf')
  puts result.content
rescue => e
  case e.message
  when /file not found/i, /no such file/i
    puts "Error: File does not exist"
  when /unsupported/i, /not supported/i
    puts "Error: File format not supported"
  when /ocr/i
    puts "Error: OCR processing failed - is Tesseract installed?"
  when /password/i, /encrypted/i
    puts "Error: Document is password-protected"
  else
    puts "Error: #{e.message}"
  end
end
```

## Advanced Topics

### Cache Management

```ruby
require 'kreuzberg'

# Clear the extraction cache
Kreuzberg.clear_cache

# Get cache statistics
stats = Kreuzberg.cache_stats
puts "Cache entries: #{stats[:total_entries]}"
puts "Cache size: #{stats[:total_size_bytes]} bytes"
```

### MIME Type Hints

When the file extension doesn't match the content, provide a MIME type hint:

```ruby
require 'kreuzberg'

# File has wrong extension or no extension
result = Kreuzberg.extract_file_sync(
  'document_without_extension',
  mime_type: 'application/pdf'
)
```

### Working with Binary Data

```ruby
require 'kreuzberg'

# Download from URL
require 'net/http'
uri = URI('https://example.com/document.pdf')
pdf_data = Net::HTTP.get(uri)

# Extract from bytes
result = Kreuzberg.extract_bytes_sync(pdf_data, 'application/pdf')
puts result.content

# Or from uploaded file in Rails
# def upload
#   file = params[:file]
#   data = file.read
#   result = Kreuzberg.extract_bytes_sync(data, file.content_type)
#   render json: { content: result.content }
# end
```

### Password-Protected PDFs

```ruby
require 'kreuzberg'

# Single password
pdf_config = Kreuzberg::Config::PDF.new(passwords: ['secret123'])

# Multiple passwords (tries in order)
pdf_config = Kreuzberg::Config::PDF.new(
  passwords: ['password1', 'password2', 'password3']
)

config = Kreuzberg::Config::Extraction.new(pdf_options: pdf_config)

begin
  result = Kreuzberg.extract_file_sync('encrypted.pdf', config: config)
  puts "Successfully decrypted and extracted"
rescue StandardError => e
  puts "Failed to decrypt: #{e.message}"
end
```

## Performance Tips

1. **Use batch processing** for multiple files:
   ```ruby
   results = Kreuzberg.batch_extract_files_sync(files)
   ```

2. **Enable caching** for repeated extractions:
   ```ruby
   config = Kreuzberg::Config::Extraction.new(use_cache: true)
   ```

3. **Use threads** for I/O-bound workloads:
   ```ruby
   threads = files.map { |f| Thread.new { Kreuzberg.extract_file_sync(f) } }
   results = threads.map(&:value)
   ```

4. **Configure OCR appropriately** (300 DPI is usually sufficient)

5. **Use quality processing** only when needed (adds overhead):
   ```ruby
   config = Kreuzberg::Config::Extraction.new(enable_quality_processing: false)
   ```

## Integration Examples

### Rails Controller

```ruby
class DocumentsController < ApplicationController
  def extract
    file = params[:file]

    result = Kreuzberg.extract_bytes_sync(
      file.read,
      file.content_type
    )

    render json: {
      content: result.content,
      mime_type: result.mime_type,
      metadata: result.metadata,
      tables: result.tables
    }
  rescue StandardError => e
    render json: { error: e.message }, status: :unprocessable_entity
  end
end
```

### Background Job (Sidekiq)

```ruby
class ExtractDocumentJob < ApplicationJob
  queue_as :default

  def perform(document_id)
    document = Document.find(document_id)

    result = Kreuzberg.extract_file_sync(document.file.path)

    document.update!(
      extracted_content: result.content,
      extracted_metadata: result.metadata,
      extracted_at: Time.current
    )
  rescue StandardError => e
    document.update!(extraction_error: e.message)
    raise
  end
end
```

### Rake Task

```ruby
# lib/tasks/documents.rake
namespace :documents do
  desc "Extract text from all documents"
  task extract: :environment do
    Document.find_each do |document|
      next unless document.file.attached?

      puts "Processing #{document.id}..."

      begin
        result = Kreuzberg.extract_file_sync(document.file.path)
        document.update!(extracted_content: result.content)
        puts "✓ Success"
      rescue StandardError => e
        puts "✗ Error: #{e.message}"
      end
    end
  end
end
```

## Next Steps

- **[Python Examples](python.md)** - Examples for Python
- **[TypeScript Examples](typescript.md)** - Examples for Node.js/TypeScript
- **[Rust Examples](rust.md)** - Examples for Rust applications
- **[Quick Start Guide](../getting-started/quickstart.md)** - Get started quickly
- **[Installation](../getting-started/installation.md)** - Installation instructions
