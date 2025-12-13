```ruby title="Ruby"
require 'kreuzberg'

config = Kreuzberg::Config::Extraction.new(
  language_detection: Kreuzberg::Config::LanguageDetection.new(
    enabled: true,
    min_confidence: 0.8,
    detect_multiple: false
  )
)
```
