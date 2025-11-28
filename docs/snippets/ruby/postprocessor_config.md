```ruby
require 'kreuzberg'

config = Kreuzberg::Config::Extraction.new(
  postprocessor: Kreuzberg::Config::PostProcessor.new(
    enabled: true,
    enabled_processors: ['deduplication', 'whitespace_normalization'],
    disabled_processors: ['mojibake_fix']
  )
)
```
