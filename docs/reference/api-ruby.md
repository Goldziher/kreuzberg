# Ruby API Reference

Complete reference for the Kreuzberg Ruby API.

## Installation

Add to your `Gemfile`:

```ruby
gem 'kreuzberg'
```

Or install directly:

```bash
gem install kreuzberg
```

## Core Functions

### Kreuzberg.extract_file_sync()

Extract content from a file (synchronous).

**Signature:**

```ruby
Kreuzberg.extract_file_sync(path, mime_type: nil, config: nil) -> Kreuzberg::Result
```

**Parameters:**

- `path` (String): Path to the file to extract
- `mime_type` (String, nil): Optional MIME type hint. If nil, MIME type is auto-detected
- `config` (Hash, Kreuzberg::Config::Extraction, nil): Extraction configuration. Uses defaults if nil

**Returns:**

- `Kreuzberg::Result`: Extraction result object

**Raises:**

- `StandardError`: Base error for all extraction failures

**Example - Basic usage:**

```ruby
require 'kreuzberg'

result = Kreuzberg.extract_file_sync("document.pdf")
puts result.content
puts "Pages: #{result.metadata['page_count']}"
```

**Example - With configuration hash:**

```ruby
require 'kreuzberg'

config = {
  ocr: {
    backend: 'tesseract',
    language: 'eng'
  }
}
result = Kreuzberg.extract_file_sync("scanned.pdf", config: config)
```

**Example - With config object:**

```ruby
require 'kreuzberg'

config = Kreuzberg::Config::Extraction.new(force_ocr: true)
result = Kreuzberg.extract_file_sync("document.pdf", config: config)
```

**Example - With explicit MIME type:**

```ruby
require 'kreuzberg'

result = Kreuzberg.extract_file_sync("document.pdf", mime_type: "application/pdf")
```

---

### Kreuzberg.extract_file()

Extract content from a file (asynchronous via Tokio runtime).

**Note:** Ruby doesn't have native async/await. This uses a blocking Tokio runtime internally. For background processing, use `extract_file_sync` in a Thread.

**Signature:**

```ruby
Kreuzberg.extract_file(path, mime_type: nil, config: nil) -> Kreuzberg::Result
```

**Parameters:**

