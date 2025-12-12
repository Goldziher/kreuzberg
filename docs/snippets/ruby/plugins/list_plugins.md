```ruby title="Ruby"
require 'kreuzberg'

processors = Kreuzberg.list_post_processors
validators = Kreuzberg.list_validators
backends = Kreuzberg.list_ocr_backends

puts "Post-processors: #{processors.inspect}"
puts "Validators: #{validators.inspect}"
puts "OCR backends: #{backends.inspect}"
```
