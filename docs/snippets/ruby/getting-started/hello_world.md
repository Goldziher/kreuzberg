```ruby title="Ruby"
require 'kreuzberg'

result = Kreuzberg.extract_file_sync('document.pdf')
puts "Extracted content:"
puts result.content[0...200]
```
