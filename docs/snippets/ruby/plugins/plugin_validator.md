```ruby
require "kreuzberg"

validator = lambda do |result|
  raise Kreuzberg::ValidationError, "Content too short" if result.content.length < 50
end

Kreuzberg.register_validator("min_length", validator, priority: 10)

result = Kreuzberg.extract_file_sync("document.pdf")
puts "Validated content length: #{result.content.length}"

Kreuzberg.unregister_validator("min_length")
```
