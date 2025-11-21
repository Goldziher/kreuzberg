```ruby
require 'kreuzberg'

config = Kreuzberg::ExtractionConfig.new(
  chunking: Kreuzberg::ChunkingConfig.new(
    max_chars: 500,
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

result = Kreuzberg.extract_file('research_paper.pdf', config: config)

result.chunks.each_with_index do |chunk, i|
  puts "Chunk #{i + 1}/#{result.chunks.length}"
  puts "Position: #{chunk.metadata[:char_start]}-#{chunk.metadata[:char_end]}"
  puts "Content: #{chunk.content[0..99]}..."
  puts "Embedding: #{chunk.embedding.length} dimensions" if chunk.embedding
end
```
