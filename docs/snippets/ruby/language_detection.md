```ruby
require 'kreuzberg'

config = Kreuzberg::Config::Extraction.new(
  language_detection: Kreuzberg::Config::LanguageDetection.new(
    enabled: true,
    min_confidence: 0.9,
    detect_multiple: true
  )
)
```