Same as [`extract_file_sync()`](#kreuzbergextract_file_sync).

**Returns:**

- `Kreuzberg::Result`: Extraction result object

**Examples:**

```ruby
# Equivalent to extract_file_sync in Ruby
result = Kreuzberg.extract_file("document.pdf")
puts result.content
```

---

### Kreuzberg.extract_bytes_sync()

Extract content from bytes (synchronous).

**Signature:**

```ruby
Kreuzberg.extract_bytes_sync(data, mime_type, config: nil) -> Kreuzberg::Result
```

**Parameters:**

- `data` (String): Binary data to extract (Ruby String in binary encoding)
- `mime_type` (String): MIME type of the data (required for format detection)
- `config` (Hash, Kreuzberg::Config::Extraction, nil): Extraction configuration

**Returns:**

- `Kreuzberg::Result`: Extraction result object

**Examples:**

```ruby
data = File.binread("document.pdf")
result = Kreuzberg.extract_bytes_sync(data, "application/pdf")
puts result.content
```

---

### Kreuzberg.extract_bytes()

Extract content from bytes (asynchronous via Tokio runtime).

**Signature:**

```ruby
Kreuzberg.extract_bytes(data, mime_type, config: nil) -> Kreuzberg::Result
```

**Parameters:**

Same as [`extract_bytes_sync()`](#kreuzbergextract_bytes_sync).

**Returns:**

- `Kreuzberg::Result`: Extraction result object

---

### Kreuzberg.batch_extract_files_sync()

Extract content from multiple files in parallel (synchronous).

**Signature:**

```ruby
Kreuzberg.batch_extract_files_sync(paths, config: nil) -> Array<Kreuzberg::Result>
```

**Parameters:**

- `paths` (Array<String>): Array of file paths to extract
- `config` (Hash, Kreuzberg::Config::Extraction, nil): Extraction configuration applied to all files

**Returns:**

- `Array<Kreuzberg::Result>`: Array of extraction result objects

**Examples:**

```ruby
paths = ["doc1.pdf", "doc2.docx", "doc3.xlsx"]
results = Kreuzberg.batch_extract_files_sync(paths)

results.each_with_index do |result, i|
  puts "#{paths[i]}: #{result.content.length} characters"
end
```

---

### Kreuzberg.batch_extract_files()

Extract content from multiple files in parallel (asynchronous via Tokio runtime).

**Signature:**

```ruby
Kreuzberg.batch_extract_files(paths, config: nil) -> Array<Kreuzberg::Result>
```

**Parameters:**

Same as [`batch_extract_files_sync()`](#kreuzbergbatch_extract_files_sync).

**Returns:**

- `Array<Kreuzberg::Result>`: Array of extraction result objects

---

## Configuration

### Hash Configuration

The simplest way to configure extraction is using a Hash:

**Example:**

```ruby
config = {
  ocr: {
    backend: 'tesseract',
    language: 'eng',
    tesseract_config: {
      psm: 6,
      enable_table_detection: true
    }
  },
  force_ocr: false,
  pdf_options: {
    passwords: ['password1', 'password2'],
    extract_images: true,
    image_dpi: 300
  },
  language_detection: {
    enabled: true,
    confidence_threshold: 0.7
  }
}

result = Kreuzberg.extract_file_sync("document.pdf", config: config)
```

**Available Options:**

- `ocr` (Hash): OCR configuration
  - `backend` (String): OCR backend ("tesseract"). Default: "tesseract"
  - `language` (String): Language code (ISO 639-3). Default: "eng"
  - `tesseract_config` (Hash): Tesseract-specific options
    - `psm` (Integer): Page segmentation mode (0-13). Default: 3
    - `oem` (Integer): OCR engine mode (0-3). Default: 3
    - `enable_table_detection` (Boolean): Enable table detection. Default: false
    - `tessedit_char_whitelist` (String): Character whitelist. Default: nil
    - `tessedit_char_blacklist` (String): Character blacklist. Default: nil

- `force_ocr` (Boolean): Force OCR even for text-based PDFs. Default: false

- `pdf_options` (Hash): PDF-specific options
  - `passwords` (Array<String>): Passwords to try for encrypted PDFs. Default: nil
  - `extract_images` (Boolean): Extract images from PDF. Default: false
  - `image_dpi` (Integer): DPI for image extraction. Default: 300

- `chunking` (Hash): Text chunking options
  - `chunk_size` (Integer): Maximum chunk size in tokens. Default: 512
  - `chunk_overlap` (Integer): Overlap between chunks. Default: 50
  - `chunking_strategy` (String): Strategy ("fixed", "semantic"). Default: "fixed"

- `language_detection` (Hash): Language detection options
  - `enabled` (Boolean): Enable language detection. Default: true
  - `confidence_threshold` (Float): Minimum confidence (0.0-1.0). Default: 0.5

---

### Kreuzberg::Config::Extraction

Object-oriented configuration using Ruby classes.

**Example:**

```ruby
config = Kreuzberg::Config::Extraction.new(
  force_ocr: true,
  ocr: Kreuzberg::Config::Ocr.new(
    backend: 'tesseract',
    language: 'eng'
  )
)

result = Kreuzberg.extract_file_sync("document.pdf", config: config)
```

---

## Results & Types

### Kreuzberg::Result

Result object returned by all extraction functions.

**Attributes:**

- `content` (String): Extracted text content
- `mime_type` (String): MIME type of the processed document
- `metadata` (Hash): Document metadata (format-specific fields)
- `tables` (Array<Hash>): Array of extracted tables
- `detected_languages` (Array<String>, nil): Array of detected language codes if language detection is enabled

**Example:**

```ruby
result = Kreuzberg.extract_file_sync("document.pdf")

puts "Content: #{result.content}"
puts "MIME type: #{result.mime_type}"
puts "Page count: #{result.metadata['page_count']}"
puts "Tables: #{result.tables.length}"

if result.detected_languages
  puts "Languages: #{result.detected_languages.join(', ')}"
end
```

---

### Metadata Hash

Document metadata with format-specific fields.

**Common Fields:**

- `language` (String): Document language (ISO 639-1 code)
- `date` (String): Document date (ISO 8601 format)
- `subject` (String): Document subject
- `format_type` (String): Format discriminator ("pdf", "excel", "email", etc.)

**PDF-Specific Fields** (when `format_type == "pdf"`):

- `title` (String): PDF title
- `author` (String): PDF author
- `page_count` (Integer): Number of pages
- `creation_date` (String): Creation date (ISO 8601)
- `modification_date` (String): Modification date (ISO 8601)
- `creator` (String): Creator application
- `producer` (String): Producer application
- `keywords` (String): PDF keywords

**Excel-Specific Fields** (when `format_type == "excel"`):

- `sheet_count` (Integer): Number of sheets
- `sheet_names` (Array<String>): List of sheet names

**Email-Specific Fields** (when `format_type == "email"`):

- `from_email` (String): Sender email address
- `from_name` (String): Sender name
- `to_emails` (Array<String>): Recipient email addresses
- `cc_emails` (Array<String>): CC email addresses
- `bcc_emails` (Array<String>): BCC email addresses
- `message_id` (String): Email message ID
- `attachments` (Array<String>): List of attachment filenames

**Example:**

```ruby
result = Kreuzberg.extract_file_sync("document.pdf")
metadata = result.metadata

if metadata['format_type'] == 'pdf'
  puts "Title: #{metadata['title']}"
  puts "Author: #{metadata['author']}"
  puts "Pages: #{metadata['page_count']}"
end
```

See the Types Reference for complete metadata field documentation.

---

### Table Hash

Extracted table structure.

**Fields:**

- `cells` (Array<Array<String>>): 2D array of table cells (rows x columns)
- `markdown` (String): Table rendered as markdown
- `page_number` (Integer): Page number where table was found

**Example:**

```ruby
result = Kreuzberg.extract_file_sync("invoice.pdf")

result.tables.each do |table|
  puts "Table on page #{table['page_number']}:"
  puts table['markdown']
  puts
end
```

---

## Error Handling

All errors are raised as `StandardError` with descriptive messages.

**Example:**

```ruby
begin
  result = Kreuzberg.extract_file_sync("document.pdf")
  puts result.content
rescue StandardError => e
  puts "Extraction failed: #{e.message}"

  # Check error details
  case e.message
  when /file not found/i
    puts "File does not exist"
  when /parsing/i
    puts "Failed to parse document"
  when /OCR/i
    puts "OCR processing failed"
  else
    puts "Unknown error"
  end
end
```

---

## Cache Management

### Kreuzberg.clear_cache()

Clear the extraction cache.

**Signature:**

```ruby
Kreuzberg.clear_cache() -> nil
```

**Example:**

```ruby
Kreuzberg.clear_cache
```

**Note:** Cache clearing is currently not implemented in the FFI layer (TODO).

---

### Kreuzberg.cache_stats()

Get cache statistics.

**Signature:**

```ruby
Kreuzberg.cache_stats() -> Hash
```

**Returns:**

- Hash with `:total_entries` (Integer) and `:total_size_bytes` (Integer)

**Example:**

```ruby
stats = Kreuzberg.cache_stats
puts "Cache entries: #{stats[:total_entries]}"
puts "Cache size: #{stats[:total_size_bytes]} bytes"
```

**Note:** Cache statistics are currently not implemented in the FFI layer (TODO).

---

## CLI Proxy

### Kreuzberg::CLIProxy

Wrapper for running the Kreuzberg CLI from Ruby.

**Example:**

```ruby
cli = Kreuzberg::CLIProxy.new

# Extract a file
output = cli.extract("document.pdf")
puts output

# Batch extract
output = cli.batch(["doc1.pdf", "doc2.pdf", "doc3.pdf"])
puts output

# Detect MIME type
mime_type = cli.detect("unknown-file.bin")
puts "MIME type: #{mime_type}"
```

---

## API Proxy

### Kreuzberg::APIProxy

Wrapper for running the Kreuzberg API server from Ruby.

**Example:**

```ruby
api = Kreuzberg::APIProxy.new

# Start server (blocks)
api.start(host: "0.0.0.0", port: 8000)

# Or in a thread
thread = Thread.new do
  api.start(host: "127.0.0.1", port: 9000)
end

# Later...
thread.kill
```

---

## MCP Proxy

### Kreuzberg::MCPProxy

Wrapper for running the Kreuzberg MCP server from Ruby.

**Example:**

```ruby
mcp = Kreuzberg::MCPProxy.new

# Start MCP server (blocks)
mcp.start
```

---

## System Requirements

**Ruby:** 3.0 or higher

**Native Dependencies:**

- Tesseract OCR (for OCR support): `brew install tesseract` (macOS) or `apt-get install tesseract-ocr` (Ubuntu)
- LibreOffice (for legacy Office formats): `brew install libreoffice` (macOS) or `apt-get install libreoffice` (Ubuntu)

**Platforms:**

- Linux (x64, arm64)
- macOS (x64, arm64)
- Windows (x64)

---

## Thread Safety

All Kreuzberg functions are thread-safe and can be called from multiple threads concurrently.

**Example:**

```ruby
threads = []

files = ["doc1.pdf", "doc2.pdf", "doc3.pdf"]
files.each do |file|
  threads << Thread.new do
    result = Kreuzberg.extract_file_sync(file)
    puts "#{file}: #{result.content.length} characters"
  end
end

threads.each(&:join)
```

However, for better performance, use the batch API instead:

```ruby
# Better approach
results = Kreuzberg.batch_extract_files_sync(files)
results.each_with_index do |result, i|
  puts "#{files[i]}: #{result.content.length} characters"
end
```
