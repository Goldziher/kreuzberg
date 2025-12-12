```ruby title="Ruby"
require 'kreuzberg'

begin
  result = Kreuzberg.extract_file_sync('document.pdf')
  puts result.content
rescue Kreuzberg::ValidationError => e
  puts "Invalid configuration: #{e.message}"
rescue Kreuzberg::ParsingError => e
  puts "Failed to parse document: #{e.message}"
rescue Kreuzberg::OCRError => e
  puts "OCR processing failed: #{e.message}"
rescue Kreuzberg::MissingDependencyError => e
  puts "Missing dependency: #{e.message}"
rescue Kreuzberg::Error => e
  puts "Extraction error: #{e.message}"
rescue StandardError => e
  puts "System error: #{e.message}"
end
```
