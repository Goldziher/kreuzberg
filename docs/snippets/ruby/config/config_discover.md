```ruby title="Ruby"
require 'kreuzberg'

config = Kreuzberg::Config::Extraction.discover
result = Kreuzberg.extract_file_sync('document.pdf', config: config)
```
