```ruby
require 'kreuzberg'

config = Kreuzberg::Config::Extraction.new(
  enable_quality_processing: true,
  language_detection: Kreuzberg::Config::LanguageDetection.new(
    enabled: true,
    detect_multiple: true
  ),
  token_reduction: Kreuzberg::Config::TokenReduction.new(mode: 'moderate'),
  chunking: Kreuzberg::Config::Chunking.new(
    max_chars: 512,
    max_overlap: 50,
    embedding: { normalize: true }
  ),
  keywords: Kreuzberg::Config::Keywords.new(
    algorithm: 'yake',
    max_keywords: 10
  )
)

result = Kreuzberg.extract_file_sync('document.pdf', config: config)
puts "Languages: #{result.detected_languages.inspect}"
puts "Chunks: #{result.chunks&.length || 0}"
```
