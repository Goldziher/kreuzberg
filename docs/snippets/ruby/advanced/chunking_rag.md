```ruby title="Ruby"
require 'kreuzberg'

config = Kreuzberg::Config::Extraction.new(
  chunking: Kreuzberg::Config::Chunking.new(
    max_chars: 500,
    max_overlap: 50,
    embedding: Kreuzberg::Config::Embedding.new(
      model: Kreuzberg::EmbeddingModelType.new(
        type: 'preset',
        name: 'all-mpnet-base-v2'
      ),
      normalize: true,
      batch_size: 16
    )
  )
)

result = Kreuzberg.extract_file_sync('research_paper.pdf', config: config)

vector_store = build_vector_store(result.chunks)
query = 'machine learning optimization'
relevant_chunks = search_vector_store(vector_store, query)

puts "Found #{relevant_chunks.length} relevant chunks"
relevant_chunks.take(3).each do |chunk|
  puts "Content: #{chunk[:content][0..80]}..."
  puts "Similarity: #{chunk[:similarity]&.round(3)}\n"
end

def build_vector_store(chunks)
  chunks.map.with_index do |chunk, idx|
    {
      id: idx,
      content: chunk.content,
      embedding: chunk.embedding,
      similarity: 0.0
    }
  end
end

def search_vector_store(store, query)
  store.sort_by { |entry| entry[:similarity] }.reverse
end
```
