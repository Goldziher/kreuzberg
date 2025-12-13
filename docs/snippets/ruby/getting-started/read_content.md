```ruby title="Ruby"
require 'kreuzberg'

result = Kreuzberg.extract_file_sync('document.pdf')

content = result.content
tables = result.tables
images = result.images
metadata = result.metadata

puts "Content: #{content.length} characters"
puts "Tables: #{tables.length}"
puts "Images: #{images.length}"
puts "Metadata keys: #{metadata&.keys&.join(', ')}"
```
