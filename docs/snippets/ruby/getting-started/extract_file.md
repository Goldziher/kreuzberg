```ruby
require 'kreuzberg'

config = Kreuzberg::Config::Extraction.new(
  use_cache: true,
  enable_quality_processing: true
)

result = Kreuzberg.extract_file_sync('contract.pdf', config: config)

puts "Extracted #{result.content.length} characters"
puts "Quality score: #{result.metadata&.dig('quality_score')}"
puts "Processing time: #{result.metadata&.dig('processing_time')}ms"
```
