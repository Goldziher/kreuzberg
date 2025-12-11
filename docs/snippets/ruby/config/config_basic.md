```ruby
require 'kreuzberg'

config = Kreuzberg::Config::Extraction.new(
  use_cache: true,
  enable_quality_processing: true
)

result = Kreuzberg.extract_file_sync('document.pdf', config: config)
```
