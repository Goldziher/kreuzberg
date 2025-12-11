```ruby
require 'kreuzberg'

# Ruby uses blocking APIs; async variants call into Tokio internally.
result = Kreuzberg.extract_file('document.pdf')
puts result.content
```
