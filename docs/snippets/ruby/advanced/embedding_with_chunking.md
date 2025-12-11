```ruby
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
      normalize: true,
      batch_size: 32,
      show_download_progress: false
    )
  )
)

result = Kreuzberg.extract_file_sync('document.pdf', config: config)

chunks = result.chunks || []
chunks.each_with_index do |chunk, idx|
  chunk_id = "doc_chunk_#{idx}"
  puts "Chunk #{chunk_id}: #{chunk.content[0...50]}"

  if chunk.embedding
    puts "  Embedding dimensions: #{chunk.embedding.length}"
  end
end
```
