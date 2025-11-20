```ruby
require 'kreuzberg'

config = Kreuzberg::ExtractionConfig.new(
  ocr: Kreuzberg::OcrConfig.new(
    language: 'eng+fra+deu',
    tesseract_config: Kreuzberg::TesseractConfig.new(
      psm: 6,
      oem: 1,
      min_confidence: 0.8,
      tessedit_char_whitelist: 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789 .,!?',
      enable_table_detection: true
    )
  )
)
```
