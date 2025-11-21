```ruby
require 'kreuzberg'

config = Kreuzberg::ExtractionConfig.new(
  language_detection: Kreuzberg::LanguageDetectionConfig.new(
    enabled: true,
    min_confidence: 0.8,
    detect_multiple: true
  )
)

result = Kreuzberg.extract_file('multilingual_document.pdf', config: config)

puts "Detected languages: #{result.detected_languages}"
# Output: ['eng', 'fra', 'deu']
```
