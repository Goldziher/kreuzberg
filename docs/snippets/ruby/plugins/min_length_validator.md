```ruby title="Ruby"
require "kreuzberg"

# Custom validator ensuring extracted content meets minimum length requirement
class MinLengthValidator
  def initialize(min_length: 100)
    @min_length = min_length
  end

  def call(result)
    content_length = result["content"].length
    if content_length < @min_length
      raise Kreuzberg::Errors::ValidationError,
            "Content too short: #{content_length} < #{@min_length}"
    end
  end
end

# Register the validator with priority
validator = MinLengthValidator.new(min_length: 100)
Kreuzberg.register_validator("min_length_validator", validator, 100)

# Usage in extraction
config = Kreuzberg::Config::Extraction.new

begin
  result = Kreuzberg.extract_file_sync("document.pdf", config: config)
  puts "Extraction successful: #{result["content"].length} characters"
rescue Kreuzberg::Errors::ValidationError => e
  puts "Validation failed: #{e.message}"
end
```
