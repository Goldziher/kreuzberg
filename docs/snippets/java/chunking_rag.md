```java
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.config.ExtractionConfig;
import dev.kreuzberg.config.ChunkingConfig;

ExtractionConfig config = ExtractionConfig.builder()
    .chunking(ChunkingConfig.builder()
        .maxChars(500)
        .maxOverlap(50)
        .embedding("balanced")
        .build())
    .build();

ExtractionResult result = Kreuzberg.extractFileSync("research_paper.pdf", null, config);

// Note: Java bindings don't currently support chunks in the result
// This feature is planned for a future release
System.out.println("Content: " + result.getContent().substring(0, Math.min(100, result.getContent().length())) + "...");
```
