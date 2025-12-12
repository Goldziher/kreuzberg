```ruby title="Ruby"
require 'kreuzberg'

config = Kreuzberg::Config::Extraction.new(
  ocr: Kreuzberg::Config::OCR.new(
    language: 'eng+fra+deu',
    tesseract_config: Kreuzberg::Config::Tesseract.new(
      psm: 6,
      oem: 1,
      min_confidence: 0.8,
      tessedit_char_whitelist: 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789 .,!?',
      enable_table_detection: true
    )
  )
)
```
