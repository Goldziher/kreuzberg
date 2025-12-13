```ruby title="Ruby"
require 'kreuzberg'

config = Kreuzberg::Config::Extraction.new(
  chunking: Kreuzberg::Config::Chunking.new(
    max_chars: 1000,
    max_overlap: 200,
    embedding: Kreuzberg::Config::Embedding.new(
      model: Kreuzberg::EmbeddingModelType.new(
        type: 'preset',
        name: 'all-minilm-l6-v2'
      ),
      normalize: true,
      batch_size: 32
    )
  )
)
```
