```ruby
require 'kreuzberg'

data = File.binread('document.pdf')

result = Kreuzberg.extract_bytes(
  data,
  'application/pdf'
)
puts result.content
```
