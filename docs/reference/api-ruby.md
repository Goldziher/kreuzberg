# Ruby API Reference

Complete reference for the Kreuzberg Ruby API.

## Installation

Add to your `Gemfile`:

```ruby title="Terminal"
gem 'kreuzberg'
```

Or install directly:

```bash title="Terminal"
gem install kreuzberg
```

## Core Functions

### Kreuzberg.extract_file_sync()

Extract content from a file (synchronous).

**Signature:**

```ruby title="Ruby"
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

```ruby title="error_handling.rb"
require 'kreuzberg'

result = Kreuzberg.extract_file_sync("document.pdf")
puts result.content
puts "Pages: #{result.metadata['page_count']}"
```

**Example - With configuration hash:**

```ruby title="config.rb"
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

```ruby title="config.rb"
require 'kreuzberg'

config = Kreuzberg::Config::Extraction.new(force_ocr: true)
result = Kreuzberg.extract_file_sync("document.pdf", config: config)
```

**Example - With explicit MIME type:**

```ruby title="config.rb"
require 'kreuzberg'

result = Kreuzberg.extract_file_sync("document.pdf", mime_type: "application/pdf")
```

---

### Kreuzberg.extract_file()

Extract content from a file (asynchronous via Tokio runtime).

**Note:** Ruby doesn't have native async/await. This uses a blocking Tokio runtime internally. For background processing, use `extract_file_sync` in a Thread.

**Signature:**

```ruby title="Ruby"
Kreuzberg.extract_file(path, mime_type: nil, config: nil) -> Kreuzberg::Result
```

**Parameters:**

