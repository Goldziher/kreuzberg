```ruby
require 'kreuzberg'

result = Kreuzberg.extract_file_sync('document.pdf')

puts result.content
puts "Tables: #{result.tables.length}"
puts "Metadata: #{result.metadata}"
```
