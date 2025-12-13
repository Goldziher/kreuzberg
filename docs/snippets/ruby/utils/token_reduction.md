```ruby title="Ruby"
require 'kreuzberg'

config = Kreuzberg::Config::Extraction.new(
  token_reduction: Kreuzberg::Config::TokenReduction.new(
    mode: 'moderate',
    preserve_important_words: true
  )
)
```
