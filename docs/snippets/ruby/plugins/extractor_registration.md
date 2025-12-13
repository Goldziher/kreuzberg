```ruby title="Ruby"
require 'kreuzberg'

# Register custom extractor with priority 50
Kreuzberg.register_document_extractor(
  name: "custom-json-extractor",
  extractor: ->(content, mime_type, config) {
    JSON.parse(content.to_s)
  },
  priority: 50
)

result = Kreuzberg.extract_file("document.json")
puts "Extracted content length: #{result.content.length}"
```
