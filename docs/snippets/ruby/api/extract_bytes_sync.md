```ruby
require 'kreuzberg'

data = File.binread('document.pdf')

result = Kreuzberg.extract_bytes_sync(
    data,
    'application/pdf'
)
puts result.content
```
