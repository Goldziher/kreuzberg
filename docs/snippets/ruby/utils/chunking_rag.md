```ruby
require 'kreuzberg'

config = Kreuzberg::Config::Extraction.new(
  chunking: Kreuzberg::Config::Chunking.new(
    max_chars: 500,
    max_overlap: 50,
    embedding: Kreuzberg::Config::Embedding.new(
      model: Kreuzberg::EmbeddingModelType.new(
        type: 'preset',
        name: 'balanced'
      ),
      normalize: true
    )
  )
)

result = Kreuzberg.extract_file_sync('research_paper.pdf', config: config)

result.chunks.each_with_index do |chunk, i|
  puts "Chunk #{i + 1}/#{result.chunks.length}"
  puts "Position: #{chunk.metadata[:char_start]}-#{chunk.metadata[:char_end]}"
  puts "Content: #{chunk.content[0..99]}..."
  puts "Embedding: #{chunk.embedding.length} dimensions" if chunk.embedding
end
```
