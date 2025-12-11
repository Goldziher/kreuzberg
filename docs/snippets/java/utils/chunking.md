```java
import dev.kreuzberg.config.ChunkingConfig;
import dev.kreuzberg.config.EmbeddingConfig;
import dev.kreuzberg.config.EmbeddingModelType;
import dev.kreuzberg.config.ExtractionConfig;

ExtractionConfig config = ExtractionConfig.builder()
    .chunking(ChunkingConfig.builder()
        .maxChars(1500)
        .maxOverlap(200)
        .embedding(EmbeddingConfig.builder()
            .model(EmbeddingModelType.builder()
                .type("preset")
                .name("text-embedding-all-minilm-l6-v2")
                .build())
            .build())
        .build())
    .build();
```
