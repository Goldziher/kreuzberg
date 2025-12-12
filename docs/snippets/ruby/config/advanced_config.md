```ruby title="Ruby"
require 'kreuzberg'

config = Kreuzberg::Config::Extraction.new(
  ocr: Kreuzberg::Config::OCR.new(
    backend: 'tesseract',
    language: 'eng+deu'
  ),
  chunking: Kreuzberg::Config::Chunking.new(
    max_chars: 1000,
    max_overlap: 100
  ),
  language_detection: Kreuzberg::Config::LanguageDetection.new,
  use_cache: true,
  enable_quality_processing: true
)

result = Kreuzberg.extract_file_sync('document.pdf', config: config)

result.chunks&.each { |chunk| puts chunk[0..100] }
puts "Languages: #{result.detected_languages.inspect}"
```
