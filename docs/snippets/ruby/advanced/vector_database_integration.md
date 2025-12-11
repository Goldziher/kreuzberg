```ruby
require 'kreuzberg'

class VectorDatabaseIntegration
  VectorRecord = Struct.new(:id, :embedding, :content, :metadata, keyword_init: true)

  def extract_and_vectorize(document_path, document_id)
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
          batch_size: 32
        )
      )
    )

    result = Kreuzberg.extract_file_sync(document_path, config: config)
    chunks = result.chunks || []

    vector_records = chunks.map.with_index do |chunk, idx|
      VectorRecord.new(
        id: "#{document_id}_chunk_#{idx}",
        content: chunk.content,
        embedding: chunk.embedding,
        metadata: {
          document_id: document_id,
          chunk_index: idx,
          content_length: chunk.content.length
        }
      )
    end

    store_in_vector_database(vector_records)
    vector_records
  end

  private

  def store_in_vector_database(records)
    records.each do |record|
      if record.embedding&.any?
        puts "Storing #{record.id}: #{record.content.length} chars, #{record.embedding.length} dims"
      end
    end
  end
end
```
