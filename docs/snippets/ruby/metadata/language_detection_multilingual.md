```ruby
require 'kreuzberg'

config = Kreuzberg::Config::Extraction.new(
  language_detection: Kreuzberg::Config::LanguageDetection.new(
    enabled: true,
    min_confidence: 0.8,
    detect_multiple: true
  )
)

result = Kreuzberg.extract_file_sync('multilingual_document.pdf', config: config)

puts "Detected languages: #{result.detected_languages}"
# Output: ['eng', 'fra', 'deu']
```
