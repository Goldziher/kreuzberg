```ruby
require 'kreuzberg'

config = Kreuzberg::Config::Extraction.new(
  ocr: Kreuzberg::Config::OCR.new(backend: 'tesseract'),
  pdf: Kreuzberg::Config::PDF.new(dpi: 300)
)

result = Kreuzberg.extract_file_sync('scanned.pdf', config: config)
```