Same as [`extract_file_sync()`](#kreuzbergextract_file_sync).

**Returns:**

- `Kreuzberg::Result`: Extraction result object

**Examples:**

```ruby title="basic_extraction.rb"
# Equivalent to extract_file_sync in Ruby
result = Kreuzberg.extract_file("document.pdf")
puts result.content
```

---

### Kreuzberg.extract_bytes_sync()

Extract content from bytes (synchronous).

**Signature:**

```ruby title="Ruby"
Kreuzberg.extract_bytes_sync(data, mime_type, config: nil) -> Kreuzberg::Result
```

**Parameters:**

- `data` (String): Binary data to extract (Ruby String in binary encoding)
- `mime_type` (String): MIME type of the data (required for format detection)
- `config` (Hash, Kreuzberg::Config::Extraction, nil): Extraction configuration

**Returns:**

- `Kreuzberg::Result`: Extraction result object

**Examples:**

```ruby title="basic_extraction.rb"
data = File.binread("document.pdf")
result = Kreuzberg.extract_bytes_sync(data, "application/pdf")
puts result.content
```

---

### Kreuzberg.extract_bytes()

Extract content from bytes (asynchronous via Tokio runtime).

**Signature:**

```ruby title="Ruby"
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

```ruby title="Ruby"
Kreuzberg.batch_extract_files_sync(paths, config: nil) -> Array<Kreuzberg::Result>
```

**Parameters:**

- `paths` (Array<String>): Array of file paths to extract
- `config` (Hash, Kreuzberg::Config::Extraction, nil): Extraction configuration applied to all files

**Returns:**

- `Array<Kreuzberg::Result>`: Array of extraction result objects

**Examples:**

```ruby title="basic_extraction.rb"
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

```ruby title="Ruby"
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

```ruby title="config.rb"
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

```ruby title="config.rb"
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

```ruby title="basic_extraction.rb"
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

```ruby title="basic_extraction.rb"
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

```ruby title="basic_extraction.rb"
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

```ruby title="error_handling.rb"
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

```ruby title="Ruby"
Kreuzberg.clear_cache() -> nil
```

**Example:**

```ruby title="basic_extraction.rb"
Kreuzberg.clear_cache
```

**Note:** Cache clearing is currently not implemented in the FFI layer (TODO).

---

### Kreuzberg.cache_stats()

Get cache statistics.

**Signature:**

```ruby title="Ruby"
Kreuzberg.cache_stats() -> Hash
```

**Returns:**

- Hash with `:total_entries` (Integer) and `:total_size_bytes` (Integer)

**Example:**

```ruby title="extract_from_bytes.rb"
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

```ruby title="basic_extraction.rb"
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

```ruby title="basic_extraction.rb"
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

```ruby title="basic_extraction.rb"
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

```ruby title="basic_extraction.rb"
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

```ruby title="batch_processing.rb"
# Better approach
results = Kreuzberg.batch_extract_files_sync(files)
results.each_with_index do |result, i|
  puts "#{files[i]}: #{result.content.length} characters"
end
```

## Batch Extract from Bytes

### `batch_extract_bytes_sync(data_list, mime_types, config = nil)`

Extract text from multiple byte arrays synchronously.

**Parameters:**
- `data_list` (Array<String>): Array of byte strings (binary data)
- `mime_types` (Array<String>): Array of MIME types corresponding to each byte array
- `config` (Hash, optional): Extraction configuration

**Returns:** Array<Hash> - Array of extraction results

**Example:**

```ruby title="basic_extraction.rb"
require 'kreuzberg'

# Read multiple files into memory
pdf_data = File.binread('invoice.pdf')
docx_data = File.binread('report.docx')
xlsx_data = File.binread('data.xlsx')

data_list = [pdf_data, docx_data, xlsx_data]
mime_types = [
  'application/pdf',
  'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
  'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet'
]

results = Kreuzberg.batch_extract_bytes_sync(data_list, mime_types)

results.each_with_index do |result, i|
  puts "Document #{i}:"
  puts "  Content length: #{result[:content].length}"
  puts "  Format: #{result[:metadata][:format_type]}"
end
```

**With OCR Configuration:**

```ruby title="with_ocr.rb"
config = {
  ocr: {
    backend: 'tesseract',
    language: 'eng'
  }
}

results = Kreuzberg.batch_extract_bytes_sync(data_list, mime_types, config)
```

### `batch_extract_bytes(data_list, mime_types, config = nil)`

Extract text from multiple byte arrays asynchronously.

**Parameters:** Same as `batch_extract_bytes_sync`

**Returns:** Array<Hash> - Array of extraction results

**Example:**

```ruby title="basic_extraction.rb"
require 'kreuzberg'

# Asynchronous batch extraction from bytes
data_list = files.map { |f| File.binread(f) }
mime_types = files.map { |f| 'application/pdf' }

results = Kreuzberg.batch_extract_bytes(data_list, mime_types, {
  chunking: {
    max_chars: 1024,
    max_overlap: 200
  }
})

results.each_with_index do |result, i|
  puts "Document #{i}: #{result[:content][0..100]}..."
  if result[:chunks]
    puts "  Chunks: #{result[:chunks].length}"
  end
end
```

## Extensibility

Kreuzberg's plugin system allows you to extend functionality with custom post-processors, validators, and OCR backends.

### Custom Post-Processors

Post-processors modify extraction results after document processing. They can enrich metadata, transform content, or add custom logic.

**Interface:**

```ruby title="Ruby"
class CustomPostProcessor
  def call(result)
    # Modify result and return it
    # result is a Hash with keys: :content, :metadata, :tables, etc.
    result
  end

  def processing_stage
    # Optional: Return :early, :default, or :late
    :default
  end
end
```

**Example:**

```ruby title="basic_extraction.rb"
require 'kreuzberg'

class WordCountProcessor
  def call(result)
    # Skip if no content
    return result if result[:content].nil? || result[:content].empty?

    # Count words
    word_count = result[:content].split.length

    # Add to metadata
    result[:metadata] ||= {}
    result[:metadata][:word_count] = word_count
    result[:metadata][:processed_by] = 'WordCountProcessor'

    result
  end

  def processing_stage
    :early  # Run early in the pipeline
  end
end

# Register the processor
processor = WordCountProcessor.new
Kreuzberg.register_post_processor('word_count', 100, processor)

# Use in extraction
result = Kreuzberg.extract_file_sync('document.pdf')
puts "Words: #{result[:metadata][:word_count]}"

# Unregister when done
Kreuzberg.unregister_post_processor('word_count')
```

**Stateful Processor Example:**

```ruby title="custom_post_processor.rb"
class PdfMetadataExtractor
  def initialize
    @processed_count = 0
    @start_time = Time.now
  end

  def call(result)
    # Only process PDFs
    return result unless result[:metadata][:mime_type] == 'application/pdf'

    @processed_count += 1

    # Add processing metadata
    result[:metadata][:pdf_processed] = true
    result[:metadata][:processing_order] = @processed_count
    result[:metadata][:processing_timestamp] = Time.now.to_i
    result[:metadata][:elapsed_seconds] = (Time.now - @start_time).round(2)

    result
  end

  def statistics
    {
      processed_count: @processed_count,
      running_for: (Time.now - @start_time).round(2)
    }
  end
end

processor = PdfMetadataExtractor.new
Kreuzberg.register_post_processor('pdf_metadata', 90, processor)

# Process multiple files
files = ['doc1.pdf', 'doc2.pdf', 'doc3.pdf']
results = Kreuzberg.batch_extract_files_sync(files)

puts "Statistics: #{processor.statistics}"
```

### Custom Validators

Validators check extraction results and raise errors if validation fails.

**Interface:**

```ruby title="Ruby"
class CustomValidator
  def call(result)
    # Check result
    # Raise Kreuzberg::Errors::ValidationError if invalid
    # Return nil if valid
  end
end
```

**Example:**

```ruby title="basic_extraction.rb"
require 'kreuzberg'

class MinimumLengthValidator
  def initialize(min_length: 100)
    @min_length = min_length
  end

  def call(result)
    content = result[:content] || ''

    if content.strip.length < @min_length
      raise Kreuzberg::Errors::ValidationError,
            "Content too short: #{content.length} chars, minimum #{@min_length} required"
    end
  end
end

# Register validator
validator = MinimumLengthValidator.new(min_length: 50)
Kreuzberg.register_validator('min_length', 100, validator)

# This will raise if content is too short
begin
  result = Kreuzberg.extract_file_sync('short_document.txt')
rescue Kreuzberg::Errors::ValidationError => e
  puts "Validation failed: #{e.message}"
end
```

**Quality Score Validator:**

```ruby title="custom_validator.rb"
class QualityScoreValidator
  def call(result)
    metadata = result[:metadata] || {}
    quality_score = metadata[:quality_score] || 0.0

    if quality_score < 0.5
      raise Kreuzberg::Errors::ValidationError,
            format('Quality score %.2f below threshold 0.50', quality_score)
    end
  end
end

validator = QualityScoreValidator.new
Kreuzberg.register_validator('quality_check', 90, validator)
```

### Custom OCR Backends

Implement custom OCR backends for specialized OCR engines or cloud services.

**Interface:**

```ruby title="Ruby"
class CustomOcrBackend
  def name
    'custom-ocr'
  end

  def supported_languages
    ['eng', 'fra', 'deu']
  end

  def process_image(image_data, language)
    # Process image and return OCR result
    # image_data is a binary string
    # Return Hash with: :content, :mime_type, :metadata, :tables
  end
end
```

**Example:**

```ruby title="basic_extraction.rb"
require 'kreuzberg'
require 'net/http'
require 'json'

class CloudOcrBackend
  def name
    'cloud-ocr'
  end

  def supported_languages
    ['eng', 'fra', 'deu', 'spa', 'jpn', 'chi_sim']
  end

  def process_image(image_data, language)
    # Send to cloud OCR service
    uri = URI('https://api.example.com/ocr')
    request = Net::HTTP::Post.new(uri)
    request['Content-Type'] = 'image/jpeg'
    request['Accept-Language'] = language
    request.body = image_data

    response = Net::HTTP.start(uri.hostname, uri.port, use_ssl: true) do |http|
      http.request(request)
    end

    # Parse response
    data = JSON.parse(response.body)

    {
      content: data['text'],
      mime_type: 'text/plain',
      metadata: {
        backend: 'cloud-ocr',
        language: language,
        confidence: data['confidence']
      },
      tables: []
    }
  end
end

# Register OCR backend
backend = CloudOcrBackend.new
Kreuzberg.register_ocr_backend(backend)

# Use with extraction
config = {
  ocr: {
    backend: 'cloud-ocr',
    language: 'eng'
  }
}

result = Kreuzberg.extract_file_sync('scanned.pdf', config)
```

### Plugin Management

**Listing Plugins:**

```ruby title="basic_extraction.rb"
# List all registered plugins
post_processors = Kreuzberg.list_post_processors
validators = Kreuzberg.list_validators
ocr_backends = Kreuzberg.list_ocr_backends

puts "Post-processors: #{post_processors.join(', ')}"
puts "Validators: #{validators.join(', ')}"
puts "OCR backends: #{ocr_backends.join(', ')}"
```

**Clearing Plugins:**

```ruby title="with_ocr.rb"
# Clear all post-processors
Kreuzberg.clear_post_processors

# Clear all validators
Kreuzberg.clear_validators
```

**Unregistering Specific Plugins:**

```ruby title="custom_validator.rb"
# Unregister by name
Kreuzberg.unregister_post_processor('word_count')
Kreuzberg.unregister_validator('min_length')
Kreuzberg.unregister_ocr_backend('cloud-ocr')
```
