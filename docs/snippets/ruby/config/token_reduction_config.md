```ruby title="Ruby"
require 'kreuzberg'

config = Kreuzberg::Config::Extraction.new(
  token_reduction: Kreuzberg::Config::TokenReduction.new(
    mode: 'moderate',
    preserve_markdown: true,
    preserve_code: true,
    language_hint: 'eng'
  )
)
```
