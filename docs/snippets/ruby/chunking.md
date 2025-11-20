```ruby
require 'kreuzberg'

config = Kreuzberg::ExtractionConfig.new(
  chunking: Kreuzberg::ChunkingConfig.new(
    max_chars: 1500,
    max_overlap: 200,
    embedding: Kreuzberg::EmbeddingConfig.new(
      model: Kreuzberg::EmbeddingModelType.new(
        type: 'preset',
        name: 'text-embedding-all-minilm-l6-v2'
      )
    )
  )
)
```
