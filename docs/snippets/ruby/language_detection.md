```ruby
require 'kreuzberg'

config = Kreuzberg::ExtractionConfig.new(
  language_detection: Kreuzberg::LanguageDetectionConfig.new(
    enabled: true,
    min_confidence: 0.9,
    detect_multiple: true
  )
)
```
