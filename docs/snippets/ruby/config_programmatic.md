```ruby
require "kreuzberg"

config = Kreuzberg::ExtractionConfig.new(
  use_cache: true,
  ocr: Kreuzberg::OcrConfig.new(
    backend: "tesseract",
    language: "eng+deu",
    tesseract: Kreuzberg::TesseractConfig.new(psm: 6)
  ),
  chunking: Kreuzberg::ChunkingConfig.new(
    max_characters: 1000,
    overlap: 200
  )
)

result = Kreuzberg.extract_file_sync("document.pdf", config)
puts "Content length: #{result.content.length}"
```
