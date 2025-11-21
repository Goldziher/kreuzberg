```ruby
require 'kreuzberg'

config = Kreuzberg::ExtractionConfig.new(
  token_reduction: Kreuzberg::TokenReductionConfig.new(
    mode: 'moderate',
    preserve_markdown: true,
    preserve_code: true,
    language_hint: 'eng'
  )
)
```
