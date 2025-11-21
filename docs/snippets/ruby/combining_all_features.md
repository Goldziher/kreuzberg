```ruby
require 'kreuzberg'

config = Kreuzberg::ExtractionConfig.new(
  enable_quality_processing: true,

  language_detection: Kreuzberg::LanguageDetectionConfig.new(
    enabled: true,
    detect_multiple: true
  ),

  token_reduction: Kreuzberg::TokenReductionConfig.new(
    mode: 'moderate',
    preserve_markdown: true
  ),

  chunking: Kreuzberg::ChunkingConfig.new(
    max_chars: 512,
    max_overlap: 50,
    embedding: Kreuzberg::EmbeddingConfig.new(
      model: Kreuzberg::EmbeddingModelType.new(
        type: 'preset',
        name: 'balanced'
      ),
      normalize: true
    )
  ),

  keywords: Kreuzberg::KeywordConfig.new(
    algorithm: Kreuzberg::KeywordAlgorithm::YAKE,
    max_keywords: 10
  )
)

result = Kreuzberg.extract_file('document.pdf', config: config)

puts "Quality: #{result.metadata['quality_score'].round(2)}"
puts "Languages: #{result.detected_languages}"
puts "Keywords: #{result.metadata['keywords'].map { |kw| kw['text'] }}"
if result.chunks && result.chunks[0].embedding
  puts "Chunks: #{result.chunks.length} with #{result.chunks[0].embedding.length} dimensions"
end
```
