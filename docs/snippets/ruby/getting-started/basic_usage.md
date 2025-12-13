```ruby title="Ruby"
require 'kreuzberg'

result = Kreuzberg.extract_file_sync('document.pdf')

puts "Content:"
puts result.content

puts "\nMetadata:"
puts "Title: #{result.metadata&.dig('title')}"
puts "Author: #{result.metadata&.dig('author')}"

puts "\nTables found: #{result.tables.length}"
puts "Images found: #{result.images.length}"
```
