```java title="Java"
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.config.ExtractionConfig;
import dev.kreuzberg.config.ChunkingConfig;

ExtractionConfig config = ExtractionConfig.builder()
    .chunking(ChunkingConfig.builder()
        .maxChars(512)
        .maxOverlap(50)
        .embedding("balanced")
        .build())
    .build();

ExtractionResult result = Kreuzberg.extractFile("document.pdf", config);

System.out.println("Extracted content: " + result.getContent().length() + " characters");
```
