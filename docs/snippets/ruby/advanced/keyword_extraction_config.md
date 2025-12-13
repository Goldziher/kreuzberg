```ruby title="Ruby"
require 'kreuzberg'

config = Kreuzberg::Config::Extraction.new(
  keywords: Kreuzberg::Config::Keywords.new(
    algorithm: Kreuzberg::KeywordAlgorithm::YAKE,
    max_keywords: 10,
    min_score: 0.3,
    ngram_range: [1, 3],
    language: 'en'
  )
)
```
