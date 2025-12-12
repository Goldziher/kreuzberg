```ruby title="Ruby"
require "kreuzberg"

config = Kreuzberg::Config::Extraction.new(
  use_cache: true,
  ocr: Kreuzberg::Config::OCR.new(
    backend: "tesseract",
    language: "eng+deu",
    tesseract: Kreuzberg::Config::Tesseract.new(psm: 6)
  ),
  chunking: Kreuzberg::Config::Chunking.new(
    max_characters: 1000,
    overlap: 200
  )
)

result = Kreuzberg.extract_file_sync("document.pdf", config)
puts "Content length: #{result.content.length}"
```
