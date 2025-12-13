```java title="Java"
import dev.kreuzberg.config.ExtractionConfig;
import dev.kreuzberg.config.KeywordConfig;
import dev.kreuzberg.config.KeywordAlgorithm;

ExtractionConfig config = ExtractionConfig.builder()
    .keywords(KeywordConfig.builder()
        .algorithm(KeywordAlgorithm.YAKE)
        .maxKeywords(10)
        .minScore(0.3)
        .ngramRange(1, 3)
        .language("en")
        .build())
    .build();
```
