```java
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.config.ExtractionConfig;
import dev.kreuzberg.config.ChunkingConfig;
import dev.kreuzberg.config.LanguageDetectionConfig;
import dev.kreuzberg.config.TokenReductionConfig;

ExtractionConfig config = ExtractionConfig.builder()
    // Enable quality scoring
    .enableQualityProcessing(true)

    // Detect languages
    .languageDetection(LanguageDetectionConfig.builder()
        .enabled(true)
        .minConfidence(0.8)
        .build())

    // Reduce tokens before chunking
    .tokenReduction(TokenReductionConfig.builder()
        .mode("moderate")
        .preserveImportantWords(true)
        .build())

    // Chunk with embeddings
    .chunking(ChunkingConfig.builder()
        .maxChars(512)
        .maxOverlap(50)
        .embedding("balanced")
        .build())

    .build();

ExtractionResult result = Kreuzberg.extractFileSync("document.pdf", null, config);

// Display results
Object qualityScore = result.getMetadata().get("quality_score");
System.out.printf("Quality: %.2f%n", ((Number)qualityScore).doubleValue());
System.out.println("Languages: " + result.getDetectedLanguages());
System.out.println("Content length: " + result.getContent().length() + " characters");

// Note: Chunks and keywords not yet available in Java bindings
```
