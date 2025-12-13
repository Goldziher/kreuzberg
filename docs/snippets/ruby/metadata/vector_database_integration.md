```ruby title="Ruby"
require 'kreuzberg'

config = Kreuzberg::Config::Extraction.new(
  chunking: Kreuzberg::Config::Chunking.new(
    max_chars: 512,
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

result = Kreuzberg.extract_file_sync('document.pdf', config: config)

result.chunks.each_with_index do |chunk, i|
  if chunk.embedding
    puts "Chunk #{i}: #{chunk.embedding.length} dimensions"
    # Store in vector database
  end
end
```
