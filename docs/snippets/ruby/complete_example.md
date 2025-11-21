```ruby
require 'kreuzberg'

config = Kreuzberg::ExtractionConfig.new(
  use_cache: true,
  enable_quality_processing: true,
  force_ocr: false,
  ocr: Kreuzberg::OcrConfig.new(
    backend: 'tesseract',
    language: 'eng+fra',
    tesseract_config: Kreuzberg::TesseractConfig.new(
      psm: 3,
      oem: 3,
      min_confidence: 0.8,
      preprocessing: Kreuzberg::ImagePreprocessingConfig.new(
        target_dpi: 300,
        denoise: true,
        deskew: true,
        contrast_enhance: true
      ),
      enable_table_detection: true
    )
  ),
  pdf_options: Kreuzberg::PdfConfig.new(
    extract_images: true,
    extract_metadata: true
  ),
  images: Kreuzberg::ImageExtractionConfig.new(
    extract_images: true,
    target_dpi: 150,
    max_image_dimension: 4096
  ),
  chunking: Kreuzberg::ChunkingConfig.new(
    max_chars: 1000,
    max_overlap: 200
  ),
  token_reduction: Kreuzberg::TokenReductionConfig.new(
    mode: 'moderate',
    preserve_important_words: true
  ),
  language_detection: Kreuzberg::LanguageDetectionConfig.new(
    enabled: true,
    min_confidence: 0.8,
    detect_multiple: false
  ),
  postprocessor: Kreuzberg::PostProcessorConfig.new(
    enabled: true
  )
)

result = Kreuzberg.extract_file('document.pdf', config: config)
puts "Extracted content length: #{result.content.length}"
```
