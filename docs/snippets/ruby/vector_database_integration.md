```ruby
require 'kreuzberg'

config = Kreuzberg::ExtractionConfig.new(
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
  )
)

result = Kreuzberg.extract_file('document.pdf', config: config)

result.chunks.each_with_index do |chunk, i|
  if chunk.embedding
    puts "Chunk #{i}: #{chunk.embedding.length} dimensions"
    # Store in vector database
  end
end
```
