```ruby
require 'kreuzberg'

config = Kreuzberg::Config::Extraction.new(
  # Enable OCR
  ocr: Kreuzberg::Config::OCR.new(
    backend: 'tesseract',
    language: 'eng+deu'  # Multiple languages
  ),

  # Enable chunking for LLM processing
  chunking: Kreuzberg::Config::Chunking.new(
    max_chars: 1000,
    max_overlap: 100
  ),

  # Enable language detection
  language_detection: Kreuzberg::Config::LanguageDetection.new,

  # Enable caching
  use_cache: true,

  # Enable quality processing
  enable_quality_processing: true
)

result = Kreuzberg.extract_file_sync('document.pdf', config: config)

# Access chunks
if result.chunks
  result.chunks.each do |chunk|
    puts "Chunk: #{chunk[0..100]}..."
  end
end

# Access detected languages
if result.detected_languages
  puts "Languages: #{result.detected_languages}"
end
```
