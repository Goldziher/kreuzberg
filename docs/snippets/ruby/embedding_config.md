```ruby
require 'kreuzberg'

config = Kreuzberg::ExtractionConfig.new(
  chunking: Kreuzberg::ChunkingConfig.new(
    max_chars: 1000,
    embedding: Kreuzberg::EmbeddingConfig.new(
      model: Kreuzberg::EmbeddingModelType.new(
        type: 'preset',
        name: 'all-mpnet-base-v2'
      ),
      batch_size: 16,
      normalize: true,
      show_download_progress: true
    )
  )
)
```
