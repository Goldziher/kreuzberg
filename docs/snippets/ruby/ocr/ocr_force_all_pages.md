```ruby title="Ruby"
require 'kreuzberg'

config = Kreuzberg::Config::Extraction.new(
  ocr: Kreuzberg::Config::OCR.new(backend: 'tesseract'),
  force_ocr: true
)

result = Kreuzberg.extract_file_sync('document.pdf', config: config)
puts result.content
```
