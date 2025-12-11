```java
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.config.ExtractionConfig;
import dev.kreuzberg.config.ChunkingConfig;
import dev.kreuzberg.config.LanguageDetectionConfig;
import dev.kreuzberg.config.TokenReductionConfig;

ExtractionConfig config = ExtractionConfig.builder()
    .enableQualityProcessing(true)
    .languageDetection(LanguageDetectionConfig.builder()
        .enabled(true)
        .minConfidence(0.8)
        .build())
    .tokenReduction(TokenReductionConfig.builder()
        .mode("moderate")
        .preserveImportantWords(true)
        .build())
    .chunking(ChunkingConfig.builder()
        .maxChars(512)
        .maxOverlap(50)
        .embedding("balanced")
        .build())
    .build();

ExtractionResult result = Kreuzberg.extractFile("document.pdf", config);

Object qualityScore = result.getMetadata().get("quality_score");
System.out.printf("Quality: %.2f%n", ((Number)qualityScore).doubleValue());
System.out.println("Languages: " + result.getDetectedLanguages());
System.out.println("Content length: " + result.getContent().length() + " characters");
```
