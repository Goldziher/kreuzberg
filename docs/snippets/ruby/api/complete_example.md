```ruby title="Ruby"
require 'kreuzberg'

config = Kreuzberg::Config::Extraction.new(
  use_cache: true,
  enable_quality_processing: true,
  force_ocr: false,
  ocr: Kreuzberg::Config::OCR.new(
    backend: 'tesseract',
    language: 'eng+fra'
  ),
  pdf_options: Kreuzberg::Config::PDF.new(
    extract_images: true,
    extract_metadata: true
  ),
  image_extraction: Kreuzberg::Config::ImageExtraction.new(
    extract_images: true,
    target_dpi: 150,
    max_image_dimension: 4096
  ),
  chunking: Kreuzberg::Config::Chunking.new(
    max_chars: 1000,
    max_overlap: 200
  ),
  token_reduction: Kreuzberg::Config::TokenReduction.new(mode: 'moderate'),
  language_detection: Kreuzberg::Config::LanguageDetection.new(
    enabled: true,
    min_confidence: 0.8
  ),
  postprocessor: Kreuzberg::Config::PostProcessor.new(enabled: true)
)

result = Kreuzberg.extract_file_sync('document.pdf', config: config)
puts "Content length: #{result.content.length}"
```
