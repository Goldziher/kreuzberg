```ruby title="Ruby"
require 'kreuzberg'

config = Kreuzberg::Config::Extraction.new(
  token_reduction: Kreuzberg::Config::TokenReduction.new(
    mode: 'moderate',
    preserve_markdown: true
  )
)

result = Kreuzberg.extract_file_sync('verbose_document.pdf', config: config)

# Check reduction statistics in metadata
original_tokens = result.metadata['original_token_count']
reduced_tokens = result.metadata['token_count']
reduction_ratio = result.metadata['token_reduction_ratio']

puts "Reduced from #{original_tokens} to #{reduced_tokens} tokens"
puts "Reduction: #{reduction_ratio * 100}%"
```
