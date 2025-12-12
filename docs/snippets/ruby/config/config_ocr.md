```ruby title="Ruby"
require 'kreuzberg'

config = Kreuzberg::Config::Extraction.new(
  ocr: Kreuzberg::Config::OCR.new(
    backend: 'tesseract',
    language: 'eng+fra',
    tesseract_config: Kreuzberg::Config::Tesseract.new(psm: 3)
  )
)
```
