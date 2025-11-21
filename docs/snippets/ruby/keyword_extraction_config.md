```ruby
require 'kreuzberg'

config = Kreuzberg::ExtractionConfig.new(
  keywords: Kreuzberg::KeywordConfig.new(
    algorithm: Kreuzberg::KeywordAlgorithm::YAKE,
    max_keywords: 10,
    min_score: 0.3,
    ngram_range: [1, 3],
    language: 'en'
  )
)
```
