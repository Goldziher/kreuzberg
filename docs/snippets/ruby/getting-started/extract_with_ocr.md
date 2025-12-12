```ruby title="Ruby"
require 'kreuzberg'

ocr_config = Kreuzberg::Config::OCR.new(
  backend: 'tesseract',
  language: 'eng'
)

config = Kreuzberg::Config::Extraction.new(ocr: ocr_config)
result = Kreuzberg.extract_file_sync('scanned.pdf', config: config)

puts "Extracted text from scanned document:"
puts result.content
puts "Used OCR backend: tesseract"
```
