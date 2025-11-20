```ruby
require 'kreuzberg'

config = Kreuzberg::ExtractionConfig.new(
  ocr: Kreuzberg::OcrConfig.new(
    backend: 'tesseract',
    language: 'eng+fra',
    tesseract_config: Kreuzberg::TesseractConfig.new(psm: 3)
  )
)
```
