```ruby
require 'kreuzberg'

config = Kreuzberg::ExtractionConfig.discover
result = Kreuzberg.extract_file('document.pdf', config: config)
```
