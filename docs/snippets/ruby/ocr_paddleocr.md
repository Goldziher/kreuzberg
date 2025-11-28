```ruby
require 'kreuzberg'

config = Kreuzberg::Config::Extraction.new(
  ocr: Kreuzberg::Config::OCR.new(
    backend: 'paddleocr',
    language: 'eng'
  )
)

result = Kreuzberg.extract_file_sync('scanned.pdf', config: config)
puts result.content[0..100]
puts "Total length: #{result.content.length}"
```
