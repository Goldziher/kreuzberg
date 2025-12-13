```java title="Java"
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.config.ExtractionConfig;
import dev.kreuzberg.config.ChunkingConfig;
import dev.kreuzberg.config.EmbeddingConfig;
import dev.kreuzberg.config.EmbeddingModelType;
import java.util.List;

ExtractionConfig config = ExtractionConfig.builder()
    .chunking(ChunkingConfig.builder()
        .maxChars(512)
        .maxOverlap(50)
        .embedding(EmbeddingConfig.builder()
            .model(EmbeddingModelType.preset("balanced"))
            .normalize(true)
            .batchSize(32)
            .showDownloadProgress(false)
            .build())
        .build())
    .build();

ExtractionResult result = Kreuzberg.extractFile("document.pdf", config);

List<Object> chunks = result.getChunks() != null ? result.getChunks() : List.of();
for (int index = 0; index < chunks.size(); index++) {
    Object chunk = chunks.get(index);
    String chunkId = "doc_chunk_" + index;
    System.out.println("Chunk " + chunkId + ": " + chunk.toString().substring(0, Math.min(50, chunk.toString().length())));

    if (chunk instanceof java.util.Map) {
        Object embedding = ((java.util.Map<String, Object>) chunk).get("embedding");
        if (embedding != null) {
            System.out.println("  Embedding dimensions: " + ((float[]) embedding).length);
        }
    }
}
```
