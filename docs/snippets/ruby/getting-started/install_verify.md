```ruby title="Ruby"
require 'kreuzberg'

puts "Kreuzberg version: #{Kreuzberg::VERSION}"
puts "FFI bindings loaded successfully"

result = Kreuzberg.extract_file_sync('sample.pdf')
puts "Installation verified! Extracted #{result.content.length} characters"
```
