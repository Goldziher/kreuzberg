```ruby
require 'kreuzberg'

config = Kreuzberg::ExtractionConfig.new(
  use_cache: true,
  enable_quality_processing: true
)

result = Kreuzberg.extract_file('document.pdf', config: config)
```
