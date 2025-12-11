```ruby
require 'kreuzberg'

config = Kreuzberg::Config::Extraction.new(
  ocr: Kreuzberg::Config::OCR.new(
    backend: 'tesseract',
    language: 'eng+deu+fra'
  )
)

result = Kreuzberg.extract_file_sync('multilingual.pdf', config: config)
puts result.content
```
