```java
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.config.ExtractionConfig;
import dev.kreuzberg.config.ChunkingConfig;
import dev.kreuzberg.config.EmbeddingConfig;
import dev.kreuzberg.config.EmbeddingModelType;
import java.util.List;

ExtractionConfig config = ExtractionConfig.builder()
    .chunking(ChunkingConfig.builder()
        .maxChars(500)
        .maxOverlap(50)
        .embedding(EmbeddingConfig.builder()
            .model(EmbeddingModelType.preset("all-mpnet-base-v2"))
            .normalize(true)
            .batchSize(16)
            .build())
        .build())
    .build();

try {
    ExtractionResult result = Kreuzberg.extractFile("research_paper.pdf", config);

    List<Object> chunks = result.getChunks() != null ? result.getChunks() : List.of();
    System.out.println("Found " + chunks.size() + " chunks for RAG pipeline");

    for (int i = 0; i < Math.min(3, chunks.size()); i++) {
        Object chunk = chunks.get(i);
        System.out.println("Chunk " + i + ": " + chunk.toString().substring(0, Math.min(80, chunk.toString().length())) + "...");
    }
} catch (Exception ex) {
    System.err.println("RAG extraction failed: " + ex.getMessage());
}
```
