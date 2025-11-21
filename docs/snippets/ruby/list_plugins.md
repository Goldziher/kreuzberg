```ruby
require 'kreuzberg'

# List registered post-processors
processors = Kreuzberg.list_post_processors
puts "Post-processors: #{processors.inspect}"

# List registered validators
validators = Kreuzberg.list_validators
puts "Validators: #{validators.inspect}"

# List registered OCR backends
backends = Kreuzberg.list_ocr_backends
puts "OCR backends: #{backends.inspect}"

# Iterate over registered plugins
Kreuzberg.list_post_processors.each do |processor_name|
  puts "- Processor: #{processor_name}"
end

Kreuzberg.list_validators.each do |validator_name|
  puts "- Validator: #{validator_name}"
end

Kreuzberg.list_ocr_backends.each do |backend_name|
  puts "- OCR backend: #{backend_name}"
end
```
