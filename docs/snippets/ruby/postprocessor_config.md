```ruby
require 'kreuzberg'

config = Kreuzberg::ExtractionConfig.new(
  postprocessor: Kreuzberg::PostProcessorConfig.new(
    enabled: true,
    enabled_processors: ['deduplication', 'whitespace_normalization'],
    disabled_processors: ['mojibake_fix']
  )
)
```
