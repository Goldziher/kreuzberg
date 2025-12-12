```ruby title="Ruby"
require 'kreuzberg'

ocr_config = Kreuzberg::Config::OCR.new(
  backend: 'tesseract',
  language: 'eng'
)

config = Kreuzberg::Config::Extraction.new(ocr: ocr_config)
result = Kreuzberg.extract_file_sync('scanned.pdf', config: config)
puts result.content
```
