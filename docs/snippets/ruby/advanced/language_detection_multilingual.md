```ruby title="Ruby"
require 'kreuzberg'

config = Kreuzberg::Config::Extraction.new(
  language_detection: Kreuzberg::Config::LanguageDetection.new(
    enabled: true,
    min_confidence: 0.8,
    detect_multiple: true
  )
)

result = Kreuzberg.extract_file_sync('multilingual_document.pdf', config: config)

languages = result.detected_languages || []

if languages.any?
  puts "Detected #{languages.length} language(s): #{languages.join(', ')}"
else
  puts "No languages detected"
end

puts "Total content: #{result.content.length} characters"
puts "MIME type: #{result.mime_type}"
```
