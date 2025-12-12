```java title="Java"
import dev.kreuzberg.config.ChunkingConfig;
import dev.kreuzberg.config.EmbeddingConfig;
import dev.kreuzberg.config.EmbeddingModelType;
import dev.kreuzberg.config.ExtractionConfig;

ExtractionConfig config = ExtractionConfig.builder()
    .chunking(ChunkingConfig.builder()
        .maxChars(1000)
        .embedding(EmbeddingConfig.builder()
            .model(EmbeddingModelType.builder()
                .type("preset")
                .name("all-mpnet-base-v2")
                .build())
            .batchSize(16)
            .normalize(true)
            .showDownloadProgress(true)
            .build())
        .build())
    .build();
```
